// Original: .upstream/zigbee-herdsman/src/adapter/ember/ezsp/buffalo.ts
// TODO: port EzspBuffalo.

use crate::adapter::ember::enums::SLStatus;

pub struct EzspBuffalo {
    pub position: usize,
    pub buffer: Vec<u8>,
}

impl EzspBuffalo {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self {
            position: 0,
            buffer,
        }
    }

    pub fn get_written(&self) -> Vec<u8> {
        todo!()
    }

    pub fn set_position(&mut self, position: usize) {
        todo!()
    }

    pub fn set_command_byte(&mut self, position: usize, value: u8) {
        todo!()
    }

    pub fn get_command_byte(&self, position: usize) -> u8 {
        todo!()
    }

    pub fn write_uint8(&mut self, value: u8) {
        todo!()
    }

    /// Upstream: `readStatus(status, true)`.
    pub fn read_status(&mut self, status: u8) -> SLStatus {
        todo!()
    }

    /// Upstream: `readStatus(status, false)`.
    pub fn read_status_no_ember(&mut self, status: u8) -> SLStatus {
        todo!()
    }
}
