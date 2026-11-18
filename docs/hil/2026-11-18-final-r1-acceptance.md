# HIL: final R1 acceptance (2026-11-18)

Audited item: CP-12. Hardware: Routerich AX3000 v1, OpenWrt 24.10.8 (aarch64),
SONOFF Dongle-P (TI CC2652P, Z-Stack3x0, revision 20250321), live network with
3 joined devices. Start/resume only — **no commissioning, no restore-backup**.

## Artifact

| Item | Value |
|---|---|
| Commit | `f1e9cf78` |
| Binary | aarch64-unknown-linux-musl release `z2m-rs` |
| SHA256 | `4146b5dce0a35e6c3c5319d0809962e288f6154d9faa33059b531ec3cc8756d6` |
| Size | 12,486,872 bytes |
| Copy on router | `/tmp/z2m-rs-hil` (same SHA verified on device) |
| Data dir | `/etc/zigbee2mqtt` (Node z2m 2.14.3-r1 data, unchanged config) |
| Frontend assets | `/opt/zigbee2mqtt/node_modules/zigbee2mqtt-windfront/dist` (2.15.1, matching `packaging/windfront.version`) |

The Node service was stopped for the run and restored afterwards; the Rust
binary was executed from `/tmp` (no package changes, no opkg remove/install).

## Startup and protocol

Measurement helper: `/proc/uptime` in milliseconds on the router; MQTT clients
are `mosquitto_sub`/`mosquitto_pub` against `mqtt://localhost`.

```text
startup to bridge/state online   2010 ms
bridge/info                     28117 bytes
bridge/devices                  5694 bytes (Coordinator + 3 devices, all interviewed)
bridge/info.version/commit      0.1.0 / f1e9cf78
network.channel                 11
permit_join                     false
```

All devices were already interviewed (`interview_completed: true`); no
interview was triggered. Coordinator resumed the existing network
(channel 11, PAN from `database.db`), no NV rewrite.

## Responsiveness

| Operation | Measured | Bounds |
|---|---:|---|
| `bridge/request/health_check` | 1020 ms (includes shell/MQTT client startup and the 1 s subscription settle) | `-W 5` |
| `bridge/request/permit_join {"time":0}` | 1250 ms | `-W 5` |
| `bridge/request/networkmap {"type":"raw","routes":true}` | 4300 ms, 4395 bytes | `-W 30` |
| `health_check` while networkmap running | 1020 ms | `-W 5` |
| HTTP `/` (Windfront index) | 200, 5.3 ms | 5 s |
| HTTP `/assets/About-…js` | 200, 4.0 ms | 5 s |
| SPA route `/devices/0x…` with `Accept: text/html` | 200 | 5 s |
| HTTP `/api` without upgrade | 404 (WebSocket path, not a static file) | 5 s |

The MQTT latencies are upper bounds: each measurement starts a fresh
`mosquitto_sub` process after a 1 s settle, so they include process/shell
overhead. They show that the event loop was not blocked by startup,
networkmap or persistence work; the networkmap itself completed in 4.3 s while
health checks stayed at ~1 s.

A first start attempt 3 s after stopping Node failed (`SYS version` SRSP was
mis-framed and readConfiguration timed out). Waiting for the serial port to
settle and starting again succeeded on all subsequent runs; this is recorded
as a startup race/robustness note, not a parity claim. No commissioning was
involved.

## Home Assistant and device registry

```text
retained discovery topics (Rust run)   8
topics carrying "sw":"0.1.0"           8
bridge connection_state topics         1
bridge/devices                         Coordinator + 3 devices
```

The 8 topics are the R1 bridge entities (connection state, restart required,
versions, logs, etc.). Per-device cluster discovery is intentionally outside
the initial R1 surface (AGENTS §4). Rename/remove/retained cleanup is covered by
the software HA tests (`clears_discovery_on_entity_removed`,
`group_removal_clears_discovery`, `keeps_discovery_topics_when_rename_is_not_a_homeassistant_rename`).

## Storage

Before the run:

```text
configuration.yaml 1756e7f249b9ce1c40eb38ed02530b6c8595b8ef2deb5a23ea52cf43130cfe6d
database.db       25fb72df12cf4e0c5b1de6e765c19ee3b07c16553d788b9d229d011b5b6ec598
state.json        3682f0d2d4a2d8c9f2fcb9e07656b1ba932a0ede33226081e52629faa95c8de6
```

After the Rust run and after restoring Node:

```text
configuration.yaml 1756e7f249b9ce1c40eb38ed02530b6c8595b8ef2deb5a23ea52cf43130cfe6d (identical)
database.db        rewritten by the Rust port (7442 bytes); Node reopened it and listed the same 3 devices
state.json         44136fa3… (`{}`, 2 bytes)
```

`configuration.yaml` is byte-identical. `database.db` is semantically intact
(Node listed `Термостат кухня`, `Датчик температуры`,
`Термостат спальная/детская`, plus Coordinator; all interviewed). The Rust port
rewrote the database in its NDJSON layout, so the hash differs from Node's
format. `state.json` is only a cache and was reset to `{}`; cached device state
is repopulated by traffic. This is recorded as a limitation, not hidden.

The earlier CP-04 evidence covers kill -9/respawn and packaged reboot recovery
(`docs/hil/2026-11-17-crash-recovery.md`,
`docs/hil/2026-11-17-packaged-service.md`); CP-12 did not repeat those
destructive runs.

## Restore

```text
kill <rust-pid>
/etc/init.d/zigbee2mqtt start
bridge/state              {"state":"online"}
bridge/devices            4 entries (Coordinator + 3 devices)
/etc/init.d/zigbee2mqtt   running
```

No package was installed or removed. The Node service is running again on the
same data directory.

## AGENTS §9 mapping

| Requirement | Evidence |
|---|---|
| Coordinator starts/resumes, network operations | this run (2.0 s to online, channel 11, 3 devices) + `docs/hil/2026-11-16-rust-port-startup.md` |
| MQTT/Windfront responsive during startup/interview/persistence | startup/health/networkmap/concurrent-health measurements above; interview was already complete (no new interview run) |
| Bridge API returns real transactions/payloads | `bridge/info`, `devices`, `health_check`, `permit_join`, `networkmap` payloads captured; software suites assert transactions/errors |
| Config/database/state recovery | CP-04 kill -9/reboot HIL; config hash unchanged; database semantically intact; state cache reset documented |
| HA bridge entities + device-registry discovery/cleanup | 8 bridge discovery topics; bridge/devices lists 3 devices; cleanup asserted in HA tests; per-device discovery out of R1 |
| No unbounded wait/deadlock | every HIL operation bounded (`-W`); concurrent health during networkmap; software timeout/cancellation tests |
| Exact hardware/hash/commands/evidence | this file: Routerich AX3000 v1, Dongle-P, SHA256, commit, commands, payload artifacts |

## Limits

- R1 is a core milestone: no converter database, no per-device HA discovery,
  no OTA/scenes/full Windfront parity, no CC2531 acceptance.
- Only coordinator-only HIL was required and performed; devices were already
  joined and interviewed.
- The state cache reset and the database format rewrite are real differences
  from Node and are listed above rather than claimed as parity.
