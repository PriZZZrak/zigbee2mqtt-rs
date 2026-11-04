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
   encodes values for:
   - integers/bitmaps/data enums from 8 to 64 bits, enum8/16, bool, f32/f64,
     short/long strings;
   - composite ZCL types: array/set/bag (`{elementType, elements}`), struct
     (`[{elmType, elmVal}]`), time-of-day, date (year-1900), UTC/BAC-OID,
     cluster/attribute ids, IEEE address and 16-byte security key;
   - Buffalo parameter types from `BuffaloZclDataType`: LIST_UINT8/16/24/32,
     LIST_ZONEINFO, EXTENSION_FIELD_SETS, BUFFER, STRUCTURED_SELECTOR and
     BIG_ENDIAN_UINT24.
   Context-dependent Buffalo types (USE_DATA_TYPE, LIST_THERMO_TRANSITIONS,
   GPD_FRAME, LIST_TUYA_DATAPOINT_VALUES, LIST_MIBOXER_ZONES, MI_STRUCT) are
   rejected with an explicit "needs command context" error. Out-of-range
   integers, unrepresentable floats, wrong buffer lengths and unknown data types
   are rejected (`Error::Zcl`) — no guessing.
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

## Real-package verification (2026-11-04)

The generator and converter host were exercised against the real pinned package:

```text
node scripts/gen-registry.mjs <converters>/dist v26.115.1
registry v26.115.1: 5011 lookup keys (122 multi-candidate), 5355 models,
  3624 fingerprints, 23323 legacy keys, 4560 definitions from 397 files
zcl-types v26.115.1: 129 typed clusters (5 genOnOff attributes)
zcl v26.115.1: 129 clusters, 723 manufacturer codes
```

`scripts/check_package_layout.py` initially **failed** on that package because
`dist/lib/*.d.ts` declaration files were treated as leaked TypeScript sources;
the validator now ignores `.d.ts` (regression test added) and the staged package
validates.

Real modules required non-relative imports that the old host rejected, so the
bootstrap now provides deterministic stubs: herdsman subpaths
(`.../definition/enums`, `.../zspec/zcl`, `.../zspec`) and `node:assert`,
`node:buffer`, `node:util`, `node:fs`; unknown npm packages become empty modules
and fail explicitly when used. `collect_sources` also follows
`zigbee-herdsman-converters/<path>` self-imports and reports missing relative
modules instead of failing silently. Registry loading now tolerates
JSON-serialized `RegExp` fingerprints (`ieeeAddr: {}`) by treating them as
non-matches.

With the real package staged, `evid04_candidate` converts a SONOFF SNZB-02
attribute report (`temperature`, `humidity`, `batteryPercentageRemaining`) to
exactly the same JSON as the Node reference (see EVID-04).

## Tests and evidence

`cargo test --locked` → **430 passed** (398 library, 2 binary, 30 compatibility);
`cargo fmt --check` clean; clippy exit 0 with 5 pre-existing warnings; native
release build succeeds; `python3 -m unittest discover -s scripts/tests` → 28.

- `zcl_types::tests` (8): metadata resolution, primitive encoding with range
  checks, command parameter order/missing-parameter rejection, float/long-string
  encoding, arrays/structs/time/date/IEEE/UTC/int64, Buffalo lists/zones/buffer/
  selector/big-endian u24, context-dependent-type rejection, array/buffer
  validation.
- `external::tests` (4): `convert_set` records a write (`genOnOff.onOff=1`) and
  returns `{window_detection: "ON"}`; records `onWithTimedOff` command payload;
  unknown cluster → `Unsupported`; unknown key → `handled=false`.
- `bridge::tests` (5): native/suffixed key exclusion, write encoding from
  published types, unknown attribute refusal, command parameter encoding and
  frame layout (with/without manufacturer code and disable-default-response),
  unknown command refusal.
- `scripts/tests/test_package_layout.py` requires a valid
  `zcl-commands.json` and accepts `.d.ts` declarations (29 tests total).
- `real_package_modules_and_enums_subpath_are_stubbed` and
  `json_serialized_regex_fingerprint_does_not_break_registry_loading` cover the
  real-package requirements found above.

## Divergences and gaps (not verified)

- The generator has now run against the pinned converters checkout, but the
  GitHub Actions publish workflow itself has not been executed end to end.
- Context-dependent Buffalo types are still rejected (green-power frames, Tuya
  datapoint lists, Miboxer zones, thermostat transition lists, MI structs), so
  converters using them still fail (logged, no frame sent). Per-attribute
  manufacturer codes are not supported (one code per operation).
- The `elementType`/`elmType` numbers come from the converter payload; they are
  not cross-checked against allow-lists beyond the type-encoding rules.
- `convertSet` option handling is partial: `manufacturerCode` and
  `disableDefaultResponse` are honoured; `transition`, `optimistic`, `publish`,
  `executeIfOff`, `direction` and `srcEndpoint` are not applied.
- `readAfterWriteTime` is ignored; converter `publish()` callbacks and
  `result.state` attribute-name rewriting for endpoints are not fully modelled.
- Bridge-level coverage (`convert_set_write_produces_exact_af_frame`,
  `convert_set_command_produces_manufacturer_specific_frame`,
  `convert_set_read_op_sends_read_and_awaits_response`,
  `convert_set_write_srsp_failure_does_not_wedge_the_link`): `handle_set` runs
  through a real `CoordinatorHandle` over an in-memory ZNP duplex with a running
  event pump. The fake device asserts the exact `AF_DATA_REQUEST` payload
  (cluster, endpoint, ZCL header/manufacturer code/TSN/parameters, APS trans id),
  answers SRSPs, and returns an AF read response for the converter read path.
  One test rejects the first AF_DATA_REQUEST (status 0x01) and proves the next
  send still succeeds, so a rejected request does not wedge the queue.
- `coordinator::znp::fake_serial_send_zcl_propagates_coordinator_rejection`:
  `send_zcl` now returns an error with the hexadecimal status instead of only
  logging, matching upstream's rejection behavior.
- No live device/converter-package run, no Home Assistant, no HIL; CONVERTER-019
  stays `IN_PROGRESS`.
