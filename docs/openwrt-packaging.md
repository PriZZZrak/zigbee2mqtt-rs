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
  `aarch64-unknown-linux-musl` binary with cargo-zigbuild, builds the ipk with
  the exact windfront dependency, and updates the same feed.

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
```

## Router setup

```sh
# signed feed: install the public key (fingerprint = `usign -F -p public.key`)
wget -O /tmp/public.key https://<org>.github.io/<repo>/feed/public.key
opkg-key add /tmp/public.key

echo 'src/gz z2m_rs https://<org>.github.io/<repo>/feed' >> /etc/opkg/customfeeds.conf
opkg update
opkg install zigbee2mqtt-windfront zigbee2mqtt-rs
```

`opkg install zigbee2mqtt-rs` pulls the exact Windfront version because of
`Depends: zigbee2mqtt-windfront (= <packaging/windfront.version>)`; an unsigned
feed works too, but opkg then only warns about missing signatures.

**OpenWrt enables `option check_signature` in `/etc/opkg.conf`**, so the feed
must be signed for a stock installation: without `USIGN_PRIVATE_KEY` /
`USIGN_PUBLIC_KEY` `opkg update` fails signature verification. Either add the
secrets or disable the check for a throwaway feed.

Space: `zigbee2mqtt-windfront` installs ~6.8 MB and `zigbee2mqtt-rs` ~12 MB
into the overlay. A router that still carries the Node.js installation in
`/opt/zigbee2mqtt` (53.6 MB) has no room; remove that first.

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
