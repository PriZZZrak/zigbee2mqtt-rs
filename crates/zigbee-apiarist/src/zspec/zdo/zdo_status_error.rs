// Original: .upstream/zigbee-herdsman/src/zspec/zdo/zdoStatusError.ts

use crate::zspec::zdo::definition::status::Status;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZdoStatusError {
    pub code: Status,
}

impl ZdoStatusError {
    pub fn new(code: Status) -> Self {
        Self { code }
    }

    pub fn message(&self) -> String {
        format!("Status '{}'", self.code.name())
    }
}
