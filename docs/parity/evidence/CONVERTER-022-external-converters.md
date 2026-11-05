# CONVERTER-022 — external converter save/remove/load

Implementation: `src/converters/external.rs` (`external_dir`, `sync_external_converters`,
`save_external_converter`, `remove_external_converter`, external registry merge),
`src/bridge.rs` (`converter/save`, `converter/remove`), `src/mqtt/mod.rs`
(`bridge/converters`). Behavior: CONVERTER-022; related FRONTEND-004, R1-001.

## Upstream contract

- `lib/extension/externalJS.ts:save`: validate `name` (`.js/.cjs/.mjs`) and
  `code`, write the file under `data/external_converters`, import it, add each
  definition to the converter registry, resolve device definitions, publish the
  retained `bridge/converters` list and answer
  `bridge/response/converter/save` `{status:"ok", data:{}}`.
- `remove`: delete the file, remove its definitions, re-resolve, publish
  `bridge/converters`, answer `bridge/response/converter/remove`.
- `loadFiles`/`getFiles`: on startup load every `*.js|*.cjs|*.mjs`, publish the
  retained `bridge/converters` list.
- `ExternalConverters.loadJS`: sets `externalConverterName`, so
  `getDefinitionPayload` reports `source: "external"`.

## Rust implementation

- User files live in `<config-dir>/external_converters` (production:
  `/etc/zigbee2mqtt-rs/external_converters`).
- `sync_external_converters` writes a synthetic package module per user file
  (`devices/__ext_<hash>_<name>.js`) that appends a wrapper converting
  `module.exports = definition|array` into `{definitions: [...]}`. This lets the
  existing package loader, `prepareDefinition`, conversion, exposes and
  `definition_payload` paths work unchanged.
- A generated `external_registry.json` (lookup/models/fingerprints) is merged
  into the package registry with external candidates first, so external
  definitions take precedence without touching the pinned catalog.
- `save`/`remove` validate names, write/delete, re-sync, clear the definition
  cache, publish `bridge/converters`, answer the bridge response, and republish
  `bridge/devices` **in a background task** (definition generation is expensive
  on Boa and must not delay the response past the frontend's 20 s request
  timeout).
- Boa contexts now carry a 5,000,000 loop-iteration limit so a broken converter
  cannot pin the bridge event loop.
- `.mjs` is rejected with a clear error (the CJS host cannot `import`); `.js`
  and `.cjs` are supported.

## Evidence

Unit/integration (`cargo test --locked`): `external_converter_is_saved_loaded_and_removed`
writes `ext.js`, syncs it, asserts the selected file is the synthetic external
module, the definition payload has `source: "external"`, invalid code is
rejected and removed, and `remove` unloads it.

Hardware (Routerich AX3000 v1 + Dongle-P, 2026-11-05):
- Published the real 28,750-byte `L101Ze_z2m.js` to
  `zigbee2mqtt/bridge/request/converter/save`; the response topic received
  `{"data":{},"status":"ok"}`.
- `/etc/zigbee2mqtt-rs/external_converters/L101Ze_z2m.js` written; startup log
  `external converter ... loaded`; `bridge/converters` contains it.
- `bridge/devices` for the two Lytko devices: `source: "external"`,
  `supported: true`, 20/34 exposes (previously `generated`/`false`).
- Removing the file and restarting no longer selects the external definition.

## Gaps

- `extension/save`/`extension/remove` are still stubs (only converter save/remove
  are implemented).
- `.mjs` (ESM) external converters are rejected; upstream supports them.
- Removing a converter on hardware and a full UI DevConsole click-through were
  not exercised (unit-tested only).
- The synthetic module is written into the converter package directory; a
  read-only package directory would require a separate cache location.
