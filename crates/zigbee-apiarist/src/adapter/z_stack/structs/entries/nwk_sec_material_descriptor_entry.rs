// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/nwk-sec-material-descriptor-entry.ts

use crate::adapter::z_stack::structs::r#struct::{MemberType, Struct};

pub fn nwk_sec_material_descriptor_entry(data: Option<&[u8]>) -> Struct {
    Struct::new()
        .member(MemberType::Uint32, "FrameCounter")
        .member(MemberType::Uint8ArrayReversed(8), "extendedPanID")
        .build(data)
}

/// Upstream `isSet` method on `nwkSecMaterialDescriptorEntry`.
pub fn is_set(entry: &Struct) -> bool {
    entry
        .get_bytes("extendedPanID")
        .map(|bytes| bytes.iter().any(|byte| *byte != 0))
        .unwrap_or(false)
}
