// Original: .upstream/zigbee-herdsman/src/zspec/zdo/buffaloZdo.ts
// TODO: port BuffaloZdo.

#[derive(Debug, Clone)]
pub struct BuffaloZdo {
    pub position: usize,
    pub buffer: Vec<u8>,
}

impl BuffaloZdo {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self {
            position: 0,
            buffer,
        }
    }

    pub fn get_position(&self) -> usize {
        self.position
    }

    pub fn set_position(&mut self, position: usize) {
        self.position = position;
    }

    pub fn get_byte(&self, position: usize) -> u8 {
        todo!()
    }

    pub fn set_byte(&mut self, position: usize, value: u8) {
        todo!()
    }

    pub fn is_more_by(&self, count: usize) -> bool {
        todo!()
    }

    pub fn read_tlvs(&mut self) -> Vec<serde_json::Value> {
        todo!()
    }

    pub fn write_global_tlv(&mut self, tlv: &serde_json::Value) {
        todo!()
    }

    /// Upstream is variadic: `buildRequest(hasZdoMessageOverhead, clusterId, ...args)`.
    pub fn build_request(
        has_zdo_message_overhead: bool,
        cluster_id: u16,
        args: &[serde_json::Value],
    ) -> Vec<u8> {
        todo!()
    }

    pub fn read_response(
        has_zdo_message_overhead: bool,
        cluster_id: u16,
        buffer: &[u8],
    ) -> serde_json::Value {
        todo!()
    }

    /// ZiGate-patched variant: same as `build_request` but with big-endian
    /// integer/address writes (upstream `patchZdoBuffaloBE`).
    pub fn build_request_be(
        has_zdo_message_overhead: bool,
        cluster_id: u16,
        args: &[serde_json::Value],
    ) -> Vec<u8> {
        todo!()
    }

    /// ZiGate-patched variant: reads stay little-endian.
    pub fn read_response_be(
        has_zdo_message_overhead: bool,
        cluster_id: u16,
        buffer: &[u8],
    ) -> serde_json::Value {
        todo!()
    }

    pub fn read_network_address_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_ieee_address_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_node_descriptor_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_power_descriptor_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_simple_descriptor_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_active_endpoints_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_match_descriptors_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_end_device_announce(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_system_server_discovery_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_parent_announce_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_lqi_table_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_routing_table_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_binding_table_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_nwk_update_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_nwk_enhanced_update_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_nwk_ieee_joining_list_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_nwk_unsolicited_enhanced_update_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_nwk_beacon_survey_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_start_key_negotiation_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_retrieve_authentication_token_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_get_authentication_level_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_set_configuration_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_get_configuration_response(&mut self) -> serde_json::Value {
        todo!()
    }
    pub fn read_challenge_response(&mut self) -> serde_json::Value {
        todo!()
    }
}
