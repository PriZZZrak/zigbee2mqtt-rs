// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/constants/zdo.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

/// Upstream `ZDO`.
pub mod status {
    pub const SUCCESS: u32 = 0;
    pub const INVALID_REQTYPE: u32 = 128;
    pub const DEVICE_NOT_FOUND: u32 = 129;
    pub const INVALID_EP: u32 = 130;
    pub const NOT_ACTIVE: u32 = 131;
    pub const NOT_SUPPORTED: u32 = 132;
    pub const TIMEOUT: u32 = 133;
    pub const NO_MATCH: u32 = 134;
    pub const NO_ENTRY: u32 = 136;
    pub const NO_DESCRIPTOR: u32 = 137;
    pub const INSUFFICIENT_SPACE: u32 = 138;
    pub const NOT_PERMITTED: u32 = 139;
    pub const TABLE_FULL: u32 = 140;
    pub const NOT_AUTHORIZED: u32 = 141;
    pub const BINDING_TABLE_FULL: u32 = 142;
}
pub mod init_dev {
    pub const RESTORED_NETWORK_STATE: u32 = 0;
    pub const NEW_NETWORK_STATE: u32 = 1;
    pub const LEAVE_NOT_STARTED: u32 = 2;
}
pub mod server_capability {
    pub const NOT_SUPPORTED: u32 = 0;
    pub const PRIM_TRUST_CENTER: u32 = 1;
    pub const BKUP_TRUST_CENTER: u32 = 2;
    pub const PRIM_BIND_TABLE: u32 = 4;
    pub const BKUP_BIND_TABLE: u32 = 8;
    pub const PRIM_DISC_TABLE: u32 = 16;
    pub const BKUP_DISC_TABLE: u32 = 32;
    pub const NETWORK_MANAGER: u32 = 64;
}
pub mod app_dev_ver {
    pub const VER_100: u32 = 0;
    pub const RESERVE01: u32 = 1;
    pub const RESERVE02: u32 = 2;
    pub const RESERVE03: u32 = 3;
    pub const RESERVE04: u32 = 4;
    pub const RESERVE05: u32 = 5;
    pub const RESERVE06: u32 = 6;
    pub const RESERVE07: u32 = 7;
    pub const RESERVE08: u32 = 8;
    pub const RESERVE09: u32 = 9;
    pub const RESERVE10: u32 = 10;
    pub const RESERVE11: u32 = 11;
    pub const RESERVE12: u32 = 12;
    pub const RESERVE13: u32 = 13;
    pub const RESERVE14: u32 = 14;
    pub const RESERVE15: u32 = 15;
}
pub mod stack_profile_id {
    pub const NETWORK_SPECIFIC: u32 = 0;
    pub const HOME_CONTROLS: u32 = 1;
    pub const ZIGBEEPRO_PROFILE: u32 = 2;
    pub const GENERIC_STAR: u32 = 3;
    pub const GENERIC_TREE: u32 = 4;
}
pub mod device_logical_type {
    pub const COORDINATOR: u32 = 0;
    pub const ROUTER: u32 = 1;
    pub const ENDDEVICE: u32 = 2;
    pub const COMPLEX_DESC_AVAIL: u32 = 4;
    pub const USER_DESC_AVAIL: u32 = 8;
    pub const RESERVED1: u32 = 16;
    pub const RESERVED2: u32 = 32;
    pub const RESERVED3: u32 = 64;
    pub const RESERVED4: u32 = 128;
}
pub mod addr_req_type {
    pub const SINGLE: u32 = 0;
    pub const EXTENDED: u32 = 1;
}
pub mod leave_and_remove_child {
    pub const NONE: u32 = 0;
    pub const LEAVE_REMOVE_CHILDREN: u32 = 1;
}
pub mod leave_ind_request {
    pub const INDICATION: u32 = 0;
    pub const REQUEST: u32 = 1;
}
pub mod leave_ind_remove {
    pub const NONE: u32 = 0;
    pub const REMOVE_CHILDREN: u32 = 1;
}
pub mod leave_ind_rejoin {
    pub const NONE: u32 = 0;
    pub const REJOIN: u32 = 1;
}
pub mod desc_capability {
    pub const EXT_LIST_NOT_SUPPORTED: u32 = 0;
    pub const EXT_ACTIVE_EP_LIST_AVAIL: u32 = 1;
    pub const EXT_SIMPLE_DESC_LIST_AVAIL: u32 = 2;
    pub const RESERVED1: u32 = 4;
    pub const RESERVED2: u32 = 8;
    pub const RESERVED3: u32 = 16;
    pub const RESERVED4: u32 = 32;
    pub const RESERVED5: u32 = 64;
    pub const RESERVED6: u32 = 128;
}
