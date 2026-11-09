// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/constants/common.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

#![allow(non_camel_case_types)]

/// Upstream `AddressMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AddressMode {
    ADDR_NOT_PRESENT = 0,
    ADDR_GROUP = 1,
    ADDR_16BIT = 2,
    ADDR_64BIT = 3,
    ADDR_BROADCAST = 15,
}

/// Upstream `DevStates`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DevStates {
    HOLD = 0,
    INIT = 1,
    NWK_DISC = 2,
    NWK_JOINING = 3,
    NWK_REJOIN = 4,
    END_DEVICE_UNAUTH = 5,
    END_DEVICE = 6,
    ROUTER = 7,
    COORD_STARTING = 8,
    ZB_COORD = 9,
    NWK_ORPHAN = 10,
    INVALID_REQTYPE = 128,
    DEVICE_NOT_FOUND = 129,
    INVALID_EP = 130,
    NOT_ACTIVE = 131,
    NOT_SUPPORTED = 132,
    TIMEOUT = 133,
    NO_MATCH = 134,
    NO_ENTRY = 136,
    NO_DESCRIPTOR = 137,
    INSUFFICIENT_SPACE = 138,
    NOT_PERMITTED = 139,
    TABLE_FULL = 140,
    NOT_AUTHORIZED = 141,
    BINDING_TABLE_FULL = 142,
}

/// Upstream `NvItemsIds` (duplicate values, so constants instead of an enum).
pub struct NvItemsIds;
impl NvItemsIds {
    pub const EXTADDR: u16 = 1;
    pub const BOOTCOUNTER: u16 = 2;
    pub const STARTUP_OPTION: u16 = 3;
    pub const START_DELAY: u16 = 4;
    pub const NIB: u16 = 33;
    pub const DEVICE_LIST: u16 = 34;
    pub const ADDRMGR: u16 = 35;
    pub const POLL_RATE: u16 = 36;
    pub const QUEUED_POLL_RATE: u16 = 37;
    pub const RESPONSE_POLL_RATE: u16 = 38;
    pub const REJOIN_POLL_RATE: u16 = 39;
    pub const DATA_RETRIES: u16 = 40;
    pub const POLL_FAILURE_RETRIES: u16 = 41;
    pub const STACK_PROFILE: u16 = 42;
    pub const INDIRECT_MSG_TIMEOUT: u16 = 43;
    pub const ROUTE_EXPIRY_TIME: u16 = 44;
    pub const EXTENDED_PAN_ID: u16 = 45;
    pub const BCAST_RETRIES: u16 = 46;
    pub const PASSIVE_ACK_TIMEOUT: u16 = 47;
    pub const BCAST_DELIVERY_TIME: u16 = 48;
    pub const NWK_MODE: u16 = 49;
    pub const CONCENTRATOR_ENABLE: u16 = 50;
    pub const CONCENTRATOR_DISCOVERY: u16 = 51;
    pub const CONCENTRATOR_RADIUS: u16 = 52;
    pub const CONCENTRATOR_RC: u16 = 54;
    pub const NWK_MGR_MODE: u16 = 55;
    pub const SRC_RTG_EXPIRY_TIME: u16 = 56;
    pub const ROUTE_DISCOVERY_TIME: u16 = 57;
    pub const NWK_ACTIVE_KEY_INFO: u16 = 58;
    pub const NWK_ALTERN_KEY_INFO: u16 = 59;
    pub const ROUTER_OFF_ASSOC_CLEANUP: u16 = 60;
    pub const NWK_LEAVE_REQ_ALLOWED: u16 = 61;
    pub const NWK_CHILD_AGE_ENABLE: u16 = 62;
    pub const DEVICE_LIST_KA_TIMEOUT: u16 = 63;
    pub const BINDING_TABLE: u16 = 65;
    pub const GROUP_TABLE: u16 = 66;
    pub const APS_FRAME_RETRIES: u16 = 67;
    pub const APS_ACK_WAIT_DURATION: u16 = 68;
    pub const APS_ACK_WAIT_MULTIPLIER: u16 = 69;
    pub const BINDING_TIME: u16 = 70;
    pub const APS_USE_EXT_PANID: u16 = 71;
    pub const APS_USE_INSECURE_JOIN: u16 = 72;
    pub const COMMISSIONED_NWK_ADDR: u16 = 73;
    pub const APS_NONMEMBER_RADIUS: u16 = 75;
    pub const APS_LINK_KEY_TABLE: u16 = 76;
    pub const APS_DUPREJ_TIMEOUT_INC: u16 = 77;
    pub const APS_DUPREJ_TIMEOUT_COUNT: u16 = 78;
    pub const APS_DUPREJ_TABLE_SIZE: u16 = 79;
    pub const DIAGNOSTIC_STATS: u16 = 80;
    pub const BDBNODEISONANETWORK: u16 = 85;
    pub const SECURITY_LEVEL: u16 = 97;
    pub const PRECFGKEY: u16 = 98;
    pub const PRECFGKEYS_ENABLE: u16 = 99;
    pub const SECURITY_MODE: u16 = 100;
    pub const SECURE_PERMIT_JOIN: u16 = 101;
    pub const APS_LINK_KEY_TYPE: u16 = 102;
    pub const APS_ALLOW_R19_SECURITY: u16 = 103;
    pub const IMPLICIT_CERTIFICATE: u16 = 105;
    pub const DEVICE_PRIVATE_KEY: u16 = 106;
    pub const CA_PUBLIC_KEY: u16 = 107;
    pub const KE_MAX_DEVICES: u16 = 108;
    pub const USE_DEFAULT_TCLK: u16 = 109;
    pub const RNG_COUNTER: u16 = 111;
    pub const RANDOM_SEED: u16 = 112;
    pub const TRUSTCENTER_ADDR: u16 = 113;
    pub const LEGACY_NWK_SEC_MATERIAL_TABLE_START: u16 = 117;
    pub const EX_NWK_SEC_MATERIAL_TABLE: u16 = 7;
    pub const USERDESC: u16 = 129;
    pub const NWKKEY: u16 = 130;
    pub const PANID: u16 = 131;
    pub const CHANLIST: u16 = 132;
    pub const LEAVE_CTRL: u16 = 133;
    pub const SCAN_DURATION: u16 = 134;
    pub const LOGICAL_TYPE: u16 = 135;
    pub const NWKMGR_MIN_TX: u16 = 136;
    pub const NWKMGR_ADDR: u16 = 137;
    pub const ZDO_DIRECT_CB: u16 = 143;
    pub const SCENE_TABLE: u16 = 145;
    pub const MIN_FREE_NWK_ADDR: u16 = 146;
    pub const MAX_FREE_NWK_ADDR: u16 = 147;
    pub const MIN_FREE_GRP_ID: u16 = 148;
    pub const MAX_FREE_GRP_ID: u16 = 149;
    pub const MIN_GRP_IDS: u16 = 150;
    pub const MAX_GRP_IDS: u16 = 151;
    pub const OTA_BLOCK_REQ_DELAY: u16 = 152;
    pub const SAPI_ENDPOINT: u16 = 161;
    pub const SAS_SHORT_ADDR: u16 = 177;
    pub const SAS_EXT_PANID: u16 = 178;
    pub const SAS_PANID: u16 = 179;
    pub const SAS_CHANNEL_MASK: u16 = 180;
    pub const SAS_PROTOCOL_VER: u16 = 181;
    pub const SAS_STACK_PROFILE: u16 = 182;
    pub const SAS_STARTUP_CTRL: u16 = 183;
    pub const SAS_TC_ADDR: u16 = 193;
    pub const SAS_TC_MASTER_KEY: u16 = 194;
    pub const SAS_NWK_KEY: u16 = 195;
    pub const SAS_USE_INSEC_JOIN: u16 = 196;
    pub const SAS_PRECFG_LINK_KEY: u16 = 197;
    pub const SAS_NWK_KEY_SEQ_NUM: u16 = 198;
    pub const SAS_NWK_KEY_TYPE: u16 = 199;
    pub const SAS_NWK_MGR_ADDR: u16 = 200;
    pub const SAS_CURR_TC_MASTER_KEY: u16 = 209;
    pub const SAS_CURR_NWK_KEY: u16 = 210;
    pub const SAS_CURR_PRECFG_LINK_KEY: u16 = 211;
    pub const TCLK_SEED: u16 = 257;
    pub const LEGACY_TCLK_TABLE_START_12: u16 = 257;
    pub const LEGACY_TCLK_TABLE_START: u16 = 273;
    pub const EX_TCLK_TABLE: u16 = 4;
    pub const APS_LINK_KEY_DATA_START: u16 = 513;
    pub const APS_LINK_KEY_DATA_END: u16 = 767;
    pub const DUPLICATE_BINDING_TABLE: u16 = 768;
    pub const DUPLICATE_DEVICE_LIST: u16 = 769;
    pub const DUPLICATE_DEVICE_LIST_KA_TIMEOUT: u16 = 770;
    pub const ZNP_HAS_CONFIGURED_ZSTACK1: u16 = 3840;
    pub const ZNP_HAS_CONFIGURED_ZSTACK3: u16 = 96;
    pub const ZCD_NV_EX_APS_KEY_DATA_TABLE: u16 = 6;
    pub const ZCD_NV_EX_ADDRMGR: u16 = 1;
}

/// Upstream `NvSystemIds`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NvSystemIds {
    ZSTACK = 1,
}

/// Upstream `ZnpCommandStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ZnpCommandStatus {
    SUCCESS = 0,
    FAILURE = 1,
    INVALID_PARAM = 2,
    NV_ITEM_INITIALIZED = 9,
    NV_OPER_FAILED = 10,
    NV_BAD_ITEM_LEN = 12,
    MEM_ERROR = 16,
    BUFFER_FULL = 17,
    UNSUPPORTED_MODE = 18,
    MAC_MEM_ERROR = 19,
    MAC_UNSUPPORTED_NOT_SPOORT = 24,
    MAC_BAD_STATE = 25,
    MAC_NO_RESOURCES = 26,
    MAC_ACK_PENDING = 27,
    MAC_NO_TIME = 28,
    MAC_TX_ABORTED = 29,
    SAPI_IN_PROGRESS = 32,
    SAPI_TIMEOUT = 33,
    SAPI_INIT = 34,
    NOT_AUTHORIZED = 126,
    MALFORMED_CMD = 128,
    UNSUP_CLUSTER_CMD = 129,
    OTA_ABORT = 149,
    OTA_IMAGE_INVALID = 150,
    OTA_WAIT_FOR_DATA = 151,
    OTA_NO_IMAGE_AVAILABLE = 152,
    OTA_REQUIRE_MORE_IMAGE = 153,
    APS_FAIL = 177,
    APS_TABLE_FULL = 178,
    APS_ILLEGAL_REQUEST = 179,
    APS_INVALID_BINDING = 180,
    APS_UNSUPPORTED_ATTRIB = 181,
    APS_NOT_SUPPORTED = 182,
    APS_NO_ACK = 183,
    APS_DUPLICATE_ENTRY = 184,
    APS_NO_BOUND_DEVICE = 185,
    APS_NOT_ALLOWED = 186,
    APS_NOT_AUTHENTICATED = 187,
    SEC_NO_KEY = 161,
    SEC_OLD_FRM_COUNT = 162,
    SEC_MAX_FRM_COUNT = 163,
    SEC_CCM_FAIL = 164,
    NWK_INVALID_PARAM = 193,
    NWK_INVALID_REQUEST = 194,
    NWK_NOT_PERMITTED = 195,
    NWK_STARTUP_FAILURE = 196,
    NWK_ALREADY_PRESENT = 197,
    NWK_SYNC_FAILURE = 198,
    NWK_TABLE_FULL = 199,
    NWK_UNKNOWN_DEVICE = 200,
    NWK_UNSUPPORTED_ATTRIBUTE = 201,
    NWK_NO_NETWORKS = 202,
    NWK_LEAVE_UNCONFIRMED = 203,
    NWK_NO_ACK = 204,
    NWK_NO_ROUTE = 205,
    MAC_BEACON_LOSS = 224,
    MAC_CHANNEL_ACCESS_FAILURE = 225,
    MAC_DENIED = 226,
    MAC_DISABLE_TRX_FAILURE = 227,
    MAC_FAILED_SECURITY_CHECK = 228,
    MAC_FRAME_TOO_LONG = 229,
    MAC_INVALID_GTS = 230,
    MAC_INVALID_HANDLE = 231,
    MAC_INVALID_PARAMETER = 232,
    MAC_NO_ACK = 233,
    MAC_NO_BEACON = 234,
    MAC_NO_DATA = 235,
    MAC_NO_SHORT_ADDR = 236,
    MAC_OUT_OF_CAP = 237,
    MAC_PANID_CONFLICT = 238,
    MAC_REALIGNMENT = 239,
    MAC_TRANSACTION_EXPIRED = 240,
    MAC_TRANSACTION_OVERFLOW = 241,
    MAC_TX_ACTIVE = 242,
    MAC_UNAVAILABLE_KEY = 243,
    MAC_UNSUPPORTED_ATTRIBUTE = 244,
    MAC_UNSUPPORTED = 245,
    MAC_ON_TIME_TOO_LING = 246,
    MAC_PAST_TIME = 247,
    MAC_TRACKING_OFF = 248,
    MAC_SCAN_IN_PROGRESS = 252,
    MAC_SRC_MATCH_INVALID_INDEX = 255,
}

impl ZnpCommandStatus {
    /// Upstream numeric enum reverse mapping `ZnpCommandStatus[code]`.
    pub fn name(value: u8) -> &'static str {
        match value {
            0 => "SUCCESS",
            1 => "FAILURE",
            2 => "INVALID_PARAM",
            9 => "NV_ITEM_INITIALIZED",
            10 => "NV_OPER_FAILED",
            12 => "NV_BAD_ITEM_LEN",
            16 => "MEM_ERROR",
            17 => "BUFFER_FULL",
            18 => "UNSUPPORTED_MODE",
            19 => "MAC_MEM_ERROR",
            24 => "MAC_UNSUPPORTED_NOT_SPOORT",
            25 => "MAC_BAD_STATE",
            26 => "MAC_NO_RESOURCES",
            27 => "MAC_ACK_PENDING",
            28 => "MAC_NO_TIME",
            29 => "MAC_TX_ABORTED",
            32 => "SAPI_IN_PROGRESS",
            33 => "SAPI_TIMEOUT",
            34 => "SAPI_INIT",
            126 => "NOT_AUTHORIZED",
            128 => "MALFORMED_CMD",
            129 => "UNSUP_CLUSTER_CMD",
            149 => "OTA_ABORT",
            150 => "OTA_IMAGE_INVALID",
            151 => "OTA_WAIT_FOR_DATA",
            152 => "OTA_NO_IMAGE_AVAILABLE",
            153 => "OTA_REQUIRE_MORE_IMAGE",
            177 => "APS_FAIL",
            178 => "APS_TABLE_FULL",
            179 => "APS_ILLEGAL_REQUEST",
            180 => "APS_INVALID_BINDING",
            181 => "APS_UNSUPPORTED_ATTRIB",
            182 => "APS_NOT_SUPPORTED",
            183 => "APS_NO_ACK",
            184 => "APS_DUPLICATE_ENTRY",
            185 => "APS_NO_BOUND_DEVICE",
            186 => "APS_NOT_ALLOWED",
            187 => "APS_NOT_AUTHENTICATED",
            161 => "SEC_NO_KEY",
            162 => "SEC_OLD_FRM_COUNT",
            163 => "SEC_MAX_FRM_COUNT",
            164 => "SEC_CCM_FAIL",
            193 => "NWK_INVALID_PARAM",
            194 => "NWK_INVALID_REQUEST",
            195 => "NWK_NOT_PERMITTED",
            196 => "NWK_STARTUP_FAILURE",
            197 => "NWK_ALREADY_PRESENT",
            198 => "NWK_SYNC_FAILURE",
            199 => "NWK_TABLE_FULL",
            200 => "NWK_UNKNOWN_DEVICE",
            201 => "NWK_UNSUPPORTED_ATTRIBUTE",
            202 => "NWK_NO_NETWORKS",
            203 => "NWK_LEAVE_UNCONFIRMED",
            204 => "NWK_NO_ACK",
            205 => "NWK_NO_ROUTE",
            224 => "MAC_BEACON_LOSS",
            225 => "MAC_CHANNEL_ACCESS_FAILURE",
            226 => "MAC_DENIED",
            227 => "MAC_DISABLE_TRX_FAILURE",
            228 => "MAC_FAILED_SECURITY_CHECK",
            229 => "MAC_FRAME_TOO_LONG",
            230 => "MAC_INVALID_GTS",
            231 => "MAC_INVALID_HANDLE",
            232 => "MAC_INVALID_PARAMETER",
            233 => "MAC_NO_ACK",
            234 => "MAC_NO_BEACON",
            235 => "MAC_NO_DATA",
            236 => "MAC_NO_SHORT_ADDR",
            237 => "MAC_OUT_OF_CAP",
            238 => "MAC_PANID_CONFLICT",
            239 => "MAC_REALIGNMENT",
            240 => "MAC_TRANSACTION_EXPIRED",
            241 => "MAC_TRANSACTION_OVERFLOW",
            242 => "MAC_TX_ACTIVE",
            243 => "MAC_UNAVAILABLE_KEY",
            244 => "MAC_UNSUPPORTED_ATTRIBUTE",
            245 => "MAC_UNSUPPORTED",
            246 => "MAC_ON_TIME_TOO_LING",
            247 => "MAC_PAST_TIME",
            248 => "MAC_TRACKING_OFF",
            252 => "MAC_SCAN_IN_PROGRESS",
            255 => "MAC_SRC_MATCH_INVALID_INDEX",
            _ => "UNKNOWN",
        }
    }
}

/// Upstream `CapabInfoMask`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CapabInfoMask {
    ALTPANCOORD = 1,
    DEVICETYPE_FFD = 2,
    POWER_AC = 4,
    RCVR_ON_IDLE = 8,
    SECURITY_CAPABLE = 64,
    ALLOC_ADDR = 128,
}

/// Upstream `SecurityLevel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SecurityLevel {
    NONE = 0,
    MIC_32 = 1,
    MIC_64 = 2,
    MIC_128 = 3,
    ENC = 4,
    ENC_MIC_32 = 5,
    ENC_MIC_64 = 6,
    ENC_MIC_128 = 7,
}

/// Upstream `ScanDuration`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ScanDuration {
    CH_CHANGE_REQ = 254,
    CH_MASK_CHANGE_REQ = 255,
    T_NO_BEACONS = 15,
    T_4_MINUTES = 14,
    T_2_MINUTES = 13,
    T_1_MINUTE = 12,
    T_31_SECONDS = 11,
    T_15_SECONDS = 10,
    T_7_5_SECONDS = 9,
    T_4_SECONDS = 8,
    T_2_SECONDS = 7,
    T_1_SECOND = 6,
    T_480_MSEC = 5,
    T_240_MSEC = 4,
    T_120_MSEC = 3,
    T_60_MSEC = 2,
    T_30_MSEC = 1,
    T_15_MSEC = 0,
}
