# CONVERTER-020 — forward `fromZigbee` response commands, reads and writes

Rust implementation: `src/converters/external.rs` (`ConvertedFrame`,
`FrameCommand`, `FrameWrite`, `FrameRead`, dispatch recording,
`frame_*_from_json`, `convert_*_full`, `console`/`process` runtime prelude,
`msg.meta`), `src/bridge.rs` (`forward_frame_commands`, `forward_frame_writes`,
`forward_frame_reads`, `process_zcl_message` coordinator handle).
Behavior: CONVERTER-020; related CONVERTER-005, EVID-004.

## Source contract

- `fromZigbee` converters may answer or enrich the frame they received:
  `msg.endpoint.command(...)`, `msg.endpoint.write(...)`,
  `msg.endpoint.read(...)` (examples: Tuya `tuyaBase`
  `dist/lib/tuya.js:3730–3790`, Tuya weather `4472–4500`, Leviton
  `leviton.js:58–70`, Yale `yale.js:359–390`).
- Upstream z2m converts inside `receive.ts` with a real herdsman endpoint, so
  these calls become normal ZCL operations on the source endpoint.
- Converters read `msg.meta.zclTransactionSequenceNumber`; herdsman fills it
  from the received frame.
- Command/write parameters are typed by the cluster definition; reads are
  `READ_ATTRIBUTES` requests.

## Read results (async-safe path)

`msg.endpoint.read` now returns the device's response to the waiting converter.
The bridge still runs a single `select!` event loop and still blocks on the
converter-session reply, so the read cannot be delegated back to that loop. A
`ZclIo` handle (`Arc<tokio::sync::Mutex<ZnpHandle>>` + Tokio runtime handle +
ZCL type metadata) is attached to the converter host; the converter thread
executes the read with `request_zcl` through the runtime while the bridge waits.
The ZNP transport and its event pump run as independent Tokio tasks, so the
waiting bridge loop is not needed and no deadlock occurs.

- The JS host exposes a native synchronous `__zclRead`; `msg.endpoint.read`
  calls it, decodes the READ_ATTRIBUTES_RSP (bounded primitive types) and
  resolves the promise with `{attributeName: value}`. Booleans are `0/1` as in
  herdsman.
- Without a coordinator (EVID-04 candidate/harness) the native function returns
  `{disabled: true}` and the call falls back to recording the request, so the
  ops/wire differentials keep working.
- The session thread sets a thread-local ZCL context per job; the per-call
  conversion path does the same.
- Consequence: the converter thread can block on device I/O; a sleeping/broken
  device costs the 3 s read timeout and does not stall the bridge event loop's
  MQTT/coordinator event processing beyond the already-blocking conversion
  wait.

## Rust change

- The Boa host records `command`, `write` and `read` calls during conversion,
  including cluster, attributes/payload, manufacturer code, disable-default-
  response and the ZCL transaction sequence number. `disableDefaultResponse`
  defaults to `true` when the converter does not specify it, matching herdsman's
  `endpoint.write`/`endpoint.read` defaults. `ConvertedFrame` now carries
  `commands`, `writes`, `reads` and state; the old conversion APIs remain
  state-only wrappers.
- `msg.meta.zclTransactionSequenceNumber` / `manufacturerCode` are exposed from
  the parsed header (`ZclFrameHeader` now keeps the sequence number).
- The bridge forwards recorded operations after conversion:
  - commands → cluster-specific frame, `send_zcl`;
  - writes → `WRITE_ATTRIBUTES` encoded with `zcl-commands` types, `send_zcl`;
  - reads → `READ_ATTRIBUTES` via `request_zcl` with the existing 3 s / 2 attempts
    policy, so the bridge waits for the device response.
  Encoding failures are logged and skipped, never guessed. Without a coordinator
  (harness), operations are dropped with a debug log.
- Runtime prelude now provides a no-op `console`, because the compiled
  `lib/logger.js` calls `console.debug/info/warn/error`; without it any converter
  that logs aborted before performing its reads (found by the Yale differential).

## Tests and evidence

- `converter_response_command_is_forwarded_as_af_frame` — exact Tuya time-sync
  AF frame: cluster `0xEF00`, payload
  `[0x01, 0x00, 0x24, 0x02, 0x00, 0xAA, 0xBB]`.
- `converter_read_and_write_are_forwarded_as_af_frames` — one incoming report
  produces a write `[0x10, 0x00, 0x02, 0x00, 0x00, 0x10, 0x01]` on genOnOff and
  a read `[0x10, 0x02, 0x00, 0x00, 0x00]` on msTemperatureMeasurement, with
  APS transaction ids 1 and 2; the fake device answers the read.
- `converter_read_returns_the_device_response_without_deadlock`
  (`multi_thread` test) — converter awaits `msg.endpoint.read`, the fake device
  answers `measuredValue = 2250`, and the published state contains
  `read_temperature: 2250` with the read frame
  `[0x10, 0x00, 0x00, 0x00, 0x00]` observed on the wire.
- `fromZigbeeOps` differential compares requested operations **and their ZCL
  wire frames** with the pinned Node reference:
  - `leviton-level-config-write` → write cluster 8 `{options: 1}`,
    frame `1001020f001801`;
  - `yale-unknown-alarm-read` → read cluster 257 `["lockState"]`,
    frame `1001000000`;
  - `tuya-gateway-status-response` → command frame `110125010001`;
  - `tuya-weather-response` → command frame `1101611100120513…`.
  Converter report: **10/10**, all pins MATCH; bridge report **7/7**. See
  ZCL-013 for the wire differential contract.

## Gaps

- Reads now return real data (see above), but the bridge blocks on the converter
  reply while the read runs; the event loop does not process unrelated MQTT
  events during that window. Devices that answer slowly cost the 3 s timeout.
- The native read decoder covers the bounded `AttributeValue` primitives; arrays,
  structs, Buffers and manufacturer-specific clusters are skipped.
- Operations are fire-and-forget except reads (writes/commands do not await an
  AF/default-response result); destination is always the source endpoint.
- Recording order between commands/writes/reads is grouped, not the converter's
  global call order.
- No live device/HIL verification of the replies/reads.
