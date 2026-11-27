#!/bin/sh
# Reproducible aarch64-musl build + staged OpenWrt payload.
# Usage: deploy/build.sh <artifact-dir> <output-dir> [--with-devices]
#   <artifact-dir> is produced by tools/artifact-generator/generate.js
#   (manifest.json, index.z2idx1, dist/).
#   --with-devices stages dist/devices (offline pairing source). Without it the
#   payload contains no device catalog; new devices need an HTTPS/local module
#   source at pairing time.
# Nothing is copied to the router by this script.
set -eu
ARTIFACT=${1:?artifact dir}
OUT=${2:-target/openwrt-payload}
WITH_DEVICES=0
[ "${3:-}" = "--with-devices" ] && WITH_DEVICES=1
TARGET=aarch64-unknown-linux-musl
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
. "$ROOT/packaging/lib.sh"

CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-1} cargo build --release --target "$TARGET" -p z2m-rs

BIN="target/$TARGET/release/z2m-rs"
mkdir -p "$OUT/usr/bin" "$OUT/usr/share/zigbee2mqtt-rs/artifact" \
  "$OUT/etc/init.d" "$OUT/etc/zigbee2mqtt-rs"
strip_or_fail "$BIN" "$OUT/usr/bin/zigbee2mqtt-rs"

ARTIFACT_OUT="$OUT/usr/share/zigbee2mqtt-rs/artifact"
if [ "$WITH_DEVICES" = "1" ]; then
  stage_artifact "$ARTIFACT" "$ARTIFACT_OUT" --with-devices
else
  stage_artifact "$ARTIFACT" "$ARTIFACT_OUT"
fi

cp -a deploy/openwrt/files/etc/init.d/zigbee2mqtt-rs "$OUT/etc/init.d/"
cp -a deploy/openwrt/files/etc/zigbee2mqtt-rs/README "$OUT/etc/zigbee2mqtt-rs/"

# The runtime needs the pinned zigbee-herdsman package; it is not part of the
# converter artifact. Stage only shipped JS (no source maps/types).
HERDSMAN_SRC=.upstream/zigbee2mqtt/node_modules/zigbee-herdsman
stage_js_tree "$HERDSMAN_SRC/dist" "$OUT/usr/share/zigbee2mqtt-rs/herdsman/dist"
[ -f "$HERDSMAN_SRC/package.json" ] && cp "$HERDSMAN_SRC/package.json" "$OUT/usr/share/zigbee2mqtt-rs/herdsman/"

sha256sum "$OUT/usr/bin/zigbee2mqtt-rs" > "$OUT/usr/bin/zigbee2mqtt-rs.sha256"
sha256sum "$OUT/usr/share/zigbee2mqtt-rs/artifact/manifest.json" \
  > "$OUT/usr/share/zigbee2mqtt-rs/artifact/manifest.sha256"
echo "staged in $OUT"
