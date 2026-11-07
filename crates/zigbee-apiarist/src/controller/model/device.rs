// Original: .upstream/zigbee-herdsman/src/controller/model/device.ts
// TODO: port the full Device/Endpoint model.

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::zspec::zcl::definition::{cluster, enums::DataType, status::Status};
use crate::zspec::zcl::zcl_frame::Frame;
use crate::zspec::zcl::zcl_header::{Direction, FrameType};

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
    /// Last default response emitted, used to de-duplicate handling of duplicate frames.
    pub last_default_response: Option<Value>,
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
            last_default_response: None,
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
        frame: &Frame,
        endpoint_index: usize,
        options: Value,
    ) {
        let cluster_name = frame.cluster.name;
        let command_name = frame.command.name;

        if command_name == "defaultRsp" {
            return;
        }

        if command_name == "read" {
            if self.custom_read_response == Some(true) {
                return;
            }
            let mut attributes = serde_json::Map::new();
            if let Some(items) = frame.payload.as_array() {
                for item in items {
                    let attr_id = item
                        .get("attrId")
                        .and_then(Value::as_u64)
                        .unwrap_or_default();
                    attributes.insert(
                        attr_id.to_string(),
                        json!({"value": null, "type": DataType::NO_DATA as u8}),
                    );
                }
            }
            self.endpoints[endpoint_index]
                .read_response_calls
                .push(json!([
                    frame.cluster.ID,
                    frame.header.transaction_sequence_number,
                    Value::Object(attributes),
                ]));
            return;
        }

        if cluster_name == "ssIasZone" && command_name == "enrollReq" {
            let endpoint = &mut self.endpoints[endpoint_index];
            endpoint.command_calls.push(json!([
                "ssIasZone",
                "enrollRsp",
                {"enrollrspcode": 0, "zoneid": 23},
                {
                    "transactionSequenceNumber": frame.header.transaction_sequence_number,
                    "disableDefaultResponse": true,
                },
            ]));
            if endpoint.fail_next_command {
                endpoint.fail_next_command = false;
                self.emit_default_response(frame, endpoint_index, Status::FAILURE);
            }
            return;
        }

        if cluster_name == "genPollCtrl" && command_name == "checkin" {
            let has_read = self.checkin_interval.is_none()
                && !self.endpoints[endpoint_index].next_read_result.is_null();

            self.endpoints[endpoint_index].command_calls.push(json!([
                "genPollCtrl",
                "checkinRsp",
                {
                    "startFastPolling": if has_read { 1 } else { 0 },
                    "fastPollTimeout": 0,
                },
                {
                    "transactionSequenceNumber": frame.header.transaction_sequence_number,
                    "disableDefaultResponse": true,
                    "sendPolicy": "immediate",
                },
            ]));

            if !has_read {
                if self.endpoints[endpoint_index].fail_next_command {
                    self.endpoints[endpoint_index].fail_next_command = false;
                    self.emit_default_response(frame, endpoint_index, Status::FAILURE);
                }
                return;
            }

            let read_result = self.endpoints[endpoint_index].next_read_result.clone();
            self.endpoints[endpoint_index].read_calls.push(json!([
                "genPollCtrl",
                ["checkinInterval"],
                {}
            ]));
            if let Some(interval) = read_result.get("checkinInterval").and_then(Value::as_u64) {
                self.checkin_interval = Some(interval);
            }

            self.endpoints[endpoint_index].command_calls.push(json!([
                "genPollCtrl",
                "fastPollStop",
                {},
                {"sendPolicy": "immediate"},
            ]));
            self.endpoints[endpoint_index].fail_next_command = false;
            return;
        }

        let direction = Direction::CLIENT_TO_SERVER as u8;
        let disable_default_response = frame.header.frame_control.disable_default_response;
        let device_disables = self
            .options
            .get("disableDefaultResponse")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let tuya_device =
            self.model_id.starts_with("TS0601") && self.manufacturer_name.starts_with("_TZE");

        let requested = options.get("defaultResponse").cloned();
        let status = match &requested {
            Some(Value::Bool(true)) => Some(Status::SUCCESS as u8),
            Some(Value::Number(number)) => number.as_u64().map(|value| value as u8),
            _ => None,
        };

        if tuya_device || (device_disables && status.is_none()) {
            return;
        }
        if disable_default_response && requested.is_none() {
            return;
        }

        emit_default_response_at(
            &mut self.endpoints[endpoint_index],
            frame,
            endpoint_index,
            self.last_default_response.as_ref(),
            status.unwrap_or(Status::SUCCESS as u8),
            direction,
        )
        .map(|value| self.last_default_response = Some(value));

        if frame.header.frame_control.frame_type == FrameType::GLOBAL && command_name == "read" {
            return;
        }
    }

    fn emit_default_response(&mut self, frame: &Frame, endpoint_index: usize, status: Status) {
        let value = default_response_value(frame, status as u8, Direction::CLIENT_TO_SERVER as u8);
        self.endpoints[endpoint_index]
            .default_response_calls
            .push(value.clone());
        self.last_default_response = Some(value);
    }
}

fn default_response_value(frame: &Frame, status: u8, direction: u8) -> Value {
    json!([
        frame.command.ID,
        status,
        frame.cluster.ID,
        frame.header.transaction_sequence_number,
        {"direction": direction},
    ])
}

fn emit_default_response_at(
    endpoint: &mut Endpoint,
    frame: &Frame,
    _endpoint_index: usize,
    last: Option<&Value>,
    status: u8,
    direction: u8,
) -> Option<Value> {
    let value = default_response_value(frame, status, direction);
    if last == Some(&value) {
        return None;
    }
    endpoint.default_response_calls.push(value.clone());
    Some(value)
}
