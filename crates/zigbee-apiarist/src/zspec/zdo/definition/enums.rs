// Original: .upstream/zigbee-herdsman/src/zspec/zdo/definition/enums.ts
// TODO: port public API from the original file.

pub struct GlobalTLV;

impl GlobalTLV {
    pub const MANUFACTURER_SPECIFIC: u8 = 64;
    pub const SUPPORTED_KEY_NEGOTIATION_METHODS: u8 = 65;
    pub const PAN_ID_CONFLICT_REPORT: u8 = 66;
    pub const NEXT_PAN_ID_CHANGE: u8 = 67;
    pub const NEXT_CHANNEL_CHANGE: u8 = 68;
    pub const SYMMETRIC_PASSPHRASE: u8 = 69;
    pub const ROUTER_INFORMATION: u8 = 70;
    pub const FRAGMENTATION_PARAMETERS: u8 = 71;
    pub const JOINER_ENCAPSULATION: u8 = 72;
    pub const BEACON_APPENDIX_ENCAPSULATION: u8 = 73;
    pub const CONFIGURATION_PARAMETERS: u8 = 75;
    pub const DEVICE_CAPABILITY_EXTENSION: u8 = 76;
}

impl Default for GlobalTLV {
    fn default() -> Self {
        Self
    }
}

pub struct JoiningPolicy;

impl JoiningPolicy {
    pub const ALL_JOIN: u8 = 0x00;
    pub const IEEELIST_JOIN: u8 = 0x01;
    pub const NO_JOIN: u8 = 0x02;
}

pub struct InitialJoinMethod;

impl InitialJoinMethod {
    pub const INSTALL_CODE_KEY: u8 = 0x01;
    pub const WELLKNOWN_PASSPHRASE: u8 = 0x02;
    pub const INSTALL_CODE_PASSPHRASE: u8 = 0x03;
}

pub struct ActiveLinkKeyType;

impl ActiveLinkKeyType {
    pub const UNAUTHENTICATED_KEY_NEGOTIATION: u8 = 0x02;
    pub const AUTHENTICATED_KEY_NEGOTIATION: u8 = 0x03;
}

pub struct RoutingTableStatus;

impl RoutingTableStatus {
    pub const ACTIVE: u8 = 0x0;
    pub const DISCOVERY_UNDERWAY: u8 = 0x1;
    pub const DISCOVERY_FAILED: u8 = 0x2;
    pub const INACTIVE: u8 = 0x3;
    pub const VALIDATION_UNDERWAY: u8 = 0x4;
}
