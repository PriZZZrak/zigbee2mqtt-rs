#!/bin/sh
# Regenerates an opkg feed index (`Packages`, `Packages.gz`, optionally
# `Packages.sig`) from the .ipk files in a directory.
#
# Usage:
#   update-feed.sh <feed-dir> [usign-private-key]
set -eu

feed=$1
key=${2:-}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

index="$feed/Packages"
: > "$index"

for ipk in "$feed"/*.ipk; do
    [ -e "$ipk" ] || continue
    name=$(basename "$ipk")
    rm -rf "$work/extract"
    mkdir -p "$work/extract"
    (cd "$work/extract" && ar x "$ipk" control.tar.gz)
    tar -C "$work/extract" -xzf "$work/extract/control.tar.gz" ./control 2>/dev/null \
        || tar -C "$work/extract" -xzf "$work/extract/control.tar.gz" control
    control="$work/extract/control"
    cat "$control" >> "$index"
    printf 'Filename: %s\n' "$name" >> "$index"
    printf 'Size: %s\n' "$(wc -c < "$ipk" | tr -d ' ')" >> "$index"
    printf 'SHA256sum: %s\n' "$(sha256sum "$ipk" | awk '{print $1}')" >> "$index"
    printf '\n' >> "$index"
done

gzip -9c "$index" > "$feed/Packages.gz"

if [ -n "$key" ] && [ -f "$key" ]; then
    # usign is provided by the OpenWrt SDK / `usign` package.
    if ! command -v usign >/dev/null 2>&1; then
        echo "error: usign is required to sign $index" >&2
        exit 1
    fi
    usign -S -m "$index" -s "$key" -x "$feed/Packages.sig"
fi

echo "updated $index"
