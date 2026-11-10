// Original: .upstream/zigbee-herdsman/src/zspec/zdo/definition/status.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

/// Upstream `Status`.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Status {
    SUCCESS = 0x00,
    INV_REQUESTTYPE = 0x80,
    DEVICE_NOT_FOUND = 0x81,
    INVALID_EP = 0x82,
    NOT_ACTIVE = 0x83,
    NOT_SUPPORTED = 0x84,
    TIMEOUT = 0x85,
    NO_MATCH = 0x86,
    NO_ENTRY = 0x88,
    NO_DESCRIPTOR = 0x89,
    INSUFFICIENT_SPACE = 0x8a,
    NOT_PERMITTED = 0x8b,
    TABLE_FULL = 0x8c,
    NOT_AUTHORIZED = 0x8d,
    DEVICE_BINDING_TABLE_FULL = 0x8e,
    INVALID_INDEX = 0x8f,
    FRAME_TOO_LARGE = 0x90,
    BAD_KEY_NEGOTIATION_METHOD = 0x91,
    TEMPORARY_FAILURE = 0x92,
    NWK_LAYER_INVALID_PARAMETER = 0xc1,
    NWK_LAYER_INV_REQUESTTYPE = 0xc2,
    NWK_LAYER_NOT_PERMITTED = 0xc3,
    NWK_LAYER_STARTUP_FAILURE = 0xc4,
    NWK_LAYER_ALREADY_PRESENT = 0xc5,
    NWK_LAYER_SYNC_FAILURE = 0xc6,
    NWK_LAYER_NEIGHBOR_TABLE_FULL = 0xc7,
    NWK_LAYER_UNKNOWN_DEVICE = 0xc8,
    NWK_LAYER_UNSUPPORTED_ATTRIBUTE = 0xc9,
    NWK_LAYER_NO_NETWORKS = 0xca,
    NWK_LAYER_MAX_FRM_COUNTER = 0xcc,
    NWK_LAYER_NO_KEY = 0xcd,
    NWK_LAYER_BAD_CCM_OUTPUT = 0xce,
    NWK_LAYER_RESERVED = 0xcf,
    NWK_LAYER_ROUTE_DISCOVERY_FAILED = 0xd0,
    NWK_LAYER_ROUTE_ERROR = 0xd1,
    NWK_LAYER_BT_TABLE_FULL = 0xd2,
    NWK_LAYER_FRAME_NOT_BUFFERED = 0xd3,
    NWK_LAYER_INVALID_INTERFACE = 0xd5,
    NWK_LAYER_MISSING_TLV = 0xd6,
    NWK_LAYER_INVALID_TLV = 0xd7,
}

impl Status {
    pub fn name(self) -> &'static str {
        match self {
            Self::SUCCESS => "SUCCESS",
            Self::INV_REQUESTTYPE => "INV_REQUESTTYPE",
            Self::DEVICE_NOT_FOUND => "DEVICE_NOT_FOUND",
            Self::INVALID_EP => "INVALID_EP",
            Self::NOT_ACTIVE => "NOT_ACTIVE",
            Self::NOT_SUPPORTED => "NOT_SUPPORTED",
            Self::TIMEOUT => "TIMEOUT",
            Self::NO_MATCH => "NO_MATCH",
            Self::NO_ENTRY => "NO_ENTRY",
            Self::NO_DESCRIPTOR => "NO_DESCRIPTOR",
            Self::INSUFFICIENT_SPACE => "INSUFFICIENT_SPACE",
            Self::NOT_PERMITTED => "NOT_PERMITTED",
            Self::TABLE_FULL => "TABLE_FULL",
            Self::NOT_AUTHORIZED => "NOT_AUTHORIZED",
            Self::DEVICE_BINDING_TABLE_FULL => "DEVICE_BINDING_TABLE_FULL",
            Self::INVALID_INDEX => "INVALID_INDEX",
            Self::FRAME_TOO_LARGE => "FRAME_TOO_LARGE",
            Self::BAD_KEY_NEGOTIATION_METHOD => "BAD_KEY_NEGOTIATION_METHOD",
            Self::TEMPORARY_FAILURE => "TEMPORARY_FAILURE",
            Self::NWK_LAYER_INVALID_PARAMETER => "NWK_LAYER_INVALID_PARAMETER",
            Self::NWK_LAYER_INV_REQUESTTYPE => "NWK_LAYER_INV_REQUESTTYPE",
            Self::NWK_LAYER_NOT_PERMITTED => "NWK_LAYER_NOT_PERMITTED",
            Self::NWK_LAYER_STARTUP_FAILURE => "NWK_LAYER_STARTUP_FAILURE",
            Self::NWK_LAYER_ALREADY_PRESENT => "NWK_LAYER_ALREADY_PRESENT",
            Self::NWK_LAYER_SYNC_FAILURE => "NWK_LAYER_SYNC_FAILURE",
            Self::NWK_LAYER_NEIGHBOR_TABLE_FULL => "NWK_LAYER_NEIGHBOR_TABLE_FULL",
            Self::NWK_LAYER_UNKNOWN_DEVICE => "NWK_LAYER_UNKNOWN_DEVICE",
            Self::NWK_LAYER_UNSUPPORTED_ATTRIBUTE => "NWK_LAYER_UNSUPPORTED_ATTRIBUTE",
            Self::NWK_LAYER_NO_NETWORKS => "NWK_LAYER_NO_NETWORKS",
            Self::NWK_LAYER_MAX_FRM_COUNTER => "NWK_LAYER_MAX_FRM_COUNTER",
            Self::NWK_LAYER_NO_KEY => "NWK_LAYER_NO_KEY",
            Self::NWK_LAYER_BAD_CCM_OUTPUT => "NWK_LAYER_BAD_CCM_OUTPUT",
            Self::NWK_LAYER_RESERVED => "NWK_LAYER_RESERVED",
            Self::NWK_LAYER_ROUTE_DISCOVERY_FAILED => "NWK_LAYER_ROUTE_DISCOVERY_FAILED",
            Self::NWK_LAYER_ROUTE_ERROR => "NWK_LAYER_ROUTE_ERROR",
            Self::NWK_LAYER_BT_TABLE_FULL => "NWK_LAYER_BT_TABLE_FULL",
            Self::NWK_LAYER_FRAME_NOT_BUFFERED => "NWK_LAYER_FRAME_NOT_BUFFERED",
            Self::NWK_LAYER_INVALID_INTERFACE => "NWK_LAYER_INVALID_INTERFACE",
            Self::NWK_LAYER_MISSING_TLV => "NWK_LAYER_MISSING_TLV",
            Self::NWK_LAYER_INVALID_TLV => "NWK_LAYER_INVALID_TLV",
        }
    }

    pub fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0x00 => Self::SUCCESS,
            0x80 => Self::INV_REQUESTTYPE,
            0x81 => Self::DEVICE_NOT_FOUND,
            0x82 => Self::INVALID_EP,
            0x83 => Self::NOT_ACTIVE,
            0x84 => Self::NOT_SUPPORTED,
            0x85 => Self::TIMEOUT,
            0x86 => Self::NO_MATCH,
            0x88 => Self::NO_ENTRY,
            0x89 => Self::NO_DESCRIPTOR,
            0x8a => Self::INSUFFICIENT_SPACE,
            0x8b => Self::NOT_PERMITTED,
            0x8c => Self::TABLE_FULL,
            0x8d => Self::NOT_AUTHORIZED,
            0x8e => Self::DEVICE_BINDING_TABLE_FULL,
            0x8f => Self::INVALID_INDEX,
            0x90 => Self::FRAME_TOO_LARGE,
            0x91 => Self::BAD_KEY_NEGOTIATION_METHOD,
            0x92 => Self::TEMPORARY_FAILURE,
            0xc1 => Self::NWK_LAYER_INVALID_PARAMETER,
            0xc2 => Self::NWK_LAYER_INV_REQUESTTYPE,
            0xc3 => Self::NWK_LAYER_NOT_PERMITTED,
            0xc4 => Self::NWK_LAYER_STARTUP_FAILURE,
            0xc5 => Self::NWK_LAYER_ALREADY_PRESENT,
            0xc6 => Self::NWK_LAYER_SYNC_FAILURE,
            0xc7 => Self::NWK_LAYER_NEIGHBOR_TABLE_FULL,
            0xc8 => Self::NWK_LAYER_UNKNOWN_DEVICE,
            0xc9 => Self::NWK_LAYER_UNSUPPORTED_ATTRIBUTE,
            0xca => Self::NWK_LAYER_NO_NETWORKS,
            0xcc => Self::NWK_LAYER_MAX_FRM_COUNTER,
            0xcd => Self::NWK_LAYER_NO_KEY,
            0xce => Self::NWK_LAYER_BAD_CCM_OUTPUT,
            0xcf => Self::NWK_LAYER_RESERVED,
            0xd0 => Self::NWK_LAYER_ROUTE_DISCOVERY_FAILED,
            0xd1 => Self::NWK_LAYER_ROUTE_ERROR,
            0xd2 => Self::NWK_LAYER_BT_TABLE_FULL,
            0xd3 => Self::NWK_LAYER_FRAME_NOT_BUFFERED,
            0xd5 => Self::NWK_LAYER_INVALID_INTERFACE,
            0xd6 => Self::NWK_LAYER_MISSING_TLV,
            0xd7 => Self::NWK_LAYER_INVALID_TLV,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_u8_roundtrip_for_all_variants() {
        for status in [
            Status::SUCCESS,
            Status::NO_ENTRY,
            Status::INVALID_INDEX,
            Status::NOT_AUTHORIZED,
            Status::NWK_LAYER_INVALID_TLV,
        ] {
            assert_eq!(Status::from_u8(status as u8), Some(status));
        }
        assert_eq!(Status::from_u8(0xff), None);
    }
}
