// Original: .upstream/zigbee-herdsman/src/adapter/ember/enums.ts
// TODO: port public API from the original file.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum SLStatus {
    OK = 0x0000,
    INVALID_PARAMETER = 0x0021,
    BUS_ERROR = 0x004b,
    ZIGBEE_NETWORK_OPENED = 0x0c18,
    ZIGBEE_EZSP_ERROR = 0x0c1e,
}
