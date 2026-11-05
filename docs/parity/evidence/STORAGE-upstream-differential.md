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

The fixture also contains a Group record and a GreenPower device so rewrite
preservation is checked for non-device entries.

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
3. `rewritten-database-import` — reference vs Rust both load the rewritten DB;
4. `rewrite-preserves-persisted-entries` — Node `--raw` compares **every**
   persisted entry before/after the Rust rewrite: endpoint `clusters`, `binds`,
   `configuredReportings`, `meta`, `lastSeen`, `swBuildId`, Group and GreenPower
   records (IDs and members included);
5. `corrupted-database-import` + `corrupted-rewrite-preserves-entries` — a
   trailing malformed JSON line is ignored by both importers and dropped on
   rewrite without losing valid entries;
6. `truncated-database-import` + `truncated-rewrite-preserves-entries` — a
   truncated final line simulates an interrupted write;
7. `stale-temp-write-recovers` — a leftover `database.db.tmp` is renamed by the
   Rust writer and the resulting DB matches the reference entry view.

```text
$ python3 scripts/storage_diff.py \
    --db docs/parity/evidence/storage/fixtures/upstream-database.db \
    --config docs/parity/evidence/storage/fixtures/configuration.yaml \
    --reference-cmd "env Z2M_APP=$PWD/.upstream/zigbee2mqtt node scripts/storage_reference.js" \
    --candidate-cmd ./target/release/evid04_storage_candidate \
    --require-pins --report docs/parity/reports/storage.json
STORAGE: 9 passed, 0 failed, pins=[all four MATCH]
```

The harness has regression tests with fake commands
(`scripts/tests/test_storage_diff.py`, 2 tests: match passes, differing device
view fails).

## Gaps

- Raw-entry comparison covers the persisted fixture fields, but does not test a
  real power-loss/kill-during-write or backup/restore procedure, only simulated
  corrupted/truncated/stale-tmp files.
- The config fixture covers `devices` names/flags; full z2m settings parsing
  (MQTT URI form, advanced options) is not compared.
- Group membership/reporting runtime behaviour and hardware remain untested.
