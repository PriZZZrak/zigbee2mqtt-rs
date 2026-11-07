// Original: .upstream/zigbee-herdsman/src/adapter/ezsp/driver/parser.ts
// TODO: port public API from the original file.

use super::frame::{Frame, FrameType};

/// Model of the EZSP stream parser (upstream `Parser`).
#[derive(Debug, Default)]
pub struct Parser {
    buffer: Vec<u8>,
    pub parsed: Vec<Frame>,
}

impl Parser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Upstream `_transform(chunk)`: append bytes and emit every complete frame.
    pub fn push(&mut self, _chunk: &[u8]) {
        todo!()
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
        self.parsed.clear();
    }
}

pub fn parser() {
    todo!()
}
pub fn constructor() {
    todo!()
}
pub fn reset() {
    todo!()
}
