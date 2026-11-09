// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/constants/mac.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

/// Upstream `MAC`.
pub mod assoc_status {
    pub const SUCCESSFUL_ASSOCIATION: u32 = 0;
    pub const PAN_AT_CAPACITY: u32 = 1;
    pub const PAN_ACCESS_DENIED: u32 = 2;
}
pub mod channel_page {
    pub const PAGE_0: u32 = 0;
    pub const PAGE_1: u32 = 1;
    pub const PAGE_2: u32 = 2;
}
pub mod tx_opt {
    pub const UNDEFINED: u32 = 0;
    pub const ACK_TRANS: u32 = 1;
    pub const GTS_TRANS: u32 = 2;
    pub const IND_TRANS: u32 = 4;
    pub const SEC_ENABLED_TRANS: u32 = 8;
    pub const NO_RE_TRANS: u32 = 16;
    pub const NO_CONFIRM_TRANS: u32 = 32;
    pub const USE_PIB_VALUE: u32 = 64;
    pub const USE_POWER_CHANNEL_VALUES: u32 = 128;
}
pub mod comm_reason {
    pub const ASSOCIATE_RSP: u32 = 0;
    pub const ORPHAN_RSP: u32 = 1;
    pub const RX_SECURE: u32 = 2;
}
pub mod disassoc_reason {
    pub const RESERVED: u32 = 0;
    pub const COOR_WISHES_DEV_LEAVE: u32 = 1;
    pub const DEV_WISHES_LEAVE: u32 = 2;
}
pub mod key_id_mode {
    pub const MODE_NONE_OR_IMPLICIT: u32 = 0;
    pub const MODE_1: u32 = 1;
    pub const MODE_4: u32 = 2;
    pub const MODE_8: u32 = 3;
}
pub mod beacon_order {
    pub const ORDER_NO_BEACONS: u32 = 15;
    pub const ORDER_4_MINUTES: u32 = 14;
    pub const ORDER_2_MINUTES: u32 = 13;
    pub const ORDER_1_MINUTE: u32 = 12;
    pub const ORDER_31_SECONDS: u32 = 11;
    pub const ORDER_15_SECONDS: u32 = 10;
    pub const ORDER_7_5_SECONDS: u32 = 9;
    pub const ORDER_4_SECONDS: u32 = 8;
    pub const ORDER_2_SECONDS: u32 = 7;
    pub const ORDER_1_SECOND: u32 = 6;
    pub const ORDER_480_MSEC: u32 = 5;
    pub const ORDER_240_MSEC: u32 = 4;
    pub const ORDER_120_MSEC: u32 = 3;
    pub const ORDER_60_MSEC: u32 = 2;
    pub const ORDER_30_MSEC: u32 = 1;
    pub const ORDER_15_MSEC: u32 = 0;
}
pub mod scan_type {
    pub const ENERGY_DETECT: u32 = 0;
    pub const ACTIVE: u32 = 1;
    pub const PASSIVE: u32 = 2;
    pub const ORPHAN: u32 = 3;
    pub const ENHANCED: u32 = 5;
}
pub mod front_end_mode {
    pub const PA_LNA_OFF: u32 = 0;
    pub const PA_LNA_ON: u32 = 1;
}
pub mod pid_attr {
    pub const ACK_WAIT_DURATION: u32 = 64;
    pub const ASSOCIATION_PERMIT: u32 = 65;
    pub const AUTO_REQUEST: u32 = 66;
    pub const BATT_LIFE_EXT: u32 = 67;
    pub const BATT_LIFE_EXT_PERIODS: u32 = 68;
    pub const BEACON_PAYLOAD: u32 = 69;
    pub const BEACON_PAYLOAD_LENGTH: u32 = 70;
    pub const BEACON_ORDER: u32 = 71;
    pub const BEACON_TX_TIME: u32 = 72;
    pub const BSN: u32 = 73;
    pub const COORD_EXTENDED_ADDRESS: u32 = 74;
    pub const COORD_SHORT_ADDRESS: u32 = 75;
    pub const DSN: u32 = 76;
    pub const GTS_PERMIT: u32 = 77;
    pub const MAX_CSMA_BACKOFFS: u32 = 78;
    pub const MIN_BE: u32 = 79;
    pub const PAN_ID: u32 = 80;
    pub const PROMISCUOUS_MODE: u32 = 81;
    pub const RX_ON_WHEN_IDLE: u32 = 82;
    pub const SHORT_ADDRESS: u32 = 83;
    pub const SUPERFRAME_ORDER: u32 = 84;
    pub const TRANSACTION_PERSISTENCE_TIME: u32 = 85;
    pub const ASSOCIATED_PAN_COORD: u32 = 86;
    pub const MAX_BE: u32 = 87;
    pub const MAX_FRAME_TOTAL_WAIT_TIME: u32 = 88;
    pub const MAX_FRAME_RETRIES: u32 = 89;
    pub const RESPONSE_WAIT_TIME: u32 = 90;
    pub const SYNC_SYMBOL_OFFSET: u32 = 91;
    pub const TIMESTAMP_SUPPORTED: u32 = 92;
    pub const SECURITY_ENABLED: u32 = 93;
    pub const KEY_TABLE: u32 = 113;
    pub const KEY_TABLE_ENTRIES: u32 = 114;
    pub const DEVICE_TABLE: u32 = 115;
    pub const DEVICE_TABLE_ENTRIES: u32 = 116;
    pub const SECURITY_LEVEL_TABLE: u32 = 117;
    pub const SECURITY_LEVEL_TABLE_ENTRIES: u32 = 118;
    pub const FRAME_COUNTER: u32 = 119;
    pub const AUTO_REQUEST_SECURITY_LEVEL: u32 = 120;
    pub const AUTO_REQUEST_KEY_ID_MODE: u32 = 121;
    pub const AUTO_REQUEST_KEY_SOURCE: u32 = 122;
    pub const AUTO_REQUEST_KEY_INDEX: u32 = 123;
    pub const DEFAULT_KEY_SOURCE: u32 = 124;
    pub const PAN_COORD_EXTENDED_ADDRESS: u32 = 125;
    pub const PAN_COORD_SHORT_ADDRESS: u32 = 126;
    pub const KEY_ID_LOOKUP_ENTRY: u32 = 208;
    pub const KEY_DEVICE_ENTRY: u32 = 209;
    pub const KEY_USAGE_ENTRY: u32 = 210;
    pub const KEY_ENTRY: u32 = 211;
    pub const DEVICE_ENTRY: u32 = 212;
    pub const SECURITY_LEVEL_ENTRY: u32 = 213;
    pub const PHY_TRANSMIT_POWER: u32 = 224;
    pub const LOGICAL_CHANNEL: u32 = 225;
    pub const EXTENDED_ADDRESS: u32 = 226;
    pub const ALT_BE: u32 = 227;
    pub const DEVICE_BEACON_ORDER: u32 = 228;
    pub const PHY_TRANSMIT_POWER_SIGNED: u32 = 229;
}
