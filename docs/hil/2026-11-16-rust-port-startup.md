# HIL run: Rust port on Routerich AX3000 (2026-11-16)

## Environment

- Routerich AX3000 v1, RouteRich/OpenWrt 24.10.8 (r29233), aarch64_cortex-a53.
- SONOFF Dongle-P (`10c4:ea60`, cp210x) at `/dev/ttyUSB0`, coordinator firmware
  `ZStack3x0` (transportrev 2, product 1, majorrel 2, minorrel 7, maintrel 1,
  revision 20250321).
- Local `mosquitto` on `127.0.0.1:1883`; no `mosquitto_sub`/`mosquitto_pub` on
  the router, MQTT checks use the `mqtt` 5.16.1 package bundled with the Node
  installation.
- The Node.js Zigbee2MQTT service was stopped **for the tests only** (user
  approved) and restarted afterwards. The coordinator was resumed, never
  re-commissioned or restored; permit join was opened for **1 second**.
- Binary: `cargo zigbuild --release -p z2m-rs --target aarch64-unknown-linux-musl`
  (12.2 MB), copied to `/tmp/z2m-rs-bin` (scp is unavailable on the router, the
  binary is piped over SSH).
- Data: copy of `/etc/zigbee2mqtt/{configuration.yaml,database.db,state.json}`
  in `/tmp/z2m-rs-test` with `base_topic: z2m-rs-test`, frontend
  `127.0.0.1:8090`, HA discovery disabled for the test.
- Windfront assets: `ZIGBEE2MQTT_FRONTEND_ASSETS=/opt/zigbee2mqtt/node_modules/zigbee2mqtt-windfront/dist`
  (2.15.1, same version as `packaging/windfront.version`).

## Final result (run 4)

```
info: z2m: Starting Zigbee2MQTT version unknown (commit #unknown)
info: z2m: Starting zigbee-herdsman (0.1.0)
info: z2m: zigbee-herdsman started (resumed)
info: z2m: Coordinator firmware version: '{"meta":{"maintrel":1,"majorrel":2,"minorrel":7,"product":1,"revision":20250321,"transportrev":2},"type":"ZStack3x0"}'
info: z2m: Термостат спальная/детская (0x1020bafffef50ee8): Not supported (Router)
info: z2m: Датчик температуры (0xa4c13887bf43cc0f): Not supported (EndDevice)
info: z2m: Термостат кухня (0x404ccafffe5c4ce0): Not supported (Router)
info: z2m: Currently 3 devices are joined.
info: z2m: External JS (converters/extensions) is disabled
info: z2m: Serving Windfront from '/opt/zigbee2mqtt/node_modules/zigbee2mqtt-windfront/dist'
info: z2m: Started frontend on port 127.0.0.1:8090
info: z2m: Zigbee2MQTT started!
info: z2m: Zigbee: allowing new devices to join.
```

MQTT (probe subscribing to `z2m-rs-test/#`):

```
bridge/state: {"state":"online"}
bridge/info: {"commit":"unknown","config":{...}}
bridge/devices: [ ... 3 devices ... ]
bridge/groups: []
bridge/response/permit_join: {"data":{"time":1},"status":"ok"}
```

HTTP (frontend on `127.0.0.1:8090`):

```
root=200
/assets/index-DjcCHtis.js -> 200, 1.4 MB
/devices/0x00124b (Accept: text/html) -> 200 (SPA fallback)
```

"Not supported" is expected for R1: the device catalog
(`zigbee-herdsman-converters`) is out of scope, devices stay definition-less
and follow upstream's not-found fallback.

## Bugs found and fixed during this HIL session

1. **Panic on the first attribute report** — `Entity.adapter was not injected`
   (`crates/zigbee-apiarist/src/controller/model/entity.rs:109`). The real
   CC2652 sends reports immediately after `Resumed`, but the adapter was
   injected only after `adapter.start()`; it is now injected before start
   (`controller/controller.rs`).
2. **Console logging was memory-only** — the z2m `Logger` collected console
   lines but never wrote them to stdout, hiding every startup error. The
   console transport now prints (`util/logger.rs`).
3. **No frame-level visibility** — `-->`/`<--` debug lines were missing, so a
   ZNP stall could only be seen as a timeout. `ZpiObject::to_string_with_frame`
   (upstream `toString(includeFrame)`) plus `ZIGBEE2MQTT_TRACE_SERIAL=1`
   (raw TX/RX chunks) were added; a differential test pins the format.
4. **Leftover `eprintln!` debug output** in the backup merge path
   (`MERGE stored=…`) — removed.
5. **`server: mqtt://localhost` was rejected** — `parse_server` required an
   explicit `:port`, so the bridge exited with "Invalid MQTT server". Ports now
   default per scheme (1883/8883) as in mqtt.js, including bracketed IPv6.
6. **`bridge/devices` publish failed** — rumqttc defaults to a 10 KiB outgoing
   packet limit, so the 28 KiB device list was rejected ("greater than the
   broker's maximum packet size of: 10240"). The outgoing limit is now the
   protocol maximum; `mqtt.maximum_packet_size` keeps describing the incoming
   direction (upstream semantics).
7. **Windfront version check only looked inside the assets directory** — the
   development layout keeps `package.json` one level above `dist/`, so the
   frontend refused to start. The check now also looks in the package root.

## Feed artifact verification (same session)

The published packages were downloaded back from `gh-pages` and exercised on
the router, so the feed is known to contain working artifacts:

- `feed/Packages` SHA256 matches `zigbee2mqtt-rs_0.0.1-dev.1_aarch64_cortex-a53.ipk`
  (`33f5ea7a…`), `ar t` = `debian-binary control.tar.gz data.tar.gz`,
  `Installed-Size: 12008`, `Depends: zigbee2mqtt-windfront (= 2.15.1)`.
- Extracted `data.tar.gz`: ELF64 AArch64 binary (12.2 MB) plus the procd init
  script (`sh -n` clean).
- The CI binary ran with the packaged Windfront assets
  (`/tmp/wfpkg/usr/share/zigbee2mqtt/windfront`): coordinator resumed,
  `Zigbee2MQTT started!`, HTTP 200 on the frontend port, and the MQTT probe
  reported `bridge/state`, `bridge/info`, `bridge/devices`, `bridge/groups`
  and `permit_join` → `{"status":"ok"}`.

## Notes for the next run

- Startup (coordinator resume + full coordinator backup) takes ~10 s on this
  device because the Z-Stack 3.x extended NV tables (ADDRMGR, APS key data,
  TCLK, network security material) are enumerated subid by subid, as upstream
  does. It finishes; the earlier "hang at Creating coordinator backup" was
  simply a 12 s observation window.
- Killing the process mid-request can leave one unanswered response in the
  coordinator's transmit queue; the next session parses it, finds no waiter and
  drops it. Only the first request after such a kill was affected.
- `/dev/ttyUSB0` is locked while the Node service runs; stop it and wait for
  `index.js` to disappear before starting the Rust binary.

## R1 acceptance matrix (bridge requests, persistence, HA, WebSocket)

Run against the live coordinator with a copied data dir (`base_topic: z2m-rs-test`,
frontend `:8090`, HA discovery enabled).

| Check | Result |
|---|---|
| `bridge/request/health_check` | ok `{healthy: true}` |
| `bridge/request/options` (`{options:{advanced:{log_level}}}`) | ok, persisted |
| `bridge/request/device/interview` (router) | ok + `bridge/event device_interview {status: successful}` |
| `bridge/request/networkmap` (`type: raw`) | ok, 5 nodes |
| `bridge/request/device/rename` | ok; `configuration.yaml` updated on disk |
| `bridge/request/device/remove` (`force+block`, fabricated IEEE) | ok; DB entry removed, `blocklist` persisted |
| `bridge/request/restart` | ok; process exits, supervisor restart completes |
| Windfront WebSocket | replay (`bridge/state|info|devices|groups` + cached device states), inbound request + `bridge/response/permit_join` |
| Home Assistant discovery | 90 `homeassistant/#` topics |
| `state.json` | rewritten (3 keys), `database.db` parseable, fake device gone |
| HTTP on the frontend port | 200 via curl and via a Node `http.get` client (root 886 B, `assets/index-*.js` 1.4 MB), 203 ms while a networkmap scan was running |

Bugs found and fixed in this run:

- **Settings were never persisted**: `Settings` had no data-directory backing, so
  `bridge/request/options`, `device/rename`, `device/remove` (blocklist) and
  `group/*` only changed memory. Added `Settings::read_data_dir` (upstream
  `read()`), real `write()` to `configuration.yaml` + split devices/groups
  files, `write()` calls in the mutators (upstream `settings.ts:206-269`), and
  `main` now fails hard on an unreadable configuration instead of silently
  continuing. Tests: `settings_persist_mutations_to_disk`,
  `settings_write_updates_split_devices_file`.
- **Duplicate `Content-Length` broke Node HTTP clients**: the static file server
  already set `content-length`, and `write_response` added a second one (304
  responses also carried a body-length header). Node's llhttp rejects duplicate
  framing headers, so a Node client saw a socket error while curl tolerated it.
  `write_response` now emits a single `Content-Length` and no body/header for
  304; regression test `serves_single_content_length_and_empty_304`.
- **`bridge/state` was missing from the Windfront replay**: the CONNACK handler
  published it through the raw rumqttc client, bypassing the retained-message
  bookkeeping. `retained_messages` is shared with the event loop and the state
  (and the offline state on disconnect) is recorded like upstream
  `Mqtt.publish`.
