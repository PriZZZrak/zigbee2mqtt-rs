// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/znp/utils.ts

use super::definition::MtCmd;
use crate::adapter::z_stack::unpi::constants::Type;

/// Upstream `isMtCmdAreqZdo`.
pub fn is_mt_cmd_areq_zdo(command: &MtCmd) -> bool {
    command.r#type == Type::AREQ && command.zdo_cluster_id.is_some()
}

/// Upstream `isMtCmdSreqZdo`.
pub fn is_mt_cmd_sreq_zdo(command: &MtCmd) -> bool {
    command.r#type == Type::SREQ && command.zdo_cluster_id.is_some()
}
