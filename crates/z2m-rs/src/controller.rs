// Original: .upstream/zigbee2mqtt/lib/controller.ts
// TODO: port controller startup, MQTT, state persistence and extension lifecycle.

use std::collections::HashMap;

use serde_json::{json, Value};

/// Device resolved by the controller.
#[derive(Debug, Clone)]
pub struct ControllerDevice {
    pub ieee_addr: String,
    pub friendly_name: String,
    pub supported: bool,
    pub disabled: bool,
    pub removed: bool,
}

/// Model of `lib/controller.ts` used by the ported tests.
#[derive(Debug)]
pub struct Controller {
    pub started: bool,
    pub stopped: bool,
    pub settings: Value,
    pub network: Value,
    pub mqtt_settings: Value,
    pub mqtt_connected: bool,
    pub mqtt_reconnect: usize,
    pub mqtt_error: Option<String>,
    pub published: Vec<(String, Value, Value)>,
    pub logs: Vec<(String, String)>,
    pub extensions: Vec<String>,
    pub devices: Vec<ControllerDevice>,
    pub state: HashMap<String, Value>,
    pub blocklist: Vec<String>,
    pub passlist: Vec<String>,
    pub retained_messages: Vec<(String, Value)>,
    pub retained_republished: usize,
    pub cache_state: bool,
    pub force_disable_retain: bool,
    pub restart_requested: bool,
    pub exit_code: Option<i32>,
    pub sd_notify: Vec<String>,
    pub signal_handlers: Vec<String>,
    pub startup_aborted: bool,
    pub state_corrupted: bool,
    pub network_generated: bool,
    pub publish_last_seen: bool,
    pub last_seen_logs: Vec<Value>,
    pub publish_attributes: bool,
    pub publish_json: bool,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            started: false,
            stopped: false,
            settings: json!({}),
            network: json!({}),
            mqtt_settings: json!({"server": "mqtt://localhost"}),
            mqtt_connected: false,
            mqtt_reconnect: 0,
            mqtt_error: None,
            published: Vec::new(),
            logs: Vec::new(),
            extensions: Vec::new(),
            devices: Vec::new(),
            state: HashMap::new(),
            blocklist: Vec::new(),
            passlist: Vec::new(),
            retained_messages: Vec::new(),
            retained_republished: 0,
            cache_state: true,
            force_disable_retain: false,
            restart_requested: false,
            exit_code: None,
            sd_notify: Vec::new(),
            signal_handlers: Vec::new(),
            startup_aborted: false,
            state_corrupted: false,
            network_generated: false,
            publish_last_seen: false,
            last_seen_logs: Vec::new(),
            publish_attributes: false,
            publish_json: false,
        }
    }
}

impl Controller {
    pub fn new(settings: Value) -> Self {
        let mut controller = Self::default();
        controller.cache_state = settings["advanced"]["cache_state"]
            .as_bool()
            .unwrap_or(true);
        controller.force_disable_retain = settings["advanced"]["force_disable_retain"]
            .as_bool()
            .unwrap_or(false);
        controller.settings = settings;
        controller
    }

    /// Upstream `start()`.
    pub async fn start(&mut self) -> Result<(), String> {
        todo!()
    }

    /// Upstream `stop()`.
    pub async fn stop(&mut self) {
        todo!()
    }

    /// Upstream `addExtension(extension)`.
    pub fn add_extension(&mut self, name: &str) {
        self.extensions.push(name.to_string());
    }

    /// Upstream `getExtension(name)`.
    pub fn get_extension(&self, name: &str) -> Option<&String> {
        self.extensions.iter().find(|extension| *extension == name)
    }

    /// Upstream `removeExtension(extension)`.
    pub fn remove_extension(&mut self, name: &str) {
        self.extensions.retain(|extension| extension != name);
    }

    /// Upstream `acceptJoiningDeviceHandler`.
    pub fn accept_joining_device(&self, ieee_addr: &str) -> bool {
        if self.blocklist.iter().any(|entry| entry == ieee_addr) {
            return false;
        }
        if self.passlist.is_empty() {
            return true;
        }
        self.passlist.iter().any(|entry| entry == ieee_addr)
    }

    /// Upstream ZHE event handler.
    pub async fn on_zhe_event(&mut self, _event: Value) {
        todo!()
    }

    /// Upstream `publishEntityState`.
    pub async fn publish_entity_state(&mut self, _id: &str, _state: Value) {
        todo!()
    }

    /// Upstream MQTT message handler.
    pub async fn handle_mqtt_message(&mut self, _topic: &str, _payload: Value) {
        todo!()
    }

    /// Upstream MQTT `connect` handler.
    pub async fn on_mqtt_connect(&mut self, _republish: bool) {
        todo!()
    }

    /// Upstream MQTT `reconnect` handler.
    pub async fn on_mqtt_reconnect(&mut self) {
        todo!()
    }

    /// Upstream state helpers.
    pub fn load_state(&mut self) {
        todo!()
    }

    pub fn save_state(&mut self) {
        todo!()
    }

    /// Upstream `resolveEntity` / device lookup.
    pub fn resolve_device(&self, id: &str) -> Option<&ControllerDevice> {
        self.devices
            .iter()
            .find(|device| device.ieee_addr == id || device.friendly_name == id)
    }

    /// Test helper: `log(level, message)`.
    pub fn log(&mut self, level: &str, message: &str) {
        self.logs.push((level.to_string(), message.to_string()));
    }

    pub fn has_publish(&self, topic: &str, payload: Value) -> bool {
        self.published
            .iter()
            .any(|(published_topic, published_payload, _)| {
                published_topic == topic && *published_payload == payload
            })
    }

    pub fn published_to(&self, topic: &str) -> Vec<&Value> {
        self.published
            .iter()
            .filter(|(published_topic, _, _)| published_topic == topic)
            .map(|(_, payload, _)| payload)
            .collect()
    }

    pub fn log_contains(&self, level: &str, needle: &str) -> bool {
        self.logs
            .iter()
            .any(|(log_level, message)| log_level == level && message.contains(needle))
    }
}
