// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/address-manager-entry.ts

use serde_json::json;

use crate::adapter::z_stack::structs::r#struct::{MemberType, Struct};

pub fn address_manager_entry(data: Option<&[u8]>) -> Struct {
    let mut entry = Struct::new()
        .member(MemberType::Uint8, "user")
        .member(MemberType::Uint16, "nwkAddr")
        .member(MemberType::Uint8ArrayReversed(8), "extAddr")
        .padding(0xff)
        .build(data, None);
    let ext = entry.get_bytes("extAddr").unwrap_or_default();
    let is_set = entry.get_u8("user").unwrap_or(0) != 0
        && ext.iter().any(|byte| *byte != 0)
        && !ext.iter().all(|byte| *byte == 0xff);
    entry.set("isSet", json!(is_set));
    entry
}
