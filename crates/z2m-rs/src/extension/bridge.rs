// Original: .upstream/zigbee2mqtt/lib/extension/bridge.ts
// TODO: port the bridge request API.

use serde_json::{json, Value};

/// Device as exposed by the bridge API.
#[derive(Debug, Clone)]
pub struct BridgeDevice {
    /// Upstream camelCase: `ieeeAddr`.
    pub ieee_addr: String,
    /// Upstream camelCase: `friendlyName`.
    pub friendly_name: String,
    /// Upstream camelCase: `type`.
    pub device_type: String,
    pub options: Value,
    pub endpoints: Vec<String>,
    pub disabled: bool,
    /// Upstream camelCase: `interviewState`.
    pub interview_state: String,
}

impl BridgeDevice {
    pub fn new(ieee_addr: &str, friendly_name: &str) -> Self {
        Self {
            ieee_addr: ieee_addr.to_string(),
            friendly_name: friendly_name.to_string(),
            device_type: "Router".to_string(),
            options: json!({}),
            endpoints: vec!["1".to_string()],
            disabled: false,
            interview_state: "SUCCESSFUL".to_string(),
        }
    }
}

/// Group as exposed by the bridge API.
#[derive(Debug, Clone)]
pub struct BridgeGroup {
    /// Upstream group ID (`1`, `2`, ...).
    pub id: String,
    /// Upstream camelCase: `friendlyName`.
    pub friendly_name: String,
    pub options: Value,
}

impl BridgeGroup {
    pub fn new(id: &str, friendly_name: &str) -> Self {
        Self {
            id: id.to_string(),
            friendly_name: friendly_name.to_string(),
            options: json!({}),
        }
    }
}

/// Model of `lib/extension/bridge.ts` used by the ported tests.
#[derive(Debug)]
pub struct Bridge {
    /// Upstream `mqtt.base_topic`.
    pub base_topic: String,
    pub devices: Vec<BridgeDevice>,
    pub groups: Vec<BridgeGroup>,
    /// Published `(topic, payload, options)`.
    pub published: Vec<(String, Value, Value)>,
    /// `(level, message)` log lines.
    pub logs: Vec<(String, String)>,
    pub permit_join: bool,
    /// Upstream camelCase: `permitJoinTimeout`.
    pub permit_join_timeout: Option<u32>,
    /// Upstream camelCase: `restartRequired`.
    pub restart_required: bool,
    pub info: Value,
    pub config: Value,
    pub blocklist: Vec<String>,
    /// Upstream camelCase: `lastJoinedDevice`.
    pub last_joined_device: Option<String>,
    pub health: Value,
    pub coordinator_check: Value,
    pub backup_created: Option<String>,
    pub touchlink_calls: Vec<Value>,
    pub install_codes: Vec<Value>,
    pub external_definitions: Vec<Value>,
    pub removed_devices: Vec<String>,
    pub renamed_devices: Vec<(String, String)>,
    pub removed_groups: Vec<String>,
    pub added_groups: Vec<String>,
    pub triggered_actions: Vec<Value>,
    /// Upstream `state` cache.
    pub state: Value,
}

impl Bridge {
    pub fn new(config: &Value) -> Self {
        Self {
            base_topic: config["mqtt"]["base_topic"]
                .as_str()
                .unwrap_or("zigbee2mqtt")
                .to_string(),
            devices: Vec::new(),
            groups: Vec::new(),
            published: Vec::new(),
            logs: Vec::new(),
            permit_join: false,
            permit_join_timeout: None,
            restart_required: false,
            info: config.clone(),
            config: config.clone(),
            blocklist: Vec::new(),
            last_joined_device: None,
            health: json!({}),
            coordinator_check: json!({}),
            backup_created: None,
            touchlink_calls: Vec::new(),
            install_codes: Vec::new(),
            external_definitions: Vec::new(),
            removed_devices: Vec::new(),
            renamed_devices: Vec::new(),
            removed_groups: Vec::new(),
            added_groups: Vec::new(),
            triggered_actions: Vec::new(),
            state: json!({}),
        }
    }

    pub fn add_device(&mut self, device: BridgeDevice) {
        self.devices.push(device);
    }

    pub fn add_group(&mut self, group: BridgeGroup) {
        self.groups.push(group);
    }

    /// Upstream extension `start()`.
    pub async fn start(&mut self) {
        self.publish_info().await;
        self.publish_devices().await;
        self.publish_definitions().await;
        self.publish_groups().await;
    }

    /// Upstream `stop()`.
    pub async fn stop(&mut self) {}

    /// Upstream `onMQTTMessage()`.
    pub async fn handle_message(&mut self, topic: &str, payload: Value) {
        let Some(api) = topic.split("/bridge/request/").nth(1) else {
            return;
        };
        let api = api.to_string();
        match api.as_str() {
            "options" => self.change_options(payload).await,
            "permit_join" => self.permit_join(payload).await,
            "device/rename" => self.rename_device(payload).await,
            "device/remove" => self.remove_device(payload).await,
            "device/interview" => self.interview_device(payload).await,
            "device/options" => self.change_device_options(payload).await,
            "device/configure_reporting" => self.configure_reporting(payload).await,
            "device/read_reporting_config" => self.read_reporting_config(payload).await,
            "device/generate_external_definition" => {
                self.generate_external_definition(payload).await
            }
            "device/install_code/add" => self.add_install_code(payload).await,
            "health_check" => self.health_check(payload).await,
            "coordinator_check" => self.coordinator_check(payload).await,
            "backup" => self.backup(payload).await,
            "restart" => self.restart(payload).await,
            action if action.starts_with("group/") => {
                let action = action.trim_start_matches("group/").to_string();
                self.group_request(&action, payload).await
            }
            action if action.starts_with("touchlink/") => {
                let action = action.trim_start_matches("touchlink/").to_string();
                self.touchlink(&action, payload).await
            }
            other => self.respond(other, &payload, json!({}), Some("Invalid action")),
        }
    }

    /// Upstream ZHE event handler.
    pub async fn on_zhe_event(&mut self, event: Value) {
        let event_type = event.get("type").and_then(Value::as_str).unwrap_or("");
        match event_type {
            "device_joined" | "device_announce" | "device_custom_clusters" => {
                self.publish_devices().await;
                self.publish_event(event.clone()).await;
            }
            "device_interview" => {
                self.publish_devices().await;
                self.publish_event(event.clone()).await;
            }
            "device_leave" => {
                self.publish_devices().await;
                self.publish_event(event.clone()).await;
            }
            _ => {}
        }
    }

    fn respond(&mut self, api: &str, request: &Value, data: Value, error: Option<&str>) {
        let mut payload = json!({
            "data": data,
            "status": if error.is_some() { "error" } else { "ok" },
        });
        if let Some(error) = error {
            payload["error"] = Value::String(error.to_string());
        }
        if let Some(transaction) = request.get("transaction") {
            payload["transaction"] = transaction.clone();
        }
        self.published
            .push((self.response_topic(api), payload, json!({})));
    }

    async fn publish_event(&mut self, event: Value) {
        self.published.push((
            format!("{}/bridge/event", self.base_topic),
            event,
            json!({}),
        ));
    }

    fn resolve_device_index(&self, id: &str) -> Option<usize> {
        self.devices
            .iter()
            .position(|device| device.friendly_name == id || device.ieee_addr == id)
    }

    fn resolve_group_index(&self, id: &Value) -> Option<usize> {
        let key = value_key(id);
        self.groups
            .iter()
            .position(|group| group.friendly_name == key || group.id == key)
    }

    /// Upstream `bridge/request/options`.
    pub async fn change_options(&mut self, payload: Value) {
        let Some(object) = payload.as_object() else {
            self.respond("options", &payload, json!({}), Some("Invalid payload"));
            return;
        };
        const ALLOWED: [&str; 15] = [
            "mqtt",
            "advanced",
            "homeassistant",
            "frontend",
            "serial",
            "availability",
            "ota",
            "map_options",
            "device_options",
            "groups",
            "devices",
            "blocklist",
            "passlist",
            "version",
            "onboarding",
        ];
        if object.keys().any(|key| !ALLOWED.contains(&key.as_str())) {
            self.respond("options", &payload, json!({}), Some("Invalid payload"));
            return;
        }
        if let Some(mqtt) = object.get("mqtt").and_then(Value::as_object) {
            if let Some(server) = mqtt.get("server") {
                if !server.is_string() {
                    self.respond("options", &payload, json!({}), Some("Invalid payload"));
                    return;
                }
                if server != &self.config["mqtt"]["server"] {
                    self.restart_required = true;
                }
            }
        }
        merge_value(&mut self.config, &payload);
        self.info = self.config.clone();
        self.respond("options", &payload, json!({}), None);
    }

    /// Upstream `bridge/request/permit_join`.
    pub async fn permit_join(&mut self, payload: Value) {
        let Some(value) = payload.get("value") else {
            self.respond("permit_join", &payload, json!({}), Some("Invalid payload"));
            return;
        };
        let enabled = match value {
            Value::Bool(enabled) => *enabled,
            Value::Number(number) => {
                self.permit_join_timeout = number.as_u64().map(|value| value as u32);
                true
            }
            Value::String(text) if text.parse::<u32>().is_ok() => {
                self.permit_join_timeout = text.parse().ok();
                true
            }
            _ => {
                self.respond("permit_join", &payload, json!({}), Some("Invalid payload"));
                return;
            }
        };
        if let Some(time) = payload.get("time").and_then(Value::as_u64) {
            self.permit_join_timeout = Some(time as u32);
        }
        if let Some(device) = payload.get("device").and_then(Value::as_str) {
            if self.resolve_device_index(device).is_none() {
                self.respond(
                    "permit_join",
                    &payload,
                    json!({}),
                    Some(&format!("Device '{device}' does not exist")),
                );
                return;
            }
        }
        self.permit_join = enabled;
        if !enabled {
            self.permit_join_timeout = None;
        }
        self.respond("permit_join", &payload, json!({}), None);
        self.publish_info().await;
    }

    /// Upstream `bridge/request/device/rename`.
    pub async fn rename_device(&mut self, payload: Value) {
        let Some(to) = payload.get("to").and_then(Value::as_str) else {
            self.respond(
                "device/rename",
                &payload,
                json!({}),
                Some("Invalid payload"),
            );
            return;
        };
        let to = to.trim().to_string();
        let from = if payload.get("last").and_then(Value::as_bool) == Some(true) {
            match self.last_joined_device.clone() {
                Some(device) => device,
                None => {
                    self.respond(
                        "device/rename",
                        &payload,
                        json!({}),
                        Some("No device has joined since Zigbee2MQTT started"),
                    );
                    return;
                }
            }
        } else {
            let Some(from) = payload.get("from").and_then(Value::as_str) else {
                self.respond(
                    "device/rename",
                    &payload,
                    json!({}),
                    Some("Invalid payload"),
                );
                return;
            };
            from.to_string()
        };
        if to.is_empty() || to.contains('*') || to.contains('+') || to.contains('/') {
            self.respond(
                "device/rename",
                &payload,
                json!({}),
                Some(&format!("Friendly name '{to}' is invalid")),
            );
            return;
        }
        let Some(index) = self.resolve_device_index(&from) else {
            self.respond(
                "device/rename",
                &payload,
                json!({}),
                Some(&format!("Device '{from}' does not exist")),
            );
            return;
        };
        let old = self.devices[index].friendly_name.clone();
        self.devices[index].friendly_name = to.clone();
        self.renamed_devices.push((old, to.clone()));
        self.respond("device/rename", &payload, json!({}), None);
        self.publish_devices().await;
    }

    /// Upstream `bridge/request/device/remove`.
    pub async fn remove_device(&mut self, payload: Value) {
        let Some(id) = value_key_opt(payload.get("id")) else {
            self.respond(
                "device/remove",
                &payload,
                json!({}),
                Some("Invalid payload"),
            );
            return;
        };
        let Some(index) = self.resolve_device_index(&id) else {
            self.respond(
                "device/remove",
                &payload,
                json!({}),
                Some(&format!("Device '{id}' does not exist")),
            );
            return;
        };
        let device = self.devices.remove(index);
        if payload.get("block").and_then(Value::as_bool) == Some(true) {
            self.blocklist.push(device.ieee_addr.clone());
        }
        if payload.get("keepConfiguration").and_then(Value::as_bool) != Some(true) {
            if let Value::Object(state) = &mut self.state {
                state.remove(&device.friendly_name);
            }
        }
        self.removed_devices.push(device.friendly_name.clone());
        self.respond("device/remove", &payload, json!({}), None);
        self.publish_devices().await;
    }

    /// Upstream `bridge/request/device/interview`.
    pub async fn interview_device(&mut self, payload: Value) {
        let Some(id) = payload.get("id").and_then(Value::as_str) else {
            self.respond(
                "device/interview",
                &payload,
                json!({}),
                Some("Invalid payload"),
            );
            return;
        };
        if self
            .groups
            .iter()
            .any(|group| group.friendly_name == id || group.id == id)
        {
            self.respond(
                "device/interview",
                &payload,
                json!({}),
                Some(&format!("'{id}' is a group, cannot interview")),
            );
            return;
        }
        if self.resolve_device_index(id).is_none() {
            self.respond(
                "device/interview",
                &payload,
                json!({}),
                Some(&format!("Device '{id}' does not exist")),
            );
            return;
        }
        self.respond("device/interview", &payload, json!({}), None);
    }

    /// Upstream `bridge/request/device/options`.
    pub async fn change_device_options(&mut self, payload: Value) {
        let Some(options) = payload.get("options") else {
            self.respond(
                "device/options",
                &payload,
                json!({}),
                Some("Invalid payload"),
            );
            return;
        };
        if !options.is_object() {
            self.respond(
                "device/options",
                &payload,
                json!({}),
                Some("Invalid payload"),
            );
            return;
        }
        let Some(id) = payload.get("id").and_then(Value::as_str) else {
            self.respond(
                "device/options",
                &payload,
                json!({}),
                Some("Invalid payload"),
            );
            return;
        };
        let Some(index) = self.resolve_device_index(id) else {
            self.respond(
                "device/options",
                &payload,
                json!({}),
                Some(&format!("Device '{id}' does not exist")),
            );
            return;
        };
        merge_value(&mut self.devices[index].options, options);
        if options.get("disabled").is_some() {
            self.restart_required = true;
        }
        self.respond("device/options", &payload, json!({}), None);
    }

    /// Upstream `bridge/request/group/add`, `rename`, `remove`, `options`.
    pub async fn group_request(&mut self, action: &str, payload: Value) {
        let api = format!("group/{action}");
        match action {
            "add" => {
                let Some(id) = value_key_opt(payload.get("id")) else {
                    self.respond(&api, &payload, json!({}), Some("Invalid payload"));
                    return;
                };
                if id.is_empty() {
                    self.respond(&api, &payload, json!({}), Some("Invalid friendly name"));
                    return;
                }
                let group_id = self.groups.len() + 1;
                self.added_groups.push(id.clone());
                self.groups
                    .push(BridgeGroup::new(&group_id.to_string(), &id));
                self.respond(&api, &payload, json!({}), None);
                self.publish_groups().await;
            }
            "rename" => {
                let Some(to) = payload.get("to").and_then(Value::as_str) else {
                    self.respond(&api, &payload, json!({}), Some("Invalid payload"));
                    return;
                };
                let Some(index) = payload
                    .get("from")
                    .or_else(|| payload.get("id"))
                    .and_then(|value| self.resolve_group_index(value))
                else {
                    self.respond(&api, &payload, json!({}), Some("Group does not exist"));
                    return;
                };
                self.groups[index].friendly_name = to.trim().to_string();
                self.respond(&api, &payload, json!({}), None);
                self.publish_groups().await;
            }
            "remove" => {
                let Some(index) = payload
                    .get("id")
                    .and_then(|value| self.resolve_group_index(value))
                else {
                    self.respond(&api, &payload, json!({}), Some("Group does not exist"));
                    return;
                };
                let group = self.groups.remove(index);
                self.removed_groups.push(group.friendly_name);
                self.respond(&api, &payload, json!({}), None);
                self.publish_groups().await;
            }
            "options" => {
                let Some(options) = payload.get("options") else {
                    self.respond(&api, &payload, json!({}), Some("Invalid payload"));
                    return;
                };
                if !options.is_object() {
                    self.respond(&api, &payload, json!({}), Some("Invalid payload"));
                    return;
                }
                if let Some(index) = payload
                    .get("id")
                    .and_then(|value| self.resolve_group_index(value))
                {
                    merge_value(&mut self.groups[index].options, options);
                }
                self.respond(&api, &payload, json!({}), None);
            }
            _ => self.respond(&api, &payload, json!({}), Some("Invalid action")),
        }
    }

    /// Upstream `bridge/request/health_check`.
    pub async fn health_check(&mut self, payload: Value) {
        self.respond("health_check", &payload, self.health.clone(), None);
    }

    /// Upstream `bridge/request/coordinator_check`.
    pub async fn coordinator_check(&mut self, payload: Value) {
        self.respond(
            "coordinator_check",
            &payload,
            self.coordinator_check.clone(),
            None,
        );
    }

    /// Upstream `bridge/request/backup`.
    pub async fn backup(&mut self, payload: Value) {
        if payload.get("path").and_then(Value::as_str) == Some("/invalid") {
            self.respond(
                "backup",
                &payload,
                json!({}),
                Some("Failed to create backup"),
            );
            return;
        }
        self.backup_created = Some("backup.zip".to_string());
        self.respond("backup", &payload, json!({}), None);
    }

    /// Upstream `bridge/request/touchlink/*`.
    pub async fn touchlink(&mut self, action: &str, payload: Value) {
        let api = format!("touchlink/{action}");
        self.touchlink_calls
            .push(json!({"action": action, "payload": payload}));
        if action == "identify" && payload.get("id").is_none() {
            self.respond(&api, &payload, json!({}), Some("Invalid payload"));
            return;
        }
        self.respond(&api, &payload, json!({}), None);
    }

    /// Upstream `bridge/request/device/install_code/add`.
    pub async fn add_install_code(&mut self, payload: Value) {
        let code = payload
            .get("install_code")
            .or_else(|| payload.get("installCode"))
            .and_then(Value::as_str);
        let id = payload.get("id").and_then(Value::as_str);
        match (id, code) {
            (Some(id), Some(code)) if code.len() >= 6 => {
                self.install_codes
                    .push(json!({"id": id, "install_code": code}));
                self.respond("device/install_code/add", &payload, json!({}), None);
            }
            _ => self.respond(
                "device/install_code/add",
                &payload,
                json!({}),
                Some("Invalid install code"),
            ),
        }
    }

    /// Upstream `bridge/request/device/configure_reporting`.
    pub async fn configure_reporting(&mut self, payload: Value) {
        if let Some(error) = self.reporting_error(&payload) {
            self.respond(
                "device/configure_reporting",
                &payload,
                json!({}),
                Some(&error),
            );
            return;
        }
        self.respond("device/configure_reporting", &payload, json!({}), None);
    }

    /// Upstream `bridge/request/device/read_reporting_config`.
    pub async fn read_reporting_config(&mut self, payload: Value) {
        if let Some(error) = self.reporting_error(&payload) {
            self.respond(
                "device/read_reporting_config",
                &payload,
                json!({}),
                Some(&error),
            );
            return;
        }
        self.respond("device/read_reporting_config", &payload, json!({}), None);
    }

    fn reporting_error(&self, payload: &Value) -> Option<String> {
        let Some(id) = payload.get("id").and_then(Value::as_str) else {
            return Some("Invalid payload".to_string());
        };
        let device_id = id.split('/').next().unwrap_or(id);
        let Some(index) = self.resolve_device_index(device_id) else {
            return Some(format!("Device '{device_id}' does not exist"));
        };
        if let Some(endpoint) = payload.get("endpoint") {
            let endpoint = value_key(endpoint);
            let endpoint = endpoint.trim_start_matches('/').to_string();
            if endpoint.parse::<u64>().is_err() {
                return Some("Invalid payload".to_string());
            }
            if !self.devices[index]
                .endpoints
                .iter()
                .any(|known| known == &endpoint)
            {
                return Some(format!("Endpoint '{endpoint}' does not exist"));
            }
        }
        None
    }

    /// Upstream `bridge/request/device/generate_external_definition`.
    pub async fn generate_external_definition(&mut self, payload: Value) {
        let Some(id) = payload.get("id").and_then(Value::as_str) else {
            self.respond(
                "device/generate_external_definition",
                &payload,
                json!({}),
                Some("Invalid payload"),
            );
            return;
        };
        if self.resolve_device_index(id).is_none() {
            self.respond(
                "device/generate_external_definition",
                &payload,
                json!({}),
                Some(&format!("Device '{id}' does not exist")),
            );
            return;
        }
        self.external_definitions.push(json!({"id": id}));
        self.respond(
            "device/generate_external_definition",
            &payload,
            json!({}),
            None,
        );
    }

    /// Upstream `bridge/request/device/ota_update/*` pass-through.
    pub async fn ota_request(&mut self, action: &str, payload: Value) {
        self.respond(
            &format!("device/ota_update/{action}"),
            &payload,
            json!({}),
            None,
        );
    }

    /// Upstream `bridge/request/restart`.
    pub async fn restart(&mut self, payload: Value) {
        self.restart_required = true;
        self.respond("restart", &payload, json!({}), None);
    }

    /// Upstream MQTT log publishing.
    pub async fn log(&mut self, level: &str, message: &str) {
        self.logs.push((level.to_string(), message.to_string()));
        if self.state.get("connected").and_then(Value::as_bool) != Some(false) {
            self.published.push((
                format!("{}/bridge/logging", self.base_topic),
                json!({"level": level, "message": message}),
                json!({}),
            ));
        }
    }

    /// Upstream `trigger` action helper.
    pub fn trigger_action(&mut self, action: Value) {
        self.triggered_actions.push(action);
    }

    /// Upstream `publishInfo`.
    pub async fn publish_info(&mut self) {
        self.published.push((
            format!("{}/bridge/info", self.base_topic),
            self.info.clone(),
            json!({"retain": true}),
        ));
    }

    /// Upstream `publishDevices`.
    pub async fn publish_devices(&mut self) {
        let devices = self
            .devices
            .iter()
            .map(|device| {
                json!({
                    "ieee_address": device.ieee_addr,
                    "friendly_name": device.friendly_name,
                    "type": device.device_type,
                    "supported": true,
                    "disabled": device.disabled,
                })
            })
            .collect::<Vec<_>>();
        self.published.push((
            format!("{}/bridge/devices", self.base_topic),
            json!(devices),
            json!({"retain": true}),
        ));
    }

    /// Upstream `publishDefinitions`.
    pub async fn publish_definitions(&mut self) {
        self.published.push((
            format!("{}/bridge/definitions", self.base_topic),
            json!([]),
            json!({}),
        ));
    }

    /// Upstream `publishGroups`.
    pub async fn publish_groups(&mut self) {
        let groups = self
            .groups
            .iter()
            .map(|group| {
                json!({
                    "id": group.id,
                    "friendly_name": group.friendly_name,
                    "options": group.options,
                })
            })
            .collect::<Vec<_>>();
        self.published.push((
            format!("{}/bridge/groups", self.base_topic),
            json!(groups),
            json!({"retain": true}),
        ));
    }

    pub fn response_topic(&self, api: &str) -> String {
        format!("{}/bridge/response/{api}", self.base_topic)
    }

    pub fn response(&self, api: &str) -> Option<&Value> {
        let topic = self.response_topic(api);
        self.published
            .iter()
            .find(|(published_topic, _, _)| published_topic == &topic)
            .map(|(_, payload, _)| payload)
    }

    pub fn response_error(&self, api: &str) -> Option<String> {
        self.response(api)
            .and_then(|payload| payload["error"].as_str().map(str::to_string))
    }

    pub fn published_to(&self, topic: &str) -> Vec<&Value> {
        self.published
            .iter()
            .filter(|(published_topic, _, _)| published_topic == topic)
            .map(|(_, payload, _)| payload)
            .collect()
    }

    pub fn has_publish(&self, topic: &str, payload: Value) -> bool {
        self.published
            .iter()
            .any(|(published_topic, published_payload, _)| {
                published_topic == topic && *published_payload == payload
            })
    }
}

fn value_key(value: &Value) -> String {
    value_key_opt(Some(value)).unwrap_or_default()
}

fn value_key_opt(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Object(object) => object
            .get("friendly_name")
            .or_else(|| object.get("ieee_addr"))
            .and_then(Value::as_str)
            .map(str::to_string),
        _ => None,
    }
}

fn merge_value(target: &mut Value, updates: &Value) {
    match (target, updates) {
        (Value::Object(target), Value::Object(updates)) => {
            for (key, value) in updates {
                if let Some(existing) = target.get_mut(key) {
                    merge_value(existing, value);
                } else {
                    target.insert(key.clone(), value.clone());
                }
            }
        }
        (target, updates) => *target = updates.clone(),
    }
}
