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

impl Type {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::POLL),
            1 => Some(Self::SREQ),
            2 => Some(Self::AREQ),
            3 => Some(Self::SRSP),
            _ => None,
        }
    }
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

impl Subsystem {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::RESERVED),
            1 => Some(Self::SYS),
            2 => Some(Self::MAC),
            3 => Some(Self::NWK),
            4 => Some(Self::AF),
            5 => Some(Self::ZDO),
            6 => Some(Self::SAPI),
            7 => Some(Self::UTIL),
            8 => Some(Self::DEBUG),
            9 => Some(Self::APP),
            15 => Some(Self::APP_CNF),
            21 => Some(Self::GREENPOWER),
            _ => None,
        }
    }
}
