include $(TOPDIR)/rules.mk

PKG_NAME:=zigbee2mqtt-rs
PKG_VERSION:=$(if $(Z2M_RS_VERSION),$(Z2M_RS_VERSION),0.1.0)
PKG_RELEASE:=1
PKG_LICENSE:=MIT
PKG_MAINTAINER:=zigbee2mqtt-rs <noreply@github.com>
PKG_BUILD_DEPENDS:=
# The binary is cross-built outside the SDK and staged in files/. Keep
# PKG_INSTALL empty (not "0") so the default Build/Install is skipped.
PKG_INSTALL:=

include $(INCLUDE_DIR)/package.mk

define Package/zigbee2mqtt-rs
  SECTION:=utils
  CATEGORY:=Utilities
  TITLE:=Zigbee to MQTT bridge (Rust)
  DEPENDS:=
  URL:=https://github.com/PriZZZrak/zigbee2mqtt-rs
endef

define Package/zigbee2mqtt-rs/description
  Pure-Rust Zigbee to MQTT bridge. The aarch64-musl binary is statically
  linked; the package carries the compact converter index, trusted manifest,
  shared JS and the pinned zigbee-herdsman runtime. Device JS is not included
  in the on-demand variant.
endef

define Package/zigbee2mqtt-rs/conffiles
/etc/zigbee2mqtt-rs/mode
endef

define Build/Prepare
	mkdir -p $(PKG_BUILD_DIR)
endef

define Build/Compile
	$(CP) $(CURDIR)/files/zigbee2mqtt-rs $(PKG_BUILD_DIR)/zigbee2mqtt-rs
	chmod 0755 $(PKG_BUILD_DIR)/zigbee2mqtt-rs
endef

define Package/zigbee2mqtt-rs/install
	$(INSTALL_DIR) $(1)/usr/bin $(1)/etc/init.d $(1)/etc/zigbee2mqtt-rs
	$(INSTALL_BIN) $(PKG_BUILD_DIR)/zigbee2mqtt-rs $(1)/usr/bin/zigbee2mqtt-rs
	$(INSTALL_BIN) ./files/zigbee2mqtt-rs.init $(1)/etc/init.d/zigbee2mqtt-rs
	$(INSTALL_DATA) ./files/mode $(1)/etc/zigbee2mqtt-rs/mode
	$(INSTALL_DIR) $(1)/usr/share/zigbee2mqtt-rs
	$(CP) ./files/artifact $(1)/usr/share/zigbee2mqtt-rs/artifact
	$(CP) ./files/herdsman $(1)/usr/share/zigbee2mqtt-rs/herdsman
endef

$(eval $(call BuildPackage,zigbee2mqtt-rs))
