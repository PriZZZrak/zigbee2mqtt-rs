// Original: .upstream/zigbee-herdsman/src/zspec/zdo/utils.ts
// TODO: port ZDO utils.

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

pub fn create_server_mask(_mask: ServerMask) -> u16 {
    todo!()
}

pub fn get_mac_cap_flags(_value: u8) -> MacCapabilities {
    todo!()
}

pub fn get_response_cluster_id(request: u16) -> Option<u16> {
    todo!()
}

pub fn get_server_mask(_value: u16) -> ServerMask {
    todo!()
}
