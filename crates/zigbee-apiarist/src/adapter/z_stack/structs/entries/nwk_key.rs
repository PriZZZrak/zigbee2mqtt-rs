// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/nwk-key.ts

use crate::adapter::z_stack::structs::r#struct::{MemberType, Struct};

pub fn nwk_key(data: Option<&[u8]>) -> Struct {
    Struct::new()
        .member(MemberType::Uint8Array(16), "key")
        .build(data, None)
}
