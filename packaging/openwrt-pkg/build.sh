#!/bin/sh
# Build the zigbee2mqtt-rs ipk through the existing OpenWrt SDK.
# Usage: packaging/openwrt-pkg/build.sh <sdk-dir> <binary> <artifact-dir> <out-dir> [--with-devices] [version]
# The SDK package is the primary packaging path; deploy/build.sh stages the
# same payload for inspection. Nothing is copied to a router.
set -eu
SDK=${1:?sdk dir}
BINARY=${2:?stripped binary}
ARTIFACT=${3:?artifact dir}
OUT=${4:?output dir}
WITH_DEVICES=${5:-}
VERSION=${6:-0.1.0}
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
. "$ROOT/packaging/lib.sh"

PKG=$(dirname -- "$0")/zigbee2mqtt-rs
rm -rf "$SDK/package/zigbee2mqtt-rs"
cp -a "$PKG" "$SDK/package/zigbee2mqtt-rs"
STAGE="$SDK/package/zigbee2mqtt-rs/files"
mkdir -p "$STAGE"
cp "$BINARY" "$STAGE/zigbee2mqtt-rs"
chmod 0755 "$STAGE/zigbee2mqtt-rs"
rm -rf "$STAGE/artifact" "$STAGE/herdsman"
stage_artifact "$ARTIFACT" "$STAGE/artifact" "$WITH_DEVICES"
stage_js_tree "$ROOT/.upstream/zigbee2mqtt/node_modules/zigbee-herdsman/dist" "$STAGE/herdsman/dist"
cp "$ROOT/.upstream/zigbee2mqtt/node_modules/zigbee-herdsman/package.json" "$STAGE/herdsman/" 2>/dev/null || true

make -C "$SDK" package/zigbee2mqtt-rs/compile V=s Z2M_RS_VERSION="$VERSION"
IPK=$(ls -t $(find "$SDK/bin/packages" -name 'zigbee2mqtt-rs_*.ipk') 2>/dev/null | head -n1)
[ -n "$IPK" ] || { echo "SDK did not produce an ipk" >&2; exit 1; }
mkdir -p "$OUT"
cp "$IPK" "$OUT/"
sha256sum "$OUT/$(basename "$IPK")" > "$OUT/$(basename "$IPK").sha256"

# Feed stanza in the same format as packaging/opkg/build-ipk.sh.
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
tar -C "$work" -xzf "$OUT/$(basename "$IPK")"
tar -C "$work" -xzf "$work/control.tar.gz"
{
	cat "$work/control"
	printf 'Filename: %s\n' "$(basename "$IPK")"
	printf 'Size: %s\n' "$(wc -c < "$OUT/$(basename "$IPK")" | tr -d ' ')"
	printf 'SHA256sum: %s\n' "$(sha256sum "$OUT/$(basename "$IPK")" | awk '{print $1}')"
	printf '\n'
} > "$OUT/$(basename "$IPK").meta"
echo "ipk: $IPK"
