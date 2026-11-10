// Original: .upstream/zigbee-herdsman/src/zspec/zdo/definition/consts.ts

/// The endpoint where the Zigbee Device Object (ZDO) resides.
pub const ZDO_ENDPOINT: u8 = 0;
/// The profile ID used by the Zigbee Device Object (ZDO).
pub const ZDO_PROFILE_ID: u16 = 0x0000;
/// ZDO messages start with a sequence number.
pub const ZDO_MESSAGE_OVERHEAD: usize = 1;
pub const MULTICAST_BINDING: u8 = 0x01;
pub const UNICAST_BINDING: u8 = 0x03;
/// 64-bit challenge value used by CHALLENGE_REQUEST/CHALLENGE_RESPONSE clusters.
pub const CHALLENGE_VALUE_SIZE: usize = 8;
/// The 256-bit Curve 25519 public point.
pub const CURVE_PUBLIC_POINT_SIZE: usize = 32;

/// Upstream `LeaveRequestFlags.WITHOUT_REJOIN` — enums.ts:7.
pub const WITHOUT_REJOIN: u8 = 0x00;
