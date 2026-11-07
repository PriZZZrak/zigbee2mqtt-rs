// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/unpi/parser.ts
// TODO: port remaining Parser API.

use super::constants::{Type, MIN_MESSAGE_LENGTH, SOF};
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
        self.buffer.extend_from_slice(chunk);
        let mut frames = Vec::new();

        loop {
            let Some(start) = self.buffer.iter().position(|byte| *byte == SOF) else {
                self.buffer.clear();
                break;
            };
            if start > 0 {
                self.buffer.drain(..start);
            }
            if self.buffer.len() < MIN_MESSAGE_LENGTH {
                break;
            }

            let length = self.buffer[1] as usize;
            let total = length + 5;
            if self.buffer.len() < total {
                break;
            }

            let fcs = self.buffer[total - 1];
            let computed = Frame::calculate_checksum(&self.buffer[1..total - 1]);
            if fcs != computed {
                // Skip this SOF and resync on the next one.
                self.buffer.drain(..1);
                continue;
            }

            let r#type = match Type::from_u8(self.buffer[2] >> 5) {
                Some(value) => value,
                None => {
                    self.buffer.drain(..1);
                    continue;
                }
            };
            let subsystem = match super::constants::Subsystem::from_u8(self.buffer[2] & 0x1f) {
                Some(value) => value,
                None => {
                    self.buffer.drain(..1);
                    continue;
                }
            };

            frames.push(Frame {
                r#type,
                subsystem,
                commandID: self.buffer[3],
                data: self.buffer[4..4 + length].to_vec(),
                length: length as u8,
                fcs,
            });
            self.buffer.drain(..total);
        }

        frames
    }
}
