// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/unpi/parser.ts
// TODO: port Parser.

use super::frame::Frame;

#[derive(Default)]
pub struct Parser {
    buffer: Vec<u8>,
}

impl Parser {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Append `chunk` and return every complete, checksum-valid frame.
    pub fn transform(&mut self, chunk: &[u8]) -> Vec<Frame> {
        todo!()
    }
}
