# ZCL-013 — wire encoding differential for converter operations

Implementation: `src/converters/wire.rs` (shared encoders),
`src/bridge.rs` (uses the shared encoders), `scripts/evid04_node_reference.js`
(`encodeFrames` using pinned herdsman `Zcl.Frame.create`),
`src/bin/evid04_candidate.rs` (`frames` in `fromZigbeeOps` output).
Behavior: ZCL-013; related CONVERTER-020, EVID-004.

## Contract

The bridge must emit the same ZCL frame bytes as herdsman for the operations a
`fromZigbee` converter requests:

- cluster-specific command → header (frame control, optional manufacturer code,
  TSN, command id) + parameters in definition order;
- Write Attributes → global frame with each `{attrId, dataType, attrData}`;
- Read Attributes → global frame with `{attrId}` records;
- header bits: `disableDefaultResponse` defaults to **true** for
  `endpoint.write` and `endpoint.read`, and for cluster commands without a
  response (herdsman `getOptionsWithDefaults` / `hasResponse`). The host records
  explicit options and otherwise uses `true`, carrying manufacturer code.

## Differential

`fromZigbeeOps` output now includes a `frames` array
(`{kind, tsn, hex, await_default_response}`), produced independently:

- Node: real herdsman `Zcl.Frame.create(...).toBuffer()` for every recorded
  command/write/read;
- Rust: `converters::wire::encode_*` for the same recorded operations.

TSN starts at the fixture `wire_tsn` (default 1) and increments once per
operation in the grouped order commands → writes → reads. This isolates frame
encoding; actual bridge TSN allocation is covered by fake-coordinator tests.

| fixture | frame | hex |
| --- | --- | --- |
| `tuya-gateway-status-response` | command 0x25 | `110125010001` |
| `tuya-weather-response` | command 0x61 | `1101611100120513…` |
| `leviton-level-config-write` | write genLevelCtrl.options | `1001020f001801` |
| `yale-unknown-alarm-read` | read closuresDoorLock.lockState | `1001000000` |

All four are byte-identical between the pinned Node encoder and Rust; the
converter differential is **10/10** with all pins MATCH
(`docs/parity/reports/evid04.json`).

## Gaps

- Only operations produced by the four ops fixtures are covered; no arrays,
  structs, manufacturer-specific custom clusters or error/length-edge cases yet.
- The Node encoder is fed the recorded operation options; upstream's full
  default-option resolution lives in the converter host (implemented here as
  `disableDefaultResponse` default true), not independently re-derived.
- TSN allocation is not compared (fixed by `wire_tsn`), only encoding.
- `await_default_response` is derived from the recorded options; all current
  real fixtures use `disableDefaultResponse=true`, so no `true` case is covered
  by the Node differential (acknowledgement behaviour is covered by
  fake-coordinator tests in CONVERTER-020).
