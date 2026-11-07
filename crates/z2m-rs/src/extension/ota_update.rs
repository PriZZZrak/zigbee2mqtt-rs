// Original: .upstream/zigbee2mqtt/lib/extension/otaUpdate.ts
// TODO: port OTA check/update/schedule/abort flows.

use std::collections::HashMap;

use serde_json::{json, Value};

/// Upstream `OtaDataSettings`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OtaDataSettings {
    /// Upstream camelCase: `requestTimeout`.
    pub request_timeout: u64,
    /// Upstream camelCase: `responseDelay`.
    pub response_delay: u64,
    /// Upstream camelCase: `baseSize`.
    pub base_size: u64,
}

impl Default for OtaDataSettings {
    fn default() -> Self {
        Self {
            request_timeout: 150000,
            response_delay: 250,
            base_size: 50,
        }
    }
}

/// One `device.updateOta(...)` call.
#[derive(Debug, Clone, PartialEq)]
pub struct OtaCall {
    /// Upstream `source` (`{url?, downgrade}`) or `undefined` for scheduled.
    pub source: Option<Value>,
    pub data_settings: OtaDataSettings,
}

/// Model of `lib/extension/otaUpdate.ts` used by the ported tests.
#[derive(Debug)]
pub struct OtaUpdate {
    /// Upstream configured `dataSettings`.
    pub data_settings: OtaDataSettings,
    /// Upstream `ota.zigbee_ota_override_index_location`.
    pub index_location: Option<String>,
    /// `mockMQTTPublishAsync` calls: `(topic, payload, options)`.
    pub published: Vec<(String, Value, Value)>,
    /// `device.scheduledOta` per device id.
    pub scheduled: HashMap<String, Value>,
    /// Device ids with an OTA check/update in progress.
    pub in_progress: Vec<String>,
    /// Controller state per device id.
    pub state: HashMap<String, Value>,
    /// `mockLogger.info` calls.
    pub info_logs: Vec<String>,
    /// `mockLogger.error` calls.
    pub error_logs: Vec<String>,
    pub saved_devices: Vec<String>,
    pub interviewed_devices: Vec<String>,
    pub configured_devices: Vec<String>,
    /// `device.updateOta(...)` calls.
    pub update_calls: Vec<OtaCall>,
    /// `device.checkOta(...)` calls (source argument).
    pub check_calls: Vec<Value>,
    /// `device.scheduleOta(...)` calls.
    pub scheduled_calls: Vec<Value>,
    /// `endpoint.commandResponse(...)` calls: `(command, payload, tsn)`.
    pub command_responses: Vec<(String, Value, Option<u8>)>,
}

impl OtaUpdate {
    pub fn new(index_location: Option<String>) -> Self {
        Self {
            data_settings: OtaDataSettings::default(),
            index_location,
            published: Vec::new(),
            scheduled: HashMap::new(),
            in_progress: Vec::new(),
            state: HashMap::new(),
            info_logs: Vec::new(),
            error_logs: Vec::new(),
            saved_devices: Vec::new(),
            interviewed_devices: Vec::new(),
            configured_devices: Vec::new(),
            update_calls: Vec::new(),
            check_calls: Vec::new(),
            scheduled_calls: Vec::new(),
            command_responses: Vec::new(),
        }
    }

    /// Upstream extension `start()` (clears stale update state).
    pub async fn start(&mut self) {
        todo!()
    }

    /// Upstream `onMQTTMessage()` / ZHE `message` handler.
    pub async fn handle_message(&mut self, _topic: &str, _message: Value) {
        todo!()
    }

    /// Upstream dev/test `clearState()`.
    pub fn clear_state(&mut self) {
        self.published.clear();
        self.update_calls.clear();
        self.check_calls.clear();
        self.scheduled_calls.clear();
        self.command_responses.clear();
    }

    /// Upstream controller startup: `{update: {state: "idle"}}`.
    pub fn clear_startup_state(&mut self, device_id: &str) {
        self.state
            .insert(device_id.to_string(), json!({"update": {"state": "idle"}}));
    }

    pub fn has_publish(&self, topic: &str, payload: Value) -> bool {
        self.published
            .iter()
            .any(|(published_topic, published_payload, _)| {
                published_topic == topic && *published_payload == payload
            })
    }

    pub fn response<'a>(&'a self, api: &str) -> Option<&'a Value> {
        self.published
            .iter()
            .find(|(topic, _, _)| {
                topic == &format!("zigbee2mqtt/bridge/response/device/ota_update/{api}")
            })
            .map(|(_, payload, _)| payload)
    }
}
