// Original: .upstream/zigbee2mqtt/lib/extension/bind.ts
// TODO: port the bind extension.
//
// The Rust port records bind/unbind/configure-reporting calls and bridge
// responses.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct BindEndpoint {
    pub output_clusters: BTreeSet<String>,
    pub input_clusters: BTreeSet<String>,
    pub bind_calls: Vec<(String, String)>,
    pub unbind_calls: Vec<(String, String)>,
    pub reads: Vec<(String, Vec<String>)>,
    pub configure_reporting: Vec<(String, Value)>,
    /// Existing binds: `(cluster, target)`.
    pub binds: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct BindDevice {
    pub ieee_address: String,
    pub endpoints: BTreeMap<String, BindEndpoint>,
}

#[derive(Debug, Clone, Default)]
pub struct BindGroup {
    pub id: u32,
    pub members: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct Bind {
    pub devices: BTreeMap<String, BindDevice>,
    pub groups: BTreeMap<String, BindGroup>,
    pub coordinator_ieee: String,
    /// `(topic, payload, options)`.
    pub published: Vec<(String, Value, Value)>,
    pub logs: Vec<(String, String)>,
    pub clear_bindings_calls: BTreeMap<String, usize>,
    pub devices_changed: usize,
    pub next_group_id: u32,
    /// When true, bind calls throw `failed`.
    pub bind_error: bool,
}

impl Bind {
    pub fn new() -> Self {
        Self {
            next_group_id: 1,
            ..Default::default()
        }
    }

    pub fn add_device(&mut self, friendly_name: &str, ieee: &str, endpoints: &[&str]) {
        let endpoints = endpoints
            .iter()
            .map(|endpoint| ((*endpoint).to_string(), BindEndpoint::default()))
            .collect();
        self.devices.insert(
            friendly_name.to_string(),
            BindDevice {
                ieee_address: ieee.to_string(),
                endpoints,
            },
        );
    }

    pub fn add_group(&mut self, friendly_name: &str) -> u32 {
        let id = self.next_group_id;
        self.next_group_id += 1;
        self.groups.insert(
            friendly_name.to_string(),
            BindGroup {
                id,
                members: Vec::new(),
            },
        );
        id
    }

    pub fn add_group_member(&mut self, group: &str, device: &str, endpoint: &str) {
        self.groups
            .get_mut(group)
            .expect("group")
            .members
            .push((device.to_string(), endpoint.to_string()));
    }

    pub fn endpoint(&mut self, device: &str, endpoint: &str) -> &mut BindEndpoint {
        self.devices
            .get_mut(device)
            .expect("device")
            .endpoints
            .get_mut(endpoint)
            .expect("endpoint")
    }

    /// Upstream `bridge/request/device/{bind,unbind,binds/clear}`.
    pub fn on_message(&mut self, topic: &str, payload: &Value) {
        todo!()
    }
}
