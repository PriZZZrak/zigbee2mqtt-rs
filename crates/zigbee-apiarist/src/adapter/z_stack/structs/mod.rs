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
}

impl Nib {
    /// Upstream: `nib.extendedPANID = value`.
    pub fn set_extended_pan_id(&mut self, value: Vec<u8>) {
        todo!()
    }

    /// Upstream: `nib.toJSON()`.
    pub fn to_json(&self) -> serde_json::Value {
        todo!()
    }

    /// Upstream: `nib.serialize()` (aligned + padded).
    pub fn serialize(&self) -> Vec<u8> {
        todo!()
    }

    /// Upstream: `nib.serialize(alignment, pad, offset)`.
    pub fn serialize_with(&self, alignment: &str, pad: bool, offset: usize) -> Vec<u8> {
        todo!()
    }

    /// Upstream: `nib.getLength(alignment)`.
    pub fn get_length(&self, alignment: &str) -> usize {
        todo!()
    }

    /// Upstream: `nib.getLength(alignment, false)`.
    pub fn get_length_unpadded(&self, alignment: &str) -> usize {
        todo!()
    }
}

pub fn nib() -> Nib {
    todo!()
}

pub fn nib_from(buffer: Vec<u8>) -> Nib {
    todo!()
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
        todo!()
    }

    pub fn serialize_mode(&self, alignment: &str) -> Vec<u8> {
        todo!()
    }
}

pub fn security_manager_table(capacity: usize) -> SecurityManagerTable {
    todo!()
}

pub fn security_manager_table_from_entries(
    entries: Vec<Vec<u8>>,
    alignment: &str,
) -> SecurityManagerTable {
    todo!()
}

pub fn security_manager_table_from_buffer(buffer: Vec<u8>) -> SecurityManagerTable {
    todo!()
}

pub fn security_manager_table_from_value(value: &serde_json::Value) -> SecurityManagerTable {
    todo!()
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
        todo!()
    }

    pub fn serialize_mode(&self, alignment: &str) -> Vec<u8> {
        todo!()
    }
}

pub fn nwk_sec_material_descriptor_table(capacity: usize) -> NwkSecMaterialDescriptorTable {
    todo!()
}
