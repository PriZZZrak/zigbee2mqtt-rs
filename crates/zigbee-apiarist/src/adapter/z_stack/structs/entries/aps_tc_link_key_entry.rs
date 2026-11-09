// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/aps-tc-link-key-entry.ts

use crate::adapter::z_stack::structs::r#struct::{MemberType, Struct};

pub fn aps_tc_link_key_entry(data: Option<&[u8]>) -> Struct {
    Struct::new()
        .member(MemberType::Uint32, "txFrmCntr")
        .member(MemberType::Uint32, "rxFrmCntr")
        .member(MemberType::Uint8ArrayReversed(8), "extAddr")
        .member(MemberType::Uint8, "keyAttributes")
        .member(MemberType::Uint8, "keyType")
        .member(MemberType::Uint8, "SeedShift_IcIndex")
        .build(data)
}
