# CONVERTER-005 — Tuya cluster-command payloads in the bridge

Rust implementation: `src/zigbee/zcl/clusters/tuya.rs`,
`src/converters/external.rs` (`ConvertData` job,
`convert_from_zigbee_session_with_data`, `dispatch_session_data`),
`src/bridge.rs` (`Bridge::process_zcl_message`, Tuya branch).
Behavior: CONVERTER-005; related CONVERTER-009/018, EVID-004.

## Source contract

- Herdsman `src/controller/controller.ts:1114` — a cluster-specific frame gets
  `type = command${capitalize(command.name)}` and `data = frame.payload`.
- Herdsman `src/zspec/zcl/definition/cluster.ts:7184–7325` — `manuSpecificTuya`
  (`0xEF00`) commands `dataRequest` (0x00), `dataResponse` (0x01),
  `dataReport` (0x02), `sendData` (0x04), `activeStatusReportAlt` (0x05),
  `activeStatusReport` (0x06) carry `seq:u16` plus
  `dpValues: LIST_TUYA_DATAPOINT_VALUES`.
- Herdsman `src/zspec/zcl/buffaloZcl.ts:702–720` — each datapoint is
  `dp:u8, datatype:u8, length:u16(BE), data[length]`; a read failure breaks the
  loop, but this implementation treats truncation as an error and refuses to
  publish partial state.
- Converters `src/lib/tuya.ts` / `dist/lib/tuya.js` — `fromZigbee` handlers match
  `cluster: "manuSpecificTuya"`, `type: ["commandDataResponse",
  "commandDataReport", "commandActiveStatusReport", ...]` and read
  `msg.data.dpValues`.

## Rust change

- New parser module: `message_type()` maps the six datapoint command ids exactly
  like upstream's `command${Capitalized}` and `parse_datapoints()` decodes
  sequence + records with bounds checks.
- The bridge's incoming-message handling was extracted from the event loop into
  `Bridge::process_zcl_message` (same behaviour for native + definition
  conversion, suffixing, merge and single publish). A cluster-specific 0xEF00
  frame now builds
  `data = {seq, dpValues:[{dp, datatype, data}]}` and calls the converter session
  with structured data instead of attribute reports.
- Converter host: `ConvertData` job plus
  `convert_from_zigbee_session_with_data`/`dispatch_session_data`, so Tuya
  commands run in the persistent per-device Boa session like other frames.
- Malformed datapoints log a warning and produce no values, so nothing is
  published for that frame.

## Tests and evidence

- `clusters::tuya::tests` (4): multi-datapoint parse, sequence-only payload,
  truncated header/data errors, command-name mapping.
- Bridge runtime with the real MQTT event loop and an in-process broker:
  `tuya_command_data_report_publishes_converted_state` (frame with manufacturer
  code, `commandDataReport`, dp 3 = 14.0 → `local_temperature`,
  `dp_id`, `tuya_seq`, `linkquality`, `last_seen`, device state),
  `tuya_command_data_response_uses_the_same_runtime_path`, and
  `truncated_tuya_command_is_not_published`.
- Real EVID-04 differential: `tuya-trv-local-temperature` (Node reference vs
  Rust candidate on the pinned app-resolved converters) produces
  `{"local_temperature":14}` on both sides.

## Gaps

- Only datapoint commands are structured. Tuya time-sync, MCU version/OTA and
  weather commands remain raw and are not dispatched to converters.
- The parser does not consult `zcl-commands.json` for the parameter list; the six
  known command ids are mapped explicitly. Unknown command ids are ignored.
- No manufacturer-code validation, no Tuya-specific HIL/device run; the bridge
  test uses a fixture definition and the real differential uses the converter
  host, not a radio.
