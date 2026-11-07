// Original: .upstream/zigbee-herdsman/src/zspec/zcl/definition/foundation.ts
// TODO: port the full Foundation command catalog.

use crate::zspec::zcl::definition::cluster::Command;

/// Upstream `Foundation`.
#[allow(non_snake_case)]
pub struct Foundation;

#[allow(non_snake_case)]
impl Foundation {
    pub const read: Command = Command::new("read", 0).with_response(1);
    pub const readRsp: Command = Command::new("readRsp", 1);
    pub const write: Command = Command::new("write", 2);
    pub const writeUndiv: Command = Command::new("writeUndiv", 3);
    pub const writeRsp: Command = Command::new("writeRsp", 4);
    pub const writeNoRsp: Command = Command::new("writeNoRsp", 5);
    pub const configReport: Command = Command::new("configReport", 6).with_response(7);
    pub const configReportRsp: Command = Command::new("configReportRsp", 7);
    pub const readReportConfig: Command = Command::new("readReportConfig", 8);
    pub const readReportConfigRsp: Command = Command::new("readReportConfigRsp", 9);
    pub const report: Command = Command::new("report", 10);
    pub const defaultRsp: Command = Command::new("defaultRsp", 11);
    pub const discover: Command = Command::new("discover", 12);
    pub const discoverRsp: Command = Command::new("discoverRsp", 13);
    pub const readStructured: Command = Command::new("readStructured", 14);
    pub const writeStructured: Command = Command::new("writeStructured", 15);
    pub const writeStructuredRsp: Command = Command::new("writeStructuredRsp", 16);
    pub const discoverCommands: Command = Command::new("discoverCommands", 17);
    pub const discoverCommandsRsp: Command = Command::new("discoverCommandsRsp", 18);
    pub const discoverCommandsGen: Command = Command::new("discoverCommandsGen", 19);
    pub const discoverCommandsGenRsp: Command = Command::new("discoverCommandsGenRsp", 20);
    pub const discoverExt: Command = Command::new("discoverExt", 21);
    pub const discoverExtRsp: Command = Command::new("discoverExtRsp", 22);
}
