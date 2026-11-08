// Original: .upstream/zigbee-herdsman/src/adapter/ember/ezsp/consts.ts

pub const EZSP_MAX_FRAME_LENGTH: usize = 218 + 1 + 1;
pub const EZSP_SEQUENCE_INDEX: usize = 0;
pub const EZSP_EXTENDED_FRAME_CONTROL_LB_INDEX: usize = 1;
pub const EZSP_FRAME_ID_INDEX: usize = 2;
pub const EZSP_PARAMETERS_INDEX: usize = 3;

pub const EZSP_FRAME_CONTROL_COMMAND: u8 = 0x00;
pub const EZSP_FRAME_CONTROL_NETWORK_INDEX_MASK: u8 = 0x60;
pub const EZSP_FRAME_CONTROL_NETWORK_INDEX_OFFSET: u8 = 5;
pub const EZSP_FRAME_CONTROL_SLEEP_MODE_MASK: u8 = 0x03;
