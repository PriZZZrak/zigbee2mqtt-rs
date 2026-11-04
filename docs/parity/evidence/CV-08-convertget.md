# CV-08 — execute upstream `convertGet`

Rust implementation: `src/converters/external.rs` (`convert_get`,
`ConvertGetRead`, `attribute_id`/`attribute_name`), `src/bridge.rs` get fallback.
Behavior: CONVERTER-015; plan CV-08.

## Source contract

Pinned converters `9dabbe4e343573c5a3070605efd843a1b3a16058`:

- `src/lib/types.ts`: a `toZigbee` converter has `key` (string or array) and an
  optional async `convertGet(entity, key, meta)`; it issues reads through the
  endpoint (`entity.read(cluster, attributes)`).
- App `lib/extension/publish.ts` calls `convertGet` for requested keys; the
  device response is later converted by the normal `fromZigbee` pipeline.

## Rust change

`ExternalConverter::convert_get`:

- selects the same prepared definition as `convert_from_zigbee`;
- builds a Boa context with the existing CJS bootstrap/stub;
- exposes an endpoint stub whose `read(cluster, attributes)` records the request
  and resolves immediately;
- calls the matching `convertGet` for each requested key, drains the Boa Promise
  job queue (`Context::run_jobs`) and reads the recorded operations;
- resolves cluster/attribute names to numeric IDs using the published `zcl.json`
  maps, ignoring unknown names;
- returns `Vec<ConvertGetRead>` for the Rust caller to execute.

`handle_get` uses this fallback only for explicitly requested keys that the
native key map does not cover. It performs the returned reads through
`request_zcl`, parses the read responses and feeds them to
`convert_from_zigbee(..., "readResponse", ...)` before publishing.

## Tests and evidence

- `async_convert_get_records_resolved_reads`: fixture `convertGet` is `async`,
  awaits `entity.read("msTemperatureMeasurement", ["measuredValue"])`; result is
  `{cluster 1026, attributes [0]}`.
- `missing_convert_get_returns_no_reads`.
- Full suite: `cargo test --locked` → **313 passed** (281 library, 2 binary,
  30 compatibility).

## Divergences and gaps (not verified)

- The stub only records `read`; `write`, `command`, `configureReporting` and
  `bind` resolve without performing anything. A convertGet that mutates or
  depends on them is not equivalent.
- Only Promise microtasks are drained; timers, external I/O and multi-turn
  promise chains are not serviced.
- `meta` contains a partial device/state snapshot and empty options.
- Endpoint identity is always `ID: 1`; multi-endpoint convertGet is not modeled.
- Errors inside convertGet are logged at debug level and do not fail the get
  operation.
- Read responses are also delivered to the normal event loop, so an explicit
  converter read can be processed twice; no dedup/transaction ownership yet.
- No real published package or Node differential test; fixture only. No HIL.
