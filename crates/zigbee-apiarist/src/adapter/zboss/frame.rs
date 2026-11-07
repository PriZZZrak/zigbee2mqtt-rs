// Original: .upstream/zigbee-herdsman/src/adapter/zboss/frame.ts
// TODO: port public API from the original file.

// pub fn ZBOSSBuffaloZcl() { todo!() }
// pub fn makeFrame() { todo!() }
// pub fn readByDesc() { todo!() }
// pub fn readZBOSSFrame() { todo!() }
// pub fn writeByDesc() { todo!() }
// pub fn writeZBOSSFrame() { todo!() }

pub fn zbossbuffalo_zcl() {
    todo!()
}
pub fn make_frame() {
    todo!()
}
pub fn read_by_desc() {
    todo!()
}
pub fn read_zbossframe() {
    todo!()
}
pub fn write_by_desc() {
    todo!()
}
pub fn write_zbossframe() {
    todo!()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    UNKNOWN = 0,
    RESPONSE = 1,
    INDICATION = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandId;

impl CommandId {
    pub const ZDO_NWK_ADDR_REQ: u16 = 0x0201;
    pub const ZDO_POWER_DESC_REQ: u16 = 0x0203;
    pub const ZDO_NODE_DESC_REQ: u16 = 0x0204;
    pub const ZDO_SIMPLE_DESC_REQ: u16 = 0x0205;
    pub const ZDO_ACTIVE_EP_REQ: u16 = 0x0206;
    pub const ZDO_MATCH_DESC_REQ: u16 = 0x0207;
    pub const ZDO_DEV_ANNCE_IND: u16 = 0x020c;
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZBOSSFrame {
    pub version: u8,
    pub r#type: FrameType,
    pub command_id: u16,
    pub tsn: u8,
    pub payload: serde_json::Value,
}

/// Upstream: `readZBOSSFrame(buffer)`.
pub fn read_zboss_frame(buffer: &[u8]) -> ZBOSSFrame {
    todo!()
}
