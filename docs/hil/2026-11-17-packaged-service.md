# HIL: packaged service install + reboot (2026-11-17)

Audited items: CP-04 (reboot with the packaged service) and CP-12 (installed
service acceptance).

## Install

- Backup before the switch: `/tmp/z2m-node-backup-20261117-090330.tar.gz`
  (copy in `/tmp/opencode/backups/`), `opkg list-installed` recorded too.
- `node-zigbee2mqtt 2.14.3-r1` removed on user approval (re-installable from the
  same `custom_node` feed with `--force-space`). Overlay after removal: 19.6 MB free.
- **The first install exposed a packaging bug**: OpenWrt 24.10's opkg rejects an
  `ar`-format .ipk as *"Malformed package file"*; the feed packages are now built
  as a **gzipped tar** with `./debian-binary`, `./data.tar.gz`,
  `./control.tar.gz` (the format the router's own feed uses).
- Installed locally from the rebuilt ipks:
  `zigbee2mqtt-windfront 2.15.1` (all) and `zigbee2mqtt-rs 0.0.1-dev.2`
  (aarch64_cortex-a53). Files: `/usr/bin/zigbee2mqtt-rs` (12.4 MB),
  `/etc/init.d/zigbee2mqtt-rs`, `/usr/share/zigbee2mqtt/windfront` (6.6 MB).

## First start

```
/etc/init.d/zigbee2mqtt-rs enable  -> S99zigbee2mqtt-rs
/etc/init.d/zigbee2mqtt-rs start   -> running
info: Currently 3 devices are joined.
info: External JS (converters/extensions) is disabled
info: Serving Windfront from '/usr/share/zigbee2mqtt/windfront'
info: Started frontend on port 0.0.0.0:8080
info: Zigbee2MQTT started!
http root=200
mosquitto_sub zigbee2mqtt/bridge/state -> {"state":"online"}
```

## Reboot recovery

Pre-reboot hashes:
```
configuration.yaml 1756e7f249b9ce1c40eb38ed02530b6c8595b8ef2deb5a23ea52cf43130cfe6d
database.db        ec058db28e35c71ed205371a3dd3485d748d9608522d7d942c1f8ba834e392f9
state.json         3c57d6e67e77e0186f5252a684db0f8392a2378d3432f97d243a0d53c8e2effd
```

`reboot` -> SSH back after 80 s:

```
/etc/init.d/zigbee2mqtt-rs status      -> running
http root=200
mosquitto_sub zigbee2mqtt/bridge/state -> {"state":"online"}
bridge/devices                         -> 3 devices
configuration.yaml 1756e7f2... (identical)
database.db 7081 bytes, state.json 1848 bytes present
device reports + default responses flowing (dataRequest/dataConfirm AREQs)
df /overlay -> 10.4 MB free
```

The service starts automatically after boot (procd `S99zigbee2mqtt-rs`), resumes
the coordinator from `database.db`, keeps the network identity and serves the
packaged Windfront at `:8080`.
