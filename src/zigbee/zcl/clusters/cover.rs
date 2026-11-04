/// ZCL Window Covering cluster (0x0102) command and attribute builders.
///
/// Generic mapping used when a device exposes the covering cluster: state
/// commands are UpOpen/DownClose/Stop, position/tilt are writes of the
/// corresponding percentage attributes (0–100).
use crate::zigbee::zcl::frame::{global, ZclFrameHeader};
use crate::zigbee::zcl::write::write_attributes_payload;

pub const CLUSTER_ID: u16 = 0x0102;
pub const CMD_UP_OPEN: u8 = 0x00;
pub const CMD_DOWN_CLOSE: u8 = 0x01;
pub const CMD_STOP: u8 = 0x02;

pub const ATTR_CURRENT_POSITION_LIFT: u16 = 0x0008;
pub const ATTR_CURRENT_POSITION_TILT: u16 = 0x0009;

/// Build a cluster-specific Window Covering command payload for a z2m state.
///
/// `OPEN` -> UpOpen, `CLOSE` -> DownClose, `STOP` -> Stop. Other values are
/// rejected instead of being silently mapped to a motion.
#[must_use]
pub fn state_payload(sequence: u8, state: &str) -> Option<Vec<u8>> {
    let command = match state {
        "OPEN" | "open" => CMD_UP_OPEN,
        "CLOSE" | "close" => CMD_DOWN_CLOSE,
        "STOP" | "stop" => CMD_STOP,
        _ => return None,
    };
    Some(vec![0x01, sequence, command])
}

fn percentage(value: u64) -> u8 {
    value.min(100) as u8
}

/// Write `currentPositionLiftPercentage` (0–100).
#[must_use]
pub fn position_payload(sequence: u8, position: u64) -> Vec<u8> {
    write_attributes_payload(
        sequence,
        None,
        &[(ATTR_CURRENT_POSITION_LIFT, 0x20, &[percentage(position)])],
    )
}

/// Write `currentPositionTiltPercentage` (0–100).
#[must_use]
pub fn tilt_payload(sequence: u8, tilt: u64) -> Vec<u8> {
    write_attributes_payload(
        sequence,
        None,
        &[(ATTR_CURRENT_POSITION_TILT, 0x20, &[percentage(tilt)])],
    )
}

/// True when the ZCL global command is a Write Attributes request (used by
/// position/tilt, which have no dedicated response payload).
#[must_use]
pub fn is_write_attributes(command_id: u8) -> bool {
    command_id == global::WRITE_ATTRIBUTES
}

/// Keep the frame header import used by callers that build cover frames.
#[must_use]
pub fn global_frame(sequence: u8, command: u8) -> Vec<u8> {
    ZclFrameHeader::encode_global(sequence, command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_payload_maps_open_close_stop() {
        assert_eq!(state_payload(7, "OPEN"), Some(vec![0x01, 7, CMD_UP_OPEN]));
        assert_eq!(
            state_payload(7, "CLOSE"),
            Some(vec![0x01, 7, CMD_DOWN_CLOSE])
        );
        assert_eq!(state_payload(7, "STOP"), Some(vec![0x01, 7, CMD_STOP]));
        assert_eq!(state_payload(7, "TOGGLE"), None);
    }

    #[test]
    fn position_and_tilt_write_percentage_attributes() {
        assert_eq!(
            position_payload(9, 150),
            vec![0x10, 9, global::WRITE_ATTRIBUTES, 0x08, 0x00, 0x20, 100]
        );
        assert_eq!(
            tilt_payload(9, 40),
            vec![0x10, 9, global::WRITE_ATTRIBUTES, 0x09, 0x00, 0x20, 40]
        );
        assert!(is_write_attributes(global::WRITE_ATTRIBUTES));
        assert!(!is_write_attributes(global::READ_ATTRIBUTES));
    }
}
