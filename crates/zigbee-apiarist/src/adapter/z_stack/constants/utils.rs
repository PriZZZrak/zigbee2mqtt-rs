// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/constants/utils.ts
// TODO: port remaining helpers.

pub fn get_channel_mask(channels: &[u8]) -> Vec<u8> {
    let value = channels
        .iter()
        .fold(0u32, |mask, channel| mask | (1u32 << u32::from(*channel)));
    vec![
        (value & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        ((value >> 16) & 0xff) as u8,
        ((value >> 24) & 0xff) as u8,
    ]
}

/// Upstream `ZnpCommandStatus` names.
const STATUS_NAMES: [(u8, &str); 40] = [
    (0x00, "SUCCESS"),
    (0x01, "FAILURE"),
    (0x02, "INVALID_PARAM"),
    (0x09, "NV_ITEM_INITIALIZED"),
    (0x0a, "NV_OPER_FAILED"),
    (0x0c, "NV_BAD_ITEM_LEN"),
    (0x10, "MEM_ERROR"),
    (0x11, "BUFFER_FULL"),
    (0x12, "UNSUPPORTED_MODE"),
    (0x13, "MAC_MEM_ERROR"),
    (0x18, "MAC_UNSUPPORTED_NOT_SPOORT"),
    (0x19, "MAC_BAD_STATE"),
    (0x1a, "MAC_NO_RESOURCES"),
    (0x1b, "MAC_ACK_PENDING"),
    (0x1c, "MAC_NO_TIME"),
    (0x1d, "MAC_TX_ABORTED"),
    (0x20, "SAPI_IN_PROGRESS"),
    (0x21, "SAPI_TIMEOUT"),
    (0x22, "SAPI_INIT"),
    (0x7e, "NOT_AUTHORIZED"),
    (0x80, "MALFORMED_CMD"),
    (0x81, "UNSUP_CLUSTER_CMD"),
    (0x95, "OTA_ABORT"),
    (0x96, "OTA_IMAGE_INVALID"),
    (0x97, "OTA_WAIT_FOR_DATA"),
    (0x98, "OTA_NO_IMAGE_AVAILABLE"),
    (0x99, "OTA_REQUIRE_MORE_IMAGE"),
    (0xa1, "SEC_NO_KEY"),
    (0xa2, "SEC_OLD_FRM_COUNT"),
    (0xb1, "APS_FAIL"),
    (0xb2, "APS_TABLE_FULL"),
    (0xb3, "APS_ILLEGAL_REQUEST"),
    (0xb4, "APS_INVALID_BINDING"),
    (0xb5, "APS_UNSUPPORTED_ATTRIB"),
    (0xb6, "APS_NOT_SUPPORTED"),
    (0xb7, "APS_NO_ACK"),
    (0xb8, "APS_DUPLICATE_ENTRY"),
    (0xb9, "APS_NO_BOUND_DEVICE"),
    (0xba, "APS_NOT_ALLOWED"),
    (0xbb, "APS_NOT_AUTHENTICATED"),
];

pub fn status_description(status: u8) -> String {
    let name = STATUS_NAMES
        .iter()
        .find_map(|(code, name)| (*code == status).then_some(*name))
        .unwrap_or("UNKNOWN");
    format!("(0x{status:02x}: {name})")
}
