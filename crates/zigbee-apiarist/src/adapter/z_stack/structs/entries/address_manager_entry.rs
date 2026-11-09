// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/structs/entries/address-manager-entry.ts

use crate::adapter::z_stack::structs::r#struct::{MemberType, Struct};

pub fn address_manager_entry(data: Option<&[u8]>) -> Struct {
    Struct::new()
        .member(MemberType::Uint8, "user")
        .member(MemberType::Uint16, "nwkAddr")
        .member(MemberType::Uint8ArrayReversed(8), "extAddr")
        .padding(0xff)
        .build(data)
}

/// Upstream `AddressManagerUser` flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AddressManagerUser {
    Default = 0x00,
    Assoc = 0x01,
    Security = 0x02,
    Binding = 0x04,
    Private1 = 0x08,
}

/// Upstream `isSet` method on `addressManagerEntry`.
pub fn is_set(entry: &Struct) -> bool {
    let ext = entry.get_bytes("extAddr").unwrap_or_default();
    entry.get_u8("user").unwrap_or(0) != 0x00 && ext != [0u8; 8] && ext != [0xffu8; 8]
}
