#!/usr/bin/env bash
# Stage and package an OpenWrt release artifact for zigbee2mqtt-rs.
#
# Usage: scripts/build-openwrt-artifact.sh [BINARY] [OUTPUT_DIR]
#
# The binary must be a statically linked aarch64 ELF built from this repository.
# The script does not compile anything: it validates the binary, assembles the
# package tree (binary, procd init, example config, mosquitto drop-in, optional
# Windfront assets) and writes a tarball plus a manifest with hashes and
# provenance fields. Compilation belongs to CI (see .github/workflows/openwrt-
# release.yml) so the manifest can pin the Rust SHA.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINARY="${1:-$ROOT/openwrt/files/zigbee2mqtt-rs}"
OUTPUT_DIR="${2:-$ROOT/dist}"
FILES_DIR="$ROOT/openwrt/files"

die() { echo "build-openwrt-artifact: $*" >&2; exit 2; }

[[ -f "$BINARY" ]] || die "binary not found: $BINARY"
[[ -f "$FILES_DIR/zigbee2mqtt-rs.init" ]] || die "missing procd init script"
[[ -f "$FILES_DIR/config.example.yaml" ]] || die "missing example config"

# ── Validate the ELF is AArch64 and statically linked ────────────────────────
# e_machine lives at offset 18 (little endian); 0x00b7 is AArch64.
machine="$(od -A n -t x1 -j 18 -N 2 "$BINARY" | tr -d ' \n')"
[[ "$machine" == "b700" ]] || die "expected AArch64 ELF (e_machine=0x00b7), got 0x$machine"

if command -v file >/dev/null 2>&1; then
  file "$BINARY" | grep -q 'ARM aarch64' || die "file(1) does not report ARM aarch64"
  if command -v readelf >/dev/null 2>&1; then
    if readelf -l "$BINARY" | grep -q 'INTERP'; then
      die "binary has a dynamic interpreter; OpenWrt artifact must be static"
    fi
  else
    file "$BINARY" | grep -q 'statically linked' || die "binary is not statically linked"
  fi
fi

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -n1)"
[[ -n "$VERSION" ]] || die "cannot read version from Cargo.toml"
GIT_SHA="$(git -C "$ROOT" rev-parse HEAD 2>/dev/null || echo unknown)"
GIT_DIRTY="false"
if [[ -n "$(git -C "$ROOT" status --porcelain --untracked-files=no 2>/dev/null || true)" ]]; then
  GIT_DIRTY="true"
fi
GIT_UNTRACKED="$(git -C "$ROOT" status --porcelain --untracked-files=normal 2>/dev/null | grep -c '^??' || true)"
RUSTC_VERSION="$(rustc --version 2>/dev/null || echo unknown)"
BUILD_UTC="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

mkdir -p "$OUTPUT_DIR"
STAGE="$(mktemp -d "$OUTPUT_DIR/.stage.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT

install -D -m 0755 "$BINARY" "$STAGE/usr/bin/zigbee2mqtt-rs"
install -D -m 0755 "$FILES_DIR/zigbee2mqtt-rs.init" "$STAGE/etc/init.d/zigbee2mqtt-rs"
install -D -m 0644 "$FILES_DIR/config.example.yaml" "$STAGE/etc/zigbee2mqtt-rs/config.yaml"
if [[ -f "$FILES_DIR/99-zigbee2mqtt-rs.conf" ]]; then
  install -D -m 0644 "$FILES_DIR/99-zigbee2mqtt-rs.conf" \
    "$STAGE/etc/mosquitto/conf.d/99-zigbee2mqtt-rs.conf"
fi
if [[ -d "$FILES_DIR/windfront" ]]; then
  mkdir -p "$STAGE/www/zigbee2mqtt"
  cp -R "$FILES_DIR/windfront/." "$STAGE/www/zigbee2mqtt/"
fi

BIN_SHA256="$(sha256sum "$BINARY" | cut -d' ' -f1)"
BIN_SIZE="$(stat -c %s "$BINARY")"
NAME="zigbee2mqtt-rs-${VERSION}-aarch64-openwrt"
ARCHIVE="$OUTPUT_DIR/${NAME}.tar.gz"

tar -C "$STAGE" --numeric-owner --owner=0 --group=0 -czf "$ARCHIVE" .
ARCHIVE_SHA256="$(sha256sum "$ARCHIVE" | cut -d' ' -f1)"
ARCHIVE_SIZE="$(stat -c %s "$ARCHIVE")"

cat > "$OUTPUT_DIR/${NAME}.manifest.json" <<JSON
{
  "schema": 1,
  "name": "${NAME}",
  "version": "${VERSION}",
  "target": "aarch64-unknown-linux-musl",
  "archive": "$(basename "$ARCHIVE")",
  "archive_sha256": "${ARCHIVE_SHA256}",
  "archive_bytes": ${ARCHIVE_SIZE},
  "binary_sha256": "${BIN_SHA256}",
  "binary_bytes": ${BIN_SIZE},
  "binary_source": "$(realpath --relative-to="$ROOT" "$BINARY" 2>/dev/null || echo "$BINARY")",
  "elf_machine": "aarch64",
  "static": true,
  "rustc": "${RUSTC_VERSION}",
  "git_sha": "${GIT_SHA}",
  "git_dirty": ${GIT_DIRTY},
  "git_untracked_files": ${GIT_UNTRACKED},
  "built_utc": "${BUILD_UTC}",
  "provenance": "compiled-binary-supplied-to-packager",
  "notes": [
    "The packager does not compile; CI must build the binary from git_sha.",
    "Verify binary_sha256 against the CI build log before publishing.",
    "Hashes are integrity checks, not signatures."
  ]
}
JSON

echo "artifact: $ARCHIVE"
echo "manifest: $OUTPUT_DIR/${NAME}.manifest.json"
echo "binary sha256: $BIN_SHA256 ($BIN_SIZE bytes)"
echo "archive sha256: $ARCHIVE_SHA256 ($ARCHIVE_SIZE bytes)"
