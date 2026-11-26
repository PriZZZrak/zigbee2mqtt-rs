# HIL readiness report — real converter transport

Status: **ready for a controlled HIL**, not executed in this task. No Node.js
service was stopped, no serial port was opened, no Zigbee command was sent and
the overlay was not modified.

## 1. Build under test

| Item | Value |
|---|---|
| Target | `aarch64-unknown-linux-musl`, Routerich AX3000 v1, OpenWrt 24.10.8 |
| Binary | `target/aarch64-unknown-linux-musl/release/z2m-rs` |
| Binary sha256 (unstripped) | `427d494eed0ea565bed5e4f22dca183ca0e95c4731af9333d3920672b1bf7834` |
| Stripped binary | 16,681,368 B, sha256 `5c22e447fee193ad25809179e92bd22e505664b58723841d9662ee377991e95c` |
| Artifact manifest sha256 | `3d96a3c64e45c1ccf94abfa334ac1abca933ca199cbba7af67380403c4819050` |
| Index sha256 (`index.z2idx1`) | `49f5186fcaadc9aaa85e725f98d366a3ebd168618775f85e3c823d327cc2c182` |
| Git commit | the commit that adds this report (`git log -- docs/hil-readiness.md`) |

Pure Rust: no C/asm crypto, `rumqttc` with `use-rustls-no-provider` and the
RustCrypto provider; aarch64 musl static build.

## 2. Automatic tests (all green)

| Area | Result |
|---|---|
| Golden runtime corpus | **36 / 36**, 0 unexpected host-call differences |
| Resolver differential | **0 / 11,562** (JSON and `z2idx1`) |
| ZCL corpus | **415 / 415** (2 duplicate-ID buffers asserted as errors on both sides) |
| ZCL frame options/custom cluster | **10 / 10** |
| ZDO frames | **28 / 28** requests+responses (bind/unbind unicast/multicast, address/descriptor/table requests, status/error) |
| Cache manager | 15 tests (generations, fsync/prune order, LKG, budgets, crash, cancellation) |
| Lazy loader | 8 tests (real nous/IKEA/Tuya, offline restart, runtime generations) |
| Dry-run transport | 5 tests (oracle frames, cached reads, custom cluster, concurrency, strict option validation) |
| Real Endpoint transport (mock adapter) | 3 tests (send via adapter, read payload, no mock fallback, bind/unbind ZDO, status errors) |
| Lifecycle | 2 tests (JS fromZigbee/toZigbee/onEvent, exposes == pinned upstream, timeout, restart, leave) |
| Pairing lifecycle | 1 test (existing devices first, bounded queue, 120 reports during pairing) |
| HTTPS fetcher | 5 local TLS tests + 1 public-mirror run |
| Bridge/extension suite | 9 converter bridge tests + 12 lib tests |

## 2a. Deployment readiness

- `deploy/build.sh <artifact> <out>` builds the aarch64-musl binary, refuses to
  ship it unstripped, stages the binary, manifest, index, shared JS tree and the
  pinned `zigbee-herdsman` JS (no maps/types) and records sha256. The staged
  payload is 16,681,368 B (binary) + 3.7 MB artifact (42 files, no device
  catalog) + 3.6 MB herdsman JS.
- Without `--with-devices` the payload has **no device catalog**; new pairing
  requires `Z2M_CONVERTER_FETCH=https` (trusted manifest retrieval) or a
  separately staged module directory. With `--with-devices`, `dist/devices` is
  the offline module source.
- `procd` service: exclusive serial-port ownership, respawn, nofile limit and
  `ulimit -v 196608` (192 MB). The measured Routerich RSS is below that.
- Cache/runtime generations keep `active.json` + `lkg.json`; rollback is a
  pointer copy (or artifact-directory restore) and a restart, see
  `deploy/openwrt/README.md`.
- The payload is **staged only**; the live overlay, the Node.js service and the
  Dongle-P were not touched.

## 3. Live devices (read-only database snapshot)

| Type | IEEE | Manufacturer | Model | Power |
|---|---|---|---|---|
| Coordinator | `0x00124b0024c2d400` | — | Dongle-P | — |
| EndDevice | `0xa4c13887bf43cc0f` | `_TZE200_locansqn` | `TS0601` | Battery |
| Router | `0x404ccafffe5c4ce0` | `LYTKO` | `L101Ze.v2` | Mains |
| Router | `0x1020bafffef50ee8` | `LYTKO` | `L101Ze.v2-TT` | Mains |

Simulated in tests with the real definitions: IKEA `TRADFRI bulb E27 WS opal
1000lm`, Tuya `TS0002`, Aurora `1GBatteryDimmer50AU`, external Lytko
`L101Ze.v2`.

## 4. Allowed operations for the HIL window

Safe (read-only / health):

- `bridge/info`, `bridge/devices`, `bridge/health_check`, `bridge/state`;
- `bridge/networkmap` (ZDO LQI/routing/binding-table reads);
- attribute reads on `genBasic`, `genPowerCfg`, `genIdentify`, `genOnOff`,
  `genLevelCtrl` and definition-supported manufacturer clusters;
- `bridge/request/device/interview` only for a device already interviewed (it
  re-reads descriptors).

Requires explicit approval per command:

- `configureReporting` on a live device (writes device reporting state);
- `bind`/`unbind` (writes binding tables; use a dedicated device);
- `set` (`state`, brightness) on a Lytko relay (physically switches the load);
- `permit_join` with a window ≤60 s (pairing);
- `device/rename`, `options`, `device/remove` with leave/block.

Never:

- `StartCommissioning`, backup restore/`beginRestore` (NIB is rewritten);
- OTA flashing;
- long/open permit-join on the live network;
- two services on the Dongle-P serial port at the same time.

## 5. Backups and rollback

Before the window:

1. Copy the Node.js data directory (`/etc/zigbee2mqtt`: `configuration.yaml`,
   `database.db`, `state.json`, `coordinator_backup.json` if present) and record
   sha256 of every file.
2. Save the staged artifact (`manifest.json`, `index.z2idx1`, `dist/`), its
   manifest sha, and the stripped binary sha.
3. Keep the existing Node.js binary/init and the previous Rust binary.
4. Coordinator backup: capture via the existing Node.js backup path or the
   agreed tool; store off-device and verify it opens.

Rollback conditions:

- coordinator does not resume the existing network;
- a device disappears or reports errors after the switch;
- MQTT/HA state diverges or reports are lost;
- the process exceeds the memory limit or restarts repeatedly.

Rollback steps:

1. Stop the Rust service; verify `/dev/ttyACM0` (or the configured port) has no
   holder.
2. Cache/runtime: copy `lkg.json` over `active.json` (or restore the previous
   artifact directory and clear the cache).
3. Restore the Node.js data directory from the verified copy if it was touched.
4. Start the Node.js service; verify `bridge/state=online`, device count and
   one report.
5. Restore the coordinator backup only if the NIB is damaged (not expected from
   the operations above).

## 6. Known limits

- The real transport has not transmitted to a physical coordinator. Its send
  path is the existing `Endpoint`/adapter code, exercised end-to-end against a
  recording `ControllerAdapter`; hardware evidence is the HIL itself.
- No artifact mirror exists; `HttpsFetcher` is validated against public mirrors
  and a local TLS server, not the future artifact host.
- Routerich overlay has ~568 KB free; the payload must be installed only after
  the agreed migration (external storage or Node removal).
- PERF-001 (≤0.94 s single-report stall during manual pairing) remains a
  low-priority debt.

## 7. GO / NO-GO

**GO for the controlled HIL** (Variant A preferred, Variant B only with the
approved maintenance window), provided the backups above exist and the exact
binary/artifact hashes are recorded in the window log.

**NO-GO for production enablement** until: HIL evidence is captured; the real
transport is validated on hardware; the artifact mirror exists and is used;
the overlay/procd migration is exercised; ZDO request/response corpus is
extended if the HIL reveals adapter-specific framing.
