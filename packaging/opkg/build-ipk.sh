#!/bin/sh
# Builds an OpenWrt .ipk from a prepared payload directory.
#
# Usage:
#   build-ipk.sh <package> <version> <architecture> <depends> <payload-dir> <out-dir> [description] [conffiles] [control-extra-dir]
#
# The payload directory contains the filesystem tree to install (e.g.
# usr/share/zigbee2mqtt/windfront/...). `depends` may be empty.
set -eu

package=$1
version=$2
architecture=$3
depends=$4
payload=$5
out=$6
description=${7:-"Zigbee2MQTT component"}
conffiles=${8:-}
control_extra=${9:-}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

installed_size=$(du -sk "$payload" | awk '{print $1}')
mkdir -p "$work/control"
{
    printf 'Package: %s\n' "$package"
    printf 'Version: %s\n' "$version"
    printf 'Architecture: %s\n' "$architecture"
    printf 'Installed-Size: %s\n' "$installed_size"
    printf 'Section: net\n'
    printf 'Priority: optional\n'
    printf 'Maintainer: zigbee2mqtt-rs <noreply@github.com>\n'
    printf 'Source: https://github.com/PriZZZrak/zigbee2mqtt-rs\n'
    if [ -n "$depends" ]; then
        printf 'Depends: %s\n' "$depends"
    fi
    printf 'Description: %s\n' "$description"
} > "$work/control/control"
if [ -n "$conffiles" ]; then
    printf 'Conffiles:\n' >> "$work/control/control"
    for file in $conffiles; do
        printf ' %s\n' "$file" >> "$work/control/control"
    done
fi
if [ -n "$control_extra" ] && [ -d "$control_extra" ]; then
    cp -a "$control_extra"/. "$work/control"/
    for script in postinst prerm preinst postrm; do
        [ -f "$work/control/$script" ] && chmod 0755 "$work/control/$script"
    done
fi

printf '2.0\n' > "$work/debian-binary"
tar -C "$work/control" -czf "$work/control.tar.gz" .
tar -C "$payload" -czf "$work/data.tar.gz" .

mkdir -p "$out"
ipk="$out/${package}_${version}_${architecture}.ipk"
rm -f "$ipk"
(cd "$work" && ar rc "$ipk" debian-binary control.tar.gz data.tar.gz)
printf '%s\n' "$ipk"
