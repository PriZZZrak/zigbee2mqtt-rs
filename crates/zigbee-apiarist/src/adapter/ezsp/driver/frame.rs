// Original: .upstream/zigbee-herdsman/src/adapter/ezsp/driver/frame.ts

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
