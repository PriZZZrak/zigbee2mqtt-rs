// Original: .upstream/zigbee-herdsman/src/adapter/zigate/adapter/zigateAdapter.ts
// TODO: port public API from the original file.

// pub fn ZiGateAdapter() { todo!() }
// pub fn addInstallCode() { todo!() }
// pub fn backup() { todo!() }
// pub fn constructor() { todo!() }
// pub fn getCoordinatorIEEE() { todo!() }
// pub fn getCoordinatorVersion() { todo!() }
// pub fn getNetworkParameters() { todo!() }
// pub fn permitJoin() { todo!() }
// pub fn reset() { todo!() }
// pub fn restoreChannelInterPAN() { todo!() }
// pub fn sendZclFrameInterPANBroadcast() { todo!() }
// pub fn sendZclFrameInterPANToIeeeAddr() { todo!() }
// pub fn sendZclFrameToAll() { todo!() }
// pub fn sendZclFrameToEndpoint() { todo!() }
// pub fn sendZclFrameToGroup() { todo!() }
// pub fn sendZdo() { todo!() }
// pub fn setChannelInterPAN() { todo!() }
// pub fn start() { todo!() }
// pub fn stop() { todo!() }
// pub fn supportsBackup() { todo!() }
// pub fn waitFor() { todo!() }

pub fn zi_gate_adapter() {
    todo!()
}
pub fn add_install_code() {
    todo!()
}
pub fn backup() {
    todo!()
}
pub fn constructor() {
    todo!()
}
pub fn get_coordinator_ieee() {
    todo!()
}
pub fn get_coordinator_version() {
    todo!()
}
pub fn get_network_parameters() {
    todo!()
}
pub fn permit_join() {
    todo!()
}
pub fn reset() {
    todo!()
}
pub fn restore_channel_inter_pan() {
    todo!()
}
pub fn send_zcl_frame_inter_panbroadcast() {
    todo!()
}
pub fn send_zcl_frame_inter_panto_ieee_addr() {
    todo!()
}
pub fn send_zcl_frame_to_all() {
    todo!()
}
pub fn send_zcl_frame_to_endpoint() {
    todo!()
}
pub fn send_zcl_frame_to_group() {
    todo!()
}
pub fn send_zdo() {
    todo!()
}
pub fn set_channel_inter_pan() {
    todo!()
}
pub fn start() {
    todo!()
}
pub fn stop() {
    todo!()
}
pub fn supports_backup() {
    todo!()
}
pub fn wait_for() {
    todo!()
}

use std::cell::RefCell;

pub struct ZiGateAdapter {
    pub options: serde_json::Value,
    pub config: serde_json::Value,
    pub backup_path: String,
    pub startup: serde_json::Value,
    /// Records `driver.requestZdo(clusterId, payload)` calls.
    pub driver_calls: RefCell<Vec<(u16, Vec<u8>)>>,
}

impl ZiGateAdapter {
    pub fn new(
        options: serde_json::Value,
        config: serde_json::Value,
        backup_path: &str,
        startup: serde_json::Value,
    ) -> Self {
        Self {
            options,
            config,
            backup_path: backup_path.to_string(),
            startup,
            driver_calls: RefCell::new(Vec::new()),
        }
    }

    /// Upstream: `adapter.sendZdo(ieee, networkAddress, clusterId, payload, disableResponse)`.
    pub async fn send_zdo(
        &self,
        ieee_address: &str,
        network_address: u16,
        cluster_id: u16,
        payload: Vec<u8>,
        disable_response: bool,
    ) {
        todo!()
    }
}
