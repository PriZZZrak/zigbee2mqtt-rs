#!/usr/bin/env bash
# Build a zigbee2mqtt-rs .ipk with an OpenWrt SDK.
#
# Usage: scripts/build-openwrt-ipk.sh <SDK_DIR> [BINARY] [OUTPUT_DIR]
#
# The binary must be the static aarch64 ELF built from this repository (the
# release workflow builds it with `cargo zigbuild`). The script stages the
# `openwrt/` package into the SDK, compiles only that package and validates the
# result with scripts/check_ipk.py. It does not install the package: install,
# upgrade and downgrade require an actual OpenWrt device.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SDK_DIR="${1:-}"
BINARY="${2:-$ROOT/openwrt/files/zigbee2mqtt-rs}"
OUTPUT_DIR="${3:-$ROOT/dist}"

die() { echo "build-openwrt-ipk: $*" >&2; exit 2; }

[[ -n "$SDK_DIR" ]] || die "usage: $0 <SDK_DIR> [BINARY] [OUTPUT_DIR]"
[[ -d "$SDK_DIR" ]] || die "SDK directory not found: $SDK_DIR"
[[ -f "$SDK_DIR/rules.mk" ]] || die "$SDK_DIR does not look like an OpenWrt SDK"
[[ -f "$BINARY" ]] || die "binary not found: $BINARY"
[[ -f "$ROOT/openwrt/Makefile" ]] || die "openwrt/Makefile is missing"
command -v make >/dev/null 2>&1 || die "make is required"
command -v python3 >/dev/null 2>&1 || die "python3 is required"

machine="$(od -A n -t x1 -j 18 -N 2 "$BINARY" | tr -d ' \n')"
[[ "$machine" == "b700" ]] || die "expected AArch64 ELF (e_machine=0x00b7), got 0x$machine"
if command -v file >/dev/null 2>&1; then
  file "$BINARY" | grep -q 'ARM aarch64' || die "file(1) does not report ARM aarch64"
fi

PKG_DIR="$SDK_DIR/package/zigbee2mqtt-rs"
rm -rf "$PKG_DIR"
cp -R "$ROOT/openwrt" "$PKG_DIR"
install -m 0755 "$BINARY" "$PKG_DIR/files/zigbee2mqtt-rs"

echo "==> make defconfig"
( cd "$SDK_DIR" && make defconfig >/dev/null )
echo "==> make package/zigbee2mqtt-rs/compile"
( cd "$SDK_DIR" && make package/zigbee2mqtt-rs/compile )

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -n1)"
[[ -n "$VERSION" ]] || die "cannot read version from Cargo.toml"

IPK="$(find "$SDK_DIR/bin/packages" -type f -name "zigbee2mqtt-rs_${VERSION}-r*_aarch64*.ipk" | head -n1)"
[[ -n "$IPK" ]] || die "SDK did not produce an .ipk"

mkdir -p "$OUTPUT_DIR"
cp "$IPK" "$OUTPUT_DIR/"
TARGET="$OUTPUT_DIR/$(basename "$IPK")"

python3 "$ROOT/scripts/check_ipk.py" "$TARGET"

echo "ipk: $TARGET"
echo "sha256: $(sha256sum "$TARGET" | cut -d' ' -f1)"
echo "bytes: $(stat -c %s "$TARGET")"
echo "note: install/upgrade/downgrade still require the target device"
