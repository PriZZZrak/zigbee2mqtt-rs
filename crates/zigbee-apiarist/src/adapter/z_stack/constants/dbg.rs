// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/constants/dbg.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

/// Upstream `DBG`.
pub mod debug_threshold {
    pub const CRITICAL: u32 = 1;
    pub const ERROR: u32 = 2;
    pub const INFORMATION: u32 = 3;
    pub const TRACE: u32 = 4;
}
pub mod component_id {
    pub const OSAL: u32 = 0;
    pub const MTEL: u32 = 1;
    pub const MTSPCI: u32 = 2;
    pub const NWK: u32 = 3;
    pub const NWKIF: u32 = 4;
    pub const MACCB: u32 = 5;
    pub const MAC: u32 = 6;
    pub const APP: u32 = 7;
    pub const TEST: u32 = 8;
    pub const RTG: u32 = 9;
    pub const DATA: u32 = 11;
}
