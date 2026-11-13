// Original: .upstream/zigbee2mqtt/lib/model/entity.ts (no such file upstream)
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// Upstream `Device` (`lib/model/device.ts:25`) and `Group` (`lib/model/group.ts:5`)
// are separate classes used through `Device | Group` unions. Rust has no
// inheritance, so the common surface lives in this module.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::{json, Value};

/// Upstream `PublishEntityState` (`types/types.d.ts:26`).
pub type EntityFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
pub type PublishEntityState =
    Arc<dyn Fn(Arc<dyn Entity>, Value, Option<String>) -> EntityFuture + Send + Sync>;

/// Upstream `entity.ID`: an IEEE address for devices, a numeric group ID for
/// groups (`model/device.ts:33`, `model/group.ts:10`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntityId {
    Ieee(String),
    Group(u64),
}

impl EntityId {
    /// `entity.ID` as a `state.json` / event-bus key (JS object keys are strings).
    pub fn key(&self) -> String {
        match self {
            Self::Ieee(ieee) => ieee.clone(),
            Self::Group(id) => id.to_string(),
        }
    }

    /// Upstream `State.load()` key handling (`state.ts:79`): keys starting with
    /// `0x` stay strings, everything else becomes a number.
    pub fn from_state_key(key: &str) -> Self {
        if key.starts_with("0x") {
            Self::Ieee(key.to_string())
        } else {
            Self::Group(key.parse::<u64>().unwrap_or(0))
        }
    }

    pub fn as_json(&self) -> Value {
        match self {
            Self::Ieee(ieee) => json!(ieee),
            Self::Group(id) => json!(id),
        }
    }
}

/// Upstream `Device | Group` surface used by `State`, `Controller` and the
/// bridge extensions.
pub trait Entity: Send + Sync {
    fn entity_id(&self) -> EntityId;

    /// Upstream `entity.name` (`model/device.ts:40`, `model/group.ts:17`).
    fn name(&self) -> String;

    /// Upstream `entity.options` (`model/device.ts:35`, `model/group.ts:13`),
    /// including `device_options` defaults for devices.
    fn options(&self) -> Value;

    fn is_device(&self) -> bool;

    /// Upstream `resolveDefinition(ignoreCache)`; groups have no definition.
    fn resolve_definition(&self, _ignore_cache: bool) {}

    /// Upstream `device.definition`; groups have none.
    fn definition(&self) -> Option<zigbee_converters::DefinitionRef> {
        None
    }

    /// Upstream `device.interviewed`.
    fn interviewed(&self) -> bool {
        true
    }

    /// Upstream `device.zh`; groups have no herdsman device.
    fn zh_device(&self) -> Option<Arc<zigbee_apiarist::controller::model::device::Device>> {
        None
    }

    /// Upstream `device.zh.lastSeen`.
    fn last_seen(&self) -> Option<u64> {
        None
    }

    /// Upstream `device.zh.linkquality`.
    fn linkquality(&self) -> Option<u8> {
        None
    }

    fn is_group(&self) -> bool {
        !self.is_device()
    }

    /// Upstream `entity.options.filtered_cache || []` (`state.ts:119`).
    fn filtered_cache(&self) -> Vec<String> {
        self.options()
            .get("filtered_cache")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// JSON snapshot replacing the upstream class instance on the event bus.
    fn event_value(&self) -> Value;
}
