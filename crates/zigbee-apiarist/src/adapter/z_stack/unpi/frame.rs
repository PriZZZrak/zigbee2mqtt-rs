// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/unpi/frame.ts
// TODO: port Frame.

use super::constants::{Subsystem, Type, DATA_START, POSITION_CMD0, POSITION_CMD1, SOF};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub r#type: Type,
    pub subsystem: Subsystem,
    pub commandID: u8,
    pub data: Vec<u8>,
    pub length: u8,
    pub fcs: u8,
}

impl Frame {
    pub fn new(r#type: Type, subsystem: Subsystem, command_id: u8, data: Vec<u8>, length: u8, fcs: u8) -> Self {
        Self { r#type, subsystem, commandID: command_id, data, length, fcs }
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        todo!()
    }

    pub fn from_buffer(length: u8, fcs_position: usize, buffer: &[u8]) -> Option<Self> {
        todo!()
    }

    fn calculate_checksum(values: &[u8]) -> u8 {
        todo!()
    }
}
