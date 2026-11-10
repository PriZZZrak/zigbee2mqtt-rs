// Original: .upstream/zigbee-herdsman/src/zspec/zdo/buffaloZdo.ts

use serde_json::{json, Value};

use crate::zspec::zdo::definition::clusters::ClusterId;
use crate::zspec::zdo::definition::enums::{GlobalTLV, RoutingTableStatus};
use crate::zspec::zdo::utils;

#[derive(Debug, Clone)]
pub struct BuffaloZdo {
    pub position: usize,
    pub buffer: Vec<u8>,
}

impl BuffaloZdo {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self {
            position: 0,
            buffer,
        }
    }

    pub fn get_position(&self) -> usize {
        self.position
    }

    pub fn set_position(&mut self, position: usize) {
        self.position = position;
    }

    pub fn get_byte(&self, position: usize) -> u8 {
        self.buffer[position]
    }

    pub fn set_byte(&mut self, position: usize, value: u8) {
        self.buffer[position] = value;
    }

    pub fn is_more_by(&self, count: usize) -> bool {
        self.position + count <= self.buffer.len()
    }

    fn read_u8(&mut self) -> u8 {
        let value = self.buffer[self.position];
        self.position += 1;
        value
    }

    fn read_u16(&mut self) -> u16 {
        let value =
            u16::from_le_bytes([self.buffer[self.position], self.buffer[self.position + 1]]);
        self.position += 2;
        value
    }

    fn read_u32(&mut self) -> u32 {
        let bytes = [
            self.buffer[self.position],
            self.buffer[self.position + 1],
            self.buffer[self.position + 2],
            self.buffer[self.position + 3],
        ];
        self.position += 4;
        u32::from_le_bytes(bytes)
    }

    fn read_eui64(&mut self) -> String {
        let bytes = self.read_buffer(8);
        let mut reversed = bytes;
        reversed.reverse();
        format!(
            "0x{}",
            reversed
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        )
    }

    fn read_buffer(&mut self, length: usize) -> Vec<u8> {
        let end = self.position + length;
        if end > self.buffer.len() {
            panic!("Not enough bytes to read");
        }
        let values = self.buffer[self.position..end].to_vec();
        self.position = end;
        values
    }

    pub fn read_tlvs(&mut self) -> Vec<Value> {
        let mut tlvs = Vec::new();
        while self.is_more_by(2) {
            let tag = self.read_u8();
            let length_minus_one = self.read_u8();
            let length = usize::from(length_minus_one) + 1;
            if !self.is_more_by(length) {
                panic!("Not enough bytes to read TLV");
            }
            let payload = self.read_buffer(length);
            if tag == 0xff {
                continue;
            }
            if tlvs
                .iter()
                .any(|existing: &Value| existing["tagId"] == json!(tag))
                && tag != GlobalTLV::MANUFACTURER_SPECIFIC
            {
                panic!("Duplicate TLV tag");
            }
            if tag == GlobalTLV::BEACON_APPENDIX_ENCAPSULATION
                && payload.first().is_some_and(|first| {
                    *first == GlobalTLV::JOINER_ENCAPSULATION
                        || *first == GlobalTLV::BEACON_APPENDIX_ENCAPSULATION
                })
            {
                panic!("Encapsulated TLV inside encapsulated TLV");
            }
            let tlv = match tag {
                GlobalTLV::MANUFACTURER_SPECIFIC => {
                    if payload.len() < 2 {
                        panic!("Malformed manufacturer TLV");
                    }
                    json!({
                        "zigbeeManufacturerId": u16::from_le_bytes([payload[0], payload[1]]),
                        "additionalData": payload[2..].to_vec(),
                    })
                }
                GlobalTLV::FRAGMENTATION_PARAMETERS => {
                    let mut reader = BuffaloZdo::new(payload.clone());
                    let mut value = json!({
                        "nwkAddress": reader.read_u16(),
                    });
                    if reader.is_more_by(1) {
                        value["fragmentationOptions"] = json!(reader.read_u8());
                    }
                    if reader.is_more_by(2) {
                        value["maxIncomingTransferUnit"] = json!(reader.read_u16());
                    }
                    value
                }
                _ => {
                    if tlvs
                        .iter()
                        .any(|existing: &Value| existing["tagId"] == json!(tag))
                    {
                        if tag != GlobalTLV::MANUFACTURER_SPECIFIC {
                            panic!("Duplicate TLV tag");
                        }
                    }
                    json!({ "raw": payload })
                }
            };
            tlvs.push(json!({
                "tagId": tag,
                "length": length,
                "tlv": tlv,
            }));
        }
        tlvs
    }

    pub fn write_global_tlv(&mut self, tlv: &Value) {
        let tag = tlv["tagId"].as_u64().unwrap_or(0xff) as u8;
        if !matches!(
            tag,
            GlobalTLV::MANUFACTURER_SPECIFIC
                | GlobalTLV::FRAGMENTATION_PARAMETERS
                | GlobalTLV::BEACON_APPENDIX_ENCAPSULATION
                | GlobalTLV::CONFIGURATION_PARAMETERS
                | GlobalTLV::ROUTER_INFORMATION
        ) {
            panic!("Invalid TLV tag");
        }
        if tag == GlobalTLV::FRAGMENTATION_PARAMETERS {
            let value = &tlv["tlv"];
            let mut payload = Vec::new();
            payload.extend_from_slice(
                &(value["nwkAddress"].as_u64().unwrap_or(0) as u16).to_le_bytes(),
            );
            if let Some(options) = value.get("fragmentationOptions").and_then(Value::as_u64) {
                payload.push(options as u8);
            }
            if let Some(max) = value.get("maxIncomingTransferUnit").and_then(Value::as_u64) {
                payload.extend_from_slice(&(max as u16).to_le_bytes());
            }
            self.buffer.push(tag);
            self.buffer.push((payload.len() - 1) as u8);
            self.buffer.extend_from_slice(&payload);
            return;
        }
        if tag == GlobalTLV::MANUFACTURER_SPECIFIC {
            let value = &tlv["tlv"];
            let mut payload = Vec::new();
            payload.extend_from_slice(
                &(value["zigbeeManufacturerId"].as_u64().unwrap_or(0) as u16).to_le_bytes(),
            );
            if let Some(data) = value.get("additionalData").and_then(Value::as_array) {
                payload.extend(data.iter().filter_map(Value::as_u64).map(|byte| byte as u8));
            }
            self.buffer.push(tag);
            self.buffer.push((payload.len() - 1) as u8);
            self.buffer.extend_from_slice(&payload);
            return;
        }
        if tag == GlobalTLV::BEACON_APPENDIX_ENCAPSULATION {
            let mut payload = Vec::new();
            if let Some(inner) = tlv["tlv"]["additionalTLVs"].as_array() {
                for nested in inner {
                    let before = payload.len();
                    let mut writer = BuffaloZdo::new(Vec::new());
                    writer.write_global_tlv(nested);
                    payload.extend_from_slice(&writer.buffer[..]);
                    let _ = before;
                }
            }
            self.buffer.push(tag);
            self.buffer.push((payload.len() - 1) as u8);
            self.buffer.extend_from_slice(&payload);
        }
    }

    pub fn build_request(
        has_zdo_message_overhead: bool,
        cluster_id: u16,
        args: &[Value],
    ) -> Vec<u8> {
        build_request_internal(has_zdo_message_overhead, cluster_id, args, false)
    }

    /// Upstream `Zdo.Buffalo.checkStatus(result)`.
    pub fn check_status(result: &Value) -> bool {
        result.get(0).and_then(Value::as_u64) == Some(0)
    }

    pub fn read_response(has_zdo_message_overhead: bool, cluster_id: u16, buffer: &[u8]) -> Value {
        read_response_internal(has_zdo_message_overhead, cluster_id, buffer)
    }

    pub fn build_request_be(
        has_zdo_message_overhead: bool,
        cluster_id: u16,
        args: &[Value],
    ) -> Vec<u8> {
        build_request_internal(has_zdo_message_overhead, cluster_id, args, true)
    }

    pub fn read_response_be(
        has_zdo_message_overhead: bool,
        cluster_id: u16,
        buffer: &[u8],
    ) -> Value {
        read_response_internal(has_zdo_message_overhead, cluster_id, buffer)
    }

    // Upstream exposes one reader per response type; the port implements the
    // same parsing in `read_response` (single entry point).
}

fn u64_arg(value: &Value) -> u64 {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
        .unwrap_or(0)
}

fn u16_arg(value: &Value) -> u16 {
    u64_arg(value) as u16
}

fn bool_arg(value: &Value) -> u8 {
    value
        .as_bool()
        .map(u8::from)
        .unwrap_or_else(|| u64_arg(value) as u8)
}

fn bytes_arg(value: &Value) -> Vec<u8> {
    value
        .as_array()
        .map(|items| items.iter().map(|item| u64_arg(item) as u8).collect())
        .unwrap_or_default()
}

fn eui64_bytes(value: &Value) -> Vec<u8> {
    let text = value.as_str().unwrap_or("0x0").trim_start_matches("0x");
    let mut bytes = Vec::new();
    let padded = format!("{text:0>16}");
    let chars: Vec<char> = padded.chars().collect();
    let mut index = chars.len();
    while index >= 2 {
        let pair: String = chars[index - 2..index].iter().collect();
        bytes.push(u8::from_str_radix(&pair, 16).unwrap_or(0));
        index -= 2;
    }
    bytes
}

fn push_u16(buffer: &mut Vec<u8>, value: u16, be: bool) {
    if be {
        buffer.extend_from_slice(&value.to_be_bytes());
    } else {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
}

fn push_u32(buffer: &mut Vec<u8>, value: u32, be: bool) {
    if be {
        buffer.extend_from_slice(&value.to_be_bytes());
    } else {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
}

fn push_eui64(buffer: &mut Vec<u8>, value: &Value, be: bool) {
    let mut bytes = eui64_bytes(value);
    if be {
        bytes.reverse();
    }
    buffer.extend_from_slice(&bytes);
}

fn build_request_internal(overhead: bool, cluster_id: u16, args: &[Value], be: bool) -> Vec<u8> {
    let mut buffer = Vec::new();
    match cluster_id {
        ClusterId::ACTIVE_ENDPOINTS_REQUEST | ClusterId::POWER_DESCRIPTOR_REQUEST => {
            buffer.push(0);
            push_u16(&mut buffer, u16_arg(&args[0]), be);
        }
        ClusterId::NODE_DESCRIPTOR_REQUEST => {
            buffer.push(0);
            push_u16(&mut buffer, u16_arg(&args[0]), be);
            if let Some(tlv) = args.get(1).filter(|value| value.is_object()) {
                let mut writer = BuffaloZdo::new(Vec::new());
                writer.write_global_tlv(&json!({
                    "tagId": GlobalTLV::FRAGMENTATION_PARAMETERS,
                    "tlv": tlv,
                }));
                buffer.extend_from_slice(&writer.buffer);
            }
        }
        ClusterId::SIMPLE_DESCRIPTOR_REQUEST => {
            buffer.push(0);
            push_u16(&mut buffer, u16_arg(&args[0]), be);
            buffer.push(u64_arg(&args[1]) as u8);
        }
        ClusterId::MATCH_DESCRIPTORS_REQUEST => {
            buffer.push(0);
            push_u16(&mut buffer, u16_arg(&args[0]), be);
            push_u16(&mut buffer, u16_arg(&args[1]), be);
            for list in &args[2..4] {
                let clusters = list.as_array().cloned().unwrap_or_default();
                buffer.push(clusters.len() as u8);
                for cluster in clusters {
                    push_u16(&mut buffer, u16_arg(&cluster), be);
                }
            }
        }
        ClusterId::BIND_REQUEST | ClusterId::UNBIND_REQUEST => {
            buffer.push(0);
            push_eui64(&mut buffer, &args[0], be);
            buffer.push(u64_arg(&args[1]) as u8);
            push_u16(&mut buffer, u16_arg(&args[2]), be);
            let mode = u64_arg(&args[3]) as u8;
            buffer.push(mode);
            if mode == 3 {
                push_eui64(&mut buffer, &args[4], be);
                buffer.push(u64_arg(&args[6]) as u8);
            } else if mode == 1 {
                push_u16(&mut buffer, u16_arg(&args[5]), be);
                if be {
                    buffer.push(u64_arg(&args[6]) as u8);
                }
            } else {
                panic!("Invalid binding type");
            }
        }
        ClusterId::BINDING_TABLE_REQUEST
        | ClusterId::LQI_TABLE_REQUEST
        | ClusterId::ROUTING_TABLE_REQUEST
        | ClusterId::NWK_IEEE_JOINING_LIST_REQUEST => {
            buffer.push(0);
            buffer.push(u64_arg(&args[0]) as u8);
        }
        ClusterId::LEAVE_REQUEST => {
            buffer.push(0);
            push_eui64(&mut buffer, &args[0], be);
            buffer.push(u64_arg(&args[1]) as u8);
            if be {
                buffer.push(0);
            }
        }
        ClusterId::PERMIT_JOINING_REQUEST => {
            buffer.push(0);
            buffer.push(u64_arg(&args[0]) as u8);
            if args.len() > 1 && !args[1].is_null() {
                buffer.push(u64_arg(&args[1]) as u8);
            }
            if let Some(tlvs) = args.get(2).and_then(Value::as_array) {
                for tlv in tlvs {
                    let mut writer = BuffaloZdo::new(Vec::new());
                    writer.write_global_tlv(tlv);
                    buffer.extend_from_slice(&writer.buffer);
                }
            }
        }
        ClusterId::NETWORK_ADDRESS_REQUEST => {
            if overhead {
                buffer.push(0);
            }
            push_eui64(&mut buffer, &args[0], be);
            buffer.push(bool_arg(&args[1]));
            buffer.push(u64_arg(&args[2]) as u8);
        }
        ClusterId::IEEE_ADDRESS_REQUEST => {
            if overhead {
                buffer.push(0);
            }
            push_u16(&mut buffer, u16_arg(&args[0]), be);
            buffer.push(bool_arg(&args[1]));
            buffer.push(u64_arg(&args[2]) as u8);
        }
        ClusterId::NWK_UPDATE_REQUEST => {
            if overhead {
                buffer.push(0);
            }
            let mask = crate::zspec::utils::channels_to_uint32_mask(&bytes_arg(&args[0]));
            push_u32(&mut buffer, mask, be);
            if u64_arg(&args[1]) == 0xfe {
                buffer.push(0xfe);
                if !args[3].is_null() {
                    buffer.push(u64_arg(&args[3]) as u8);
                }
            } else if args.get(3).is_none_or(Value::is_null)
                && args.get(4).is_none_or(Value::is_null)
            {
                buffer.push(u64_arg(&args[1]) as u8);
                if u64_arg(&args[1]) != 64 {
                    buffer.push(u64_arg(&args[2]) as u8);
                }
            } else {
                buffer.push(u64_arg(&args[1]) as u8);
                buffer.push(u64_arg(&args[3]) as u8);
                push_u16(&mut buffer, u16_arg(&args[4]), be);
            }
        }
        ClusterId::NWK_ENHANCED_UPDATE_REQUEST => {
            if let Some(pages) = args[0].as_array() {
                buffer.push(0);
                buffer.push(pages.len() as u8);
                for page in pages {
                    push_u32(&mut buffer, u64_arg(page) as u32, be);
                }
                let mode = u64_arg(&args[1]) as u8;
                buffer.push(mode);
                if mode == 0xff {
                    buffer.push(u64_arg(&args[3]) as u8);
                    push_u16(&mut buffer, u16_arg(&args[4]), be);
                    buffer.push(u64_arg(&args[5]) as u8);
                } else if mode == 6 {
                    buffer.push(u64_arg(&args[5]) as u8);
                } else if mode == 0xfe {
                    buffer.push(u64_arg(&args[3]) as u8);
                    buffer.push(u64_arg(&args[5]) as u8);
                } else {
                    buffer.push(u64_arg(&args[2]) as u8);
                    buffer.push(u64_arg(&args[5]) as u8);
                }
            } else {
                buffer.push(0);
                buffer.push(1);
                push_u32(&mut buffer, u64_arg(&args[0]) as u32, be);
                buffer.push(0xfe);
                buffer.push(u64_arg(&args[3]) as u8);
                buffer.push(u64_arg(&args[5]) as u8);
            }
        }
        ClusterId::CLEAR_ALL_BINDINGS_REQUEST | ClusterId::DECOMMISSION_REQUEST => {
            let list = args[0]["eui64List"].as_array().cloned().unwrap_or_default();
            buffer.push(0);
            buffer.push(0);
            buffer.push((8 * list.len() + 1 - 1) as u8);
            buffer.push(list.len() as u8);
            for eui in list {
                push_eui64(&mut buffer, &eui, be);
            }
        }
        ClusterId::PARENT_ANNOUNCE => {
            let list = args[0].as_array().cloned().unwrap_or_default();
            buffer.push(0);
            buffer.push(list.len() as u8);
            for eui in list {
                push_eui64(&mut buffer, &eui, be);
            }
        }
        ClusterId::CHALLENGE_REQUEST => {
            buffer.push(0);
            buffer.push(0);
            buffer.push(8 + 8 - 1);
            push_eui64(&mut buffer, &args[0]["senderEui64"], be);
            buffer.extend_from_slice(&bytes_arg(&args[0]["challengeValue"]));
        }
        ClusterId::START_KEY_NEGOTIATION_REQUEST => {
            buffer.push(0);
            buffer.push(0);
            buffer.push(8 + 32 - 1);
            push_eui64(&mut buffer, &args[0]["eui64"], be);
            buffer.extend_from_slice(&bytes_arg(&args[0]["publicPoint"]));
        }
        ClusterId::START_KEY_UPDATE_REQUEST => {
            buffer.push(0);
            buffer.push(0);
            buffer.push(8 + 2 - 1);
            buffer.push(u64_arg(&args[0]["protocol"]) as u8);
            buffer.push(u64_arg(&args[0]["presharedSecret"]) as u8);
            push_eui64(&mut buffer, &args[0]["sendingDeviceEui64"], be);
            let mut writer = BuffaloZdo::new(Vec::new());
            writer.write_global_tlv(&json!({
                "tagId": GlobalTLV::FRAGMENTATION_PARAMETERS,
                "tlv": args[1],
            }));
            buffer.extend_from_slice(&writer.buffer);
        }
        ClusterId::RETRIEVE_AUTHENTICATION_TOKEN_REQUEST => {
            buffer.push(0);
            buffer.push(0);
            buffer.push(0);
            buffer.push(u64_arg(&args[0]["tlvTypeTagId"]) as u8);
        }
        ClusterId::GET_AUTHENTICATION_LEVEL_REQUEST => {
            buffer.push(0);
            buffer.push(0);
            buffer.push(8 - 1);
            push_eui64(&mut buffer, &args[0]["ieee"], be);
        }
        ClusterId::GET_CONFIGURATION_REQUEST => {
            let list = args[0].as_array().cloned().unwrap_or_default();
            buffer.push(0);
            buffer.push(list.len() as u8);
            for item in list {
                buffer.push(u64_arg(&item) as u8);
            }
        }
        ClusterId::SYSTEM_SERVER_DISCOVERY_REQUEST => {
            buffer.push(0);
            let mask = server_mask_from_value(&args[0]);
            push_u16(&mut buffer, mask, be);
        }
        ClusterId::NWK_BEACON_SURVEY_REQUEST => {
            let pages = args[0]["scanChannelList"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            buffer.push(0);
            buffer.push(0);
            buffer.push((1 + 4 * pages.len()) as u8);
            buffer.push(pages.len() as u8);
            for page in pages {
                push_u32(&mut buffer, u64_arg(&page) as u32, be);
            }
            buffer.push(u64_arg(&args[0]["configurationBitmask"]) as u8);
        }
        _ => panic!("Unknown cluster id {cluster_id:#06x}"),
    }
    buffer
}

fn server_mask_from_value(value: &Value) -> u16 {
    let mask = crate::zspec::zdo::utils::ServerMask {
        primary_trust_center: value["primaryTrustCenter"].as_u64().unwrap_or(0) as u16,
        backup_trust_center: value["backupTrustCenter"].as_u64().unwrap_or(0) as u16,
        deprecated1: value["deprecated1"].as_u64().unwrap_or(0) as u16,
        deprecated2: value["deprecated2"].as_u64().unwrap_or(0) as u16,
        deprecated3: value["deprecated3"].as_u64().unwrap_or(0) as u16,
        deprecated4: value["deprecated4"].as_u64().unwrap_or(0) as u16,
        network_manager: value["networkManager"].as_u64().unwrap_or(0) as u16,
        reserved1: value["reserved1"].as_u64().unwrap_or(0) as u16,
        reserved2: value["reserved2"].as_u64().unwrap_or(0) as u16,
        stack_compliance_revision: value["stackComplianceRevision"].as_u64().unwrap_or(0) as u16,
    };
    utils::create_server_mask(mask)
}

fn server_mask_to_value(mask: u16) -> Value {
    let value = utils::get_server_mask(mask);
    json!({
        "primaryTrustCenter": value.primary_trust_center,
        "backupTrustCenter": value.backup_trust_center,
        "deprecated1": value.deprecated1,
        "deprecated2": value.deprecated2,
        "deprecated3": value.deprecated3,
        "deprecated4": value.deprecated4,
        "networkManager": value.network_manager,
        "reserved1": value.reserved1,
        "reserved2": value.reserved2,
        "stackComplianceRevision": value.stack_compliance_revision,
    })
}

fn capabilities_to_value(value: u8) -> Value {
    json!({
        "alternatePANCoordinator": value & 0x01,
        "deviceType": (value & 0x02) >> 1,
        "powerSource": (value & 0x04) >> 2,
        "rxOnWhenIdle": (value & 0x08) >> 3,
        "reserved1": (value & 0x10) >> 4,
        "reserved2": (value & 0x20) >> 5,
        "securityCapability": (value & 0x40) >> 6,
        "allocateAddress": (value & 0x80) >> 7,
    })
}

fn is_supported_response(cluster_id: u16) -> bool {
    matches!(
        cluster_id,
        ClusterId::ACTIVE_ENDPOINTS_RESPONSE
            | ClusterId::MATCH_DESCRIPTORS_RESPONSE
            | ClusterId::NETWORK_ADDRESS_RESPONSE
            | ClusterId::IEEE_ADDRESS_RESPONSE
            | ClusterId::NODE_DESCRIPTOR_RESPONSE
            | ClusterId::POWER_DESCRIPTOR_RESPONSE
            | ClusterId::SIMPLE_DESCRIPTOR_RESPONSE
            | ClusterId::BINDING_TABLE_RESPONSE
            | ClusterId::CHALLENGE_RESPONSE
            | ClusterId::GET_AUTHENTICATION_LEVEL_RESPONSE
            | ClusterId::GET_CONFIGURATION_RESPONSE
            | ClusterId::RETRIEVE_AUTHENTICATION_TOKEN_RESPONSE
            | ClusterId::START_KEY_NEGOTIATION_RESPONSE
            | ClusterId::LQI_TABLE_RESPONSE
            | ClusterId::ROUTING_TABLE_RESPONSE
            | ClusterId::BIND_RESPONSE
            | ClusterId::UNBIND_RESPONSE
            | ClusterId::CLEAR_ALL_BINDINGS_RESPONSE
            | ClusterId::LEAVE_RESPONSE
            | ClusterId::PERMIT_JOINING_RESPONSE
            | ClusterId::START_KEY_UPDATE_RESPONSE
            | ClusterId::DECOMMISSION_RESPONSE
            | ClusterId::NWK_UPDATE_RESPONSE
            | ClusterId::NWK_ENHANCED_UPDATE_RESPONSE
            | ClusterId::NWK_UNSOLICITED_ENHANCED_UPDATE_RESPONSE
            | ClusterId::NWK_IEEE_JOINING_LIST_RESPONSE
            | ClusterId::NWK_BEACON_SURVEY_RESPONSE
            | ClusterId::PARENT_ANNOUNCE_RESPONSE
            | ClusterId::SET_CONFIGURATION_RESPONSE
            | ClusterId::SYSTEM_SERVER_DISCOVERY_RESPONSE
            | ClusterId::END_DEVICE_ANNOUNCE
    )
}

fn read_response_internal(overhead: bool, cluster_id: u16, buffer: &[u8]) -> Value {
    // Upstream switches on the cluster first and throws for anything unknown,
    // before any status parsing.
    if !is_supported_response(cluster_id) {
        panic!("Unsupported response reading for cluster ID '{cluster_id}'.");
    }
    let mut reader = BuffaloZdo::new(buffer.to_vec());
    if overhead {
        reader.read_u8();
    }
    if cluster_id == ClusterId::END_DEVICE_ANNOUNCE {
        let nwk = reader.read_u16();
        let eui = reader.read_eui64();
        let capabilities = reader.read_u8();
        return json!([
            0,
            {
                "nwkAddress": nwk,
                "eui64": eui,
                "capabilities": capabilities_to_value(capabilities),
            }
        ]);
    }
    let status = reader.read_u8();
    // Every upstream response reader only consumes the payload when the status
    // is `SUCCESS`; otherwise it returns `[status, undefined]`. This also keeps
    // short error responses from reading past the buffer.
    if status != 0 {
        return json!([status, Value::Null]);
    }
    let value = match cluster_id {
        ClusterId::ACTIVE_ENDPOINTS_RESPONSE | ClusterId::MATCH_DESCRIPTORS_RESPONSE => {
            let nwk = reader.read_u16();
            let count = reader.read_u8();
            let endpoints = reader.read_buffer(usize::from(count));
            json!({"nwkAddress": nwk, "endpointList": endpoints})
        }
        ClusterId::NETWORK_ADDRESS_RESPONSE | ClusterId::IEEE_ADDRESS_RESPONSE => {
            let eui = reader.read_eui64();
            let nwk = reader.read_u16();
            if reader.is_more_by(1) {
                let count = reader.read_u8();
                let start_index = reader.read_u8();
                let list = (0..count)
                    .map(|_| json!(reader.read_u16()))
                    .collect::<Vec<_>>();
                json!({
                    "eui64": eui,
                    "nwkAddress": nwk,
                    "startIndex": start_index,
                    "assocDevList": list,
                })
            } else {
                json!({
                    "eui64": eui,
                    "nwkAddress": nwk,
                    "startIndex": 0,
                    "assocDevList": [],
                })
            }
        }
        ClusterId::NODE_DESCRIPTOR_RESPONSE => {
            let nwk = reader.read_u16();
            let logical_byte = reader.read_u8();
            let flags = reader.read_u8();
            let capabilities = reader.read_u8();
            let manufacturer = reader.read_u16();
            let max_buf = reader.read_u8();
            let max_inc = reader.read_u16();
            let mask = reader.read_u16();
            let max_out = reader.read_u16();
            let deprecated = reader.read_u8();
            let tlvs = reader.read_tlvs();
            json!({
                "nwkAddress": nwk,
                "logicalType": logical_byte & 0x07,
                "fragmentationSupported": if tlvs.is_empty() { Value::Null } else { json!(true) },
                "apsFlags": flags & 0x07,
                "frequencyBand": (flags >> 3) & 0x1f,
                "capabilities": capabilities_to_value(capabilities),
                "manufacturerCode": manufacturer,
                "maxBufSize": max_buf,
                "maxIncTxSize": max_inc,
                "serverMask": server_mask_to_value(mask),
                "maxOutTxSize": max_out,
                "deprecated1": deprecated,
                "tlvs": tlvs,
            })
        }
        ClusterId::POWER_DESCRIPTOR_RESPONSE => {
            let nwk = reader.read_u16();
            let first = reader.read_u8();
            let second = reader.read_u8();
            json!({
                "nwkAddress": nwk,
                "currentPowerMode": first & 0x0f,
                "availPowerSources": first >> 4,
                "currentPowerSource": second & 0x0f,
                "currentPowerSourceLevel": second >> 4,
            })
        }
        ClusterId::SIMPLE_DESCRIPTOR_RESPONSE => {
            let nwk = reader.read_u16();
            let length = reader.read_u8();
            let endpoint = reader.read_u8();
            let profile = reader.read_u16();
            let device_id = reader.read_u16();
            let device_version = reader.read_u8();
            let in_count = reader.read_u8();
            let in_clusters = (0..in_count)
                .map(|_| json!(reader.read_u16()))
                .collect::<Vec<_>>();
            let out_count = reader.read_u8();
            let out_clusters = (0..out_count)
                .map(|_| json!(reader.read_u16()))
                .collect::<Vec<_>>();
            json!({
                "nwkAddress": nwk,
                "length": length,
                "endpoint": endpoint,
                "profileId": profile,
                "deviceId": device_id,
                "deviceVersion": device_version,
                "inClusterList": in_clusters,
                "outClusterList": out_clusters,
            })
        }
        ClusterId::BINDING_TABLE_RESPONSE => {
            let entries = reader.read_u8();
            let start_index = reader.read_u8();
            let count = reader.read_u8();
            let mut list = Vec::new();
            for _ in 0..count {
                let source = reader.read_eui64();
                let source_endpoint = reader.read_u8();
                let cluster = reader.read_u16();
                let mode = reader.read_u8();
                let (dest, dest_endpoint) = if mode == 3 {
                    (json!(reader.read_eui64()), json!(reader.read_u8()))
                } else if mode == 1 {
                    (json!(reader.read_u16()), Value::Null)
                } else {
                    continue;
                };
                list.push(json!({
                    "sourceEui64": source,
                    "sourceEndpoint": source_endpoint,
                    "clusterId": cluster,
                    "destAddrMode": mode,
                    "dest": dest,
                    "destEndpoint": dest_endpoint,
                }));
            }
            json!({
                "bindingTableEntries": entries,
                "startIndex": start_index,
                "entryList": list,
            })
        }
        ClusterId::CHALLENGE_RESPONSE
        | ClusterId::GET_AUTHENTICATION_LEVEL_RESPONSE
        | ClusterId::GET_CONFIGURATION_RESPONSE
        | ClusterId::RETRIEVE_AUTHENTICATION_TOKEN_RESPONSE
        | ClusterId::START_KEY_NEGOTIATION_RESPONSE => {
            let tlvs = read_typed_tlvs(&mut reader, cluster_id);
            json!({"tlvs": tlvs})
        }
        ClusterId::LQI_TABLE_RESPONSE => {
            let entries = reader.read_u8();
            let start_index = reader.read_u8();
            let count = reader.read_u8();
            let mut list = Vec::new();
            for _ in 0..count {
                let ext_pan = reader.read_buffer(8);
                let eui = reader.read_eui64();
                let nwk = reader.read_u16();
                let first = reader.read_u8();
                let second = reader.read_u8();
                let depth = reader.read_u8();
                let lqi = reader.read_u8();
                list.push(json!({
                    "extendedPanId": ext_pan,
                    "eui64": eui,
                    "nwkAddress": nwk,
                    "deviceType": first & 0x03,
                    "rxOnWhenIdle": (first >> 2) & 0x01,
                    "relationship": (first >> 4) & 0x07,
                    "reserved1": (first >> 7) & 0x01,
                    "permitJoining": second & 0x01,
                    "reserved2": (second >> 1) & 0x7f,
                    "depth": depth,
                    "lqi": lqi,
                }));
            }
            json!({
                "neighborTableEntries": entries,
                "startIndex": start_index,
                "entryList": list,
            })
        }
        ClusterId::ROUTING_TABLE_RESPONSE => {
            let entries = reader.read_u8();
            let start_index = reader.read_u8();
            let count = reader.read_u8();
            let mut list = Vec::new();
            for _ in 0..count {
                let dest = reader.read_u16();
                let flags = reader.read_u8();
                let next_hop = reader.read_u16();
                list.push(json!({
                    "destinationAddress": dest,
                    "status": match flags & 0x07 {
                        value if value == RoutingTableStatus::ACTIVE => "ACTIVE",
                        value if value == RoutingTableStatus::DISCOVERY_UNDERWAY => "DISCOVERY_UNDERWAY",
                        value if value == RoutingTableStatus::DISCOVERY_FAILED => "DISCOVERY_FAILED",
                        value if value == RoutingTableStatus::INACTIVE => "INACTIVE",
                        _ => "VALIDATION_UNDERWAY",
                    },
                    "memoryConstrained": (flags >> 3) & 0x01,
                    "manyToOne": (flags >> 4) & 0x01,
                    "routeRecordRequired": (flags >> 5) & 0x01,
                    "reserved1": (flags >> 6) & 0x01,
                    "nextHopAddress": next_hop,
                }));
            }
            json!({
                "routingTableEntries": entries,
                "startIndex": start_index,
                "entryList": list,
            })
        }
        ClusterId::BIND_RESPONSE
        | ClusterId::UNBIND_RESPONSE
        | ClusterId::CLEAR_ALL_BINDINGS_RESPONSE
        | ClusterId::LEAVE_RESPONSE
        | ClusterId::PERMIT_JOINING_RESPONSE
        | ClusterId::START_KEY_UPDATE_RESPONSE
        | ClusterId::DECOMMISSION_RESPONSE => Value::Null,
        ClusterId::NWK_UPDATE_RESPONSE | ClusterId::NWK_ENHANCED_UPDATE_RESPONSE => {
            let scanned = reader.read_u32();
            let transmissions = reader.read_u16();
            let failures = reader.read_u16();
            let count = reader.read_u8();
            let list = reader.read_buffer(usize::from(count));
            json!({
                "scannedChannels": scanned,
                "totalTransmissions": transmissions,
                "totalFailures": failures,
                "entryList": list,
            })
        }
        ClusterId::NWK_UNSOLICITED_ENHANCED_UPDATE_RESPONSE => {
            json!({
                "channelInUse": reader.read_u32(),
                "macTxUCastTotal": reader.read_u16(),
                "macTxUCastFailures": reader.read_u16(),
                "macTxUCastRetries": reader.read_u16(),
                "timePeriod": reader.read_u8(),
            })
        }
        ClusterId::NWK_IEEE_JOINING_LIST_RESPONSE => {
            let update_id = reader.read_u8();
            let policy = reader.read_u8();
            let total = reader.read_u8();
            if total == 0 {
                json!({
                    "updateId": update_id,
                    "joiningPolicy": policy,
                    "entryListTotal": total,
                    "startIndex": Value::Null,
                    "entryList": Value::Null,
                })
            } else {
                let start_index = reader.read_u8();
                let count = reader.read_u8();
                let list = (0..count)
                    .map(|_| json!(reader.read_eui64()))
                    .collect::<Vec<_>>();
                json!({
                    "updateId": update_id,
                    "joiningPolicy": policy,
                    "entryListTotal": total,
                    "startIndex": start_index,
                    "entryList": list,
                })
            }
        }
        ClusterId::NWK_BEACON_SURVEY_RESPONSE => {
            let tlvs = read_beacon_survey_tlvs(&mut reader);
            json!({"tlvs": tlvs})
        }
        ClusterId::PARENT_ANNOUNCE_RESPONSE => {
            let count = reader.read_u8();
            let list = (0..count)
                .map(|_| json!(reader.read_eui64()))
                .collect::<Vec<_>>();
            json!({"children": list})
        }
        ClusterId::SET_CONFIGURATION_RESPONSE => {
            let tlvs = read_set_configuration_tlvs(&mut reader);
            json!({"tlvs": tlvs})
        }
        ClusterId::SYSTEM_SERVER_DISCOVERY_RESPONSE => {
            json!({"serverMask": server_mask_to_value(reader.read_u16())})
        }
        _ => panic!("Unknown cluster id {cluster_id:#06x}"),
    };
    json!([status, value])
}

fn read_typed_tlvs(reader: &mut BuffaloZdo, cluster_id: u16) -> Vec<Value> {
    let mut tlvs = Vec::new();
    while reader.is_more_by(2) {
        let tag = reader.read_u8();
        let length_minus_one = reader.read_u8();
        let length = usize::from(length_minus_one) + 1;
        let payload = reader.read_buffer(length);
        let mut sub = BuffaloZdo::new(payload.clone());
        let tlv = match cluster_id {
            ClusterId::CHALLENGE_RESPONSE if tag == 0 => json!({
                "responderEui64": sub.read_eui64(),
                "receivedChallengeValue": sub.read_buffer(8),
                "apsFrameCounter": sub.read_u32(),
                "challengeSecurityFrameCounter": sub.read_u32(),
                "mic": sub.read_buffer(8),
            }),
            ClusterId::GET_AUTHENTICATION_LEVEL_RESPONSE if tag == 0 => json!({
                "remoteNodeIeee": sub.read_eui64(),
                "initialJoinMethod": sub.read_u8(),
                "activeLinkKeyType": sub.read_u8(),
            }),
            ClusterId::GET_CONFIGURATION_RESPONSE
            | ClusterId::RETRIEVE_AUTHENTICATION_TOKEN_RESPONSE
                if tag == GlobalTLV::FRAGMENTATION_PARAMETERS =>
            {
                let mut value = json!({"nwkAddress": sub.read_u16()});
                if sub.is_more_by(1) {
                    value["fragmentationOptions"] = json!(sub.read_u8());
                }
                if sub.is_more_by(2) {
                    value["maxIncomingTransferUnit"] = json!(sub.read_u16());
                }
                value
            }
            ClusterId::START_KEY_NEGOTIATION_RESPONSE if tag == 0 => json!({
                "eui64": sub.read_eui64(),
                "publicPoint": sub.read_buffer(32),
            }),
            _ => json!({"raw": payload}),
        };
        tlvs.push(json!({"tagId": tag, "length": length, "tlv": tlv}));
    }
    tlvs
}

fn read_beacon_survey_tlvs(reader: &mut BuffaloZdo) -> Vec<Value> {
    let mut tlvs = Vec::new();
    while reader.is_more_by(2) {
        let tag = reader.read_u8();
        let length_minus_one = reader.read_u8();
        let length = usize::from(length_minus_one) + 1;
        let payload = reader.read_buffer(length);
        let raw = payload.clone();
        let mut sub = BuffaloZdo::new(payload);
        let tlv = match tag {
            0x01 => json!({
                "totalBeaconsReceived": sub.read_u8(),
                "onNetworkBeacons": sub.read_u8(),
                "potentialParentBeacons": sub.read_u8(),
                "otherNetworkBeacons": sub.read_u8(),
            }),
            0x02 => {
                let nwk = sub.read_u16();
                let lqa = sub.read_u8();
                let count = sub.read_u8();
                let parents = (0..count)
                    .map(|_| json!({"nwkAddress": sub.read_u16(), "lqa": sub.read_u8()}))
                    .collect::<Vec<_>>();
                json!({
                    "currentParentNwkAddress": nwk,
                    "currentParentLQA": lqa,
                    "entryCount": count,
                    "potentialParents": parents,
                })
            }
            _ => json!({"raw": raw}),
        };
        tlvs.push(json!({"tagId": tag, "length": length, "tlv": tlv}));
    }
    tlvs
}

fn read_set_configuration_tlvs(reader: &mut BuffaloZdo) -> Vec<Value> {
    let mut tlvs = Vec::new();
    while reader.is_more_by(2) {
        let tag = reader.read_u8();
        let length_minus_one = reader.read_u8();
        let length = usize::from(length_minus_one) + 1;
        let payload = reader.read_buffer(length);
        let raw = payload.clone();
        let mut sub = BuffaloZdo::new(payload);
        let tlv = if tag == 0x00 {
            let count = sub.read_u8();
            let entries = (0..count)
                .map(|_| {
                    json!({
                        "tagId": sub.read_u8(),
                        "processingStatus": sub.read_u8(),
                    })
                })
                .collect::<Vec<_>>();
            json!({"count": count, "tlvs": entries})
        } else {
            json!({"raw": raw})
        };
        tlvs.push(json!({"tagId": tag, "length": length, "tlv": tlv}));
    }
    tlvs
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Upstream `readBindResponse` / `readUnbindResponse` /
    /// `readClearAllBindingsResponse` / `readLeaveResponse` /
    /// `readPermitJoiningResponse` / `readStartKeyUpdateResponse` /
    /// `readDecommissionResponse` all return `[status, undefined]`.
    #[test]
    fn reads_status_only_responses() {
        for cluster in [
            ClusterId::BIND_RESPONSE,
            ClusterId::UNBIND_RESPONSE,
            ClusterId::CLEAR_ALL_BINDINGS_RESPONSE,
            ClusterId::LEAVE_RESPONSE,
            ClusterId::PERMIT_JOINING_RESPONSE,
            ClusterId::START_KEY_UPDATE_RESPONSE,
            ClusterId::DECOMMISSION_RESPONSE,
        ] {
            assert_eq!(
                BuffaloZdo::read_response(false, cluster, &[0x00]),
                serde_json::json!([0, null])
            );
            assert_eq!(
                BuffaloZdo::read_response(false, cluster, &[0x02]),
                serde_json::json!([2, null])
            );
        }
    }
}

#[cfg(test)]
mod short_error_response_tests {
    use super::*;

    /// Upstream returns `[status, undefined]` and does not read the payload on
    /// non-success responses; short error frames must not panic.
    #[test]
    fn non_success_responses_return_status_only() {
        for cluster in [
            ClusterId::ACTIVE_ENDPOINTS_RESPONSE,
            ClusterId::NETWORK_ADDRESS_RESPONSE,
            ClusterId::IEEE_ADDRESS_RESPONSE,
            ClusterId::NODE_DESCRIPTOR_RESPONSE,
            ClusterId::POWER_DESCRIPTOR_RESPONSE,
            ClusterId::SIMPLE_DESCRIPTOR_RESPONSE,
            ClusterId::LQI_TABLE_RESPONSE,
            ClusterId::ROUTING_TABLE_RESPONSE,
            ClusterId::BINDING_TABLE_RESPONSE,
            ClusterId::NWK_UPDATE_RESPONSE,
            ClusterId::PARENT_ANNOUNCE_RESPONSE,
        ] {
            assert_eq!(
                BuffaloZdo::read_response(false, cluster, &[0x02]),
                serde_json::json!([2, null]),
                "cluster {cluster:#06x}"
            );
        }
    }
}
