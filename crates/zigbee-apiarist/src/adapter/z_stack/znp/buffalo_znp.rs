// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/znp/buffaloZnp.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).
//
// Upstream `BuffaloZnp extends Buffalo`. The Rust port composes `Buffalo` so the
// primitive codec is implemented once and reused by ZNP/ZCL/ZDO.

use serde_json::{json, Value};

use crate::adapter::z_stack::znp::parameter_type::ParameterType;
use crate::buffalo::buffalo::Buffalo;

/// Upstream `BuffaloZnpOptions`.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuffaloZnpOptions {
    pub length: Option<usize>,
}

impl BuffaloZnpOptions {
    fn from_value(options: &Value) -> Self {
        Self {
            length: options
                .get("length")
                .and_then(Value::as_u64)
                .map(|length| length as usize),
        }
    }
}

/// Upstream `Network` entry returned by `readListNetwork`.
fn network_entry(buffalo: &mut Buffalo) -> Value {
    let neightbor_pan_id = buffalo.read_uint16();
    let logical_channel = buffalo.read_uint8();
    let value1 = buffalo.read_uint8() as u8;
    let value2 = buffalo.read_uint8() as u8;
    let permit_join = buffalo.read_uint8();

    json!({
        "neightborPanId": neightbor_pan_id,
        "logicalChannel": logical_channel,
        "stackProfile": value1 & 0x0f,
        "zigbeeVersion": (value1 & 0xf0) >> 4,
        "beaconOrder": value2 & 0x0f,
        "superFrameOrder": (value2 & 0xf0) >> 4,
        "permitJoin": permit_join,
    })
}

/// Upstream `BuffaloZnp`.
pub struct BuffaloZnp {
    buffalo: Buffalo,
}

impl BuffaloZnp {
    pub fn new(buffer: Vec<u8>, position: usize) -> Self {
        Self {
            buffalo: Buffalo::new(buffer, position),
        }
    }

    pub fn get_position(&self) -> usize {
        self.buffalo.get_position()
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        self.buffalo.get_buffer()
    }

    pub fn get_written(&self) -> Vec<u8> {
        self.buffalo.get_written()
    }

    /// Upstream `read(type, options)`.
    pub fn read(&mut self, parameter_type: u32, options: &Value) -> Value {
        let options = BuffaloZnpOptions::from_value(options);
        match parameter_type {
            value if value == ParameterType::UINT8 as u32 => json!(self.buffalo.read_uint8()),
            value if value == ParameterType::UINT16 as u32 => json!(self.buffalo.read_uint16()),
            value if value == ParameterType::UINT32 as u32 => json!(self.buffalo.read_uint32()),
            value if value == ParameterType::IEEEADDR as u32 => {
                json!(self.buffalo.read_ieee_addr())
            }
            value if value == ParameterType::BUFFER as u32 => {
                let length = options.length.unwrap_or_else(|| {
                    panic!("Cannot read BUFFER without length option specified")
                });
                json!(self.buffalo.read_buffer(length))
            }
            value if value == ParameterType::BUFFER8 as u32 => {
                json!(self.buffalo.read_buffer(8))
            }
            value if value == ParameterType::BUFFER16 as u32 => {
                json!(self.buffalo.read_buffer(16))
            }
            value if value == ParameterType::BUFFER18 as u32 => {
                json!(self.buffalo.read_buffer(18))
            }
            value if value == ParameterType::BUFFER32 as u32 => {
                json!(self.buffalo.read_buffer(32))
            }
            value if value == ParameterType::BUFFER42 as u32 => {
                json!(self.buffalo.read_buffer(42))
            }
            value if value == ParameterType::BUFFER100 as u32 => {
                json!(self.buffalo.read_buffer(100))
            }
            value if value == ParameterType::LIST_UINT8 as u32 => {
                let length = options.length.unwrap_or_else(|| {
                    panic!("Cannot read LIST_UINT8 without length option specified")
                });
                json!(self.buffalo.read_list_uint8(length))
            }
            value if value == ParameterType::LIST_UINT16 as u32 => {
                let length = options.length.unwrap_or_else(|| {
                    panic!("Cannot read LIST_UINT16 without length option specified")
                });
                json!(self.buffalo.read_list_uint16(length))
            }
            value if value == ParameterType::LIST_NETWORK as u32 => {
                let length = options.length.unwrap_or_else(|| {
                    panic!("Cannot read LIST_NETWORK without length option specified")
                });
                let entries = (0..length)
                    .map(|_| network_entry(&mut self.buffalo))
                    .collect::<Vec<_>>();
                Value::Array(entries)
            }
            value if value == ParameterType::INT8 as u32 => json!(self.buffalo.read_int8()),
            _ => panic!("Read for '{parameter_type}' not available"),
        }
    }

    /// Upstream `write(type, value, options)`.
    pub fn write(&mut self, parameter_type: u32, value: &Value, options: &Value) {
        let options = BuffaloZnpOptions::from_value(options);
        let bytes = |value: &Value| -> Vec<u8> {
            value
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .map(|item| item.as_u64().unwrap_or(0) as u8)
                        .collect()
                })
                .unwrap_or_default()
        };

        match parameter_type {
            v if v == ParameterType::UINT8 as u32 => {
                self.buffalo.write_uint8(value.as_u64().unwrap_or(0))
            }
            v if v == ParameterType::UINT16 as u32 => {
                self.buffalo.write_uint16(value.as_u64().unwrap_or(0))
            }
            v if v == ParameterType::UINT32 as u32 => {
                self.buffalo.write_uint32(value.as_u64().unwrap_or(0))
            }
            v if v == ParameterType::IEEEADDR as u32 => self
                .buffalo
                .write_ieee_addr(value.as_str().unwrap_or("0x0")),
            v if v == ParameterType::BUFFER as u32 => {
                let data = bytes(value);
                let length = options.length.unwrap_or(data.len());
                self.buffalo.write_buffer(&data, length);
            }
            v if v == ParameterType::BUFFER8 as u32 => self.buffalo.write_buffer(&bytes(value), 8),
            v if v == ParameterType::BUFFER16 as u32 => {
                self.buffalo.write_buffer(&bytes(value), 16)
            }
            v if v == ParameterType::BUFFER18 as u32 => {
                self.buffalo.write_buffer(&bytes(value), 18)
            }
            v if v == ParameterType::BUFFER32 as u32 => {
                self.buffalo.write_buffer(&bytes(value), 32)
            }
            v if v == ParameterType::BUFFER42 as u32 => {
                self.buffalo.write_buffer(&bytes(value), 42)
            }
            v if v == ParameterType::BUFFER100 as u32 => {
                self.buffalo.write_buffer(&bytes(value), 100)
            }
            v if v == ParameterType::LIST_UINT8 as u32 => {
                let values = value
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .map(|item| item.as_i64().unwrap_or(0))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                self.buffalo.write_list_uint8(&values);
            }
            v if v == ParameterType::LIST_UINT16 as u32 => {
                let values = value
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .map(|item| item.as_i64().unwrap_or(0))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                self.buffalo.write_list_uint16(&values);
            }
            v if v == ParameterType::INT8 as u32 => {
                self.buffalo.write_int8(value.as_i64().unwrap_or(0))
            }
            _ => panic!("Write for '{parameter_type}' not available"),
        }
    }
}
