#!/bin/sh
# Shared staging helpers for the OpenWrt payload and the SDK package.
# Usage: . packaging/lib.sh

stage_js_tree() {
	src=$1
	dst=$2
	mkdir -p "$dst"
	dst=$(CDPATH= cd -- "$dst" && pwd)
	(cd "$src" && find . -type f \( -name '*.js' -o -name '*.json' \) \
		! -name '*.map' ! -name '*.d.ts' -exec cp --parents {} "$dst" \;)
}

# stage_artifact <artifact-dir> <dest-root> [--with-devices]
# Produces <dest-root>/manifest.json, index.z2idx1, dist/{index.js,models-index.json,
# converters/,lib/} and, only with --with-devices, dist/devices/.
stage_artifact() {
	artifact=$1
	dest=$2
	with_devices=${3:-}
	mkdir -p "$dest/dist"
	dest=$(CDPATH= cd -- "$dest" && pwd)
	cp "$artifact/manifest.json" "$dest/manifest.json"
	sha256sum "$dest/manifest.json" | awk '{print $1}' > "$dest/manifest.json.sha256"
	cp "$artifact/index.z2idx1" "$dest/index.z2idx1"
	cp "$artifact/dist/index.js" "$dest/dist/index.js"
	cp "$artifact/dist/models-index.json" "$dest/dist/models-index.json"
	stage_js_tree "$artifact/dist/converters" "$dest/dist/converters"
	stage_js_tree "$artifact/dist/lib" "$dest/dist/lib"
	if [ "$with_devices" = "--with-devices" ]; then
		stage_js_tree "$artifact/dist/devices" "$dest/dist/devices"
	fi
}

# strip_or_fail <input> <output>
strip_or_fail() {
	input=$1
	output=$2
	strip_bin=${LLVM_STRIP:-$(command -v llvm-strip || true)}
	if [ -z "$strip_bin" ]; then
		strip_bin=$(find "${RUSTUP_HOME:-$HOME/.rustup}" -path '*/bin/llvm-strip' 2>/dev/null | head -n1)
	fi
	if [ -n "$strip_bin" ]; then
		"$strip_bin" "$input" -o "$output"
	elif command -v strip >/dev/null 2>&1; then
		cp "$input" "$output"
		strip "$output"
	else
		echo "no usable strip tool; refusing to ship an unstripped binary" >&2
		return 1
	fi
}
