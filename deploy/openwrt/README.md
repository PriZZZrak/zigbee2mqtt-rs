# OpenWrt deployment (staged, not applied)

This directory prepares a reproducible payload. Applying it to the router and
touching `/overlay` requires explicit approval; the live Node.js service and its
data directory must stay untouched until the HIL window.

## Layout

```
/usr/bin/zigbee2mqtt-rs
/usr/bin/zigbee2mqtt-rs.sha256
/usr/share/zigbee2mqtt-rs/artifact/{manifest.json,index.z2idx1,dist/...}
/usr/share/zigbee2mqtt-rs/herdsman/{dist/,package.json}
/etc/init.d/zigbee2mqtt-rs
/var/lib/zigbee2mqtt-rs/cache      (created by procd)
/var/lib/zigbee2mqtt-rs/runtime
```

`deploy/build.sh <artifact-dir> <output-dir>` builds the aarch64-musl binary,
strips it (refuses to ship unstripped), copies only the shared artifact tree
(index, manifest, `dist/index.js`, `dist/models-index.json`,
`dist/converters/`, `dist/lib/`; no device catalog, no maps/types) and records
sha256 for the binary and manifest. `--with-devices` explicitly stages
`dist/devices` for an offline pairing source. Without it, new devices need
`Z2M_CONVERTER_FETCH=https` (trusted manifest retrieval, `ALLOW_NETWORK=1`) or
a separately staged module directory.

## Configuration (env only)

The bridge configuration file is not changed. Lazy mode is enabled through the
procd environment:

```
Z2M_CONVERTER_LAZY=1
Z2M_CONVERTER_TRANSPORT=endpoint      # dry-run for offline validation
Z2M_CONVERTER_ARTIFACT_DIR=/usr/share/zigbee2mqtt-rs/artifact
Z2M_CONVERTER_HERDSMAN_DIR=/usr/share/zigbee2mqtt-rs/herdsman
Z2M_CONVERTER_CACHE_DIR=/var/lib/zigbee2mqtt-rs/cache
Z2M_CONVERTER_RUNTIME_DIR=/var/lib/zigbee2mqtt-rs/runtime
Z2M_CONVERTER_ALLOW_NETWORK=0
Z2M_CONVERTER_FETCH=file            # or https (requires ALLOW_NETWORK=1)
```

`Z2M_CONVERTER_FETCH=https` uses the manifest `modules.retrieval` base and
verifies every module against the trusted manifest before activation. The
`file` fetcher reads `dist/devices` from the artifact directory (only present
with `--with-devices`).

`ulimit -v 131072` (128 MB address space) replaces the earlier 192 MB. The
measured Routerich probe VmPeak is 46–53 MB and VmRSS 27–33 MB; the limit bounds
address space, not RSS, and RSS is monitored during HIL. The cache/runtime generations keep the active
and last-known-good pointers, so rollback does not require regenerating.

## Generation rollback

1. Stop the Rust service (`/etc/init.d/zigbee2mqtt-rs stop`).
2. Cache: copy `cache/<artifactId>/lkg.json` over `active.json` (or restore the
   previous artifact directory and clear the cache).
3. Runtime: copy `runtime/lkg.json` over `runtime/active.json`.
4. Start the service and verify `bridge/state` and the device list.
5. Restart the Node.js service only after the Rust service is stopped; never run
   both against the same serial port.

## Port exclusivity

There is exactly one Dongle-P serial device. `procd` and the Node.js init script
must not be started together: stop one service, wait for the serial port to be
released, then start the other. The HIL plan (`docs/hil-plan.md`) records the
exact order and rollback.
