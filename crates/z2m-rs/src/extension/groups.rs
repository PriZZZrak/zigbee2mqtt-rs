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
