// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/unpi/constants.ts

pub const SOF: u8 = 0xfe;
pub const DATA_START: usize = 4;
pub const MIN_MESSAGE_LENGTH: usize = 5;
pub const MAX_DATA_SIZE: usize = 250;
pub const POSITION_DATA_LENGTH: usize = 1;
pub const POSITION_CMD0: usize = 2;
pub const POSITION_CMD1: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Type {
    POLL = 0,
    SREQ = 1,
    AREQ = 2,
    SRSP = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Subsystem {
    RESERVED = 0,
    SYS = 1,
    MAC = 2,
    NWK = 3,
    AF = 4,
    ZDO = 5,
    SAPI = 6,
    UTIL = 7,
    DEBUG = 8,
    APP = 9,
    APP_CNF = 15,
    GREENPOWER = 21,
}
