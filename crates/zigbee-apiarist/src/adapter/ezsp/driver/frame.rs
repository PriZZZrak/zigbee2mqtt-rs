// Original: .upstream/zigbee-herdsman/src/adapter/ezsp/driver/frame.ts
// TODO: port public API from the original file.

/// Upstream `FrameType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    RSTACK,
    DATA,
    NAK,
}

/// A parsed EZSP frame (upstream `Frame`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub frame_type: FrameType,
    pub sequence: i32,
    pub payload: Vec<u8>,
}

pub fn frame() {
    todo!()
}
pub fn check_crc() {
    todo!()
}
pub fn constructor() {
    todo!()
}
pub fn to_string() {
    todo!()
}
