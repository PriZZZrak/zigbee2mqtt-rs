# DB/config format parity between the Rust port and upstream Node

Status: complete (2026-11-19) — implementation, tests, oracle and HIL.

Implemented:
- canonical DB entry writer (omit `undefined`, upstream field order,
  `powerSource` observable, JS number formatting);
- upstream bind/reporting shapes (`cluster`, `deviceIeeeAddress`,
  `manufacturerCode` omission);
- `database_roundtrip.rs`: Node golden, model round-trip, real Node DB bytes,
  Rust -> Node idempotence;
- `yaml_dump.rs`: js-yaml-compatible writer; `yaml_dump.rs` tests against a
  Node-generated golden; `write_if_changed` now uses it;
- oracle fixtures: `tools/oracle/db-fixture.mjs`, `yaml-fixture.mjs`,
  `examples/db_roundtrip.rs`.

## Goal

Make `database.db` and `configuration.yaml` (plus split device/group files)
byte-compatible between the Rust port and upstream Zigbee2MQTT/zigbee-herdsman,
so a data directory can be moved in both directions without data loss or
unexpected rewrites.

## Findings

1. `Device::to_database_entry` writes `null` for absent optional fields; Node
   `JSON.stringify` omits `undefined` properties.
2. Endpoint binds serialize as `{type,endpointID,deviceIeeeAddr}` and drop
   `cluster`; upstream writes
   `{cluster,type,deviceIeeeAddress,endpointID}` (and
   `{cluster,groupID,type}` for groups).
3. `powerSource` code 0 is observable as `undefined` upstream; Rust writes
   `"Unknown"`.
4. Optional endpoint/reporting fields (`profId`, `devId`, `manufacturerCode`)
   are written as `null` instead of omitted.
5. Integral `f64` values (e.g. `repChange`) serialize as `0.0`; JS writes `0`.
6. `serde_yaml_ng` does not match `js-yaml.dump` sequence indentation and
   scalar/line-width rules, so a config rewrite changes bytes.

## Work plan (remaining)

1. [done] Canonical DB encoder:
   - ordered maps matching upstream field order;
   - omit `None` fields;
   - upstream-compatible `powerSource`;
   - JS-compatible integral-number serialization;
   - fix bind/reporting shapes.
2. [done] Tests: Node golden DB fixtures; model load -> serialize -> byte equality;
   Rust write -> Node `Database.open().write()` idempotence; synthetic cases
   for all device types, endpoint/group binds, reportings, groups.
3. [done] Config writer: reproduce `js-yaml.dump` defaults (indent 2, indented
   sequences, quoting, line width, insertion order); corpus test against the
   Node oracle and the router configuration.
4. [done] HIL: data copy Node -> Rust -> Node and reverse, byte hashes after a no-op
   run and after rename/revert.
5. [done] Docs: parity matrix + `docs/hil/2026-11-19-db-format-parity.md`; final gates recorded in the commit.

## Acceptance

- Frozen Node `database.db` -> Rust load/save -> identical bytes.
- Rust `database.db` -> Node open/save -> identical bytes.
- Frozen Node `configuration.yaml`/split files -> Rust load/save -> identical
  bytes.
- HIL no-op start/stop preserves hashes; rename/revert restores the original
  bytes.
