# CONVERTER-005 — Tuya cluster-command payloads in the bridge

Rust implementation: `src/zigbee/zcl/clusters/tuya.rs`,
`src/converters/external.rs` (`ConvertData` job,
`convert_from_zigbee_session_with_data`, `dispatch_session_data`, Tuya weather
Buffer), `src/bridge.rs` (`Bridge::process_zcl_message`, `tuya_command_data`).
Behavior: CONVERTER-005; related CONVERTER-009/018, EVID-004, MQTT-003.

## Source contract

- Herdsman `src/controller/controller.ts:1114` — a cluster-specific frame gets
  `type = command${capitalize(command.name)}` and `data = frame.payload`.
- Herdsman `src/zspec/zcl/definition/cluster.ts:7184–7367` — `manuSpecificTuya`
  (`0xEF00`) commands:
  - `dataRequest` 0x00, `dataResponse` 0x01, `dataReport` 0x02, `sendData` 0x04,
    `activeStatusReportAlt` 0x05, `activeStatusReport` 0x06:
    `seq:u16` + `LIST_TUYA_DATAPOINT_VALUES`.
  - `mcuVersionResponse` 0x11: `seq:u16`, `version:u8`.
  - `mcuOtaNotify` 0x12, `mcuOtaBlockDataRequest` 0x13,
    `mcuOtaBlockDataResponse` 0x14: image/key/offset chunks.
  - `mcuSyncTime` 0x24: `payloadSize:u16`, `payload:LIST_UINT8`.
  - `mcuGatewayConnectionStatus` 0x25: `payloadSize:u16`, `payload:u8`.
  - `tuyaWeatherRequest` 0x60 / `tuyaWeatherSync` 0x61: `payload:BUFFER`.
- Herdsman `src/zspec/zcl/buffaloZcl.ts:702–720` — datapoint record is
  `dp:u8, datatype:u8, length:u16(BE), data[length]`; `BUFFER` becomes a Node
  `Buffer`, `LIST_UINT8` an array, `UINT8` a number.
- Converters `src/lib/tuya.ts` — handlers match `commandDataReport/Response`,
  `commandMcuSyncTime`, `commandMcuGatewayConnectionStatus`,
  `commandMcuVersionResponse`, `commandTuyaWeatherRequest`. Time sync, gateway
  status and weather handlers **answer** the frame via
  `msg.endpoint.command(...)` and return no state; the MCU version handler
  updates device metadata (`swBuildId`) and returns no state.

## Rust change

- `tuya.rs` exposes `message_type()` for all known command ids and
  `parse_command()` returning `CommandPayload`:
  - `DataPoints {sequence, points}`,
  - `McuVersion {sequence, version}`,
  - `TimeSync {payload_size, payload}`,
  - `GatewayStatus {payload_size, payload}`,
  - `Weather {payload}`.
  Truncated payloads are errors. OTA ids 0x12–0x14 are recognised and rejected
  with an explicit “not structured” error instead of being treated as unknown.
- The bridge builds the same `msg.data` shape herdsman produces
  (`seq/dpValues`, `seq/version`, `payloadSize/payload`, or `payload`), dispatches
  it through the persistent converter session, and merges/publishes only when a
  converter returns state.
- Weather request/sync `payload` is wrapped as a `Buffer` before the converter
  runs (herdsman behaviour); time-sync stays an array and gateway status a
  number.
- `msg.endpoint.command()` resolves so converters that answer a frame complete;
  **the requested response command is not forwarded to the coordinator yet**.

## Tests and evidence

- `clusters::tuya::tests` (5): datapoints, sequence-only payload, version/time/
  gateway/weather parsing, truncated + OTA rejection, message-type mapping.
- Bridge runtime with the real MQTT event loop and an in-process broker:
  `tuya_command_data_report_publishes_converted_state`,
  `tuya_command_data_response_uses_the_same_runtime_path`,
  `truncated_tuya_command_is_not_published`,
  `tuya_time_sync_fields_reach_the_converter`,
  `tuya_weather_request_payload_is_a_buffer`,
  `tuya_ota_command_is_rejected_without_publishing`.
- Real EVID-04 differentials against the pinned Node reference:
  `tuya-trv-local-temperature` (converter output),
  `bridge-tuya-trv-report` (published topic/payload), and
  `bridge-tuya-mcu-sync-time`, `bridge-tuya-gateway-status`,
  `bridge-tuya-weather-request`, `bridge-tuya-ota-block` (both sides publish no
  state; OTA is explicitly rejected only on the Rust side).

## Gaps

- Converter-requested response commands during `fromZigbee` (time sync, gateway
  status, weather sync) are resolved but **not sent**; upstream responds to the
  device. The differential therefore verifies “no state publish”, not the reply.
- `mcuVersionResponse` is parsed but not covered by a bridge differential (the
  converter updates device metadata and returns no state; device metadata
  publishing is not implemented).
- OTA image handling is not modelled at all; OTA commands are rejected.
- No manufacturer-code validation and no Tuya hardware/HIL run.
