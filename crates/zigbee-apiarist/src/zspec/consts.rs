// Original: .upstream/zigbee-herdsman/src/zspec/consts.ts
// TODO: port the full constants file.

pub const BROADCAST_MIN: u16 = 0xfff8;

pub const ALL_802_15_4_CHANNELS_MASK: u32 = 0x07fff800;
pub const PREFERRED_802_15_4_CHANNELS_MASK: u32 = 0x0318c800;

pub const ALL_802_15_4_CHANNELS: [u8; 16] = [
    11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
];
pub const PREFERRED_802_15_4_CHANNELS: [u8; 7] = [11, 14, 15, 19, 20, 24, 25];
