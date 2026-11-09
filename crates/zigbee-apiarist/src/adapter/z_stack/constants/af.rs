// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/constants/af.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

/// Upstream `AF`.
pub mod interpan_ctl {
    pub const CTL: u32 = 0;
    pub const SET: u32 = 1;
    pub const REG: u32 = 2;
    pub const CHK: u32 = 3;
}
pub mod network_latency_req {
    pub const NO_LATENCY_REQS: u32 = 0;
    pub const FAST_BEACONS: u32 = 1;
    pub const SLOW_BEACONS: u32 = 2;
}
pub mod options {
    pub const PREPROCESS: u32 = 4;
    pub const LIMIT_CONCENTRATOR: u32 = 8;
    pub const ACK_REQUEST: u32 = 16;
    pub const DISCV_ROUTE: u32 = 32;
    pub const EN_SECURITY: u32 = 64;
    pub const SKIP_ROUTING: u32 = 128;
}
pub const DEFAULT_RADIUS: u32 = 30;
