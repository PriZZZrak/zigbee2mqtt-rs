# HIL: crash / kill -9 / respawn recovery (2026-11-17)

Audited item: CP-04 in `docs/test-coverage-audit.md`.

## Environment

- Routerich AX3000 v1, RouteRich/OpenWrt 24.10.8 (aarch64), SONOFF Dongle-P
  `/dev/ttyUSB0`, local mosquitto `:1883`.
- Binary: `target/aarch64-unknown-linux-musl/release/z2m-rs` (static musl,
  commit `0c4f0d7` at build time), piped to `/tmp/z2m-rs-bin`.
- Data: copy of `/etc/zigbee2mqtt/{configuration.yaml,database.db,state.json}`
  in `/tmp/z2m-rs-crash` with `base_topic: z2m-rs-crash`, frontend
  `127.0.0.1:8091`, HA discovery disabled for the run.
- The Node service was stopped for the run and restarted afterwards; the
  coordinator was resumed, never re-commissioned.

## Run

```sh
# run 1: clean start, rename through the bridge API, then kill -9
started_lines=1 waited=10s
rename {"data":{"from":"Термостат спальная/детская","homeassistant_rename":false,"to":"LYTKO crash rename"},"status":"ok"}
before_kill9 cfg=afef7d9801688f0e810af222fd668187166223b2ffa176219946d88041baddb6
            db=26614ec4f8701a9198be00df9f49286df435e3fd6cc7e51ebed1a059bc6f2d5b
            state=03cb19fbf587366557006a4821b58e53862b6b54b5154c0603166d2c604d7e9e

# kill -9 while a device interview request was in flight
ps w | grep -c '[z]2m-rs-bin'  -> 0

# run 2: respawn
started_lines=2 waited=1s
after_respawn cfg=afef7d9801688f0e810af222fd668187166223b2ffa176219946d88041baddb6 same=yes
grep -c "LYTKO crash rename"  -> 1
database entries 4 parseable true
state keys 3
http_after_respawn=200
grep -c "Started with result 'Resumed'" -> 2
```

## What is proven

- `configuration.yaml` is written synchronously and survives `kill -9`; the
  rename is present after respawn (identical SHA256).
- `database.db` stays parseable (4 NDJSON entries, the live devices) and
  `state.json` (3 keys) is readable after the crash.
- The bridge resumes the coordinator (`Resumed`) and serves HTTP after the
  respawn; MQTT `bridge/state` was published again (the reconnect tests in
  `crates/z2m-rs/tests/mqtt_reconnect.rs` cover the broker side).
- `yaml::write_if_changed` and the `state.json` save now use temp-file + rename,
  so an interrupted write cannot truncate the previous file (rollback-safe).

## Not covered by this run

Reboot with the **packaged** service (`/etc/init.d/zigbee2mqtt-rs` from the
signed feed) is not possible while `node-zigbee2mqtt` occupies 53.6 MB of the
63 MB overlay: installing `zigbee2mqtt-rs` + `zigbee2mqtt-windfront` needs
~19 MB. This needs the user decision recorded in
`docs/openwrt-packaging.md` ("Status"): remove the Node package (re-installable
from the same opkg feed) and switch to the Rust service.
