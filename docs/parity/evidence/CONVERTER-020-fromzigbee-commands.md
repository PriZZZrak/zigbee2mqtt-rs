# CONVERTER-020 — forward `fromZigbee` response commands to the coordinator

Rust implementation: `src/converters/external.rs` (`ConvertedFrame`,
`FrameCommand`, command recording in the dispatch scripts,
`frame_commands_from_json`, `convert_*_full` APIs), `src/bridge.rs`
(`Bridge::forward_frame_commands`, `process_zcl_message` coordinator handle).
Behavior: CONVERTER-020; related CONVERTER-005, EVID-004.

## Source contract

- Some `fromZigbee` converters answer the frame they received:
  `msg.endpoint.command(cluster, command, payload, options)` — for example Tuya
  `tuyaBase` (time-sync and gateway status replies) and
  `tuyaWeatherForecast` (weather sync reply), converters
  `dist/lib/tuya.js:3730–3790`, `4472–4500`.
- Upstream z2m converts inside `receive.ts` and gives converters a real herdsman
  endpoint, so these calls are sent to the device as normal ZCL cluster commands.
  The endpoint is the source endpoint of the received frame (`msg.endpoint`).
- Command parameters are encoded according to the cluster definition
  (`zcl-commands.json`); options may carry `manufacturerCode` and
  `disableDefaultResponse`.

## Rust change

- The Boa host records `msg.endpoint.command(...)` calls during conversion
  instead of resolving them into a no-op. Each recorded call captures the
  cluster, command, payload, manufacturer code and disable-default-response flag.
- `ConvertedFrame {state, commands}` is returned by the new full conversion
  APIs (`convert_from_zigbee_with_message_full`,
  `convert_from_zigbee_with_data_full`,
  `convert_from_zigbee_session_full`,
  `convert_from_zigbee_session_with_data_full`). The old APIs remain wrappers
  that return only state.
- `Bridge::process_zcl_message` forwards recorded commands after conversion:
  parameters are encoded with the published ZCL metadata, a cluster-specific
  frame is built, a transaction id is allocated and `send_zcl` is called on the
  source endpoint. Encoding failures are logged and skipped, not guessed.
- If the bridge runs without a coordinator (broker-free tests/harness), the
  commands are dropped with a debug log rather than silently lost.

## Tests and evidence

- `converter_response_command_is_forwarded_as_af_frame` — fake coordinator at
  the ZNP duplex observes the exact `AF_DATA_REQUEST` for a Tuya time-sync
  reply: cluster `0xEF00`, endpoint 1, payload
  `[0x01, 0x00, 0x24, 0x02, 0x00, 0xAA, 0xBB]` (control, TSN, command 0x24,
  `payloadSize` u16 LE + payload bytes), APS transaction id 1.
- Differential mode `fromZigbeeOps` (Node reference vs Rust candidate) compares
  the requested response commands, including payload bytes:
  - `tuya-gateway-status-response` → `cluster 61184 / command 37 /
    {payloadSize:1, payload:1}`;
  - `tuya-weather-response` → `cluster 61184 / command 97 /
    {payload:[17,0,18,5,19,0,...]}` (the Buffer produced by the real converter).
  Both fixtures pass with the pinned Node reference; report
  `docs/parity/reports/evid04.json` (8/8 converter fixtures).

## Gaps

- Only `command` is forwarded. `msg.endpoint.read/write` requested during
  `fromZigbee` remain resolved no-ops.
- Command replies are fire-and-forget: no AF response status is awaited and
  failures are only logged.
- The destination endpoint is the frame source endpoint; converters targeting a
  different endpoint are not supported yet.
- No live device/HIL verification of the replies (time sync / weather) exists.
