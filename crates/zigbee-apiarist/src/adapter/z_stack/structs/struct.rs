// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/struct.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).
//
// Buffer-backed struct. Upstream exposes members as getters/setters over a
// Node Buffer; Rust exposes equivalent typed accessors. Member order and
// offsets are identical, so serialize/toJSON stay byte-compatible.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberType {
    Uint8,
    Uint16,
    Uint32,
    Uint8Array(usize),
    Uint8ArrayReversed(usize),
    Child,
}

#[derive(Debug, Clone)]
pub struct Member {
    pub key: String,
    pub offset: usize,
    pub kind: MemberType,
}

#[derive(Debug, Clone, Default)]
pub struct Struct {
    members: Vec<Member>,
    children: BTreeMap<String, Struct>,
    buffer: Vec<u8>,
    default_data: Option<Vec<u8>>,
    length: usize,
    padding_byte: u8,
}

impl Struct {
    /// Upstream `Struct.new()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Upstream `member(type, name, length?)`.
    pub fn member(mut self, kind: MemberType, name: &str) -> Self {
        let offset = self.length;
        let length = match kind {
            MemberType::Uint8 => 1,
            MemberType::Uint16 => 2,
            MemberType::Uint32 => 4,
            MemberType::Uint8Array(length) | MemberType::Uint8ArrayReversed(length) => {
                if length == 0 {
                    panic!(
                        "Struct builder requires length for `uint8array` and `uint8array-reversed` type"
                    );
                }
                length
            }
            MemberType::Child => panic!("use `child()` for struct members"),
        };
        self.members.push(Member {
            key: name.to_string(),
            offset,
            kind,
        });
        self.length += length;
        self
    }

    /// Upstream `member("struct", name, factory)`.
    pub fn child(mut self, name: &str, child: Struct) -> Self {
        let offset = self.length;
        self.length += child.length;
        self.children.insert(name.to_string(), child);
        self.members.push(Member {
            key: name.to_string(),
            offset,
            kind: MemberType::Child,
        });
        self
    }

    /// Upstream `default(data)`.
    pub fn default_data(mut self, data: &[u8]) -> Self {
        if data.len() != self.length {
            panic!("Default value needs to have the length of unaligned structure.");
        }
        self.default_data = Some(data.to_vec());
        self
    }

    /// Upstream `padding(byte)`.
    pub fn padding(mut self, byte: u8) -> Self {
        self.padding_byte = byte;
        self
    }

    /// Upstream `build(data?)`.
    pub fn build(mut self, data: Option<&[u8]>) -> Self {
        let unaligned = self.length;
        let aligned = self.get_length(Alignment::Aligned, true, 0);

        self.buffer = match data {
            Some(data) if data.len() == unaligned => data.to_vec(),
            Some(data) if data.len() == aligned => {
                let mut buffer = vec![self.padding_byte; unaligned];
                let mut offset = 0usize;
                for member in &self.members {
                    match member.kind {
                        MemberType::Uint8 => {
                            buffer[member.offset] = data[offset];
                            offset += 1;
                        }
                        MemberType::Uint16 => {
                            offset += offset % 2;
                            buffer[member.offset..member.offset + 2]
                                .copy_from_slice(&data[offset..offset + 2]);
                            offset += 2;
                        }
                        MemberType::Uint32 => {
                            offset += offset % 2;
                            buffer[member.offset..member.offset + 4]
                                .copy_from_slice(&data[offset..offset + 4]);
                            offset += 4;
                        }
                        MemberType::Uint8Array(length) | MemberType::Uint8ArrayReversed(length) => {
                            buffer[member.offset..member.offset + length]
                                .copy_from_slice(&data[offset..offset + length]);
                            offset += length;
                        }
                        MemberType::Child => {
                            let child = self.children.get(&member.key).expect("child struct");
                            let child_length = child.length;
                            let child_data = &data[offset..offset + child_length];
                            buffer[member.offset..member.offset + child_length]
                                .copy_from_slice(child_data);
                            offset += child_length;
                        }
                    }
                }
                buffer
            }
            Some(data) => panic!(
                "Struct length mismatch (expected={unaligned}/{aligned}, got={})",
                data.len()
            ),
            None => self
                .default_data
                .clone()
                .unwrap_or_else(|| vec![self.padding_byte; unaligned]),
        };

        for member in self.members.clone() {
            if let MemberType::Child = member.kind {
                let child = self.children.remove(&member.key).expect("child struct");
                let child_length = child.length;
                let bytes = self.buffer[member.offset..member.offset + child_length].to_vec();
                self.children.insert(member.key, child.build(Some(&bytes)));
            }
        }

        self
    }

    /// Upstream `serialize(alignment, padLength, parentOffset)`.
    pub fn serialize(
        &self,
        alignment: Alignment,
        pad_length: bool,
        parent_offset: usize,
    ) -> Vec<u8> {
        match alignment {
            Alignment::Unaligned => {
                let mut buffer = self.buffer.clone();
                for member in &self.members {
                    if let MemberType::Child = member.kind {
                        let child = self.children.get(&member.key).expect("child struct");
                        let data = child.serialize(Alignment::Unaligned, false, 0);
                        buffer[member.offset..member.offset + data.len()].copy_from_slice(&data);
                    }
                }
                buffer
            }
            Alignment::Aligned => {
                let mut aligned =
                    vec![self.padding_byte; self.get_length(alignment, pad_length, parent_offset)];
                let mut offset = 0usize;
                for member in &self.members {
                    match member.kind {
                        MemberType::Uint8 => {
                            aligned[offset] = self.buffer[member.offset];
                            offset += 1;
                        }
                        MemberType::Uint16 => {
                            offset += offset % 2;
                            aligned[offset..offset + 2]
                                .copy_from_slice(&self.buffer[member.offset..member.offset + 2]);
                            offset += 2;
                        }
                        MemberType::Uint32 => {
                            offset += offset % 2;
                            aligned[offset..offset + 4]
                                .copy_from_slice(&self.buffer[member.offset..member.offset + 4]);
                            offset += 4;
                        }
                        MemberType::Uint8Array(length) | MemberType::Uint8ArrayReversed(length) => {
                            aligned[offset..offset + length].copy_from_slice(
                                &self.buffer[member.offset..member.offset + length],
                            );
                            offset += length;
                        }
                        MemberType::Child => {
                            let child = self.children.get(&member.key).expect("child struct");
                            let data = child.serialize(Alignment::Aligned, false, offset);
                            aligned[offset..offset + data.len()].copy_from_slice(&data);
                            offset += data.len();
                        }
                    }
                }
                aligned
            }
        }
    }

    /// Upstream `getLength(alignment, padLength, parentOffset)`.
    pub fn get_length(
        &self,
        alignment: Alignment,
        pad_length: bool,
        parent_offset: usize,
    ) -> usize {
        match alignment {
            Alignment::Unaligned => self.length,
            Alignment::Aligned => {
                let mut length = 0usize;
                for member in &self.members {
                    match member.kind {
                        MemberType::Uint8 => length += 1,
                        MemberType::Uint16 => {
                            length += ((parent_offset + length) % 2) + 2;
                        }
                        MemberType::Uint32 => {
                            length += ((parent_offset + length) % 2) + 4;
                        }
                        MemberType::Uint8Array(length_)
                        | MemberType::Uint8ArrayReversed(length_) => {
                            length += length_;
                        }
                        MemberType::Child => {
                            let child = self.children.get(&member.key).expect("child struct");
                            length += child.get_length(Alignment::Aligned, false, 0);
                        }
                    }
                }
                if pad_length {
                    length += length % 2;
                }
                length
            }
        }
    }

    /// Upstream `toJSON()`.
    pub fn to_json(&self) -> Value {
        let mut object = serde_json::Map::new();
        for member in &self.members {
            let value = match member.kind {
                MemberType::Uint8 => json!(self.get_u8(&member.key).unwrap_or(0)),
                MemberType::Uint16 => json!(self.get_u16(&member.key).unwrap_or(0)),
                MemberType::Uint32 => json!(self.get_u32(&member.key).unwrap_or(0)),
                MemberType::Uint8Array(_) | MemberType::Uint8ArrayReversed(_) => {
                    json!(self.get_bytes(&member.key).unwrap_or_default())
                }
                MemberType::Child => self
                    .children
                    .get(&member.key)
                    .map(Struct::to_json)
                    .unwrap_or(Value::Null),
            };
            object.insert(member.key.clone(), value);
        }
        Value::Object(object)
    }

    fn member_ref(&self, key: &str) -> Option<&Member> {
        self.members.iter().find(|member| member.key == key)
    }

    pub fn get_u8(&self, key: &str) -> Option<u8> {
        let member = self.member_ref(key)?;
        matches!(member.kind, MemberType::Uint8)
            .then(|| self.buffer.get(member.offset).copied())
            .flatten()
    }

    pub fn get_u16(&self, key: &str) -> Option<u16> {
        let member = self.member_ref(key)?;
        if !matches!(member.kind, MemberType::Uint16) {
            return None;
        }
        let bytes = self.buffer.get(member.offset..member.offset + 2)?;
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn get_u32(&self, key: &str) -> Option<u32> {
        let member = self.member_ref(key)?;
        if !matches!(member.kind, MemberType::Uint32) {
            return None;
        }
        let bytes = self.buffer.get(member.offset..member.offset + 4)?;
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn get_bytes(&self, key: &str) -> Option<Vec<u8>> {
        let member = self.member_ref(key)?;
        let length = match member.kind {
            MemberType::Uint8Array(length) | MemberType::Uint8ArrayReversed(length) => length,
            _ => return None,
        };
        let mut bytes = self
            .buffer
            .get(member.offset..member.offset + length)?
            .to_vec();
        if matches!(member.kind, MemberType::Uint8ArrayReversed(_)) {
            bytes.reverse();
        }
        Some(bytes)
    }

    pub fn set_u8(&mut self, key: &str, value: u8) {
        let member = self.member_ref(key).expect("unknown struct member");
        assert!(
            matches!(member.kind, MemberType::Uint8),
            "member type mismatch"
        );
        let offset = member.offset;
        self.buffer[offset] = value;
    }

    pub fn set_u16(&mut self, key: &str, value: u16) {
        let member = self.member_ref(key).expect("unknown struct member");
        assert!(
            matches!(member.kind, MemberType::Uint16),
            "member type mismatch"
        );
        let offset = member.offset;
        self.buffer[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    pub fn set_u32(&mut self, key: &str, value: u32) {
        let member = self.member_ref(key).expect("unknown struct member");
        assert!(
            matches!(member.kind, MemberType::Uint32),
            "member type mismatch"
        );
        let offset = member.offset;
        self.buffer[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    pub fn set_bytes(&mut self, key: &str, value: &[u8]) {
        let member = self.member_ref(key).expect("unknown struct member");
        let (length, reversed) = match member.kind {
            MemberType::Uint8Array(length) => (length, false),
            MemberType::Uint8ArrayReversed(length) => (length, true),
            _ => panic!("member type mismatch"),
        };
        if value.len() != length {
            panic!(
                "Invalid length for member {key} (expected={length}, got={})",
                value.len()
            );
        }
        let offset = member.offset;
        let data = if reversed {
            value.iter().rev().copied().collect::<Vec<_>>()
        } else {
            value.to_vec()
        };
        self.buffer[offset..offset + length].copy_from_slice(&data);
    }

    pub fn child_ref(&self, key: &str) -> Option<&Struct> {
        self.children.get(key)
    }

    pub fn child_mut(&mut self, key: &str) -> Option<&mut Struct> {
        self.children.get_mut(key)
    }
}
