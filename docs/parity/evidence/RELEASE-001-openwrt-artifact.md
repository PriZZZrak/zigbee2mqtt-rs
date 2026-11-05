# RELEASE-001 — reproducible OpenWrt release artifact

Implementation: `scripts/build-openwrt-artifact.sh`,
`scripts/build-openwrt-ipk.sh`, `scripts/check_ipk.py`,
`scripts/tests/test_openwrt_artifact.py`, `scripts/tests/test_check_ipk.py`,
`.github/workflows/openwrt-release.yml`, `openwrt/Makefile`, `.gitignore`.
Behavior: RELEASE-001; plan OPS-02.

## Contract

`openwrt/Makefile` packages a prebuilt static aarch64 binary. OPS-02 requires the
binary to come from a reproducible build of a pinned Rust SHA, with a manifest
containing target, toolchain, features and binary hash/size, and the package to
carry service/config files that survive upgrades.

## What this change does

The manifest distinguishes tracked modifications (`git_dirty`, sources changed)
from untracked files (`git_untracked_files`), so an unrelated untracked file no
longer makes a build look source-dirty.

`scripts/build-openwrt-artifact.sh [BINARY] [OUTPUT_DIR]`:

- rejects non-AArch64 ELF (`e_machine` read from bytes 18–19) and a binary with a
  dynamic interpreter; requires `file` to report `ARM aarch64` when available;
- stages the package tree: `usr/bin/zigbee2mqtt-rs`, `etc/init.d/zigbee2mqtt-rs`
  (procd), `etc/zigbee2mqtt-rs/config.yaml` from the example, the mosquitto
  drop-in, and optional `www/zigbee2mqtt` Windfront assets;
- writes `<name>.tar.gz` with numeric `root:root` ownership and
  `<name>.manifest.json` with schema, version, target, archive/binary SHA-256 and
  sizes, ELF machine, static flag, `rustc --version`, git SHA/dirty state, UTC
  timestamp, provenance and explicit notes that hashes are not signatures.

Because the script never compiles, an artifact can never silently claim a build
it did not perform; the manifest records `compiled-binary-supplied-to-packager`
and CI must pass its own freshly built binary.

`.github/workflows/openwrt-release.yml` (manual/tag trigger) installs the
`aarch64-unknown-linux-musl` target, zig + `cargo-zigbuild` (linker only), builds
`cargo zigbuild --release --locked`, runs the packager, checks the ELF has no
`INTERP`, uploads the tarball/manifest as an artifact and attaches them to a tag
release. `dist/` is git-ignored.

## Verification available here

```text
$ cargo zigbuild --release --locked --target aarch64-unknown-linux-musl
$ cp target/aarch64-unknown-linux-musl/release/zigbee2mqtt-rs openwrt/files/zigbee2mqtt-rs
$ scripts/build-openwrt-artifact.sh
artifact: dist/zigbee2mqtt-rs-0.1.0-aarch64-openwrt.tar.gz
binary sha256: 1d812e5412ab231aa1d4e4263da452f135a8efa7d5a023e7e0e38242f72b68fc (5748488 bytes)
archive sha256: 205c55916025123d0e39a92391275081a1f78daeaf4b68a5988c8dc047e95990 (5680661 bytes)
manifest: target=aarch64-unknown-linux-musl, static=true, git_sha=d4d3ee1..., rustc 1.99.1

$ python3 -m unittest discover -s scripts/tests
Ran 38 tests ... OK
```

`test_openwrt_artifact.py` runs the packager on the committed aarch64 binary and
asserts manifest fields, recomputed SHA-256, archive contents and rejection of a
non-AArch64 binary.

## OpenWrt `.ipk` (2026-11-05)

SDK: `openwrt-sdk-24.10.8-mediatek-filogic_gcc-13.3.0_musl.Linux-x86_64`
(`sha256 d9fd074f…`, target `mediatek/filogic`, `aarch64_cortex-a53`).

```text
$ scripts/build-openwrt-ipk.sh /tmp/opencode/sdk/openwrt-sdk-24.10.8-...
$ make defconfig
$ make package/zigbee2mqtt-rs/compile
ipk OK: zigbee2mqtt-rs 0.1.0-r1 (aarch64_cortex-a53);
        installed 12175360 B, data 5712977 B, binary 5746985 B, web 6266677 B
ipk:   dist/zigbee2mqtt-rs_0.1.0-r1_aarch64_cortex-a53.ipk
sha256: 4f71a4f6d68f3bf385938461ab71058c6eac7d987c83bfa8ed6fbb281183522b
bytes:  5695423
```

Control metadata: `Package: zigbee2mqtt-rs`, `Version: 0.1.0-r1`,
`Depends: libc, mosquitto`, `Architecture:
aarch64_cortex-a53`, `Installed-Size: 12175360`; `conffiles` contains
`/etc/zigbee2mqtt-rs/config.yaml` (so upgrades keep the user's config).

Data payload: `usr/bin/zigbee2mqtt-rs` (statically linked AArch64, executable),
`etc/init.d/zigbee2mqtt-rs`, `etc/zigbee2mqtt-rs/config.yaml`,
`etc/mosquitto/conf.d/99-zigbee2mqtt-rs.conf`, and the pinned Windfront frontend
under `www/zigbee2mqtt` (6 266 677 uncompressed bytes). The assets are copied
from the **app-resolved** `zigbee2mqtt-windfront@2.15.0` `dist`
(`index.html` sha256 `da527869…`); the historical frontend repo tag (`2.15.1`,
commit `fbe6e2d`) is recorded separately in `upstream-lock.json` and differs.

The binary inside the `.ipk` is **not byte-identical** to
`openwrt/files/zigbee2mqtt-rs`: OpenWrt packs it with `sstrip`
(5 475 952 → 5 474 449 bytes, no section headers). That is expected and is
recorded here so the hash difference is not mistaken for a different build.
Re-running the SDK build a second time produced the **same** `.ipk` SHA-256
(`4f71a4f6…`), i.e. packaging is reproducible with the pinned SDK fed by the same
binary and frontend assets.

`scripts/check_ipk.py` validates the outer/inner layout, control fields,
`conffiles`, the required data paths, the ELF machine and static linkage;
`scripts/tests/test_check_ipk.py` covers valid/missing-config/missing-binary/
non-aarch64/dynamic-interpreter/missing-mosquitto paths (9 tests).
`openwrt-release.yml` now has an `ipk` job that pins the SDK checksum, stages the
pinned Windfront npm package, builds and validates the `.ipk`, and attaches it to
tag releases. **That job has not run yet.**

## Binary-only updates after the packaged build

Per user request, the HIL iteration after the packaged build deployed newer
aarch64 binaries directly to `/usr/bin` (config_schema, ZCL widths, UI data
contract) without rebuilding the `.ipk`/tarball. The hashes above describe the
**last packaged build**, not the currently running binary. Re-run
`scripts/build-openwrt-ipk.sh` and `scripts/build-openwrt-artifact.sh` before any
release.

## Gaps (not verified)## Gaps (not verified)

- The artifact is now built from the current SHA with `cargo zigbuild` (zig
  0.13.0 is available). Original GNU-`ld` failure (`--fix-cortex-a53-843419`) is
  avoided by zig's linker.
- The GitHub Actions workflow itself still has not run; the same zigbuild +
  packaging steps were executed locally instead.
- **Real binary is 5 748 488 bytes** (rustc 1.99.1, `opt-level=z`, LTO, strip),
  much larger than the previously committed 1 775 456-byte binary (which
  predated the Boa converter host). The installed size is **12 165 120 B**
  including 6.27 MB of frontend assets; the router's overlay was **5.6 MB free**
  before the test, so the Node installation had to be removed (see R1-001).
- The `.ipk` was **installed and started** on the Routerich AX3000 v1 with the
  Dongle-P on 2026-11-05 (see `R1-routerich-hil-20261105.md`). The initial
  packaging required `mosquitto-client`, which is absent from the Routerich
  feeds; it was removed because the Rust binary does not use it and
  `mosquitto` is provided by `mosquitto-nossl`. Upgrade, downgrade and uninstall
  were not exercised; rollback (`node-zigbee2mqtt 2.14.1-r2`) is available in
  the feed.
- Measured installed size is **12 175 360 B (≈11.6 MiB)** including 6.27 MB of
  frontend assets. The router overlay is 54.3 MB total with **5.6 MB free**
  before removing the Node installation; RAM is 497 MB total (76 MB free at
  test start). The aarch64 binary was executed on the router.
- The earlier local `.ipk` had staged assets that were **not** the app-resolved
  Windfront build; it was rebuilt from `zigbee2mqtt-windfront@2.15.0` `dist`. The
  previous hashes are superseded and must not be published.
- The GitHub Actions workflow (both jobs) still has not run; the local SDK build
  is the evidence here.
- Binary provenance: the committed binary is a fresh zigbuild of the current
  tree, but the release manifest is generated locally; CI must reproduce the
  hash before publishing.
