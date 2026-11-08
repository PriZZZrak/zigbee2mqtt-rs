// Original: .upstream/zigbee-herdsman/src/adapter/ezsp/driver/types/basic.ts

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EzspStatus {
    SUCCESS,
    ASH_ACK_TIMEOUT,
    ASH_BAD_CRC,
    HOST_FATAL_ERROR,
    NO_RX_DATA,
}
// Original: .upstream/zigbee-herdsman/src/adapter/ezsp/driver/types/basic.ts
