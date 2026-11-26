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

CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-1} cargo build --release --target "$TARGET" -p z2m-rs

BIN="target/$TARGET/release/z2m-rs"
mkdir -p "$OUT/usr/bin" "$OUT/usr/share/zigbee2mqtt-rs/artifact" \
  "$OUT/etc/init.d" "$OUT/etc/zigbee2mqtt-rs"
STRIP=${LLVM_STRIP:-$(command -v llvm-strip || true)}
if [ -z "$STRIP" ]; then
  STRIP=$(find "${RUSTUP_HOME:-$HOME/.rustup}" -path '*/bin/llvm-strip' 2>/dev/null | head -n1)
fi
if [ -n "$STRIP" ]; then
  "$STRIP" "$BIN" -o "$OUT/usr/bin/zigbee2mqtt-rs"
elif command -v strip >/dev/null 2>&1; then
  cp "$BIN" "$OUT/usr/bin/zigbee2mqtt-rs"
  strip "$OUT/usr/bin/zigbee2mqtt-rs"
else
  echo "no usable strip tool; refusing to ship an unstripped binary" >&2
  exit 1
fi
stage_tree() {
  src=$1
  dst=$2
  mkdir -p "$dst"
  (cd "$src" && find . -type f \( -name '*.js' -o -name '*.json' \) \
    ! -name '*.map' ! -name '*.d.ts' -exec cp --parents {} "$dst" \;)
}

ARTIFACT_OUT="$OUT/usr/share/zigbee2mqtt-rs/artifact"
mkdir -p "$ARTIFACT_OUT"
cp "$ARTIFACT/manifest.json" "$ARTIFACT_OUT/"
sha256sum "$ARTIFACT_OUT/manifest.json" | awk '{print $1}' > "$ARTIFACT_OUT/manifest.json.sha256"
cp "$ARTIFACT/index.z2idx1" "$ARTIFACT_OUT/"
mkdir -p "$ARTIFACT_OUT/dist"
cp "$ARTIFACT/dist/index.js" "$ARTIFACT_OUT/dist/"
cp "$ARTIFACT/dist/models-index.json" "$ARTIFACT_OUT/dist/" 2>/dev/null || true
stage_tree "$ARTIFACT/dist/converters" "$ARTIFACT_OUT/dist/converters"
stage_tree "$ARTIFACT/dist/lib" "$ARTIFACT_OUT/dist/lib"
if [ "$WITH_DEVICES" = "1" ]; then
  stage_tree "$ARTIFACT/dist/devices" "$ARTIFACT_OUT/dist/devices"
fi

cp -a deploy/openwrt/files/etc/init.d/zigbee2mqtt-rs "$OUT/etc/init.d/"
cp -a deploy/openwrt/files/etc/zigbee2mqtt-rs/README "$OUT/etc/zigbee2mqtt-rs/"

# The runtime needs the pinned zigbee-herdsman package; it is not part of the
# converter artifact. Stage only shipped JS (no source maps/types).
HERDSMAN_SRC=.upstream/zigbee2mqtt/node_modules/zigbee-herdsman
stage_tree "$HERDSMAN_SRC/dist" "$OUT/usr/share/zigbee2mqtt-rs/herdsman/dist"
[ -f "$HERDSMAN_SRC/package.json" ] && cp "$HERDSMAN_SRC/package.json" "$OUT/usr/share/zigbee2mqtt-rs/herdsman/"

sha256sum "$OUT/usr/bin/zigbee2mqtt-rs" > "$OUT/usr/bin/zigbee2mqtt-rs.sha256"
sha256sum "$OUT/usr/share/zigbee2mqtt-rs/artifact/manifest.json" \
  > "$OUT/usr/share/zigbee2mqtt-rs/artifact/manifest.sha256"
echo "staged in $OUT"
