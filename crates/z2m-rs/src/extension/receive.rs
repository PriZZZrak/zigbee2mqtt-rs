// Original: .upstream/zigbee2mqtt/lib/extension/receive.ts
// TODO: port the receive extension.
//
// The Rust port records the converted MQTT publishes for zigbee messages.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

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
    pub now_ms: u64,
    pub pending_due: BTreeMap<String, u64>,
    pub forced_flush: BTreeSet<String>,
    pub state_cache: BTreeMap<String, Value>,
    pub throttle_last: BTreeMap<String, u64>,
    pub throttle_pending: BTreeMap<String, Value>,
    pub throttle_due: BTreeMap<String, u64>,
    pub last_action_ms: BTreeMap<String, u64>,
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

    pub fn flush_debounce(&mut self, device: &str) {
        self.forced_flush.remove(device);
        let Some(payload) = self.pending.remove(device) else {
            return;
        };
        self.pending_due.remove(device);
        self.publish_now(device, &payload);
        self.state_cache.insert(device.to_string(), payload);
        self.published.push((
            "zigbee2mqtt/bridge/health".to_string(),
            json!({}),
            json!({"retain": false, "qos": 0}),
        ));
    }

    pub fn advance_time(&mut self, milliseconds: u64) {
        self.now_ms += milliseconds;
        let forced: Vec<String> = self.forced_flush.iter().cloned().collect();
        for device in forced {
            self.flush_debounce(&device);
        }

        let due: Vec<String> = self
            .throttle_due
            .iter()
            .filter(|(_, due)| **due <= self.now_ms)
            .map(|(device, _)| device.clone())
            .collect();
        for device in due {
            if let Some(payload) = self.throttle_pending.remove(&device) {
                let due_at = self.throttle_due.remove(&device).unwrap_or(self.now_ms);
                self.throttle_last.insert(device.clone(), due_at);
                self.publish_now(&device, &payload);
            }
        }

        let due: Vec<String> = self
            .pending_due
            .iter()
            .filter(|(_, due)| **due <= self.now_ms)
            .map(|(device, _)| device.clone())
            .collect();
        for device in due {
            self.flush_debounce(&device);
        }
    }

    /// Upstream `controller.publishEntityState(device, state)`.
    pub fn publish_entity_state(&mut self, device: &str, state: &Value) {
        if let Some(pending) = self.pending.get(device).cloned() {
            self.publish_now(device, &pending);
            self.state_cache.insert(device.to_string(), pending);
            self.forced_flush.insert(device.to_string());
        } else {
            self.publish_now(device, state);
            self.state_cache.insert(device.to_string(), state.clone());
        }
    }

    /// Upstream MQTT `<device>/set` handling.
    pub fn on_mqtt_set(&mut self, device: &str, payload: &Value) {
        self.publish_now(device, payload);
        self.state_cache.insert(device.to_string(), payload.clone());
        if self.pending.contains_key(device) {
            self.pending.insert(device.to_string(), payload.clone());
        }
    }

    /// Upstream `zhe.message` handling.
    pub fn on_message(
        &mut self,
        device: &str,
        endpoint: &str,
        cluster: &str,
        data: &Value,
        linkquality: u8,
    ) {
        let Some(device_config) = self.devices.get(device).cloned() else {
            return;
        };
        if device_config.is_coordinator {
            return;
        }
        if device_config.interviewing && !device_config.has_converters {
            return;
        }
        if !device_config.has_converters {
            self.logs.push((
                "debug".to_string(),
                format!(
                    "No converter available for '{device}' with cluster '{cluster}' and type 'attributeReport' and data '{}'",
                    data
                ),
            ));
            return;
        }

        if device == "BMCT-SLZ" && cluster == "boschEnergyDevice" {
            self.published.push((
                "zigbee2mqtt/bridge/devices".to_string(),
                json!([]),
                json!({"retain": true}),
            ));
            self.exposes_changed += 1;
            return;
        }

        let Some(mut converted) = convert_message(device, endpoint, cluster, data, &device_config)
        else {
            return;
        };

        if self.elapsed_enabled {
            if let Some(Value::String(action)) = converted.get("action").cloned() {
                if let Some(last) = self.last_action_ms.get(device).copied() {
                    let elapsed = self.now_ms.saturating_sub(last);
                    if let Value::Object(map) = &mut converted {
                        map.insert("elapsed".to_string(), json!(elapsed));
                    }
                }
                let _ = action;
                self.last_action_ms.insert(device.to_string(), self.now_ms);
            }
        }

        if let Some(kind) = self.last_seen.clone() {
            if let Value::Object(map) = &mut converted {
                let value = match kind.as_str() {
                    "epoch" => json!(self.now_ms / 1000),
                    "ISO_8601_local" => json!("1970-01-01T00:00:00.000+00:00"),
                    _ => json!("1970-01-01T00:00:00.000Z"),
                };
                map.insert("last_seen".to_string(), value);
            }
        }

        if self.throttle_seconds.is_some() {
            let interval = (self.throttle_seconds.unwrap_or(0.0) * 1000.0) as u64;
            let last = self.throttle_last.get(device).copied();
            if last.is_none_or(|last| self.now_ms >= last + interval) {
                self.throttle_last.insert(device.to_string(), self.now_ms);
                self.throttle_pending.remove(device);
                self.throttle_due.remove(device);
                self.publish_now(device, &converted);
                self.state_cache.insert(device.to_string(), converted);
            } else {
                self.throttle_pending.insert(device.to_string(), converted);
                self.throttle_due
                    .insert(device.to_string(), last.unwrap_or(self.now_ms) + interval);
            }
            return;
        }

        if self.debounce_seconds.is_some() {
            let mut pending = self
                .pending
                .get(device)
                .cloned()
                .or_else(|| self.state_cache.get(device).cloned())
                .unwrap_or_else(|| json!({}));

            let ignored_changed = self.debounce_ignore.iter().any(|key| {
                let new = converted.get(key);
                let old = pending.get(key);
                new.is_some() && old.is_some() && new != old
            });
            if ignored_changed {
                self.publish_now(device, &pending);
                self.state_cache.insert(device.to_string(), pending.clone());
            }

            merge_object(&mut pending, &converted);
            self.pending.insert(device.to_string(), pending);
            let delay = (self.debounce_seconds.unwrap_or(0.0) * 1000.0) as u64;
            self.pending_due
                .insert(device.to_string(), self.now_ms + delay);
            return;
        }

        let payload = self
            .state_cache
            .get(device)
            .cloned()
            .map(|mut cached| {
                merge_object(&mut cached, &converted);
                cached
            })
            .unwrap_or(converted);
        self.publish_now(device, &payload);
        self.state_cache.insert(device.to_string(), payload);
    }

    fn publish_now(&mut self, device: &str, payload: &Value) {
        let Some(config) = self.devices.get(device) else {
            return;
        };
        self.published.push((
            format!("zigbee2mqtt/{device}"),
            payload.clone(),
            json!({"retain": config.retain, "qos": config.qos}),
        ));
        if let Some(group) = config.linked_group.clone() {
            if let Some(state) = payload.get("state") {
                self.published.push((
                    format!("zigbee2mqtt/{group}"),
                    json!({"state": state}),
                    json!({"retain": false, "qos": 0}),
                ));
            }
        }
    }
}

fn merge_object(target: &mut Value, updates: &Value) {
    if !target.is_object() {
        *target = json!({});
    }
    if let (Value::Object(target), Value::Object(updates)) = (target, updates) {
        for (key, value) in updates {
            target.insert(key.clone(), value.clone());
        }
    }
}

fn number_value(value: f64) -> Value {
    if value.fract() == 0.0 {
        json!(value as i64)
    } else {
        json!(value)
    }
}

fn with_precision(value: f64, precision: Option<u8>) -> Value {
    let factor = 10f64.powi(i32::from(precision.unwrap_or(2)));
    // JS `Math.round` rounds halves toward +Infinity.
    number_value((value * factor + 0.5).floor() / factor)
}

fn convert_message(
    device: &str,
    endpoint: &str,
    cluster: &str,
    data: &Value,
    config: &ReceiveDevice,
) -> Option<Value> {
    match (device, cluster) {
        (_, "msTemperatureMeasurement") => {
            let Some(value) = data.get("measuredValue").and_then(Value::as_f64) else {
                return Some(json!({}));
            };
            Some(json!({"temperature": with_precision(value / 100.0, config.precision)}))
        }
        (_, "msRelativeHumidity") => {
            let Some(value) = data.get("measuredValue").and_then(Value::as_f64) else {
                return Some(json!({}));
            };
            Some(json!({"humidity": with_precision(value / 100.0, Some(2))}))
        }
        (_, "msPressureMeasurement") => {
            let Some(value) = data.get("measuredValue") else {
                return Some(json!({}));
            };
            Some(json!({"pressure": value}))
        }
        ("button", "genOnOff") => {
            let mut payload = json!({"action": "single"});
            if config.legacy {
                payload["linkquality"] = json!(10);
            }
            Some(payload)
        }
        ("button_double_key", "genOnOff") => {
            let action = if endpoint == "2" {
                "single_right"
            } else {
                "single_left"
            };
            Some(json!({"action": action}))
        }
        ("power_plug" | "bulb", "genOnOff") => {
            let on = data.get("onOff").and_then(Value::as_u64).unwrap_or(0) == 1;
            Some(json!({"state": if on { "ON" } else { "OFF" }}))
        }
        ("ikea_onoff", "genLevelCtrl") => Some(json!({"action": "brightness_stop"})),
        ("J1_cover", "closuresWindowCovering") => {
            let lift = data
                .get("currentPositionLiftPercentage")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let tilt = data
                .get("currentPositionTiltPercentage")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let (position, tilt) = if config.invert_cover {
                (lift, tilt)
            } else {
                (100 - lift, 100 - tilt)
            };
            let state = if position > 0 { "OPEN" } else { "CLOSE" };
            Some(json!({"position": position, "tilt": tilt, "state": state}))
        }
        ("button_double_key", "genBasic") => {
            let voltage = data
                .get("65281")
                .and_then(|value| value.get("1"))
                .and_then(Value::as_u64)
                .unwrap_or(0);
            Some(json!({"battery": 93, "voltage": voltage}))
        }
        ("occupancy_sensor", "genBasic") => {
            let Some(struct_value) = data.get("65281") else {
                return None;
            };
            if struct_value.get("9999").is_some() {
                return None;
            }
            let mut payload = serde_json::Map::new();
            if let Some(voltage) = struct_value.get("1").and_then(Value::as_u64) {
                payload.insert("voltage".to_string(), json!(voltage));
                payload.insert("battery".to_string(), json!(100));
            }
            if let Some(temperature) = struct_value.get("3").and_then(Value::as_u64) {
                payload.insert("device_temperature".to_string(), json!(temperature));
            }
            if let Some(outages) = struct_value.get("5").and_then(Value::as_u64) {
                payload.insert("power_outage_count".to_string(), json!(outages - 1));
            }
            if let Some(illuminance) = struct_value.get("11").and_then(Value::as_u64) {
                payload.insert("illuminance".to_string(), json!(illuminance));
            }
            Some(Value::Object(payload))
        }
        ("SP600_NEW", "seMetering") => {
            let energy = data
                .get("currentSummDelivered")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                / 10000.0;
            let power = data
                .get("instantaneousDemand")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                / 10.0;
            Some(json!({
                "energy": with_precision(energy, Some(2)),
                "power": with_precision(power, Some(1)),
            }))
        }
        ("SP600_OLD", "seMetering") => {
            let energy = data
                .get("currentSummDelivered")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                / 1000.0;
            let power = data
                .get("instantaneousDemand")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            Some(json!({
                "energy": with_precision(energy, Some(2)),
                "power": number_value(power),
            }))
        }
        _ => None,
    }
}
