// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/serializable-memory-object.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use super::r#struct::{Alignment, Struct};
use super::table::Table;

/// Upstream `SerializableMemoryObject`.
pub trait SerializableMemoryObject: Sync {
    /// Upstream `serialize(alignment?)`.
    fn serialize_memory(&self, alignment: Alignment) -> Vec<u8>;
}

impl SerializableMemoryObject for Struct {
    fn serialize_memory(&self, alignment: Alignment) -> Vec<u8> {
        self.serialize(alignment, true, 0)
    }
}

impl SerializableMemoryObject for Table {
    fn serialize_memory(&self, alignment: Alignment) -> Vec<u8> {
        self.serialize(alignment)
    }
}
