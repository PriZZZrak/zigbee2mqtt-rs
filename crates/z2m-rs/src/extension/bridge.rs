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
        todo!()
    }

    /// Upstream `stop()`.
    pub async fn stop(&mut self) {
        todo!()
    }

    /// Upstream `onMQTTMessage()`.
    pub async fn handle_message(&mut self, _topic: &str, _payload: Value) {
        todo!()
    }

    /// Upstream ZHE event handler.
    pub async fn on_zhe_event(&mut self, _event: Value) {
        todo!()
    }

    /// Upstream `bridge/request/options`.
    pub async fn change_options(&mut self, _payload: Value) {
        todo!()
    }

    /// Upstream `bridge/request/permit_join`.
    pub async fn permit_join(&mut self, _payload: Value) {
        todo!()
    }

    /// Upstream `bridge/request/device/rename`.
    pub async fn rename_device(&mut self, _payload: Value) {
        todo!()
    }

    /// Upstream `bridge/request/device/remove`.
    pub async fn remove_device(&mut self, _payload: Value) {
        todo!()
    }

    /// Upstream `bridge/request/device/interview`.
    pub async fn interview_device(&mut self, _payload: Value) {
        todo!()
    }

    /// Upstream `bridge/request/device/options`.
    pub async fn change_device_options(&mut self, _payload: Value) {
        todo!()
    }

    /// Upstream `bridge/request/group/add`, `rename`, `remove`, `options`.
    pub async fn group_request(&mut self, _action: &str, _payload: Value) {
        todo!()
    }

    /// Upstream `bridge/request/health_check`.
    pub async fn health_check(&mut self, _payload: Value) {
        todo!()
    }

    /// Upstream `bridge/request/coordinator_check`.
    pub async fn coordinator_check(&mut self, _payload: Value) {
        todo!()
    }

    /// Upstream `bridge/request/backup`.
    pub async fn backup(&mut self, _payload: Value) {
        todo!()
    }

    /// Upstream `bridge/request/touchlink/*`.
    pub async fn touchlink(&mut self, _action: &str, _payload: Value) {
        todo!()
    }

    /// Upstream `bridge/request/device/install_code/add`.
    pub async fn add_install_code(&mut self, _payload: Value) {
        todo!()
    }

    /// Upstream `bridge/request/device/configure_reporting`.
    pub async fn configure_reporting(&mut self, _payload: Value) {
        todo!()
    }

    /// Upstream `bridge/request/device/read_reporting_config`.
    pub async fn read_reporting_config(&mut self, _payload: Value) {
        todo!()
    }

    /// Upstream `bridge/request/device/generate_external_definition`.
    pub async fn generate_external_definition(&mut self, _payload: Value) {
        todo!()
    }

    /// Upstream `bridge/request/device/ota_update/*` pass-through.
    pub async fn ota_request(&mut self, _action: &str, _payload: Value) {
        todo!()
    }

    /// Upstream `bridge/request/restart`.
    pub async fn restart(&mut self, _payload: Value) {
        todo!()
    }

    /// Upstream MQTT log publishing.
    pub async fn log(&mut self, _level: &str, _message: &str) {
        todo!()
    }

    /// Upstream `trigger` action helper.
    pub fn trigger_action(&mut self, action: Value) {
        self.triggered_actions.push(action);
    }

    /// Upstream `publishInfo`.
    pub async fn publish_info(&mut self) {
        todo!()
    }

    /// Upstream `publishDevices`.
    pub async fn publish_devices(&mut self) {
        todo!()
    }

    /// Upstream `publishDefinitions`.
    pub async fn publish_definitions(&mut self) {
        todo!()
    }

    /// Upstream `publishGroups`.
    pub async fn publish_groups(&mut self) {
        todo!()
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
