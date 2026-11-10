// Original: .upstream/zigbee-herdsman/src/zspec/zcl/definition/status.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

/// Upstream `Status`.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Status {
    SUCCESS = 0x00,
    FAILURE = 0x01,
    NOT_AUTHORIZED = 0x7e,
    RESERVED = 0x7f,
    MALFORMED_COMMAND = 0x80,
    UNSUP_COMMAND = 0x81,
    UNSUP_GENERAL_COMMAND = 0x82,
    UNSUP_MANUF_CLUSTER_COMMAND = 0x83,
    UNSUP_MANUF_GENERAL_COMMAND = 0x84,
    INVALID_FIELD = 0x85,
    UNSUPPORTED_ATTRIBUTE = 0x86,
    INVALID_VALUE = 0x87,
    READ_ONLY = 0x88,
    INSUFFICIENT_SPACE = 0x89,
    DUPLICATE_EXISTS = 0x8a,
    NOT_FOUND = 0x8b,
    UNREPORTABLE_ATTRIBUTE = 0x8c,
    INVALID_DATA_TYPE = 0x8d,
    INVALID_SELECTOR = 0x8e,
    WRITE_ONLY = 0x8f,
    INCONSISTENT_STARTUP_STATE = 0x90,
    DEFINED_OUT_OF_BAND = 0x91,
    RESERVED14 = 0x92,
    ACTION_DENIED = 0x93,
    TIMEOUT = 0x94,
    ABORT = 0x95,
    INVALID_IMAGE = 0x96,
    WAIT_FOR_DATA = 0x97,
    NO_IMAGE_AVAILABLE = 0x98,
    REQUIRE_MORE_IMAGE = 0x99,
    NOTIFICATION_PENDING = 0x9a,
    HARDWARE_FAILURE = 0xc0,
    SOFTWARE_FAILURE = 0xc1,
    RESERVED15 = 0xc2,
    UNSUPPORTED_CLUSTER = 0xc3,
    LIMIT_REACHED = 0xc4,
}

impl Status {
    pub fn name(self) -> &'static str {
        match self {
            Self::SUCCESS => "SUCCESS",
            Self::FAILURE => "FAILURE",
            Self::NOT_AUTHORIZED => "NOT_AUTHORIZED",
            Self::RESERVED => "RESERVED",
            Self::MALFORMED_COMMAND => "MALFORMED_COMMAND",
            Self::UNSUP_COMMAND => "UNSUP_COMMAND",
            Self::UNSUP_GENERAL_COMMAND => "UNSUP_GENERAL_COMMAND",
            Self::UNSUP_MANUF_CLUSTER_COMMAND => "UNSUP_MANUF_CLUSTER_COMMAND",
            Self::UNSUP_MANUF_GENERAL_COMMAND => "UNSUP_MANUF_GENERAL_COMMAND",
            Self::INVALID_FIELD => "INVALID_FIELD",
            Self::UNSUPPORTED_ATTRIBUTE => "UNSUPPORTED_ATTRIBUTE",
            Self::INVALID_VALUE => "INVALID_VALUE",
            Self::READ_ONLY => "READ_ONLY",
            Self::INSUFFICIENT_SPACE => "INSUFFICIENT_SPACE",
            Self::DUPLICATE_EXISTS => "DUPLICATE_EXISTS",
            Self::NOT_FOUND => "NOT_FOUND",
            Self::UNREPORTABLE_ATTRIBUTE => "UNREPORTABLE_ATTRIBUTE",
            Self::INVALID_DATA_TYPE => "INVALID_DATA_TYPE",
            Self::INVALID_SELECTOR => "INVALID_SELECTOR",
            Self::WRITE_ONLY => "WRITE_ONLY",
            Self::INCONSISTENT_STARTUP_STATE => "INCONSISTENT_STARTUP_STATE",
            Self::DEFINED_OUT_OF_BAND => "DEFINED_OUT_OF_BAND",
            Self::RESERVED14 => "RESERVED14",
            Self::ACTION_DENIED => "ACTION_DENIED",
            Self::TIMEOUT => "TIMEOUT",
            Self::ABORT => "ABORT",
            Self::INVALID_IMAGE => "INVALID_IMAGE",
            Self::WAIT_FOR_DATA => "WAIT_FOR_DATA",
            Self::NO_IMAGE_AVAILABLE => "NO_IMAGE_AVAILABLE",
            Self::REQUIRE_MORE_IMAGE => "REQUIRE_MORE_IMAGE",
            Self::NOTIFICATION_PENDING => "NOTIFICATION_PENDING",
            Self::HARDWARE_FAILURE => "HARDWARE_FAILURE",
            Self::SOFTWARE_FAILURE => "SOFTWARE_FAILURE",
            Self::RESERVED15 => "RESERVED15",
            Self::UNSUPPORTED_CLUSTER => "UNSUPPORTED_CLUSTER",
            Self::LIMIT_REACHED => "LIMIT_REACHED",
        }
    }

    /// Upstream numeric statuses are thrown as `Status` values.
    pub fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0x00 => Self::SUCCESS,
            0x01 => Self::FAILURE,
            0x7e => Self::NOT_AUTHORIZED,
            0x7f => Self::RESERVED,
            0x80 => Self::MALFORMED_COMMAND,
            0x81 => Self::UNSUP_COMMAND,
            0x82 => Self::UNSUP_GENERAL_COMMAND,
            0x83 => Self::UNSUP_MANUF_CLUSTER_COMMAND,
            0x84 => Self::UNSUP_MANUF_GENERAL_COMMAND,
            0x85 => Self::INVALID_FIELD,
            0x86 => Self::UNSUPPORTED_ATTRIBUTE,
            0x87 => Self::INVALID_VALUE,
            0x88 => Self::READ_ONLY,
            0x89 => Self::INSUFFICIENT_SPACE,
            0x8a => Self::DUPLICATE_EXISTS,
            0x8b => Self::NOT_FOUND,
            0x8c => Self::UNREPORTABLE_ATTRIBUTE,
            0x8d => Self::INVALID_DATA_TYPE,
            0x8e => Self::INVALID_SELECTOR,
            0x8f => Self::WRITE_ONLY,
            0x90 => Self::INCONSISTENT_STARTUP_STATE,
            0x91 => Self::DEFINED_OUT_OF_BAND,
            0x92 => Self::RESERVED14,
            0x93 => Self::ACTION_DENIED,
            0x94 => Self::TIMEOUT,
            0x95 => Self::ABORT,
            0x96 => Self::INVALID_IMAGE,
            0x97 => Self::WAIT_FOR_DATA,
            0x98 => Self::NO_IMAGE_AVAILABLE,
            0x99 => Self::REQUIRE_MORE_IMAGE,
            0x9a => Self::NOTIFICATION_PENDING,
            0xc0 => Self::HARDWARE_FAILURE,
            0xc1 => Self::SOFTWARE_FAILURE,
            0xc2 => Self::RESERVED15,
            0xc3 => Self::UNSUPPORTED_CLUSTER,
            0xc4 => Self::LIMIT_REACHED,
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
            Status::FAILURE,
            Status::DUPLICATE_EXISTS,
            Status::NOT_FOUND,
            Status::UNREPORTABLE_ATTRIBUTE,
            Status::NO_IMAGE_AVAILABLE,
            Status::UNSUPPORTED_CLUSTER,
            Status::LIMIT_REACHED,
        ] {
            assert_eq!(Status::from_u8(status as u8), Some(status));
        }
        assert_eq!(Status::from_u8(0xff), None);
    }
}
