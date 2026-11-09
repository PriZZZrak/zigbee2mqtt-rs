// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/models/startup-options.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use crate::adapter::z_stack::adapter::tstype::ZnpVersion;

/// Upstream `TsType.NetworkOptions` (number/buffer form used before parsing).
#[derive(Debug, Clone, Default)]
pub struct ConfigNetworkOptions {
    pub pan_id: u16,
    pub extended_pan_id: Option<Vec<u8>>,
    pub network_key: Option<Vec<u8>>,
    pub channel_list: Vec<u8>,
    pub network_key_distribute: Option<bool>,
}

/// Upstream `TsType.AdapterOptions`.
#[derive(Debug, Clone, Default)]
pub struct AdapterOptions {
    pub concurrent: Option<u16>,
    pub delay: Option<u16>,
    pub disable_led: bool,
    pub transmit_power: Option<i8>,
    pub force_start_with_inconsistent_adapter_configuration: bool,
}

/// Upstream `StartupOptions`.
#[derive(Debug, Clone)]
pub struct StartupOptions {
    pub version: ZnpVersion,
    pub network_options: ConfigNetworkOptions,
    pub green_power_group: u16,
    pub backup_path: String,
    pub adapter_options: AdapterOptions,
}
