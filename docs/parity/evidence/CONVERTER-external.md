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

## Still not as original
- modernExtend only 3 of ~50 helpers
- exposes only 4 of 100+ types
- reporting only bind/payload, not configureReporting per cluster
- tuya only dp 1, not full EF00
- external eval stub (boa_engine not yet wired to real fromZigbee)
- Rust resolver does not yet implement fingerprint-priority/whiteLabel selection
  (reads only the flat `registry` map); `lookup`+`fingerprints` published for it.
- No generated definitions for 4000 devices in-repo (by design: on-demand JS).
