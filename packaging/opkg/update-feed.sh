#!/bin/sh
# Regenerates an opkg feed index (`Packages`, `Packages.gz`, optionally
# `Packages.sig`) from the `<ipk>.meta` stanzas written by `build-ipk.sh`.
#
# Usage:
#   update-feed.sh <feed-dir> [usign-private-key]
set -eu

feed=$1
key=${2:-}

index="$feed/Packages"
: > "$index"

count=0
for meta in "$feed"/*.meta; do
    [ -e "$meta" ] || continue
    cat "$meta" >> "$index"
    count=$((count + 1))
done

if [ "$count" -eq 0 ]; then
    echo "error: no *.meta package stanzas in $feed" >&2
    exit 1
fi

gzip -9c "$index" > "$feed/Packages.gz"

if [ -n "$key" ] && [ -f "$key" ]; then
    # usign is provided by the OpenWrt SDK / `usign` package.
    if ! command -v usign >/dev/null 2>&1; then
        echo "error: usign is required to sign $index" >&2
        exit 1
    fi
    usign -S -m "$index" -s "$key" -x "$feed/Packages.sig"
fi

echo "updated $index ($count packages)"
