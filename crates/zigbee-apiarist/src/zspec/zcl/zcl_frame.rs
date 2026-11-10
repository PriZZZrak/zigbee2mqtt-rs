// Original: .upstream/zigbee-herdsman/src/zspec/zcl/zclFrame.ts

use serde_json::{json, Value};

use crate::zspec::zcl::buffalo_zcl::BuffaloZcl;
use crate::zspec::zcl::definition::enums::{BuffaloZclDataType, DataType};
use crate::zspec::zcl::utils::{
    Cluster, Command, CustomClusters, Parameter, ParameterCondition, Utils,
};
use crate::zspec::zcl::zcl_header::{Direction, FrameControl, FrameType, Header};

/// Upstream command argument: a command name or a numeric command identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRef {
    Id(u8),
    Name(&'static str),
}

impl From<u8> for CommandRef {
    fn from(value: u8) -> Self {
        Self::Id(value)
    }
}

impl From<&'static str> for CommandRef {
    fn from(value: &'static str) -> Self {
        Self::Name(value)
    }
}

/// Upstream cluster argument: a cluster name or a numeric cluster identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterRef {
    Id(u16),
    Name(&'static str),
}

impl From<u16> for ClusterRef {
    fn from(value: u16) -> Self {
        Self::Id(value)
    }
}

impl From<&'static str> for ClusterRef {
    fn from(value: &'static str) -> Self {
        Self::Name(value)
    }
}

/// Upstream `ZclFrame`.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub header: Header,
    pub cluster: Cluster,
    pub command: Command,
    pub payload: Value,
}

/// Upstream `ZclStatusError` / decode failures surfaced by `Frame.fromBuffer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameParseError {
    /// Upstream `ZclStatusError.code`.
    Status(u8),
    /// Any non-status parse failure.
    Other(String),
}

impl std::fmt::Display for FrameParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Status(status) => write!(formatter, "ZCL status error '{status}'"),
            Self::Other(message) => write!(formatter, "{message}"),
        }
    }
}

/// Fallible variant of `Frame::fromBuffer` for callers that upstream wraps
/// in `try/catch` (controller `onZclPayload`, green power decryption).
///
/// The codec port signals `ZclStatusError`/invalid fields with panics, so
/// the boundary converts them into `Result` without changing the existing
/// infallible `from_buffer` contract used by the model layer.
pub fn try_from_buffer(
    cluster_id: u16,
    header: Option<Header>,
    buffer: &[u8],
    custom_clusters: &CustomClusters,
) -> Result<Frame, FrameParseError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Frame::from_buffer(cluster_id, header, buffer, custom_clusters)
    }))
    .map_err(|payload| {
        let message = if let Some(message) = payload.downcast_ref::<&str>() {
            (*message).to_string()
        } else if let Some(message) = payload.downcast_ref::<String>() {
            message.clone()
        } else {
            "Frame parse failed".to_string()
        };

        if message == "INVALID_FIELD" {
            FrameParseError::Status(0x85)
        } else if message.starts_with("Command ")
            || message.starts_with("Global command does not exist")
        {
            FrameParseError::Status(0x81)
        } else {
            FrameParseError::Other(message)
        }
    })
}

impl Frame {
    /// Upstream `ZclFrame.create(...)`.
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        frame_type: u8,
        direction: u8,
        disable_default_response: bool,
        manufacturer_code: Option<u16>,
        transaction_sequence_number: u8,
        command: impl Into<CommandRef>,
        cluster: impl Into<ClusterRef>,
        payload: Value,
        _custom_clusters: Value,
        reserved_bits: u8,
    ) -> Self {
        if frame_type > 1 {
            panic!("Frametype '{frame_type}' not valid");
        }
        let custom_clusters = CustomClusters::empty();
        let cluster = match cluster.into() {
            ClusterRef::Id(id) => Utils::get_cluster(id, manufacturer_code, &custom_clusters),
            ClusterRef::Name(name) => Utils::get_cluster(name, manufacturer_code, &custom_clusters),
        };
        let is_specific = frame_type == 1;
        let command = resolve_command(&cluster, command.into(), is_specific, direction == 1);
        Self::create_with_definitions(
            cluster,
            command,
            frame_type,
            direction,
            disable_default_response,
            manufacturer_code,
            transaction_sequence_number,
            payload,
            reserved_bits,
        )
    }

    /// Upstream `ZclFrame.create(...)` with already resolved definitions
    /// (used for custom clusters and dynamic commands).
    #[allow(clippy::too_many_arguments)]
    pub fn create_with_definitions(
        cluster: Cluster,
        command: Command,
        frame_type: u8,
        direction: u8,
        disable_default_response: bool,
        manufacturer_code: Option<u16>,
        transaction_sequence_number: u8,
        payload: Value,
        reserved_bits: u8,
    ) -> Self {
        if frame_type > 1 {
            panic!("Frametype '{frame_type}' not valid");
        }
        let frame_type = if frame_type == 0 {
            FrameType::GLOBAL
        } else {
            FrameType::SPECIFIC
        };
        let direction = if direction == 0 {
            Direction::CLIENT_TO_SERVER
        } else {
            Direction::SERVER_TO_CLIENT
        };
        let header = Header::new(
            FrameControl {
                reserved_bits,
                frame_type,
                direction,
                disable_default_response,
                manufacturer_specific: manufacturer_code.is_some(),
            },
            manufacturer_code,
            transaction_sequence_number,
            command.ID,
        );
        Self {
            header,
            cluster,
            command,
            payload,
        }
    }

    /// Upstream `ZclFrame.fromBuffer(clusterId, header, buffer, customClusters)`.
    pub fn from_buffer(
        cluster_id: u16,
        header: Option<Header>,
        buffer: &[u8],
        custom_clusters: &CustomClusters,
    ) -> Self {
        let Some(header) = header else {
            if buffer.is_empty() {
                panic!("Invalid ZclHeader.");
            }
            let frame_type = buffer[0] & 0x03;
            panic!("Unsupported frameType '{frame_type}'");
        };
        let cluster = Utils::get_cluster(cluster_id, None, custom_clusters);
        let server_to_client = header.frame_control.direction == Direction::SERVER_TO_CLIENT;
        let (command, payload) = if header.frame_control.frame_type == FrameType::SPECIFIC {
            let command = resolve_command(
                &cluster,
                CommandRef::Id(header.command_identifier),
                true,
                server_to_client,
            );
            let payload = decode_parameters(&command, buffer, header.length);
            (command, payload)
        } else {
            let command = Utils::get_global_command(header.command_identifier);
            let payload = decode_payload(&command.name, buffer, header.length, &cluster.name);
            (command, payload)
        };
        Self {
            header,
            cluster,
            command,
            payload,
        }
    }

    /// Upstream `frame.toBuffer()`.
    pub fn to_buffer(&self) -> Vec<u8> {
        let mut buffer = self.header.to_buffer();
        if self.header.frame_control.frame_type == FrameType::SPECIFIC {
            encode_parameters(&self.command, &self.payload, &mut buffer);
        } else {
            encode_payload(&self.command.name, &self.payload, &mut buffer);
        }
        buffer
    }

    /// Upstream `frame.toString()`.
    pub fn to_string(&self) -> String {
        format!(
            "{{\"header\":{},\"payload\":{},\"command\":{}}}",
            header_to_string(&self.header),
            payload_to_string(&self.command.name, &self.payload),
            command_to_string(&self.command, self.header.frame_control.frame_type),
        )
    }

    /// Upstream `frame.isCluster(name)`.
    pub fn is_cluster(&self, name: &str) -> bool {
        self.cluster.name == name
    }

    /// Upstream `frame.isCommand(name)`.
    pub fn is_command(&self, name: &str) -> bool {
        self.command.name == name
    }

    /// Upstream `ZclFrame.conditionsValid(parameter, payload, minimumRemainingBufferBytes)`.
    pub fn conditions_valid(
        parameter: &Parameter,
        payload: &Value,
        minimum_remaining_buffer_bytes: Option<usize>,
    ) -> bool {
        parameter
            .conditions
            .iter()
            .all(|condition| match condition {
                ParameterCondition::MINIMUM_REMAINING_BUFFER_BYTES(value) => {
                    minimum_remaining_buffer_bytes
                        .map(|remaining| remaining >= *value as usize)
                        .unwrap_or(false)
                }
                ParameterCondition::BITMASK_SET {
                    field,
                    value,
                    reversed,
                } => {
                    let actual = payload
                        .get(field.as_str())
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let set = actual & u64::from(*value) == u64::from(*value);
                    if *reversed {
                        !set
                    } else {
                        set
                    }
                }
                ParameterCondition::BITFIELD_ENUM {
                    field,
                    offset,
                    size,
                    value,
                } => {
                    let actual = payload
                        .get(field.as_str())
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let mask = (1u64 << size) - 1;
                    ((actual >> offset) & mask) == u64::from(*value)
                }
                ParameterCondition::FIELD_EQUAL {
                    field,
                    value,
                    reversed,
                } => {
                    let actual = payload
                        .get(field.as_str())
                        .and_then(Value::as_u64)
                        .unwrap_or(u64::MAX);
                    let equal = actual == u64::from(*value);
                    if *reversed {
                        !equal
                    } else {
                        equal
                    }
                }
            })
    }
}

fn u8_value(value: &Value) -> u8 {
    value.as_u64().unwrap_or(0) as u8
}

fn u16_value(value: &Value) -> u16 {
    value.as_u64().unwrap_or(0) as u16
}

fn push_u16(buffer: &mut Vec<u8>, value: u16) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn push_selector(buffer: &mut Vec<u8>, selector: &Value) {
    let indicator = selector
        .get("indicatorType")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u8;
    buffer.push(indicator);
    if indicator != 0 {
        if let Some(index) = selector.get("index").and_then(Value::as_u64) {
            push_u16(buffer, index as u16);
        }
        if let Some(field) = selector.get("fieldIndex").and_then(Value::as_u64) {
            push_u16(buffer, field as u16);
        }
    }
}

pub(crate) fn write_data_type(buffer: &mut Vec<u8>, data_type: u8, value: &Value) {
    match data_type {
        0x00 => {}
        0x10 => buffer.push(
            if value
                .as_bool()
                .unwrap_or_else(|| value.as_u64().unwrap_or(0) != 0)
            {
                1
            } else {
                0
            },
        ),
        0x08 | 0x18 | 0x20 | 0x28 | 0x30 => buffer.push(u8_value(value)),
        0x09 | 0x19 | 0x21 | 0x29 | 0x31 | 0xe8 | 0xe9 => push_u16(buffer, u16_value(value)),
        0x0a | 0x1a | 0x22 | 0x2a => {
            let number = value.as_u64().unwrap_or(0);
            buffer.extend_from_slice(&number.to_le_bytes()[..3]);
        }
        0x0b | 0x1b | 0x23 | 0x2b => {
            buffer.extend_from_slice(&(value.as_u64().unwrap_or(0) as u32).to_le_bytes())
        }
        0x0c | 0x1c | 0x24 | 0x2c => {
            buffer.extend_from_slice(&value.as_u64().unwrap_or(0).to_le_bytes()[..5])
        }
        0x0d | 0x1d | 0x25 | 0x2d => {
            buffer.extend_from_slice(&value.as_u64().unwrap_or(0).to_le_bytes()[..6])
        }
        0x0e | 0x1e | 0x26 | 0x2e => {
            buffer.extend_from_slice(&value.as_u64().unwrap_or(0).to_le_bytes()[..7])
        }
        0x0f | 0x1f | 0x27 | 0x2f => {
            buffer.extend_from_slice(&value.as_u64().unwrap_or(0).to_le_bytes())
        }
        0x38 => {
            let number = value.as_f64().unwrap_or(0.0) as f32;
            let half = f32_to_half(number);
            push_u16(buffer, half);
        }
        0x39 => {
            let number = value.as_f64().unwrap_or(0.0) as f32;
            buffer.extend_from_slice(&number.to_le_bytes());
        }
        0x3a => {
            let number = value.as_f64().unwrap_or(0.0);
            buffer.extend_from_slice(&number.to_le_bytes());
        }
        0x41 | 0x42 => {
            let bytes = value.as_str().map(str::as_bytes).unwrap_or_default();
            buffer.push(bytes.len() as u8);
            buffer.extend_from_slice(bytes);
        }
        0x43 | 0x44 => {
            let bytes = value.as_str().map(str::as_bytes).unwrap_or_default();
            push_u16(buffer, bytes.len() as u16);
            buffer.extend_from_slice(bytes);
        }
        0xe2 => buffer.extend_from_slice(&(u64_value(value) as u32).to_le_bytes()),
        0xf0 => {
            let hex = value.as_str().unwrap_or("0x0").trim_start_matches("0x");
            let padded = format!("{hex:0>16}");
            let mut bytes = Vec::new();
            let mut index = padded.len();
            while index >= 2 {
                bytes.push(u8::from_str_radix(&padded[index - 2..index], 16).unwrap_or(0));
                index -= 2;
            }
            buffer.extend_from_slice(&bytes);
        }
        0xf1 => {
            if let Some(items) = value.as_array() {
                buffer.extend(items.iter().map(|item| u8_value(item)));
            }
        }
        0x4c => {
            let items = value.as_array().cloned().unwrap_or_default();
            let length = items.len() as u16;
            buffer.extend_from_slice(&length.to_le_bytes());
            for item in items {
                let element_type = u8_value(&item["elmType"]);
                buffer.push(element_type);
                write_data_type(buffer, element_type, &item["elmVal"]);
            }
        }
        0x48 => {
            let element_type = u8_value(&value["elementType"]);
            let elements = value["elements"].as_array().cloned().unwrap_or_default();
            buffer.push(element_type);
            buffer.extend_from_slice(&(elements.len() as u16).to_le_bytes());
            for element in elements {
                write_data_type(buffer, element_type, &element);
            }
        }
        _ => {
            if let Some(items) = value.as_array() {
                buffer.extend(items.iter().map(|item| u8_value(item)));
            }
        }
    }
}

fn u64_value(value: &Value) -> u64 {
    value.as_u64().unwrap_or(0)
}

fn f32_to_half(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let mut exponent = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    let mut mantissa = (bits >> 13) & 0x3ff;
    if exponent <= 0 {
        mantissa = (mantissa | 0x400) >> (1 - exponent);
        exponent = 0;
    } else if exponent >= 31 {
        exponent = 31;
        mantissa = 0;
    }
    sign | ((exponent as u16) << 10) | mantissa as u16
}

pub(crate) fn read_data_type(buffer: &[u8], position: &mut usize, data_type: u8) -> Value {
    let take = |length: usize, position: &mut usize| -> Vec<u8> {
        let end = (*position + length).min(buffer.len());
        let values = buffer[*position..end].to_vec();
        *position = end;
        values
    };
    match data_type {
        0x00 => Value::Null,
        0x08 | 0x10 | 0x18 | 0x20 | 0x28 | 0x30 => json!(take(1, position)[0]),
        0x09 | 0x19 | 0x21 | 0x29 | 0x31 | 0xe8 | 0xe9 => {
            let bytes = take(2, position);
            json!(u16::from_le_bytes([bytes[0], bytes[1]]))
        }
        0x0a | 0x1a | 0x22 | 0x2a => json!(read_uint(&take(3, position))),
        0x0b | 0x1b | 0x23 | 0x2b => json!(read_uint(&take(4, position))),
        0x0c | 0x1c | 0x24 | 0x2c => json!(read_uint(&take(5, position))),
        0x0d | 0x1d | 0x25 | 0x2d => json!(read_uint(&take(6, position))),
        0x0e | 0x1e | 0x26 | 0x2e => json!(read_uint(&take(7, position))),
        0x0f | 0x1f | 0x27 | 0x2f => json!(read_uint(&take(8, position))),
        0x38 => {
            let bytes = take(2, position);
            json!(half_to_f32(u16::from_le_bytes([bytes[0], bytes[1]])) as f64)
        }
        0x39 => {
            let bytes = take(4, position);
            json!(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64)
        }
        0x3a => {
            let bytes = take(8, position);
            json!(f64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]
            ]))
        }
        0x41 | 0x42 => {
            let length = take(1, position).first().copied().unwrap_or(0) as usize;
            json!(String::from_utf8_lossy(&take(length, position)).to_string())
        }
        0x43 | 0x44 => {
            let bytes = take(2, position);
            let length = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
            json!(String::from_utf8_lossy(&take(length, position)).to_string())
        }
        0xe2 => json!(read_uint(&take(4, position))),
        0xf0 => {
            let bytes = take(8, position);
            let mut reversed = bytes;
            reversed.reverse();
            json!(format!(
                "0x{}",
                reversed
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            ))
        }
        0xf1 => json!(take(16, position)),
        0x4c => {
            let bytes = take(2, position);
            let count = u16::from_le_bytes([bytes[0], bytes[1]]);
            if count == 0xffff {
                return json!([]);
            }
            let mut items = Vec::new();
            for _ in 0..count {
                let element_type = take(1, position)[0];
                let value = read_data_type(buffer, position, element_type);
                items.push(json!({"elmType": element_type, "elmVal": value}));
            }
            json!(items)
        }
        0x48 => {
            let element_type = take(1, position)[0];
            let bytes = take(2, position);
            let count = u16::from_le_bytes([bytes[0], bytes[1]]);
            if count == 0xffff {
                return json!([]);
            }
            let mut items = Vec::new();
            for _ in 0..count {
                items.push(read_data_type(buffer, position, element_type));
            }
            json!(items)
        }
        _ => json!(take(1, position)),
    }
}

fn is_analog(data_type: u8) -> bool {
    matches!(
        data_type,
        0x20..=0x2f | 0x38..=0x3a | 0xe0..=0xe2
    )
}

fn read_uint(bytes: &[u8]) -> u64 {
    let mut value = 0u64;
    for (index, byte) in bytes.iter().enumerate() {
        value |= u64::from(*byte) << (index * 8);
    }
    value
}

fn half_to_f32(half: u16) -> f32 {
    let sign = if half & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = ((half >> 10) & 0x1f) as i32;
    let mantissa = f32::from(half & 0x03ff);
    if exponent == 0 {
        sign * mantissa * 2f32.powi(-24)
    } else if exponent == 31 {
        sign * f32::INFINITY
    } else {
        sign * (1.0 + mantissa / 1024.0) * 2f32.powi(exponent - 15)
    }
}

fn encode_payload(command: &str, payload: &Value, buffer: &mut Vec<u8>) {
    match command {
        "read" | "readStructured" => {
            let items = payload.as_array().expect("read payload must be an array");
            for item in items {
                push_u16(buffer, u16_value(&item["attrId"]));
                if command == "readStructured" {
                    push_selector(buffer, &item["selector"]);
                }
            }
        }
        "readRsp" | "report" => {
            let items = payload.as_array().expect("payload must be an array");
            for item in items {
                push_u16(buffer, u16_value(&item["attrId"]));
                if command == "readRsp" {
                    buffer.push(u8_value(&item["status"]));
                }
                let data_type = u8_value(&item["dataType"]);
                buffer.push(data_type);
                write_data_type(buffer, data_type, &item["attrData"]);
            }
        }
        "write" | "writeUndiv" | "writeNoRsp" | "writeStructured" => {
            let items = payload.as_array().expect("payload must be an array");
            for item in items {
                push_u16(buffer, u16_value(&item["attrId"]));
                if command == "writeStructured" {
                    push_selector(buffer, &item["selector"]);
                }
                let data_type = u8_value(&item["dataType"]);
                buffer.push(data_type);
                let value = if command == "writeStructured" {
                    &item["elementData"]
                } else {
                    &item["attrData"]
                };
                write_data_type(buffer, data_type, value);
            }
        }
        "writeRsp" => {
            let items = payload.as_array().expect("payload must be an array");
            if items
                .first()
                .is_some_and(|item| item.get("attrId").is_some())
            {
                for item in items {
                    buffer.push(u8_value(&item["status"]));
                    push_u16(buffer, u16_value(&item["attrId"]));
                }
            } else {
                buffer.push(u8_value(&items[0]["status"]));
            }
        }
        "writeStructuredRsp" => {
            let items = payload.as_array().expect("payload must be an array");
            if items
                .first()
                .is_some_and(|item| item.get("attrId").is_some())
            {
                for item in items {
                    buffer.push(u8_value(&item["status"]));
                    push_u16(buffer, u16_value(&item["attrId"]));
                    push_selector(buffer, &item["selector"]);
                }
            } else {
                buffer.push(u8_value(&items[0]["status"]));
            }
        }
        "configReport" => {
            let items = payload.as_array().expect("payload must be an array");
            for item in items {
                let direction = u8_value(&item["direction"]);
                buffer.push(direction);
                push_u16(buffer, u16_value(&item["attrId"]));
                if direction == 0 {
                    let data_type = u8_value(&item["dataType"]);
                    buffer.push(data_type);
                    push_u16(buffer, u16_value(&item["minRepIntval"]));
                    push_u16(buffer, u16_value(&item["maxRepIntval"]));
                    if is_analog(data_type) {
                        if let Some(change) = item.get("repChange").filter(|value| !value.is_null())
                        {
                            write_data_type(buffer, data_type, change);
                        }
                    }
                } else {
                    push_u16(buffer, u16_value(&item["timeout"]));
                }
            }
        }
        "configReportRsp" => {
            let items = payload.as_array().expect("payload must be an array");
            if items
                .first()
                .is_some_and(|item| item.get("attrId").is_some())
            {
                for item in items {
                    buffer.push(u8_value(&item["status"]));
                    buffer.push(u8_value(&item["direction"]));
                    push_u16(buffer, u16_value(&item["attrId"]));
                }
            } else {
                buffer.push(u8_value(&items[0]["status"]));
            }
        }
        "readReportConfig" => {
            let items = payload.as_array().expect("payload must be an array");
            for item in items {
                buffer.push(u8_value(&item["direction"]));
                push_u16(buffer, u16_value(&item["attrId"]));
            }
        }
        "readReportConfigRsp" => {
            let items = payload.as_array().expect("payload must be an array");
            for item in items {
                buffer.push(u8_value(&item["status"]));
                let direction = u8_value(&item["direction"]);
                buffer.push(direction);
                push_u16(buffer, u16_value(&item["attrId"]));
                if u8_value(&item["status"]) == 0 && direction == 0 {
                    let data_type = u8_value(&item["dataType"]);
                    buffer.push(data_type);
                    push_u16(buffer, u16_value(&item["minRepIntval"]));
                    push_u16(buffer, u16_value(&item["maxRepIntval"]));
                    if is_analog(data_type) {
                        if let Some(change) = item.get("repChange").filter(|value| !value.is_null())
                        {
                            write_data_type(buffer, data_type, change);
                        }
                    }
                } else if u8_value(&item["status"]) == 0 && direction == 1 {
                    push_u16(buffer, u16_value(&item["timeout"]));
                }
            }
        }
        "defaultRsp" => {
            assert!(payload.is_object(), "defaultRsp payload must be an object");
            buffer.push(u8_value(&payload["cmdId"]));
            buffer.push(u8_value(&payload["statusCode"]));
        }
        "discover" | "discoverExt" => {
            assert!(payload.is_object(), "discover payload must be an object");
            push_u16(buffer, u16_value(&payload["startAttrId"]));
            buffer.push(u8_value(&payload["maxAttrIds"]));
        }
        "discoverRsp" => {
            buffer.push(u8_value(&payload["discComplete"]));
            for info in payload["attrInfos"].as_array().expect("attrInfos") {
                push_u16(buffer, u16_value(&info["attrId"]));
                buffer.push(u8_value(&info["dataType"]));
            }
        }
        "discoverExtRsp" => {
            buffer.push(u8_value(&payload["discComplete"]));
            for info in payload["attrInfos"].as_array().expect("attrInfos") {
                push_u16(buffer, u16_value(&info["attrId"]));
                buffer.push(u8_value(&info["dataType"]));
                buffer.push(u8_value(&info["access"]));
            }
        }
        "discoverCommands" | "discoverCommandsGen" => {
            assert!(
                payload.is_object(),
                "discoverCommands payload must be an object"
            );
            buffer.push(u8_value(&payload["startCmdId"]));
            buffer.push(u8_value(&payload["maxCmdIds"]));
        }
        "discoverCommandsRsp" | "discoverCommandsGenRsp" => {
            buffer.push(u8_value(&payload["discComplete"]));
            for id in payload["commandIds"].as_array().expect("commandIds") {
                buffer.push(u8_value(id));
            }
        }
        _ => {}
    }
}

fn decode_payload(command: &str, buffer: &[u8], header_length: usize, cluster_name: &str) -> Value {
    let mut position = header_length.min(buffer.len());
    let read_u8 = |position: &mut usize| -> u8 {
        let value = buffer.get(*position).copied().unwrap_or(0);
        *position += 1;
        value
    };
    let read_u16 = |position: &mut usize| -> u16 {
        let first = buffer.get(*position).copied().unwrap_or(0);
        let second = buffer.get(*position + 1).copied().unwrap_or(0);
        *position += 2;
        u16::from_le_bytes([first, second])
    };
    let read_selector = |position: &mut usize| -> Value {
        let indicator = read_u8(position);
        if indicator == 0 {
            json!({"indicatorType": indicator})
        } else {
            json!({
                "indicatorType": indicator,
                "index": read_u16(position),
                "fieldIndex": read_u16(position),
            })
        }
    };
    let mut items = Vec::new();
    match command {
        "read" | "readStructured" => {
            while position < buffer.len() {
                let attr_id = read_u16(&mut position);
                let mut item = json!({"attrId": attr_id});
                if command == "readStructured" {
                    item["selector"] = read_selector(&mut position);
                }
                items.push(item);
            }
            Value::Array(items)
        }
        "readRsp" | "report" => {
            while position < buffer.len() {
                let attr_id = read_u16(&mut position);
                let status = if command == "readRsp" {
                    Some(read_u8(&mut position))
                } else {
                    None
                };
                let data_type = read_u8(&mut position);
                let mut data = if attr_id == 0xff01 && data_type == 0x42 {
                    let length = buffer.get(position).copied().unwrap_or(0) as usize;
                    position += 1;
                    let end = (position + length).min(buffer.len());
                    let mut inner = position;
                    let value = read_mi_struct(&buffer[..end], &mut inner);
                    position = end;
                    value
                } else {
                    read_data_type(buffer, &mut position, data_type)
                };
                if attr_id == 0xff01 && data_type == 0x42 {
                    // Already converted.
                } else if data_type == 0x00 {
                    data = Value::Null;
                }
                let mut item = json!({"attrId": attr_id});
                if let Some(status) = status {
                    item["status"] = json!(status);
                }
                item["dataType"] = json!(data_type);
                if command == "readRsp" && status != Some(0) {
                    // Failed entries have no attribute data.
                } else {
                    item["attrData"] = data;
                }
                items.push(item);
            }
            Value::Array(items)
        }
        "write" | "writeUndiv" | "writeNoRsp" | "writeStructured" => {
            while position < buffer.len() {
                let attr_id = read_u16(&mut position);
                let mut item = json!({"attrId": attr_id});
                if command == "writeStructured" {
                    item["selector"] = read_selector(&mut position);
                }
                let data_type = read_u8(&mut position);
                item["dataType"] = json!(data_type);
                let data = read_data_type(buffer, &mut position, data_type);
                if command == "writeStructured" {
                    item["elementData"] = data;
                } else {
                    item["attrData"] = data;
                }
                items.push(item);
            }
            Value::Array(items)
        }
        "writeRsp" => {
            if position >= buffer.len() {
                Value::Array(vec![])
            } else {
                let status = read_u8(&mut position);
                if position + 1 < buffer.len() {
                    while position < buffer.len() {
                        let status = if items.is_empty() {
                            status
                        } else {
                            read_u8(&mut position)
                        };
                        let attr_id = read_u16(&mut position);
                        items.push(json!({"status": status, "attrId": attr_id}));
                    }
                    Value::Array(items)
                } else {
                    json!([{"status": status}])
                }
            }
        }
        "writeStructuredRsp" => {
            if position >= buffer.len() {
                Value::Array(vec![])
            } else {
                let first_status = read_u8(&mut position);
                if position + 1 < buffer.len() {
                    loop {
                        let status = if items.is_empty() {
                            first_status
                        } else {
                            read_u8(&mut position)
                        };
                        if position + 1 >= buffer.len() {
                            items.push(json!({"status": status}));
                            break;
                        }
                        let attr_id = read_u16(&mut position);
                        let selector = read_selector(&mut position);
                        items.push(
                            json!({"status": status, "attrId": attr_id, "selector": selector}),
                        );
                        if position >= buffer.len() {
                            break;
                        }
                    }
                    Value::Array(items)
                } else {
                    json!([{"status": first_status}])
                }
            }
        }
        "configReport" => {
            while position < buffer.len() {
                let direction = read_u8(&mut position);
                let attr_id = read_u16(&mut position);
                let mut item = json!({"direction": direction, "attrId": attr_id});
                if direction == 0 {
                    let data_type = read_u8(&mut position);
                    let min = read_u16(&mut position);
                    let max = read_u16(&mut position);
                    item["dataType"] = json!(data_type);
                    item["minRepIntval"] = json!(min);
                    item["maxRepIntval"] = json!(max);
                    if is_analog(data_type) && position < buffer.len() {
                        item["repChange"] = read_data_type(buffer, &mut position, data_type);
                    }
                } else {
                    item["timeout"] = json!(read_u16(&mut position));
                }
                items.push(item);
            }
            Value::Array(items)
        }
        "configReportRsp" => {
            if position >= buffer.len() {
                return json!([]);
            }
            let first_status = read_u8(&mut position);
            if position + 1 < buffer.len() {
                loop {
                    let status = if items.is_empty() {
                        first_status
                    } else {
                        read_u8(&mut position)
                    };
                    let direction = read_u8(&mut position);
                    let attr_id = read_u16(&mut position);
                    items.push(json!({
                        "status": status,
                        "direction": direction,
                        "attrId": attr_id,
                    }));
                    if position >= buffer.len() {
                        break;
                    }
                }
                Value::Array(items)
            } else {
                json!([{"status": first_status}])
            }
        }
        "readReportConfig" => {
            while position < buffer.len() {
                items.push(json!({
                    "direction": read_u8(&mut position),
                    "attrId": read_u16(&mut position),
                }));
            }
            Value::Array(items)
        }
        "readReportConfigRsp" => {
            while position < buffer.len() {
                let status = read_u8(&mut position);
                let direction = read_u8(&mut position);
                let attr_id = read_u16(&mut position);
                let mut item = json!({
                    "status": status,
                    "direction": direction,
                    "attrId": attr_id,
                });
                if status == 0 && direction == 0 {
                    let data_type = read_u8(&mut position);
                    let min = read_u16(&mut position);
                    let max = read_u16(&mut position);
                    item["dataType"] = json!(data_type);
                    item["minRepIntval"] = json!(min);
                    item["maxRepIntval"] = json!(max);
                    if is_analog(data_type) && position < buffer.len() {
                        item["repChange"] = read_data_type(buffer, &mut position, data_type);
                    }
                } else if status == 0 && direction == 1 {
                    item["timeout"] = json!(read_u16(&mut position));
                }
                items.push(item);
            }
            Value::Array(items)
        }
        "defaultRsp" => json!({
            "cmdId": read_u8(&mut position),
            "statusCode": read_u8(&mut position),
        }),
        "discover" | "discoverExt" => json!({
            "startAttrId": read_u16(&mut position),
            "maxAttrIds": read_u8(&mut position),
        }),
        "discoverRsp" => {
            let complete = read_u8(&mut position);
            let mut infos = Vec::new();
            while position + 2 < buffer.len() {
                infos.push(json!({
                    "attrId": read_u16(&mut position),
                    "dataType": read_u8(&mut position),
                }));
            }
            json!({"discComplete": complete, "attrInfos": infos})
        }
        "discoverExtRsp" => {
            let complete = read_u8(&mut position);
            let mut infos = Vec::new();
            while position + 3 < buffer.len() {
                infos.push(json!({
                    "attrId": read_u16(&mut position),
                    "dataType": read_u8(&mut position),
                    "access": read_u8(&mut position),
                }));
            }
            json!({"discComplete": complete, "attrInfos": infos})
        }
        "discoverCommands" | "discoverCommandsGen" => json!({
            "startCmdId": read_u8(&mut position),
            "maxCmdIds": read_u8(&mut position),
        }),
        "discoverCommandsRsp" | "discoverCommandsGenRsp" => {
            let complete = read_u8(&mut position);
            let mut ids = Vec::new();
            while position < buffer.len() {
                ids.push(json!(read_u8(&mut position)));
            }
            json!({"discComplete": complete, "commandIds": ids})
        }
        _ => {
            let _ = cluster_name;
            Value::Null
        }
    }
}

fn read_mi_struct(buffer: &[u8], position: &mut usize) -> Value {
    let mut output = serde_json::Map::new();
    while *position < buffer.len() {
        let id = buffer[*position];
        *position += 1;
        if *position >= buffer.len() {
            break;
        }
        let data_type = buffer[*position];
        *position += 1;
        let value = read_data_type(buffer, position, data_type);
        output.insert(id.to_string(), value);
    }
    Value::Object(output)
}

fn header_to_string(header: &Header) -> String {
    let control = header.frame_control;
    let mut text = format!(
        "{{\"frameControl\":{{\"reservedBits\":{},\"frameType\":{},\"direction\":{},\"disableDefaultResponse\":{},\"manufacturerSpecific\":{}}}",
        control.reserved_bits,
        control.frame_type as u8,
        control.direction as u8,
        control.disable_default_response,
        control.manufacturer_specific,
    );
    if let Some(code) = header.manufacturer_code {
        text.push_str(&format!(",\"manufacturerCode\":{code}"));
    }
    text.push_str(&format!(
        ",\"transactionSequenceNumber\":{},\"commandIdentifier\":{}}}",
        header.transaction_sequence_number, header.command_identifier
    ));
    text
}

/// Upstream `getClusterCommand` / `getClusterCommandResponse` /
/// `getGlobalCommand` resolution used by `ZclFrame.create`.
fn resolve_command(
    cluster: &Cluster,
    key: CommandRef,
    is_specific: bool,
    server_to_client: bool,
) -> Command {
    if !is_specific {
        return match key {
            CommandRef::Id(id) => Utils::get_global_command(id),
            CommandRef::Name(name) => Utils::get_global_command(name),
        };
    }

    let primary = if server_to_client {
        &cluster.commandsResponse
    } else {
        &cluster.commands
    };
    let secondary = if server_to_client {
        &cluster.commands
    } else {
        &cluster.commandsResponse
    };
    let found = match key {
        CommandRef::Id(id) => primary
            .values()
            .chain(secondary.values())
            .find(|command| command.ID == id),
        CommandRef::Name(name) => primary.get(name).or_else(|| secondary.get(name)),
    };
    found.cloned().unwrap_or_else(|| {
        panic!(
            "Command {key:?} does not exist for cluster '{}'",
            cluster.name
        )
    })
}

fn command_to_string(command: &Command, frame_type: FrameType) -> String {
    let mut text = format!(
        "{{\"name\":{},\"ID\":{}",
        json_string(&json!(command.name)),
        command.ID
    );
    if let Some(response) = command.response {
        text.push_str(&format!(",\"response\":{response}"));
    }
    if !command.parameters.is_empty() || frame_type == FrameType::SPECIFIC {
        text.push_str(",\"parameters\":[");
        for (index, parameter) in command.parameters.iter().enumerate() {
            if index > 0 {
                text.push(',');
            }
            text.push_str(&parameter_to_string(parameter));
        }
        text.push(']');
        if !command.parameters.is_empty() && command.required {
            text.push_str(",\"required\":true");
        }
    }
    text.push('}');
    text
}

fn parameter_to_string(parameter: &Parameter) -> String {
    let data_type = if parameter.name == "grouplist" {
        1002
    } else {
        parameter_type_code(parameter) as u16
    };
    let mut text = format!(
        "{{\"name\":{},\"type\":{data_type}",
        json_string(&json!(parameter.name))
    );
    if !parameter.conditions.is_empty() {
        text.push_str(",\"conditions\":[");
        for (index, condition) in parameter.conditions.iter().enumerate() {
            if index > 0 {
                text.push(',');
            }
            text.push_str(&match condition {
                ParameterCondition::MINIMUM_REMAINING_BUFFER_BYTES(value) => format!(
                    "{{\"type\":\"minimumRemainingBufferBytes\",\"value\":{value}}}"
                ),
                ParameterCondition::BITMASK_SET {
                    field,
                    value,
                    reversed,
                } => {
                    if *reversed {
                        format!(
                            "{{\"type\":\"bitmaskSet\",\"field\":{},\"value\":{value},\"reversed\":true}}",
                            json_string(&json!(field))
                        )
                    } else {
                        format!(
                            "{{\"type\":\"bitmaskSet\",\"field\":{},\"value\":{value}}}",
                            json_string(&json!(field))
                        )
                    }
                }
                ParameterCondition::BITFIELD_ENUM {
                    field,
                    offset,
                    size,
                    value,
                } => format!(
                    "{{\"type\":\"bitfieldEnum\",\"field\":{},\"offset\":{offset},\"size\":{size},\"value\":{value}}}",
                    json_string(&json!(field))
                ),
                ParameterCondition::FIELD_EQUAL {
                    field,
                    value,
                    reversed,
                } => {
                    if *reversed {
                        format!(
                            "{{\"type\":\"fieldEquals\",\"field\":{},\"value\":{value},\"reversed\":true}}",
                            json_string(&json!(field))
                        )
                    } else {
                        format!(
                            "{{\"type\":\"fieldEquals\",\"field\":{},\"value\":{value}}}",
                            json_string(&json!(field))
                        )
                    }
                }
            });
        }
        text.push(']');
    }
    if let Some(max) = parameter.max {
        text.push_str(&format!(",\"max\":{max}"));
    }
    text.push('}');
    text
}

fn payload_to_string(command: &str, payload: &Value) -> String {
    match (command, payload) {
        ("read" | "readStructured", Value::Array(items)) => {
            let rendered = items
                .iter()
                .map(|item| {
                    let mut keys: Vec<&str> = vec!["attrId"];
                    if command == "readStructured" && item.get("selector").is_some() {
                        keys.push("selector");
                    }
                    object_from_keys(item, &keys)
                })
                .collect::<Vec<_>>();
            format!("[{}]", rendered.join(","))
        }
        ("readRsp" | "report", Value::Array(items)) => {
            let rendered = items
                .iter()
                .map(|item| {
                    let data_type = item.get("dataType").and_then(Value::as_u64);
                    let mut keys: Vec<&str> = vec!["attrId"];
                    if command == "readRsp" {
                        keys.push("status");
                    }
                    keys.push("dataType");
                    keys.push("attrData");
                    if let Some(number) = item.get("attrData").and_then(Value::as_u64) {
                        if matches!(data_type, Some(0x26) | Some(0x27)) {
                            let mut converted = item.clone();
                            converted["attrData"] = json!(number.to_string());
                            return object_from_keys(&converted, &keys);
                        }
                    }
                    object_from_keys(item, &keys)
                })
                .collect::<Vec<_>>();
            format!("[{}]", rendered.join(","))
        }
        ("configReport", Value::Array(items)) => {
            let rendered = items
                .iter()
                .map(|item| object_from_keys(item, &["direction", "attrId", "timeout"]))
                .collect::<Vec<_>>();
            format!("[{}]", rendered.join(","))
        }
        ("defaultRsp", Value::Object(_)) => object_from_keys(payload, &["cmdId", "statusCode"]),
        ("discover", Value::Object(_)) | ("discoverExt", Value::Object(_)) => {
            object_from_keys(payload, &["startAttrId", "maxAttrIds"])
        }
        ("discoverRsp", Value::Object(_)) => payload_generic(payload),
        (_, Value::Object(_)) if command == "alarm" => {
            object_from_keys(payload, &["alarmcode", "clusterid"])
        }
        (_, Value::Object(map)) => {
            // Preserve insertion order from the decoded payload where possible.
            let keys: Vec<String> = map.keys().cloned().collect();
            let key_refs: Vec<&str> = keys.iter().map(String::as_str).collect();
            object_from_keys(payload, &key_refs)
        }
        _ => payload_generic(payload),
    }
}

fn object_from_keys(value: &Value, keys: &[&str]) -> String {
    let mut parts = Vec::new();
    for key in keys {
        if let Some(entry) = value.get(*key) {
            parts.push(format!(
                "{}:{}",
                json_string(&json!(key)),
                payload_generic(entry)
            ));
        }
    }
    format!("{{{}}}", parts.join(","))
}

fn payload_generic(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}

fn json_string(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

fn parameter_to_json(parameter: &Parameter) -> Value {
    let mut object = serde_json::Map::new();
    object.insert("name".to_string(), json!(parameter.name));
    let data_type = if parameter.name == "grouplist" {
        1002
    } else {
        parameter_type_code(parameter) as u16
    };
    object.insert("type".to_string(), json!(data_type));
    if !parameter.conditions.is_empty() {
        object.insert(
            "conditions".to_string(),
            Value::Array(
                parameter
                    .conditions
                    .iter()
                    .map(|condition| match condition {
                        ParameterCondition::MINIMUM_REMAINING_BUFFER_BYTES(value) => {
                            json!({"type": "minimumRemainingBufferBytes", "value": value})
                        }
                        ParameterCondition::BITMASK_SET {
                            field,
                            value,
                            reversed,
                        } => {
                            if *reversed {
                                json!({"type": "bitmaskSet", "field": field, "value": value, "reversed": true})
                            } else {
                                json!({"type": "bitmaskSet", "field": field, "value": value})
                            }
                        }
                        ParameterCondition::BITFIELD_ENUM {
                            field,
                            offset,
                            size,
                            value,
                        } => json!({
                            "type": "bitfieldEnum",
                            "field": field,
                            "offset": offset,
                            "size": size,
                            "value": value,
                        }),
                        ParameterCondition::FIELD_EQUAL {
                            field,
                            value,
                            reversed,
                        } => {
                            if *reversed {
                                json!({
                                    "type": "fieldEquals",
                                    "field": field,
                                    "value": value,
                                    "reversed": true,
                                })
                            } else {
                                json!({
                                    "type": "fieldEquals",
                                    "field": field,
                                    "value": value,
                                })
                            }
                        }
                    })
                    .collect(),
            ),
        );
    }
    if let Some(max) = parameter.max {
        object.insert("max".to_string(), json!(max));
    }
    Value::Object(object)
}

fn parameter_type_code(parameter: &Parameter) -> u32 {
    match parameter.buffalo_type {
        Some(buffalo_type) => buffalo_type as u32,
        None => parameter.r#type as u32,
    }
}

fn encode_parameters(command: &Command, payload: &Value, buffer: &mut Vec<u8>) {
    for parameter in &command.parameters {
        let include = if parameter.conditions.iter().any(|condition| {
            matches!(
                condition,
                ParameterCondition::MINIMUM_REMAINING_BUFFER_BYTES(_)
            )
        }) {
            payload.get(parameter.name.as_str()).is_some()
        } else {
            Frame::conditions_valid(parameter, payload, None)
        };
        if !include {
            continue;
        }
        let Some(value) = payload.get(parameter.name.as_str()) else {
            panic!("Parameter '{}' is missing", parameter.name);
        };
        if parameter.name == "grouplist" {
            for item in value.as_array().cloned().unwrap_or_default() {
                push_u16(buffer, u16_value(&item));
            }
        } else if parameter.buffalo_type == Some(BuffaloZclDataType::GPD_FRAME) {
            let mut buffalo = BuffaloZcl::new(Vec::new());
            buffalo.write(parameter_type_code(parameter), value, &json!({}));
            buffer.extend_from_slice(&buffalo.get_written());
        } else {
            write_data_type(buffer, parameter.r#type as u8, value);
        }
    }
}

fn decode_parameters(command: &Command, buffer: &[u8], header_length: usize) -> Value {
    let mut position = header_length.min(buffer.len());
    let mut payload = serde_json::Map::new();
    for parameter in &command.parameters {
        let remaining = buffer.len().saturating_sub(position);
        let current = Value::Object(payload.clone());
        if !Frame::conditions_valid(parameter, &current, Some(remaining)) {
            continue;
        }
        if position >= buffer.len() {
            if parameter.name == "minimumBlockPeriod" {
                break;
            }
            panic!("INVALID_FIELD");
        }
        let value = if parameter.name == "grouplist" {
            let count = payload
                .get("groupcount")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let mut items = Vec::new();
            for _ in 0..count {
                let bytes = &buffer[position..position + 2];
                position += 2;
                items.push(json!(u16::from_le_bytes([bytes[0], bytes[1]])));
            }
            Value::Array(items)
        } else if parameter.buffalo_type == Some(BuffaloZclDataType::GPD_FRAME) {
            let mut buffalo = BuffaloZcl::new(buffer.to_vec());
            buffalo.set_position(position);
            let options = json!({"payload": current});
            let value = buffalo.read(parameter_type_code(parameter), &options);
            position = buffalo.get_position();
            value
        } else {
            let size = parameter_size(parameter, &payload);
            if remaining < size {
                if parameter.name == "minimumBlockPeriod" {
                    break;
                }
                panic!("INVALID_FIELD");
            }
            read_data_type(buffer, &mut position, parameter.r#type as u8)
        };
        payload.insert(parameter.name.to_string(), value);
    }
    Value::Object(payload)
}

fn parameter_size(parameter: &Parameter, payload: &serde_json::Map<String, Value>) -> usize {
    if parameter.name == "grouplist" {
        return payload
            .get("groupcount")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize
            * 2;
    }
    match parameter.r#type as u8 {
        0x00 => 0,
        0x08 | 0x10 | 0x18 | 0x20 | 0x28 | 0x30 => 1,
        0x09 | 0x19 | 0x21 | 0x29 | 0x31 | 0xe8 | 0xe9 => 2,
        0x0a | 0x1a | 0x22 | 0x2a => 3,
        0x0b | 0x1b | 0x23 | 0x2b | 0xe2 | 0xea => 4,
        0x0c | 0x1c | 0x24 | 0x2c => 5,
        0x0d | 0x1d | 0x25 | 0x2d => 6,
        0x0e | 0x1e | 0x26 | 0x2e => 7,
        0x0f | 0x1f | 0x27 | 0x2f | 0xf0 => 8,
        0xf1 => 16,
        _ => 0,
    }
}
