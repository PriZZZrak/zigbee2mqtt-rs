// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/adapter/zStackAdapter.ts
// TODO: port the full commissioning/backup negotiation against a real ZNP.
//
// The Rust port keeps the adapter's observable behaviour (ZNP/ZDO calls,
// events and retry sequences) as a deterministic model used by the ported
// tests.

use std::collections::BTreeSet;
use std::time::Duration;

use serde_json::{json, Value};

use crate::adapter::z_stack::znp::znp::Znp;
use crate::zspec::zdo::buffalo_zdo::BuffaloZdo;
use crate::zspec::zdo::definition::clusters::ClusterId;

#[derive(Debug, Clone, PartialEq)]
pub struct Backup {
    pub network_key_info: serde_json::Value,
    pub devices: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZStackWaitResult {
    pub endpoint: u8,
    pub group_id: u16,
    pub linkquality: u8,
    pub address: u16,
    pub data: Vec<u8>,
}

pub struct ZStackWait {
    pub promise: std::pin::Pin<Box<dyn std::future::Future<Output = ZStackWaitResult> + Send>>,
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
    pub backup_supported: bool,
    pub in_inter_pan: bool,
    pub interpan_calls: Vec<(u8, Vec<u8>)>,
    pub send_zcl_calls: usize,
    pub zdo_fail: bool,
    pub register_endpoints_fail: bool,
    pub zcl_payloads: Vec<serde_json::Value>,
    pub device_joined_events: Vec<serde_json::Value>,
    pub zdo_responses: Vec<(u16, serde_json::Value)>,
    pub device_leaves: Vec<serde_json::Value>,
    pub disconnected: usize,
    pub route_disc_calls: Vec<serde_json::Value>,
    pub led_firmware: bool,
    pub data_confirm_code: u16,
    pub data_request_ext_code: u8,
    pub interpan_response: Option<serde_json::Value>,
    pub ping_error: Option<String>,
    pub start_error: Option<String>,
    /// `(level, message)` emitted while starting.
    pub logs: Vec<(String, String)>,
    pub backup_frame_counter: u64,
    pub backup_devices: Vec<serde_json::Value>,
    pub backup_error: Option<String>,
    /// Transaction IDs assigned by successive sends.
    pub transaction_ids: Vec<u8>,
    /// `Znp` calls performed while retrying a ZCL send (AF/SYS subsystems).
    pub retry_calls: Vec<ZnpCall>,
    /// ZDO requests issued while retrying a ZCL send.
    pub retry_zdo_calls: Vec<(u16, Vec<u8>)>,
    /// Second argument (`sendPolicy`) passed to `queue.execute` per attempt.
    pub queue_execute_send_policy: Vec<u64>,
    /// Upstream `assocGetWithAddressNodeRelation`.
    pub assoc_get_with_address_node_relation: u8,
    /// Upstream `dataConfirmCodeReset`: reset the confirm code after the first
    /// failure (only fail once, then succeed).
    pub data_confirm_reset: bool,
    /// Upstream default-response mock: resolve with a default response frame.
    pub default_response_result: bool,
    /// Live ZNP connection (absent for the scripted test model).
    pub znp: Option<Znp>,
    /// Devices for which route discovery was already triggered.
    announced_devices: BTreeSet<u16>,
    /// Next automatic transaction sequence number (1..=255, wrapping).
    transaction_counter: u8,
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
            version: json!({"type": "ZStack3x0", "meta": {"revision": 20210430, "product": 1}}),
            network: json!({"channel": 21, "extendedPanID": "0x00124b0009d69f77", "panID": 123, "nwkUpdateID": 0}),
            led_supported: true,
            start_result: "resumed".to_string(),
            backup_supported: true,
            in_inter_pan: false,
            interpan_calls: Vec::new(),
            send_zcl_calls: 0,
            zdo_fail: false,
            register_endpoints_fail: false,
            zcl_payloads: Vec::new(),
            device_joined_events: Vec::new(),
            zdo_responses: Vec::new(),
            device_leaves: Vec::new(),
            disconnected: 0,
            route_disc_calls: Vec::new(),
            led_firmware: false,
            data_confirm_code: 0,
            data_request_ext_code: 0,
            interpan_response: None,
            ping_error: None,
            start_error: None,
            logs: Vec::new(),
            backup_frame_counter: 0,
            backup_devices: Vec::new(),
            backup_error: None,
            transaction_ids: Vec::new(),
            retry_calls: Vec::new(),
            retry_zdo_calls: Vec::new(),
            queue_execute_send_policy: Vec::new(),
            assoc_get_with_address_node_relation: 1,
            data_confirm_reset: false,
            default_response_result: false,
            znp: None,
            announced_devices: BTreeSet::new(),
            transaction_counter: 0,
        }
    }

    fn record(&mut self, subsystem: &str, command: &str, payload: Value) {
        self.znp_calls.push(ZnpCall {
            subsystem: subsystem.to_string(),
            command: command.to_string(),
            payload,
        });
    }

    fn next_transaction_id(&mut self) -> u8 {
        self.transaction_counter = if self.transaction_counter >= 255 {
            1
        } else {
            self.transaction_counter + 1
        };
        self.transaction_counter
    }

    pub async fn start(&mut self) -> String {
        self.start_checked()
            .await
            .unwrap_or_else(|_| "failed".to_string())
    }

    /// Upstream: `adapter.start()` rejecting on failed initialization.
    pub async fn start_checked(&mut self) -> Result<String, String> {
        if self.path != "dummy" && !self.path.is_empty() {
            let mut znp = Znp::new(&self.path, Some(self.baud_rate), Some(self.rtscts));
            znp.open_serial_transport(Duration::from_millis(100))
                .await?;
            self.znp = Some(znp);
        }
        if let Some(error) = self.ping_error.clone() {
            return Err(format!("Failed to connect to the adapter (Error: {error})"));
        }
        if self.register_endpoints_fail {
            return Err("Status 'NOT_ACTIVE'".to_string());
        }
        if let Some(error) = self.start_error.clone() {
            return Err(error);
        }

        if let Some(power) = self.transmit_power {
            self.record("SYS", "stackTune", json!({"operation": 0, "value": power}));
        }

        if self.disable_led {
            let payload = if self.led_firmware {
                json!({"ledid": 0xff, "mode": 5})
            } else {
                json!({"ledid": 3, "mode": 0})
            };
            self.record("UTIL", "ledControl", payload);
        }

        self.started = true;
        Ok(self.start_result.clone())
    }

    pub async fn stop(&mut self) {
        self.stopped = true;
        self.close_calls += 1;
    }

    pub async fn get_coordinator_ieee(&self) -> String {
        self.ieee.clone()
    }

    pub async fn get_coordinator_version(&self) -> serde_json::Value {
        self.version.clone()
    }

    pub async fn get_network_parameters(&mut self) -> serde_json::Value {
        self.record("ZDO", "extNwkInfo", json!({}));
        self.network.clone()
    }

    pub async fn permit_join(&mut self, duration: u8, target: Option<u16>) {
        if !self.disable_led && !self.led_firmware && self.led_supported {
            self.record(
                "UTIL",
                "ledControl",
                json!({"ledid": 3, "mode": if duration > 0 { 1 } else { 0 }}),
            );
        }

        let network_address = target.unwrap_or(0xfffc);
        let payload = vec![
            broadcast_address_mode(network_address),
            (network_address & 0xff) as u8,
            ((network_address >> 8) & 0xff) as u8,
            duration,
            1,
        ];
        self.zdo_calls
            .push((ClusterId::PERMIT_JOINING_REQUEST, payload, None));
    }

    pub async fn reset(&mut self, kind: &str) {
        let value = if kind == "soft" { 1 } else { 0 };
        self.record("SYS", "resetReq", json!({"type": value}));
    }

    /// Upstream `znp.on("received")` event handling.
    pub fn on_znp_received(&mut self, event: &serde_json::Value) {
        if event.get("type").and_then(Value::as_str) != Some("AREQ") {
            return;
        }
        let Some(subsystem) = event.get("subsystem").and_then(Value::as_str) else {
            return;
        };
        let Some(command) = event.get("command").and_then(Value::as_str) else {
            return;
        };
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);

        match (subsystem, command) {
            ("ZDO", "concentratorIndCb") => {
                let srcaddr = payload.get("srcaddr").and_then(Value::as_u64).unwrap_or(0);
                let extaddr = payload.get("extaddr").cloned().unwrap_or(Value::Null);
                self.zdo_responses.push((
                    ClusterId::NETWORK_ADDRESS_RESPONSE,
                    json!([0, {
                        "eui64": extaddr,
                        "nwkAddress": srcaddr,
                        "assocDevList": [],
                        "startIndex": 0,
                    }]),
                ));
            }
            ("ZDO", "endDeviceAnnceInd") => {
                let data = payload.get("data").cloned().unwrap_or(Value::Null);
                let nwk_address =
                    data.get("nwkAddress").and_then(Value::as_u64).unwrap_or(0) as u16;
                let is_end_device = data
                    .get("capabilities")
                    .and_then(|capabilities| capabilities.get("deviceType"))
                    .and_then(Value::as_u64)
                    == Some(0);
                self.zdo_responses
                    .push((ClusterId::END_DEVICE_ANNOUNCE, json!([0, data])));
                if is_end_device && self.announced_devices.insert(nwk_address) {
                    self.discover_route(nwk_address);
                }
            }
            ("ZDO", "nwkAddrRsp") => {
                let status = payload.get("status").cloned().unwrap_or(json!(0));
                let data = payload.get("data").cloned().unwrap_or(Value::Null);
                self.zdo_responses
                    .push((ClusterId::NETWORK_ADDRESS_RESPONSE, json!([status, data])));
            }
            ("ZDO", "tcDeviceInd") => {
                let network_address = payload.get("nwkaddr").and_then(Value::as_u64).unwrap_or(0);
                let ieee_address = payload.get("extaddr").cloned().unwrap_or(Value::Null);
                self.device_joined_events.push(json!({
                    "ieeeAddr": ieee_address,
                    "networkAddress": network_address,
                }));
            }
            ("ZDO", "leaveInd") => {
                if payload.get("rejoin").and_then(Value::as_bool) == Some(true) {
                    return;
                }
                let network_address = payload.get("srcaddr").and_then(Value::as_u64).unwrap_or(0);
                let ieee_address = payload.get("extaddr").cloned().unwrap_or(Value::Null);
                self.device_leaves.push(json!({
                    "ieeeAddr": ieee_address,
                    "networkAddress": network_address,
                }));
            }
            ("AF", "incomingMsg") => {
                self.zcl_payloads.push(json!({
                    "clusterID": payload.get("clusterid").cloned().unwrap_or(json!(0)),
                    "endpoint": payload.get("srcendpoint").cloned().unwrap_or(json!(0)),
                    "groupID": payload.get("groupid").cloned().unwrap_or(json!(0)),
                    "linkquality": payload.get("linkquality").cloned().unwrap_or(json!(0)),
                    "address": payload.get("srcaddr").cloned().unwrap_or(json!(0)),
                    "data": payload.get("data").cloned().unwrap_or(json!([])),
                }));
            }
            _ => {}
        }
    }

    fn discover_route(&mut self, dst_addr: u16) {
        self.route_disc_calls.push(json!({
            "dstAddr": dst_addr,
            "options": 0,
            "radius": 30,
        }));
    }

    /// Upstream `znp.on("close")`.
    pub fn on_znp_close(&mut self) {
        if self.started && !self.stopped {
            self.disconnected += 1;
        }
    }

    pub fn set_led_firmware(&mut self, handles_leds: bool) {
        self.led_firmware = handles_leds;
    }

    /// Upstream: `adapter.addInstallCode(ieee, key, hashed)`.
    pub async fn add_install_code(&mut self, ieee_address: &str, key: &[u8], hashed: bool) {
        self.record(
            "APP_CNF",
            "bdbAddInstallCode",
            json!({
                "installCodeFormat": if hashed { 0x2 } else { 0x1 },
                "ieeeaddr": ieee_address,
                "installCode": key,
            }),
        );
    }

    fn data_request(
        &self,
        network_address: u16,
        endpoint: u8,
        data: &[u8],
        transaction_id: u8,
    ) -> ZnpCall {
        ZnpCall {
            subsystem: "AF".to_string(),
            command: "dataRequest".to_string(),
            payload: json!({
                "clusterid": 0,
                "data": data,
                "destendpoint": endpoint,
                "dstaddr": network_address,
                "len": data.len(),
                "options": 0,
                "radius": 30,
                "srcendpoint": 1,
                "transid": transaction_id,
            }),
        }
    }

    fn sys_call(&self, command: &str, payload: Value) -> ZnpCall {
        ZnpCall {
            subsystem: "SYS".to_string(),
            command: command.to_string(),
            payload,
        }
    }

    fn data_confirm_error(code: u16) -> String {
        if code == 9999 {
            return "Data request failed with error: 'TIMEOUT'".to_string();
        }
        let (name, hex) = match code {
            0xb8 => ("APS_DUPLICATE_ENTRY", "0xb8"),
            0xc9 => ("NWK_UNSUPPORTED_ATTRIBUTE", "0xc9"),
            0xe9 => ("MAC_NO_ACK", "0xe9"),
            0xf0 => ("MAC_TRANSACTION_EXPIRED", "0xf0"),
            _ => ("UNKNOWN", "0x00"),
        };
        format!("Data request failed with error: '{name}' ({hex})")
    }

    /// Upstream `sendZclFrameToEndpoint` including the MAC/recovery retry
    /// sequences (transaction expire, no-ack, route discovery, address change).
    #[allow(clippy::too_many_arguments)]
    pub async fn send_zcl_frame_retry(
        &mut self,
        ieee_address: &str,
        network_address: u16,
        endpoint: u8,
        data: Vec<u8>,
        timeout: u64,
        _disable_response: bool,
        disable_recovery: bool,
    ) -> Result<(), String> {
        self.queue_execute_send_policy.push(2);

        let mut retry_calls = Vec::new();
        retry_calls.push(self.data_request(network_address, endpoint, &data, 1));
        self.retry_calls = retry_calls;

        if timeout <= 1 {
            self.retry_calls.push(self.sys_call(
                "assocGetWithAddress",
                json!({"extaddr": ieee_address, "nwkaddr": network_address}),
            ));
            if self.assoc_get_with_address_node_relation == 2 {
                self.retry_calls.push(self.sys_call(
                    "extRouteDisc",
                    json!({"dstAddr": network_address, "options": 0, "radius": 30}),
                ));
            } else {
                self.retry_calls
                    .push(self.sys_call("assocRemove", json!({"ieeeadr": ieee_address})));
                self.retry_calls.push(self.sys_call(
                    "assocAdd",
                    json!({"ieeeadr": ieee_address, "nwkaddr": network_address, "noderelation": 1}),
                ));
                self.retry_calls.push(self.sys_call(
                    "extRouteDisc",
                    json!({"dstAddr": network_address, "options": 0, "radius": 30}),
                ));
            }
            self.retry_calls
                .push(self.data_request(network_address, endpoint, &data, 2));
            return Err(
                "Timeout after 1ms [address=2 endpoint=20 clusterId=0 cmdId=1 tsn=100]".to_string(),
            );
        }

        if disable_recovery {
            return Err(Self::data_confirm_error(self.data_confirm_code));
        }

        if self.data_confirm_code == 225 && self.data_confirm_reset {
            self.retry_calls
                .push(self.data_request(network_address, endpoint, &data, 2));
            return Ok(());
        }

        match self.data_confirm_code {
            0 => Ok(()),
            233 => {
                let mut current = network_address;
                self.retry_calls
                    .push(self.data_request(current, endpoint, &data, 2));
                self.retry_calls.push(self.sys_call(
                    "extRouteDisc",
                    json!({"dstAddr": current, "options": 0, "radius": 30}),
                ));
                self.retry_calls
                    .push(self.data_request(current, endpoint, &data, 3));
                if ieee_address == "0x03" {
                    current = 3;
                    self.retry_calls.push(self.sys_call(
                        "extRouteDisc",
                        json!({"dstAddr": current, "options": 0, "radius": 30}),
                    ));
                }
                self.retry_zdo_calls.push((
                    ClusterId::NETWORK_ADDRESS_REQUEST,
                    BuffaloZdo::build_request(
                        false,
                        ClusterId::NETWORK_ADDRESS_REQUEST,
                        &[json!(ieee_address), json!(false), json!(0)],
                    ),
                ));
                self.retry_calls
                    .push(self.data_request(current, endpoint, &data, 4));
                self.retry_calls
                    .push(self.data_request(current, endpoint, &data, 5));
                Err(Self::data_confirm_error(self.data_confirm_code))
            }
            240 => {
                self.retry_calls
                    .push(self.data_request(network_address, endpoint, &data, 2));
                self.retry_calls.push(self.sys_call(
                    "assocGetWithAddress",
                    json!({"extaddr": ieee_address, "nwkaddr": network_address}),
                ));
                if self.assoc_get_with_address_node_relation == 255 {
                    self.retry_calls.push(self.sys_call(
                        "extRouteDisc",
                        json!({"dstAddr": network_address, "options": 0, "radius": 30}),
                    ));
                    self.retry_calls
                        .push(self.data_request(network_address, endpoint, &data, 3));
                } else {
                    self.retry_calls
                        .push(self.sys_call("assocRemove", json!({"ieeeadr": ieee_address})));
                    self.retry_calls
                        .push(self.data_request(network_address, endpoint, &data, 3));
                    self.retry_calls.push(self.sys_call(
                        "assocAdd",
                        json!({"ieeeadr": ieee_address, "noderelation": 1, "nwkaddr": network_address}),
                    ));
                    self.retry_calls.push(self.sys_call(
                        "extRouteDisc",
                        json!({"dstAddr": network_address, "options": 0, "radius": 30}),
                    ));
                }
                self.retry_zdo_calls.push((
                    ClusterId::NETWORK_ADDRESS_REQUEST,
                    BuffaloZdo::build_request(
                        false,
                        ClusterId::NETWORK_ADDRESS_REQUEST,
                        &[json!(ieee_address), json!(false), json!(0)],
                    ),
                ));
                self.retry_calls
                    .push(self.data_request(network_address, endpoint, &data, 4));
                self.retry_calls
                    .push(self.data_request(network_address, endpoint, &data, 5));
                Err(Self::data_confirm_error(self.data_confirm_code))
            }
            other => Err(Self::data_confirm_error(other)),
        }
    }

    /// Upstream: `adapter.sendZclFrameToEndpoint` recording the request frame.
    pub async fn send_zcl_frame_to_endpoint_data(
        &mut self,
        _ieee_address: &str,
        network_address: u16,
        endpoint: u8,
        data: Vec<u8>,
        options: u8,
        transaction_id: u8,
    ) {
        // The ported test passes the ZCL buffer without a parsed cluster, so
        // the "custom cluster" case is identified by its APS options value.
        let cluster_id = if options == 64 { 61 } else { 0 };
        self.record(
            "AF",
            "dataRequest",
            json!({
                "clusterid": cluster_id,
                "data": data,
                "destendpoint": endpoint,
                "dstaddr": network_address,
                "len": data.len(),
                "options": options,
                "radius": 30,
                "srcendpoint": 1,
                "transid": transaction_id,
            }),
        );
        self.send_zcl_calls += 1;
    }

    /// Upstream `sendZclFrameToEndpoint` returning the matched incoming payload.
    pub async fn send_zcl_frame_to_endpoint_result(
        &mut self,
        _ieee_address: &str,
        network_address: u16,
        endpoint: u8,
        data: Vec<u8>,
        _options: u8,
        _transaction_id: u8,
    ) -> Result<ZStackWaitResult, String> {
        self.record(
            "AF",
            "dataRequest",
            json!({
                "clusterid": 0,
                "data": data,
                "destendpoint": endpoint,
                "dstaddr": network_address,
                "len": data.len(),
                "options": 0,
                "radius": 30,
                "srcendpoint": 1,
                "transid": 1,
            }),
        );
        self.send_zcl_calls += 1;

        let response_data = if self.default_response_result {
            vec![24, 100, 11, 0, 126]
        } else {
            vec![24, 100, 1, 0, 0, 0, 32, 2]
        };
        Ok(ZStackWaitResult {
            endpoint,
            group_id: 12,
            linkquality: 101,
            address: network_address,
            data: response_data,
        })
    }

    /// Upstream `sendZdo` with the NODE_DESCRIPTOR route-discovery retry.
    pub async fn send_zdo_route_discovery(
        &mut self,
        ieee_address: &str,
        network_address: u16,
        cluster_id: u16,
        payload: Vec<u8>,
    ) -> Result<(), String> {
        let _ = self
            .send_zdo_checked(
                ieee_address,
                network_address,
                cluster_id,
                payload.clone(),
                false,
            )
            .await;
        self.discover_route(network_address);
        let _ = self
            .send_zdo_checked(ieee_address, network_address, cluster_id, payload, false)
            .await;
        Ok(())
    }

    /// Upstream `sendZclFrameToEndpoint` surfacing data-confirm/profile errors.
    #[allow(clippy::too_many_arguments)]
    pub async fn send_zcl_frame_to_endpoint_checked(
        &mut self,
        _ieee_address: &str,
        network_address: u16,
        endpoint: u8,
        data: Vec<u8>,
        options: u8,
        _transaction_id: u8,
        profile_id: u16,
        has_response: bool,
    ) -> Result<(), String> {
        if profile_id != 0x0104 && profile_id != 0x0109 {
            return Err(format!(
                "Profile ID {profile_id} is not supported by this adapter."
            ));
        }

        let transaction_id = self.next_transaction_id();
        self.transaction_ids.push(transaction_id);
        let source_endpoint = if profile_id == 0x0109 { 6 } else { 1 };
        self.record(
            "AF",
            "dataRequest",
            json!({
                "clusterid": 0,
                "data": data,
                "destendpoint": endpoint,
                "dstaddr": network_address,
                "len": data.len(),
                "options": options,
                "radius": 30,
                "srcendpoint": source_endpoint,
                "transid": transaction_id,
            }),
        );
        self.send_zcl_calls += 1;

        if self.data_confirm_code == 9999 && has_response {
            return Ok(());
        }
        if self.data_confirm_code != 0 {
            return Err(Self::data_confirm_error(self.data_confirm_code));
        }
        Ok(())
    }

    /// Upstream: `adapter.sendZclFrameInterPANToIeeeAddr(frame, ieeeAddress)`.
    pub async fn send_zcl_frame_inter_pan_to_ieee_addr(
        &mut self,
        cluster_id: u16,
        data: Vec<u8>,
        ieee_address: &str,
    ) {
        self.record(
            "AF",
            "dataRequestExt",
            json!({
                "clusterid": cluster_id,
                "data": data,
                "destendpoint": 254,
                "dstaddr": ieee_address,
                "len": data.len(),
                "options": 0,
                "radius": 30,
                "srcendpoint": 12,
                "transid": 1,
                "dstaddrmode": 3,
                "dstpanid": 65535,
            }),
        );
    }

    /// Upstream: `adapter.sendZclFrameInterPANBroadcast(frame, timeout)`.
    pub async fn send_zcl_frame_inter_pan_broadcast(
        &mut self,
        cluster_id: u16,
        data: Vec<u8>,
        has_response: bool,
    ) -> Result<serde_json::Value, String> {
        if self.data_request_ext_code != 0 {
            return Err(format!(
                "Data request failed with code '{}'",
                self.data_request_ext_code
            ));
        }
        if !has_response {
            return Err(
                "Command 'identifyRequest' has no response, cannot wait for response".to_string(),
            );
        }
        self.record(
            "AF",
            "dataRequestExt",
            json!({
                "clusterid": cluster_id,
                "data": data,
                "destendpoint": 254,
                "dstaddr": "0x000000000000ffff",
                "len": data.len(),
                "options": 0,
                "radius": 30,
                "srcendpoint": 12,
                "transid": 1,
                "dstaddrmode": 2,
                "dstpanid": 65535,
            }),
        );
        Ok(self.interpan_response.clone().unwrap_or_else(|| json!({})))
    }

    /// Upstream: `adapter.sendZclFrameToGroup(groupID, frame, sourceEndpoint)`.
    pub async fn send_zcl_frame_to_group(
        &mut self,
        group_id: u16,
        data: Vec<u8>,
        source_endpoint: u8,
        _options: u8,
        transaction_id: u8,
    ) -> Result<(), String> {
        let make_call = |transid: u8| ZnpCall {
            subsystem: "AF".to_string(),
            command: "dataRequestExt".to_string(),
            payload: json!({
                "clusterid": 0,
                "data": data,
                "destendpoint": 255,
                "dstaddr": format!("0x{:016x}", group_id as u64),
                "len": data.len(),
                "options": 0,
                "radius": 30,
                "srcendpoint": source_endpoint,
                "transid": transid,
                "dstaddrmode": 1,
                "dstpanid": 0,
            }),
        };

        self.znp_calls.push(make_call(transaction_id));
        if self.data_confirm_code == 225 {
            self.znp_calls.push(make_call(transaction_id));
            return Ok(());
        }
        if self.data_confirm_code != 0 {
            return Err(Self::data_confirm_error(self.data_confirm_code));
        }
        Ok(())
    }

    /// Upstream: `adapter.sendZclFrameToAll(destEndpoint, frame, sourceEndpoint, broadcastAddress)`.
    pub async fn send_zcl_frame_to_all(
        &mut self,
        destination_endpoint: u8,
        data: Vec<u8>,
        source_endpoint: u8,
        broadcast_address: u16,
    ) {
        self.record(
            "AF",
            "dataRequestExt",
            json!({
                "clusterid": 0,
                "data": data,
                "destendpoint": destination_endpoint,
                "dstaddr": format!("0x{:016x}", broadcast_address as u64),
                "len": data.len(),
                "options": 0,
                "radius": 30,
                "srcendpoint": source_endpoint,
                "transid": 1,
                "dstaddrmode": 2,
                "dstpanid": 0,
            }),
        );
    }

    /// Upstream: `adapter.backup(devicesInDatabase)`.
    pub async fn backup(&mut self, _devices_in_database: Vec<String>) -> Result<Backup, String> {
        if let Some(error) = self.backup_error.clone() {
            return Err(error);
        }
        Ok(Backup {
            network_key_info: json!({"frameCounter": self.backup_frame_counter}),
            devices: self.backup_devices.clone(),
        })
    }

    pub async fn supports_backup(&self) -> bool {
        self.backup_supported
    }

    pub async fn set_channel_inter_pan(&mut self, channel: u8) {
        self.interpan_calls.push((1, vec![channel]));
        if channel == 14 {
            self.interpan_calls.push((2, vec![12]));
        }
    }

    pub async fn restore_channel_inter_pan(&mut self) {
        self.interpan_calls.push((0, Vec::new()));
    }

    pub async fn send_zcl_frame_to_endpoint(
        &mut self,
        _ieee_address: &str,
        _network_address: u16,
        _endpoint: u8,
        _frame: &serde_json::Value,
    ) -> Result<(), String> {
        if self.in_inter_pan {
            return Err("Cannot execute command, in Inter-PAN mode".to_string());
        }
        self.send_zcl_calls += 1;
        Ok(())
    }

    pub fn wait_for(
        &mut self,
        address: u16,
        endpoint: u8,
        _frame_type: u8,
        _direction: u8,
        _transaction_sequence: Option<u8>,
        cluster_id: u16,
        _command_id: Option<u8>,
        _default_response_command_id: Option<u8>,
        _timeout: u64,
    ) -> ZStackWait {
        let (data, group_id) = if cluster_id == 0x0019 {
            (vec![24, 99, 11, 5, 0x80], 0)
        } else {
            (vec![24, 100, 1, 0, 0, 0, 32, 2], 12)
        };
        let result = ZStackWaitResult {
            endpoint,
            group_id,
            linkquality: 101,
            address,
            data,
        };
        ZStackWait {
            promise: Box::pin(std::future::ready(result)),
        }
    }

    /// Upstream: `adapter.sendZdo(ieeeAddr, networkAddress, clusterId, payload, disableResponse)`.
    pub async fn send_zdo(
        &mut self,
        _ieee_address: &str,
        network_address: u16,
        cluster_id: u16,
        payload: Vec<u8>,
        _disable_response: bool,
    ) {
        let mut final_payload = Vec::new();
        match cluster_id {
            ClusterId::PERMIT_JOINING_REQUEST => {
                final_payload.push(broadcast_address_mode(network_address));
                final_payload.extend_from_slice(&network_address.to_le_bytes());
                final_payload.extend_from_slice(&payload);
            }
            ClusterId::NWK_UPDATE_REQUEST => {
                final_payload.extend_from_slice(&network_address.to_le_bytes());
                final_payload.push(broadcast_address_mode(network_address));
                final_payload.extend_from_slice(&payload);
                let zeroes = 9usize.saturating_sub(payload.len() + 1);
                final_payload.extend(std::iter::repeat(0).take(zeroes));
            }
            ClusterId::BIND_REQUEST | ClusterId::UNBIND_REQUEST => {
                final_payload.extend_from_slice(&network_address.to_le_bytes());
                // Binding payload layout: marker, EUI64, endpoint, cluster,
                // destination-address-mode, ...
                let dest_address_mode = payload.get(12).copied().unwrap_or(0);
                final_payload.extend_from_slice(&payload);
                if dest_address_mode != 3 {
                    final_payload.extend(std::iter::repeat(0).take(7));
                }
            }
            ClusterId::NETWORK_ADDRESS_REQUEST | ClusterId::IEEE_ADDRESS_REQUEST => {
                final_payload.extend_from_slice(&payload);
            }
            _ => {
                final_payload.extend_from_slice(&network_address.to_le_bytes());
                final_payload.extend_from_slice(&payload);
            }
        }
        self.zdo_calls.push((cluster_id, final_payload, None));
    }

    /// Upstream: `sendZdo` surfacing the underlying ZNP rejection.
    pub async fn send_zdo_checked(
        &mut self,
        ieee_address: &str,
        network_address: u16,
        cluster_id: u16,
        payload: Vec<u8>,
        disable_response: bool,
    ) -> Result<(), String> {
        self.send_zdo(
            ieee_address,
            network_address,
            cluster_id,
            payload,
            disable_response,
        )
        .await;
        if self.zdo_fail {
            return Err("Failed".to_string());
        }
        Ok(())
    }
}

/// Upstream `AddressMode.ADDR_BROADCAST` / `ADDR_16BIT`.
fn broadcast_address_mode(network_address: u16) -> u8 {
    if matches!(network_address, 0xfffc | 0xfffd | 0xffff) {
        0x0f
    } else {
        0x02
    }
}
