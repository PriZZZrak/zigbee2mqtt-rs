// Original: .upstream/zigbee2mqtt/lib/extension/receive.ts
// TODO: port the receive extension.
//
// The Rust port records the converted MQTT publishes for zigbee messages.

use std::collections::BTreeMap;

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ReceiveDevice {
    pub friendly_name: String,
    pub qos: u8,
    pub retain: bool,
    pub precision: Option<u8>,
    pub legacy: bool,
    pub invert_cover: bool,
    pub is_coordinator: bool,
    pub interviewing: bool,
    pub has_converters: bool,
    pub linked_group: Option<String>,
    pub elapsed: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Receive {
    pub devices: BTreeMap<String, ReceiveDevice>,
    pub last_seen: Option<String>,
    /// `(topic, payload, options)`.
    pub published: Vec<(String, Value, Value)>,
    pub logs: Vec<(String, String)>,
    pub exposes_changed: usize,
    pub debounce_seconds: Option<f64>,
    pub debounce_ignore: Vec<String>,
    pub throttle_seconds: Option<f64>,
    pub elapsed_enabled: bool,
    pub pending: BTreeMap<String, Value>,
}

impl Receive {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_device(&mut self, friendly_name: &str, qos: u8, retain: bool) {
        self.devices.insert(
            friendly_name.to_string(),
            ReceiveDevice {
                friendly_name: friendly_name.to_string(),
                qos,
                retain,
                precision: None,
                legacy: true,
                invert_cover: false,
                is_coordinator: false,
                interviewing: false,
                has_converters: true,
                linked_group: None,
                elapsed: false,
            },
        );
    }

    pub fn device_mut(&mut self, friendly_name: &str) -> &mut ReceiveDevice {
        self.devices.get_mut(friendly_name).expect("device")
    }

    pub fn set_debounce(&mut self, seconds: f64) {
        self.debounce_seconds = Some(seconds);
    }

    pub fn set_throttle(&mut self, seconds: f64) {
        self.throttle_seconds = Some(seconds);
    }

    pub fn flush_debounce(&mut self, _device: &str) {
        todo!()
    }

    pub fn advance_time(&mut self, _milliseconds: u64) {
        todo!()
    }

    /// Upstream `controller.publishEntityState(device, state)`.
    pub fn publish_entity_state(&mut self, _device: &str, _state: &Value) {
        todo!()
    }

    /// Upstream MQTT `<device>/set` handling.
    pub fn on_mqtt_set(&mut self, _device: &str, _payload: &Value) {
        todo!()
    }

    /// Upstream `zhe.message` handling.
    pub fn on_message(
        &mut self,
        _device: &str,
        _endpoint: &str,
        _cluster: &str,
        _data: &Value,
        _linkquality: u8,
    ) {
        todo!()
    }
}
