// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/constants/utils.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use super::common::ZnpCommandStatus;

/// Upstream `getChannelMask(channels)`.
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

/// Upstream `statusDescription(code)`.
pub fn status_description(status: u8) -> String {
    format!("(0x{status:02x}: {})", ZnpCommandStatus::name(status))
}

/// Upstream `DataConfirmError` message.
///
/// Anchor: src/adapter/z-stack/adapter/zStackAdapter.ts:29-39.
pub fn data_confirm_error(status: u16) -> String {
    if status == 9999 {
        "Data request failed with error: 'TIMEOUT'".to_string()
    } else {
        format!(
            "Data request failed with error: '{}' (0x{:x})",
            ZnpCommandStatus::name(status as u8),
            status
        )
    }
}
