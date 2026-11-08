// Original: .upstream/zigbee-herdsman/src/zspec/zcl/definition/status.ts

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Status {
    SUCCESS = 0x00,
    FAILURE = 0x01,
    NOT_AUTHORIZED = 0x7e,
    MALFORMED_COMMAND = 0x80,
    INVALID_FIELD = 0x85,
    UNSUPPORTED_ATTRIBUTE = 0x86,
    ABORT = 0x95,
}

impl Status {
    pub fn name(self) -> &'static str {
        match self {
            Status::SUCCESS => "SUCCESS",
            Status::FAILURE => "FAILURE",
            Status::NOT_AUTHORIZED => "NOT_AUTHORIZED",
            Status::MALFORMED_COMMAND => "MALFORMED_COMMAND",
            Status::INVALID_FIELD => "INVALID_FIELD",
            Status::UNSUPPORTED_ATTRIBUTE => "UNSUPPORTED_ATTRIBUTE",
            Status::ABORT => "ABORT",
        }
    }
}
