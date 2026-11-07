// Original: .upstream/zigbee-herdsman/src/controller/model/device.ts
// TODO: port the full Device/Endpoint model.

use std::collections::HashMap;

use serde_json::Value;

use crate::zspec::zcl::zcl_frame::Frame;

/// Upstream `ZclPayload`.
#[derive(Debug, Clone, PartialEq)]
pub struct ZclPayload {
    /// Upstream camelCase: `clusterID`.
    pub cluster_id: u16,
    pub address: u16,
    pub data: Vec<u8>,
    pub endpoint: u8,
    pub linkquality: u8,
    /// Upstream camelCase: `groupID`.
    pub group_id: u16,
    /// Upstream camelCase: `wasBroadcast`.
    pub was_broadcast: bool,
    /// Upstream camelCase: `destinationEndpoint`.
    pub destination_endpoint: u8,
}

/// Upstream `Endpoint` response calls recorded by the tests.
#[derive(Debug, Default)]
pub struct Endpoint {
    pub id: u8,
    /// `endpoint.readResponse(...)` calls.
    pub read_response_calls: Vec<Value>,
    /// `endpoint.command(...)` calls.
    pub command_calls: Vec<Value>,
    /// `endpoint.read(...)` calls.
    pub read_calls: Vec<Value>,
    /// `endpoint.defaultResponse(...)` calls.
    pub default_response_calls: Vec<Value>,
    /// Force the next `command(...)` to reject.
    pub fail_next_command: bool,
    /// Force the next `read(...)` to reject.
    pub fail_next_read: bool,
    /// Result returned by `read(...)`.
    pub next_read_result: Value,
}

impl Endpoint {
    pub fn new(id: u8) -> Self {
        Self {
            id,
            next_read_result: Value::Null,
            ..Self::default()
        }
    }
}

/// Upstream `Device`.
#[derive(Debug)]
pub struct Device {
    pub device_type: String,
    /// Upstream camelCase: `ieeeAddr`.
    pub ieee_addr: String,
    /// Upstream camelCase: `networkAddress`.
    pub network_address: u16,
    /// Upstream camelCase: `manufacturerID`.
    pub manufacturer_id: u16,
    /// Upstream camelCase: `manufacturerName`.
    pub manufacturer_name: String,
    /// Upstream camelCase: `powerSource`.
    pub power_source: String,
    /// Upstream camelCase: `modelID`.
    pub model_id: String,
    /// Upstream camelCase: `interviewState`.
    pub interview_state: String,
    pub endpoints: Vec<Endpoint>,
    /// Upstream `customReadResponse`: `Some(false)` handles read, `Some(true)` skips reply.
    pub custom_read_response: Option<bool>,
    /// Upstream camelCase: `checkinInterval`.
    pub checkin_interval: Option<u64>,
    /// Upstream `options`.
    pub options: Value,
    /// Upstream `meta`.
    pub meta: Value,
    /// Stored ZCL attributes.
    pub attrs: HashMap<(u16, u16), Value>,
}

impl Device {
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        device_type: &str,
        ieee_addr: &str,
        network_address: u16,
        manufacturer_id: u16,
        manufacturer_name: &str,
        power_source: &str,
        model_id: &str,
        interview_state: &str,
        endpoints: Option<Vec<u8>>,
    ) -> Self {
        let mut device = Self {
            device_type: device_type.to_string(),
            ieee_addr: ieee_addr.to_string(),
            network_address,
            manufacturer_id,
            manufacturer_name: manufacturer_name.to_string(),
            power_source: power_source.to_string(),
            model_id: model_id.to_string(),
            interview_state: interview_state.to_string(),
            endpoints: Vec::new(),
            custom_read_response: None,
            checkin_interval: None,
            options: Value::Null,
            meta: Value::Null,
            attrs: HashMap::new(),
        };
        if let Some(endpoints) = endpoints {
            for id in endpoints {
                device.create_endpoint(id);
            }
        }
        device
    }

    /// Upstream `Device.resetCache()`.
    pub fn reset_cache() {}

    /// Upstream `device.createEndpoint(id)`.
    pub fn create_endpoint(&mut self, id: u8) {
        self.endpoints.push(Endpoint::new(id));
    }

    /// Upstream `device.getEndpoint(id)`.
    pub fn get_endpoint(&self, id: u8) -> Option<&Endpoint> {
        self.endpoints.iter().find(|endpoint| endpoint.id == id)
    }

    pub fn endpoint_mut(&mut self, index: usize) -> &mut Endpoint {
        &mut self.endpoints[index]
    }

    /// Upstream `device.onZclData(...)`.
    pub async fn on_zcl_data(
        &mut self,
        _data_payload: &ZclPayload,
        _frame: &Frame,
        _endpoint_index: usize,
        _options: Value,
    ) {
        todo!()
    }
}
