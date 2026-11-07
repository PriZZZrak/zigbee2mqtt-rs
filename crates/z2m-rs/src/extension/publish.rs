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
}

#[derive(Debug, Clone, Default)]
pub struct Publish {
    pub devices: BTreeMap<String, PublishDevice>,
    pub group_commands: BTreeMap<String, usize>,
    /// `(topic, payload, options)`.
    pub published: Vec<(String, Value, Value)>,
    pub logs: Vec<(String, String)>,
}

impl Publish {
    pub fn new() -> Self {
        Self::default()
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
            },
        );
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
