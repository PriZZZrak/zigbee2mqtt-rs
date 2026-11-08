// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/unpi/frame.ts

use super::constants::{Subsystem, Type, SOF};

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
    pub fn new(
        r#type: Type,
        subsystem: Subsystem,
        command_id: u8,
        data: Vec<u8>,
        length: u8,
        fcs: u8,
    ) -> Self {
        let calculated = Self::calculate_checksum(&data);
        let data_length = data.len() as u8;
        Self {
            r#type,
            subsystem,
            commandID: command_id,
            data,
            length: if length == 0 { data_length } else { length },
            fcs: if fcs == 0 { calculated } else { fcs },
        }
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        let mut buffer = vec![
            SOF,
            self.length,
            ((self.r#type as u8) << 5) | (self.subsystem as u8),
            self.commandID,
        ];
        buffer.extend_from_slice(&self.data);
        let checksum = Self::calculate_checksum(&buffer[1..]);
        buffer.push(checksum);
        buffer
    }

    pub fn from_buffer(length: u8, fcs_position: usize, buffer: &[u8]) -> Option<Self> {
        if buffer.len() < fcs_position + 1 || length as usize + 4 > buffer.len() {
            return None;
        }
        let r#type = Type::from_u8(buffer[2] >> 5)?;
        let subsystem = Subsystem::from_u8(buffer[2] & 0x1f)?;
        let data = buffer[4..4 + length as usize].to_vec();
        Some(Self {
            r#type,
            subsystem,
            commandID: buffer[3],
            data,
            length,
            fcs: buffer[fcs_position],
        })
    }

    pub fn calculate_checksum(data: &[u8]) -> u8 {
        let mut checksum = 0u8;
        for byte in data {
            checksum ^= byte;
        }
        checksum
    }
}
