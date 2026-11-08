// Original: .upstream/zigbee-herdsman/src/models/network-options.ts

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkOptions {
    pub pan_id: u16,
    pub extended_pan_id: Vec<u8>,
    pub channel_list: Vec<u8>,
    pub network_key: Vec<u8>,
    pub network_key_distribute: bool,
    pub has_default_extended_pan_id: bool,
}
