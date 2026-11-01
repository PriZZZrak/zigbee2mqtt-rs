use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: u16,
    pub friendly_name: String,
    pub members: Vec<GroupMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    pub ieee_address: String,
    pub endpoint: u8,
}

pub struct GroupRegistry {
    pub(crate) inner: DashMap<u16, Group>,
    pub(crate) by_name: DashMap<String, u16>,
    pub(crate) next_id: std::sync::atomic::AtomicU16,
}

impl GroupRegistry {
    pub fn new() -> Self {
        Self {
            inner: DashMap::new(),
            by_name: DashMap::new(),
            next_id: std::sync::atomic::AtomicU16::new(1),
        }
    }

    pub fn add(&self, friendly_name: String) -> Group {
        let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let group = Group { id, friendly_name: friendly_name.clone(), members: Vec::new() };
        self.by_name.insert(friendly_name.clone(), id);
        self.inner.insert(id, group.clone());
        group
    }

    pub fn get_by_id(&self, id: u16) -> Option<Group> {
        self.inner.get(&id).map(|v| v.clone())
    }

    pub fn get_by_name(&self, name: &str) -> Option<Group> {
        let id = self.by_name.get(name).map(|v| *v)?;
        self.get_by_id(id)
    }

    pub fn remove_by_id(&self, id: u16) -> Option<Group> {
        if let Some((_, g)) = self.inner.remove(&id) {
            self.by_name.remove(&g.friendly_name);
            Some(g)
        } else { None }
    }

    pub fn rename(&self, from: &str, to: String) -> Option<Group> {
        let id = self.by_name.get(from).map(|v| *v)?;
        let mut entry = self.inner.get_mut(&id)?;
        self.by_name.remove(from);
        entry.friendly_name = to.clone();
        self.by_name.insert(to, id);
        Some(entry.clone())
    }

    pub fn add_member(&self, gid: u16, ieee_address: String, endpoint: u8) -> bool {
        if let Some(mut g) = self.inner.get_mut(&gid) {
            if !g.members.iter().any(|m| m.ieee_address == ieee_address && m.endpoint == endpoint) {
                g.members.push(GroupMember { ieee_address, endpoint });
            }
            return true;
        }
        false
    }

    pub fn remove_member(&self, gid: u16, ieee_address: &str, endpoint: u8) -> bool {
        if let Some(mut g) = self.inner.get_mut(&gid) {
            g.members.retain(|m| !(m.ieee_address == ieee_address && m.endpoint == endpoint));
            return true;
        }
        false
    }

    pub fn clear_members(&self, gid: u16) -> bool {
        if let Some(mut g) = self.inner.get_mut(&gid) {
            g.members.clear();
            return true;
        }
        false
    }

    pub fn all(&self) -> Vec<Group> {
        self.inner.iter().map(|v| v.value().clone()).collect()
    }

    pub fn to_json(&self) -> serde_json::Value {
        let groups: Vec<_> = self.all().into_iter().map(|g| json!({
            "id": g.id,
            "friendly_name": g.friendly_name,
            "members": g.members,
            "scenes": []
        })).collect();
        json!(groups)
    }
}

impl Default for GroupRegistry {
    fn default() -> Self { Self::new() }
}
