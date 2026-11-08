// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/unpi/writer.ts

use super::frame::Frame;

#[derive(Default)]
pub struct Writer {
    pub pushed: Vec<Vec<u8>>,
}

impl Writer {
    pub fn push(&mut self, buffer: Vec<u8>) {
        self.pushed.push(buffer);
    }

    pub fn write_frame(&mut self, frame: &Frame) {
        self.push(frame.to_buffer());
    }

    pub fn write_buffer(&mut self, buffer: &[u8]) {
        self.push(buffer.to_vec());
    }

    pub fn read(&mut self) {}
}
