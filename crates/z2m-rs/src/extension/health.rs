// Original: .upstream/zigbee2mqtt/lib/extension/health.ts
// TODO: port OS/process metrics and real timer integration.

use std::collections::BTreeMap;

use serde_json::{json, Value};

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
    pub elapsed_seconds: u64,
    pub last_publish_seconds: u64,
    pub check_count: u64,
    pub mqtt_published: u64,
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
            elapsed_seconds: 0,
            last_publish_seconds: 0,
            check_count: 0,
            mqtt_published: 0,
        }
    }

    fn entry(&mut self, ieee_address: &str) -> &mut DeviceHealth {
        self.devices.entry(ieee_address.to_string()).or_default()
    }

    /// Upstream: `getExtension().clearStats()`.
    pub fn clear_stats(&mut self) {
        self.devices.clear();
        self.mqtt_received = 0;
    }

    pub fn on_last_seen_changed(&mut self, ieee_address: &str) {
        self.entry(ieee_address).messages += 1;
    }

    pub fn on_device_leave(&mut self, ieee_address: &str) {
        self.entry(ieee_address).leave_count += 1;
    }

    pub fn on_device_joined(&mut self, ieee_address: &str) {
        self.entry(ieee_address);
    }

    pub fn on_device_network_address_changed(&mut self, ieee_address: &str) {
        self.entry(ieee_address).network_address_changes += 1;
    }

    pub fn on_mqtt_message(&mut self, _topic: &str, _payload: &str) {
        self.mqtt_received += 1;
    }

    /// Fake-timer replacement.
    pub fn advance_seconds(&mut self, seconds: u64) {
        self.elapsed_seconds += seconds;
        let interval_seconds = self.interval_minutes * 60;
        if self.elapsed_seconds < self.last_publish_seconds + interval_seconds {
            return;
        }

        self.last_publish_seconds = self.elapsed_seconds;
        self.check_count += 1;
        self.mqtt_published += 1;

        let denominator = (interval_seconds * self.check_count) as f64;
        let mut devices = json!({});
        for (ieee_address, health) in &self.devices {
            devices[ieee_address] = json!({
                "leave_count": health.leave_count,
                "messages": health.messages,
                "messages_per_sec": if health.messages == 0 || denominator <= 0.0 {
                    json!(0)
                } else {
                    json!(round4(health.messages as f64 / denominator))
                },
                "network_address_changes": health.network_address_changes,
            });
        }

        self.published.push(json!({
            "response_time": 1,
            "os": {
                "load_average": [0.1, 0.2, 0.3],
                "memory_used_mb": 1,
                "memory_percent": 1,
            },
            "process": {
                "uptime_sec": self.elapsed_seconds,
                "memory_used_mb": 1,
                "memory_percent": 1,
            },
            "mqtt": {
                "connected": true,
                "queued": 0,
                "published": self.mqtt_published,
                "received": self.mqtt_received,
            },
            "devices": devices,
        }));
        self.publish_options.push((true, 1));

        if self.reset_on_check {
            self.devices.clear();
        }
    }

    pub fn advance_minutes(&mut self, minutes: u64) {
        self.advance_seconds(minutes * 60)
    }
}

fn round4(value: f64) -> f64 {
    (value * 10000.0).round() / 10000.0
}
