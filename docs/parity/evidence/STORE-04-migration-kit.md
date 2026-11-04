# STORE-04 — offline preflight and snapshot kit

Rust implementation: `src/migration.rs`, `src/main.rs` flags, `src/config.rs`
(`validate` visibility). Behavior: STORAGE-006 (inspect) and STORAGE-007
(snapshot); plan STORE-04.

## Sources and scope

- App `344b6e03b43644c97154a72ca9eae17193c553a2`:
  `lib/zigbee.ts:46` configures `data/coordinator_backup.json`; the app reads
  `configuration.yaml`, `database.db` and `state.json` from its data directory.
- Herdsman `3cb89ad906bd9b2de8b3dfed20d5a21070a467d2`:
  `src/controller/database.ts` defines the NDJSON records examined here;
  `adapter/z-stack/adapter/manager.ts` owns coordinator backup/restore, which this
  offline tool intentionally does **not** duplicate.
- This is a Rust operational safety feature for migration. Upstream has no
  “inspect” CLI with an identical contract; no upstream parity claim is made.

## CLI

```bash
zigbee2mqtt-rs --inspect -c /path/configuration.yaml
zigbee2mqtt-rs --inspect --snapshot-dir /outside/data -c /path/configuration.yaml
```

`--inspect` prints JSON and never opens the serial port. Exit codes: `0` no
blockers, `1` blockers found, `2` snapshot/report failure. `--snapshot-dir`
creates a new directory, copies the inputs, writes `manifest.json` last, and
prints the snapshot description in the same JSON report.

## Inspection contents

- Configuration parse and validation errors; network summary (PAN, extended PAN,
  channel, adapter, whether a key is configured — never the key).
- Database path/line counts, unparseable line numbers, device/interviewed/group
  counts, coordinator IEEE, duplicate record ids, duplicate IEEE addresses,
  model histogram.
- Referenced `devices`/`groups` files and `!secret` files, with existence/size.
- Presence of `coordinator_backup.json` and `state.json`.
- Blockers: invalid config, missing referenced file, unparseable database lines,
  duplicate ids/IEEE. Warnings: fresh install (no database), missing coordinator
  record/backup.
- `unperformed_checks` explicitly lists coordinator firmware/NV, network
  parameter comparison and device/converter evaluation as not done offline.

## Snapshot contents and safety

Copies the configuration, referenced device/group and secret files, database,
coordinator backup and state file when present. Destination must be outside the
application directory. File names are prefixed with an index to avoid collisions;
files are `0600`, the directory `0700` (Unix). Each copy is re-read and verified
against an FNV-1a integrity checksum recorded in the manifest. The manifest is
written last, so its absence marks an incomplete snapshot. Checksums are not
signatures.

## Tests and manual evidence

`src/migration.rs`: checksum vector, inspection health/blockers/read-only,
missing referenced file, snapshot copy/checksum/permissions/uniqueness,
destination-inside-application rejection.
`src/main.rs`: CLI flag parsing and required snapshot argument.

Manual run on 2026-11-04 (Linux):

```text
inspect_exit:0
devices 1 coord 0x00124b00120144ae blockers [] warnings []
snapshot_exit:0
snapshot /tmp/opencode/store04-demo/snapshots/snapshot-1793789845-758423
files ['00-configuration.yaml', '01-coordinator_backup.json', '02-database.db']
-rw------- 1 abc users 911 .../manifest.json
```

## Remaining gaps (not verified)

- No automated restore/rollback command; rollback is currently manual from the
  snapshot. A destructive restore needs explicit confirmation semantics.
- Network key/PAN/channel/IEEE are not compared with coordinator NV; this needs
  the ZB-01 startup strategy and hardware. The report says so explicitly.
- No upgrade/downgrade test across Rust versions or converter cache generations.
- Real original → Rust → original migration on a Dongle-P has not been run.
- Cross-platform directory permissions are Unix-only; Windows relies on defaults.
