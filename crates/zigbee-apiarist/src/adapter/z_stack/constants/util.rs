// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/constants/util.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

/// Upstream `UTIL`.
pub mod get_nv_status {
    pub const SUCCESS: u32 = 0;
    pub const GET_IEEE_ADDR_FAIL: u32 = 1;
    pub const GET_SCAN_CHANNEL_FAIL: u32 = 2;
    pub const GET_PAN_ID_FAIL: u32 = 4;
    pub const GET_SECURITY_LEVEL_FAIL: u32 = 8;
    pub const GET_PRECONFIG_KEY_FAIL: u32 = 16;
}
pub mod subsystem_id {
    pub const SYS: u32 = 256;
    pub const MAC: u32 = 512;
    pub const NWK: u32 = 768;
    pub const AF: u32 = 1024;
    pub const ZDO: u32 = 1280;
    pub const SAPI: u32 = 1536;
    pub const UTIL: u32 = 1792;
    pub const DBG: u32 = 2048;
    pub const APP: u32 = 2304;
    pub const ALL_SUBSYSTEM: u32 = 65535;
}
pub mod device_type {
    pub const NONE: u32 = 0;
    pub const COORDINATOR: u32 = 1;
    pub const ROUTER: u32 = 2;
    pub const END_DEVICE: u32 = 4;
}
pub mod key_event {
    pub const KEY_1: u32 = 0;
    pub const KEY_2: u32 = 1;
    pub const KEY_3: u32 = 2;
    pub const KEY_4: u32 = 3;
    pub const KEY_5: u32 = 4;
    pub const KEY_6: u32 = 5;
    pub const KEY_7: u32 = 6;
    pub const KEY_8: u32 = 7;
}
pub mod key_value {
    pub const KEY_1: u32 = 1;
    pub const KEY_2: u32 = 2;
    pub const KEY_3: u32 = 4;
    pub const KEY_4: u32 = 8;
    pub const KEY_5: u32 = 16;
    pub const KEY_6: u32 = 32;
    pub const KEY_7: u32 = 64;
    pub const KEY_8: u32 = 128;
}
pub mod led_mode {
    pub const OFF: u32 = 0;
    pub const ON: u32 = 1;
    pub const BLINK: u32 = 2;
    pub const FLASH: u32 = 3;
    pub const TOGGLE: u32 = 4;
}
pub mod led_num {
    pub const LED_1: u32 = 1;
    pub const LED_2: u32 = 2;
    pub const LED_3: u32 = 3;
    pub const LED_4: u32 = 4;
    pub const ALL_LEDS: u32 = 255;
}
pub mod subs_action {
    pub const UNSUBSCRIBE: u32 = 0;
    pub const SUBSCRIBE: u32 = 1;
}
pub mod ack_pending_option {
    pub const ACK_DISABLE: u32 = 0;
    pub const ACK_ENABLE: u32 = 1;
}
pub mod node_relation {
    pub const PARENT: u32 = 0;
    pub const CHILD_RFD: u32 = 1;
    pub const CHILD_RFD_RX_IDLE: u32 = 2;
    pub const CHILD_FFD: u32 = 3;
    pub const CHILD_FFD_RX_IDLE: u32 = 4;
    pub const NEIGHBOR: u32 = 5;
    pub const OTHER: u32 = 6;
    pub const NOTUSED: u32 = 255;
}
