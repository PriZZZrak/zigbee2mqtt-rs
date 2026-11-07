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
    pub const PARENT_ANNOUNCE: u16 = 0x001f;
    pub const BIND_REQUEST: u16 = 0x0021;
    pub const UNBIND_REQUEST: u16 = 0x0022;
    pub const CLEAR_ALL_BINDINGS_REQUEST: u16 = 0x002b;
    pub const LQI_TABLE_REQUEST: u16 = 0x0031;
    pub const ROUTING_TABLE_REQUEST: u16 = 0x0032;
    pub const BINDING_TABLE_REQUEST: u16 = 0x0033;
    pub const LEAVE_REQUEST: u16 = 0x0034;
    pub const PERMIT_JOINING_REQUEST: u16 = 0x0036;
    pub const NWK_UPDATE_REQUEST: u16 = 0x0038;
    pub const NWK_ENHANCED_UPDATE_REQUEST: u16 = 0x0039;
    pub const NWK_IEEE_JOINING_LIST_REQUEST: u16 = 0x003a;
    pub const NWK_BEACON_SURVEY_REQUEST: u16 = 0x003c;
    pub const START_KEY_NEGOTIATION_REQUEST: u16 = 0x0040;
    pub const DECOMMISSION_REQUEST: u16 = 0x0046;
    pub const CHALLENGE_REQUEST: u16 = 0x0047;
}
