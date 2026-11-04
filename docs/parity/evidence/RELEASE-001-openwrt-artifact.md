# RELEASE-001 — reproducible OpenWrt release artifact

Implementation: `scripts/build-openwrt-artifact.sh`,
`scripts/tests/test_openwrt_artifact.py`,
`.github/workflows/openwrt-release.yml`, `openwrt/Makefile`, `.gitignore`.
Behavior: RELEASE-001; plan OPS-02.

## Contract

`openwrt/Makefile` packages a prebuilt static aarch64 binary. OPS-02 requires the
binary to come from a reproducible build of a pinned Rust SHA, with a manifest
containing target, toolchain, features and binary hash/size, and the package to
carry service/config files that survive upgrades.

## What this change does

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
$ scripts/build-openwrt-artifact.sh
artifact: /config/workspace/zigbee2mqtt-rs/dist/zigbee2mqtt-rs-0.1.0-aarch64-openwrt.tar.gz
manifest: .../zigbee2mqtt-rs-0.1.0-aarch64-openwrt.manifest.json
binary sha256: ea61a9e3bc702c0ee39b5b162109720e6e18d2229d5820c030e0a9af99ec6613 (1775456 bytes)
archive sha256: 9068729063b3185333aae90f00bd80ed7445620c101b898d196f5c41fd5e060a (3795053 bytes)

$ python3 -m unittest discover -s scripts/tests
Ran 26 tests ... OK
```

`test_openwrt_artifact.py` runs the packager on the committed aarch64 binary and
asserts manifest fields, recomputed SHA-256, archive contents and rejection of a
non-AArch64 binary.

## Gaps (not verified)

- **No cross-compilation in this environment**: the installed
  `aarch64-unknown-linux-musl` target has no aarch64 linker (`/usr/bin/ld`
  rejects `--fix-cortex-a53-843419`), so the artifact was staged from the
  committed prebuilt binary, not a fresh build of the current SHA. The manifest
  correctly records this; it is **not** proof that the committed binary matches
  the Rust source.
- The GitHub Actions workflow has not run (no network/runner access here). Its
  zigbuild/native-audit/release steps are code-reviewed only.
- The `.ipk` is still produced by the OpenWrt SDK outside this repository; the
  tarball is not an opkg package, and the SDK build is unverified.
- Installed size/RAM budgets, sysupgrade/conffiles retention, install/upgrade/
  downgrade/uninstall on Routerich and the Dongle-P HIL run remain untested.
- Binary provenance/licensing of the committed prebuilt binary is unknown and
  must be replaced by a CI-built artifact before publishing.
