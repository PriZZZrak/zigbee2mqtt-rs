// Original: .upstream/zigbee2mqtt/lib/extension/availability.ts
// TODO: port the availability extension.
//
// The Rust port records ping calls and availability publishes while advancing
// time explicitly.

use std::collections::BTreeMap;

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct DeviceAvailability {
    pub passive: bool,
    /// `availability` setting for this device.
    pub enabled: bool,
    pub disabled: bool,
    pub timeout_minutes: u64,
    pub last_seen_minutes_ago: u64,
    /// `true` for a normal ping, `false` for the retry after a failure.
    pub ping_calls: Vec<bool>,
    pub fail_first_ping: bool,
    pub removed: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Availability {
    pub enabled: bool,
    pub active_timeout_minutes: u64,
    pub passive_timeout_minutes: u64,
    pub devices: BTreeMap<String, DeviceAvailability>,
    /// group friendly_name -> member device friendly_names.
    pub groups: BTreeMap<String, Vec<String>>,
    /// `(topic, payload, options)`.
    pub published: Vec<(String, Value, Value)>,
    pub ping_queue: Vec<String>,
    pub stopped: bool,
    pub endpoint_reads: usize,
}

impl Availability {
    pub fn new() -> Self {
        Self {
            enabled: true,
            active_timeout_minutes: 10,
            passive_timeout_minutes: 25 * 60,
            ..Default::default()
        }
    }

    pub fn add_device(&mut self, friendly_name: &str, passive: bool) {
        self.devices.insert(
            friendly_name.to_string(),
            DeviceAvailability {
                passive,
                enabled: true,
                disabled: false,
                timeout_minutes: if passive {
                    self.passive_timeout_minutes
                } else {
                    self.active_timeout_minutes
                },
                last_seen_minutes_ago: 0,
                ping_calls: Vec::new(),
                fail_first_ping: false,
                removed: false,
            },
        );
    }

    pub fn add_group(&mut self, group: &str, members: &[&str]) {
        self.groups.insert(
            group.to_string(),
            members.iter().map(|member| (*member).to_string()).collect(),
        );
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn set_active_timeout(&mut self, minutes: u64) {
        self.active_timeout_minutes = minutes;
    }

    pub fn set_availability(&mut self, device: &str, enabled: bool) {
        self.devices.get_mut(device).unwrap().enabled = enabled;
    }

    pub fn set_timeout(&mut self, device: &str, minutes: u64) {
        self.devices.get_mut(device).unwrap().timeout_minutes = minutes;
    }

    pub fn set_disabled(&mut self, device: &str) {
        self.devices.get_mut(device).unwrap().disabled = true;
    }

    pub fn set_last_seen_minutes_ago(&mut self, device: &str, minutes: u64) {
        self.devices.get_mut(device).unwrap().last_seen_minutes_ago = minutes;
    }

    pub fn set_fail_first_ping(&mut self, device: &str) {
        self.devices.get_mut(device).unwrap().fail_first_ping = true;
    }

    pub fn add_to_ping_queue(&mut self, device: &str) {
        self.ping_queue.push(device.to_string());
    }

    /// Upstream `extension.start()`.
    pub fn start(&mut self) {
        todo!()
    }

    pub fn stop(&mut self) {
        todo!()
    }

    pub fn advance_seconds(&mut self, _seconds: u64) {
        todo!()
    }

    pub fn advance_minutes(&mut self, minutes: u64) {
        self.advance_seconds(minutes * 60)
    }

    pub fn advance_hours(&mut self, hours: u64) {
        self.advance_seconds(hours * 60 * 60)
    }

    pub fn on_last_seen_changed(&mut self, device: &str) {
        todo!()
    }

    pub fn on_device_leave(&mut self, device: &str) {
        todo!()
    }

    pub fn on_device_remove(&mut self, device: &str) {
        todo!()
    }

    pub fn on_device_announce(&mut self, device: &str) {
        todo!()
    }

    pub fn on_rename(&mut self, from: &str, to: &str) {
        todo!()
    }
}
