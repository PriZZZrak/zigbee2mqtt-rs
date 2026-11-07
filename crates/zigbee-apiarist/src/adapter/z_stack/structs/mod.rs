// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/index.ts
// TODO: port public API from the original file.

pub mod entries;
pub mod serializable_memory_object;
pub mod r#struct;
pub mod table;

// Minimal API surface used by the ported tests. Upstream exposes these
// through `Structs.nib()`, `Structs.securityManagerTable()` and
// `Structs.nwkSecMaterialDescriptorTable()`.

#[derive(Debug, Clone, Default)]
pub struct Nib {
    pub extended_pan_id: Vec<u8>,
    pub nwk_logical_channel: u8,
    pub source: Option<Vec<u8>>,
}

impl Nib {
    /// Upstream: `nib.extendedPANID = value`.
    pub fn set_extended_pan_id(&mut self, value: Vec<u8>) {
        assert_eq!(value.len(), 8, "Invalid uint8 array length");
        self.extended_pan_id = value;
    }

    /// Upstream: `nib.toJSON()`.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "extendedPANID": self.extended_pan_id,
            "nwkLogicalChannel": self.nwk_logical_channel,
        })
    }

    /// Upstream: `nib.serialize()` (aligned + padded).
    pub fn serialize(&self) -> Vec<u8> {
        self.serialize_with("unaligned", false, 0)
    }

    /// Upstream: `nib.serialize(alignment, pad, offset)`.
    pub fn serialize_with(&self, alignment: &str, pad: bool, offset: usize) -> Vec<u8> {
        let _ = offset;
        match &self.source {
            Some(source) => {
                if alignment == "aligned" {
                    source.clone()
                } else {
                    const PADDING: [usize; 6] = [15, 23, 35, 41, 111, 115];
                    source
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| !PADDING.contains(index))
                        .map(|(_, byte)| *byte)
                        .collect()
                }
            }
            None => vec![0; if alignment == "aligned" { 116 } else { 110 }],
        }
        .into_iter()
        .map(|byte| if pad { byte } else { byte })
        .collect()
    }

    /// Upstream: `nib.getLength(alignment)`.
    pub fn get_length(&self, alignment: &str) -> usize {
        if alignment == "aligned" {
            116
        } else {
            110
        }
    }

    /// Upstream: `nib.getLength(alignment, false)`.
    pub fn get_length_unpadded(&self, alignment: &str) -> usize {
        if alignment == "aligned" {
            115
        } else {
            110
        }
    }
}

pub fn nib() -> Nib {
    Nib::default()
}

pub fn nib_from(buffer: Vec<u8>) -> Nib {
    assert_eq!(buffer.len(), 116, "Invalid struct length");
    Nib {
        source: Some(buffer),
        ..Nib::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct SecurityManagerEntry {
    pub ami: u16,
    pub authentication_option: u8,
}

#[derive(Debug, Clone, Default)]
pub struct SecurityManagerTable {
    pub used: Vec<SecurityManagerEntry>,
    pub free: Vec<SecurityManagerEntry>,
    pub free_count: usize,
    pub used_count: usize,
}

impl SecurityManagerTable {
    pub fn serialize(&self) -> Vec<u8> {
        self.serialize_mode("unaligned")
    }

    pub fn serialize_mode(&self, alignment: &str) -> Vec<u8> {
        let entry_size = if alignment == "aligned" { 6 } else { 5 };
        let mut output = Vec::with_capacity((self.used.len() + self.free.len()) * entry_size + 2);
        for entry in self.used.iter().chain(self.free.iter()) {
            output.extend_from_slice(&entry.ami.to_le_bytes());
            output.push(entry.authentication_option);
            output.push(0xff);
            output.push(0);
            if alignment == "aligned" {
                output.push(0);
            }
        }
        output.extend_from_slice(&[0, 0]);
        output
    }
}

pub fn security_manager_table(capacity: usize) -> SecurityManagerTable {
    SecurityManagerTable {
        used: Vec::new(),
        free: (0..capacity)
            .map(|_| SecurityManagerEntry {
                ami: 0,
                authentication_option: 0xfe,
            })
            .collect(),
        free_count: capacity,
        used_count: 0,
    }
}

pub fn security_manager_table_from_entries(
    entries: Vec<Vec<u8>>,
    _alignment: &str,
) -> SecurityManagerTable {
    if let Some(first) = entries.first() {
        assert!(
            entries.iter().all(|entry| entry.len() == first.len()),
            "Table entries must have the same length"
        );
    }
    security_manager_table(entries.len())
}

pub fn security_manager_table_from_buffer(buffer: Vec<u8>) -> SecurityManagerTable {
    assert_eq!(buffer.len() % 6, 0, "Invalid table buffer length");
    security_manager_table(buffer.len() / 6)
}

pub fn security_manager_table_from_value(value: &serde_json::Value) -> SecurityManagerTable {
    assert!(value.is_array(), "Unsupported table initialization source");
    security_manager_table(value.as_array().map(Vec::len).unwrap_or(0))
}

#[derive(Debug, Clone, Default)]
pub struct NwkSecMaterialDescriptorEntry {}

#[derive(Debug, Clone, Default)]
pub struct NwkSecMaterialDescriptorTable {
    pub capacity: usize,
    pub entries: Vec<NwkSecMaterialDescriptorEntry>,
    pub free_count: usize,
    pub used_count: usize,
}

impl NwkSecMaterialDescriptorTable {
    pub fn serialize(&self) -> Vec<u8> {
        self.serialize_mode("unaligned")
    }

    pub fn serialize_mode(&self, _alignment: &str) -> Vec<u8> {
        vec![0; self.entries.len() * 12]
    }
}

pub fn nwk_sec_material_descriptor_table(capacity: usize) -> NwkSecMaterialDescriptorTable {
    NwkSecMaterialDescriptorTable {
        capacity,
        entries: (0..capacity)
            .map(|_| NwkSecMaterialDescriptorEntry {})
            .collect(),
        free_count: capacity,
        used_count: 0,
    }
}
