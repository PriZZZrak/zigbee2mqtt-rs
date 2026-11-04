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
    pub data_type: u8,
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
    pub data_type: u8,
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

fn integer(value: &Value, data_type: u8) -> Result<i128> {
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

fn decode_signed(value: &Value, data_type: u8, bits: u32) -> Result<Vec<u8>> {
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

fn decode_unsigned(value: &Value, data_type: u8, bits: u32) -> Result<Vec<u8>> {
    let raw = integer(value, data_type)?;
    if raw < 0 || raw > (1i128 << bits) - 1 {
        return Err(Error::Zcl(format!(
            "value {raw} out of range for ZCL type 0x{data_type:02X} ({bits}-bit unsigned)"
        )));
    }
    Ok(raw.to_le_bytes()[..(bits as usize) / 8].to_vec())
}

/// Encode one attribute/parameter value for a ZCL data type.
pub fn encode_value(data_type: u8, value: &Value) -> Result<Vec<u8>> {
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
}
