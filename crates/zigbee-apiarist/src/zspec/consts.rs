// Original: .upstream/zigbee-herdsman/src/zspec/consts.ts

/// Upstream `ZIGBEE_REVISION` — consts.ts:4.
pub const ZIGBEE_REVISION: u16 = 23;

pub const BROADCAST_MIN: u16 = 0xfff8;

pub const ALL_802_15_4_CHANNELS_MASK: u32 = 0x07fff800;
pub const PREFERRED_802_15_4_CHANNELS_MASK: u32 = 0x0318c800;

pub const ALL_802_15_4_CHANNELS: [u8; 16] = [
    11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
];
pub const PREFERRED_802_15_4_CHANNELS: [u8; 7] = [11, 14, 15, 19, 20, 24, 25];

/// Upstream `INSTALL_CODE_SIZES` — consts.ts:89.
pub const INSTALL_CODE_SIZES: [usize; 4] = [18, 8, 10, 14];

/// Upstream `INSTALL_CODE_CRC_SIZE` — consts.ts:91.
pub const INSTALL_CODE_CRC_SIZE: usize = 2;

/// Upstream `BLANK_EUI64` — consts.ts:50.
pub const BLANK_EUI64: &str = "0xffffffffffffffff";

/// Upstream `COORDINATOR_ADDRESS` — consts.ts:7.
pub const COORDINATOR_ADDRESS: u16 = 0x0000;

/// Upstream `GP_ENDPOINT` — consts.ts:30.
pub const GP_ENDPOINT: u8 = 0xf2;

/// Upstream `GP_GROUP_ID` — consts.ts:32.
pub const GP_GROUP_ID: u16 = 0x0b84;

/// Upstream `INTEROPERABILITY_LINK_KEY` — consts.ts:67 ("ZigBeeAlliance09").
pub const INTEROPERABILITY_LINK_KEY: [u8; 16] = [
    0x5a, 0x69, 0x67, 0x42, 0x65, 0x65, 0x41, 0x6c, 0x6c, 0x69, 0x61, 0x6e, 0x63, 0x65, 0x30, 0x39,
];
