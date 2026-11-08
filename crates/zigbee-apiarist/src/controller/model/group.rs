// Original: .upstream/zigbee-herdsman/src/controller/model/group.ts
//
// Standalone Group model. Device/Endpoint lookups are kept as (IEEE, endpoint)
// pairs so this type has no dependency on the runtime controller.

use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct Group {
    pub database_id: u64,
    pub group_id: u64,
    pub members: Vec<(String, u8)>,
    pub meta: Value,
}

impl Group {
    pub fn new(database_id: u64, group_id: u64) -> Self {
        Self {
            database_id,
            group_id,
            members: Vec::new(),
            meta: json!({}),
        }
    }

    /// Upstream `addMember(endpoint)`.
    pub fn add_member(&mut self, device_ieee_address: &str, endpoint_id: u8) {
        let member = (device_ieee_address.to_string(), endpoint_id);
        if !self.members.contains(&member) {
            self.members.push(member);
        }
    }

    /// Upstream `hasMember(endpoint)`.
    pub fn has_member(&self, device_ieee_address: &str, endpoint_id: u8) -> bool {
        self.members
            .iter()
            .any(|(ieee, endpoint)| ieee == device_ieee_address && *endpoint == endpoint_id)
    }

    /// Upstream `removeMember(endpoint)`.
    pub fn remove_member(&mut self, device_ieee_address: &str, endpoint_id: u8) {
        self.members
            .retain(|(ieee, endpoint)| !(ieee == device_ieee_address && *endpoint == endpoint_id));
    }

    /// Upstream `removeFromDatabase()` returns the record to delete.
    pub fn remove_from_database(&self) -> Value {
        json!({"id": self.database_id, "type": "Group"})
    }

    /// Upstream `removeFromNetwork()`: members for which `removeFromGroup`
    /// must be called.
    pub fn remove_from_network(&self) -> Vec<(String, u8)> {
        self.members.clone()
    }

    /// Upstream `toDatabaseRecord()` / `save()`.
    pub fn save(&self) -> Value {
        json!({
            "id": self.database_id,
            "type": "Group",
            "groupID": self.group_id,
            "members": self.members.iter().map(|(ieee, endpoint)| json!({
                "deviceIeeeAddr": ieee,
                "endpointID": endpoint,
            })).collect::<Vec<_>>(),
            "meta": self.meta,
        })
    }
}
