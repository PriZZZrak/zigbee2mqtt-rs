// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/nwk-key-descriptor.ts

use crate::adapter::z_stack::structs::r#struct::{MemberType, Struct};

pub fn nwk_key_descriptor(data: Option<&[u8]>) -> Struct {
    Struct::new()
        .member(MemberType::Uint8, "keySeqNum")
        .member(MemberType::Uint8Array(16), "key")
        .build(data)
}
