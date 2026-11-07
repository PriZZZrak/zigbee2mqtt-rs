// Original: .upstream/zigbee-herdsman/src/zspec/zcl/zclFrame.ts
// TODO: port frame creation/parsing/serialization.

use serde_json::Value;

use crate::zspec::zcl::definition::cluster::{Cluster, Command, Parameter};
use crate::zspec::zcl::zcl_header::Header;

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
        _frame_type: u8,
        _direction: u8,
        _disable_default_response: bool,
        _manufacturer_code: Option<u16>,
        _transaction_sequence_number: u8,
        _command: impl Into<CommandRef>,
        _cluster: impl Into<ClusterRef>,
        _payload: Value,
        _custom_clusters: Value,
        _reserved_bits: u8,
    ) -> Self {
        todo!()
    }

    /// Upstream `ZclFrame.fromBuffer(clusterId, header, buffer, customClusters)`.
    pub fn from_buffer(
        _cluster_id: u16,
        _header: Option<Header>,
        _buffer: &[u8],
        _custom_clusters: Value,
    ) -> Self {
        todo!()
    }

    /// Upstream `frame.toBuffer()`.
    pub fn to_buffer(&self) -> Vec<u8> {
        todo!()
    }

    /// Upstream `frame.toString()`.
    pub fn to_string(&self) -> String {
        todo!()
    }

    /// Upstream `frame.isCluster(name)`.
    pub fn is_cluster(&self, _name: &str) -> bool {
        todo!()
    }

    /// Upstream `frame.isCommand(name)`.
    pub fn is_command(&self, _name: &str) -> bool {
        todo!()
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
