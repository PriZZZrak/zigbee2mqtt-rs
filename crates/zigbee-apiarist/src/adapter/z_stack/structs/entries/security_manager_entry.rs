// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/security-manager-entry.ts

use crate::adapter::z_stack::structs::r#struct::{MemberType, Struct};

pub fn security_manager_entry(data: Option<&[u8]>) -> Struct {
    Struct::new()
        .member(MemberType::Uint16, "ami")
        .member(MemberType::Uint16, "keyNvId")
        .member(MemberType::Uint8, "authenticationOption")
        .build(data, Some(&[0xfe, 0xff, 0x00, 0x00, 0x00]))
}
