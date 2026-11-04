/// ZCL attribute/command metadata and value encoders.
///
/// Converters specify writes (`entity.write(cluster, {attribute: value})`) and
/// cluster commands (`entity.command(cluster, command, payload)`) symbolically;
/// encoding them requires the data type of each attribute/parameter. That
/// metadata is published by `scripts/gen-registry.mjs` as `zcl-commands.json`
/// (generated from the pinned herdsman ZCL definition). Unsupported data types
/// are rejected instead of being guessed.
use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ZclTypesFile {
    #[serde(default)]
    pub clusters: HashMap<String, ClusterTypes>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ClusterTypes {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub attributes: HashMap<String, AttributeInfo>,
    #[serde(default)]
    pub commands: HashMap<String, CommandInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AttributeInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommandInfo {
    pub name: String,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: u32,
}

impl ZclTypesFile {
    pub fn from_json(text: &str) -> Result<Self> {
        serde_json::from_str(text)
            .map_err(|error| Error::Config(format!("invalid zcl-commands.json: {error}")))
    }

    #[must_use]
    pub fn cluster(&self, cluster_id: u16) -> Option<&ClusterTypes> {
        self.clusters.get(&cluster_id.to_string())
    }

    #[must_use]
    pub fn cluster_id_by_name(&self, name: &str) -> Option<u16> {
        self.clusters.iter().find_map(|(id, cluster)| {
            (cluster.name == name)
                .then(|| id.parse::<u16>().ok())
                .flatten()
        })
    }

    #[must_use]
    pub fn attribute(&self, cluster_id: u16, attribute_id: u16) -> Option<&AttributeInfo> {
        self.cluster(cluster_id)?
            .attributes
            .get(&attribute_id.to_string())
    }

    #[must_use]
    pub fn attribute_by_name(&self, cluster_id: u16, name: &str) -> Option<u16> {
        self.cluster(cluster_id)?
            .attributes
            .iter()
            .find_map(|(id, info)| {
                (info.name == name)
                    .then(|| id.parse::<u16>().ok())
                    .flatten()
            })
    }

    #[must_use]
    pub fn command(&self, cluster_id: u16, command_id: u8) -> Option<&CommandInfo> {
        self.cluster(cluster_id)?
            .commands
            .get(&command_id.to_string())
    }

    #[must_use]
    pub fn command_id_by_name(&self, cluster_id: u16, name: &str) -> Option<u8> {
        self.cluster(cluster_id)?
            .commands
            .iter()
            .find_map(|(id, info)| (info.name == name).then(|| id.parse::<u8>().ok()).flatten())
    }
}

fn integer(value: &Value, data_type: u32) -> Result<i128> {
    let raw = match value {
        Value::Number(number) => number
            .as_i64()
            .map(i128::from)
            .or_else(|| number.as_u64().map(i128::from)),
        Value::Bool(boolean) => Some(i128::from(u8::from(*boolean))),
        Value::String(text) => text.trim().parse::<i128>().ok(),
        _ => None,
    };
    raw.ok_or_else(|| {
        Error::Zcl(format!(
            "ZCL type 0x{data_type:02X} needs an integer, got {value}"
        ))
    })
}

fn decode_signed(value: &Value, data_type: u32, bits: u32) -> Result<Vec<u8>> {
    let raw = integer(value, data_type)?;
    let min = -(1i128 << (bits - 1));
    let max = (1i128 << (bits - 1)) - 1;
    if raw < min || raw > max {
        return Err(Error::Zcl(format!(
            "value {raw} out of range for ZCL type 0x{data_type:02X} ({bits}-bit signed)"
        )));
    }
    Ok(raw.to_le_bytes()[..(bits as usize) / 8].to_vec())
}

fn decode_unsigned(value: &Value, data_type: u32, bits: u32) -> Result<Vec<u8>> {
    let raw = integer(value, data_type)?;
    if raw < 0 || raw > (1i128 << bits) - 1 {
        return Err(Error::Zcl(format!(
            "value {raw} out of range for ZCL type 0x{data_type:02X} ({bits}-bit unsigned)"
        )));
    }
    Ok(raw.to_le_bytes()[..(bits as usize) / 8].to_vec())
}

fn as_array(value: &Value, data_type: u32) -> Result<Vec<Value>> {
    value.as_array().cloned().ok_or_else(|| {
        Error::Zcl(format!(
            "ZCL type 0x{data_type:02X} needs an array, got {value}"
        ))
    })
}

/// Accept `[1,2,3]`, `"0x0102"` or `"0102"` as raw bytes.
fn as_bytes(value: &Value, data_type: u32) -> Result<Vec<u8>> {
    match value {
        Value::Array(items) => items
            .iter()
            .map(|item| {
                item.as_u64()
                    .filter(|byte| *byte <= u8::MAX as u64)
                    .map(|byte| byte as u8)
                    .ok_or_else(|| {
                        Error::Zcl(format!("byte array contains a non-u8 value: {item}"))
                    })
            })
            .collect(),
        Value::String(text) => {
            let trimmed = text.trim();
            let hex = trimmed
                .strip_prefix("0x")
                .or_else(|| trimmed.strip_prefix("0X"))
                .unwrap_or(trimmed);
            hex::decode(hex).map_err(|error| {
                Error::Zcl(format!(
                    "ZCL type 0x{data_type:02X} is not valid hex: {error}"
                ))
            })
        }
        other => Err(Error::Zcl(format!(
            "ZCL type 0x{data_type:02X} needs a byte array or hex string, got {other}"
        ))),
    }
}

fn optional_u8_field(object: &serde_json::Map<String, Value>, name: &str) -> Result<u8> {
    match object.get(name) {
        None | Some(Value::Null) => Ok(0xFF),
        Some(value) => {
            let raw = integer(value, 0xE0)?;
            if !(0..=0xFF).contains(&raw) {
                return Err(Error::Zcl(format!(
                    "field '{name}' value {raw} does not fit in one byte"
                )));
            }
            Ok(raw as u8)
        }
    }
}

fn encode_array(value: &Value, data_type: u32) -> Result<Vec<u8>> {
    let object = value.as_object().ok_or_else(|| {
        Error::Zcl(format!(
            "ZCL array type 0x{data_type:02X} needs {{elementType, elements}}"
        ))
    })?;
    let element_type = object
        .get("elementType")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| Error::Zcl("array elementType is required".into()))?;
    if element_type > u8::MAX as u64 {
        return Err(Error::Zcl("array elementType must fit in one byte".into()));
    }
    let elements = object
        .get("elements")
        .and_then(|value| value.as_array())
        .ok_or_else(|| Error::Zcl("array elements must be an array".into()))?;
    if elements.len() > u16::MAX as usize {
        return Err(Error::Zcl("array has too many elements".into()));
    }
    let mut out = vec![element_type as u8];
    out.extend_from_slice(&(elements.len() as u16).to_le_bytes());
    for element in elements {
        out.extend_from_slice(&encode_value(element_type as u32, element)?);
    }
    Ok(out)
}

fn encode_struct(value: &Value) -> Result<Vec<u8>> {
    let elements = as_array(value, 0x4C)?;
    if elements.len() > u16::MAX as usize {
        return Err(Error::Zcl("struct has too many elements".into()));
    }
    let mut out = (elements.len() as u16).to_le_bytes().to_vec();
    for element in &elements {
        let object = element
            .as_object()
            .ok_or_else(|| Error::Zcl("struct element must be {elmType, elmVal}".into()))?;
        let elm_type = object
            .get("elmType")
            .and_then(|value| value.as_u64())
            .filter(|value| *value <= u8::MAX as u64)
            .ok_or_else(|| Error::Zcl("struct elmType is required".into()))?;
        out.push(elm_type as u8);
        out.extend_from_slice(&encode_value(
            elm_type as u32,
            object
                .get("elmVal")
                .ok_or_else(|| Error::Zcl("struct elmVal is required".into()))?,
        )?);
    }
    Ok(out)
}

/// Encode one attribute/parameter value for a ZCL data type.
pub fn encode_value(data_type: u32, value: &Value) -> Result<Vec<u8>> {
    match data_type {
        // data8/data16/data24/data32, unsigned integers, enums, bitmaps
        0x08 | 0x20 | 0x30 | 0x18 => decode_unsigned(value, data_type, 8),
        0x09 | 0x21 | 0x31 | 0x19 => decode_unsigned(value, data_type, 16),
        0x0A | 0x22 => decode_unsigned(value, data_type, 24),
        0x0B | 0x23 => decode_unsigned(value, data_type, 32),
        0x10 => match value {
            Value::Bool(boolean) => Ok(vec![u8::from(*boolean)]),
            other => decode_unsigned(other, data_type, 8),
        },
        0x28 => decode_signed(value, data_type, 8),
        0x29 => decode_signed(value, data_type, 16),
        0x2A => decode_signed(value, data_type, 24),
        0x2B => decode_signed(value, data_type, 32),
        0x39 | 0x3A => {
            let number = match value {
                Value::Number(number) => number.as_f64(),
                Value::String(text) => text.trim().parse::<f64>().ok(),
                _ => None,
            }
            .ok_or_else(|| {
                Error::Zcl(format!(
                    "ZCL float type 0x{data_type:02X} needs a number, got {value}"
                ))
            })?;
            if data_type == 0x39 {
                let narrowed = number as f32;
                if !narrowed.is_finite() || (narrowed as f64 - number).abs() > f64::EPSILON {
                    return Err(Error::Zcl(format!(
                        "value {number} is not representable as a 32-bit float"
                    )));
                }
                Ok(narrowed.to_le_bytes().to_vec())
            } else {
                Ok(number.to_le_bytes().to_vec())
            }
        }
        0x41..=0x44 => {
            let text = match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            let bytes = text.as_bytes();
            let mut out = Vec::new();
            if data_type == 0x41 || data_type == 0x42 {
                if bytes.len() > u8::MAX as usize {
                    return Err(Error::Zcl("string exceeds 255 bytes".into()));
                }
                out.push(bytes.len() as u8);
            } else {
                if bytes.len() > u16::MAX as usize {
                    return Err(Error::Zcl("string exceeds 65535 bytes".into()));
                }
                out.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
            }
            out.extend_from_slice(bytes);
            Ok(out)
        }
        // Wider integers/bitmaps/data enums.
        0x1A => decode_unsigned(value, data_type, 24),
        0x1B => decode_unsigned(value, data_type, 32),
        0x0C | 0x1C | 0x24 => decode_unsigned(value, data_type, 40),
        0x0D | 0x1D | 0x25 => decode_unsigned(value, data_type, 48),
        0x0E | 0x1E | 0x26 => decode_unsigned(value, data_type, 56),
        0x0F | 0x1F | 0x27 => decode_unsigned(value, data_type, 64),
        0x2C => decode_signed(value, data_type, 40),
        0x2D => decode_signed(value, data_type, 48),
        0x2E => decode_signed(value, data_type, 56),
        0x2F => decode_signed(value, data_type, 64),
        // Time / identifier types.
        0xE2 | 0xEA => decode_unsigned(value, data_type, 32),
        0xE8 | 0xE9 => decode_unsigned(value, data_type, 16),
        0xF0 => {
            let bytes = as_bytes(value, data_type)?;
            if bytes.len() != 8 {
                return Err(Error::Zcl(format!(
                    "IEEE address needs 8 bytes, got {}",
                    bytes.len()
                )));
            }
            let mut out = bytes;
            out.reverse();
            Ok(out)
        }
        0xF1 => {
            let bytes = as_bytes(value, data_type)?;
            if bytes.len() != 16 {
                return Err(Error::Zcl(format!(
                    "security key needs 16 bytes, got {}",
                    bytes.len()
                )));
            }
            Ok(bytes)
        }
        // Composite ZCL types.
        0x48 | 0x50 | 0x51 => encode_array(value, data_type),
        0x4C => encode_struct(value),
        0xE0 => {
            let object = value
                .as_object()
                .ok_or_else(|| Error::Zcl("time-of-day needs an object".into()))?;
            Ok(vec![
                optional_u8_field(object, "hours")?,
                optional_u8_field(object, "minutes")?,
                optional_u8_field(object, "seconds")?,
                optional_u8_field(object, "hundredths")?,
            ])
        }
        0xE1 => {
            let object = value
                .as_object()
                .ok_or_else(|| Error::Zcl("date needs an object".into()))?;
            let year = match object.get("year") {
                None | Some(Value::Null) => 0xFF,
                Some(value) => {
                    let raw = integer(value, 0xE1)?;
                    if !(1900..=2155).contains(&raw) {
                        return Err(Error::Zcl(format!(
                            "date year {raw} is outside 1900..=2155"
                        )));
                    }
                    (raw - 1900) as u8
                }
            };
            Ok(vec![
                year,
                optional_u8_field(object, "month")?,
                optional_u8_field(object, "dayOfMonth")?,
                optional_u8_field(object, "dayOfWeek")?,
            ])
        }
        // BuffaloZclDataType extras used by cluster commands.
        1001..=1004 => {
            let width = match data_type {
                1001 => 8,
                1002 => 16,
                1003 => 24,
                _ => 32,
            };
            let mut out = Vec::new();
            for element in as_array(value, data_type)? {
                out.extend_from_slice(&decode_unsigned(&element, data_type, width)?);
            }
            Ok(out)
        }
        1005 => {
            let mut out = Vec::new();
            for element in as_array(value, data_type)? {
                let object = element
                    .as_object()
                    .ok_or_else(|| Error::Zcl("zone info entry must be an object".into()))?;
                out.extend_from_slice(&decode_unsigned(
                    object
                        .get("zoneID")
                        .ok_or_else(|| Error::Zcl("zoneID is required".into()))?,
                    data_type,
                    8,
                )?);
                out.extend_from_slice(&decode_unsigned(
                    object
                        .get("zoneStatus")
                        .ok_or_else(|| Error::Zcl("zoneStatus is required".into()))?,
                    data_type,
                    16,
                )?);
            }
            Ok(out)
        }
        1006 => {
            let mut out = Vec::new();
            for element in as_array(value, data_type)? {
                let object = element
                    .as_object()
                    .ok_or_else(|| Error::Zcl("extension field set must be an object".into()))?;
                out.extend_from_slice(&decode_unsigned(
                    object
                        .get("clstId")
                        .ok_or_else(|| Error::Zcl("clstId is required".into()))?,
                    data_type,
                    16,
                )?);
                let ext_field = as_bytes(
                    object
                        .get("extField")
                        .ok_or_else(|| Error::Zcl("extField is required".into()))?,
                    data_type,
                )?;
                if ext_field.len() > u8::MAX as usize {
                    return Err(Error::Zcl("extField exceeds 255 bytes".into()));
                }
                out.push(ext_field.len() as u8);
                out.extend_from_slice(&ext_field);
            }
            Ok(out)
        }
        1008 => as_bytes(value, data_type),
        1010 => {
            let object = value.as_object().ok_or_else(|| {
                Error::Zcl("structured selector needs {indicatorType, indexes}".into())
            })?;
            let indicator_type = object
                .get("indicatorType")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            let indexes = object
                .get("indexes")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default();
            if indexes.len() > 0x0F {
                return Err(Error::Zcl(
                    "structured selector supports at most 15 indexes".into(),
                ));
            }
            let indicator = indicator_type | indexes.len() as u64;
            let mut out = vec![indicator as u8];
            for index in indexes {
                out.extend_from_slice(&decode_unsigned(&index, data_type, 16)?);
            }
            Ok(out)
        }
        1013 => {
            let raw = integer(value, data_type)?;
            if !(0..=(1i128 << 24) - 1).contains(&raw) {
                return Err(Error::Zcl(format!(
                    "big-endian uint24 value {raw} is out of range"
                )));
            }
            let bytes = raw.to_le_bytes();
            Ok(vec![bytes[2], bytes[1], bytes[0]])
        }
        // Explicitly unsupported Buffalo payloads: they need per-command context.
        1000 | 1007 | 1009 | 1011 | 1012 | 1014 => Err(Error::Zcl(format!(
            "ZCL type 0x{data_type:04X} needs command context and is not supported"
        ))),
        other => Err(Error::Zcl(format!(
            "unsupported ZCL data type 0x{other:02X} for converter output"
        ))),
    }
}

/// Encode command parameters in the cluster definition's declared order.
pub fn encode_command_parameters(command: &CommandInfo, payload: &Value) -> Result<Vec<u8>> {
    let empty = serde_json::Map::new();
    let object = match payload {
        Value::Null => &empty,
        Value::Object(object) => object,
        Value::Array(_) => {
            return Err(Error::Zcl(
                "command payload must be an object keyed by parameter name".into(),
            ))
        }
        other => {
            return Err(Error::Zcl(format!(
                "command payload must be an object, got {other}"
            )))
        }
    };
    let mut bytes = Vec::new();
    for parameter in &command.parameters {
        let value = object.get(&parameter.name).ok_or_else(|| {
            Error::Zcl(format!(
                "command parameter '{}' is missing from the converter payload",
                parameter.name
            ))
        })?;
        bytes.extend_from_slice(&encode_value(parameter.data_type, value)?);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TYPES: &str = r#"{
      "clusters": {
        "6": {
          "name": "genOnOff",
          "attributes": {
            "0": {"name": "onOff", "type": 16},
            "16385": {"name": "onTime", "type": 33},
            "16386": {"name": "offWaitTime", "type": 33},
            "16387": {"name": "startUpOnOff", "type": 48}
          },
          "commands": {
            "0": {"name": "off", "parameters": []},
            "1": {"name": "on", "parameters": []},
            "66": {"name": "onWithTimedOff", "parameters": [
              {"name": "ctrlbits", "type": 32},
              {"name": "ontime", "type": 33},
              {"name": "offwaittime", "type": 33}
            ]}
          }
        }
      }
    }"#;

    #[test]
    fn resolves_clusters_attributes_and_commands() {
        let types = ZclTypesFile::from_json(TYPES).unwrap();
        assert_eq!(types.cluster_id_by_name("genOnOff"), Some(6));
        assert_eq!(types.attribute(6, 0).unwrap().name, "onOff");
        assert_eq!(types.attribute_by_name(6, "onTime"), Some(0x4001));
        assert_eq!(types.command_id_by_name(6, "on"), Some(1));
        assert_eq!(types.command(6, 66).unwrap().parameters.len(), 3);
    }

    #[test]
    fn encodes_primitives_with_range_checks() {
        assert_eq!(encode_value(0x10, &Value::Bool(true)).unwrap(), vec![1]);
        assert_eq!(encode_value(0x20, &Value::from(200)).unwrap(), vec![200]);
        assert_eq!(
            encode_value(0x21, &Value::from(0x1A62)).unwrap(),
            vec![0x62, 0x1A]
        );
        assert_eq!(
            encode_value(0x23, &Value::from(0x01020304)).unwrap(),
            vec![4, 3, 2, 1]
        );
        assert_eq!(
            encode_value(0x29, &Value::from(-2)).unwrap(),
            vec![0xFE, 0xFF]
        );
        assert_eq!(encode_value(0x30, &Value::from(1)).unwrap(), vec![1]);
        assert!(encode_value(0x20, &Value::from(256)).is_err());
        assert!(encode_value(0x20, &Value::from(-1)).is_err());
        assert_eq!(
            encode_value(0x42, &Value::from("hi")).unwrap(),
            vec![2, b'h', b'i']
        );
        assert!(encode_value(0x99, &Value::from(1)).is_err());
    }

    #[test]
    fn encodes_command_parameters_in_declared_order() {
        let types = ZclTypesFile::from_json(TYPES).unwrap();
        let command = types.command(6, 66).unwrap();
        let bytes = encode_command_parameters(
            command,
            &serde_json::json!({"offwaittime": 5, "ctrlbits": 0, "ontime": 10}),
        )
        .unwrap();
        assert_eq!(bytes, vec![0, 10, 0, 5, 0]);
        let missing =
            encode_command_parameters(command, &serde_json::json!({"ctrlbits": 0})).unwrap_err();
        assert!(missing.to_string().contains("ontime"));
    }

    #[test]
    fn encodes_float_and_long_strings() {
        assert_eq!(
            encode_value(0x39, &serde_json::json!(1.5)).unwrap(),
            1.5f32.to_le_bytes().to_vec()
        );
        assert_eq!(
            encode_value(0x3A, &serde_json::json!(1.5)).unwrap(),
            1.5f64.to_le_bytes().to_vec()
        );
        assert_eq!(
            encode_value(0x44, &Value::from("ab")).unwrap(),
            vec![2, 0, b'a', b'b']
        );
    }

    #[test]
    fn encodes_arrays_structs_and_time_types() {
        assert_eq!(
            encode_value(
                0x48,
                &serde_json::json!({"elementType": 0x20, "elements": [1, 2]})
            )
            .unwrap(),
            vec![0x20, 0x02, 0x00, 1, 2]
        );
        assert_eq!(
            encode_value(0x4C, &serde_json::json!([{"elmType": 0x21, "elmVal": 258}])).unwrap(),
            vec![1, 0, 0x21, 0x02, 0x01]
        );
        assert_eq!(
            encode_value(
                0xE0,
                &serde_json::json!({"hours": 1, "minutes": 2, "seconds": 3, "hundredths": 4})
            )
            .unwrap(),
            vec![1, 2, 3, 4]
        );
        assert_eq!(
            encode_value(
                0xE1,
                &serde_json::json!({"year": 2024, "month": 5, "dayOfMonth": 6, "dayOfWeek": 2})
            )
            .unwrap(),
            vec![124, 5, 6, 2]
        );
        assert_eq!(
            encode_value(0xF0, &Value::from("0x0011223344556677")).unwrap(),
            vec![0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, 0x00]
        );
        assert_eq!(
            encode_value(0xE2, &Value::from(0x01020304)).unwrap(),
            vec![4, 3, 2, 1]
        );
        assert_eq!(
            encode_value(0x2F, &Value::from(-2)).unwrap(),
            (-2i64).to_le_bytes().to_vec()
        );
    }

    #[test]
    fn encodes_buffalo_lists_selectors_and_buffers() {
        assert_eq!(
            encode_value(1002, &serde_json::json!([1, 258])).unwrap(),
            vec![1, 0, 2, 1]
        );
        assert_eq!(
            encode_value(
                1005,
                &serde_json::json!([{"zoneID": 1, "zoneStatus": 0x1234}])
            )
            .unwrap(),
            vec![1, 0x34, 0x12]
        );
        assert_eq!(
            encode_value(1008, &Value::from("0x0aff")).unwrap(),
            vec![0x0A, 0xFF]
        );
        assert_eq!(
            encode_value(1010, &serde_json::json!({"indexes": [1, 2]})).unwrap(),
            vec![2, 1, 0, 2, 0]
        );
        assert_eq!(
            encode_value(1013, &Value::from(0x010203)).unwrap(),
            vec![1, 2, 3]
        );
        assert_eq!(
            encode_value(
                1006,
                &serde_json::json!([{"clstId": 6, "extField": [1, 2]}])
            )
            .unwrap(),
            vec![6, 0, 2, 1, 2]
        );
    }

    #[test]
    fn rejects_context_dependent_buffalo_types() {
        for data_type in [1000u32, 1007, 1009, 1011, 1012, 1014] {
            let error = encode_value(data_type, &Value::from(1)).unwrap_err();
            assert!(
                error.to_string().contains("command context"),
                "type {data_type} should be refused explicitly, got {error}"
            );
        }
    }

    #[test]
    fn array_and_fixed_buffer_validation() {
        let error = encode_value(0x48, &serde_json::json!({"elements": [1]})).unwrap_err();
        assert!(error.to_string().contains("elementType"));
        let error = encode_value(
            0x48,
            &serde_json::json!({"elementType": 999, "elements": []}),
        )
        .unwrap_err();
        assert!(error.to_string().contains("one byte"));
        let error = encode_value(0xF0, &Value::from("0x0011")).unwrap_err();
        assert!(error.to_string().contains("8 bytes"));
        let error = encode_value(0xF1, &Value::from("0x00")).unwrap_err();
        assert!(error.to_string().contains("16 bytes"));
    }
}
