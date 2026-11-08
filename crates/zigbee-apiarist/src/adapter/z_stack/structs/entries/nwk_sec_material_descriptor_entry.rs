// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/nwk-sec-material-descriptor-entry.ts

use serde_json::json;

use crate::adapter::z_stack::structs::r#struct::{MemberType, Struct};

pub fn nwk_sec_material_descriptor_entry(data: Option<&[u8]>) -> Struct {
    let mut entry = Struct::new()
        .member(MemberType::Uint32, "FrameCounter")
        .member(MemberType::Uint8ArrayReversed(8), "extendedPanID")
        .build(data, None);
    let is_set = entry
        .get_bytes("extendedPanID")
        .map(|bytes| bytes.iter().any(|byte| *byte != 0))
        .unwrap_or(false);
    entry.set("isSet", json!(is_set));
    entry
}
