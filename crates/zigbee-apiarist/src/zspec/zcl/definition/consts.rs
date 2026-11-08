// Original: .upstream/zigbee-herdsman/src/zspec/zcl/definition/consts.ts

use std::collections::BTreeMap;

/// Upstream `POWER_SOURCES`.
pub fn power_sources() -> BTreeMap<u8, &'static str> {
    BTreeMap::from([
        (0, "Unknown"),
        (1, "Mains (single phase)"),
        (2, "Mains (3 phase)"),
        (3, "Battery"),
        (4, "DC Source"),
        (5, "Emergency mains constantly powered"),
        (6, "Emergency mains and transfer switch"),
    ])
}

/// Upstream `ENDPOINT_DEVICE_TYPE`.
pub fn endpoint_device_type() -> BTreeMap<&'static str, u16> {
    BTreeMap::from([
        ("ZLLOnOffLight", 0x0000u16),
        ("ZLLOnOffPluginUnit", 0x0010),
        ("ZLLDimmableLight", 0x0100),
        ("ZLLDimmablePluginUnit", 0x0110),
        ("ZLLColorLight", 0x0200),
        ("ZLLExtendedColorLight", 0x0210),
        ("ZLLColorTemperatureLight", 0x0220),
        ("HAOnOffLight", 0x0100),
        ("HADimmableLight", 0x0101),
        ("HAColorLight", 0x0102),
    ])
}
