// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/nwk-pan-id.ts

use crate::adapter::z_stack::structs::r#struct::{MemberType, Struct};

pub fn nwk_pan_id(data: Option<&[u8]>) -> Struct {
    Struct::new()
        .member(MemberType::Uint16, "panId")
        .build(data, None)
}
