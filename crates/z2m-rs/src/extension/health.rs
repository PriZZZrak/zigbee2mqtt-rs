// Original: .upstream/zigbee2mqtt/lib/extension/health.ts
// TODO: port the health extension.
//
// Upstream relies on the controller event bus and fake timers. The Rust port
// records health statistics on an explicit `Health` value and publishes on
// `advance_*` calls.

use std::collections::BTreeMap;

use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DeviceHealth {
    pub leave_count: u64,
    pub messages: u64,
    pub messages_per_sec: f64,
    pub network_address_changes: u64,
}

#[derive(Debug)]
pub struct Health {
    pub interval_minutes: u64,
    pub reset_on_check: bool,
    pub devices: BTreeMap<String, DeviceHealth>,
    pub published: Vec<Value>,
    /// `(retain, qos)` for each published payload.
    pub publish_options: Vec<(bool, u8)>,
    pub mqtt_received: u64,
}

impl Health {
    pub fn new(interval_minutes: u64, reset_on_check: bool) -> Self {
        Self {
            interval_minutes,
            reset_on_check,
            devices: BTreeMap::new(),
            published: Vec::new(),
            publish_options: Vec::new(),
            mqtt_received: 0,
        }
    }

    /// Upstream: `getExtension().clearStats()`.
    pub fn clear_stats(&mut self) {
        todo!()
    }

    pub fn on_last_seen_changed(&mut self, ieee_address: &str) {
        todo!()
    }

    pub fn on_device_leave(&mut self, ieee_address: &str) {
        todo!()
    }

    pub fn on_device_joined(&mut self, ieee_address: &str) {
        todo!()
    }

    pub fn on_device_network_address_changed(&mut self, ieee_address: &str) {
        todo!()
    }

    pub fn on_mqtt_message(&mut self, _topic: &str, _payload: &str) {
        todo!()
    }

    /// Fake-timer replacement.
    pub fn advance_seconds(&mut self, _seconds: u64) {
        todo!()
    }

    pub fn advance_minutes(&mut self, minutes: u64) {
        self.advance_seconds(minutes * 60)
    }
}
