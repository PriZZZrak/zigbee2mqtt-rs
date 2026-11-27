# OpenWrt packaging (zigbee2mqtt-rs + Windfront)

Two packages, published to an opkg feed on the `gh-pages` branch:

| Package | Arch | Contents |
|---|---|---|
| `zigbee2mqtt-windfront` | `all` | `/usr/share/zigbee2mqtt/windfront` (Windfront `dist` + `package.json` + LICENSE) |
| `zigbee2mqtt-rs` | `aarch64_cortex-a53` | `/usr/bin/zigbee2mqtt-rs`, `/etc/init.d/zigbee2mqtt-rs`, postinst adds `/etc/zigbee2mqtt` to `/etc/sysupgrade.conf` |

## Version policy

- `packaging/windfront.version` is the single source of truth (currently
  `2.15.1`, the version pinned by upstream zigbee2mqtt 2.14.3).
- The Rust binary embeds that value (`include_str!`) and compares it with
  `<assets>/package.json` at startup. A mismatch logs a warning and refuses to
  start unless `ZIGBEE2MQTT_ALLOW_WINDFRONT_MISMATCH=1` is set.
- `zigbee2mqtt-rs` depends on the exact windfront version:
  `Depends: zigbee2mqtt-windfront (= <packaging/windfront.version>)`.
  Windfront releases (1–2/month) are **not** in lockstep with zigbee2mqtt, so
  the pair is bumped together and `frontend` protocol tests are re-run.

## Workflows

- `.github/workflows/windfront-package.yml` — downloads the pinned npm tarball,
  verifies `name`/`version`, builds the `all` ipk, updates the feed, and (for
  `windfront-v*` tags) attaches the ipk to a GitHub release.
- `.github/workflows/z2m-rs-package.yml` — cross-builds the static
  `aarch64-unknown-linux-musl` binary with cargo-zigbuild, downloads and
  verifies the pinned converter Release (`packaging/converter-release.json`,
  every asset hash/SHA256SUMS), regenerates locally and requires manifest/index
  equality, builds the ipk **through the OpenWrt SDK** with
  `HERDSMAN_DIR` pointing at the npm-locked herdsman, verifies the real ipk
  payload (stripped aarch64 binary, manifest/index/shared JS, no device
  catalog) and updates the same feed. It refuses to build if the Release is
  missing or has unexpected files.
- `.github/workflows/converter-artifacts.yml` — generates the artifact twice
  from the npm-locked pinned packages, runs the manifest/graph/split gates, the
  resolver differential, ZCL/ZDO differentials and golden parity, then stages
  flat release assets with `SHA256SUMS` and publishes an immutable versioned
  GitHub Release (draft-first, fail-closed; only this run's draft is removed on
  failure).

**ipk container format:** OpenWrt 24.10 opkg reads `.ipk` as a gzipped tar with
`./debian-binary`, `./data.tar.gz`, `./control.tar.gz`; an `ar` archive is
rejected with "Malformed package file" (found during the 2026-11-17 install).
`build-ipk.sh` therefore produces the tar container.

Both workflows share `concurrency: opkg-feed` so the `Packages` index is not
rewritten concurrently. The feed is updated on a second `actions/checkout` of
the `gh-pages` branch (`path: feed`) and pushed with the workflow token.

First publication: `windfront-v2.15.1` (2026-11-16) produced
`feed/Packages`, `feed/Packages.gz` and
`feed/zigbee2mqtt-windfront_2.15.1_all.ipk` on `gh-pages` (commit `19fa4ec`);
`Packages.sig`/`public.key` appear once the usign secrets are configured.

A failing run writes `feed/ci-error.txt` (run URL + captured output) to the
same branch: the Actions log itself is only readable with repository admin
rights. The next successful run deletes that file.

## Feed layout

The repository's `gh-pages` branch already hosts unrelated content, so the
packages live in a subdirectory:

```
gh-pages/
  feed/
    Packages
    Packages.gz
    Packages.sig      (only when USIGN_PRIVATE_KEY is set)
    public.key        (only when USIGN_PUBLIC_KEY is set)
    zigbee2mqtt-windfront_2.15.1_all.ipk
    zigbee2mqtt-rs_<version>_aarch64_cortex-a53.ipk
```

URL: `https://<org>.github.io/<repo>/feed`.

## Feed signing (needs the private key)

Store the OpenWrt `usign` keypair as the repository secrets
`USIGN_PRIVATE_KEY` and `USIGN_PUBLIC_KEY` (contents of the two files, not the
paths). When both exist, the workflows publish `Packages.sig` and
`feed/public.key`; without them the feed is published unsigned.

Generate a keypair once (on any machine with `usign`, e.g. the router itself):

```sh
usign -G -p z2m-rs.pub -s z2m-rs.sec -c "zigbee2mqtt-rs feed"
usign -F -p z2m-rs.pub          # fingerprint, used as /etc/opkg/keys/<fp>
usign -F -s z2m-rs.sec          # must print the same fingerprint
```

Store the **whole file content** of each key as a secret, including the
`untrusted comment: …` line: `usign` decodes the key from the second line but
rejects a file that starts with the base64 payload ("Failed to decode secret
key"). A trailing blank line is harmless. `.sec` (~200 chars) is
`USIGN_PRIVATE_KEY`, `.pub` (~60 chars) is `USIGN_PUBLIC_KEY`; both start with
`RWR`.

## Converter artifact package and modes

The `zigbee2mqtt-rs` ipk now carries `manifest.json`, `index.z2idx1`,
`dist/index.js`, `dist/models-index.json`, `dist/converters/`, `dist/lib/` and
the pinned herdsman JS. It does **not** carry the device catalog. Two explicit
modes are selected by `/etc/zigbee2mqtt-rs/mode`:

- `on-demand` (default): `Z2M_CONVERTER_FETCH=https` and
  `Z2M_CONVERTER_ALLOW_NETWORK=1`; modules are fetched from the trusted GitHub
  Release named in the manifest (`modules.retrieval`) and verified by size and
  SHA-256 before activation.
- `offline`: `FETCH=file`, `ALLOW_NETWORK=0`; requires a pre-seeded verified
  cache or a package built with `packaging/openwrt-pkg/build.sh ... --with-devices`.
  Missing definitions are reported with the device IEEE and module id; nothing
  falls back silently.

The service runs with `ulimit -v 131072` (128 MB address space). Measured
Routerich VmPeak is 46–53 MB and VmRSS 27–33 MB; the address-space limit is not
an RSS limit and is reviewed after HIL. Installed footprint is ~24.1 MB
(binary 16.68 MB + artifact 3.7 MB + herdsman 3.6 MB) plus 0.5–1 MB cache/runtime;
the current overlay has ~568 KB free, so the migration plan is required before
installation.

## Verified first package (2026-11-27)

Actions run
`https://github.com/PriZZZrak/zigbee2mqtt-rs/actions/runs/46016877970`
built `feed/zigbee2mqtt-rs_0.0.1-hil1-r1_aarch64_cortex-a53.ipk`
(sha256 `33f7abb87c76a923b71e8d4bbc715af8dc4db7c25bf57c7b6811198d64900648`,
8,315,124 B) through the SDK against converter Release
`converter-vz2hc-26.125.0-zh-11.2.0-abi1`. Inspection: aarch64 ELF,
`Installed-Size: 23,818,240`, `Depends: libc`, manifest/index byte-identical to
the Release, shared JS present, no device JS, init/mode present.

## Router setup

```sh
# signed feed: install the public key (fingerprint = `usign -F -p public.key`)
wget -O /tmp/public.key https://<org>.github.io/<repo>/feed/public.key
opkg-key add /tmp/public.key

# customfeeds.conf usually has no trailing newline: prefix the new line
printf '\nsrc/gz z2m_rs https://<org>.github.io/<repo>/feed\n' >> /etc/opkg/customfeeds.conf
opkg update
opkg install zigbee2mqtt-windfront zigbee2mqtt-rs
```

Verified on Routerich AX3000 (2026-11-16) with the signed feed: key
fingerprint `70a94d533b5a96a5`, `opkg update` prints
`Signature check passed.` for `/feed/Packages.sig`, and `opkg list` shows
`zigbee2mqtt-rs - 0.0.1-dev.1` and `zigbee2mqtt-windfront - 2.15.1`.

`opkg install zigbee2mqtt-rs` pulls the exact Windfront version because of
`Depends: zigbee2mqtt-windfront (= <packaging/windfront.version>)`; an unsigned
feed works too, but opkg then only warns about missing signatures.

**OpenWrt enables `option check_signature` in `/etc/opkg.conf`**, so the feed
must be signed for a stock installation: without `USIGN_PRIVATE_KEY` /
`USIGN_PUBLIC_KEY` `opkg update` fails signature verification. Either add the
secrets or disable the check for a throwaway feed.

Space: `zigbee2mqtt-windfront` installs ~6.8 MB and `zigbee2mqtt-rs` ~12 MB
into the overlay. If the router still carries the Node.js installation in
`/opt/zigbee2mqtt` (53.6 MB), remove it first. Re-installing the Node package
later needs `opkg install --force-space node-zigbee2mqtt` because the overlay
check runs before our packages are removed.

The service waits for the serial port from `configuration.yaml`, exports
`ZIGBEE2MQTT_DATA=/etc/zigbee2mqtt` and
`ZIGBEE2MQTT_FRONTEND_ASSETS=/usr/share/zigbee2mqtt/windfront`, and respawns
without a crash-count limit (`respawn 3600 10 0`).

`/etc/zigbee2mqtt` (configuration.yaml, database.db, state.json) is listed in
`/etc/sysupgrade.conf`, so a firmware upgrade keeps the installation data.

## Updating

- `opkg update && opkg upgrade zigbee2mqtt-windfront zigbee2mqtt-rs`
- a package-only update of Windfront is possible, but the daemon refuses a
  version it was not tested with; update both packages together.
- firmware images are not built by these workflows; the feed is meant for an
  already-running Routerich AX3000 / OpenWrt 24.10 installation.

## Status (2026-11-16)

Both packages are published to the signed feed and were exercised on the
Routerich AX3000 (see `docs/hil/2026-11-16-rust-port-startup.md`). Installing
them on the live router is **deferred**: the overlay only has ~1 MB free and
frees enough space only when `node-zigbee2mqtt` (53.6 MB in
`/opt/zigbee2mqtt`, re-installable from the same custom feed) is removed.
R1 has no converter layer yet, so the Rust bridge reports every device as
"Not supported" and would not publish device state; keeping the Node service is
the only way to keep the live devices working until the device layer lands.

## Local builds

```sh
# Windfront ipk (+ <ipk>.meta feed stanza)
./packaging/opkg/build-ipk.sh zigbee2mqtt-windfront 2.15.1 all "" \
  /path/to/payload /tmp/out "Windfront static UI for Zigbee2MQTT"

# feed index from the *.meta stanzas (+ optional usign key -> Packages.sig)
./packaging/opkg/update-feed.sh /tmp/feed /path/to/usign.sec
```

Both workflows build `usign` from `https://github.com/openwrt/usign` when the
runner does not ship it; `update-feed.sh` fails with an explicit error if a key
is given but `usign` is unavailable.

Verified locally against the real `zigbee2mqtt-windfront-2.15.1.tgz`
(2026-11-16): ipk 2.7 MB / `Installed-Size: 6792`, `ar t` =
`debian-binary control.tar.gz data.tar.gz`, `Packages` + `Packages.gz` +
`Packages.sig` written and `usign -V` reports `OK`.
