// Original: .upstream/zigbee-herdsman/src/adapter/adapter.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use serde_json::Value;

/// Upstream `AdapterEvents.ZclPayload` — adapter.ts.
#[derive(Debug, Clone)]
pub struct ZclPayload {
    /// Upstream camelCase: `clusterID`.
    pub cluster_id: u16,
    pub data: Vec<u8>,
    pub header: crate::zspec::zcl::zcl_header::Header,
    pub address: Value,
    pub endpoint: u8,
    pub linkquality: u8,
    /// Upstream camelCase: `groupID`.
    pub group_id: u16,
    /// Upstream camelCase: `wasBroadcast`.
    pub was_broadcast: bool,
    /// Upstream camelCase: `destinationEndpoint`.
    pub destination_endpoint: u8,
}
