// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/struct.ts
//
// Buffer-backed struct used by Z-Stack NV entries. Supports the member types
// needed by the ported entries: uint8/16/32, byte arrays (normal/reversed),
// child structs and explicit padding.

use std::collections::BTreeMap;

use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Unaligned,
    Aligned,
}

impl Alignment {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unaligned => "unaligned",
            Self::Aligned => "aligned",
        }
    }
}

#[derive(Debug, Clone)]
pub enum MemberType {
    Uint8,
    Uint16,
    Uint32,
    Uint8Array(usize),
    Uint8ArrayReversed(usize),
    Child,
    Padding,
}

#[derive(Debug, Clone)]
pub struct Member {
    pub key: String,
    pub kind: MemberType,
}

#[derive(Debug, Clone, Default)]
pub struct Struct {
    members: Vec<Member>,
    /// Raw unaligned buffer.
    buffer: Vec<u8>,
    values: BTreeMap<String, Value>,
    children: BTreeMap<String, Box<Struct>>,
    padding_byte: u8,
}

impl Struct {
    pub fn new() -> Self {
        Self {
            padding_byte: 0x00,
            ..Self::default()
        }
    }

    pub fn member(mut self, kind: MemberType, name: &str) -> Self {
        self.members.push(Member {
            key: name.to_string(),
            kind,
        });
        self
    }

    pub fn child(mut self, name: &str, child: Struct) -> Self {
        self.children.insert(name.to_string(), Box::new(child));
        self.members.push(Member {
            key: name.to_string(),
            kind: MemberType::Child,
        });
        self
    }

    pub fn padding(mut self, byte: u8) -> Self {
        self.padding_byte = byte;
        self
    }

    fn unaligned_length(&self) -> usize {
        self.members
            .iter()
            .map(|member| match member.kind {
                MemberType::Uint8 | MemberType::Padding => 1,
                MemberType::Uint16 => 2,
                MemberType::Uint32 => 4,
                MemberType::Uint8Array(length) | MemberType::Uint8ArrayReversed(length) => length,
                MemberType::Child => self
                    .children
                    .get(&member.key)
                    .map(|child| child.unaligned_length())
                    .unwrap_or(0),
            })
            .sum()
    }

    /// Upstream `Struct.build(data)`.
    pub fn build(mut self, data: Option<&[u8]>, defaults: Option<&[u8]>) -> Self {
        let length = self.unaligned_length();
        let mut buffer = match data {
            Some(data) => {
                let mut buffer = data.to_vec();
                buffer.resize(length, 0);
                buffer
            }
            None => match defaults {
                Some(defaults) => {
                    let mut buffer = defaults.to_vec();
                    buffer.resize(length, self.padding_byte);
                    buffer
                }
                None => vec![self.padding_byte; length],
            },
        };
        // Copy child structs into their slots and decode values.
        let mut offset = 0;
        for member in self.members.clone() {
            match member.kind {
                MemberType::Uint8 => {
                    self.values
                        .insert(member.key.clone(), json!(buffer[offset]));
                    offset += 1;
                }
                MemberType::Uint16 => {
                    let value = u16::from_le_bytes([buffer[offset], buffer[offset + 1]]);
                    self.values.insert(member.key.clone(), json!(value));
                    offset += 2;
                }
                MemberType::Uint32 => {
                    let bytes = [
                        buffer[offset],
                        buffer[offset + 1],
                        buffer[offset + 2],
                        buffer[offset + 3],
                    ];
                    self.values
                        .insert(member.key.clone(), json!(u32::from_le_bytes(bytes)));
                    offset += 4;
                }
                MemberType::Uint8Array(length) => {
                    let bytes = buffer[offset..offset + length].to_vec();
                    self.values.insert(member.key.clone(), json!(bytes));
                    offset += length;
                }
                MemberType::Uint8ArrayReversed(length) => {
                    let mut bytes = buffer[offset..offset + length].to_vec();
                    bytes.reverse();
                    self.values.insert(member.key.clone(), json!(bytes));
                    offset += length;
                }
                MemberType::Child => {
                    if let Some(child) = self.children.get_mut(&member.key) {
                        let length = child.unaligned_length();
                        let bytes = buffer[offset..offset + length].to_vec();
                        let built = std::mem::take(child).build(Some(&bytes), None);
                        *child = Box::new(built);
                        self.values.insert(member.key.clone(), child.to_json());
                        offset += length;
                    }
                }
                MemberType::Padding => offset += 1,
            }
        }
        self.buffer = buffer;
        self
    }

    pub fn get_length(
        &self,
        alignment: Alignment,
        pad_length: bool,
        parent_offset: usize,
    ) -> usize {
        match alignment {
            Alignment::Unaligned => self.unaligned_length(),
            Alignment::Aligned => {
                let mut offset = 0usize;
                for member in &self.members {
                    match member.kind {
                        MemberType::Uint8 | MemberType::Padding => offset += 1,
                        MemberType::Uint16 => {
                            offset += ((parent_offset + offset) % 2) + 2;
                        }
                        MemberType::Uint32 => {
                            offset += ((parent_offset + offset) % 2) + 4;
                        }
                        MemberType::Uint8Array(length) | MemberType::Uint8ArrayReversed(length) => {
                            offset += length
                        }
                        MemberType::Child => {
                            offset += self
                                .children
                                .get(&member.key)
                                .map(|child| child.get_length(alignment, false, parent_offset))
                                .unwrap_or(0)
                        }
                    }
                }
                if pad_length {
                    offset += offset % 2;
                }
                offset
            }
        }
    }

    /// Upstream `Struct.serialize(alignment, padLength, parentOffset)`.
    pub fn serialize(&self, alignment: Alignment, pad_length: bool) -> Vec<u8> {
        match alignment {
            Alignment::Unaligned => self.buffer.clone(),
            Alignment::Aligned => {
                let mut aligned =
                    vec![self.padding_byte; self.get_length(alignment, pad_length, 0)];
                let mut source = 0usize;
                let mut target = 0usize;
                for member in &self.members {
                    let length = match member.kind {
                        MemberType::Uint8 | MemberType::Padding => 1,
                        MemberType::Uint16 => {
                            target += target % 2;
                            2
                        }
                        MemberType::Uint32 => {
                            target += target % 2;
                            4
                        }
                        MemberType::Uint8Array(length) | MemberType::Uint8ArrayReversed(length) => {
                            length
                        }
                        MemberType::Child => self
                            .children
                            .get(&member.key)
                            .map(|child| child.unaligned_length())
                            .unwrap_or(0),
                    };
                    let end = (source + length).min(self.buffer.len());
                    let copy_length = end.saturating_sub(source);
                    aligned[target..target + copy_length]
                        .copy_from_slice(&self.buffer[source..end]);
                    source += length;
                    target += length;
                }
                aligned
            }
        }
    }

    pub fn to_json(&self) -> Value {
        Value::Object(
            self.values
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        )
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    pub fn set(&mut self, key: &str, value: Value) {
        self.values.insert(key.to_string(), value);
    }

    pub fn get_u32(&self, key: &str) -> Option<u32> {
        self.values
            .get(key)
            .and_then(Value::as_u64)
            .map(|v| v as u32)
    }

    pub fn get_u16(&self, key: &str) -> Option<u16> {
        self.values
            .get(key)
            .and_then(Value::as_u64)
            .map(|v| v as u16)
    }

    pub fn get_u8(&self, key: &str) -> Option<u8> {
        self.values
            .get(key)
            .and_then(Value::as_u64)
            .map(|v| v as u8)
    }

    pub fn get_bytes(&self, key: &str) -> Option<Vec<u8>> {
        self.values.get(key).and_then(Value::as_array).map(|items| {
            items
                .iter()
                .filter_map(Value::as_u64)
                .map(|value| value as u8)
                .collect()
        })
    }
}
