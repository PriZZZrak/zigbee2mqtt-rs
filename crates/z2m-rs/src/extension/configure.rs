// Original: .upstream/zigbee2mqtt/lib/extension/configure.ts
// TODO: port the configure extension.
//
// The Rust port records endpoint read/bind/write calls and bridge responses.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct Endpoint {
    /// `(cluster, attributes)` passed to `endpoint.read`.
    pub reads: Vec<(String, Vec<String>)>,
    /// `(cluster, coordinator)` passed to `endpoint.bind`.
    pub binds: Vec<(String, String)>,
    /// `(cluster, payload, options)` passed to `endpoint.write`.
    pub writes: Vec<(String, Value, Value)>,
}

#[derive(Debug, Clone)]
pub struct Device {
    pub friendly_name: String,
    pub ieee_address: String,
    /// `Router` / `EndDevice`.
    pub kind: String,
    pub endpoints: BTreeMap<u8, Endpoint>,
    pub meta_configured: Option<String>,
    pub interview_state: String,
    pub disabled: bool,
    pub has_configure: bool,
    pub definition_version: String,
    /// Number of times the definition `configure` ran.
    pub configure_calls: usize,
    /// When set, the next configure attempt fails with this message.
    pub configure_error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Configure {
    pub devices: BTreeMap<String, Device>,
    pub coordinator_ieee: String,
    pub configuring: BTreeSet<String>,
    pub attempts: BTreeMap<String, usize>,
    pub published: Vec<(String, Value, Value)>,
    pub devices_changed: usize,
    pub exposes_changed: Vec<String>,
    pub logs: Vec<(String, String)>,
}

impl Configure {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_device(&mut self, friendly_name: &str, ieee: &str, kind: &str, endpoints: &[u8]) {
        let endpoints = endpoints
            .iter()
            .map(|endpoint| (*endpoint, Endpoint::default()))
            .collect();
        self.devices.insert(
            friendly_name.to_string(),
            Device {
                friendly_name: friendly_name.to_string(),
                ieee_address: ieee.to_string(),
                kind: kind.to_string(),
                endpoints,
                meta_configured: None,
                interview_state: "Successful".to_string(),
                disabled: false,
                has_configure: true,
                definition_version: "0.0.0".to_string(),
                configure_calls: 0,
                configure_error: None,
            },
        );
    }

    pub fn device_mut(&mut self, friendly_name: &str) -> &mut Device {
        self.devices.get_mut(friendly_name).expect("device")
    }

    pub fn disable(&mut self, friendly_name: &str) {
        self.device_mut(friendly_name).disabled = true;
    }

    /// Upstream `extension.start()`.
    pub fn start(&mut self) {
        todo!()
    }

    pub fn on_device_joined(&mut self, friendly_name: &str) {
        todo!()
    }

    pub fn on_last_seen_changed(&mut self, friendly_name: &str) {
        todo!()
    }

    pub fn on_reconfigure(&mut self, friendly_name: &str) {
        todo!()
    }

    pub fn on_device_interview(&mut self, friendly_name: &str) {
        todo!()
    }

    /// Upstream `bridge/request/device/configure`.
    pub fn on_mqtt_configure(&mut self, payload: &Value) {
        todo!()
    }
}
