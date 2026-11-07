// Original: .upstream/zigbee-herdsman/src/zspec/zcl/zclFrame.ts
// TODO: port frame creation/parsing/serialization.

use serde_json::Value;

use crate::zspec::zcl::definition::cluster::{Cluster, Command, Parameter};
use crate::zspec::zcl::zcl_header::{Direction, FrameControl, FrameType, Header};

/// Upstream command argument: a command name or a numeric command identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRef {
    Id(u8),
    Name(&'static str),
}

impl From<u8> for CommandRef {
    fn from(value: u8) -> Self {
        Self::Id(value)
    }
}

impl From<&'static str> for CommandRef {
    fn from(value: &'static str) -> Self {
        Self::Name(value)
    }
}

/// Upstream cluster argument: a cluster name or a numeric cluster identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterRef {
    Id(u16),
    Name(&'static str),
}

impl From<u16> for ClusterRef {
    fn from(value: u16) -> Self {
        Self::Id(value)
    }
}

impl From<&'static str> for ClusterRef {
    fn from(value: &'static str) -> Self {
        Self::Name(value)
    }
}

/// Upstream `ZclFrame`.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub header: Header,
    pub cluster: Cluster,
    pub command: Command,
    pub payload: Value,
}

impl Frame {
    /// Upstream `ZclFrame.create(...)`.
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        frame_type: u8,
        direction: u8,
        disable_default_response: bool,
        manufacturer_code: Option<u16>,
        transaction_sequence_number: u8,
        command: impl Into<CommandRef>,
        cluster: impl Into<ClusterRef>,
        payload: Value,
        _custom_clusters: Value,
        reserved_bits: u8,
    ) -> Self {
        let cluster = match cluster.into() {
            ClusterRef::Id(id) => cluster_from_id(id),
            ClusterRef::Name(name) => cluster_from_name(name),
        };
        let command = match command.into() {
            CommandRef::Id(id) => command_from_id(id),
            CommandRef::Name(name) => command_from_name(name),
        };
        let frame_type = if frame_type == 0 {
            FrameType::GLOBAL
        } else {
            FrameType::SPECIFIC
        };
        let direction = if direction == 0 {
            Direction::CLIENT_TO_SERVER
        } else {
            Direction::SERVER_TO_CLIENT
        };
        let header = Header::new(
            FrameControl {
                reserved_bits,
                frame_type,
                direction,
                disable_default_response,
                manufacturer_specific: manufacturer_code.is_some(),
            },
            manufacturer_code,
            transaction_sequence_number,
            command.ID,
        );
        Self {
            header,
            cluster,
            command,
            payload,
        }
    }

    /// Upstream `ZclFrame.fromBuffer(clusterId, header, buffer, customClusters)`.
    pub fn from_buffer(
        cluster_id: u16,
        header: Option<Header>,
        _buffer: &[u8],
        _custom_clusters: Value,
    ) -> Self {
        let header = header.unwrap_or_else(|| {
            Header::new(
                FrameControl {
                    reserved_bits: 0,
                    frame_type: FrameType::GLOBAL,
                    direction: Direction::CLIENT_TO_SERVER,
                    disable_default_response: false,
                    manufacturer_specific: false,
                },
                None,
                0,
                0,
            )
        });
        let cluster = cluster_from_id(cluster_id);
        let command = command_from_id(header.command_identifier);
        Self {
            header,
            cluster,
            command,
            payload: Value::Null,
        }
    }

    /// Upstream `frame.toBuffer()`.
    pub fn to_buffer(&self) -> Vec<u8> {
        let mut buffer = self.header.to_buffer();
        buffer.extend_from_slice(self.payload.to_string().as_bytes());
        buffer
    }

    /// Upstream `frame.toString()`.
    pub fn to_string(&self) -> String {
        todo!()
    }

    /// Upstream `frame.isCluster(name)`.
    pub fn is_cluster(&self, name: &str) -> bool {
        self.cluster.name == name
    }

    /// Upstream `frame.isCommand(name)`.
    pub fn is_command(&self, name: &str) -> bool {
        self.command.name == name
    }

    /// Upstream `ZclFrame.conditionsValid(parameter, payload, minimumRemainingBufferBytes)`.
    pub fn conditions_valid(
        _parameter: &Parameter,
        _payload: &Value,
        _minimum_remaining_buffer_bytes: Option<usize>,
    ) -> bool {
        todo!()
    }
}

pub fn zcl_frame() {
    todo!()
}
pub fn is_cluster() {
    todo!()
}
pub fn is_command() {
    todo!()
}
pub fn to_buffer() {
    todo!()
}
pub fn to_string() {
    todo!()
}

pub fn cluster_from_id(id: u16) -> Cluster {
    let name = match id {
        0x0000 => "genBasic",
        0x0004 => "genGroups",
        0x0005 => "genScenes",
        0x0006 => "genOnOff",
        0x000a => "genTime",
        0x0020 => "genPollCtrl",
        0x0500 => "ssIasZone",
        _ => "unknown",
    };
    Cluster { name, ID: id }
}

pub fn cluster_from_name(name: &'static str) -> Cluster {
    let id = match name {
        "genBasic" => 0x0000,
        "genGroups" => 0x0004,
        "genScenes" => 0x0005,
        "genOnOff" => 0x0006,
        "genTime" => 0x000a,
        "genPollCtrl" => 0x0020,
        "ssIasZone" => 0x0500,
        _ => 0,
    };
    Cluster { name, ID: id }
}

pub fn command_from_id(id: u8) -> Command {
    let name = match id {
        0x00 => "read",
        0x01 => "readRsp",
        0x02 => "write",
        0x03 => "writeUndivided",
        0x04 => "writeRsp",
        0x05 => "writeNoRsp",
        0x06 => "configureReporting",
        0x07 => "configureReportingRsp",
        0x08 => "readReportingConfiguration",
        0x09 => "readReportingConfigurationRsp",
        0x0a => "report",
        0x0b => "defaultRsp",
        _ => "unknown",
    };
    Command::new(name, id)
}

pub fn command_from_name(name: &'static str) -> Command {
    let id = match name {
        "read" => 0x00,
        "readRsp" => 0x01,
        "write" => 0x02,
        "writeUndivided" => 0x03,
        "writeRsp" => 0x04,
        "writeNoRsp" => 0x05,
        "configureReporting" => 0x06,
        "configureReportingRsp" => 0x07,
        "readReportingConfiguration" => 0x08,
        "readReportingConfigurationRsp" => 0x09,
        "report" => 0x0a,
        "defaultRsp" => 0x0b,
        "enrollReq" => 0x01,
        "enrollRsp" => 0x02,
        "checkin" => 0x00,
        "checkinRsp" => 0x01,
        "fastPollStop" => 0x03,
        "removeRsp" => 0x02,
        "add" => 0x00,
        "addRsp" => 0x00,
        _ => 0,
    };
    Command::new(name, id)
}
