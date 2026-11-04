//! Tuya manufacturer cluster (0xEF00) command payloads.
//!
//! Unlike attribute reports, Tuya datapoints arrive as cluster-specific
//! commands whose payload is `seq:u16` followed by repeated
//! `dp:u8, datatype:u8, length:u16(BE), data[len]` records. Herdsman parses
//! this into `msg.data = {seq, dpValues:[{dp, datatype, data}]}` and upstream
//! z2m derives the converter message type from the command name
//! (`commandDataReport`, `commandDataResponse`, ...).
//!
//! Only the commands that carry datapoints are modelled here. Other Tuya
//! commands (time sync, OTA, weather) keep their raw payloads for now.

pub const CLUSTER_ID: u16 = 0xEF00;

pub const CMD_DATA_REQUEST: u8 = 0x00;
pub const CMD_DATA_RESPONSE: u8 = 0x01;
pub const CMD_DATA_REPORT: u8 = 0x02;
pub const CMD_SEND_DATA: u8 = 0x04;
pub const CMD_ACTIVE_STATUS_REPORT_ALT: u8 = 0x05;
pub const CMD_ACTIVE_STATUS_REPORT: u8 = 0x06;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataPoint {
    pub dp: u8,
    pub datatype: u8,
    pub data: Vec<u8>,
}

/// Converter message type for a Tuya command id, matching upstream
/// `command${capitalize(command.name)}`.
#[must_use]
pub fn message_type(command_id: u8) -> Option<&'static str> {
    Some(match command_id {
        CMD_DATA_REQUEST => "commandDataRequest",
        CMD_DATA_RESPONSE => "commandDataResponse",
        CMD_DATA_REPORT => "commandDataReport",
        CMD_SEND_DATA => "commandSendData",
        CMD_ACTIVE_STATUS_REPORT_ALT => "commandActiveStatusReportAlt",
        CMD_ACTIVE_STATUS_REPORT => "commandActiveStatusReport",
        _ => return None,
    })
}

/// Parse `seq:u16` plus datapoint records.
///
/// A truncated record is an error rather than a silently dropped tail: the
/// caller logs it and converts nothing, so a malformed frame cannot produce a
/// partially wrong published state.
pub fn parse_datapoints(payload: &[u8]) -> Result<(u16, Vec<DataPoint>), String> {
    if payload.len() < 2 {
        return Err(format!(
            "Tuya command payload is {} bytes, needs at least the 2-byte sequence",
            payload.len()
        ));
    }
    let sequence = u16::from_le_bytes([payload[0], payload[1]]);
    let mut points = Vec::new();
    let mut offset = 2;
    while offset < payload.len() {
        if payload.len() - offset < 4 {
            return Err(format!(
                "truncated Tuya datapoint header at offset {offset} ({} bytes left)",
                payload.len() - offset
            ));
        }
        let dp = payload[offset];
        let datatype = payload[offset + 1];
        let length = u16::from_be_bytes([payload[offset + 2], payload[offset + 3]]) as usize;
        offset += 4;
        if payload.len() - offset < length {
            return Err(format!(
                "truncated Tuya datapoint {dp} (datatype 0x{datatype:02X}): needs {length} bytes, {} left",
                payload.len() - offset
            ));
        }
        points.push(DataPoint {
            dp,
            datatype,
            data: payload[offset..offset + length].to_vec(),
        });
        offset += length;
    }
    Ok((sequence, points))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multiple_tuya_datapoints() {
        let payload = [
            0x34, 0x12, // sequence 0x1234
            0x03, 0x02, 0x00, 0x02, 0x00, 0x8C, // dp 3, value 2, len 2, 140
            0x08, 0x01, 0x00, 0x01, 0x01, // dp 8, bool, len 1, true
        ];
        let (sequence, points) = parse_datapoints(&payload).expect("valid payload");
        assert_eq!(sequence, 0x1234);
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].dp, 3);
        assert_eq!(points[0].datatype, 2);
        assert_eq!(points[0].data, vec![0x00, 0x8C]);
        assert_eq!(points[1].dp, 8);
        assert_eq!(points[1].data, vec![1]);
    }

    #[test]
    fn sequence_only_payload_has_no_datapoints() {
        let (sequence, points) = parse_datapoints(&[0x01, 0x00]).expect("valid");
        assert_eq!(sequence, 1);
        assert!(points.is_empty());
    }

    #[test]
    fn truncated_header_and_data_are_errors() {
        let error = parse_datapoints(&[0x00]).unwrap_err();
        assert!(error.contains("sequence"), "{error}");

        let error = parse_datapoints(&[0x00, 0x00, 0x03, 0x02]).unwrap_err();
        assert!(error.contains("truncated Tuya datapoint header"), "{error}");

        let error = parse_datapoints(&[0x00, 0x00, 0x03, 0x02, 0x00, 0x04, 0x01]).unwrap_err();
        assert!(error.contains("truncated Tuya datapoint 3"), "{error}");
    }

    #[test]
    fn message_types_match_upstream_command_names() {
        assert_eq!(message_type(CMD_DATA_REQUEST), Some("commandDataRequest"));
        assert_eq!(message_type(CMD_DATA_RESPONSE), Some("commandDataResponse"));
        assert_eq!(message_type(CMD_DATA_REPORT), Some("commandDataReport"));
        assert_eq!(message_type(CMD_SEND_DATA), Some("commandSendData"));
        assert_eq!(
            message_type(CMD_ACTIVE_STATUS_REPORT_ALT),
            Some("commandActiveStatusReportAlt")
        );
        assert_eq!(
            message_type(CMD_ACTIVE_STATUS_REPORT),
            Some("commandActiveStatusReport")
        );
        assert_eq!(message_type(0x10), None);
        assert_eq!(CLUSTER_ID, 0xEF00);
    }
}
