// Original: .upstream/zigbee-herdsman/src/zspec/zcl/definition/enums.ts
// TODO: port the full DataType/BuffaloZclDataType enums.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DataType {
    NO_DATA = 0x00,
    OCTET_STR = 0x41,
    CHAR_STR = 0x42,
    LONG_OCTET_STR = 0x43,
    LONG_CHAR_STR = 0x44,
    UNKNOWN = 0xff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum BuffaloZclDataType {
    USE_DATA_TYPE = 1000,
    LIST_UINT8 = 1001,
    LIST_UINT16 = 1002,
    LIST_UINT24 = 1003,
    LIST_UINT32 = 1004,
    LIST_ZONEINFO = 1005,
    EXTENSION_FIELD_SETS = 1006,
    LIST_THERMO_TRANSITIONS = 1007,
    BUFFER = 1008,
    GPD_FRAME = 1009,
    STRUCTURED_SELECTOR = 1010,
    LIST_TUYA_DATAPOINT_VALUES = 1011,
    LIST_MIBOXER_ZONES = 1012,
    BIG_ENDIAN_UINT24 = 1013,
    MI_STRUCT = 1014,
}
