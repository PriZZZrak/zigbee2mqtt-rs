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
    pub fn push(&mut self, chunk: &[u8]) {
        self.buffer.extend_from_slice(chunk);
        while let Some(position) = self.buffer.iter().position(|byte| *byte == 0x7e) {
            let frame_bytes: Vec<u8> = self.buffer.drain(..=position).collect();
            if frame_bytes.len() < 2 {
                continue;
            }
            let frame_type = match frame_bytes[0] & 0xe0 {
                0xc0 => FrameType::RSTACK,
                0xa0 => FrameType::NAK,
                _ => FrameType::DATA,
            };
            let sequence = frame_bytes
                .get(1)
                .map(|byte| i32::from(byte & 0x7f))
                .unwrap_or(0);
            self.parsed.push(Frame {
                frame_type,
                sequence,
                payload: frame_bytes[1..frame_bytes.len() - 1].to_vec(),
            });
        }
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
