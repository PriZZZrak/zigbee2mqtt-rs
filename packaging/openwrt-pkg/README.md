# OpenWrt SDK package

`HERDSMAN_DIR=<npm node_modules/zigbee-herdsman> build.sh <sdk> <binary>
<artifact> <out> [--with-devices] [version]` is the primary packaging path.
`HERDSMAN_DIR` is required; the script verifies its `package.json` version and
`dist/index.js`. It copies this package definition into the SDK, stages:

- the stripped aarch64-musl binary,
- `manifest.json`, `index.z2idx1`, `dist/index.js`, `dist/models-index.json`,
  `dist/converters/`, `dist/lib/` (no device catalog by default),
- the pinned `zigbee-herdsman` JS,
- the init script and `mode` file,

then runs `make package/zigbee2mqtt-rs/compile` and copies the resulting ipk.

Modes (`/etc/zigbee2mqtt-rs/mode`):

- `on-demand` (default): `Z2M_CONVERTER_FETCH=https`,
  `Z2M_CONVERTER_ALLOW_NETWORK=1`; the manifest `modules.retrieval` points at
  the trusted GitHub Release.
- `offline`: `FETCH=file`, `ALLOW_NETWORK=0`; requires a pre-seeded verified
  cache or a `--with-devices` package variant.

Missing definitions are reported per device and module; there is no silent
fallback. `ulimit -v 131072` (128 MB address space) is based on the measured
Routerich VmPeak (46–53 MB); RSS is ~27–33 MB and is monitored during HIL.
