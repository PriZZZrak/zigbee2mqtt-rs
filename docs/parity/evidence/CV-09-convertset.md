# CV-09 — execute `toZigbee.convertSet`

Rust implementation: `src/converters/external.rs` (`ConvertSetOp`,
`ConvertSetOutcome`, `dispatch_session_set`, `convert_set`, `zcl_types`),
`src/converters/zcl_types.rs`, `src/bridge.rs` (`converter_set_keys`,
`encode_write_attributes`, `encode_command_op`, `command_frame`, `handle_set`
execution), `scripts/gen-registry.mjs`, `scripts/check_package_layout.py`.
Behavior: CONVERTER-019; plan CV-06/07/UI.

## Source contract

Pinned converters `9dabbe4e343573c5a3070605efd843a1b3a16058`:

- `src/lib/types.ts:448` — `toZigbee[].convertSet(entity, key, value, meta)`
  returns `{state?: KeyValue, ...}`; converters call `entity.write(cluster,
  {attribute: value}, options)`, `entity.command(cluster, command, payload,
  options)` and `entity.read(cluster, attributes)`.
- Herdsman `src/zspec/zcl/definition/cluster.ts` — per-cluster attribute data
  types (`genOnOff.onOff: {ID: 0x0000, type: DataType.BOOLEAN}`) and command
  parameter types (`onWithTimedOff: ctrlbits:UINT8, ontime:UINT16,
  offwaittime:UINT16`). Without this table the symbolic converter calls cannot be
  encoded, so the generator now exports it as `zcl-commands.json`.
- `endpoint.write`/`command` serialize with exactly those types; manufacturer
  code / disable-default-response come from `utils.getOptions(meta.mapped,
  entity)`.

## Rust change

1. **Type metadata** (`zcl_types.rs`): parses `zcl-commands.json`
   (`clusters → {attributes: {id: {name, type}}, commands: {id: {name,
   parameters}}}`), resolves clusters/attributes/commands by id or name, and
   encodes values for bool, u8/u16/u24/u32, i8/i16/i24/i32, enum8/16, bitmap8/16,
   f32/f64 and short/long strings. Out-of-range integers, unrepresentable floats
   and unknown data types are rejected (`Error::Zcl`) — no guessing.
2. **Converter host** (`external.rs`): `dispatch_session_set` runs the matching
   `toZigbee.convertSet` inside the persistent per-device Boa session with an
   entity stub that records `write`/`command`/`read` operations (cluster name/id,
   attributes, payload, manufacturer code, disable-default-response) and awaits
   the async converter. `ConvertSetOutcome` carries `handled`, returned `state`
   and the recorded ops. Unresolvable clusters/commands become
   `ConvertSetOp::Unsupported`.
3. **Bridge**: `handle_set` keeps native handling first and then runs
   `convertSet` for every remaining payload key (endpoint-suffixed native keys
   such as `brightness_l2` are excluded). Returned state is merged into the
   optimistic payload; recorded writes are encoded with `write_attributes_payload`,
   commands with a cluster-specific frame (`0x01 | 0x04 | 0x10` control byte,
   optional manufacturer code), and reads via `request_zcl`. Every encoding
   failure is logged and the operation is skipped instead of sending a malformed
   frame.
4. **Packaging**: `scripts/gen-registry.mjs` emits `zcl-commands.json` from the
   pinned herdsman definition with smoke checks (≥50 typed clusters,
   `genOnOff.onWithTimedOff` has 3 typed parameters);
   `scripts/check_package_layout.py` requires the file and a `genOnOff` entry
   before publication.

## Tests and evidence

`cargo test --locked` → **417 passed** (385 library, 2 binary, 30 compatibility);
`cargo fmt --check` clean; clippy exit 0 with 5 pre-existing warnings; native
release build succeeds; `python3 -m unittest discover -s scripts/tests` → 28.

- `zcl_types::tests` (4): metadata resolution, primitive encoding with range
  checks, command parameter order/missing-parameter rejection, float/long-string
  encoding.
- `external::tests` (4): `convert_set` records a write (`genOnOff.onOff=1`) and
  returns `{window_detection: "ON"}`; records `onWithTimedOff` command payload;
  unknown cluster → `Unsupported`; unknown key → `handled=false`.
- `bridge::tests` (5): native/suffixed key exclusion, write encoding from
  published types, unknown attribute refusal, command parameter encoding and
  frame layout (with/without manufacturer code and disable-default-response),
  unknown command refusal.
- `scripts/tests/test_package_layout.py` now requires a valid
  `zcl-commands.json`.

## Divergences and gaps (not verified)

- **The generated table and the Node generator were not executed**: no
  Node/pnpm here, so `zcl-commands.json` has only been exercised through a
  hand-written fixture. The GitHub workflow must build a package and run
  `check_package_layout.py` before this can be trusted.
- Only primitive/string types are encoded. ZCL arrays, structs, time-of-day,
  `BuffaloZCLDataType` (e.g. some Tuya/green-power payloads), per-attribute
  manufacturer codes and `ListThermoTransitions` are rejected, so converters
  using them still fail (logged, no frame sent).
- `convertSet` option handling is partial: `manufacturerCode` and
  `disableDefaultResponse` are honoured; `transition`, `optimistic`, `publish`,
  `executeIfOff`, `direction` and `srcEndpoint` are not applied.
- `readAfterWriteTime` is ignored; converter `publish()` callbacks and
  `result.state` attribute-name rewriting for endpoints are not fully modelled.
- No bridge-level test drives `handle_set` with a fake coordinator, and the
  socket test still asserts the MQTT command only (not the resulting AF frame).
- No live device/converter-package run, no Home Assistant, no HIL; CONVERTER-019
  stays `IN_PROGRESS`.
