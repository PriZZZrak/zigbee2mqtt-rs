//! Encoder for ZDO response payloads.
//!
//! The wire layouts mirror `BuffaloZdo::read_response`
//! (`crates/zigbee-apiarist/src/zspec/zdo/buffalo_zdo.rs`), which follows
//! upstream `Zdo.Buffalo.readResponse`.

#![allow(dead_code)]

use serde_json::{json, Value};

use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;
use zigbee_apiarist::zspec::zdo::definition::enums::GlobalTLV;

fn push_eui64(buffer: &mut Vec<u8>, value: &Value) {
    let text = value.as_str().unwrap_or("0x0").trim_start_matches("0x");
    let bytes = (0..8)
        .map(|index| {
            let start = text.len().saturating_sub((index + 1) * 2);
            let end = text.len().saturating_sub(index * 2);
            u8::from_str_radix(&text[start.min(text.len())..end], 16).unwrap_or(0)
        })
        .collect::<Vec<_>>();
    buffer.extend_from_slice(&bytes);
}

fn push_u16(buffer: &mut Vec<u8>, value: u16) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn value_u16(value: &Value) -> u16 {
    value.as_u64().unwrap_or(0) as u16
}

/// Encode a ZDO response payload (status byte included).
pub fn build_zdo_response(cluster_id: u16, args: &[Value]) -> Vec<u8> {
    let mut buffer = Vec::new();
    let status = args.first().and_then(Value::as_u64).unwrap_or(0) as u8;
    buffer.push(status);
    if status != 0 {
        return buffer;
    }
    let payload = args.get(1).cloned().unwrap_or(Value::Null);
    match cluster_id {
        ClusterId::ACTIVE_ENDPOINTS_RESPONSE | ClusterId::MATCH_DESCRIPTORS_RESPONSE => {
            push_u16(&mut buffer, value_u16(&payload["nwkAddress"]));
            let endpoints = payload["endpointList"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            buffer.push(endpoints.len() as u8);
            buffer.extend(
                endpoints
                    .iter()
                    .map(|value| value.as_u64().unwrap_or(0) as u8),
            );
        }
        ClusterId::NETWORK_ADDRESS_RESPONSE | ClusterId::IEEE_ADDRESS_RESPONSE => {
            push_eui64(&mut buffer, &payload["eui64"]);
            push_u16(&mut buffer, value_u16(&payload["nwkAddress"]));
            let list = payload["assocDevList"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            // Z-Stack wire order is `startIndex` then `numassocdev`.
            buffer.push(payload["startIndex"].as_u64().unwrap_or(0) as u8);
            buffer.push(list.len() as u8);
            for entry in list {
                push_u16(&mut buffer, value_u16(&entry));
            }
        }
        ClusterId::NODE_DESCRIPTOR_RESPONSE => {
            push_u16(&mut buffer, value_u16(&payload["nwkAddress"]));
            let logical_type = payload["logicalType"].as_u64().unwrap_or(0) as u8;
            let frequency = payload["frequencyBand"].as_u64().unwrap_or(0) as u8;
            buffer.push(logical_type | (frequency << 3));
            buffer.push(payload["apsFlags"].as_u64().unwrap_or(0) as u8);
            let capabilities = &payload["capabilities"];
            let capability_bits = (capabilities["allocateAddress"].as_u64().unwrap_or(0) as u8)
                | ((capabilities["deviceType"].as_u64().unwrap_or(0) as u8) << 1)
                | ((capabilities["powerSource"].as_u64().unwrap_or(0) as u8) << 3)
                | ((capabilities["rxOnWhenIdle"].as_u64().unwrap_or(0) as u8) << 4)
                | ((capabilities["securityCapability"].as_u64().unwrap_or(0) as u8) << 6);
            buffer.push(capability_bits);
            push_u16(&mut buffer, value_u16(&payload["manufacturerCode"]));
            buffer.push(payload["maxBufSize"].as_u64().unwrap_or(0) as u8);
            push_u16(&mut buffer, value_u16(&payload["maxIncTxSize"]));
            let mask = &payload["serverMask"];
            let mut server_mask = 0u16;
            for (name, bit) in [
                ("primaryTrustCenter", 0),
                ("backupTrustCenter", 1),
                ("deprecated1", 2),
                ("deprecated2", 3),
                ("deprecated3", 4),
                ("deprecated4", 5),
                ("networkManager", 6),
                ("reserved1", 7),
                ("reserved2", 8),
            ] {
                server_mask |= (mask[name].as_u64().unwrap_or(0) as u16 & 1) << bit;
            }
            server_mask |= (mask["stackComplianceRevision"].as_u64().unwrap_or(0) as u16) << 9;
            push_u16(&mut buffer, server_mask);
            push_u16(&mut buffer, value_u16(&payload["maxOutTxSize"]));
            buffer.push(payload["deprecated1"].as_u64().unwrap_or(0) as u8);
            if payload["fragmentationSupported"].as_bool().unwrap_or(false) {
                let mut writer = BuffaloZdo::new(Vec::new());
                writer.write_global_tlv(&json!({
                    "tagId": GlobalTLV::FRAGMENTATION_PARAMETERS,
                    "tlv": {"fragmentationSupported": true},
                }));
                buffer.extend_from_slice(&writer.buffer);
            }
        }
        ClusterId::POWER_DESCRIPTOR_RESPONSE => {
            push_u16(&mut buffer, value_u16(&payload["nwkAddress"]));
            buffer.push(
                (payload["currentPowerMode"].as_u64().unwrap_or(0) as u8 & 0x0f)
                    | ((payload["availPowerSources"].as_u64().unwrap_or(0) as u8 & 0x0f) << 4),
            );
            buffer.push(
                (payload["currentPowerSource"].as_u64().unwrap_or(0) as u8 & 0x0f)
                    | ((payload["currentPowerSourceLevel"].as_u64().unwrap_or(0) as u8 & 0x0f)
                        << 4),
            );
        }
        ClusterId::SIMPLE_DESCRIPTOR_RESPONSE => {
            push_u16(&mut buffer, value_u16(&payload["nwkAddress"]));
            buffer.push(payload["length"].as_u64().unwrap_or(0) as u8);
            buffer.push(payload["endpoint"].as_u64().unwrap_or(0) as u8);
            push_u16(&mut buffer, value_u16(&payload["profileId"]));
            push_u16(&mut buffer, value_u16(&payload["deviceId"]));
            buffer.push(payload["deviceVersion"].as_u64().unwrap_or(0) as u8);
            let in_list = payload["inClusterList"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            buffer.push(in_list.len() as u8);
            for entry in in_list {
                push_u16(&mut buffer, value_u16(&entry));
            }
            let out_list = payload["outClusterList"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            buffer.push(out_list.len() as u8);
            for entry in out_list {
                push_u16(&mut buffer, value_u16(&entry));
            }
        }
        ClusterId::BINDING_TABLE_RESPONSE => {
            buffer.push(payload["bindingTableEntries"].as_u64().unwrap_or(0) as u8);
            buffer.push(payload["startIndex"].as_u64().unwrap_or(0) as u8);
            let list = payload["entryList"].as_array().cloned().unwrap_or_default();
            buffer.push(list.len() as u8);
            for entry in list {
                push_eui64(&mut buffer, &entry["sourceEui64"]);
                buffer.push(entry["sourceEndpoint"].as_u64().unwrap_or(0) as u8);
                push_u16(&mut buffer, value_u16(&entry["clusterId"]));
                let mode = entry["destAddrMode"].as_u64().unwrap_or(0) as u8;
                buffer.push(mode);
                if mode == 3 {
                    push_eui64(&mut buffer, &entry["dest"]);
                    buffer.push(entry["destEndpoint"].as_u64().unwrap_or(0) as u8);
                } else if mode == 1 {
                    push_u16(&mut buffer, value_u16(&entry["dest"]));
                }
            }
        }
        ClusterId::LQI_TABLE_RESPONSE => {
            buffer.push(payload["neighborTableEntries"].as_u64().unwrap_or(0) as u8);
            buffer.push(payload["startIndex"].as_u64().unwrap_or(0) as u8);
            let list = payload["entryList"].as_array().cloned().unwrap_or_default();
            buffer.push(list.len() as u8);
            for entry in list {
                let extended_pan_id = entry["extendedPanId"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                buffer.extend(
                    extended_pan_id
                        .iter()
                        .map(|value| value.as_u64().unwrap_or(0) as u8),
                );
                push_eui64(&mut buffer, &entry["eui64"]);
                push_u16(&mut buffer, value_u16(&entry["nwkAddress"]));
                buffer.push(
                    (entry["deviceType"].as_u64().unwrap_or(0) as u8 & 0x03)
                        | ((entry["rxOnWhenIdle"].as_u64().unwrap_or(0) as u8 & 1) << 2)
                        | ((entry["relationship"].as_u64().unwrap_or(0) as u8 & 0x07) << 4)
                        | ((entry["reserved1"].as_u64().unwrap_or(0) as u8 & 1) << 7),
                );
                buffer.push(
                    (entry["permitJoining"].as_u64().unwrap_or(0) as u8 & 1)
                        | ((entry["reserved2"].as_u64().unwrap_or(0) as u8 & 0x7f) << 1),
                );
                buffer.push(entry["depth"].as_u64().unwrap_or(0) as u8);
                buffer.push(entry["lqi"].as_u64().unwrap_or(0) as u8);
            }
        }
        ClusterId::ROUTING_TABLE_RESPONSE => {
            buffer.push(payload["routingTableEntries"].as_u64().unwrap_or(0) as u8);
            buffer.push(payload["startIndex"].as_u64().unwrap_or(0) as u8);
            let list = payload["entryList"].as_array().cloned().unwrap_or_default();
            buffer.push(list.len() as u8);
            for entry in list {
                push_u16(&mut buffer, value_u16(&entry["destinationAddress"]));
                let status = match entry["status"].as_str().unwrap_or("ACTIVE") {
                    "ACTIVE" => 0u8,
                    "DISCOVERY_UNDERWAY" => 1,
                    "DISCOVERY_FAILED" => 2,
                    "INACTIVE" => 3,
                    _ => 4,
                };
                buffer.push(
                    status
                        | ((entry["memoryConstrained"].as_u64().unwrap_or(0) as u8 & 1) << 3)
                        | ((entry["manyToOne"].as_u64().unwrap_or(0) as u8 & 1) << 4)
                        | ((entry["routeRecordRequired"].as_u64().unwrap_or(0) as u8 & 1) << 5)
                        | ((entry["reserved1"].as_u64().unwrap_or(0) as u8 & 1) << 6),
                );
                push_u16(&mut buffer, value_u16(&entry["nextHopAddress"]));
            }
        }
        _ => {}
    }
    buffer
}
