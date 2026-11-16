# HIL run: Rust port on Routerich AX3000 (2026-11-16)

## Environment

- Routerich AX3000 v1, RouteRich/OpenWrt 24.10.8 (r29233), aarch64_cortex-a53.
- SONOFF Dongle-P (`10c4:ea60`, cp210x) at `/dev/ttyUSB0`.
- Local `mosquitto-nossl` on `127.0.0.1:1883`.
- The Node.js Zigbee2MQTT service was stopped **for the test only** (user
  approved) and restarted afterwards; the coordinator was resumed, never
  re-commissioned or restored.
- Binary: `cargo zigbuild --release -p z2m-rs --target aarch64-unknown-linux-musl`
  (12.3 MB), copied to `/tmp/z2m-rs-bin` (scp is unavailable on the router, the
  binary was piped over SSH).
- Data: copy of `/etc/zigbee2mqtt/{configuration.yaml,database.db,state.json}`
  in `/tmp/z2m-rs-test` with `base_topic: z2m-rs-test`, frontend
  `127.0.0.1:8090`, HA discovery disabled for the test.
- Windfront assets: `ZIGBEE2MQTT_FRONTEND_ASSETS=/opt/zigbee2mqtt/node_modules/zigbee2mqtt-windfront/dist`
  (2.15.1, same version as `packaging/windfront.version`).

## Run 1 — panic during startup

```
[zh:adapter:zstack:manager] beginning znp startup
[zh:controller] Started with result 'Resumed'
thread 'main' panicked at crates/zigbee-apiarist/src/controller/model/entity.rs:109:
Entity.adapter was not injected
```

The coordinator resumed, a thermostat attribute report arrived immediately, and
the ZCL response path called `Entity.adapter()` before `Entity::inject_adapter`
ran (injection happened after `adapter.start()`; upstream only registers
adapter events after the injection, while this port dispatches frames from
`start`).

**Fix:** `Entity::inject_adapter` now runs before `adapter.start()`
(`crates/zigbee-apiarist/src/controller/controller.rs`).

## Run 2 — ZNP version detection / desync

After the fix the panic is gone, but the adapter handshake fails:

```
[zh:zstack] Failed to get zStack version, assuming 1.2
[zh:zstack:znp] Error while parsing to ZpiObject: CommandID '0' from subsystem 'RESERVED' not found
[z2m] error: SRSP - SAPI - readConfiguration after 6000ms
[z2m] error: Error while starting zigbee-herdsman
[z2m] info: Stopping Zigbee2MQTT (restart=false, code=1, signal=None)
```

The `SYS version` request times out and a malformed frame is parsed as
`RESERVED/0`, so the manager falls back to Z-Stack 1.2 and sends SAPI commands
to a CC2652 (Z-Stack 3.x), which times out. The Node.js service starts from the
same coordinator and port without this error.

Also fixed in this run: the z2m `Logger` recorded console output only in memory
(no stdout), hiding the startup error; it now writes the console transport to
stdout (`crates/z2m-rs/src/util/logger.rs`).

## Status

- Coordinator open + network resume works (`Started with result 'Resumed'`).
- Blocked before MQTT/frontend: ZNP startup/version detection needs debugging
  against the real CC2652 (likely bootloader-skip/leftover-buffer handling or a
  serial flush after open).
- The Node.js service was restored (`/etc/init.d/zigbee2mqtt status` → running).
