// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/constants/sapi.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

/// Upstream `SAPI`.
pub mod zb_device_info {
    pub const DEV_STATE: u32 = 0;
    pub const IEEE_ADDR: u32 = 1;
    pub const SHORT_ADDR: u32 = 2;
    pub const PARENT_SHORT_ADDR: u32 = 3;
    pub const PARENT_IEEE_ADDR: u32 = 4;
    pub const CHANNEL: u32 = 5;
    pub const PAN_ID: u32 = 6;
    pub const EXT_PAN_ID: u32 = 7;
}
pub mod bind_action {
    pub const REMOVE_BIND: u32 = 0;
    pub const CREATE_BIND: u32 = 1;
}
pub mod search_type {
    pub const ZB_IEEE_SEARCH: u32 = 1;
}
pub mod tx_opt_ack {
    pub const NONE: u32 = 0;
    pub const END_TO_END_ACK: u32 = 1;
}
