# HIL plan — real Endpoint transport acceptance

Status: **plan only, not executed.** Hardware-in-the-loop work needs explicit
user approval and a coordinated redeploy. The test router currently runs the
Node.js Zigbee2MQTT; `zigbee2mqtt-rs` is not installed there.

## 1. Scope

Validate the real `Endpoint` transport and the lazy converter lifecycle on the
R1 hardware:

- Routerich AX3000 v1, OpenWrt 24.10.8 (aarch64), SONOFF Dongle-P
  (CC2652P / Z-Stack);
- coordinator starts/resumes the existing network without NV rewrite;
- a real (or explicitly dedicated) device exercises `read`, `write`,
  `configureReporting`, `bind`, standard and manufacturer-specific commands;
- lazy startup prepares paired devices from the artifact cache; a new device is
  paired through the background queue; reports continue during pairing;
- MQTT, Home Assistant and Windfront stay responsive throughout.

Out of scope: `StartCommissioning`, backup restore, OTA, network-key changes,
any operation that re-forms the network.

## 2. Forbidden operations

These must never be issued against a live installation:

- `StartCommissioning` / `beginRestore` (NIB is deleted, devices lose the key);
- `permit_join` with a long window on a live network without a dedicated
  device and a second operator;
- `device/remove` with `force`/leave on production devices;
- firmware/OTA flashing;
- any write to `ref/configuration.yaml`, the Node.js data directory, or the
  production overlay without a migration plan.

## 2a. Two deployment variants

### Variant A — dedicated test coordinator (preferred)

- Use a separate Dongle-P/test coordinator flashed with the same Z-Stack
  coordinator firmware and a dedicated test network.
- The live Dongle-P and the Node.js service stay untouched for the whole run.
- Pair one dedicated lamp/switch to the test network; no production devices.
- The Rust service owns the test serial port exclusively.

### Variant B — agreed switchover of the existing Dongle-P

Only inside an approved maintenance window and with a verified backup:

1. Stop the Node.js service and wait until the serial device is released.
2. Verify no process holds the port (`fuser /dev/ttyACM0`, `lsof`); never run
   both services.
3. Start the Rust service with the existing network database and
   `Z2M_CONVERTER_TRANSPORT=endpoint`.
4. Run the safe operation order below; capture all evidence.
5. Rollback: stop the Rust service, confirm the port is free, restart the
   Node.js service with its untouched data directory.

Simultaneous access by two services to one serial port is forbidden. The
switchover must record the exact stop/start times and the port owner.

## 3. Prerequisites

1. Dedicated test coordinator (or an explicit maintenance window agreed with
   the user); the live installation must not share it.
2. Built binary recorded: `sha256sum` of the aarch64 musl `z2m-rs`, target
   triple, git commit and artifact manifest hash.
3. Artifact installed on the overlay (shared tree ~2.4 MB + selected modules)
   or on external storage; the overlay currently has ~568 KB free, so the
   package migration must be agreed first.
4. Isolated data directory (`Z2M_DATA_DIR`) and MQTT topic prefix. Variant A
   keeps Node.js running; Variant B stops it for the window and keeps its data
   directory untouched.
5. `Z2M_CONVERTER_LAZY=1`, `Z2M_CONVERTER_TRANSPORT=endpoint` (real transport;
   this build only supports `dry-run`), artifact/herdsman/cache/runtime paths
   set explicitly.
6. Coordinator backup captured and verified before the run:
   `zigpy`/`bellows` or the existing Node backup, stored off-device.

## 4. Safe operation order

The first HIL is limited to network restore, state verification and **one safe
read**. Later phases are approved separately.

### Phase 1 (first HIL)

1. **Offline start**: no coordinator attached. Verify lazy startup resolves the
   `database.db` devices from cache, the runtime generation is complete, and
   `bridge/state` is `online` only after MQTT connects.
2. **Coordinator resume**: attach Dongle-P, start with the existing network
   parameters (no `StartCommissioning`). Confirm the network is not re-formed
   and the coordinator IEEE is unchanged.
3. **State verification**: `bridge/info`, `bridge/devices`, device count and
   availability; one safe read on one agreed device (e.g. `genBasic` model or
   `genPowerCfg` battery), no writes.
4. **Stop/rollback** and capture the evidence.

### Later phases (separate approval)

5. **Existing devices**: read one attribute per device, configure reporting on
   one benign attribute, and issue one `set` per distinct command family
   (on/off, level, color if applicable). Verify MQTT state round-trip.
3. **Existing devices**: read one attribute per device, configure reporting on
   one benign attribute, and issue one `set` per distinct command family
   (on/off, level, color if applicable). Verify MQTT state round-trip.
6. **Bind/unbind** only on an explicitly dedicated device (e.g. a lamp bound to
   the coordinator group) and revert the binding afterwards.
7. **Pairing**: permit join for ≤60 s and pair one dedicated device. Observe
   the bounded pairing queue, no report loss for existing devices (MQTT
   sequence counters), and p95 report latency.
8. **Restart recovery**: `kill -9` the process, respawn, verify cache/state/
   runtime generations and that the network and devices are intact.
9. **Reboot** only after the restart test passes; verify the same.

## 5. Rollback

- Keep the previous binary and artifact generation; `active.json`/`lkg.json`
  allow the cache and runtime to roll back without regenerating.
- Stop the Rust service and restart the Node.js service with its untouched data
  directory and configuration.
- Restore the coordinator backup only if the NIB is damaged (not expected from
  the operations above).
- Remove the test MQTT retained topics under the test prefix.

## 6. Evidence to record

- binary hash, commit, artifact manifest hash, OpenWrt version, coordinator
  firmware;
- command transcript with timestamps, MQTT topics/payloads, HA entity ids;
- pairing timeline (queue length, pairing duration, report p50/p95 during
  pairing, any backpressure events);
- cache/runtime directory listing and `active.json`/`lkg.json` contents;
- restart/reboot logs and data-loss check (`database.db`, `state.json`,
  `configuration.yaml` hashes before/after).

## 7. Readiness evidence

`docs/hil-readiness.md` lists the automatic tests, live devices, allowed
commands, backups and the exact rollback conditions for this build. HIL starts
only when that report is GO.

## 8. Approval gate

HIL starts only after the user approves:

1. the dedicated coordinator/maintenance window;
2. the overlay/package migration;
3. the exact commands above and the rollback plan.

The 2026-11-10 destructive-path evidence must not be repeated.
