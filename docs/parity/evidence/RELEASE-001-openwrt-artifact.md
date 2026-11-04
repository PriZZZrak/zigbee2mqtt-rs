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
binary sha256: 54410a4de33a431b2d23279749fb544c584580a5d051d8dea191427be00fbe78 (5506056 bytes)
archive sha256: 4d2be11adee1ef19c8303f555a61e51d420cfa2b5a55bab99bdda9e5a893d6d1 (5560030 bytes)
manifest: target=aarch64-unknown-linux-musl, static=true, git_sha=a367941..., rustc 1.99.1

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
        installed 11929600 B, data 5588623 B, binary 5504553 B, web 6266677 B
ipk:   dist/zigbee2mqtt-rs_0.1.0-r1_aarch64_cortex-a53.ipk
sha256: e37eee4b4eaad5f57fc0d14959ce244aeef0db4edd559890c46ddbad401d8971
bytes:  5569984
```

Control metadata: `Package: zigbee2mqtt-rs`, `Version: 0.1.0-r1`,
`Depends: libc, mosquitto, mosquitto-client`, `Architecture:
aarch64_cortex-a53`, `Installed-Size: 11898880`; `conffiles` contains
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
(`e37eee4b…`), i.e. packaging is reproducible with the pinned SDK fed by the same
binary and frontend assets.

`scripts/check_ipk.py` validates the outer/inner layout, control fields,
`conffiles`, the required data paths, the ELF machine and static linkage;
`scripts/tests/test_check_ipk.py` covers valid/missing-config/missing-binary/
non-aarch64/dynamic-interpreter/missing-mosquitto paths (9 tests).
`openwrt-release.yml` now has an `ipk` job that pins the SDK checksum, stages the
pinned Windfront npm package, builds and validates the `.ipk`, and attaches it to
tag releases. **That job has not run yet.**

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
- The `.ipk` builds and validates, but **install/upgrade/downgrade/uninstall are
  BLOCKED**: they require the Routerich AX3000 v1 (OpenWrt 24.10.8) and the
  SONOFF Dongle-P. `Depends: mosquitto, mosquitto-client` are not in the SDK
  base feed (warning during `make`), so the device needs the `packages` feed or
  a locally built broker before `opkg install` can succeed.
- Measured installed size is **11 929 600 B (≈11.4 MiB)** including 6.27 MB of
  frontend assets; free overlay space still must be measured on the router (54 MB
  is total storage, not free). RAM usage and the 5.47 MB binary on aarch64 remain
  unmeasured.
- The earlier local `.ipk` had staged assets that were **not** the app-resolved
  Windfront build; it was rebuilt from `zigbee2mqtt-windfront@2.15.0` `dist`. The
  previous hashes are superseded and must not be published.
- The GitHub Actions workflow (both jobs) still has not run; the local SDK build
  is the evidence here.
- Binary provenance: the committed binary is a fresh zigbuild of the current
  tree, but the release manifest is generated locally; CI must reproduce the
  hash before publishing.
