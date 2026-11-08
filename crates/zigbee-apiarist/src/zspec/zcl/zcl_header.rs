// Original: .upstream/zigbee-herdsman/src/zspec/zcl/zclHeader.ts
// TODO: port header encoding/decoding.

/// Upstream `FrameType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    GLOBAL = 0,
    SPECIFIC = 1,
}

/// Upstream `Direction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Direction {
    CLIENT_TO_SERVER = 0,
    SERVER_TO_CLIENT = 1,
}

/// Upstream `FrameControl`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameControl {
    /// Upstream camelCase: `reservedBits`.
    pub reserved_bits: u8,
    /// Upstream camelCase: `frameType`.
    pub frame_type: FrameType,
    /// Upstream camelCase: `direction`.
    pub direction: Direction,
    /// Upstream camelCase: `disableDefaultResponse`.
    pub disable_default_response: bool,
    /// Upstream camelCase: `manufacturerSpecific`.
    pub manufacturer_specific: bool,
}

/// Upstream `ZclHeader`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// Upstream camelCase: `frameControl`.
    pub frame_control: FrameControl,
    /// Upstream camelCase: `manufacturerCode`.
    pub manufacturer_code: Option<u16>,
    /// Upstream camelCase: `transactionSequenceNumber`.
    pub transaction_sequence_number: u8,
    /// Upstream camelCase: `commandIdentifier`.
    pub command_identifier: u8,
    pub length: usize,
}

impl Header {
    /// Upstream `new ZclHeader(frameControl, manufacturerCode, transactionSequenceNumber, commandId)`.
    pub fn new(
        frame_control: FrameControl,
        manufacturer_code: Option<u16>,
        transaction_sequence_number: u8,
        command_identifier: u8,
    ) -> Self {
        let length = if frame_control.manufacturer_specific {
            5
        } else {
            3
        };
        Self {
            frame_control,
            manufacturer_code,
            transaction_sequence_number,
            command_identifier,
            length,
        }
    }

    /// Upstream `ZclHeader.fromBuffer(buffer)`.
    pub fn from_buffer(buffer: &[u8]) -> Option<Self> {
        let control = *buffer.first()?;
        let frame_type = match control & 0x03 {
            0 => FrameType::GLOBAL,
            1 => FrameType::SPECIFIC,
            _ => return None,
        };
        let manufacturer_specific = control & 0x04 != 0;
        let direction = if control & 0x08 != 0 {
            Direction::SERVER_TO_CLIENT
        } else {
            Direction::CLIENT_TO_SERVER
        };
        let frame_control = FrameControl {
            reserved_bits: (control >> 5) & 0x07,
            frame_type,
            direction,
            disable_default_response: control & 0x10 != 0,
            manufacturer_specific,
        };
        let mut offset = 1;
        let manufacturer_code = if manufacturer_specific {
            let bytes = [*buffer.get(offset)?, *buffer.get(offset + 1)?];
            offset += 2;
            Some(u16::from_le_bytes(bytes))
        } else {
            None
        };
        let transaction_sequence_number = *buffer.get(offset)?;
        let command_identifier = *buffer.get(offset + 1)?;
        Some(Self::new(
            frame_control,
            manufacturer_code,
            transaction_sequence_number,
            command_identifier,
        ))
    }

    /// Upstream `header.write(buffalo)` returning the encoded header bytes.
    pub fn to_buffer(&self) -> Vec<u8> {
        let control = self.frame_control;
        let mut byte = control.reserved_bits << 5;
        byte |= (control.frame_type as u8) & 0x03;
        byte |= (control.manufacturer_specific as u8) << 2;
        byte |= (control.direction as u8) << 3;
        byte |= (control.disable_default_response as u8) << 4;

        let mut buffer = vec![byte];
        if control.manufacturer_specific {
            if let Some(code) = self.manufacturer_code {
                buffer.extend_from_slice(&code.to_le_bytes());
            }
        }
        buffer.push(self.transaction_sequence_number);
        buffer.push(self.command_identifier);
        buffer
    }

    /// Upstream getter `isGlobal`.
    pub fn is_global(&self) -> bool {
        self.frame_control.frame_type == FrameType::GLOBAL
    }

    /// Upstream getter `isSpecific`.
    pub fn is_specific(&self) -> bool {
        self.frame_control.frame_type == FrameType::SPECIFIC
    }
}
