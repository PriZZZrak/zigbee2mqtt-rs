//! ZCL wire encoding for converter-requested operations.
//!
//! Shared by the bridge (which sends the frames) and the EVID-04 harness
//! (which compares them with pins from the upstream herdsman encoder).

use crate::converters::zcl_types::{encode_command_parameters, encode_value, ZclTypesFile};
use crate::zigbee::zcl::frame::{read_attributes_payload_with_options, ZclFrameHeader};
use crate::zigbee::zcl::write::write_attributes_payload_with_options;
use serde_json::Value;

/// Encode attributes for a Write Attributes request.
pub fn encode_write_attributes(
    types: &ZclTypesFile,
    cluster: u16,
    attributes: &[(String, Value)],
) -> Result<Vec<(u16, u8, Vec<u8>)>, String> {
    let mut out = Vec::new();
    for (name, value) in attributes {
        let attribute_id = name
            .parse::<u16>()
            .ok()
            .or_else(|| types.attribute_by_name(cluster, name))
            .ok_or_else(|| format!("unknown attribute '{name}' for cluster 0x{cluster:04X}"))?;
        let data_type = types
            .attribute(cluster, attribute_id)
            .map(|info| info.data_type)
            .ok_or_else(|| {
                format!(
                    "no ZCL type metadata for attribute 0x{attribute_id:04X} (cluster 0x{cluster:04X})"
                )
            })?;
        let attribute_type = u8::try_from(data_type).map_err(|_| {
            format!(
                "attribute 0x{attribute_id:04X} has non-ZCL type {data_type} (cluster 0x{cluster:04X})"
            )
        })?;
        let bytes = encode_value(data_type, value).map_err(|error| error.to_string())?;
        out.push((attribute_id, attribute_type, bytes));
    }
    Ok(out)
}

/// Encode cluster-specific command parameters in the definition's order.
pub fn encode_command_op(
    types: &ZclTypesFile,
    cluster: u16,
    command: u8,
    payload: &Value,
) -> Result<Vec<u8>, String> {
    let info = types.command(cluster, command).ok_or_else(|| {
        format!("no ZCL command metadata for command 0x{command:02X} (cluster 0x{cluster:04X})")
    })?;
    encode_command_parameters(info, payload).map_err(|error| error.to_string())
}

/// Build a cluster-specific ZCL frame (header + encoded command parameters).
pub fn command_frame(
    tsn: u8,
    command: u8,
    manufacturer_code: Option<u16>,
    disable_default_response: bool,
    parameters: &[u8],
) -> Vec<u8> {
    let mut control = 0x01u8; // cluster-specific, client -> server
    if manufacturer_code.is_some() {
        control |= 0x04;
    }
    if disable_default_response {
        control |= 0x10;
    }
    let mut frame = vec![control];
    if let Some(code) = manufacturer_code {
        frame.extend_from_slice(&code.to_le_bytes());
    }
    frame.push(tsn);
    frame.push(command);
    frame.extend_from_slice(parameters);
    frame
}

/// Build a full cluster command frame from metadata and a converter payload.
pub fn encode_command_frame(
    types: &ZclTypesFile,
    tsn: u8,
    cluster: u16,
    command: u8,
    manufacturer_code: Option<u16>,
    disable_default_response: bool,
    payload: &Value,
) -> Result<Vec<u8>, String> {
    let parameters = encode_command_op(types, cluster, command, payload)?;
    Ok(command_frame(
        tsn,
        command,
        manufacturer_code,
        disable_default_response,
        &parameters,
    ))
}

/// Build a Write Attributes frame.
pub fn encode_write_frame(
    types: &ZclTypesFile,
    tsn: u8,
    cluster: u16,
    manufacturer_code: Option<u16>,
    disable_default_response: bool,
    attributes: &[(String, Value)],
) -> Result<Vec<u8>, String> {
    let encoded = encode_write_attributes(types, cluster, attributes)?;
    let refs: Vec<(u16, u8, &[u8])> = encoded
        .iter()
        .map(|(id, data_type, bytes)| (*id, *data_type, bytes.as_slice()))
        .collect();
    Ok(write_attributes_payload_with_options(
        tsn,
        manufacturer_code,
        disable_default_response,
        &refs,
    ))
}

/// Build a Read Attributes frame.
pub fn encode_read_frame(
    types: &ZclTypesFile,
    tsn: u8,
    cluster: u16,
    manufacturer_code: Option<u16>,
    disable_default_response: bool,
    attributes: &[String],
) -> Result<Vec<u8>, String> {
    let mut ids = Vec::new();
    for name in attributes {
        let id = name
            .parse::<u16>()
            .ok()
            .or_else(|| types.attribute_by_name(cluster, name))
            .ok_or_else(|| format!("unknown attribute '{name}' for cluster 0x{cluster:04X}"))?;
        ids.push(id);
    }
    if ids.is_empty() {
        return Err("read request has no attributes".into());
    }
    Ok(read_attributes_payload_with_options(
        tsn,
        manufacturer_code,
        disable_default_response,
        &ids,
    ))
}

/// Global frame control used by the timestamp helpers (exposed for tests).
#[must_use]
pub fn global_frame(sequence: u8, command: u8) -> Vec<u8> {
    ZclFrameHeader::encode_global(sequence, command)
}
