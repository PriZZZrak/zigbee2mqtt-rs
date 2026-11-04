/// ZCL Door Lock cluster (0x0101) command builders.
///
/// `state` follows the z2m lock converter: `LOCK` -> Lock Door (0x00),
/// `UNLOCK` -> Unlock Door (0x01). Other values are rejected.
use crate::zigbee::zcl::frame::ZclFrameHeader;

pub const CLUSTER_ID: u16 = 0x0101;
pub const CMD_LOCK_DOOR: u8 = 0x00;
pub const CMD_UNLOCK_DOOR: u8 = 0x01;
pub const RSP_LOCK_DOOR: u8 = 0x00;
pub const RSP_UNLOCK_DOOR: u8 = 0x01;

#[must_use]
pub fn state_payload(sequence: u8, state: &str) -> Option<Vec<u8>> {
    let command = match state {
        "LOCK" | "lock" => CMD_LOCK_DOOR,
        "UNLOCK" | "unlock" => CMD_UNLOCK_DOOR,
        _ => return None,
    };
    Some(vec![0x01, sequence, command])
}

/// Parse a Lock Door / Unlock Door response payload (`status`).
#[must_use]
pub fn parse_response(raw: &[u8]) -> Option<(u8, u8)> {
    let (header, offset) = ZclFrameHeader::parse(raw).ok()?;
    if header.command_id != RSP_LOCK_DOOR && header.command_id != RSP_UNLOCK_DOOR {
        return None;
    }
    Some((header.command_id, raw.get(offset).copied().unwrap_or(0xFF)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_payload_maps_lock_and_unlock() {
        assert_eq!(state_payload(5, "LOCK"), Some(vec![0x01, 5, CMD_LOCK_DOOR]));
        assert_eq!(
            state_payload(5, "UNLOCK"),
            Some(vec![0x01, 5, CMD_UNLOCK_DOOR])
        );
        assert_eq!(state_payload(5, "TOGGLE"), None);
    }

    #[test]
    fn parses_lock_response_status() {
        assert_eq!(parse_response(&[0x18, 5, 0x00, 0x00]), Some((0x00, 0x00)));
        assert_eq!(parse_response(&[0x18, 5, 0x01, 0x01]), Some((0x01, 0x01)));
        assert_eq!(parse_response(&[0x18, 5, 0x02, 0x00]), None);
    }
}
