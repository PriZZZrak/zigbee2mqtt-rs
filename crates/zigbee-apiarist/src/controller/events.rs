// Original: .upstream/zigbee-herdsman/src/controller/events.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use std::sync::Arc;

use serde_json::Value;

use crate::controller::model::device::Device;
use crate::controller::model::endpoint::Endpoint;
use crate::zspec::zcl::zcl_header::FrameControl;

/// Upstream `DeviceJoinedPayload`.
#[derive(Clone)]
pub struct DeviceJoinedPayload {
    pub device: Arc<Device>,
}

/// Upstream `DeviceInterviewPayload`.
#[derive(Clone)]
pub struct DeviceInterviewPayload {
    pub status: String,
    pub device: Arc<Device>,
}

/// Upstream `DeviceNetworkAddressChangedPayload`.
#[derive(Clone)]
pub struct DeviceNetworkAddressChangedPayload {
    pub device: Arc<Device>,
}

/// Upstream `DeviceAnnouncePayload`.
#[derive(Clone)]
pub struct DeviceAnnouncePayload {
    pub device: Arc<Device>,
}

/// Upstream `DeviceLeavePayload`.
#[derive(Clone)]
pub struct DeviceLeavePayload {
    /// Upstream camelCase: `ieeeAddr`.
    pub ieee_addr: String,
}

/// Upstream `PermitJoinChangedPayload`.
#[derive(Clone)]
pub struct PermitJoinChangedPayload {
    pub permitted: bool,
    pub time: Option<u64>,
}

/// Upstream `LastSeenChangedPayload`.
#[derive(Clone)]
pub struct LastSeenChangedPayload {
    pub device: Arc<Device>,
    pub reason: String,
}

/// Upstream `MessagePayload.meta`.
#[derive(Clone)]
pub struct MessageMeta {
    /// Upstream camelCase: `zclTransactionSequenceNumber`.
    pub zcl_transaction_sequence_number: Option<u8>,
    /// Upstream camelCase: `manufacturerCode`.
    pub manufacturer_code: Option<u16>,
    /// Upstream camelCase: `frameControl`.
    pub frame_control: Option<FrameControl>,
    /// Upstream camelCase: `rawData`.
    pub raw_data: Vec<u8>,
}

/// Upstream `MessagePayload`.
#[derive(Clone)]
pub struct MessagePayload {
    pub r#type: String,
    pub device: Arc<Device>,
    pub endpoint: Arc<Endpoint>,
    pub linkquality: u8,
    /// Upstream camelCase: `groupID`.
    pub group_id: u16,
    pub cluster: Value,
    pub data: Value,
    pub meta: MessageMeta,
}

/// Upstream `ControllerEventMap` discriminator used by the Rust event bus.
#[derive(Clone)]
pub enum ControllerEvent {
    Message(MessagePayload),
    AdapterDisconnected,
    DeviceJoined(DeviceJoinedPayload),
    DeviceInterview(DeviceInterviewPayload),
    DeviceAnnounce(DeviceAnnouncePayload),
    DeviceNetworkAddressChanged(DeviceNetworkAddressChangedPayload),
    DeviceLeave(DeviceLeavePayload),
    PermitJoinChanged(PermitJoinChangedPayload),
    LastSeenChanged(LastSeenChangedPayload),
}

/// Upstream `GreenPowerDeviceJoinedPayload`.
#[derive(Clone)]
pub struct GreenPowerDeviceJoinedPayload {
    /// Upstream camelCase: `sourceID`.
    pub source_id: u32,
    /// Upstream camelCase: `deviceID`.
    pub device_id: u32,
    /// Upstream camelCase: `networkAddress`.
    pub network_address: u16,
    /// Upstream camelCase: `securityKey`.
    pub security_key: Option<Vec<u8>>,
}
