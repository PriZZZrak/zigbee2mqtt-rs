// Original: .upstream/zigbee-herdsman/src/zspec/zdo/utils.ts

/// Upstream `getMacCapFlags` result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacCapabilities {
    /// Upstream camelCase: `alternatePANCoordinator`.
    pub alternate_pan_coordinator: u8,
    /// Upstream camelCase: `deviceType`.
    pub device_type: u8,
    /// Upstream camelCase: `powerSource`.
    pub power_source: u8,
    /// Upstream camelCase: `rxOnWhenIdle`.
    pub rx_on_when_idle: u8,
    /// Upstream camelCase: `reserved1`.
    pub reserved1: u8,
    /// Upstream camelCase: `reserved2`.
    pub reserved2: u8,
    /// Upstream camelCase: `securityCapability`.
    pub security_capability: u8,
    /// Upstream camelCase: `allocateAddress`.
    pub allocate_address: u8,
}

/// Upstream `getServerMask` result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerMask {
    /// Upstream camelCase: `primaryTrustCenter`.
    pub primary_trust_center: u16,
    /// Upstream camelCase: `backupTrustCenter`.
    pub backup_trust_center: u16,
    /// Upstream camelCase: `deprecated1`.
    pub deprecated1: u16,
    /// Upstream camelCase: `deprecated2`.
    pub deprecated2: u16,
    /// Upstream camelCase: `deprecated3`.
    pub deprecated3: u16,
    /// Upstream camelCase: `deprecated4`.
    pub deprecated4: u16,
    /// Upstream camelCase: `networkManager`.
    pub network_manager: u16,
    /// Upstream camelCase: `reserved1`.
    pub reserved1: u16,
    /// Upstream camelCase: `reserved2`.
    pub reserved2: u16,
    /// Upstream camelCase: `stackComplianceRevision`.
    pub stack_compliance_revision: u16,
}

pub fn create_server_mask(mask: ServerMask) -> u16 {
    (mask.primary_trust_center & 0x01)
        | ((mask.backup_trust_center << 1) & 0x02)
        | ((mask.deprecated1 << 2) & 0x04)
        | ((mask.deprecated2 << 3) & 0x08)
        | ((mask.deprecated3 << 4) & 0x10)
        | ((mask.deprecated4 << 5) & 0x20)
        | ((mask.network_manager << 6) & 0x40)
        | ((mask.reserved1 << 7) & 0x80)
        | ((mask.reserved2 << 8) & 0x100)
        | ((mask.stack_compliance_revision << 9) & 0xfe00)
}

pub fn get_mac_cap_flags(value: u8) -> MacCapabilities {
    MacCapabilities {
        alternate_pan_coordinator: value & 0x01,
        device_type: (value & 0x02) >> 1,
        power_source: (value & 0x04) >> 2,
        rx_on_when_idle: (value & 0x08) >> 3,
        reserved1: (value & 0x10) >> 4,
        reserved2: (value & 0x20) >> 5,
        security_capability: (value & 0x40) >> 6,
        allocate_address: (value & 0x80) >> 7,
    }
}

pub fn get_response_cluster_id(request: u16) -> Option<u16> {
    use crate::zspec::zdo::definition::clusters::ClusterId;
    const RESPONSES: &[u16] = &[
        ClusterId::NETWORK_ADDRESS_RESPONSE,
        ClusterId::IEEE_ADDRESS_RESPONSE,
        ClusterId::NODE_DESCRIPTOR_RESPONSE,
        ClusterId::POWER_DESCRIPTOR_RESPONSE,
        ClusterId::SIMPLE_DESCRIPTOR_RESPONSE,
        ClusterId::ACTIVE_ENDPOINTS_RESPONSE,
        ClusterId::MATCH_DESCRIPTORS_RESPONSE,
        ClusterId::SYSTEM_SERVER_DISCOVERY_RESPONSE,
        ClusterId::PARENT_ANNOUNCE_RESPONSE,
        ClusterId::BIND_RESPONSE,
        ClusterId::UNBIND_RESPONSE,
        ClusterId::CLEAR_ALL_BINDINGS_RESPONSE,
        ClusterId::LQI_TABLE_RESPONSE,
        ClusterId::ROUTING_TABLE_RESPONSE,
        ClusterId::BINDING_TABLE_RESPONSE,
        ClusterId::LEAVE_RESPONSE,
        ClusterId::PERMIT_JOINING_RESPONSE,
        ClusterId::NWK_UPDATE_RESPONSE,
        ClusterId::NWK_ENHANCED_UPDATE_RESPONSE,
        ClusterId::NWK_IEEE_JOINING_LIST_RESPONSE,
        ClusterId::NWK_BEACON_SURVEY_RESPONSE,
        ClusterId::START_KEY_NEGOTIATION_RESPONSE,
        ClusterId::RETRIEVE_AUTHENTICATION_TOKEN_RESPONSE,
        ClusterId::GET_AUTHENTICATION_LEVEL_RESPONSE,
        ClusterId::SET_CONFIGURATION_RESPONSE,
        ClusterId::GET_CONFIGURATION_RESPONSE,
        ClusterId::START_KEY_UPDATE_RESPONSE,
        ClusterId::DECOMMISSION_RESPONSE,
        ClusterId::CHALLENGE_RESPONSE,
    ];
    if request > 0x8000 || request == ClusterId::END_DEVICE_ANNOUNCE {
        return None;
    }
    let response = request + 0x8000;
    RESPONSES.contains(&response).then_some(response)
}

pub fn get_server_mask(value: u16) -> ServerMask {
    ServerMask {
        primary_trust_center: value & 0x01,
        backup_trust_center: (value & 0x02) >> 1,
        deprecated1: (value & 0x04) >> 2,
        deprecated2: (value & 0x08) >> 3,
        deprecated3: (value & 0x10) >> 4,
        deprecated4: (value & 0x20) >> 5,
        network_manager: (value & 0x40) >> 6,
        reserved1: (value & 0x80) >> 7,
        reserved2: (value & 0x100) >> 8,
        stack_compliance_revision: (value & 0xfe00) >> 9,
    }
}
