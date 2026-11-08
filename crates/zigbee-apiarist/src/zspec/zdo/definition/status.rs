// Original: .upstream/zigbee-herdsman/src/zspec/zdo/definition/status.ts

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    INVALID_INDEX,
}

impl Status {
    pub fn name(self) -> &'static str {
        match self {
            Status::INVALID_INDEX => "INVALID_INDEX",
        }
    }
}
