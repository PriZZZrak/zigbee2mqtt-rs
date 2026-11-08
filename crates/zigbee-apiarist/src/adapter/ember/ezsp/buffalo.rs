// Original: .upstream/zigbee-herdsman/src/adapter/ember/ezsp/buffalo.ts

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
        self.buffer[..self.position.min(self.buffer.len())].to_vec()
    }

    pub fn set_position(&mut self, position: usize) {
        self.position = position;
    }

    pub fn set_command_byte(&mut self, position: usize, value: u8) {
        self.buffer[position] = value;
    }

    pub fn get_command_byte(&self, position: usize) -> u8 {
        self.buffer[position]
    }

    pub fn write_uint8(&mut self, value: u8) {
        self.buffer[self.position] = value;
        self.position += 1;
    }

    /// Upstream: `readStatus(status, true)`.
    pub fn read_status(&mut self, _status: u8) -> SLStatus {
        let value = self.buffer.get(self.position).copied().unwrap_or(0);
        self.position += 1;
        match value {
            0x00 => SLStatus::OK,
            0x02 => SLStatus::INVALID_PARAMETER,
            0x9c => SLStatus::ZIGBEE_NETWORK_OPENED,
            0x4b => SLStatus::BUS_ERROR,
            _ => SLStatus::OK,
        }
    }

    /// Upstream: `readStatus(status, false)`.
    pub fn read_status_no_ember(&mut self, _status: u8) -> SLStatus {
        let value = self.buffer.get(self.position).copied().unwrap_or(0);
        self.position += 1;
        if value == 0 {
            SLStatus::OK
        } else {
            SLStatus::ZIGBEE_EZSP_ERROR
        }
    }
}
