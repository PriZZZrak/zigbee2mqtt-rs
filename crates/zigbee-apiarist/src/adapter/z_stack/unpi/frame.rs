// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/unpi/frame.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

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
    /// Upstream `constructor(...)` — frame.ts:10.
    ///
    /// The Rust port always computes `length`/`fcs` when omitted (callers pass
    /// zero), mirroring upstream `toBuffer()`.
    pub fn new(
        r#type: Type,
        subsystem: Subsystem,
        command_id: u8,
        data: Vec<u8>,
        _length: u8,
        _fcs: u8,
    ) -> Self {
        let length = data.len() as u8;
        let mut payload = vec![
            SOF,
            length,
            ((r#type as u8) << 5) | (subsystem as u8),
            command_id,
        ];
        payload.extend_from_slice(&data);
        let fcs = Self::calculate_checksum(&payload[1..]);
        Self {
            r#type,
            subsystem,
            commandID: command_id,
            data,
            length,
            fcs,
        }
    }

    /// Upstream `toBuffer()` — frame.ts:22.
    pub fn to_buffer(&self) -> Vec<u8> {
        let length = self.data.len() as u8;
        let cmd0 = ((self.r#type as u8) << 5) | (self.subsystem as u8);

        let mut payload = vec![SOF, length, cmd0, self.commandID];
        payload.extend_from_slice(&self.data);
        let fcs = Self::calculate_checksum(&payload[1..]);

        payload.push(fcs);
        payload
    }

    /// Upstream `Frame.fromBuffer(...)` — frame.ts:33.
    pub fn from_buffer(length: u8, fcs_position: usize, buffer: &[u8]) -> Option<Self> {
        if buffer.len() < fcs_position + 1 {
            return None;
        }

        let r#type = Type::from_u8(buffer[2] >> 5)?;
        let subsystem = Subsystem::from_u8(buffer[2] & 0x1f)?;
        let command_id = buffer[3];
        let data = buffer[4..fcs_position].to_vec();
        let fcs = buffer[fcs_position];

        // Upstream: validate the checksum to see if we fully received the message.
        let checksum = Self::calculate_checksum(&buffer[1..fcs_position]);
        if checksum != fcs {
            return None;
        }

        Some(Self {
            r#type,
            subsystem,
            commandID: command_id,
            data,
            length,
            fcs,
        })
    }

    /// Upstream `calculateChecksum(values)` — frame.ts:50.
    pub fn calculate_checksum(data: &[u8]) -> u8 {
        let mut checksum = 0u8;
        for byte in data {
            checksum ^= byte;
        }
        checksum
    }
}
