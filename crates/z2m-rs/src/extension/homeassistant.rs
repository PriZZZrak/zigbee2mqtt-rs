// Original: .upstream/zigbee2mqtt/lib/extension/homeassistant.ts
// TODO: port discovery generation and HA status handling.

use std::collections::HashMap;

use serde_json::{json, Value};

/// Device/group exposed to Home Assistant discovery.
#[derive(Debug, Clone)]
pub struct HaEntity {
    /// Upstream `ieeeAddr` (devices) or group id.
    pub id: String,
    /// Upstream `friendly_name`.
    pub friendly_name: String,
    pub exposes: Vec<Value>,
    pub options: Value,
    pub state: Value,
    pub definition: Value,
    pub disabled: bool,
}

impl HaEntity {
    pub fn device(id: &str, friendly_name: &str) -> Self {
        Self {
            id: id.to_string(),
            friendly_name: friendly_name.to_string(),
            exposes: Vec::new(),
            options: json!({}),
            state: json!({}),
            definition: json!({}),
            disabled: false,
        }
    }

    pub fn group(id: &str, friendly_name: &str) -> Self {
        Self::device(id, friendly_name)
    }
}

/// Model of `lib/extension/homeassistant.ts` used by the ported tests.
#[derive(Debug)]
pub struct HomeAssistant {
    /// Upstream `mqtt.base_topic`.
    pub base_topic: String,
    /// Upstream `homeassistant.discovery_topic`.
    pub discovery_topic: String,
    /// Upstream `homeassistant.status_topic`.
    pub status_topic: String,
    /// Upstream `homeassistant.enabled`.
    pub enabled: bool,
    /// Upstream `advanced.cache_state`.
    pub cache_state: bool,
    /// Upstream `experimental_event_entities`.
    pub experimental_event_entities: bool,
    pub devices: Vec<HaEntity>,
    pub groups: Vec<HaEntity>,
    /// Published `(topic, payload, options)`.
    pub published: Vec<(String, Value, Value)>,
    /// Retained discovery configs keyed by discovery topic.
    pub discovery: HashMap<String, Value>,
    /// Warnings emitted at startup.
    pub warnings: Vec<String>,
    /// Errors emitted at startup.
    pub errors: Vec<String>,
    pub started: bool,
}

impl HomeAssistant {
    pub fn new(config: &Value) -> Self {
        Self {
            base_topic: config["mqtt"]["base_topic"]
                .as_str()
                .unwrap_or("zigbee2mqtt")
                .to_string(),
            discovery_topic: config["homeassistant"]["discovery_topic"]
                .as_str()
                .unwrap_or("homeassistant")
                .to_string(),
            status_topic: config["homeassistant"]["status_topic"]
                .as_str()
                .unwrap_or("homeassistant/status")
                .to_string(),
            enabled: config["homeassistant"]["enabled"].as_bool().unwrap_or(true),
            cache_state: config["advanced"]["cache_state"].as_bool().unwrap_or(true),
            experimental_event_entities: config["experimental"]["event_entities"]
                .as_bool()
                .unwrap_or(false),
            devices: Vec::new(),
            groups: Vec::new(),
            published: Vec::new(),
            discovery: HashMap::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
            started: false,
        }
    }

    pub fn add_device(&mut self, entity: HaEntity) {
        self.devices.push(entity);
    }

    pub fn add_group(&mut self, entity: HaEntity) {
        self.groups.push(entity);
    }

    /// Upstream extension `start()`.
    pub async fn start(&mut self) -> Result<(), String> {
        todo!()
    }

    /// Upstream ZHE `message` / MQTT handler.
    pub async fn handle_message(&mut self, _topic: &str, _payload: Value) {
        todo!()
    }

    /// Upstream `onPublishEntityState`.
    pub async fn on_publish_entity_state(&mut self, _id: &str, _state: Value) {
        todo!()
    }

    /// Upstream discovery generation for one entity.
    pub async fn discover_entity(&mut self, _id: &str) {
        todo!()
    }

    /// Upstream `removeDevice` / `removeGroup`.
    pub async fn remove_entity(&mut self, _id: &str) {
        todo!()
    }

    /// Upstream rename refresh.
    pub async fn rename_entity(&mut self, _old_id: &str, _new_name: &str) {
        todo!()
    }

    /// Upstream `clearOutdatedConfigs`.
    pub async fn clear_outdated_configs(&mut self) {
        todo!()
    }

    /// Upstream `sendAllStatus`.
    pub async fn send_all_status(&mut self) {
        todo!()
    }

    /// Upstream options change handling.
    pub async fn on_options_change(&mut self, _id: &str, _options: Value) {
        todo!()
    }

    /// Upstream scene change handling.
    pub async fn on_scene_change(&mut self, _id: &str) {
        todo!()
    }

    /// Upstream expose-changed announcement.
    pub async fn on_exposes_changed(&mut self, _id: &str) {
        todo!()
    }

    /// Config validation performed by `start()`.
    pub fn validate_config(&self) -> Result<(), String> {
        todo!()
    }

    pub fn discovery_for(&self, topic: &str) -> Option<&Value> {
        self.discovery.get(topic)
    }

    pub fn has_discovery_containing(&self, needle: &str) -> bool {
        self.discovery
            .iter()
            .any(|(topic, payload)| topic.contains(needle) || payload.to_string().contains(needle))
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
