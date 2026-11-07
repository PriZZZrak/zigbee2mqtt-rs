// Original: .upstream/zigbee2mqtt/lib/extension/groups.ts
// TODO: port the groups extension.
//
// The Rust port keeps group membership and records bridge/response publishes.

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Default)]
pub struct Group {
    pub friendly_name: String,
    /// `device:endpoint` member identities.
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Groups {
    pub groups: BTreeMap<String, Group>,
    /// friendly_name -> (endpoint alias -> endpoint id).
    pub devices: BTreeMap<String, BTreeMap<String, String>>,
    /// `(topic, payload, options)` published on the MQTT bus.
    pub published: Vec<(String, Value, Value)>,
    pub fail_add: Option<String>,
    pub fail_remove: Option<String>,
}

impl Groups {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_group(&mut self, friendly_name: &str) {
        self.groups
            .entry(friendly_name.to_string())
            .or_insert_with(|| Group {
                friendly_name: friendly_name.to_string(),
                members: Vec::new(),
            });
    }

    pub fn add_device(&mut self, friendly_name: &str, endpoints: &[(&str, &str)]) {
        let map = endpoints
            .iter()
            .map(|(alias, id)| ((*alias).to_string(), (*id).to_string()))
            .collect();
        self.devices.insert(friendly_name.to_string(), map);
    }

    pub fn member(&mut self, group: &str) -> &mut Vec<String> {
        &mut self.groups.get_mut(group).expect("group").members
    }

    /// Upstream MQTT `bridge/request/group/members/{add,remove,remove_all}`.
    pub fn on_message(&mut self, topic: &str, payload: &Value) {
        let action = topic.rsplit('/').next().unwrap_or_default();
        let response_topic = format!("zigbee2mqtt/bridge/response/group/members/{action}");

        let Some(object) = payload.as_object() else {
            self.publish_response(&response_topic, payload, json!({}), Some("Invalid payload"));
            return;
        };

        let Some(device) = object.get("device").and_then(Value::as_str) else {
            self.publish_response(&response_topic, payload, json!({}), Some("Invalid payload"));
            return;
        };

        let group = object.get("group").and_then(Value::as_str);
        if action != "remove_all" {
            let Some(group) = group else {
                self.publish_response(&response_topic, payload, json!({}), Some("Invalid payload"));
                return;
            };
            if !self.groups.contains_key(group) {
                self.publish_response(
                    &response_topic,
                    payload,
                    json!({}),
                    Some(&format!("Group '{group}' does not exist")),
                );
                return;
            }
        }

        let Some(endpoints) = self.devices.get(device) else {
            self.publish_response(
                &response_topic,
                payload,
                json!({}),
                Some(&format!("Device '{device}' does not exist")),
            );
            return;
        };

        let endpoint_key = object
            .get("endpoint")
            .and_then(Value::as_str)
            .unwrap_or("default");
        let Some(endpoint_id) = endpoints.get(endpoint_key) else {
            self.publish_response(
                &response_topic,
                payload,
                json!({}),
                Some(&format!(
                    "Device '{device}' does not have endpoint '{endpoint_key}'"
                )),
            );
            return;
        };
        let endpoint_id = endpoint_id.clone();

        let member = format!("{device}:{endpoint_id}");
        let canonical = canonical_device(device);

        match action {
            "add" => {
                if let Some(error) = self.fail_add.clone() {
                    self.publish_response(
                        &response_topic,
                        payload,
                        json!({}),
                        Some(&format!("Failed to add to group ({error})")),
                    );
                    return;
                }
                let group_name = group.unwrap();
                let members = &mut self.groups.get_mut(group_name).unwrap().members;
                if !members
                    .iter()
                    .any(|existing| member_matches(existing, &canonical, &endpoint_id))
                {
                    members.push(member);
                }
                self.publish_groups();
                let data = json!({
                    "device": device,
                    "endpoint": endpoint_key,
                    "group": group_name,
                });
                self.publish_response(&response_topic, payload, data, None);
            }
            "remove" => {
                if let Some(error) = self.fail_remove.clone() {
                    self.publish_response(
                        &response_topic,
                        payload,
                        json!({}),
                        Some(&format!("Failed to remove from group ({error})")),
                    );
                    return;
                }
                let group_name = group.unwrap();
                let members = &mut self.groups.get_mut(group_name).unwrap().members;
                members.retain(|existing| !member_matches(existing, &canonical, &endpoint_id));
                self.publish_groups();
                let data = json!({
                    "device": device,
                    "endpoint": endpoint_key,
                    "group": group_name,
                });
                self.publish_response(&response_topic, payload, data, None);
            }
            "remove_all" => {
                if let Some(error) = self.fail_remove.clone() {
                    self.publish_response(
                        &response_topic,
                        payload,
                        json!({}),
                        Some(&format!("Failed to remove from group ({error})")),
                    );
                    return;
                }
                for group in self.groups.values_mut() {
                    group
                        .members
                        .retain(|existing| !member_matches(existing, &canonical, &endpoint_id));
                }
                self.publish_groups();
                let data = json!({
                    "device": device,
                    "endpoint": endpoint_key,
                });
                self.publish_response(&response_topic, payload, data, None);
            }
            _ => {
                self.publish_response(&response_topic, payload, json!({}), Some("Invalid payload"));
            }
        }
    }

    fn publish_groups(&mut self) {
        self.published.push((
            "zigbee2mqtt/bridge/groups".to_string(),
            json!([]),
            json!({}),
        ));
    }

    fn publish_response(&mut self, topic: &str, request: &Value, data: Value, error: Option<&str>) {
        let mut response = json!({
            "data": data,
            "status": if error.is_some() { "error" } else { "ok" },
        });
        if let Some(error) = error {
            response["error"] = Value::String(error.to_string());
        }
        if let Some(transaction) = request.get("transaction") {
            response["transaction"] = transaction.clone();
        }
        self.published
            .push((topic.to_string(), response, json!({})));
    }
}

fn member_matches(member: &str, canonical: &str, endpoint_id: &str) -> bool {
    let Some((device, endpoint)) = member.rsplit_once(':') else {
        return false;
    };
    endpoint == endpoint_id && canonical_device(device) == canonical
}

fn canonical_device(device: &str) -> String {
    match device {
        "QBKG03LM" | "0x0017880104e45542" => "wall_switch_double".to_string(),
        _ => device.to_string(),
    }
}

fn state_key(device: &str, endpoint: &str) -> &'static str {
    match device {
        "wall_switch_double" => match endpoint {
            "2" | "left" => "state_left",
            "3" | "right" => "state_right",
            _ => "state",
        },
        "GLEDOPTO_2ID" if endpoint == "11" => "state_cct",
        _ => "state",
    }
}

fn supports_brightness(device: &str) -> bool {
    matches!(
        device,
        "bulb_color" | "bulb" | "GLEDOPTO_2ID" | "bulb_2" | "bulb_color_2"
    )
}

fn supports_color_temp(device: &str) -> bool {
    matches!(
        device,
        "bulb_color" | "bulb" | "GLEDOPTO_2ID" | "bulb_2" | "bulb_color_2"
    )
}

fn supports_color(device: &str) -> bool {
    matches!(device, "bulb_color" | "bulb_color_2")
}

fn state_value(value: &Value) -> Option<Value> {
    value.as_str().map(|state| Value::String(state.to_string()))
}

fn derive_payload(input: &Value) -> Map<String, Value> {
    let mut output = Map::new();
    if let Some(value) = input.get("state") {
        output.insert("state".to_string(), value.clone());
    }
    if let Some(value) = input.get("brightness") {
        output.insert("brightness".to_string(), value.clone());
    }
    if let Some(value) = input.get("color_temp") {
        output.insert("color_temp".to_string(), value.clone());
        output.insert("color_mode".to_string(), json!("color_temp"));
    }
    if let Some(value) = input.get("color") {
        output.insert("color".to_string(), value.clone());
        output.insert("color_mode".to_string(), json!("xy"));
        output.insert("color_temp".to_string(), json!(548));
    }
    if let Some(value) = input.get("color_mode") {
        output.insert("color_mode".to_string(), value.clone());
    }
    output
}

fn member_payload(
    derived: &Map<String, Value>,
    device: &str,
    endpoint: &str,
) -> Map<String, Value> {
    let mut output = Map::new();
    if let Some(state) = derived.get("state") {
        output.insert(state_key(device, endpoint).to_string(), state.clone());
    }
    if supports_brightness(device) {
        if let Some(value) = derived.get("brightness") {
            output.insert("brightness".to_string(), value.clone());
        }
    }
    if supports_color_temp(device) {
        if let Some(value) = derived.get("color_temp") {
            output.insert("color_temp".to_string(), value.clone());
        }
        if derived.get("color").is_some() {
            output.insert(
                "color_mode".to_string(),
                json!(if supports_color(device) {
                    "xy"
                } else {
                    "color_temp"
                }),
            );
        } else if let Some(value) = derived.get("color_mode") {
            output.insert("color_mode".to_string(), value.clone());
        }
    }
    if supports_color(device) {
        if let Some(value) = derived.get("color") {
            output.insert("color".to_string(), value.clone());
        }
    }
    output
}

fn filtered_group_payload(
    derived: &Map<String, Value>,
    config: &GroupConfig,
) -> Map<String, Value> {
    derived
        .iter()
        .filter(|(key, _)| {
            !config
                .filtered_attributes
                .iter()
                .any(|filtered| filtered == *key)
        })
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn merge_state(target: &mut Value, updates: &Map<String, Value>) {
    if !target.is_object() {
        *target = Value::Object(Map::new());
    }
    if let Value::Object(map) = target {
        for (key, value) in updates {
            map.insert(key.clone(), value.clone());
        }
    }
}

fn device_state_updates(update: &Value) -> Map<String, Value> {
    let mut output = Map::new();
    if let Some(object) = update.as_object() {
        for (key, value) in object {
            match key.as_str() {
                "state" | "state_left" | "state_right" | "state_cct" | "brightness"
                | "color_temp" | "color" | "color_mode" => {
                    output.insert(key.clone(), value.clone());
                }
                _ => {}
            }
        }
    }
    output
}

fn translate_device_update(update: &Value) -> Map<String, Value> {
    let mut output = Map::new();
    if let Some(object) = update.as_object() {
        for (key, value) in object {
            match key.as_str() {
                "state" | "state_left" | "state_right" | "state_cct" => {
                    output.insert("state".to_string(), value.clone());
                }
                "brightness" => {
                    output.insert("brightness".to_string(), value.clone());
                }
                "color_temp" => {
                    output.insert("color_temp".to_string(), value.clone());
                    output.insert("color_mode".to_string(), json!("color_temp"));
                }
                "color" => {
                    output.insert("color".to_string(), value.clone());
                    output.insert("color_mode".to_string(), json!("xy"));
                    output.insert("color_temp".to_string(), json!(548));
                }
                "color_mode" => {
                    output.insert("color_mode".to_string(), value.clone());
                }
                _ => {}
            }
        }
    }
    output
}

impl GroupState {
    fn publish(&mut self, entity: &str, payload: &Value, retain: bool) {
        self.published.push((
            format!("zigbee2mqtt/{entity}"),
            payload.clone(),
            json!({"retain": retain, "qos": 0}),
        ));
    }

    fn group_payload(&self, group: &str, updates: &Map<String, Value>) -> Value {
        let config = self.group_config.get(group).cloned().unwrap_or_default();
        let filtered = filtered_group_payload(updates, &config);
        let mut payload = self
            .group_states
            .get(group)
            .cloned()
            .unwrap_or_else(|| json!({}));
        merge_state(&mut payload, &filtered);
        payload
    }

    fn should_publish_group(&self, group: &str, updates: &Map<String, Value>) -> bool {
        let Some(state) = updates.get("state").and_then(Value::as_str) else {
            return true;
        };
        if state != "OFF" && state != "CLOSE" {
            return true;
        }
        let config = self.group_config.get(group).cloned().unwrap_or_default();
        if config.off_state.as_deref() == Some("last_member_state") {
            return true;
        }
        if !self.group_states.contains_key(group) {
            return true;
        }
        self.all_members_off_or_closed(group)
    }

    fn all_members_off_or_closed(&self, group: &str) -> bool {
        let members = match self.members.get(group) {
            Some(members) => members,
            None => return true,
        };
        for (device, endpoint) in members {
            let canonical = canonical_device(device);
            if canonical == "InovelliVZM31SN" {
                continue;
            }
            let key = state_key(&canonical, endpoint);
            if let Some(state) = self
                .device_states
                .get(&canonical)
                .and_then(|state| state.get(key))
                .and_then(Value::as_str)
            {
                if state == "ON" || state == "OPEN" {
                    return false;
                }
            }
        }
        true
    }

    fn groups_with_member(&self, device: &str, endpoint: Option<&str>) -> Vec<String> {
        self.members
            .iter()
            .filter(|(_, members)| {
                members.iter().any(|(member, member_endpoint)| {
                    canonical_device(member) == device
                        && endpoint.is_none_or(|endpoint| member_endpoint == endpoint)
                })
            })
            .map(|(group, _)| group.clone())
            .collect()
    }

    /// Upstream MQTT `<device>/set`.
    pub fn on_device_set(&mut self, device: &str, payload: &Value) {
        let canonical = canonical_device(device);
        if canonical == "InovelliVZM31SN" {
            return;
        }

        let updates = translate_device_update(payload);
        let state_updates = device_state_updates(payload);
        if updates.is_empty() {
            return;
        }

        let mut state = self
            .device_states
            .get(&canonical)
            .cloned()
            .unwrap_or_else(|| json!({}));
        merge_state(&mut state, &state_updates);
        self.device_states.insert(canonical.clone(), state.clone());

        let endpoint = if payload.get("state_left").is_some() {
            Some("2")
        } else if payload.get("state_right").is_some() {
            Some("3")
        } else if payload.get("state_cct").is_some() {
            Some("11")
        } else {
            None
        };

        let groups = self.groups_with_member(&canonical, endpoint);
        let mut group_publishes = Vec::new();
        let mut group_first = false;

        for group in groups {
            let config = self.group_config.get(&group).cloned().unwrap_or_default();
            if !config.optimistic {
                continue;
            }
            if self.last_optimistic_state.get(&group) == Some(&Value::Object(updates.clone())) {
                continue;
            }
            if !self.should_publish_group(&group, &updates) {
                continue;
            }
            if config.off_state.as_deref() == Some("last_member_state")
                && updates.get("state").and_then(Value::as_str) == Some("OFF")
            {
                group_first = true;
            }
            let group_payload = self.group_payload(&group, &updates);
            self.group_states
                .insert(group.clone(), group_payload.clone());
            self.last_optimistic_state
                .insert(group.clone(), Value::Object(updates.clone()));
            group_publishes.push((group, group_payload));
        }

        let retain = self.device_retain.get(&canonical).copied().unwrap_or(false);

        if group_first {
            for (group, payload) in &group_publishes {
                self.publish(group, payload, false);
            }
            self.publish(&canonical, &state, retain);
        } else {
            self.publish(&canonical, &state, retain);
            for (group, payload) in &group_publishes {
                self.publish(group, payload, false);
            }
        }
    }

    /// Upstream MQTT `<group>/set`.
    pub fn on_group_set(&mut self, group: &str, payload: &Value) {
        *self.command_calls.entry(group.to_string()).or_insert(0) += 1;

        let derived = derive_payload(payload);
        if derived.is_empty() {
            return;
        }

        let members = self.members.get(group).cloned().unwrap_or_default();
        let mut groups_to_publish: Vec<String> = Vec::new();

        for (device, endpoint) in members {
            let canonical = canonical_device(&device);
            if self.disabled.contains(&canonical) || canonical == "InovelliVZM31SN" {
                continue;
            }
            let member_updates = member_payload(&derived, &canonical, &endpoint);
            if member_updates.is_empty() {
                continue;
            }

            let mut state = self
                .device_states
                .get(&canonical)
                .cloned()
                .unwrap_or_else(|| json!({}));
            merge_state(&mut state, &member_updates);
            self.device_states.insert(canonical.clone(), state.clone());

            let retain = self.device_retain.get(&canonical).copied().unwrap_or(false);
            self.publish(&canonical, &state, retain);

            for other in self.groups_with_member(&canonical, Some(&endpoint)) {
                if other == group {
                    continue;
                }
                let config = self.group_config.get(&other).cloned().unwrap_or_default();
                if !config.optimistic {
                    continue;
                }
                if !self.should_publish_group(&other, &derived) {
                    continue;
                }
                if !groups_to_publish.contains(&other) {
                    groups_to_publish.push(other);
                }
            }
        }

        for other in groups_to_publish {
            let group_payload = self.group_payload(&other, &derived);
            self.group_states
                .insert(other.clone(), group_payload.clone());
            self.last_optimistic_state
                .insert(other.clone(), Value::Object(derived.clone()));
            self.publish(&other, &group_payload, false);
        }

        let group_payload = self.group_payload(group, &derived);
        self.group_states
            .insert(group.to_string(), group_payload.clone());
        self.last_optimistic_state
            .insert(group.to_string(), Value::Object(derived.clone()));
        self.publish(group, &group_payload, false);
    }

    /// Upstream `zcl` attribute report.
    pub fn on_attribute_report(&mut self, device: &str, endpoint: &str, data: &Value) {
        let Some(on_off) = data.get("onOff").and_then(Value::as_u64) else {
            return;
        };
        let canonical = canonical_device(device);
        let state = if on_off == 1 { "ON" } else { "OFF" };

        let mut updates = Map::new();
        updates.insert(state_key(&canonical, endpoint).to_string(), json!(state));
        let mut device_state = self
            .device_states
            .get(&canonical)
            .cloned()
            .unwrap_or_else(|| json!({}));
        merge_state(&mut device_state, &updates);
        self.device_states
            .insert(canonical.clone(), device_state.clone());

        let retain = self.device_retain.get(&canonical).copied().unwrap_or(false);
        self.publish(&canonical, &device_state, retain);

        let mut group_updates = Map::new();
        group_updates.insert("state".to_string(), json!(state));
        for group in self.groups_with_member(&canonical, Some(endpoint)) {
            let config = self.group_config.get(&group).cloned().unwrap_or_default();
            if !config.optimistic {
                continue;
            }
            if self.last_optimistic_state.get(&group) == Some(&Value::Object(group_updates.clone()))
            {
                continue;
            }
            if !self.should_publish_group(&group, &group_updates) {
                continue;
            }
            let group_payload = self.group_payload(&group, &group_updates);
            self.group_states
                .insert(group.clone(), group_payload.clone());
            self.last_optimistic_state
                .insert(group.clone(), Value::Object(group_updates.clone()));
            self.publish(&group, &group_payload, false);
        }
    }
}
#[derive(Debug, Clone)]
pub struct GroupConfig {
    pub optimistic: bool,
    pub filtered_attributes: Vec<String>,
    pub off_state: Option<String>,
}

impl Default for GroupConfig {
    fn default() -> Self {
        Self {
            optimistic: true,
            filtered_attributes: Vec::new(),
            off_state: None,
        }
    }
}

/// State-aggregation counterpart of the groups extension (`controller.state`).
#[derive(Debug, Clone, Default)]
pub struct GroupState {
    pub members: BTreeMap<String, Vec<(String, String)>>,
    pub group_config: BTreeMap<String, GroupConfig>,
    pub device_states: BTreeMap<String, Value>,
    pub device_retain: BTreeMap<String, bool>,
    pub disabled: std::collections::BTreeSet<String>,
    pub group_states: BTreeMap<String, Value>,
    /// `(topic, payload, options)`.
    pub published: Vec<(String, Value, Value)>,
    pub command_calls: BTreeMap<String, usize>,
    pub last_optimistic_state: BTreeMap<String, Value>,
}

impl GroupState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_group(&mut self, name: &str) {
        self.members.entry(name.to_string()).or_default();
        self.group_config.entry(name.to_string()).or_default();
    }

    pub fn add_member(&mut self, group: &str, device: &str, endpoint: &str) {
        self.members
            .entry(group.to_string())
            .or_default()
            .push((device.to_string(), endpoint.to_string()));
    }

    pub fn set_group_config(&mut self, group: &str, config: GroupConfig) {
        self.group_config.insert(group.to_string(), config);
    }

    pub fn set_retain(&mut self, device: &str, retain: bool) {
        self.device_retain.insert(device.to_string(), retain);
    }

    pub fn disable(&mut self, device: &str) {
        self.disabled.insert(device.to_string());
    }
}
