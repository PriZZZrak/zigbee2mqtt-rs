# R1 — first hardware test on Routerich AX3000 v1 + Dongle-P

Date: 2026-11-05 (UTC). Status: **first HIL pass, network resumed, several
blockers remain**. No secrets (network key, MQTT credentials) are recorded here;
the device backup archive is stored outside the repository (see below).

## Environment (measured, not assumed)

- `ubus call system board`: `Routerich AX3000 v1`, `mediatek/filogic`,
  `aarch64_cortex-a53`, RouteRich/OpenWrt `24.10.8` (`r29233-443ec4032a`),
  kernel `6.6.144`.
- Overlay: **54.3 MB total**, initially **5.6 MB free** (89 % used), 497 MB RAM.
- Radio: `/dev/ttyUSB0`, `cp210x` USB-serial (SONOFF Dongle-P), ZNP
  `transport_rev=2 product_id=1`, Z-Stack `2.7`.
- Pre-existing install: `node-zigbee2mqtt 2.14.1-r1` + `node127` (22.23.4),
  `/opt/zigbee2mqtt` 53.1 MB; mosquitto 2.0.18 (`mosquitto-nossl`), running.
- Devices in the real `database.db`: 3 (two Lytko `L101Ze.v2`/`v2-TT`, one Tuya
  `TS0601` / `_TZE200_locansqn`, white-label Nous `TH01Z`).
- Network (from `coordinator_backup.json`): PAN `0x1a62`, extended PAN
  `00124b0024c2d400`, channel 11, default z2m network key (key not recorded).

## Backup

Before any change, `/etc/zigbee2mqtt` was archived on the router
(`/root/z2mback/snapshots/etc-zigbee2mqtt-20261105T092852Z.tar.gz`, sha256
`c9c1a93e…`) and downloaded with its manifest; the archive was verified
byte-for-byte. Loose copies in `/root/z2mback` were refreshed. The archive is
kept in the working directory under `backups/` and is git-ignored because it
contains credentials.

## Procedure

1. Stopped the original service and removed `node-zigbee2mqtt`, `/opt/zigbee2mqtt`
   (53.1 MB), then the now-unused `node127`/`node-bufferutil`/`node-utf-8-validate`
   runtime (another ~29 MB). Free overlay: 40.0 MB.
2. `opkg install /tmp/z2m-rs.ipk` (12.1 MB installed) — succeeded, service
   auto-started by procd.
3. Wrote `/etc/zigbee2mqtt-rs/config.yaml` from the original config, preserving
   the serial port, MQTT settings, device names and the backup's network
   parameters (`pan_id`, `ext_pan_id`, `channel`, `network_key`).
4. Copied the real `database.db` and `coordinator_backup.json` into
   `/etc/zigbee2mqtt-rs/`.
5. Staged the pinned converter package (`v26.115.1`) to
   `/etc/zigbee2mqtt-rs/herdsman-converters` (18.3 MB), because the configured
   GitHub Pages URL serves only `registry.json` (see blockers).
6. Restarted and inspected logs, MQTT (`mosquitto_sub` over an SSH tunnel) and
   the frontend (`curl`).

## Result

Working:

- Install, procd service start, and frontend listener on `0.0.0.0:8080`
  (`GET /` → 200, Windfront assets).
- Real `database.db` imported: 3 devices, 3 interviewed; no dummy device left.
- `bridge/info`: coordinator `0x00124b0024c2d400`, network PAN `6754`,
  extended PAN `0x00124b0024c2d400`, channel 11, `permit_join=false`.
- `bridge/devices` contains the coordinator plus the three real devices.
- Both Lytko thermostats publish live states (`local_temperature_l1/l2`,
  `fault_*`, `hysteresis_l1`, `energy_l1/l2`, `linkquality`, `last_seen`).
  With the user's `L101Ze_z2m.js` external converter saved through
  `bridge/request/converter/save`, their definitions are now
  `source: external`, `supported: true` with 20/34 exposes.
- The coordinator resumes its existing network; NV is **not** rewritten and
  `ZDO_STARTUP_FROM_APP` reports "restored network state from NV".
- The original `state.json` was copied and the bridge restored cached state for
  all 3 devices on startup. The sleepy Tuya sensor immediately published
  `temperature: 26.4`, `humidity: 45`, `battery: 45`, the alarm thresholds and
  the report intervals instead of an empty payload; `state.json` is persisted
  atomically every 30 s and on shutdown.
- MQTT commands resolve both friendly names and IEEE addresses (upstream
  `resolveEntity`): `zigbee2mqtt/0x404ccafffe5c4ce0/set
  {"system_mode":"heat"}` and `{"sensor_type":"10 кОм"}` now reach the device
  and the retained state shows `system_mode: heat`, `sensor_type: 10 кОм`,
  `running_mode: heat`.

Partial / failing:

- The Tuya `TH01Z` **live** frames observed so far contain only DP 10
  (`max_temperature`), a ~5 s heartbeat; temperature/humidity are reported at
  the device's configured 30/60-minute intervals. The original `state.json`
  also had its last temperature at 08:16 before shutdown, so the cached values
  are the upstream behavior between reports. The state cache restores them,
  but a fresh temperature/humidity report has not yet been observed on this
  bridge. (The `unsupported attribute type
  Invalid` warning was **not** from this sensor: it came from the Lytko
  thermostats' `seMetering` uint48 report and is fixed.)
- Lytko definitions are `source: generated` and `supported=false` (10 generated
  exposes) because user `external_converters/L101Ze_z2m.js` is not loaded. Native
  state conversion works, but the frontend/HA definition payload does not match.
- Retained topics for the old IEEE names (`zigbee2mqtt/0x1020…`,
  `zigbee2mqtt/0x404c…`) remain on the broker after the rename; they are stale
  and confuse consumers.
- The converter host at `https://prizzzrak.github.io/.../latest` returns only
  `registry.json`; device files 404 and the bridge retried `nous.js` every 5 s
  until the full package was staged locally.

## Defects found and fixed during this test

1. **NIB layout**: real CC2652P firmware returns a 116-byte **aligned** NIB; the
   bridge rejected it and took the legacy path. `parse_nib` now decodes the
   aligned offsets (`nwkPanId` 36, `channelList` 40, `extendedPANID` 57) and the
   packed 112-byte variant (33/36/53).
2. **`PRECFGKEY` parsing**: NV `0x0062` is a raw 16-byte key on Z-Stack 3, not a
   17-byte key descriptor; parsing it as a descriptor aborted startup.
3. **Destructive unknown-strategy fallback**: when the NV probe failed, the
   bridge wrote NV and commissioned a network on every start. It now resumes
   without rewriting NV and fails explicitly for a genuinely uncommissioned
   adapter.
4. **Coordinator IEEE**: `UTIL_GET_DEVICE_INFO` was parsed from byte 0
   (including the status byte), producing `0x124b0024c2d40000`. It now reads
   bytes `1..9`, yielding `0x00124b0024c2d400`.
5. **Packaging dependency**: the `.ipk` required `mosquitto-client`, which is not
   in the Routerich feeds (and the Rust binary does not use it). Removed;
   `mosquitto` is provided by `mosquitto-nossl`.
6. **Frontend `bridge/info` data contract**: the pinned Windfront HomePage and
   About pages crashed on missing `config.availability.enabled`,
   `zigbee_herdsman.version` and `os.node_version`. The bridge now publishes the
   full upstream-shaped config (raw YAML merged with runtime values and upstream
   defaults) plus the version fields; see FRONTEND-004.
10. **Tuya datapoint commands were rejected by the ZCL encoder**: setting
    `temperature_unit_convert` on the Nous/TH01Z sensor calls
    `sendDataPointEnum` -> `manuSpecificTuya.dataRequest` with the custom
    `dpValues` type (1011). The encoder logged `ZCL type 0x03F3 needs command
    context` and dropped the command. Added the Tuya datapoint encoder
    (`seq:u16LE, dp:u8, datatype:u8, length:u16BE, data`) for
    `dataRequest`/`sendData`; HIL: fahrenheit/celsius commands are now sent
    without the warning.
9. **External converter save timed out / did not load**: `converter/save` had
   no handler and external converters were never loaded. Implemented
   save/remove/load, retained `bridge/converters`, external registry precedence
   and background device republish (Boa takes ~6-9 s on the router, so the
   response must not wait for the expensive definition republish). See
   CONVERTER-022.
8. **Missing `config_schema`**: the Settings pages crashed on
   `config_schema.properties`. The bridge now publishes a hand-written
   compatibility schema (root sections plus device/group definitions); see
   FRONTEND-004.
7. **ZCL 40/48/56/64-bit types**: the Lytko thermostats report `seMetering`
   `currentSummationDelivered` (0x0000) as **uint48 (0x25)**. The attribute
   parser only knew up to 32-bit types, logged `unsupported attribute type
   Invalid` every report and dropped `energy`. Added 0x1A/0x1B bitmap and
   0x24..0x27 / 0x2C..0x2F integer types; the thermostats now publish
   `energy_l1`/`energy_l2` (e.g. 0.032/0.005 kWh) and the warning is gone.

A defensive decision was added: when the adapter backup matches the network but
the alternate-key descriptor is missing/different, the bridge resumes with a
warning instead of refusing (upstream would refuse; refusing makes a known-good
network unstartable). This never rewrites NV; see `docs/parity/decisions.md`.

## Operational note: duplicate MQTT commands (fixed 2026-11-05)

The router's mosquitto bridges all topics to the Home Assistant broker on the
other side of the VPN (`connection ha-bridge`, `topic # both 0`) so that
Zigbee2MQTT keeps working when the VPN drops. The bridge was configured with
`try_private false`, which disabled Mosquitto's bridge loop protection: every
local publish was forwarded to HA and echoed back, so local subscribers (and
the bridge) saw each message twice.

Fix applied on the router (backup:
`/root/z2mback/mosquitto.conf.20261105T210942Z`): `try_private true`, then
`/etc/init.d/mosquitto restart`. Verification:

- an independent subscriber receives a single copy of a probe publish
  (`test/loop-probe`, previously two);
- the HA bridge reconnects (`192.168.0.1:<port> -> 192.168.0.30:1883`
  ESTABLISHED);
- Zigbee2MQTT reconnects (`MQTT connected and subscribed`) and continues
  publishing device state.

The HA broker is Mosquitto and supports the bridge flag, so `try_private true`
is safe here. If a future remote broker does not support it, use directional
`topic ... out/in` rules instead.

## Not yet verified

- Full sensor decoding, external-converter loading, stale retained-topic cleanup.
- Restart/power-loss persistence, coordinator reconnect, interview/reporting and
  command paths on hardware.
- Home Assistant discovery against a live HA instance.
- Rollback (`opkg install node-zigbee2mqtt`) was not exercised, though the
  package is available in the router feed (`2.14.1-r2`).

## Artifacts

The installed `.ipk` used for this test was rebuilt with the fixes above; the
final hashes are recorded in `RELEASE-001-openwrt-artifact.md`. The router is
running the same aarch64 binary with the fixes, with the converter package and
device data staged locally.
