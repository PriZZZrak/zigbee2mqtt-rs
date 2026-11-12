// Original: .upstream/zigbee2mqtt/lib/model/group.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use zigbee_apiarist::controller::model::group::Group as ZhGroup;

use crate::model::device::Device;
use crate::model::entity::{Entity, EntityId};
use crate::util::settings::Settings;
use crate::util::utils::DEFAULT_BIND_GROUP_ID;

pub type ResolveDevice = Arc<dyn Fn(&str) -> Option<Arc<Device>> + Send + Sync>;

/// Upstream `Group` (`lib/model/group.ts:5`).
pub struct Group {
    pub zh: Arc<ZhGroup>,
    resolve_device: ResolveDevice,
    settings: Arc<Mutex<Settings>>,
}

impl Group {
    /// Upstream constructor (`model/group.ts:21`).
    pub fn new(
        zh: Arc<ZhGroup>,
        resolve_device: ResolveDevice,
        settings: Arc<Mutex<Settings>>,
    ) -> Self {
        Self {
            zh,
            resolve_device,
            settings,
        }
    }

    /// Upstream `get ID()` (`model/group.ts:10`).
    pub fn id(&self) -> u64 {
        self.zh.group_id as u64
    }

    /// Upstream `get options()` (`model/group.ts:13`).
    pub fn options(&self) -> Value {
        self.settings
            .lock()
            .unwrap()
            .get_group(&self.id().to_string())
            .unwrap_or_else(|| json!({}))
    }

    /// Upstream `get name()` (`model/group.ts:17`).
    pub fn name(&self) -> String {
        self.options()
            .get("friendly_name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| self.id().to_string())
    }

    /// Upstream `ensureInSettings()` (`model/group.ts:26`).
    pub fn ensure_in_settings(&self, settings: &mut Settings) {
        if self.id() != DEFAULT_BIND_GROUP_ID as u64
            && settings.get_group(&self.id().to_string()).is_none()
        {
            settings.add_group(&self.name(), Some(&self.id().to_string()));
        }
    }

    /// Upstream `hasMember(device)` (`model/group.ts:32`).
    pub fn has_member(&self, device: &Device) -> bool {
        let members = self.zh.members();
        device
            .zh
            .endpoints()
            .iter()
            .any(|endpoint| members.iter().any(|member| Arc::ptr_eq(member, endpoint)))
    }

    /// Upstream `membersDevices()` (`model/group.ts:36`).
    pub fn members_devices(&self) -> Vec<Arc<Device>> {
        self.zh
            .members()
            .iter()
            .filter_map(|member| (self.resolve_device)(&member.device_ieee_address.lock().unwrap()))
            .collect()
    }

    pub fn is_device(&self) -> bool {
        false
    }

    pub fn is_group(&self) -> bool {
        true
    }
}

impl Entity for Group {
    fn entity_id(&self) -> EntityId {
        EntityId::Group(self.id())
    }

    fn name(&self) -> String {
        Group::name(self)
    }

    fn options(&self) -> Value {
        Group::options(self)
    }

    fn is_device(&self) -> bool {
        false
    }

    fn event_value(&self) -> Value {
        json!({
            "ID": self.id(),
            "name": Group::name(self),
        })
    }
}
