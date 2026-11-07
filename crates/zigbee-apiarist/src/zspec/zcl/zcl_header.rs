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
    pub fn from_buffer(_buffer: &[u8]) -> Option<Self> {
        todo!()
    }

    /// Upstream `header.write(buffalo)` returning the encoded header bytes.
    pub fn to_buffer(&self) -> Vec<u8> {
        todo!()
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

pub fn zcl_header() {
    todo!()
}
pub fn write() {
    todo!()
}
