// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/aps-link-key-data-entry.ts

use crate::adapter::z_stack::structs::r#struct::{MemberType, Struct};

pub fn aps_link_key_data_entry(data: Option<&[u8]>) -> Struct {
    Struct::new()
        .member(MemberType::Uint8Array(16), "key")
        .member(MemberType::Uint32, "txFrmCntr")
        .member(MemberType::Uint32, "rxFrmCntr")
        .build(data)
}
