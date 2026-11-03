# Evidence: Converters registry on top of upstream models-index (CONVERTER-001..006)

## Goal
Support 4000+ devices without a 5MB file in the repo, 90% windfront, pure Rust.

## Source pins (2026-11-02T15:00Z refresh, DRIFT_DETECTED)
- converters 9dabbe4 v26.115.1 `src/lib/{types,modernExtend,exposes,reporting,tuya}.ts` + `src/converters/*`
- upstream resolution: `src/index.ts: findDefinition()` L525 (`getFromIndex` L228: `MODELS_INDEX[modelID.toLowerCase()]`, fallback `normalizeModelID` L141 strip-NUL/trim; single-candidate fast path L541; fingerprint priority L551; zigbeeModel equality L577; whiteLabel override in `findByDevice` L509)
- upstream index: `src/indexer.ts: buildIndex()` L42 → `dist/models-index.json` format `Record<zigbeeModel|fingerprint.modelID|null, [file, index][]>`; duplicate model/zigbeeModel/fingerprint throws
- baseline z2m 344b6e03 (2.14.2), herdsman 3cb89ad (10.10.0)

## Rust mapping
- src/converters/types.rs: DefinitionWithExtend
- src/converters/modern_extend.rs: on_off, light_brightness, battery
- src/converters/exposes.rs: light_brightness, temperature
- src/zigbee/reporting.rs: bind, payload
- src/converters/tuya.rs: EF00 dp 1
- src/converters/external.rs: boa_engine, /etc/herdsman-converters, registry.json via gh-pages

## Registry v2 (P0 fix, 2026-11-02)
- `scripts/gen-registry.mjs` no longer regexes `model:`/`vendor:` (that missed
  `zigbeeModel`/`fingerprint.modelID`/`whiteLabel` and could misbind files).
- Primary source is upstream `dist/models-index.json`, copied verbatim into
  output `lookup` (incl. `"null"` keys, NUL/trim normalization preserved by keeping
  raw keys — Rust must apply the same `normalizeModelID` before lookup).
- `models`: every `definitions[i].model` (+ whiteLabel models with parent pointer).
- `fingerprints`: definition + whiteLabel fingerprint entries with priority.
- `registry` (legacy flat map for the current Rust client): exact model/vendor
  keys always; `zigbeeModel`/`fingerprint.modelID` keys ONLY when their lookup
  entry has exactly one candidate (mirrors upstream single-candidate fast path;
  multi-candidate keys resolve exclusively via `lookup` + `fingerprints`).
- Validation (workflow publish gated): every lookup `[file, index]` resolves to an
  existing `dist/devices/*.js` definition; no duplicate models; every registry
  target exists; every device file contributes ≥1 model.
- Smoke tests (publish impossible without passing): >5000 lookup keys,
  >3000 models, non-empty fingerprints, ≥1 multi-candidate key, all registry
  targets exist.
- Workflow: single gh-pages deploy (`<ref>/` + `latest/` copy + VERSION),
  `concurrency: converters-gh-pages` (no parallel interleaving), ref
  auto-detect (latest upstream release) + tag/SHA validation, no TS fallback
  (TS in `.js` fails the build via grep + `node --check`).
  Verified end-to-end 2026-11-03: workflow published v26.125.0 (real CJS,
  registry.json with lookup/models/fingerprints, zcl.json, latest/ copy +
  VERSION). Live site verified after Pages enable — VERSION/registry.json/
  zcl.json/lytko.js all HTTP 200, lytko.js has zero TS matches, registry
  resolves L101Ze-SLN -> lytko.js [index 6] (5069 lookup keys, 5426 models,
  3700 fingerprints).

## Implementation 2026-11-02/03 (real loader + bridge, P0 fixed)
- `scripts/gen-registry.mjs` emits `registry.json` (`lookup` verbatim,
  `models` incl. whiteLabel, `fingerprints` incl. priority) **plus `zcl.json`**
  (ManufacturerCode/DataType/cluster+attribute IDs read from the built
  `zigbee-herdsman` package in hc/node_modules — zspec lives there, NOT in
  converters dist; see 2026-11-03 CI fix for `../zspec/...` MODULE_NOT_FOUND)
  (`manufacturerCodes`, `dataTypes`, `clusters` id->name, `attributes`
  cluster->id->name) — all values read from built upstream dist, never hand-copied.
- `src/converters/external.rs`: CommonJS loader (relative `require` graph from
  real files, builtin `zigbee-herdsman` stub fed with published zcl.json;
  TS in `.js` fails loudly, unknown modules error loudly). `convert_from_zigbee`
  selects the definition by exact model, builds real `msg` (`cluster` name via
  zcl.json, `type`, `data` with attribute names, `linkquality`) and `meta`
  (`state`, minimal `device`, `options`), dispatches matching `fromZigbee`
  entries (cluster+type), merges returned payloads; the old `{"raw": ...}`
  stub is removed. Per-converter try/catch: one bad converter never kills
  the frame. On-demand fetch (`ensure_device_files`, 80-file cap) pulls the
  device module + transitive deps + zcl.json via curl/wget/uclient-fetch.
- `src/bridge.rs`: `Ok(None)` fallback now parses header (0x0A ->
  `attributeReport`, 0x01 -> `readResponse` via `parse_read_attr_rsp`), converts
  attributes to JSON (`Bytes`/`Invalid` skipped, documented), ensures files,
  publishes merged state via the normal path.
- Tests: 6 unit tests with a CJS fixture (`fromZigbee` dispatch, case-insensitive
  model, unknown cluster/model -> None, TS rejected loudly, require-graph walk).

## Still not as original (2026-11-03)
- `toZigbee.convertSet` not implemented (no entity command plumbing) — set path
  stays native-only.
- No fingerprint tie-breaking: definition chosen by exact model only; upstream
  would fingerprint-match among candidates and apply whiteLabel overrides.
- `meta.device` is minimal (modelID/manufacturerID/manufacturerName/ieeeAddr/
  powerSource); converters reading deeper device state will throw and be
  skipped with a warning.
- `publish()` inside converters is merged into state instead of immediate MQTT
  publish (documented deviation; matches for pure-return converters).
- `Bytes`/`Invalid` attribute values are dropped, not passed through.
- No hardware-in-loop verification (simulated CJS fixtures only).
- modernExtend only 3 of ~50 helpers; exposes only 4 of 100+ types (unchanged).
