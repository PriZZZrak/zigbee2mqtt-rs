// Original: .upstream/zigbee2mqtt/lib/extension/publish.ts
// TODO: port the publish extension.
//
// The Rust port records endpoint commands/writes and optimistic state publishes.

use std::collections::BTreeMap;

use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct PublishEndpoint {
    /// `(cluster, command, payload, options)`.
    pub commands: Vec<(String, String, Value, Value)>,
    pub reads: Vec<(String, Vec<String>)>,
    pub writes: Vec<(String, Value, Value)>,
}

#[derive(Debug, Clone, Default)]
pub struct PublishDevice {
    pub friendly_name: String,
    pub endpoints: BTreeMap<String, PublishEndpoint>,
    pub has_set_converter: bool,
    pub optimistic: bool,
    pub filtered_optimistic: Vec<String>,
    pub has_definition: bool,
    /// endpoint alias (left/right) -> endpoint id.
    pub endpoint_aliases: BTreeMap<String, String>,
    /// transition in deciseconds.
    pub transition: Option<u32>,
    pub invert_cover: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PublishGroup {
    pub id: Option<u32>,
    pub members: Vec<(String, String)>,
    pub commands: Vec<(String, String, Value, Value)>,
    pub optimistic: bool,
    pub transition: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct Publish {
    pub devices: BTreeMap<String, PublishDevice>,
    pub groups: BTreeMap<String, PublishGroup>,
    pub group_commands: BTreeMap<String, usize>,
    /// `(topic, payload, options)`.
    pub published: Vec<(String, Value, Value)>,
    /// `(level, message, namespace)`.
    pub logs: Vec<(String, String, String)>,
    pub base_topic: String,
    pub device_states: BTreeMap<String, Value>,
    pub homeassistant_enabled: bool,
}

impl Publish {
    pub fn new() -> Self {
        Self {
            base_topic: "zigbee2mqtt".to_string(),
            ..Default::default()
        }
    }

    pub fn add_device(&mut self, friendly_name: &str, endpoints: &[&str]) {
        let endpoints = endpoints
            .iter()
            .map(|endpoint| ((*endpoint).to_string(), PublishEndpoint::default()))
            .collect();
        self.devices.insert(
            friendly_name.to_string(),
            PublishDevice {
                friendly_name: friendly_name.to_string(),
                endpoints,
                has_set_converter: true,
                optimistic: true,
                filtered_optimistic: Vec::new(),
                has_definition: true,
                endpoint_aliases: BTreeMap::new(),
                transition: None,
                invert_cover: false,
            },
        );
    }

    pub fn set_device_state(&mut self, device: &str, state: Value) {
        self.device_states.insert(device.to_string(), state);
    }

    pub fn add_endpoint_alias(&mut self, device: &str, alias: &str, endpoint: &str) {
        self.devices
            .get_mut(device)
            .expect("device")
            .endpoint_aliases
            .insert(alias.to_string(), endpoint.to_string());
    }

    pub fn add_group(&mut self, friendly_name: &str) {
        self.groups.insert(
            friendly_name.to_string(),
            PublishGroup {
                optimistic: true,
                ..Default::default()
            },
        );
    }

    pub fn add_group_member(&mut self, group: &str, device: &str, endpoint: &str) {
        self.groups
            .get_mut(group)
            .expect("group")
            .members
            .push((device.to_string(), endpoint.to_string()));
    }

    pub fn group(&mut self, group: &str) -> &mut PublishGroup {
        self.groups.get_mut(group).expect("group")
    }

    pub fn endpoint(&mut self, device: &str, endpoint: &str) -> &mut PublishEndpoint {
        self.devices
            .get_mut(device)
            .expect("device")
            .endpoints
            .get_mut(endpoint)
            .expect("endpoint")
    }

    /// Upstream MQTT message handler (`<topic>/set`).
    pub fn on_mqtt(&mut self, topic: &str, payload: &Value) {
        todo!()
    }

    pub fn on_mqtt_raw(&mut self, topic: &str, payload: &str) {
        todo!()
    }
}
