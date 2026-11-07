// Original: .upstream/zigbee-herdsman/src/zspec/zcl/definition/status.ts
// TODO: port the full Status enum.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Status {
    SUCCESS = 0x00,
    INVALID_FIELD = 0x85,
    ABORT = 0x95,
}

impl Status {
    pub fn name(self) -> &'static str {
        match self {
            Status::SUCCESS => "SUCCESS",
            Status::INVALID_FIELD => "INVALID_FIELD",
            Status::ABORT => "ABORT",
        }
    }
}
