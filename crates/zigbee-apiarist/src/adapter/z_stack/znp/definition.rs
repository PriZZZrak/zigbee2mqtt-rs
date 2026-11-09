// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/znp/definition.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).
//
// The command catalog itself is generated from upstream by
// tools/oracle/gen-definition.mjs (see definition_data.rs).

use crate::adapter::z_stack::unpi::constants::{Subsystem, Type};
use crate::adapter::z_stack::znp::parameter_type::ParameterType;

/// Upstream `MtParameter`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MtParameter {
    pub name: &'static str,
    pub parameter_type: u32,
}

/// Upstream `MtCmd`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MtCmd {
    pub name: &'static str,
    pub id: u8,
    pub r#type: Type,
    pub request: &'static [MtParameter],
    pub response: &'static [MtParameter],
    pub zdo_cluster_id: Option<u16>,
}

include!("definition_data.rs");

/// Upstream `Definition[subsystem]`.
pub fn for_subsystem(subsystem: Subsystem) -> &'static [MtCmd] {
    match subsystem {
        Subsystem::RESERVED => RESERVED,
        Subsystem::SYS => SYS,
        Subsystem::MAC => MAC,
        Subsystem::NWK => NWK,
        Subsystem::AF => AF,
        Subsystem::ZDO => ZDO,
        Subsystem::SAPI => SAPI,
        Subsystem::UTIL => UTIL,
        Subsystem::DEBUG => DEBUG,
        Subsystem::APP => APP,
        Subsystem::APP_CNF => APP_CNF,
        Subsystem::GREENPOWER => GREENPOWER,
    }
}

pub fn for_subsystem_number(subsystem: u16) -> Option<&'static [MtCmd]> {
    let subsystem = Subsystem::from_u8(subsystem as u8)?;
    Some(for_subsystem(subsystem))
}

/// Upstream `Definition[subsystem].find((c) => c.name === name)`.
pub fn find_by_name(subsystem: Subsystem, name: &str) -> Option<&'static MtCmd> {
    for_subsystem(subsystem)
        .iter()
        .find(|command| command.name == name)
}

/// Upstream `Definition[subsystem].find((c) => c.ID === id)`.
pub fn find_by_id(subsystem: Subsystem, id: u8) -> Option<&'static MtCmd> {
    for_subsystem(subsystem)
        .iter()
        .find(|command| command.id == id)
}

/// Upstream `Definition[Subsystem.ZDO].find((c) => isMtCmdSreqZdo(c) && c.zdoClusterId === id)`.
pub fn find_zdo_request(subsystem: Subsystem, cluster_id: u16) -> Option<&'static MtCmd> {
    for_subsystem(subsystem)
        .iter()
        .find(|command| command.r#type == Type::SREQ && command.zdo_cluster_id == Some(cluster_id))
}

/// Upstream `isMtCmdAreqZdo`.
pub fn is_mt_cmd_areq_zdo(command: &MtCmd) -> bool {
    command.r#type == Type::AREQ && command.zdo_cluster_id.is_some()
}

/// Upstream `isMtCmdSreqZdo`.
pub fn is_mt_cmd_sreq_zdo(command: &MtCmd) -> bool {
    command.r#type == Type::SREQ && command.zdo_cluster_id.is_some()
}

/// Upstream `Definition[Subsystem.ZDO].find((c) => isMtCmdAreqZdo(c) && c.zdoClusterId === id)`.
pub fn find_zdo_response(subsystem: Subsystem, cluster_id: u16) -> Option<&'static MtCmd> {
    for_subsystem(subsystem)
        .iter()
        .find(|command| is_mt_cmd_areq_zdo(command) && command.zdo_cluster_id == Some(cluster_id))
}
