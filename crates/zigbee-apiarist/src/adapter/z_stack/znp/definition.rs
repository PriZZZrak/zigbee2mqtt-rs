// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/znp/definition.ts
// TODO: port the remaining MT command catalog entries.
//
// Only the commands exercised by the ported tests are modelled here. Names,
// IDs, types and parameter lists mirror upstream.

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
    pub has_response: bool,
    pub zdo_cluster_id: Option<u16>,
}

impl MtCmd {
    const fn sreq(
        name: &'static str,
        id: u8,
        request: &'static [MtParameter],
        response: &'static [MtParameter],
    ) -> Self {
        Self {
            name,
            id,
            r#type: Type::SREQ,
            request,
            response,
            has_response: true,
            zdo_cluster_id: None,
        }
    }

    const fn areq(name: &'static str, id: u8, request: &'static [MtParameter]) -> Self {
        Self {
            name,
            id,
            r#type: Type::AREQ,
            request,
            response: &[],
            has_response: false,
            zdo_cluster_id: None,
        }
    }

    const fn zdo_sreq(name: &'static str, id: u8, cluster_id: u16) -> Self {
        Self {
            name,
            id,
            r#type: Type::SREQ,
            request: &[],
            response: &[],
            has_response: false,
            zdo_cluster_id: Some(cluster_id),
        }
    }

    const fn zdo_areq(name: &'static str, id: u8, cluster_id: u16) -> Self {
        Self {
            name,
            id,
            r#type: Type::AREQ,
            request: &[],
            response: &[],
            has_response: false,
            zdo_cluster_id: Some(cluster_id),
        }
    }
}

const fn param(name: &'static str, parameter_type: ParameterType) -> MtParameter {
    MtParameter {
        name,
        parameter_type: parameter_type as u32,
    }
}

// ------------------------------------ SYS ------------------------------------

const RESET_REQUEST: [MtParameter; 1] = [param("type", ParameterType::UINT8)];
const VERSION_RESPONSE: [MtParameter; 6] = [
    param("transportrev", ParameterType::UINT8),
    param("product", ParameterType::UINT8),
    param("majorrel", ParameterType::UINT8),
    param("minorrel", ParameterType::UINT8),
    param("maintrel", ParameterType::UINT8),
    param("revision", ParameterType::UINT32),
];
const OSAL_NV_READ_REQUEST: [MtParameter; 2] = [
    param("id", ParameterType::UINT16),
    param("offset", ParameterType::UINT8),
];
const OSAL_NV_READ_RESPONSE: [MtParameter; 3] = [
    param("status", ParameterType::UINT8),
    param("len", ParameterType::UINT8),
    param("value", ParameterType::BUFFER),
];
const RESET_INDICATION: [MtParameter; 6] = [
    param("reason", ParameterType::UINT8),
    param("transportrev", ParameterType::UINT8),
    param("productid", ParameterType::UINT8),
    param("majorrel", ParameterType::UINT8),
    param("minorrel", ParameterType::UINT8),
    param("hwrev", ParameterType::UINT8),
];
const PING_RESPONSE: [MtParameter; 1] = [param("capabilities", ParameterType::UINT16)];

pub const SYS: &[MtCmd] = &[
    MtCmd::areq("resetReq", 0, &RESET_REQUEST),
    MtCmd::sreq("ping", 1, &[], &PING_RESPONSE),
    MtCmd::sreq("version", 2, &[], &VERSION_RESPONSE),
    MtCmd::sreq(
        "osalNvRead",
        8,
        &OSAL_NV_READ_REQUEST,
        &OSAL_NV_READ_RESPONSE,
    ),
    MtCmd::areq("resetInd", 128, &RESET_INDICATION),
];

// ------------------------------------- AF ------------------------------------

const REGISTER_REQUEST: [MtParameter; 9] = [
    param("endpoint", ParameterType::UINT8),
    param("appprofid", ParameterType::UINT16),
    param("appdeviceid", ParameterType::UINT16),
    param("appdevver", ParameterType::UINT8),
    param("latencyreq", ParameterType::UINT8),
    param("appnuminclusters", ParameterType::UINT8),
    param("appinclusterlist", ParameterType::LIST_UINT16),
    param("appnumoutclusters", ParameterType::UINT8),
    param("appoutclusterlist", ParameterType::LIST_UINT16),
];
const STATUS_RESPONSE: [MtParameter; 1] = [param("status", ParameterType::UINT8)];
const DATA_CONFIRM: [MtParameter; 3] = [
    param("status", ParameterType::UINT8),
    param("endpoint", ParameterType::UINT8),
    param("transid", ParameterType::UINT8),
];

pub const AF: &[MtCmd] = &[
    MtCmd::sreq("register", 0, &REGISTER_REQUEST, &STATUS_RESPONSE),
    MtCmd::areq("dataConfirm", 128, &DATA_CONFIRM),
];

// ------------------------------------ ZDO ------------------------------------

const STARTUP_FROM_APP_REQUEST: [MtParameter; 1] = [param("startdelay", ParameterType::UINT16)];
const AUTO_FIND_DESTINATION_REQUEST: [MtParameter; 1] = [param("endpoint", ParameterType::UINT8)];
const STATE_CHANGE_INDICATION: [MtParameter; 1] = [param("state", ParameterType::UINT8)];

pub const ZDO: &[MtCmd] = &[
    MtCmd::zdo_sreq("nwkAddrReq", 0, 0x0000),
    MtCmd::zdo_sreq("ieeeAddrReq", 1, 0x0001),
    MtCmd::zdo_sreq("nodeDescReq", 2, 0x0002),
    MtCmd::zdo_sreq("powerDescReq", 3, 0x0003),
    MtCmd::zdo_sreq("simpleDescReq", 4, 0x0004),
    MtCmd::zdo_sreq("activeEpReq", 5, 0x0005),
    MtCmd::zdo_sreq("matchDescReq", 6, 0x0006),
    MtCmd::sreq(
        "startupFromApp",
        64,
        &STARTUP_FROM_APP_REQUEST,
        &STATUS_RESPONSE,
    ),
    MtCmd::areq("autoFindDestination", 65, &AUTO_FIND_DESTINATION_REQUEST),
    MtCmd::zdo_areq("nwkAddrRsp", 128, 0x8000),
    MtCmd::zdo_areq("ieeeAddrRsp", 129, 0x8001),
    MtCmd::zdo_areq("nodeDescRsp", 130, 0x8002),
    MtCmd::zdo_areq("powerDescRsp", 131, 0x8003),
    MtCmd::zdo_areq("simpleDescRsp", 132, 0x8004),
    MtCmd::zdo_areq("activeEpRsp", 133, 0x8005),
    MtCmd::zdo_areq("matchDescRsp", 134, 0x8006),
    MtCmd::areq("stateChangeInd", 192, &STATE_CHANGE_INDICATION),
    MtCmd::zdo_areq("endDeviceAnnceInd", 193, 0x0013),
];

// ------------------------------------ SAPI -----------------------------------

const START_CONFIRM: [MtParameter; 1] = [param("status", ParameterType::UINT8)];

pub const SAPI: &[MtCmd] = &[MtCmd::areq("startConfirm", 128, &START_CONFIRM)];

/// Upstream `Definition[subsystem]`.
pub fn for_subsystem(subsystem: Subsystem) -> &'static [MtCmd] {
    match subsystem {
        Subsystem::SYS => SYS,
        Subsystem::AF => AF,
        Subsystem::ZDO => ZDO,
        Subsystem::SAPI => SAPI,
        _ => &[],
    }
}

pub fn for_subsystem_number(subsystem: u16) -> Option<&'static [MtCmd]> {
    let subsystem = Subsystem::from_u8(subsystem as u8)?;
    Some(for_subsystem(subsystem))
}

pub fn find_by_name(subsystem: Subsystem, name: &str) -> Option<&'static MtCmd> {
    for_subsystem(subsystem)
        .iter()
        .find(|command| command.name == name)
}

pub fn find_by_id(subsystem: Subsystem, id: u8) -> Option<&'static MtCmd> {
    for_subsystem(subsystem)
        .iter()
        .find(|command| command.id == id)
}

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
