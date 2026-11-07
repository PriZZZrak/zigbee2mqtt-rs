// Original: .upstream/zigbee-herdsman/src/zspec/zcl/zclFrame.ts
// TODO: port frame creation/parsing/serialization.

use serde_json::{json, Value};

use crate::zspec::zcl::definition::cluster::{Cluster, Command, Parameter};
use crate::zspec::zcl::definition::enums::DataType;
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
        let cluster = match cluster.into() {
            ClusterRef::Id(id) => cluster_from_id(id),
            ClusterRef::Name(name) => cluster_from_name(name),
        };
        let command = match command.into() {
            CommandRef::Id(id) => command_from_id(id),
            CommandRef::Name(name) => command_from_name(name),
        };
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
        _custom_clusters: Value,
    ) -> Self {
        let header = header.unwrap_or_else(|| {
            Header::new(
                FrameControl {
                    reserved_bits: 0,
                    frame_type: FrameType::GLOBAL,
                    direction: Direction::CLIENT_TO_SERVER,
                    disable_default_response: false,
                    manufacturer_specific: false,
                },
                None,
                0,
                0,
            )
        });
        let cluster = cluster_from_id(cluster_id);
        let command = command_from_id(header.command_identifier);
        let payload = decode_payload(command.name, buffer, header.length, cluster.name);
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
        encode_payload(self.command.name, &self.payload, &mut buffer);
        buffer
    }

    /// Upstream `frame.toString()`.
    pub fn to_string(&self) -> String {
        todo!()
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
        _parameter: &Parameter,
        _payload: &Value,
        _minimum_remaining_buffer_bytes: Option<usize>,
    ) -> bool {
        todo!()
    }
}

pub fn zcl_frame() {
    todo!()
}
pub fn is_cluster() {
    todo!()
}
pub fn is_command() {
    todo!()
}
pub fn to_buffer() {
    todo!()
}
pub fn to_string() {
    todo!()
}

pub fn cluster_from_id(id: u16) -> Cluster {
    let name = match id {
        0x0000 => "genBasic",
        0x0004 => "genGroups",
        0x0005 => "genScenes",
        0x0006 => "genOnOff",
        0x000a => "genTime",
        0x0020 => "genPollCtrl",
        0x0500 => "ssIasZone",
        _ => "unknown",
    };
    Cluster { name, ID: id }
}

pub fn cluster_from_name(name: &'static str) -> Cluster {
    let id = match name {
        "genBasic" => 0x0000,
        "genGroups" => 0x0004,
        "genScenes" => 0x0005,
        "genOnOff" => 0x0006,
        "genTime" => 0x000a,
        "genPollCtrl" => 0x0020,
        "ssIasZone" => 0x0500,
        _ => 0,
    };
    Cluster { name, ID: id }
}

pub fn command_from_id(id: u8) -> Command {
    let name = match id {
        0x00 => "read",
        0x01 => "readRsp",
        0x02 => "write",
        0x03 => "writeUndiv",
        0x04 => "writeRsp",
        0x05 => "writeNoRsp",
        0x06 => "configReport",
        0x07 => "configReportRsp",
        0x08 => "readReportConfig",
        0x09 => "readReportConfigRsp",
        0x0a => "report",
        0x0b => "defaultRsp",
        0x0c => "discover",
        0x0d => "discoverRsp",
        0x0e => "readStructured",
        0x0f => "writeStructured",
        0x10 => "writeStructuredRsp",
        0x11 => "discoverCommands",
        0x12 => "discoverCommandsRsp",
        0x13 => "discoverCommandsGen",
        0x14 => "discoverCommandsGenRsp",
        0x15 => "discoverExt",
        0x16 => "discoverExtRsp",
        _ => "unknown",
    };
    Command::new(name, id)
}

pub fn command_from_name(name: &'static str) -> Command {
    let id = match name {
        "read" => 0x00,
        "readRsp" => 0x01,
        "write" => 0x02,
        "writeUndiv" => 0x03,
        "writeRsp" => 0x04,
        "writeNoRsp" => 0x05,
        "configReport" => 0x06,
        "configReportRsp" => 0x07,
        "readReportConfig" => 0x08,
        "readReportConfigRsp" => 0x09,
        "report" => 0x0a,
        "defaultRsp" => 0x0b,
        "discover" => 0x0c,
        "discoverRsp" => 0x0d,
        "readStructured" => 0x0e,
        "writeStructured" => 0x0f,
        "writeStructuredRsp" => 0x10,
        "discoverCommands" => 0x11,
        "discoverCommandsRsp" => 0x12,
        "discoverCommandsGen" => 0x13,
        "discoverCommandsGenRsp" => 0x14,
        "discoverExt" => 0x15,
        "discoverExtRsp" => 0x16,
        "enrollReq" => 0x01,
        "enrollRsp" => 0x02,
        "checkin" => 0x00,
        "checkinRsp" => 0x01,
        "fastPollStop" => 0x03,
        "removeRsp" => 0x02,
        "add" => 0x00,
        "addRsp" => 0x00,
        _ => 0,
    };
    Command::new(name, id)
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

fn write_data_type(buffer: &mut Vec<u8>, data_type: u8, value: &Value) {
    match data_type {
        0x00 => {}
        0x08 | 0x10 | 0x18 | 0x20 | 0x28 | 0x30 => buffer.push(u8_value(value)),
        0x09 | 0x19 | 0x21 | 0x29 | 0x31 => push_u16(buffer, u16_value(value)),
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

fn read_data_type(buffer: &[u8], position: &mut usize, data_type: u8) -> Value {
    let take = |length: usize, position: &mut usize| -> Vec<u8> {
        let end = (*position + length).min(buffer.len());
        let values = buffer[*position..end].to_vec();
        *position = end;
        values
    };
    match data_type {
        0x00 => Value::Null,
        0x08 | 0x10 | 0x18 | 0x20 | 0x28 | 0x30 => json!(take(1, position)[0]),
        0x09 | 0x19 | 0x21 | 0x29 | 0x31 => {
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
