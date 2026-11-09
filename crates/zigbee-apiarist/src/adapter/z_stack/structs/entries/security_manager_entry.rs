// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/security-manager-entry.ts

use crate::adapter::z_stack::structs::r#struct::{MemberType, Struct};

/// Upstream `SecurityManagerAuthenticationOption`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SecurityManagerAuthenticationOption {
    Default = 0x00,
    AuthenticatedCBCK = 0x01,
    AuthenticatedEA = 0x02,
}

/// Upstream occupancy function for `securityManagerTable`.
pub fn is_used(entry: &Struct) -> bool {
    let ami = entry.get_u16("ami").unwrap_or(0);
    let authentication_option = entry.get_u8("authenticationOption").unwrap_or(0);
    !matches!(ami, 0xfffe | 0xffff)
        && !(ami == 0x0000
            && authentication_option == SecurityManagerAuthenticationOption::Default as u8)
}

pub fn security_manager_entry(data: Option<&[u8]>) -> Struct {
    Struct::new()
        .member(MemberType::Uint16, "ami")
        .member(MemberType::Uint16, "keyNvId")
        .member(MemberType::Uint8, "authenticationOption")
        .default_data(&[0xfe, 0xff, 0x00, 0x00, 0x00])
        .build(data)
}
