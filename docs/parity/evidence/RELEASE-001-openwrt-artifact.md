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
binary sha256: 81bab3876e053b1a2e719f9a6adde87ac4b5fc954ac4c993c365794126c238fa (5461248 bytes)
archive sha256: 744ca8cb6a0bf3569756e5cc626ba718306c08263bed9177d5ebf9978f29c1d4 (5613075 bytes)
manifest: target=aarch64-unknown-linux-musl, static=true, git_sha=57d9478..., rustc 1.99.1

$ python3 -m unittest discover -s scripts/tests
Ran 26 tests ... OK
```

`test_openwrt_artifact.py` runs the packager on the committed aarch64 binary and
asserts manifest fields, recomputed SHA-256, archive contents and rejection of a
non-AArch64 binary.

## Gaps (not verified)

- The artifact is now built from the current SHA with `cargo zigbuild` (zig
  0.13.0 is available). Original GNU-`ld` failure (`--fix-cortex-a53-843419`) is
  avoided by zig's linker.
- The GitHub Actions workflow itself still has not run; the same zigbuild +
  packaging steps were executed locally instead.
- **Real binary is 5 450 608 bytes** (rustc 1.99.1, `opt-level=z`, LTO, strip),
  much larger than the previously committed 1 775 456-byte binary (which
  predated the Boa converter host). This is a concrete OPS-03 budget input:
  installed size + Windfront assets still needs measuring against the router's
  free overlay, not the reported 54 MB total.
- The `.ipk` is still produced by the OpenWrt SDK outside this repository; the
  tarball is not an opkg package, and the SDK build is unverified.
- Installed size/RAM budgets, sysupgrade/conffiles retention, install/upgrade/
  downgrade/uninstall on Routerich and the Dongle-P HIL run remain untested.
- Binary provenance/licensing of the committed prebuilt binary is unknown and
  must be replaced by a CI-built artifact before publishing.
