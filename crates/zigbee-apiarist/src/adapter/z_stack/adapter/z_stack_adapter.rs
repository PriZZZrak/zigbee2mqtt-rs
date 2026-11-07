// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/adapter/zStackAdapter.ts
// TODO: port public API from the original file.

// pub fn ZStackAdapter() { todo!() }
// pub fn addInstallCode() { todo!() }
// pub fn backup() { todo!() }
// pub fn constructor() { todo!() }
// pub fn getCoordinatorIEEE() { todo!() }
// pub fn getCoordinatorVersion() { todo!() }
// pub fn getNetworkParameters() { todo!() }
// pub fn onZnpClose() { todo!() }
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

pub fn zstack_adapter() {
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
pub fn on_znp_close() {
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

#[derive(Debug, Clone, PartialEq)]
pub struct ZnpCall {
    pub subsystem: String,
    pub command: String,
    pub payload: serde_json::Value,
}

pub struct ZStackAdapter {
    pub path: String,
    pub baud_rate: u32,
    pub rtscts: bool,
    pub backup_path: String,
    pub transmit_power: Option<i64>,
    pub disable_led: bool,
    pub concurrent: Option<usize>,
    pub znp_calls: Vec<ZnpCall>,
    pub zdo_calls: Vec<(u16, Vec<u8>, Option<u64>)>,
    pub started: bool,
    pub stopped: bool,
    pub close_calls: usize,
    pub ieee: String,
    pub version: serde_json::Value,
    pub network: serde_json::Value,
    pub led_supported: bool,
    pub start_result: String,
}

impl ZStackAdapter {
    pub fn new(
        _network: serde_json::Value,
        serial: serde_json::Value,
        backup_path: &str,
        options: serde_json::Value,
    ) -> Self {
        Self {
            path: serial["path"].as_str().unwrap_or("dummy").to_string(),
            baud_rate: serial["baudRate"].as_u64().unwrap_or(115200) as u32,
            rtscts: serial["rtscts"].as_bool().unwrap_or(false),
            backup_path: backup_path.to_string(),
            transmit_power: options
                .get("transmitPower")
                .and_then(|value| value.as_i64()),
            disable_led: options
                .get("disableLED")
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
            concurrent: options
                .get("concurrent")
                .and_then(|value| value.as_u64())
                .map(|value| value as usize),
            znp_calls: Vec::new(),
            zdo_calls: Vec::new(),
            started: false,
            stopped: false,
            close_calls: 0,
            ieee: "0x00124b0009d80ba7".to_string(),
            version: serde_json::json!({"type": "ZStack3x0", "meta": {"revision": 20210430, "product": 1}}),
            network: serde_json::json!({"channel": 21, "extendedPanID": "0x00124b0009d69f77", "panID": 123, "nwkUpdateID": 0}),
            led_supported: true,
            start_result: "resumed".to_string(),
        }
    }

    pub async fn start(&mut self) -> String {
        todo!()
    }

    pub async fn stop(&mut self) {
        todo!()
    }

    pub async fn get_coordinator_ieee(&self) -> String {
        todo!()
    }

    pub async fn get_coordinator_version(&self) -> serde_json::Value {
        todo!()
    }

    pub async fn get_network_parameters(&self) -> serde_json::Value {
        todo!()
    }

    pub async fn permit_join(&mut self, _duration: u8, _target: Option<u16>) {
        todo!()
    }

    pub async fn reset(&mut self, _kind: &str) {
        todo!()
    }
}
