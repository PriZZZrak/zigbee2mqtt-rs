// Original: .upstream/zigbee2mqtt/lib/extension/groups.ts
// TODO: port the groups extension.
//
// The Rust port keeps group membership and records bridge/response publishes.

use std::collections::BTreeMap;

use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct Group {
    pub friendly_name: String,
    /// `device:endpoint` member identities.
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Groups {
    pub groups: BTreeMap<String, Group>,
    /// friendly_name -> (endpoint alias -> endpoint id).
    pub devices: BTreeMap<String, BTreeMap<String, String>>,
    /// `(topic, payload, options)` published on the MQTT bus.
    pub published: Vec<(String, Value, Value)>,
    pub fail_add: Option<String>,
    pub fail_remove: Option<String>,
}

impl Groups {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_group(&mut self, friendly_name: &str) {
        self.groups
            .entry(friendly_name.to_string())
            .or_insert_with(|| Group {
                friendly_name: friendly_name.to_string(),
                members: Vec::new(),
            });
    }

    pub fn add_device(&mut self, friendly_name: &str, endpoints: &[(&str, &str)]) {
        let map = endpoints
            .iter()
            .map(|(alias, id)| ((*alias).to_string(), (*id).to_string()))
            .collect();
        self.devices.insert(friendly_name.to_string(), map);
    }

    pub fn member(&mut self, group: &str) -> &mut Vec<String> {
        &mut self.groups.get_mut(group).expect("group").members
    }

    /// Upstream MQTT `bridge/request/group/members/{add,remove,remove_all}`.
    pub fn on_message(&mut self, topic: &str, payload: &Value) {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct GroupConfig {
    pub optimistic: bool,
    pub filtered_attributes: Vec<String>,
    pub off_state: Option<String>,
}

impl Default for GroupConfig {
    fn default() -> Self {
        Self {
            optimistic: true,
            filtered_attributes: Vec::new(),
            off_state: None,
        }
    }
}

/// State-aggregation counterpart of the groups extension (`controller.state`).
#[derive(Debug, Clone, Default)]
pub struct GroupState {
    pub members: BTreeMap<String, Vec<(String, String)>>,
    pub group_config: BTreeMap<String, GroupConfig>,
    pub device_states: BTreeMap<String, Value>,
    pub device_retain: BTreeMap<String, bool>,
    pub disabled: std::collections::BTreeSet<String>,
    pub group_states: BTreeMap<String, Value>,
    /// `(topic, payload, options)`.
    pub published: Vec<(String, Value, Value)>,
    pub command_calls: BTreeMap<String, usize>,
}

impl GroupState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_group(&mut self, name: &str) {
        self.members.entry(name.to_string()).or_default();
        self.group_config.entry(name.to_string()).or_default();
    }

    pub fn add_member(&mut self, group: &str, device: &str, endpoint: &str) {
        self.members
            .entry(group.to_string())
            .or_default()
            .push((device.to_string(), endpoint.to_string()));
    }

    pub fn set_group_config(&mut self, group: &str, config: GroupConfig) {
        self.group_config.insert(group.to_string(), config);
    }

    pub fn set_retain(&mut self, device: &str, retain: bool) {
        self.device_retain.insert(device.to_string(), retain);
    }

    pub fn disable(&mut self, device: &str) {
        self.disabled.insert(device.to_string());
    }

    /// Upstream MQTT `<device>/set`.
    pub fn on_device_set(&mut self, device: &str, payload: &Value) {
        todo!()
    }

    /// Upstream MQTT `<group>/set`.
    pub fn on_group_set(&mut self, group: &str, payload: &Value) {
        todo!()
    }

    /// Upstream `zcl` attribute report.
    pub fn on_attribute_report(&mut self, device: &str, endpoint: &str, data: &Value) {
        todo!()
    }
}
