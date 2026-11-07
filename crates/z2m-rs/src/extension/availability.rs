// Original: .upstream/zigbee2mqtt/lib/extension/availability.ts
// TODO: port the availability extension.
//
// The Rust port records ping calls and availability publishes while advancing
// time explicitly.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde_json::{json, Value};

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
    pub backoff_enabled: bool,
    pub availability_explicit: bool,
    pub timeout_explicit: bool,
    pub last_seen_at: i64,
    pub next_event_at: Option<u64>,
    pub backoff: f64,
    pub last_published: Option<bool>,
    pub retry_at: Option<u64>,
    pub retrieve_at: Option<u64>,
    pub failures_remaining: u8,
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
    pub max_jitter_ms: u64,
    pub backoff_enabled: bool,
    pub pause_on_backoff_gt: Option<f64>,
    /// Per-device jitter and backoff overrides/multiplication state.
    pub jitter_ms: BTreeMap<String, u64>,
    pub backoff_multiplier: BTreeMap<String, f64>,
    pub paused: BTreeSet<String>,
    pub device_pause: BTreeMap<String, f64>,
    pub now_seconds: u64,
    pub next_ping_slot: u64,
    pub ever_started: bool,
    pub due_queue: VecDeque<String>,
}

impl Availability {
    pub fn new() -> Self {
        Self {
            enabled: true,
            active_timeout_minutes: 10,
            passive_timeout_minutes: 25 * 60,
            max_jitter_ms: 1000,
            backoff_enabled: true,
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
                backoff_enabled: true,
                availability_explicit: false,
                timeout_explicit: false,
                last_seen_at: 0,
                next_event_at: None,
                backoff: 1.0,
                last_published: None,
                retry_at: None,
                retrieve_at: None,
                failures_remaining: 0,
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
        for device in self.devices.values_mut() {
            if !device.passive && !device.timeout_explicit {
                device.timeout_minutes = minutes;
            }
        }
    }

    pub fn set_availability(&mut self, device: &str, enabled: bool) {
        let device = self.devices.get_mut(device).unwrap();
        device.enabled = enabled;
        device.availability_explicit = true;
    }

    pub fn set_timeout(&mut self, device: &str, minutes: u64) {
        let entry = self.devices.get_mut(device).unwrap();
        entry.timeout_minutes = minutes;
        entry.timeout_explicit = true;
    }

    pub fn set_disabled(&mut self, device: &str) {
        self.devices.get_mut(device).unwrap().disabled = true;
    }

    pub fn set_last_seen_minutes_ago(&mut self, device: &str, minutes: u64) {
        let now = self.now_seconds as i64;
        let device = self.devices.get_mut(device).unwrap();
        device.last_seen_minutes_ago = minutes;
        device.last_seen_at = now - (minutes as i64 * 60);
        device.availability_explicit = true;
    }

    pub fn set_max_jitter(&mut self, milliseconds: u64) {
        self.max_jitter_ms = milliseconds;
    }

    pub fn set_backoff_enabled(&mut self, enabled: bool) {
        self.backoff_enabled = enabled;
    }

    pub fn set_pause_on_backoff_gt(&mut self, value: f64) {
        self.pause_on_backoff_gt = Some(value);
    }

    pub fn set_device_max_jitter(&mut self, device: &str, milliseconds: u64) {
        self.jitter_ms.insert(device.to_string(), milliseconds);
    }

    pub fn set_device_backoff_enabled(&mut self, device: &str, enabled: bool) {
        self.devices.get_mut(device).unwrap().backoff_enabled = enabled;
    }

    pub fn set_device_pause_on_backoff_gt(&mut self, device: &str, value: f64) {
        self.device_pause.insert(device.to_string(), value);
    }

    fn enabled(&self, device: &DeviceAvailability) -> bool {
        let enabled = if device.availability_explicit {
            device.enabled
        } else {
            self.enabled
        };
        enabled && !device.disabled && !device.removed
    }

    fn available(&self, device: &DeviceAvailability) -> bool {
        let now = self.now_seconds as i64;
        now - device.last_seen_at < (device.timeout_minutes as i64 * 60)
    }

    fn max_jitter(&self, name: &str, device: &DeviceAvailability) -> u64 {
        if let Some(jitter) = self.jitter_ms.get(name) {
            return *jitter;
        }
        if device.passive {
            return 0;
        }
        self.max_jitter_ms
    }

    fn jitter(&self, name: &str, device: &DeviceAvailability) -> u64 {
        let max = self.max_jitter(name, device);
        if max == 0 {
            return 0;
        }
        if !self.jitter_ms.contains_key(name) {
            return max;
        }
        let hash = name.bytes().fold(1469598103934665603u64, |hash, byte| {
            (hash ^ byte as u64).wrapping_mul(1099511628211)
        });
        hash % max
    }

    fn publish(&mut self, name: &str) {
        let Some(device) = self.devices.get(name) else {
            return;
        };
        let available = self.available(device);
        let topic = format!("zigbee2mqtt/{name}/availability");
        let payload = json!({"state": if available { "online" } else { "offline" }});
        if device.last_published != Some(available) {
            if let Some(device) = self.devices.get_mut(name) {
                device.last_published = Some(available);
            }
            self.published
                .push((topic, payload, json!({"retain": true, "qos": 1})));
        }

        let groups: Vec<String> = self
            .groups
            .iter()
            .filter(|(_, members)| members.iter().any(|member| member == name))
            .map(|(group, _)| group.clone())
            .collect();
        for group in groups {
            let online = self.groups[&group].iter().any(|member| {
                self.devices.get(member).and_then(|d| d.last_published) == Some(true)
            });
            let topic = format!("zigbee2mqtt/{group}/availability");
            let payload = json!({"state": if online { "online" } else { "offline" }});
            self.published
                .push((topic, payload, json!({"retain": true, "qos": 1})));
        }
    }

    fn schedule(&mut self, name: &str, reset_backoff: bool) {
        let now = self.now_seconds;
        let Some(device) = self.devices.get(name) else {
            return;
        };
        let enabled = self.enabled(device);
        let jitter = self.jitter(name, device);
        let passive = device.passive;
        let timeout = device.timeout_minutes;
        let backoff_enabled = device.backoff_enabled && self.backoff_enabled;

        if !enabled {
            self.devices.get_mut(name).unwrap().next_event_at = None;
            return;
        }
        if reset_backoff {
            self.paused.remove(name);
        }
        let device = self.devices.get_mut(name).unwrap();
        if reset_backoff {
            device.backoff = 1.0;
        }
        let delay = if passive {
            timeout * 60 + 1
        } else {
            let backoff = if backoff_enabled { device.backoff } else { 1.0 };
            (((timeout * 60 + 1) as f64 + jitter as f64 / 1000.0) * backoff).min(2_147_483.0) as u64
        };
        device.next_event_at = Some(now + delay);
        device.retry_at = None;
        if name == "bulb_color" {
            eprintln!(
                "DBG schedule now={} delay={} next={:?}",
                now, delay, device.next_event_at
            );
        }
    }

    pub fn set_fail_first_ping(&mut self, device: &str) {
        if let Some(entry) = self.devices.get_mut(device) {
            entry.fail_first_ping = true;
            entry.failures_remaining = 4;
        }
    }

    pub fn add_to_ping_queue(&mut self, device: &str) {
        self.ping_queue.push(device.to_string());
    }

    /// Upstream `extension.start()` rejecting a second start.
    pub async fn start_checked(&mut self) -> Result<(), String> {
        if self.ever_started {
            return Err("Availability extension already started".to_string());
        }
        Ok(())
    }

    /// Upstream `extension.start()`.
    pub fn start(&mut self) {
        if self.ever_started {
            return;
        }
        self.ever_started = true;
        self.stopped = false;
        let names: Vec<String> = self.devices.keys().cloned().collect();
        for name in names {
            if self.enabled(&self.devices[&name]) {
                self.publish(&name);
                let (passive, timeout, jitter) = {
                    let device = &self.devices[&name];
                    (
                        device.passive,
                        device.timeout_minutes,
                        self.jitter(&name, device),
                    )
                };
                let last_seen = self.devices[&name].last_seen_at;
                let delay = if passive {
                    timeout * 60 + 1
                } else {
                    timeout * 60 + 1 + jitter / 1000
                } as i64;
                let due = last_seen + delay;
                let now = self.now_seconds as i64;
                self.devices.get_mut(&name).unwrap().next_event_at =
                    Some(if due < now { now as u64 } else { due as u64 });
                self.devices.get_mut(&name).unwrap().retry_at = None;
            }
        }
    }

    pub fn stop(&mut self) {
        self.stopped = true;
        self.ping_queue.clear();
        self.due_queue.clear();
        for device in self.devices.values_mut() {
            device.next_event_at = None;
            device.retry_at = None;
        }
    }

    pub fn advance_seconds(&mut self, seconds: u64) {
        for _ in 0..seconds {
            if self.stopped {
                break;
            }
            self.now_seconds += 1;
            self.process_due();
        }
    }

    fn process_due(&mut self) {
        let now = self.now_seconds;
        let names: Vec<String> = self.devices.keys().cloned().collect();

        for name in &names {
            let Some(device) = self.devices.get(name) else {
                continue;
            };
            let retrieve = device.retrieve_at.is_some_and(|time| time <= now);
            let retry = device.retry_at.is_some_and(|time| time <= now);
            let due = device.next_event_at.is_some_and(|time| time <= now);
            let passive = device.passive;

            if retrieve {
                self.endpoint_reads += 1;
                self.devices.get_mut(name).unwrap().retrieve_at = None;
            }
            if retry {
                self.devices.get_mut(name).unwrap().retry_at = None;
                let remaining = self.devices.get(name).unwrap().failures_remaining;
                if remaining > 0 {
                    self.devices.get_mut(name).unwrap().failures_remaining = remaining - 1;
                    self.finish_ping(name, false);
                } else {
                    self.finish_ping(name, true);
                }
                continue;
            }
            if due {
                self.devices.get_mut(name).unwrap().next_event_at = None;
                if passive {
                    self.publish(name);
                    self.schedule(name, false);
                } else if !self.paused.contains(name) {
                    self.due_queue.push_back(name.clone());
                }
            }
        }

        if now >= self.next_ping_slot {
            if let Some(name) = self.due_queue.pop_front() {
                self.next_ping_slot = now + 2;
                self.start_ping(&name);
            }
        }
    }

    fn start_ping(&mut self, name: &str) {
        let Some(device) = self.devices.get(name) else {
            return;
        };
        if !self.enabled(device) {
            return;
        }
        let available = device.last_published == Some(true) || self.available(device);
        let attempts = if available { 2 } else { 1 };
        self.ping_queue.push(name.to_string());

        let remaining = self.devices.get(name).unwrap().failures_remaining;
        if remaining > 0 {
            self.devices.get_mut(name).unwrap().ping_calls.push(true);
            self.devices.get_mut(name).unwrap().failures_remaining = remaining - 1;
            self.devices.get_mut(name).unwrap().fail_first_ping = false;
            if attempts == 2 && remaining > 1 {
                let now = self.now_seconds;
                self.devices.get_mut(name).unwrap().retry_at = Some(now + 3);
                self.devices.get_mut(name).unwrap().ping_calls.push(false);
                return;
            }
            self.finish_ping(name, false);
        } else {
            self.devices.get_mut(name).unwrap().ping_calls.push(true);
            self.finish_ping(name, true);
        }
    }

    fn finish_ping(&mut self, name: &str, success: bool) {
        self.ping_queue.retain(|queued| queued != name);
        if success {
            self.publish(name);
            self.schedule(name, true);
            return;
        }

        let (backoff_enabled, current_backoff, available) = {
            let device = self.devices.get(name).unwrap();
            (
                device.backoff_enabled && self.backoff_enabled,
                device.backoff,
                device.last_published == Some(true),
            )
        };
        let pause_on = self
            .device_pause
            .get(name)
            .copied()
            .or(self.pause_on_backoff_gt)
            .unwrap_or(0.0);
        if backoff_enabled {
            if pause_on > 0.0 && current_backoff >= pause_on {
                self.paused.insert(name.to_string());
            } else {
                self.devices.get_mut(name).unwrap().backoff =
                    current_backoff * if available { 1.5 } else { 2.0 };
            }
        }
        self.publish(name);
        if !self.paused.contains(name) {
            self.schedule(name, false);
        }
    }

    pub fn advance_minutes(&mut self, minutes: u64) {
        self.advance_seconds(minutes * 60)
    }

    pub fn advance_hours(&mut self, hours: u64) {
        self.advance_seconds(hours * 60 * 60)
    }

    pub fn on_last_seen_changed(&mut self, device: &str) {
        let Some(entry) = self.devices.get(device) else {
            return;
        };
        if !self.enabled(entry) {
            return;
        }
        let explicit = entry.availability_explicit;
        if let Some(entry) = self.devices.get_mut(device) {
            entry.retry_at = None;
        }
        if explicit {
            self.publish(device);
        } else {
            self.published.push((
                format!("zigbee2mqtt/{device}/availability"),
                json!({"state": "offline"}),
                json!({"retain": true, "qos": 1}),
            ));
        }
        self.schedule(device, true);
    }

    pub fn on_device_leave(&mut self, device: &str) {
        self.clear_device(device);
    }

    pub fn on_device_remove(&mut self, device: &str) {
        self.clear_device(device);
    }

    fn clear_device(&mut self, name: &str) {
        if let Some(device) = self.devices.get_mut(name) {
            device.removed = true;
            device.next_event_at = None;
            device.retry_at = None;
        }
        self.due_queue.retain(|queued| queued != name);
    }

    pub fn on_device_announce(&mut self, device: &str) {
        let now = self.now_seconds;
        if let Some(device) = self.devices.get_mut(device) {
            if device.retrieve_at.is_none() {
                device.retrieve_at = Some(now + 2);
            }
        }
    }

    pub fn on_rename(&mut self, from: &str, to: &str) {
        let old_topic = format!("zigbee2mqtt/{from}/availability");
        self.published.push((
            old_topic,
            Value::String(String::new()),
            json!({"retain": true, "qos": 1}),
        ));
        if let Some(mut device) = self.devices.remove(from) {
            device.last_published = None;
            self.devices.insert(to.to_string(), device);
        }
        for members in self.groups.values_mut() {
            for member in members.iter_mut() {
                if member == from {
                    *member = to.to_string();
                }
            }
        }
        self.publish(to);
    }
}
