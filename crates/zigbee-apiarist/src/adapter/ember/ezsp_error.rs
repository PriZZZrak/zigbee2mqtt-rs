// Original: .upstream/zigbee-herdsman/src/adapter/ember/ezspError.ts
// TODO: port EzspError.

use crate::adapter::ezsp::driver::types::basic::EzspStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EzspError {
    pub code: EzspStatus,
}

impl EzspError {
    pub fn new(code: EzspStatus) -> Self {
        Self { code }
    }

    pub fn message(&self) -> String {
        format!("{:?}", self.code)
    }
}

impl std::fmt::Display for EzspError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message())
    }
}

impl std::error::Error for EzspError {}
