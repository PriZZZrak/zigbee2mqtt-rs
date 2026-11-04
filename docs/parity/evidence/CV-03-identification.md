# CV-03 — converter identification from the published registry

Rust implementation: `src/converters/external.rs` (`RegistryFile`,
`select_definition`, `fingerprint_matches`), `src/bridge.rs` dispatch.
Behavior: CONVERTER-007; plan CV-03/CV-05.

## Source contract

Pinned converters `9dabbe4e343573c5a3070605efd843a1b3a16058`,
`src/index.ts`:

- `getFromIndex` (217–234) lowercases the Zigbee `modelID` (or uses `"null"`),
  looks it up in `MODELS_INDEX`, then retries with `normalizeModelID`
  (139–141: strip from the first NUL and trim).
- `findDefinition` (525–593): single candidate fast path; otherwise the
  candidate with the highest-priority matching fingerprint wins (ties keep the
  first in candidate order); then the first candidate whose `zigbeeModel`
  includes the modelID; then optional generated definitions.
- `isFingerprintMatch` (605–628) compares modelID, manufacturer name/ID, type,
  date code, hardware/application/stack/ZCL versions, power source, software
  build id, IEEE regex and endpoint ID arrays only when present.
- `models` is a product-model catalogue used by the UI, **not** the Zigbee
  modelID lookup.

Published registry schema observed from `v26.125.0`:
`lookup` maps normalized modelID → ordered `[file,index]` candidates;
`fingerprints` is a flat list of `{file,index,fingerprint,...}`; `models` maps
product model → metadata.

## Change

- Deserialize and use `lookup` and `fingerprints`; the old `models[modelID]`
  selection is retained only for registries that have no lookup index at all.
- Apply NUL stripping/trim and the `"null"` bucket for a missing modelID.
- Match available identity fields and choose the highest fingerprint priority,
  preserving candidate order on ties; fall back to the first candidate.
- Stop using manufacturer name as a model key in the bridge.
- `ExternalDeviceInfo` now carries type, software build, optional version
  fields and endpoint IDs for fingerprint matching.

## Tests added

`src/converters/external.rs`: fingerprint priority selection, unmatched
fingerprint fallback, null bucket + NUL normalization, product-model map not used
when lookup exists, lookup-based `ensure_device_files` success/miss, plus the
existing case-insensitive dispatch test.

## Divergences and gaps (not verified)

- Fingerprints requiring IEEE regex or identity fields Rust does not yet know
  (applicationVersion/dateCode/hardware/stack/ZCL) are treated as non-matches.
  Upstream may match them, so a specific definition can be missed and the first
  candidate used until those fields are populated.
- The `zigbeeModel` fallback is approximated by the first lookup candidate;
  candidate definitions are not re-read, so a Node differential oracle is still
  required.
- No execution of a real published package with these candidates; tests use
  fixtures. Boa host API/prepareDefinition gaps remain (CV-05..08).
- Imported external definitions have no precedence handling in this change.
