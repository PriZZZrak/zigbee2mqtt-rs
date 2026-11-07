// Original: .upstream/zigbee2mqtt/lib/extension/bind.ts
// TODO: port the bind extension.
//
// The Rust port records bind/unbind/clear calls and bridge responses.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct BindDevice {
    pub ieee_address: String,
    pub endpoints: BTreeSet<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Bind {
    pub devices: BTreeMap<String, BindDevice>,
    pub groups: BTreeSet<String>,
    /// `(topic, payload, options)`.
    pub published: Vec<(String, Value, Value)>,
    pub logs: Vec<(String, String)>,
    pub clear_bindings_calls: BTreeMap<String, usize>,
    pub devices_changed: usize,
}

impl Bind {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_device(&mut self, friendly_name: &str, ieee: &str, endpoints: &[&str]) {
        self.devices.insert(
            friendly_name.to_string(),
            BindDevice {
                ieee_address: ieee.to_string(),
                endpoints: endpoints.iter().map(|e| (*e).to_string()).collect(),
            },
        );
    }

    pub fn add_group(&mut self, friendly_name: &str) {
        self.groups.insert(friendly_name.to_string());
    }

    /// Upstream `bridge/request/device/{bind,unbind,binds/clear}`.
    pub fn on_message(&mut self, topic: &str, payload: &Value) {
        todo!()
    }
}
