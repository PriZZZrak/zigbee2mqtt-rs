// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/constants/sys.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

/// Upstream `SYS`.
pub mod reset_type {
    pub const HARD: u32 = 0;
    pub const SOFT: u32 = 1;
}
pub mod capabilities {
    pub const SYS: u32 = 1;
    pub const MAC: u32 = 2;
    pub const NWK: u32 = 4;
    pub const AF: u32 = 8;
    pub const ZDO: u32 = 16;
    pub const SAPI: u32 = 32;
    pub const UTIL: u32 = 64;
    pub const DEBUG: u32 = 128;
    pub const APP: u32 = 256;
    pub const ZOAD: u32 = 4096;
}
pub mod osal_timer_event {
    pub const EVENT_0: u32 = 0;
    pub const EVENT_1: u32 = 1;
    pub const EVENT_2: u32 = 2;
    pub const EVENT_3: u32 = 3;
}
pub mod adc_channels {
    pub const AIN0: u32 = 0;
    pub const AIN1: u32 = 1;
    pub const AIN2: u32 = 2;
    pub const AIN3: u32 = 3;
    pub const AIN4: u32 = 4;
    pub const AIN5: u32 = 5;
    pub const AIN6: u32 = 6;
    pub const AIN7: u32 = 7;
    pub const TEMP_SENSOR: u32 = 14;
    pub const VOLT_READ: u32 = 15;
}
pub mod adc_resolution {
    pub const BIT_8: u32 = 0;
    pub const BIT_10: u32 = 1;
    pub const BIT_12: u32 = 2;
    pub const BIT_14: u32 = 3;
}
pub mod gpio_operation {
    pub const SET_DIRECTION: u32 = 0;
    pub const SET_INPUT_MODE: u32 = 1;
    pub const SET: u32 = 2;
    pub const CLEAR: u32 = 3;
    pub const TOGGLE: u32 = 4;
    pub const READ: u32 = 5;
}
pub mod sys_stk_tune {
    pub const TX_PWR: u32 = 0;
    pub const RX_ON_IDLE: u32 = 1;
}
pub mod reset_reason {
    pub const POWER_UP: u32 = 0;
    pub const EXTERNAL: u32 = 1;
    pub const WATCH_DOG: u32 = 2;
}
pub mod nv_item_init_status {
    pub const ALREADY_EXISTS: u32 = 0;
    pub const SUCCESS: u32 = 9;
    pub const FAILED: u32 = 10;
}
pub mod nv_item_delete_status {
    pub const SUCCESS: u32 = 0;
    pub const NOT_EXISTS: u32 = 9;
    pub const FAILED: u32 = 10;
    pub const BAD_LENGTH: u32 = 12;
}
