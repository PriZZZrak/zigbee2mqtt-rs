// Original: .upstream/zigbee-herdsman/src/adapter/ezsp/driver/ezsp.ts
// TODO: port public API from the original file.

// pub fn EZSPFrameData() { todo!() }
// pub fn EZSPZDORequestFrameData() { todo!() }
// pub fn EZSPZDOResponseFrameData() { todo!() }
// pub fn Ezsp() { todo!() }
// pub fn close() { todo!() }
// pub fn connect() { todo!() }
// pub fn execCommand() { todo!() }
// pub fn isInitialized() { todo!() }
// pub fn makeZDOframe() { todo!() }
// pub fn sendBroadcast() { todo!() }
// pub fn sendMulticast() { todo!() }
// pub fn sendUnicast() { todo!() }
// pub fn setSourceRouting() { todo!() }
// pub fn waitFor() { todo!() }

pub fn ezspframe_data() {
    todo!()
}
pub fn ezspzdorequest_frame_data() {
    todo!()
}
pub fn ezspzdoresponse_frame_data() {
    todo!()
}
pub fn ezsp() {
    todo!()
}
pub fn close() {
    todo!()
}
pub fn connect() {
    todo!()
}
pub fn exec_command() {
    todo!()
}
pub fn is_initialized() {
    todo!()
}
pub fn make_zdoframe() {
    todo!()
}
pub fn send_broadcast() {
    todo!()
}
pub fn send_multicast() {
    todo!()
}
pub fn send_unicast() {
    todo!()
}
pub fn set_source_routing() {
    todo!()
}
pub fn wait_for() {
    todo!()
}

#[derive(Debug, Clone, PartialEq)]
pub struct EzspFrame {
    pub cls: &'static str,
    pub id: u16,
    pub fields: serde_json::Value,
}

pub struct EZSPFrameData;

impl EZSPFrameData {
    /// Upstream: `EZSPFrameData.createFrame(version, frameId, isRequest, buffer)`.
    pub fn create_frame(
        version: u8,
        frame_id: u16,
        is_request: bool,
        buffer: &[u8],
    ) -> Option<EzspFrame> {
        todo!()
    }
}

impl Default for EZSPFrameData {
    fn default() -> Self {
        Self
    }
}
