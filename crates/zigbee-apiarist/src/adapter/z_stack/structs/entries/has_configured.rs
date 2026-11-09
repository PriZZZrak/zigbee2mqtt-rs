// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/has-configured.ts

use crate::adapter::z_stack::structs::r#struct::{MemberType, Struct};

pub fn has_configured(data: Option<&[u8]>) -> Struct {
    Struct::new()
        .member(MemberType::Uint8, "hasConfigured")
        .build(data)
}

pub fn is_configured(entry: &Struct) -> bool {
    entry.get_u8("hasConfigured") == Some(0x55)
}
