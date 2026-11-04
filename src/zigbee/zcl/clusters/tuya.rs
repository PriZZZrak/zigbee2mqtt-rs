//! Tuya manufacturer cluster (0xEF00) command payloads.
//!
//! Tuya frames are cluster-specific commands, not attribute reports. Herdsman
//! parses each command according to the `manuSpecificTuya` definition and z2m
//! derives the converter message type from the command name
//! (`commandDataReport`, `commandMcuSyncTime`, ...).
//!
//! Datapoint commands and the small metadata commands (MCU version, time sync,
//! gateway status, weather request/sync) are structured here. The MCU OTA
//! commands are recognised but deliberately rejected: their payloads carry
//! image chunks and need OTA state that this bridge does not model.

pub const CLUSTER_ID: u16 = 0xEF00;

pub const CMD_DATA_REQUEST: u8 = 0x00;
pub const CMD_DATA_RESPONSE: u8 = 0x01;
pub const CMD_DATA_REPORT: u8 = 0x02;
pub const CMD_SEND_DATA: u8 = 0x04;
pub const CMD_ACTIVE_STATUS_REPORT_ALT: u8 = 0x05;
pub const CMD_ACTIVE_STATUS_REPORT: u8 = 0x06;
pub const CMD_MCU_VERSION_RESPONSE: u8 = 0x11;
pub const CMD_MCU_OTA_NOTIFY: u8 = 0x12;
pub const CMD_MCU_OTA_BLOCK_REQUEST: u8 = 0x13;
pub const CMD_MCU_OTA_BLOCK_RESPONSE: u8 = 0x14;
pub const CMD_MCU_SYNC_TIME: u8 = 0x24;
pub const CMD_MCU_GATEWAY_CONNECTION_STATUS: u8 = 0x25;
pub const CMD_WEATHER_REQUEST: u8 = 0x60;
pub const CMD_WEATHER_SYNC: u8 = 0x61;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataPoint {
    pub dp: u8,
    pub datatype: u8,
    pub data: Vec<u8>,
}

/// Structured payload of a `manuSpecificTuya` command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandPayload {
    /// `seq` plus datapoint records (dataRequest/Response/Report/sendData/...).
    DataPoints {
        sequence: u16,
        points: Vec<DataPoint>,
    },
    /// `mcuVersionResponse`: sequence + MCU version byte.
    McuVersion { sequence: u16, version: u8 },
    /// `mcuSyncTime`: payload size + opaque payload bytes.
    TimeSync { payload_size: u16, payload: Vec<u8> },
    /// `mcuGatewayConnectionStatus`: payload size + one status byte.
    GatewayStatus { payload_size: u16, payload: u8 },
    /// `tuyaWeatherRequest` / `tuyaWeatherSync`: opaque weather buffer.
    Weather { payload: Vec<u8> },
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
        CMD_MCU_VERSION_RESPONSE => "commandMcuVersionResponse",
        CMD_MCU_OTA_NOTIFY => "commandMcuOtaNotify",
        CMD_MCU_OTA_BLOCK_REQUEST => "commandMcuOtaBlockDataRequest",
        CMD_MCU_OTA_BLOCK_RESPONSE => "commandMcuOtaBlockDataResponse",
        CMD_MCU_SYNC_TIME => "commandMcuSyncTime",
        CMD_MCU_GATEWAY_CONNECTION_STATUS => "commandMcuGatewayConnectionStatus",
        CMD_WEATHER_REQUEST => "commandTuyaWeatherRequest",
        CMD_WEATHER_SYNC => "commandTuyaWeatherSync",
        _ => return None,
    })
}

/// OTA commands are recognised, but their image-chunk payloads are not
/// structured here. Callers must skip them explicitly instead of treating the
/// frame as an unknown command.
#[must_use]
pub fn is_ota(command_id: u8) -> bool {
    matches!(
        command_id,
        CMD_MCU_OTA_NOTIFY | CMD_MCU_OTA_BLOCK_REQUEST | CMD_MCU_OTA_BLOCK_RESPONSE
    )
}

fn require(payload: &[u8], length: usize, what: &str) -> Result<(), String> {
    if payload.len() < length {
        return Err(format!(
            "Tuya {what} payload is {} bytes, needs at least {length}",
            payload.len()
        ));
    }
    Ok(())
}

fn parse_datapoints(payload: &[u8]) -> Result<CommandPayload, String> {
    require(payload, 2, "datapoint")?;
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
    Ok(CommandPayload::DataPoints { sequence, points })
}

/// Parse one command payload into the same `msg.data` shape herdsman produces.
///
/// Returns `Ok(None)` for commands that have no structured payload yet, an
/// explicit error for malformed frames and for recognised-but-rejected OTA
/// commands.
pub fn parse_command(command_id: u8, payload: &[u8]) -> Result<Option<CommandPayload>, String> {
    if is_ota(command_id) {
        return Err(format!(
            "Tuya OTA command 0x{command_id:02X} is not structured (image chunks are handled by the OTA manager, not the converter)"
        ));
    }
    match command_id {
        CMD_DATA_REQUEST
        | CMD_DATA_RESPONSE
        | CMD_DATA_REPORT
        | CMD_SEND_DATA
        | CMD_ACTIVE_STATUS_REPORT_ALT
        | CMD_ACTIVE_STATUS_REPORT => parse_datapoints(payload).map(Some),
        CMD_MCU_VERSION_RESPONSE => {
            require(payload, 3, "mcuVersionResponse")?;
            Ok(Some(CommandPayload::McuVersion {
                sequence: u16::from_le_bytes([payload[0], payload[1]]),
                version: payload[2],
            }))
        }
        CMD_MCU_SYNC_TIME => {
            require(payload, 2, "mcuSyncTime")?;
            Ok(Some(CommandPayload::TimeSync {
                payload_size: u16::from_le_bytes([payload[0], payload[1]]),
                payload: payload[2..].to_vec(),
            }))
        }
        CMD_MCU_GATEWAY_CONNECTION_STATUS => {
            require(payload, 3, "mcuGatewayConnectionStatus")?;
            Ok(Some(CommandPayload::GatewayStatus {
                payload_size: u16::from_le_bytes([payload[0], payload[1]]),
                payload: payload[2],
            }))
        }
        CMD_WEATHER_REQUEST | CMD_WEATHER_SYNC => Ok(Some(CommandPayload::Weather {
            payload: payload.to_vec(),
        })),
        _ => Ok(None),
    }
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
        let parsed = parse_command(CMD_DATA_REPORT, &payload)
            .expect("valid payload")
            .expect("structured");
        let CommandPayload::DataPoints { sequence, points } = parsed else {
            panic!("expected datapoints, got {parsed:?}");
        };
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
        let parsed = parse_command(CMD_DATA_RESPONSE, &[0x01, 0x00])
            .expect("valid")
            .expect("structured");
        assert_eq!(
            parsed,
            CommandPayload::DataPoints {
                sequence: 1,
                points: vec![]
            }
        );
    }

    #[test]
    fn parses_mcu_version_time_sync_gateway_and_weather() {
        assert_eq!(
            parse_command(CMD_MCU_VERSION_RESPONSE, &[0x34, 0x12, 0x45]).unwrap(),
            Some(CommandPayload::McuVersion {
                sequence: 0x1234,
                version: 0x45
            })
        );
        assert_eq!(
            parse_command(CMD_MCU_SYNC_TIME, &[0x08, 0x00, 1, 2, 3, 4, 5, 6, 7, 8]).unwrap(),
            Some(CommandPayload::TimeSync {
                payload_size: 8,
                payload: vec![1, 2, 3, 4, 5, 6, 7, 8]
            })
        );
        assert_eq!(
            parse_command(CMD_MCU_GATEWAY_CONNECTION_STATUS, &[0x01, 0x00, 0x00]).unwrap(),
            Some(CommandPayload::GatewayStatus {
                payload_size: 1,
                payload: 0
            })
        );
        assert_eq!(
            parse_command(CMD_WEATHER_REQUEST, &[0x12, 0x05]).unwrap(),
            Some(CommandPayload::Weather {
                payload: vec![0x12, 0x05]
            })
        );
    }

    #[test]
    fn truncated_and_ota_commands_are_errors() {
        let error = parse_command(CMD_DATA_REPORT, &[0x00]).unwrap_err();
        assert!(error.contains("datapoint"), "{error}");

        let error = parse_command(CMD_MCU_VERSION_RESPONSE, &[0x00, 0x00]).unwrap_err();
        assert!(error.contains("mcuVersionResponse"), "{error}");

        for command in [
            CMD_MCU_OTA_NOTIFY,
            CMD_MCU_OTA_BLOCK_REQUEST,
            CMD_MCU_OTA_BLOCK_RESPONSE,
        ] {
            let error = parse_command(command, &[0x00]).unwrap_err();
            assert!(error.contains("OTA"), "{error}");
            assert!(is_ota(command));
        }
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
        assert_eq!(
            message_type(CMD_MCU_VERSION_RESPONSE),
            Some("commandMcuVersionResponse")
        );
        assert_eq!(message_type(CMD_MCU_SYNC_TIME), Some("commandMcuSyncTime"));
        assert_eq!(
            message_type(CMD_MCU_GATEWAY_CONNECTION_STATUS),
            Some("commandMcuGatewayConnectionStatus")
        );
        assert_eq!(
            message_type(CMD_WEATHER_REQUEST),
            Some("commandTuyaWeatherRequest")
        );
        assert_eq!(
            message_type(CMD_WEATHER_SYNC),
            Some("commandTuyaWeatherSync")
        );
        assert_eq!(message_type(0x10), None);
        assert_eq!(CLUSTER_ID, 0xEF00);
    }
}
