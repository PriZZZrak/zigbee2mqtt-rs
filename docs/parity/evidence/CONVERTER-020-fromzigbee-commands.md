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

## Rust change

- The Boa host records `command`, `write` and `read` calls during conversion,
  including cluster, attributes/payload, manufacturer code, disable-default-
  response and the ZCL transaction sequence number. `ConvertedFrame` now carries
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
- `fromZigbeeOps` differential compares requested operations with the pinned
  Node reference:
  - `leviton-level-config-write` → write cluster 8 `{options: 1}`;
  - `yale-unknown-alarm-read` → read cluster 257 `["lockState"]`;
  - plus the existing Tuya gateway-status and weather response commands.
  Converter report: **10/10**, all pins MATCH; bridge report **7/7**.

## Gaps

- Reads are forwarded **after** conversion, so the converter does not receive
  the read result (upstream awaits it). Converters that branch on the value are
  not fully equivalent.
- Operations are fire-and-forget except reads (writes/commands do not await an
  AF/default-response result); destination is always the source endpoint.
- Recording order between commands/writes/reads is grouped, not the converter's
  global call order.
- No live device/HIL verification of the replies/reads.
