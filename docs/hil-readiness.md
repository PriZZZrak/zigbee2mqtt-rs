# HIL readiness report — real converter transport

Status: **ready for a controlled HIL**, not executed in this task. No Node.js
service was stopped, no serial port was opened, no Zigbee command was sent and
the overlay was not modified.

## 1. Build under test

| Item | Value |
|---|---|
| Target | `aarch64-unknown-linux-musl`, Routerich AX3000 v1, OpenWrt 24.10.8 |
| Binary | `target/aarch64-unknown-linux-musl/release/z2m-rs` |
| Binary sha256 (unstripped) | `ba8e89df698a2f94981019668cb30a227c9ea880f956c7a3324bd92f3fde4c61` |
| Stripped binary | 16,684,808 B, sha256 `ebb4b57959b5576004f5ffce34a2edf4bb27e0143788a9272d523af9b61102c4` |
| Artifact manifest sha256 | `5cd34c1226da5ec8dda6745b9d9b88a71596b664416c622b3b96dd1d28942ced` |
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
| Converter release flow | generator twice (determinism), `manifest_verify`, graph/split/asset gates, resolver 0/11,562, ZCL/ZDO, golden 36/36; real Actions run 46012408128 published and 413 assets verified |
| Real Release fetch | direct `tuya-1.js` hash `055da527…` and `ModuleCache` activation from the published Release (2 ignored tests, run manually) |
| Cache manager | 15 tests (generations, fsync/prune order, LKG, budgets, crash, cancellation) |
| Lazy loader | 8 tests (real nous/IKEA/Tuya, offline restart, runtime generations) |
| Dry-run transport | 5 tests (oracle frames, cached reads, custom cluster, concurrency, strict option validation) |
| Real Endpoint transport (mock adapter) | 3 tests (send via adapter, read payload, no mock fallback, bind/unbind ZDO, status errors) |
| Lifecycle | 3 tests (JS fromZigbee/toZigbee/onEvent, exposes == pinned upstream, static vs dynamic exposes cache, timeout+recovery, restart, leave) |
| Pairing lifecycle | 1 test (existing devices first, bounded queue, 120 reports during pairing) |
| HTTPS fetcher | 5 local TLS tests + public mirror + real public GitHub Release asset (CDN redirect) |
| Bridge/extension suite | 9 converter bridge tests + 12 lib tests |

## 2a. Deployment readiness

- `deploy/build.sh <artifact> <out>` builds the aarch64-musl binary, refuses to
  ship it unstripped, stages the binary, manifest, index, shared JS tree and the
  pinned `zigbee-herdsman` JS (no maps/types) and records sha256. The staged
  payload is 16,684,808 B (binary) + 3.7 MB artifact (42 files, no device
  catalog) + 3.6 MB herdsman JS.
- Without `--with-devices` the payload has **no device catalog**; new pairing
  requires `Z2M_CONVERTER_FETCH=https` (trusted manifest retrieval) or a
  separately staged module directory. With `--with-devices`, `dist/devices` is
  the offline module source.
- `procd` service: exclusive serial-port ownership, respawn, nofile limit and
  `ulimit -v 131072` (128 MB address space), based on the measured Routerich
  VmPeak (46–53 MB); RSS is ~27–33 MB and is monitored during HIL.
- Cache/runtime generations keep `active.json` + `lkg.json`; rollback is a
  pointer copy (or artifact-directory restore) and a restart, see
  `deploy/openwrt/README.md`.
- The payload is **staged only**; the live overlay, the Node.js service and the
  Dongle-P were not touched.

## 2b. GitHub Releases distribution

- Trusted store: `https://github.com/PriZZZrak/zigbee2mqtt-rs/releases/download/converter-vz2hc-26.125.0-zh-11.2.0-abi1/`.
- Release assets: `manifest.json`, `index.z2idx1`, `index.json`, `graph.json` and
  **408 flat module files** (e.g. `tuya-1.js`), 412 assets total (< 1000 cap).
  Example: `.../converter-vz2hc-26.125.0-zh-11.2.0-abi1/tuya-1.js`.
- The manifest `modules.retrieval` base is the same release URL; each module
  carries a flat `asset` name. The loader downloads only the selected closure,
  never a full archive.
- GitHub redirects to `release-assets.githubusercontent.com` /
  `objects.githubusercontent.com`; the fetcher allows only those hosts when the
  base is `github.com`, rejects HTTP/downgrade, arbitrary hosts and fragments,
  and keeps the redirect/time/size limits. Signed CDN query strings are allowed
  on redirects only.
- **Publication verified.** Actions run
  `https://github.com/PriZZZrak/zigbee2mqtt-rs/actions/runs/46012408128`
  succeeded (3m30s) and published the release. All 413 assets were downloaded
  and verified (`verify-release.js`): manifest `5cd34c12…`, index
  `49f5186f…`, `tuya-1.js` `055da527…`; `SHA256SUMS` matches.
- The fetcher fetched the published `tuya-1.js` both directly and through
  `ModuleCache` (size/SHA-256 verified before activation); the real
  `cli/cli` public asset test and the local TLS redirect suite also pass.

## 2c. SDK package and modes

- `packaging/openwrt-pkg/build.sh` copies the tracked package definition into
  the existing SDK, stages the stripped binary, manifest, index, shared JS and
  herdsman JS, and runs `make package/zigbee2mqtt-rs/compile`. The same staging
  helpers are used by `deploy/build.sh`; there is no second packaging pipeline.
- Package contents: binary (16.68 MB), artifact (3.7 MB, 41 files, **no device
  catalog**), herdsman JS (3.6 MB), init script, mode file. Installed footprint
  ≈ **24.1 MB** plus cache/runtime state (~0.5–1 MB).
- `mode` file selects the explicit configuration:
  `on-demand` → `FETCH=https`, `ALLOW_NETWORK=1`; `offline` → `FETCH=file`,
  `ALLOW_NETWORK=0` with a pre-seeded cache or a `--with-devices` package
  variant. Missing definitions are reported per device/module, never silently
  degraded.
- **SDK build verified in CI.** Actions run
  `https://github.com/PriZZZrak/zigbee2mqtt-rs/actions/runs/46016877970`
  succeeded (5m10s). The published ipk
  `feed/zigbee2mqtt-rs_0.0.1-hil1-r1_aarch64_cortex-a53.ipk`
  (sha256 `33f7abb87c76a923b71e8d4bbc715af8dc4db7c25bf57c7b6811198d64900648`,
  8,315,124 B, Installed-Size 23,818,240 B, `Depends: libc`,
  `aarch64_cortex-a53`) was downloaded and inspected: aarch64 ELF, manifest
  and index byte-identical to the Release, 41 artifact files, shared
  `dist/lib` + `dist/converters`, no `dist/devices`, init/mode present.

## 2d. Routerich memory

Measured with the lazy probe (same quickjs runtime, 20 paired devices, one
module cached; `/tmp` tmpfs):

| Mode | VmPeak | VmSize | VmRSS (peak) | Threads |
|---|---:|---:|---:|---:|
| 20 devices startup | 46.4–46.7 MB | ≈46.7 MB | 26.7–27.3 MB | 7–8 |
| pairing impact | 52.9 MB | 48.4 MB | 33.2 MB | 7 |

`ulimit -v 131072` (128 MB) replaces 192 MB: it bounds address space at ~2.4×
the measured probe peak and is still above the full bridge (MQTT/HA/frontend
add a few MB). RSS remains ~27–33 MB; VmRSS is recorded during HIL. The limit
is address space, not RSS, and will be revisited if many modules are active.

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
transport is validated on hardware; the overlay/procd migration is exercised
on the router; the ZDO corpus is extended if HIL reveals adapter-specific
framing. The converter Release, asset verification, real fetcher download and
SDK-built ipk are now verified (Actions runs 46012408128 and 46016877970).
