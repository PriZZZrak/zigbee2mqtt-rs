// Original: .upstream/zigbee-herdsman/src/models/backup.ts

use serde_json::Value;

use super::network_options::NetworkOptions;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NetworkKeyInfo {
    pub sequence_number: u32,
    pub frame_counter: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LinkKey {
    pub key: Vec<u8>,
    pub rx_counter: u32,
    pub tx_counter: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BackupDevice {
    pub network_address: Option<u16>,
    pub ieee_address: Vec<u8>,
    pub is_direct_child: bool,
    pub link_key: Option<LinkKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Backup {
    pub network_options: NetworkOptions,
    pub logical_channel: u8,
    pub network_key_info: NetworkKeyInfo,
    pub coordinator_ieee_address: Vec<u8>,
    pub security_level: u8,
    pub network_update_id: u8,
    pub devices: Vec<BackupDevice>,
    pub znp_version: Option<u16>,
    pub trust_center_link_key_seed: Option<Vec<u8>>,
    pub ezsp_version: Option<u16>,
    pub hashed_tclk: Option<Vec<u8>>,
}

impl Default for NetworkOptions {
    fn default() -> Self {
        Self {
            pan_id: 0,
            extended_pan_id: Vec::new(),
            channel_list: Vec::new(),
            network_key: Vec::new(),
            network_key_distribute: false,
            has_default_extended_pan_id: false,
        }
    }
}

pub type UnifiedBackupStorage = Value;
pub type LegacyBackupStorage = Value;
