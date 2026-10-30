#!/usr/bin/env bash
set -euo pipefail

# 0. Base packages
sudo apt update
sudo apt install -y build-essential curl wget git python3 file \
  libssl-dev pkg-config ca-certificates unzip xz-utils

# 1. Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
source "$HOME/.cargo/env"
rustup target add aarch64-unknown-linux-musl

# 2. Zig cross-compiler
ZIG_VER=0.13.0
wget -q "https://ziglang.org/download/${ZIG_VER}/zig-linux-x86_64-${ZIG_VER}.tar.xz" -O /tmp/zig.tar.xz
sudo mkdir -p /opt/zig && sudo tar -xf /tmp/zig.tar.xz -C /opt/zig --strip-components=1
sudo ln -sf /opt/zig/zig /usr/local/bin/zig

# 3. cargo-zigbuild
cargo install cargo-zigbuild

# 4. Verify cross-build
cd /config/workspace/zigbee2mqtt-rs
cargo zigbuild --release --target aarch64-unknown-linux-musl
file target/aarch64-unknown-linux-musl/release/zigbee2mqtt-rs

# 5. OpenWRT SDK (mediatek/filogic, 24.10)
SDK_VER=24.10.0
wget -q "https://downloads.openwrt.org/releases/${SDK_VER}/targets/mediatek/filogic/openwrt-sdk-${SDK_VER}-mediatek-filogic_gcc-13.3.0_musl.Linux-x86_64.tar.zst" -O /tmp/sdk.tar.zst
sudo mkdir -p /opt/openwrt-sdk && sudo tar --zstd -xf /tmp/sdk.tar.zst -C /opt/openwrt-sdk --strip-components=1
cd /opt/openwrt-sdk
./scripts/feeds update -a && ./scripts/feeds install -a

# 6. Package
mkdir -p package/zigbee2mqtt-rs
cp -r /config/workspace/zigbee2mqtt-rs/openwrt/* package/zigbee2mqtt-rs/
make defconfig
make package/zigbee2mqtt-rs/compile V=s
