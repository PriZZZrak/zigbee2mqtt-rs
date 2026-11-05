include $(TOPDIR)/rules.mk

PKG_NAME:=zigbee2mqtt-rs
PKG_VERSION:=0.1.0
PKG_RELEASE:=1

PKG_LICENSE:=MIT
PKG_MAINTAINER:=chr4
PKG_BUILD_DEPENDS:=
# No upstream build system is run inside the SDK: the binary is
# cross-compiled beforehand and only packaged here. PKG_INSTALL must be
# empty (not "0") - $(if $(PKG_INSTALL),...) treats "0" as true.
PKG_INSTALL:=

include $(INCLUDE_DIR)/package.mk

define Package/zigbee2mqtt-rs
  SECTION:=utils
  CATEGORY:=Utilities
  TITLE:=Zigbee to MQTT bridge (Rust)
  DEPENDS:=+libc +mosquitto
  URL:=https://github.com/PriZZZrak/zigbee2mqtt-rs
endef

define Package/zigbee2mqtt-rs/description
  Zigbee to MQTT bridge written in Rust.
  The binary is cross-built for aarch64-unknown-linux-musl and is
  statically linked, so it needs no additional runtime libraries.
endef

define Package/zigbee2mqtt-rs/conffiles
/etc/zigbee2mqtt-rs/config.yaml
endef

# The Rust binary is cross-compiled outside the SDK with
#   cargo zigbuild --release --target aarch64-unknown-linux-musl
# and dropped here as files/zigbee2mqtt-rs. Packaging it through the
# SDK keeps the generated .ipk byte-compatible with what opkg expects.
define Build/Prepare
	mkdir -p $(PKG_BUILD_DIR)/bin
endef

define Build/Compile
	$(CP) $(CURDIR)/files/zigbee2mqtt-rs $(PKG_BUILD_DIR)/bin/zigbee2mqtt-rs
	chmod 0755 $(PKG_BUILD_DIR)/bin/zigbee2mqtt-rs
endef

define Package/zigbee2mqtt-rs/install
	$(INSTALL_DIR) $(1)/usr/bin
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/bin/zigbee2mqtt-rs $(1)/usr/bin/zigbee2mqtt-rs
	$(INSTALL_DIR) $(1)/etc/init.d
	$(INSTALL_BIN) ./files/zigbee2mqtt-rs.init $(1)/etc/init.d/zigbee2mqtt-rs
	$(INSTALL_DIR) $(1)/etc/zigbee2mqtt-rs
	$(INSTALL_CONF) ./files/config.example.yaml $(1)/etc/zigbee2mqtt-rs/config.yaml
	$(INSTALL_DIR) $(1)/etc/mosquitto/conf.d
	$(INSTALL_CONF) ./files/99-zigbee2mqtt-rs.conf $(1)/etc/mosquitto/conf.d/99-zigbee2mqtt-rs.conf
	# Frontend (optional): only install if files/frontend/ exists
	if [ -d ./files/windfront ]; then \
		$(INSTALL_DIR) $(1)/www/zigbee2mqtt; \
		$(CP) ./files/windfront/* $(1)/www/zigbee2mqtt/; \
	fi
endef

$(eval $(call BuildPackage,zigbee2mqtt-rs))
