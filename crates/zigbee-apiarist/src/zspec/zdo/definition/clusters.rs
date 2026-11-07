// Original: .upstream/zigbee-herdsman/src/zspec/zdo/definition/clusters.ts
// TODO: port the full ClusterId enum and request/response maps.

pub struct ClusterId;

impl ClusterId {
    pub const NETWORK_ADDRESS_REQUEST: u16 = 0x0000;
    pub const IEEE_ADDRESS_REQUEST: u16 = 0x0001;
    pub const NODE_DESCRIPTOR_REQUEST: u16 = 0x0002;
    pub const POWER_DESCRIPTOR_REQUEST: u16 = 0x0003;
    pub const SIMPLE_DESCRIPTOR_REQUEST: u16 = 0x0004;
    pub const ACTIVE_ENDPOINTS_REQUEST: u16 = 0x0005;
    pub const MATCH_DESCRIPTORS_REQUEST: u16 = 0x0006;
    pub const SYSTEM_SERVER_DISCOVERY_REQUEST: u16 = 0x0015;
}
