# Storage differential — upstream database/config import and rewrite

Implementation: `scripts/storage_reference.js` (pinned herdsman `Database` +
z2m-style YAML), `scripts/storage_diff.py`, `src/bin/evid04_storage_candidate.rs`,
fixtures under `docs/parity/evidence/storage/fixtures/`.
Behavior: STORAGE-001, STORAGE-004; report `docs/parity/reports/storage.json`.

## Fixture provenance

- `database.template.db` is the authored NDJSON source; it is canonicalized by
  the **pinned upstream** `zigbee-herdsman/dist/controller/database.js`
  `Database.open(...).write()` into `upstream-database.db`, so the fixture is
  emitted by upstream storage code (not by the Rust writer).
- `configuration.yaml` is a z2m-style config with `devices` friendly names; the
  Rust `Config::load` parses it and the Node reference reads the same mapping
  through `js-yaml`.
- Devices: coordinator, an IKEA router with bindings/configured-reporting/cluster
  values, and a battery end device with `interviewState`, `swBuildId`, and
  firmware attributes.

## Differential

Both sides load the same database and configuration and print a normalized view:

- `ieeeAddr`, `nwkAddr`, `friendlyName`, `disabled`, `manufacturerName`,
  `manufacturerId`, `powerSource`, `modelId`, `swBuildId`, `interviewComplete`,
  and sorted endpoints (`id`, `profileId`, `deviceId`, `inputClusters`,
  `outputClusters`).
- Node: real herdsman `Database.open`, z2m config mapping.
- Rust: `database::load_database` with `Config::load`, exactly what the bridge's
  `import_database`/config application uses.

Cases:

1. `import-upstream-database` — reference vs Rust on the fixture;
2. `rewrite-preserves-reference-view` — Node reads the Rust-written database
   (`PersistentDatabase::sync_devices` + `write`) and it must match the original
   view;
3. `rewritten-database-import` — reference vs Rust both load the rewritten DB.

```text
$ python3 scripts/storage_diff.py \
    --db docs/parity/evidence/storage/fixtures/upstream-database.db \
    --config docs/parity/evidence/storage/fixtures/configuration.yaml \
    --reference-cmd "env Z2M_APP=$PWD/.upstream/zigbee2mqtt node scripts/storage_reference.js" \
    --candidate-cmd ./target/release/evid04_storage_candidate \
    --require-pins --report docs/parity/reports/storage.json
STORAGE: 3 passed, 0 failed, pins=[all four MATCH]
```

The harness has regression tests with fake commands
(`scripts/tests/test_storage_diff.py`, 2 tests: match passes, differing device
view fails).

## Gaps

- Compares the normalized import view, not every persisted field: endpoints'
  `clusters`, `binds`, `configuredReportings`, `meta`, `lastSeen`, and group /
  GreenPower entries are preserved on rewrite by code paths but not asserted by
  this differential.
- The config fixture covers `devices` names/flags; full z2m settings parsing
  (MQTT URI form, advanced options) is not compared.
- No crash/corruption/backup or interrupted-write scenarios; no hardware/HIL.
