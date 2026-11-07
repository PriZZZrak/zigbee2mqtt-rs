// Original: .upstream/zigbee-herdsman/src/zspec/zcl/zclStatusError.ts

use crate::zspec::zcl::definition::status::Status;

/// Upstream `ZclStatusError`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZclStatusError {
    pub code: Status,
}

impl ZclStatusError {
    pub fn new(code: Status) -> Self {
        Self { code }
    }

    pub fn message(&self) -> String {
        format!("Status '{}'", self.code.name())
    }
}
