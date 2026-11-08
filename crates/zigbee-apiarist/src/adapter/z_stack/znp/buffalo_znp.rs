// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/znp/buffaloZnp.ts

pub struct BuffaloZnp {
    pub position: usize,
    pub buffer: Vec<u8>,
}

impl BuffaloZnp {
    pub fn new(buffer: Vec<u8>, position: usize) -> Self {
        Self { position, buffer }
    }

    pub fn get_position(&self) -> usize {
        self.position
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        self.buffer.clone()
    }

    pub fn get_written(&self) -> Vec<u8> {
        self.buffer[..self.position.min(self.buffer.len())].to_vec()
    }

    /// Upstream: `read(type, options)`.
    pub fn read(&mut self, parameter_type: u32, options: &serde_json::Value) -> serde_json::Value {
        use crate::adapter::z_stack::znp::parameter_type::ParameterType;
        let length = options
            .get("length")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as usize;
        match parameter_type {
            value if value == ParameterType::UINT8 as u32 => serde_json::json!(self.read_uint(1)),
            value if value == ParameterType::UINT16 as u32 => serde_json::json!(self.read_uint(2)),
            value if value == ParameterType::UINT32 as u32 => serde_json::json!(self.read_uint(4)),
            value if value == ParameterType::INT8 as u32 => {
                serde_json::json!(self.read_uint(1) as u8 as i8)
            }
            value if value == ParameterType::IEEEADDR as u32 => {
                let bytes = self.read_bytes(8);
                let mut reversed = bytes;
                reversed.reverse();
                serde_json::json!(format!(
                    "0x{}",
                    reversed
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>()
                ))
            }
            value if value == ParameterType::BUFFER as u32 => {
                serde_json::json!(self.read_bytes(length))
            }
            value if value == ParameterType::BUFFER8 as u32 => {
                serde_json::json!(self.read_bytes(8))
            }
            value if value == ParameterType::BUFFER16 as u32 => {
                serde_json::json!(self.read_bytes(16))
            }
            value if value == ParameterType::BUFFER18 as u32 => {
                serde_json::json!(self.read_bytes(18))
            }
            value if value == ParameterType::BUFFER32 as u32 => {
                serde_json::json!(self.read_bytes(32))
            }
            value if value == ParameterType::BUFFER42 as u32 => {
                serde_json::json!(self.read_bytes(42))
            }
            value if value == ParameterType::BUFFER100 as u32 => {
                serde_json::json!(self.read_bytes(100))
            }
            value if value == ParameterType::LIST_UINT8 as u32 => {
                serde_json::json!((0..length).map(|_| self.read_uint(1)).collect::<Vec<_>>())
            }
            value if value == ParameterType::LIST_UINT16 as u32 => {
                serde_json::json!((0..length).map(|_| self.read_uint(2)).collect::<Vec<_>>())
            }
            value if value == ParameterType::LIST_NETWORK as u32 => {
                let mut entries = Vec::new();
                for _ in 0..length {
                    let pan = self.read_uint(2);
                    let channel = self.read_uint(1);
                    let order = self.read_uint(1) as u8;
                    let profile = self.read_uint(1) as u8;
                    let permit = self.read_uint(1);
                    entries.push(serde_json::json!({
                        "beaconOrder": (order >> 4) & 0x0f,
                        "logicalChannel": channel,
                        "neightborPanId": pan,
                        "permitJoin": permit,
                        "stackProfile": (profile >> 4) & 0x0f,
                        "superFrameOrder": order & 0x0f,
                        "zigbeeVersion": profile & 0x0f,
                    }));
                }
                serde_json::Value::Array(entries)
            }
            _ => panic!("Unsupported parameter type {parameter_type}"),
        }
    }

    /// Upstream: `write(type, value, options)`.
    pub fn write(
        &mut self,
        parameter_type: u32,
        value: &serde_json::Value,
        _options: &serde_json::Value,
    ) {
        use crate::adapter::z_stack::znp::parameter_type::ParameterType;
        let bytes = |value: &serde_json::Value| -> Vec<u8> {
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
                self.write_uint(value.as_u64().unwrap_or(0), 1)
            }
            v if v == ParameterType::UINT16 as u32 => {
                self.write_uint(value.as_u64().unwrap_or(0), 2)
            }
            v if v == ParameterType::UINT32 as u32 => {
                self.write_uint(value.as_u64().unwrap_or(0), 4)
            }
            v if v == ParameterType::INT8 as u32 => {
                self.write_uint(value.as_i64().unwrap_or(0) as u64, 1)
            }
            v if v == ParameterType::IEEEADDR as u32 => {
                let hex = value.as_str().unwrap_or("0x0").trim_start_matches("0x");
                let padded = format!("{hex:0>16}");
                let mut output = Vec::new();
                let mut index = padded.len();
                while index >= 2 {
                    output.push(u8::from_str_radix(&padded[index - 2..index], 16).unwrap_or(0));
                    index -= 2;
                }
                self.write_bytes(&output);
            }
            v if v == ParameterType::BUFFER as u32 => self.write_bytes(&bytes(value)),
            v if v == ParameterType::BUFFER8 as u32
                || v == ParameterType::BUFFER16 as u32
                || v == ParameterType::BUFFER18 as u32
                || v == ParameterType::BUFFER32 as u32
                || v == ParameterType::BUFFER42 as u32
                || v == ParameterType::BUFFER100 as u32 =>
            {
                let expected = match v {
                    x if x == ParameterType::BUFFER8 as u32 => 8,
                    x if x == ParameterType::BUFFER16 as u32 => 16,
                    x if x == ParameterType::BUFFER18 as u32 => 18,
                    x if x == ParameterType::BUFFER32 as u32 => 32,
                    x if x == ParameterType::BUFFER42 as u32 => 42,
                    _ => 100,
                };
                let data = bytes(value);
                assert_eq!(data.len(), expected, "Invalid buffer length");
                self.write_bytes(&data);
            }
            v if v == ParameterType::LIST_UINT8 as u32 => {
                for item in value.as_array().cloned().unwrap_or_default() {
                    self.write_uint(item.as_u64().unwrap_or(0), 1);
                }
            }
            v if v == ParameterType::LIST_UINT16 as u32 => {
                for item in value.as_array().cloned().unwrap_or_default() {
                    self.write_uint(item.as_u64().unwrap_or(0), 2);
                }
            }
            v if v == ParameterType::LIST_NETWORK as u32 => {
                let items = value.as_array().cloned().unwrap_or_default();
                assert!(!items.is_empty(), "LIST_NETWORK cannot be empty");
                for item in items {
                    let order = ((item["beaconOrder"].as_u64().unwrap_or(0) & 0x0f) << 4)
                        | (item["superFrameOrder"].as_u64().unwrap_or(0) & 0x0f);
                    let profile = ((item["stackProfile"].as_u64().unwrap_or(0) & 0x0f) << 4)
                        | (item["zigbeeVersion"].as_u64().unwrap_or(0) & 0x0f);
                    self.write_uint(item["neightborPanId"].as_u64().unwrap_or(0), 2);
                    self.write_uint(item["logicalChannel"].as_u64().unwrap_or(0), 1);
                    self.write_uint(order as u64, 1);
                    self.write_uint(profile as u64, 1);
                    self.write_uint(item["permitJoin"].as_u64().unwrap_or(0), 1);
                }
            }
            _ => panic!("Unsupported parameter type {parameter_type}"),
        }
    }

    fn read_bytes(&mut self, length: usize) -> Vec<u8> {
        let end = self.position + length;
        if end > self.buffer.len() {
            panic!("Not enough bytes to read");
        }
        let values = self.buffer[self.position..end].to_vec();
        self.position = end;
        values
    }

    fn read_uint(&mut self, length: usize) -> u64 {
        let bytes = self.read_bytes(length);
        let mut value = 0u64;
        for (index, byte) in bytes.iter().enumerate() {
            value |= u64::from(*byte) << (index * 8);
        }
        value
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        if self.position + bytes.len() > self.buffer.len() {
            self.buffer.resize(self.position + bytes.len(), 0);
        }
        self.buffer[self.position..self.position + bytes.len()].copy_from_slice(bytes);
        self.position += bytes.len();
    }

    fn write_uint(&mut self, value: u64, length: usize) {
        let bytes = value.to_le_bytes();
        self.write_bytes(&bytes[..length]);
    }
}
