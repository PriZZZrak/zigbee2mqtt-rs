// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/unpi/writer.ts
// TODO: port Writer.

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
        todo!()
    }

    pub fn write_buffer(&mut self, buffer: &[u8]) {
        todo!()
    }

    pub fn read(&mut self) {
        todo!()
    }
}
