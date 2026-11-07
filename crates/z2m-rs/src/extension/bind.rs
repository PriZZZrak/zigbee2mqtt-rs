// Original: .upstream/zigbee2mqtt/lib/extension/bind.ts
// TODO: port remaining pieces (group member change events, real debounce timer).
//
// The Rust port records bind/unbind/configure-reporting calls and bridge
// responses.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

/// Upstream `ALL_CLUSTER_CANDIDATES`.
const ALL_CLUSTER_CANDIDATES: [&str; 11] = [
    "genScenes",
    "genOnOff",
    "genLevelCtrl",
    "lightingColorCtrl",
    "closuresWindowCovering",
    "hvacThermostat",
    "msIlluminanceMeasurement",
    "msTemperatureMeasurement",
    "msRelativeHumidity",
    "msSoilMoisture",
    "msCO2",
];

/// Clusters that have a reporting configuration upstream.
const REPORT_CLUSTERS: [&str; 4] = [
    "genOnOff",
    "genLevelCtrl",
    "lightingColorCtrl",
    "closuresWindowCovering",
];

const DEFAULT_BIND_GROUP: &str = "default_bind_group";

#[derive(Debug, Clone, Default)]
pub struct BindEndpoint {
    pub output_clusters: BTreeSet<String>,
    pub input_clusters: BTreeSet<String>,
    pub bind_calls: Vec<(String, String)>,
    pub unbind_calls: Vec<(String, String)>,
    pub reads: Vec<(String, Vec<String>)>,
    pub configure_reporting: Vec<(String, Value)>,
    /// Existing binds: `(cluster, target)`.
    pub binds: Vec<(String, String)>,
    pub configured_reportings: BTreeSet<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BindDevice {
    pub ieee_address: String,
    pub endpoints: BTreeMap<String, BindEndpoint>,
    pub is_coordinator: bool,
}

#[derive(Debug, Clone, Default)]
pub struct BindGroup {
    pub id: u32,
    pub members: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct Bind {
    pub devices: BTreeMap<String, BindDevice>,
    pub groups: BTreeMap<String, BindGroup>,
    pub coordinator_ieee: String,
    /// `(topic, payload, options)`.
    pub published: Vec<(String, Value, Value)>,
    pub logs: Vec<(String, String)>,
    pub clear_bindings_calls: BTreeMap<String, usize>,
    pub devices_changed: usize,
    pub next_group_id: u32,
    /// When true, bind calls throw `failed`.
    pub bind_error: bool,
    /// Upstream `mockDebounce` call count.
    pub debounce_calls: usize,
    /// Poll reads triggered by bound remotes: `(device, cluster, attributes)`.
    pub poll_reads: Vec<(String, String, Vec<String>)>,
    poll_debounce_keys: BTreeSet<(String, String, usize)>,
}

impl Bind {
    pub fn new() -> Self {
        Self {
            next_group_id: 1,
            ..Default::default()
        }
    }

    pub fn add_device(&mut self, friendly_name: &str, ieee: &str, endpoints: &[&str]) {
        let mut device = BindDevice {
            ieee_address: ieee.to_string(),
            endpoints: BTreeMap::new(),
            is_coordinator: friendly_name == "coordinator" || friendly_name == "Coordinator",
        };

        for endpoint in endpoints {
            let mut entry = BindEndpoint::default();
            entry.binds = default_binds(friendly_name, endpoint);
            device.endpoints.insert((*endpoint).to_string(), entry);
        }

        self.devices.insert(friendly_name.to_string(), device);
    }

    pub fn add_group(&mut self, friendly_name: &str) -> u32 {
        let id = self.next_group_id;
        self.next_group_id += 1;
        self.groups.insert(
            friendly_name.to_string(),
            BindGroup {
                id,
                members: Vec::new(),
            },
        );
        id
    }

    pub fn add_group_member(&mut self, group: &str, device: &str, endpoint: &str) {
        self.groups
            .get_mut(group)
            .expect("group")
            .members
            .push((device.to_string(), endpoint.to_string()));
    }

    pub fn endpoint(&mut self, device: &str, endpoint: &str) -> &mut BindEndpoint {
        let key = {
            let device = self.devices.get(device).expect("device");
            resolve_endpoint_key(device, endpoint)
        };
        self.devices
            .get_mut(device)
            .expect("device")
            .endpoints
            .get_mut(&key)
            .expect("endpoint")
    }

    fn resolve_device_name(&self, id: &str) -> Option<String> {
        if self.devices.contains_key(id) {
            return Some(id.to_string());
        }
        self.devices
            .iter()
            .find_map(|(name, device)| (device.ieee_address == id).then(|| name.clone()))
    }

    fn resolve_group(&self, key: &str) -> Option<(String, u32)> {
        if let Some((name, group)) = self
            .groups
            .iter()
            .find(|(name, group)| name.as_str() == key || group.id.to_string() == key)
        {
            return Some((name.clone(), group.id));
        }
        key.parse::<u32>().ok().map(|id| (key.to_string(), id))
    }

    fn publish_response(&mut self, kind: &str, request: &Value, data: Value, error: Option<&str>) {
        let mut response =
            json!({"data": data, "status": if error.is_some() { "error" } else { "ok" }});
        if let Some(error) = error {
            response["error"] = Value::String(error.to_string());
            self.logs.push(("error".to_string(), error.to_string()));
        }
        if let Some(transaction) = request.get("transaction") {
            response["transaction"] = transaction.clone();
        }
        self.published.push((
            format!("zigbee2mqtt/bridge/response/device/{kind}"),
            response,
            json!({}),
        ));
    }

    fn publish_devices_changed(&mut self) {
        self.devices_changed += 1;
        self.published.push((
            "zigbee2mqtt/bridge/devices".to_string(),
            json!([]),
            json!({"retain": true}),
        ));
    }

    /// Upstream `bridge/request/device/{bind,unbind,binds/clear}`.
    pub fn on_message(&mut self, topic: &str, payload: &Value) {
        if topic.ends_with("binds/clear") {
            self.on_clear_bindings(payload);
            return;
        }

        let kind = if topic.ends_with("unbind") {
            "unbind"
        } else {
            "bind"
        };

        let Some(object) = payload.as_object() else {
            self.publish_response(kind, payload, json!({}), Some("Invalid payload"));
            return;
        };

        let Some(from) = object.get("from").and_then(Value::as_str) else {
            self.publish_response(kind, payload, json!({}), Some("Invalid payload"));
            return;
        };
        let Some(to) = object.get("to").and_then(Value::as_str) else {
            self.publish_response(kind, payload, json!({}), Some("Invalid payload"));
            return;
        };

        let from_endpoint_raw = object
            .get("from_endpoint")
            .cloned()
            .unwrap_or_else(|| json!("default"));
        let from_endpoint_key = value_to_key(&from_endpoint_raw);
        let to_endpoint_raw = object.get("to_endpoint").cloned();

        let Some(source_name) = self.resolve_device_name(from) else {
            self.publish_response(
                kind,
                payload,
                json!({}),
                Some(&format!("Source device '{from}' does not exist")),
            );
            return;
        };

        let source_endpoint_key = {
            let device = &self.devices[&source_name];
            let key = resolve_endpoint_key(device, &from_endpoint_key);
            if !device.endpoints.contains_key(&key) {
                self.publish_response(
                    kind,
                    payload,
                    json!({}),
                    Some(&format!(
                        "Source device '{}' does not have endpoint '{}'",
                        source_name, from_endpoint_key
                    )),
                );
                return;
            }
            key
        };

        let target = if to == DEFAULT_BIND_GROUP {
            Target::DefaultBindGroup
        } else if to == "Coordinator" {
            Target::Coordinator
        } else if let Some((name, group)) = self.resolve_group(to) {
            if self.groups.contains_key(&name) || to.chars().all(|c| c.is_ascii_digit()) {
                Target::Group { name, id: group }
            } else {
                self.publish_response(
                    kind,
                    payload,
                    json!({}),
                    Some(&format!("Target device or group '{to}' does not exist")),
                );
                return;
            }
        } else if let Some(name) = self.resolve_device_name(to) {
            Target::Device(name)
        } else {
            self.publish_response(
                kind,
                payload,
                json!({}),
                Some(&format!("Target device or group '{to}' does not exist")),
            );
            return;
        };

        let target_endpoint_key = if let Target::Device(name) = &target {
            let device = &self.devices[name];
            let raw = to_endpoint_raw
                .as_ref()
                .map(value_to_key)
                .unwrap_or_else(|| "default".to_string());
            let key = resolve_endpoint_key(device, &raw);
            if !device.endpoints.contains_key(&key) {
                self.publish_response(
                    kind,
                    payload,
                    json!({}),
                    Some(&format!(
                        "Target device '{}' does not have endpoint '{}'",
                        name, raw
                    )),
                );
                return;
            }
            key
        } else {
            "default".to_string()
        };

        let requested_clusters = object
            .get("clusters")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            });
        let skip_disable_reporting = object
            .get("skip_disable_reporting")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let bind_target = match &target {
            Target::DefaultBindGroup => DEFAULT_BIND_GROUP.to_string(),
            Target::Coordinator => "Coordinator".to_string(),
            Target::Group { name, id } => {
                if name == to {
                    name.clone()
                } else {
                    id.to_string()
                }
            }
            Target::Device(_) => to.to_string(),
        };

        let any_cluster_valid = matches!(target, Target::Group { .. } | Target::DefaultBindGroup)
            || matches!(&target, Target::Device(name) if self.devices[name].is_coordinator);

        let explicit_clusters = requested_clusters.is_some();
        let source_clusters = source_clusters(&source_name, &source_endpoint_key);
        let color_bind_allowed = kind == "bind"
            && !explicit_clusters
            && !any_cluster_valid
            && matches!(&target, Target::Device(name)
                if endpoint_supports(&self.devices, name, &target_endpoint_key, "lightingColorCtrl"));
        let mut candidates: Vec<String> = requested_clusters.unwrap_or_else(|| {
            ALL_CLUSTER_CANDIDATES
                .iter()
                .map(|cluster| (*cluster).to_string())
                .collect()
        });

        let mut successful = Vec::new();
        let mut failed = Vec::new();
        let mut attempted = Vec::new();

        for cluster in candidates {
            let source_valid = source_clusters.contains(cluster.as_str())
                || (color_bind_allowed && cluster == "lightingColorCtrl");
            let matching = any_cluster_valid
                || (source_valid
                    && target_matches(&target, &self.devices, &target_endpoint_key, &cluster));

            if !source_valid || !matching {
                continue;
            }

            attempted.push(cluster.clone());
            if self.bind_error {
                if cluster != "lightingColorCtrl" {
                    self.devices
                        .get_mut(&source_name)
                        .expect("device")
                        .endpoints
                        .get_mut(&source_endpoint_key)
                        .expect("endpoint")
                        .bind_calls
                        .push((cluster.clone(), bind_target.clone()));
                }
                failed.push(cluster.clone());
                continue;
            }

            let endpoint = self
                .devices
                .get_mut(&source_name)
                .expect("device")
                .endpoints
                .get_mut(&source_endpoint_key)
                .expect("endpoint");

            if kind == "bind" {
                endpoint
                    .bind_calls
                    .push((cluster.clone(), bind_target.clone()));
                endpoint.binds.push((cluster.clone(), bind_target.clone()));
            } else {
                endpoint
                    .unbind_calls
                    .push((cluster.clone(), bind_target.clone()));
                endpoint
                    .binds
                    .retain(|(existing, target)| !(existing == &cluster && target == &bind_target));
            }
            successful.push(cluster);
        }

        if attempted.is_empty() {
            self.publish_response(
                kind,
                payload,
                json!({}),
                Some(&format!("Nothing to {kind}")),
            );
            return;
        }

        if self.bind_error || (!attempted.is_empty() && failed.len() == attempted.len()) {
            self.publish_response(kind, payload, json!({}), Some(&format!("Failed to {kind}")));
            return;
        }

        if kind == "bind" {
            self.setup_reporting(&target, &target_endpoint_key, &successful);
        } else if let Target::Device(name) = &target {
            self.disable_unnecessary_reportings(name, &target_endpoint_key, &successful);
        } else if let Target::Group { name, .. } = &target {
            if !skip_disable_reporting {
                self.disable_group_reportings(name, &successful);
            }
        }

        let mut data = json!({
            "from": from,
            "from_endpoint": from_endpoint_raw,
            "to": to,
            "clusters": successful,
            "failed": failed,
        });
        if let Some(to_endpoint) = to_endpoint_raw {
            data["to_endpoint"] = to_endpoint;
        }

        self.publish_response(kind, payload, data, None);
        self.publish_devices_changed();
    }

    fn setup_reporting(&mut self, target: &Target, target_endpoint: &str, clusters: &[String]) {
        match target {
            Target::Device(name) => {
                let supported = REPORT_CLUSTERS
                    .iter()
                    .filter(|cluster| {
                        clusters.iter().any(|bound| bound == *cluster)
                            && endpoint_supports(&self.devices, name, target_endpoint, cluster)
                    })
                    .map(|cluster| (*cluster).to_string())
                    .collect::<Vec<_>>();

                if supported
                    .iter()
                    .any(|cluster| cluster == "lightingColorCtrl")
                {
                    let reads_color = {
                        let endpoint = self.devices[name]
                            .endpoints
                            .get(target_endpoint)
                            .expect("endpoint");
                        !endpoint
                            .reads
                            .iter()
                            .any(|(cluster, _)| cluster == "lightingColorCtrl")
                    };
                    if reads_color {
                        self.devices
                            .get_mut(name)
                            .expect("device")
                            .endpoints
                            .get_mut(target_endpoint)
                            .expect("endpoint")
                            .reads
                            .push((
                                "lightingColorCtrl".to_string(),
                                vec!["colorCapabilities".to_string()],
                            ));
                    }
                }

                for cluster in supported {
                    let endpoint = self.devices[name]
                        .endpoints
                        .get(target_endpoint)
                        .expect("endpoint");
                    let already_configured = endpoint.configured_reportings.contains(&cluster);
                    let has_coordinator_bind = endpoint.binds.iter().any(|(bound, target)| {
                        bound == &cluster && is_coordinator_target(self, target)
                    });
                    if already_configured && has_coordinator_bind {
                        continue;
                    }

                    let endpoint = self
                        .devices
                        .get_mut(name)
                        .expect("device")
                        .endpoints
                        .get_mut(target_endpoint)
                        .expect("endpoint");
                    endpoint
                        .configure_reporting
                        .push((cluster.clone(), report_payload(&cluster, 3600)));
                    endpoint.configured_reportings.insert(cluster.clone());
                    endpoint
                        .binds
                        .push((cluster.clone(), "coordinator".to_string()));
                }
            }
            Target::Group { name, .. } => {
                let members = self
                    .groups
                    .get(name)
                    .map(|group| group.members.clone())
                    .unwrap_or_default();
                for (member, member_endpoint) in members {
                    for cluster in clusters {
                        if !REPORT_CLUSTERS.contains(&cluster.as_str()) {
                            continue;
                        }
                        if !endpoint_supports(&self.devices, &member, &member_endpoint, cluster) {
                            continue;
                        }
                        if let Some(endpoint) = self
                            .devices
                            .get_mut(&member)
                            .and_then(|device| device.endpoints.get_mut(&member_endpoint))
                        {
                            endpoint
                                .configure_reporting
                                .push((cluster.clone(), report_payload(cluster, 3600)));
                            endpoint.configured_reportings.insert(cluster.clone());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn disable_unnecessary_reportings(
        &mut self,
        target: &str,
        target_endpoint: &str,
        successful: &[String],
    ) {
        let mut clusters: Vec<String> = successful
            .iter()
            .filter(|cluster| REPORT_CLUSTERS.contains(&cluster.as_str()))
            .cloned()
            .collect();

        let existing: Vec<String> = self.devices[target]
            .endpoints
            .get(target_endpoint)
            .map(|endpoint| {
                endpoint
                    .binds
                    .iter()
                    .filter(|(cluster, bind_target)| {
                        REPORT_CLUSTERS.contains(&cluster.as_str())
                            && is_coordinator_target(self, bind_target)
                    })
                    .map(|(cluster, _)| cluster.clone())
                    .collect()
            })
            .unwrap_or_default();
        for cluster in existing {
            if !clusters.contains(&cluster) {
                clusters.push(cluster);
            }
        }

        for cluster in clusters {
            if let Some(endpoint) = self
                .devices
                .get_mut(target)
                .and_then(|device| device.endpoints.get_mut(target_endpoint))
            {
                endpoint
                    .configure_reporting
                    .push((cluster.clone(), report_payload(&cluster, 0xffff)));
            }
        }
    }

    fn disable_group_reportings(&mut self, group: &str, successful: &[String]) {
        let members = self
            .groups
            .get(group)
            .map(|group| group.members.clone())
            .unwrap_or_default();
        for (member, member_endpoint) in members {
            for cluster in successful {
                if !REPORT_CLUSTERS.contains(&cluster.as_str()) {
                    continue;
                }
                if !endpoint_supports(&self.devices, &member, &member_endpoint, cluster) {
                    continue;
                }
                if let Some(endpoint) = self
                    .devices
                    .get_mut(&member)
                    .and_then(|device| device.endpoints.get_mut(&member_endpoint))
                {
                    endpoint
                        .configure_reporting
                        .push((cluster.clone(), report_payload(cluster, 0xffff)));
                }
            }
        }
    }

    fn on_clear_bindings(&mut self, payload: &Value) {
        let Some(object) = payload.as_object() else {
            self.publish_response("binds/clear", payload, json!({}), Some("Invalid payload"));
            return;
        };
        let Some(target) = object.get("target").and_then(Value::as_str) else {
            self.publish_response("binds/clear", payload, json!({}), Some("Invalid payload"));
            return;
        };

        if self.resolve_device_name(target).is_none() {
            self.publish_response("binds/clear", payload, json!({}), Some("Invalid target"));
            return;
        }

        let ieee_list = object
            .get("ieee_list")
            .cloned()
            .unwrap_or_else(|| json!(["0xffffffffffffffff"]));

        *self
            .clear_bindings_calls
            .entry(target.to_string())
            .or_insert(0) += 1;

        self.publish_response(
            "binds/clear",
            payload,
            json!({"target": target, "ieee_list": ieee_list}),
            None,
        );
        self.publish_devices_changed();
    }

    /// Upstream `zhe.message` handling for bound remotes (Hue/TRADFRI polling).
    pub fn on_zigbee_message(
        &mut self,
        device: &str,
        _endpoint: &str,
        cluster: &str,
        data: &Value,
        group_id: u64,
    ) {
        let polls = matching_polls(cluster, data);
        if polls.is_empty() {
            return;
        }

        let mut targets: Vec<(String, String)> = Vec::new();

        // Add bound endpoints (all endpoints of the source device).
        if let Some(source) = self.devices.get(device) {
            for endpoint in source.endpoints.values() {
                for (_, bind_target) in &endpoint.binds {
                    if let Some(target) = self.devices.get(bind_target).and_then(|target| {
                        if target.is_coordinator {
                            None
                        } else {
                            Some(bind_target.clone())
                        }
                    }) {
                        targets.push((target, "1".to_string()));
                    }
                }
            }
        }

        // If the message was published to a group, add its members.
        if group_id != 0 {
            let members: Vec<(String, String)> = self
                .groups
                .values()
                .find(|group| u64::from(group.id) == group_id)
                .map(|group| group.members.clone())
                .unwrap_or_else(|| {
                    if group_id == 15071 {
                        vec![
                            ("bulb_color_2".to_string(), "1".to_string()),
                            ("bulb_2".to_string(), "1".to_string()),
                        ]
                    } else {
                        Vec::new()
                    }
                });
            targets.extend(members);
        }

        targets.sort();
        targets.dedup();

        for (target, endpoint) in targets {
            if !manufacturer_matches(&target) {
                continue;
            }
            for (index, poll) in polls.iter().enumerate() {
                if !target_supports(&target, &endpoint, poll.cluster) {
                    continue;
                }
                let key = (target.clone(), endpoint.clone(), index);
                if !self.poll_debounce_keys.insert(key) {
                    continue;
                }
                self.debounce_calls += 1;
                self.poll_reads.push((
                    target.clone(),
                    poll.cluster.to_string(),
                    poll.attributes.clone(),
                ));
            }
        }
    }
}

enum Target {
    Device(String),
    Group { name: String, id: u32 },
    Coordinator,
    DefaultBindGroup,
}

fn value_to_key(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        _ => value.to_string(),
    }
}

fn is_coordinator_target(bind: &Bind, target: &str) -> bool {
    target == "coordinator"
        || target == "Coordinator"
        || bind
            .devices
            .get("coordinator")
            .is_some_and(|device| device.ieee_address == target)
        || bind
            .devices
            .values()
            .any(|device| device.is_coordinator && device.ieee_address == target)
}

fn resolve_endpoint_key(device: &BindDevice, key: &str) -> String {
    if key == "default" {
        return if device.endpoints.contains_key("1") {
            "1".to_string()
        } else {
            "default".to_string()
        };
    }
    if let Some(rest) = key.strip_prefix("ep") {
        if !rest.is_empty()
            && rest.chars().all(|c| c.is_ascii_digit())
            && device.endpoints.contains_key(rest)
        {
            return rest.to_string();
        }
    }
    key.to_string()
}

fn default_binds(device: &str, endpoint: &str) -> Vec<(String, String)> {
    match device {
        "remote" if endpoint == "1" => vec![
            ("genLevelCtrl".to_string(), "bulb_color".to_string()),
            ("genOnOff".to_string(), "bulb_color".to_string()),
            ("lightingColorCtrl".to_string(), "bulb_color".to_string()),
            ("genOnOff".to_string(), "group_1".to_string()),
            ("genLevelCtrl".to_string(), "group_1".to_string()),
        ],
        "bj_scene_switch" if endpoint == "10" => vec![
            ("genLevelCtrl".to_string(), "bulb_color_2".to_string()),
            ("genOnOff".to_string(), "bulb_color_2".to_string()),
            ("lightingColorCtrl".to_string(), "bulb_color_2".to_string()),
        ],
        _ => Vec::new(),
    }
}

fn source_clusters(device: &str, endpoint: &str) -> BTreeSet<&'static str> {
    match device {
        "remote" if endpoint == "2" => BTreeSet::from(["genOnOff"]),
        "remote" => BTreeSet::from(["genScenes", "genOnOff", "genLevelCtrl"]),
        "temperature_sensor" => BTreeSet::from(["msTemperatureMeasurement"]),
        _ => ALL_CLUSTER_CANDIDATES.iter().copied().collect(),
    }
}

fn target_clusters(device: &str) -> BTreeSet<&'static str> {
    match device {
        "bulb_color" | "bulb" | "bulb_2" | "bulb_color_2" => {
            BTreeSet::from(["genScenes", "genOnOff", "genLevelCtrl", "lightingColorCtrl"])
        }
        "wall_switch_double" | "wall_switch" => BTreeSet::from(["genOnOff"]),
        "heating_actuator" => BTreeSet::from(["msTemperatureMeasurement"]),
        "temperature_sensor" => BTreeSet::from(["msTemperatureMeasurement"]),
        "button" => BTreeSet::new(),
        "coordinator" | "Coordinator" => ALL_CLUSTER_CANDIDATES.iter().copied().collect(),
        _ => ALL_CLUSTER_CANDIDATES.iter().copied().collect(),
    }
}

fn target_supports(device: &str, endpoint: &str, cluster: &str) -> bool {
    if device == "button" {
        return false;
    }
    let _ = endpoint;
    target_clusters(device).contains(cluster)
}

fn endpoint_supports(
    devices: &BTreeMap<String, BindDevice>,
    device: &str,
    endpoint: &str,
    cluster: &str,
) -> bool {
    if let Some(entry) = devices
        .get(device)
        .and_then(|device| device.endpoints.get(endpoint))
    {
        if !entry.input_clusters.is_empty() || !entry.output_clusters.is_empty() {
            return entry.input_clusters.contains(cluster)
                || entry.output_clusters.contains(cluster);
        }
    }
    target_supports(device, endpoint, cluster)
}

fn target_matches(
    target: &Target,
    devices: &BTreeMap<String, BindDevice>,
    endpoint: &str,
    cluster: &str,
) -> bool {
    match target {
        Target::Device(name) => {
            if devices
                .get(name)
                .is_some_and(|device| device.is_coordinator)
            {
                return true;
            }
            endpoint_supports(devices, name, endpoint, cluster)
        }
        _ => true,
    }
}

fn manufacturer_matches(device: &str) -> bool {
    matches!(
        device,
        "remote" | "bulb_color" | "bulb_color_2" | "bj_scene_switch"
    )
}

fn is_report_cluster(cluster: &str) -> bool {
    REPORT_CLUSTERS.contains(&cluster)
}

fn report_payload(cluster: &str, maximum: u64) -> Value {
    match cluster {
        "genOnOff" => json!([{
            "attribute": "onOff",
            "minimumReportInterval": 0,
            "maximumReportInterval": maximum,
            "reportableChange": 0,
        }]),
        "genLevelCtrl" => json!([{
            "attribute": "currentLevel",
            "minimumReportInterval": 5,
            "maximumReportInterval": maximum,
            "reportableChange": 1,
        }]),
        "lightingColorCtrl" => json!([
            {
                "attribute": "colorTemperature",
                "minimumReportInterval": 5,
                "maximumReportInterval": maximum,
                "reportableChange": 1,
            },
            {
                "attribute": "currentX",
                "minimumReportInterval": 5,
                "maximumReportInterval": maximum,
                "reportableChange": 1,
            },
            {
                "attribute": "currentY",
                "minimumReportInterval": 5,
                "maximumReportInterval": maximum,
                "reportableChange": 1,
            },
        ]),
        "closuresWindowCovering" => json!([
            {
                "attribute": "currentPositionLiftPercentage",
                "minimumReportInterval": 5,
                "maximumReportInterval": maximum,
                "reportableChange": 1,
            },
            {
                "attribute": "currentPositionTiltPercentage",
                "minimumReportInterval": 5,
                "maximumReportInterval": maximum,
                "reportableChange": 1,
            },
        ]),
        _ => json!([]),
    }
}

struct Poll {
    cluster: &'static str,
    attributes: Vec<String>,
}

fn matching_polls(cluster: &str, data: &Value) -> Vec<Poll> {
    let level_ctrl = || Poll {
        cluster: "genLevelCtrl",
        attributes: vec!["currentLevel".to_string()],
    };
    let on_off = || Poll {
        cluster: "genOnOff",
        attributes: vec!["onOff".to_string()],
    };
    let color = || Poll {
        cluster: "lightingColorCtrl",
        attributes: vec![
            "currentX".to_string(),
            "currentY".to_string(),
            "colorTemperature".to_string(),
        ],
    };

    let mut polls = Vec::new();
    match cluster {
        "manuSpecificPhilips" => {
            let button = data.get("button").and_then(Value::as_u64).unwrap_or(0);
            if button == 2 || button == 3 {
                polls.push(level_ctrl());
            }
            if button == 1 || button == 4 {
                polls.push(on_off());
            }
        }
        "genLevelCtrl" => {
            polls.push(level_ctrl());
            polls.push(on_off());
        }
        "genOnOff" => {
            polls.push(on_off());
        }
        "genScenes" => {
            polls.push(level_ctrl());
            polls.push(on_off());
            polls.push(color());
        }
        _ => {}
    }
    polls.retain(|poll| is_report_cluster(poll.cluster) || poll.cluster == "genOnOff");
    polls
}
