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
