// Original: .upstream/zigbee2mqtt/lib/model/device.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::sync::{Arc, Mutex};

use serde_json::{json, Map, Value};
use zigbee_apiarist::controller::database::DeviceType;
use zigbee_apiarist::controller::model::device::{Device as ZhDevice, InterviewState};
use zigbee_apiarist::controller::model::endpoint::Endpoint;

use crate::model::entity::{Entity, EntityId};
use crate::util::settings::Settings;

/// Upstream `Device` (`lib/model/device.ts:16`).
///
/// R1 has no `zigbee-herdsman-converters` (AGENTS §2), so `definition` never
/// exists: `isSupported` is only true for the coordinator and
/// definition-dependent helpers fall back to their upstream no-definition
/// branches.
pub struct Device {
    pub zh: Arc<ZhDevice>,
    settings: Arc<Mutex<Settings>>,
}

impl Device {
    pub fn new(zh: Arc<ZhDevice>, settings: Arc<Mutex<Settings>>) -> Self {
        Self { zh, settings }
    }

    /// Upstream `get ieeeAddr()` (`model/device.ts:28`).
    pub fn ieee_addr(&self) -> String {
        self.zh.ieee_addr()
    }

    /// Upstream `get ID()` (`model/device.ts:33`).
    pub fn id(&self) -> String {
        self.zh.ieee_addr()
    }

    /// Upstream `get options()` (`model/device.ts:35`):
    /// `{...settings.get().device_options, ...deviceOptions}`.
    pub fn options(&self) -> Value {
        let settings = self.settings.lock().unwrap();

        let mut options = Map::new();
        if let Some(Value::Object(defaults)) = settings.get().get("device_options").cloned() {
            for (key, value) in defaults {
                options.insert(key, value);
            }
        }

        let device = settings
            .get_device(&self.ieee_addr())
            .unwrap_or_else(|| json!({"friendly_name": self.ieee_addr(), "ID": self.ieee_addr()}));
        if let Value::Object(device) = device {
            for (key, value) in device {
                options.insert(key, value);
            }
        }

        Value::Object(options)
    }

    /// Upstream `get name()` (`model/device.ts:40`).
    pub fn name(&self) -> String {
        if self.zh.r#type() == DeviceType::Coordinator {
            return "Coordinator".to_string();
        }

        self.options()
            .get("friendly_name")
            .and_then(Value::as_str)
            .unwrap_or(&self.ieee_addr())
            .to_string()
    }

    /// Upstream `get isSupported()` (`model/device.ts:43`). Without converters
    /// only the coordinator has a non-generated definition.
    pub fn is_supported(&self) -> bool {
        self.zh.r#type() == DeviceType::Coordinator
    }

    /// Upstream `get interviewed()` (`model/device.ts:62`).
    pub fn interviewed(&self) -> bool {
        matches!(
            self.zh.interview_state(),
            InterviewState::Successful | InterviewState::Failed
        )
    }

    /// Upstream `ensureInSettings()` (`model/device.ts:93`).
    pub fn ensure_in_settings(&self, settings: &mut Settings) {
        if self.zh.r#type() != DeviceType::Coordinator
            && settings.get_device(&self.ieee_addr()).is_none()
        {
            settings.add_device(&self.ieee_addr());
        }
    }

    /// Upstream `endpoint(key)` (`model/device.ts:99`). Endpoint *names* require
    /// a converter definition and therefore resolve to `None` in R1.
    pub fn endpoint(&self, key: Option<&str>) -> Option<Arc<Endpoint>> {
        let key = key.unwrap_or("default");

        if let Ok(id) = key.parse::<u8>() {
            return self.zh.get_endpoint(id);
        }

        if key == "default" {
            return self.zh.endpoints().first().cloned();
        }

        None
    }

    /// Upstream `endpointName(endpoint)` (`model/device.ts:117`); definition
    /// based, so always `None` in R1.
    pub fn endpoint_name(&self, _endpoint: &Endpoint) -> Option<String> {
        None
    }

    /// Upstream `getEndpointNames()` (`model/device.ts:135`); definition based,
    /// so empty in R1.
    pub fn get_endpoint_names(&self) -> Vec<String> {
        Vec::new()
    }

    pub fn is_device(&self) -> bool {
        true
    }

    pub fn is_group(&self) -> bool {
        false
    }
}

impl Entity for Device {
    fn entity_id(&self) -> EntityId {
        EntityId::Ieee(self.ieee_addr())
    }

    fn name(&self) -> String {
        Device::name(self)
    }

    fn options(&self) -> Value {
        Device::options(self)
    }

    fn is_device(&self) -> bool {
        true
    }

    fn event_value(&self) -> Value {
        json!({
            "ID": self.id(),
            "ieeeAddr": self.ieee_addr(),
            "name": Device::name(self),
            "type": self.zh.r#type().as_str(),
        })
    }
}
