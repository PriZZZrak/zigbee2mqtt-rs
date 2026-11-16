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
rewritten concurrently.

## Feed signing (needs the private key)

Store the OpenWrt `usign` private key as the repository secret
`USIGN_PRIVATE_KEY`. When the runner has `usign` available, the workflows
publish `Packages.sig` and `public.key` next to the index. Without the secret
(or without `usign` on the runner) the feed is published unsigned.

## Router setup

```sh
# copy the public key (from https://<org>.github.io/<repo>/feed/public.key)
opkg-key add public.key
echo 'src/gz z2m_rs https://<org>.github.io/<repo>/feed' >> /etc/opkg/customfeeds.conf
opkg update
opkg install zigbee2mqtt-rs zigbee2mqtt-windfront
```

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
# Windfront ipk
./packaging/openwrt/build-ipk.sh zigbee2mqtt-windfront 2.15.1 all "" \
  /path/to/payload /tmp/out "Windfront static UI for Zigbee2MQTT"

# feed index (+ optional usign key)
./packaging/openwrt/update-feed.sh /tmp/feed /path/to/usign.key
```
