# HIL: database/config format parity (2026-11-19)

Follow-up to `database-format-parity-plan.md`: make `database.db` and
`configuration.yaml` byte-compatible with upstream Zigbee2MQTT, so a data
directory can be moved between the Rust port and Node without format drift.

## Artifact

| Item | Value |
|---|---|
| Source tree | working tree on top of `6dbbd7c` (format-parity change) |
| Binary | `target/aarch64-unknown-linux-musl/release/z2m-rs` |
| SHA256 | `da6941473a0c49c62dea0f22a6ad93a663e9370d17fd75d7fa0f6be9cfbe9873` |
| Size | 12,471,760 bytes |
| Copy on router | `/tmp/z2m-rs-db-parity` (same SHA on device) |
| Hardware | Routerich AX3000 v1, OpenWrt 24.10.8, Dongle-P Z-Stack3x0 20250321 |

## Data repair before the run

The earlier Rust runs wrote endpoint binds without `cluster` and used the wrong
key (`deviceIeeeAddr`). Node then loaded and persisted that shape, so the live
`/etc/zigbee2mqtt/database.db` had lost the bind cluster IDs. Bind counts and
order still matched the 2026-11-17 Node backup, so the clusters were restored by
matching IEEE/endpoint/index (11 bind records), the key was renamed to
`deviceIeeeAddress`, and the file was rewritten through the Node
`Database.open().write()` writer. The pre-repair file is kept at
`/tmp/database.db.before-format-repair` (SHA256
`1e9f9eaaeb753047be92f1385607439f34c6810bc8857602a8352c507bfe6f24`).

Canonical repaired DB: 7083 bytes, SHA256
`a39ff2c62b035734a654610ad041b8ed703fd4e5a489b1f8762425b046998ab7`.

## Run

```text
copy /etc/zigbee2mqtt/{configuration.yaml,database.db,state.json} -> /tmp/z2m-db-parity2
start Rust with ZIGBEE2MQTT_DATA=/tmp/z2m-db-parity2
bridge/state -> {"state":"online"}
Currently 3 devices are joined.
```

Hashes before/after the Rust start:

```text
configuration.yaml  1756e7f249b9ce1c40eb38ed02530b6c8595b8ef2deb5a23ea52cf43130cfe6d
                    unchanged (Node js-yaml bytes)
database.db         a39ff2c62b035734a654610ad041b8ed703fd4e5a489b1f8762425b046998ab7
                    unchanged (canonical Node bytes)
state.json          8295ef… (1529 bytes, 3 keys, unchanged)
```

Device rename round-trip through the bridge API on a previous copy:

```text
before rename   configuration.yaml 1756e7f2…
rename -> "HIL DB Temp"             04505a00…
rename back -> "Датчик температуры" 1756e7f2…   (exact Node bytes restored)
```

Database checks:

```text
Rust DB copied back == copy input                          byte-identical
Node Database.open(rust_db).write() == rust_db             byte-identical
binds after the run:
  0xa4c13887bf43cc0f/1   [0]
  0x404ccafffe5c4ce0/1   [513, 2820, 1794]
  0x404ccafffe5c4ce0/3   [15]
  0x1020bafffef50ee8/1   [513, 2820, 1794]
  0x1020bafffef50ee8/2   [513, 2820, 1794]
```

Offline oracle on the real Node DB
(`node tools/oracle/db-roundtrip.mjs /tmp/opencode/node-database.db`):

```text
OK: 7081 bytes preserved through Node -> Rust -> Node
```

## Restore

```text
kill -INT <rust pid>          -> "Stopped Zigbee2MQTT"
/etc/init.d/zigbee2mqtt start -> running
zigbee2mqtt/bridge/state      -> {"state":"online"}
zigbee2mqtt/bridge/devices    -> Coordinator + 3 devices
configuration.yaml            unchanged (1756e7f2…)
database.db                   a39ff2c6… immediately after Rust stop; Node then
                              rewrote runtime lastSeen values (088a5321…), keeping
                              the canonical field/bind shape and all clusters
state.json                    unchanged (8295ef…)
```

## Notes

- No commissioning, restore-backup or coordinator identity change was performed.
- The DB was already repaired before this run; the run proves the new writer
  leaves canonical bytes untouched and Node rewrites them identically.
- The YAML round-trip was measured on the previous copy with the same binary
  family; the current binary keeps `configuration.yaml` byte-identical on start.
- `state.json` remains a runtime cache; the CP-04 preservation evidence is in
  `2026-11-18-cp12-repeat.md`.
