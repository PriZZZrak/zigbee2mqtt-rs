//! Mock adapter and fixtures for the ported `controller.test.ts` suite.
//!
//! The upstream suite mocks `ZStackAdapter`; the Rust controller takes a
//! `ControllerRuntimeAdapter`, so this harness provides the same seams at the
//! trait boundary and records every adapter call.

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tokio::sync::mpsc;

use zigbee_apiarist::adapter::events::ZclPayload;
pub use zigbee_apiarist::controller::controller::StartResult;
use zigbee_apiarist::controller::controller::{
    Controller, ControllerAdapterEvent, ControllerCoordinatorVersion, ControllerFuture,
    ControllerNetworkParameters, ControllerOptions, ControllerRuntimeAdapter,
};
use zigbee_apiarist::controller::events::ControllerEvent;
use zigbee_apiarist::controller::green_power::{GreenPowerAdapter, GreenPowerNetworkParameters};
use zigbee_apiarist::controller::model::device::{Device, InterviewState};
use zigbee_apiarist::controller::model::entity::ControllerAdapter;
use zigbee_apiarist::controller::model::group::Group;
use zigbee_apiarist::controller::touchlink::TouchlinkAdapter;
use zigbee_apiarist::models::backup::Backup;
use zigbee_apiarist::models::network_options::NetworkOptions;
use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;

pub type ZdoHandler =
    Arc<dyn Fn(&str, u16, u16, &[u8], bool) -> Result<Value, String> + Send + Sync>;
pub type ZclHandler =
    Arc<dyn Fn(&str, u16, u8, &Frame) -> Result<Option<ZclPayload>, String> + Send + Sync>;

pub struct MockAdapter {
    sender: Mutex<Option<mpsc::UnboundedSender<ControllerAdapterEvent>>>,
    pub start_result: Mutex<StartResult>,
    pub coordinator_ieee: Mutex<String>,
    pub network_parameters: Mutex<ControllerNetworkParameters>,
    pub backup_payload: Mutex<Backup>,
    pub supports_backup: AtomicBool,
    pub zdo_handler: Mutex<Option<ZdoHandler>>,
    pub zcl_handler: Mutex<Option<ZclHandler>>,
    pub devices: Arc<Mutex<HashMap<u16, MockDevice>>>,
    /// Upstream `enroll170`: whether the mock IAS device acknowledges enrollment.
    pub ias_enroll: AtomicBool,
    /// Command name whose endpoint send should fail (upstream `enrollRspThrow`).
    pub fail_zcl_command: Mutex<Option<String>>,
    /// Upstream `configureReportStatus` (status returned by configReport).
    pub configure_report_status: AtomicU64,
    /// Upstream `configureReportDefaultRsp` (configReport answered by defaultRsp).
    pub configure_report_default_rsp: AtomicBool,
    /// Status returned by genGroups add/remove responses.
    pub group_status: AtomicU64,
    /// Makes `sendZclFrameToGroup` fail (upstream mock rejection).
    pub fail_group_send: AtomicBool,
    /// Payload returned for `readReportConfig`.
    pub read_report_config_payload: Mutex<Option<Value>>,
    /// Command name answered with no response (upstream mock returning undefined).
    pub no_response_command: Mutex<Option<String>>,
    pub zdo_calls: Mutex<Vec<(String, u16, u16, Vec<u8>, bool)>>,
    pub sent_endpoint: Mutex<Vec<Frame>>,
    pub sent_group: Mutex<Vec<Frame>>,
    pub sent_all: Mutex<Vec<Frame>>,
    pub interpan_to_ieee: Mutex<Vec<(Frame, String)>>,
    pub interpan_broadcast: Mutex<Vec<Frame>>,
    pub permit_joins: Mutex<Vec<(u8, Option<u16>)>>,
    pub resets: Mutex<Vec<String>>,
    pub install_codes: Mutex<Vec<(String, Vec<u8>, bool)>>,
    pub stopped: AtomicBool,
    pub started: AtomicU64,
    pub interpan_channels: Mutex<Vec<u8>>,
    pub interpan_broadcast_handler:
        Mutex<Option<Arc<dyn Fn(&Frame) -> Result<Option<ZclPayload>, String> + Send + Sync>>>,
}

impl MockAdapter {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            sender: Mutex::new(None),
            start_result: Mutex::new(StartResult::Resumed),
            coordinator_ieee: Mutex::new("0x0000012300000000".to_string()),
            network_parameters: Mutex::new(ControllerNetworkParameters {
                pan_id: 1,
                extended_pan_id: "0x64c5fd698daf0c00".to_string(),
                channel: 15,
                nwk_update_id: 0,
            }),
            backup_payload: Mutex::new(empty_backup()),
            supports_backup: AtomicBool::new(true),
            zdo_handler: Mutex::new(None),
            zcl_handler: Mutex::new(None),
            devices: Arc::new(Mutex::new(HashMap::new())),
            ias_enroll: AtomicBool::new(true),
            fail_zcl_command: Mutex::new(None),
            configure_report_status: AtomicU64::new(0),
            configure_report_default_rsp: AtomicBool::new(false),
            group_status: AtomicU64::new(0),
            fail_group_send: AtomicBool::new(false),
            read_report_config_payload: Mutex::new(None),
            no_response_command: Mutex::new(None),
            zdo_calls: Mutex::new(Vec::new()),
            sent_endpoint: Mutex::new(Vec::new()),
            sent_group: Mutex::new(Vec::new()),
            sent_all: Mutex::new(Vec::new()),
            interpan_to_ieee: Mutex::new(Vec::new()),
            interpan_broadcast: Mutex::new(Vec::new()),
            permit_joins: Mutex::new(Vec::new()),
            resets: Mutex::new(Vec::new()),
            install_codes: Mutex::new(Vec::new()),
            stopped: AtomicBool::new(false),
            started: AtomicU64::new(0),
            interpan_channels: Mutex::new(Vec::new()),
            interpan_broadcast_handler: Mutex::new(None),
        })
    }

    pub fn add_device(&self, network_address: u16, device: MockDevice) {
        self.devices.lock().unwrap().insert(network_address, device);
    }

    pub fn update_device<F>(&self, network_address: u16, update: F)
    where
        F: FnOnce(&mut MockDevice),
    {
        if let Some(device) = self.devices.lock().unwrap().get_mut(&network_address) {
            update(device);
        }
    }

    pub fn set_zdo<F>(&self, handler: F)
    where
        F: Fn(&str, u16, u16, &[u8], bool) -> Result<Value, String> + Send + Sync + 'static,
    {
        *self.zdo_handler.lock().unwrap() = Some(Arc::new(handler));
    }

    pub fn set_zcl<F>(&self, handler: F)
    where
        F: Fn(&str, u16, u8, &Frame) -> Result<Option<ZclPayload>, String> + Send + Sync + 'static,
    {
        *self.zcl_handler.lock().unwrap() = Some(Arc::new(handler));
    }

    pub fn set_interpan_broadcast_handler<F>(&self, handler: F)
    where
        F: Fn(&Frame) -> Result<Option<ZclPayload>, String> + Send + Sync + 'static,
    {
        *self.interpan_broadcast_handler.lock().unwrap() = Some(Arc::new(handler));
    }

    pub fn emit(&self, event: ControllerAdapterEvent) {
        if let Some(sender) = self.sender.lock().unwrap().as_ref() {
            let _ = sender.send(event);
        }
    }

    pub fn emit_zcl(&self, payload: ZclPayload) {
        self.emit(ControllerAdapterEvent::ZclPayload(payload));
    }
}

impl ControllerAdapter for MockAdapter {
    fn has_zdo_message_overhead(&self) -> bool {
        false
    }

    fn send_zdo(
        &self,
        ieee_address: &str,
        network_address: u16,
        cluster_id: u16,
        payload: &[u8],
        disable_response: bool,
    ) -> ControllerFuture<'_, Value> {
        self.zdo_calls.lock().unwrap().push((
            ieee_address.to_string(),
            network_address,
            cluster_id,
            payload.to_vec(),
            disable_response,
        ));
        let handler = self.zdo_handler.lock().unwrap().clone();
        let devices = self.devices.lock().unwrap().clone();
        if cluster_id
            == zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::NODE_DESCRIPTOR_REQUEST
            && devices
                .get(&network_address)
                .and_then(|device| device.key.as_deref())
                == Some("xiaomi")
            && devices
                .get(&network_address)
                .is_some_and(|device| device.node_descriptor.is_none())
        {
            // Upstream mock: a Xiaomi device answers the node descriptor request
            // with an unsolicited genBasic read response carrying its modelID.
            // The controller awaits the interview inside its event loop, so the
            // harness applies the same state directly instead of queueing it.
            if let Some(device) = Device::by_network_address(network_address, true) {
                device.set_model_id("lumi.occupancy".to_string());
                device.set_manufacturer_name(Some("LUMI".to_string()));
            }
        }
        let ieee = ieee_address.to_string();
        let payload = payload.to_vec();
        Box::pin(async move {
            if let Some(handler) = handler {
                return handler(
                    &ieee,
                    network_address,
                    cluster_id,
                    &payload,
                    disable_response,
                );
            }
            default_zdo(&devices, network_address, cluster_id, &payload)
        })
    }

    fn send_zcl_frame_to_endpoint(
        &self,
        ieee_address: &str,
        network_address: u16,
        endpoint: u8,
        frame: &Frame,
        _timeout: u64,
        _disable_response: bool,
        _disable_recovery: bool,
        _source_endpoint: Option<u8>,
        _profile_id: Option<u16>,
    ) -> ControllerFuture<'_, Option<ZclPayload>> {
        self.sent_endpoint.lock().unwrap().push(frame.clone());
        let handler = self.zcl_handler.lock().unwrap().clone();
        let devices = self.devices.clone();
        let state = MockZclState {
            devices: self.devices.clone(),
            ias_enroll: self.ias_enroll.load(Ordering::SeqCst),
            configure_report_status: self.configure_report_status.load(Ordering::SeqCst) as u8,
            configure_report_default_rsp: self.configure_report_default_rsp.load(Ordering::SeqCst),
            group_status: self.group_status.load(Ordering::SeqCst) as u8,
            read_report_config_payload: self.read_report_config_payload.lock().unwrap().clone(),
        };
        let fail_command = {
            let mut slot = self.fail_zcl_command.lock().unwrap();
            if slot.as_deref() == Some(frame.command.name.as_str()) {
                slot.take()
            } else {
                None
            }
        };
        let no_response = {
            let mut slot = self.no_response_command.lock().unwrap();
            if slot.as_deref() == Some(frame.command.name.as_str()) {
                slot.take()
            } else {
                None
            }
        };
        let ieee = ieee_address.to_string();
        let frame = frame.clone();
        Box::pin(async move {
            if let Some(handler) = handler {
                return handler(&ieee, network_address, endpoint, &frame);
            }
            if fail_command.as_deref() == Some(frame.command.name.as_str()) {
                return Err("timeout".to_string());
            }
            if no_response.as_deref() == Some(frame.command.name.as_str()) {
                return Ok(None);
            }
            default_zcl(&state, network_address, endpoint, &frame)
        })
    }

    fn send_zcl_frame_to_group(
        &self,
        _group_id: u16,
        frame: &Frame,
        _source_endpoint: Option<u8>,
        _profile_id: Option<u16>,
    ) -> ControllerFuture<'_, ()> {
        self.sent_group.lock().unwrap().push(frame.clone());
        let fail = self.fail_group_send.load(Ordering::SeqCst);
        Box::pin(async move {
            if fail {
                Err("timeout".to_string())
            } else {
                Ok(())
            }
        })
    }

    fn send_zcl_frame_to_all(
        &self,
        _endpoint: u8,
        frame: &Frame,
        _source_endpoint: u8,
        _destination: u16,
        _profile_id: Option<u16>,
    ) -> ControllerFuture<'_, ()> {
        self.sent_all.lock().unwrap().push(frame.clone());
        Box::pin(async { Ok(()) })
    }
}

impl GreenPowerAdapter for MockAdapter {
    fn get_coordinator_ieee(&self) -> ControllerFuture<'_, String> {
        let value = self.coordinator_ieee.lock().unwrap().clone();
        Box::pin(async move { Ok(value) })
    }

    fn get_network_parameters(&self) -> ControllerFuture<'_, GreenPowerNetworkParameters> {
        let channel = self.network_parameters.lock().unwrap().channel;
        Box::pin(async move { Ok(GreenPowerNetworkParameters { channel }) })
    }

    fn send_zcl_frame_to_all(
        &self,
        endpoint: u8,
        frame: &Frame,
        source_endpoint: u8,
        destination: u16,
        profile_id: Option<u16>,
    ) -> ControllerFuture<'_, ()> {
        ControllerAdapter::send_zcl_frame_to_all(
            self,
            endpoint,
            frame,
            source_endpoint,
            destination,
            profile_id,
        )
    }

    fn send_zcl_frame_to_endpoint(
        &self,
        ieee_address: &str,
        network_address: u16,
        endpoint: u8,
        frame: &Frame,
        timeout: u64,
        disable_response: bool,
        disable_recovery: bool,
        source_endpoint: Option<u8>,
        profile_id: Option<u16>,
    ) -> ControllerFuture<'_, Option<ZclPayload>> {
        ControllerAdapter::send_zcl_frame_to_endpoint(
            self,
            ieee_address,
            network_address,
            endpoint,
            frame,
            timeout,
            disable_response,
            disable_recovery,
            source_endpoint,
            profile_id,
        )
    }
}

impl TouchlinkAdapter for MockAdapter {
    fn set_channel_inter_pan(&self, channel: u8) -> ControllerFuture<'_, ()> {
        self.interpan_channels.lock().unwrap().push(channel);
        Box::pin(async { Ok(()) })
    }

    fn restore_channel_inter_pan(&self) -> ControllerFuture<'_, ()> {
        Box::pin(async { Ok(()) })
    }

    fn send_zcl_frame_inter_pan_broadcast(
        &self,
        frame: &Frame,
        _timeout: u64,
        _disable_response: bool,
    ) -> ControllerFuture<'_, Option<ZclPayload>> {
        self.interpan_broadcast.lock().unwrap().push(frame.clone());
        let result = self
            .interpan_broadcast_handler
            .lock()
            .unwrap()
            .as_ref()
            .map(|handler| handler(frame));
        Box::pin(async move { result.unwrap_or(Ok(None)) })
    }

    fn send_zcl_frame_inter_pan_to_ieee_addr(
        &self,
        frame: &Frame,
        ieee_address: &str,
    ) -> ControllerFuture<'_, ()> {
        self.interpan_to_ieee
            .lock()
            .unwrap()
            .push((frame.clone(), ieee_address.to_string()));
        Box::pin(async { Ok(()) })
    }
}

impl ControllerRuntimeAdapter for MockAdapter {
    fn start(&self) -> ControllerFuture<'_, StartResult> {
        self.started.fetch_add(1, Ordering::SeqCst);
        let result = *self.start_result.lock().unwrap();
        Box::pin(async move { Ok(result) })
    }

    fn stop(&self) -> ControllerFuture<'_, ()> {
        self.stopped.store(true, Ordering::SeqCst);
        Box::pin(async { Ok(()) })
    }

    fn subscribe(&self) -> mpsc::UnboundedReceiver<ControllerAdapterEvent> {
        let (sender, receiver) = mpsc::unbounded_channel();
        *self.sender.lock().unwrap() = Some(sender);
        receiver
    }

    fn permit_join(&self, time: u8, network_address: Option<u16>) -> ControllerFuture<'_, ()> {
        self.permit_joins
            .lock()
            .unwrap()
            .push((time, network_address));
        Box::pin(async { Ok(()) })
    }

    fn get_network_parameters_full(&self) -> ControllerFuture<'_, ControllerNetworkParameters> {
        let value = self.network_parameters.lock().unwrap().clone();
        Box::pin(async move { Ok(value) })
    }

    fn get_coordinator_version(&self) -> ControllerFuture<'_, ControllerCoordinatorVersion> {
        Box::pin(async {
            Ok(ControllerCoordinatorVersion {
                r#type: "zStack".to_string(),
                meta: json!({"version": 1}),
            })
        })
    }

    fn supports_backup(&self) -> ControllerFuture<'_, bool> {
        let value = self.supports_backup.load(Ordering::SeqCst);
        Box::pin(async move { Ok(value) })
    }

    fn backup(&self, _ieee_addresses: &[String]) -> ControllerFuture<'_, Backup> {
        let value = self.backup_payload.lock().unwrap().clone();
        Box::pin(async move { Ok(value) })
    }

    fn reset(&self, r#type: &str) -> ControllerFuture<'_, ()> {
        self.resets.lock().unwrap().push(r#type.to_string());
        Box::pin(async { Ok(()) })
    }

    fn add_install_code(
        &self,
        ieee_address: &str,
        key: &[u8],
        hashed: bool,
    ) -> ControllerFuture<'_, ()> {
        self.install_codes
            .lock()
            .unwrap()
            .push((ieee_address.to_string(), key.to_vec(), hashed));
        Box::pin(async { Ok(()) })
    }

    fn manufacturer_id(&self) -> Option<u16> {
        Some(0x0007)
    }
}

fn empty_backup() -> Backup {
    Backup {
        network_options: NetworkOptions {
            pan_id: 6755,
            extended_pan_id: vec![0xde, 0xad, 0xbe, 0xef, 0x01, 0x02, 0x03, 0x04],
            channel_list: vec![11],
            network_key: vec![
                0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xb1, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6,
                0xb7, 0xb8,
            ],
            network_key_distribute: false,
            has_default_extended_pan_id: false,
        },
        logical_channel: 11,
        network_key_info: zigbee_apiarist::models::backup::NetworkKeyInfo {
            sequence_number: 0,
            frame_counter: 10000,
        },
        coordinator_ieee_address: vec![1, 2, 3, 4, 5, 6, 7, 8],
        security_level: 5,
        network_update_id: 0,
        devices: Vec::new(),
        znp_version: Some(1),
        trust_center_link_key_seed: None,
        ezsp_version: None,
        hashed_tclk: None,
    }
}

pub fn default_network_options() -> NetworkOptions {
    NetworkOptions {
        pan_id: 0x1a63,
        extended_pan_id: vec![0xdd; 8],
        channel_list: vec![15],
        network_key: vec![1, 3, 5, 7, 9, 11, 13, 15, 0, 2, 4, 6, 8, 10, 12, 13],
        network_key_distribute: false,
        has_default_extended_pan_id: false,
    }
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn temp_path(name: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "controller_suite_{name}_{}_{}.db",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_file(&path);
    path.to_string_lossy().to_string()
}

pub struct Harness {
    pub controller: Arc<Controller>,
    pub adapter: Arc<MockAdapter>,
    pub events: Arc<Mutex<Vec<ControllerEvent>>>,
    pub database_path: String,
    pub backup_path: String,
}

impl Harness {
    pub fn new(adapter: Arc<MockAdapter>) -> Self {
        Self::with_options_and_network(adapter, default_network_options())
    }

    pub fn with_options_and_network(adapter: Arc<MockAdapter>, network: NetworkOptions) -> Self {
        Self::with_options_network_and_accept(adapter, network, true)
    }

    pub fn with_options_network_and_accept(
        adapter: Arc<MockAdapter>,
        network: NetworkOptions,
        accept: bool,
    ) -> Self {
        install_default_devices(&adapter);
        Device::reset_cache();
        Group::reset_cache();
        let database_path = temp_path("database");
        let backup_path = temp_path("backup");
        let options = ControllerOptions {
            network,
            transport: json!({}),
            database_path: database_path.clone(),
            database_backup_path: Some(format!("{database_path}.backup")),
            backup_path: Some(backup_path.clone()),
            adapter: json!({}),
            accept_joining_device_handler: Some(Arc::new(move |_| {
                Box::pin(async move { Ok(accept) })
            })),
        };
        let controller = Arc::new(Controller::new(options).expect("controller options"));
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = events.clone();
        controller.on(move |event| sink.lock().unwrap().push(event.clone()));
        Self {
            controller,
            adapter,
            events,
            database_path,
            backup_path,
        }
    }

    pub async fn start(&self) -> Result<StartResult, String> {
        self.controller.start(self.adapter.clone()).await
    }

    pub fn event_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    pub fn take_events(&self) -> Vec<ControllerEvent> {
        std::mem::take(&mut *self.events.lock().unwrap())
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.database_path);
        let _ = std::fs::remove_file(&self.backup_path);
        let _ = std::fs::remove_file(format!("{}.backup", self.database_path));
    }
}

static TEST_LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> = std::sync::OnceLock::new();

pub async fn test_lock() -> tokio::sync::MutexGuard<'static, ()> {
    TEST_LOCK
        .get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

pub fn create_device(
    ieee_addr: &str,
    network_address: u16,
    device_type: zigbee_apiarist::controller::database::DeviceType,
) -> Arc<Device> {
    Device::create(
        device_type,
        ieee_addr,
        network_address,
        None,
        Some("Test".to_string()),
        Some("Mains (single phase)".to_string()),
        Some("TEST".to_string()),
        InterviewState::Successful,
        None,
    )
    .expect("device create")
}

#[derive(Clone, Default)]
pub struct MockDevice {
    pub node_descriptor: Option<Value>,
    pub active_endpoints: Option<Value>,
    pub simple_descriptor: HashMap<u8, Value>,
    pub lqi_table: Option<Value>,
    pub routing_table: Option<Value>,
    pub binding_table: Option<Value>,
    pub attributes: HashMap<u8, HashMap<String, Value>>,
    pub key: Option<String>,
}

fn default_zdo(
    devices: &HashMap<u16, MockDevice>,
    network_address: u16,
    cluster_id: u16,
    payload: &[u8],
) -> Result<Value, String> {
    if matches!(network_address, 0xfffb..=0xffff) {
        return Ok(json!([0, Value::Null]));
    }
    let Some(device) = devices.get(&network_address) else {
        return Err(format!("Mock device {network_address} not found"));
    };
    match cluster_id {
        zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::NODE_DESCRIPTOR_REQUEST => {
            device
                .node_descriptor
                .clone()
                .ok_or_else(|| "NODE_DESCRIPTOR_REQUEST timeout".to_string())
        }
        zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::ACTIVE_ENDPOINTS_REQUEST => {
            device
                .active_endpoints
                .clone()
                .ok_or_else(|| "ACTIVE_ENDPOINTS_REQUEST timeout".to_string())
        }
        zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::SIMPLE_DESCRIPTOR_REQUEST => {
            let endpoint = payload.get(2).copied().unwrap_or(0);
            device
                .simple_descriptor
                .get(&endpoint)
                .cloned()
                .ok_or_else(|| format!("SIMPLE_DESCRIPTOR_REQUEST({endpoint}) timeout"))
        }
        zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::LQI_TABLE_REQUEST => device
            .lqi_table
            .clone()
            .ok_or_else(|| "LQI_TABLE_REQUEST timeout".to_string()),
        zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::ROUTING_TABLE_REQUEST => {
            device
                .routing_table
                .clone()
                .ok_or_else(|| "ROUTING_TABLE_REQUEST timeout".to_string())
        }
        zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::BINDING_TABLE_REQUEST => {
            device
                .binding_table
                .clone()
                .ok_or_else(|| "BINDING_TABLE_REQUEST timeout".to_string())
        }
        _ => Ok(json!([0, Value::Null])),
    }
}

#[derive(Clone)]
pub struct MockZclState {
    pub devices: Arc<Mutex<HashMap<u16, MockDevice>>>,
    pub ias_enroll: bool,
    pub configure_report_status: u8,
    pub configure_report_default_rsp: bool,
    pub group_status: u8,
    pub read_report_config_payload: Option<Value>,
}

fn default_zcl(
    state: &MockZclState,
    network_address: u16,
    endpoint: u8,
    frame: &Frame,
) -> Result<Option<ZclPayload>, String> {
    eprintln!(
        "MOCK default_zcl cmd={} cluster={}",
        frame.command.name, frame.cluster.ID
    );
    let devices = &state.devices;
    let ias_enroll = state.ias_enroll;
    let is_global = frame.header.frame_control.frame_type
        == zigbee_apiarist::zspec::zcl::zcl_header::FrameType::GLOBAL;
    if is_global && frame.command.name == "read" {
        let Some(mut device) = devices.lock().unwrap().get(&network_address).cloned() else {
            return Err(format!("Mock device {network_address} not found"));
        };
        let attributes = match device.attributes.get(&endpoint).cloned() {
            Some(attributes) => attributes,
            None => {
                // Upstream mock throws when the endpoint has no attribute map,
                // which the adapter surfaces as a failed request.
                return Err("timeout".to_string());
            }
        };
        let mut payload = Vec::new();
        for item in frame.payload.as_array().cloned().unwrap_or_default() {
            let attr_id = item["attrId"].as_u64().unwrap_or(0) as u16;
            if let Some(attribute) =
                zigbee_apiarist::zspec::zcl::utils::Utils::get_cluster_attribute(
                    &frame.cluster,
                    zigbee_apiarist::zspec::zcl::utils::AttributeKey::Id(u32::from(attr_id)),
                    frame.header.manufacturer_code,
                )
            {
                // Attributes are stored by contract name; fall back to the
                // definition name and then the numeric ID.
                let value = attributes
                    .get(&attribute.name)
                    .cloned()
                    .or_else(|| attributes.get(&attr_id.to_string()).cloned());
                if let Some(value) = value {
                    payload.push(json!({
                        "attrId": attr_id,
                        "status": 0,
                        "dataType": attribute.r#type as u8,
                        "attrData": value,
                    }));
                }
            }
        }
        let response = Frame::create(
            zigbee_apiarist::zspec::zcl::zcl_header::FrameType::GLOBAL as u8,
            zigbee_apiarist::zspec::zcl::zcl_header::Direction::SERVER_TO_CLIENT as u8,
            true,
            None,
            10,
            "readRsp",
            frame.cluster.ID as u16,
            Value::Array(payload),
            json!({}),
            0,
        );
        return Ok(Some(ZclPayload {
            cluster_id: frame.cluster.ID as u16,
            data: response.to_buffer(),
            header: Some(response.header),
            address: json!(network_address),
            endpoint,
            linkquality: 50,
            group_id: 1,
            was_broadcast: false,
            destination_endpoint: 1,
        }));
    }
    if frame.cluster.ID == 0x0500 && is_global && frame.command.name == "write" {
        if ias_enroll {
            if let Some(device) = devices.lock().unwrap().get_mut(&network_address) {
                device
                    .attributes
                    .entry(endpoint)
                    .or_default()
                    .insert("zoneState".to_string(), json!(1));
            }
            let enroll = Frame::create(
                zigbee_apiarist::zspec::zcl::zcl_header::FrameType::SPECIFIC as u8,
                zigbee_apiarist::zspec::zcl::zcl_header::Direction::SERVER_TO_CLIENT as u8,
                false,
                None,
                1,
                "enrollReq",
                0x0500,
                json!({"zonetype": 0, "manucode": 1}),
                json!({}),
                0,
            );
            return Ok(Some(ZclPayload {
                cluster_id: 0x0500,
                data: enroll.to_buffer(),
                header: Some(enroll.header),
                address: json!(network_address),
                endpoint,
                linkquality: 50,
                group_id: 0,
                was_broadcast: false,
                destination_endpoint: 1,
            }));
        }
        return Ok(None);
    }
    if !is_global
        && (frame.command.name == "add" || frame.command.name == "remove")
        && frame.cluster.ID == 0x0004
    {
        let response_name = if frame.command.name == "add" {
            "addRsp"
        } else {
            "removeRsp"
        };
        let response = Frame::create(
            zigbee_apiarist::zspec::zcl::zcl_header::FrameType::SPECIFIC as u8,
            zigbee_apiarist::zspec::zcl::zcl_header::Direction::SERVER_TO_CLIENT as u8,
            true,
            None,
            10,
            response_name,
            0x0004,
            json!({"status": state.group_status, "groupid": 1}),
            json!({}),
            0,
        );
        return Ok(Some(ZclPayload {
            cluster_id: 0x0004,
            data: response.to_buffer(),
            header: Some(response.header),
            address: json!(network_address),
            endpoint,
            linkquality: 50,
            group_id: 0,
            was_broadcast: false,
            destination_endpoint: 1,
        }));
    }
    if is_global && frame.command.name == "configReport" {
        if state.configure_report_default_rsp {
            let response = Frame::create(
                0,
                1,
                true,
                None,
                10,
                "defaultRsp",
                0,
                json!({"cmdId": 6, "statusCode": state.configure_report_status}),
                json!({}),
                0,
            );
            return Ok(Some(ZclPayload {
                cluster_id: 0,
                data: response.to_buffer(),
                header: Some(response.header),
                address: json!(network_address),
                endpoint,
                linkquality: 50,
                group_id: 0,
                was_broadcast: false,
                destination_endpoint: 1,
            }));
        }
        let items = frame
            .payload
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|item| {
                json!({
                    "status": state.configure_report_status,
                    "direction": item["direction"],
                    "attrId": item["attrId"],
                    "dataType": item["dataType"],
                })
            })
            .collect::<Vec<_>>();
        let response = Frame::create(
            0,
            1,
            true,
            None,
            10,
            "configReportRsp",
            frame.cluster.ID as u16,
            Value::Array(items),
            json!({}),
            0,
        );
        return Ok(Some(ZclPayload {
            cluster_id: frame.cluster.ID as u16,
            data: response.to_buffer(),
            header: Some(response.header),
            address: json!(network_address),
            endpoint,
            linkquality: 50,
            group_id: 0,
            was_broadcast: false,
            destination_endpoint: 1,
        }));
    }
    if is_global && frame.command.name == "readReportConfig" {
        let items = match &state.read_report_config_payload {
            Some(payload) => payload.clone(),
            None => Value::Array(
                frame
                    .payload
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|item| {
                        json!({
                            "status": 0,
                            "direction": item["direction"],
                            "attrId": item["attrId"],
                            "dataType": 32,
                            "minRepIntval": 60,
                            "maxRepIntval": 3600,
                            "repChange": 10,
                        })
                    })
                    .collect(),
            ),
        };
        let response = Frame::create(
            0,
            1,
            true,
            None,
            10,
            "readReportConfigRsp",
            frame.cluster.ID as u16,
            items,
            json!({}),
            0,
        );
        return Ok(Some(ZclPayload {
            cluster_id: frame.cluster.ID as u16,
            data: response.to_buffer(),
            header: Some(response.header),
            address: json!(network_address),
            endpoint,
            linkquality: 50,
            group_id: 0,
            was_broadcast: false,
            destination_endpoint: 1,
        }));
    }
    if is_global && frame.command.name == "write" {
        let payload = frame
            .payload
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|item| json!({"attrId": item["attrId"], "status": 0}))
            .collect::<Vec<_>>();
        let response = Frame::create(
            0,
            1,
            true,
            None,
            10,
            "writeRsp",
            0,
            Value::Array(payload),
            json!({}),
            0,
        );
        return Ok(Some(ZclPayload {
            cluster_id: response.cluster.ID as u16,
            data: response.to_buffer(),
            header: Some(response.header),
            address: json!(network_address),
            endpoint,
            linkquality: 50,
            group_id: 1,
            was_broadcast: false,
            destination_endpoint: 1,
        }));
    }
    Ok(None)
}

pub fn node_descriptor(network_address: u16, logical_type: u8, manufacturer_code: u16) -> Value {
    json!([
        0,
        {
            "nwkAddress": network_address,
            "logicalType": logical_type,
            "manufacturerCode": manufacturer_code,
            "apsFlags": 0,
            "frequencyBand": 3,
            "capabilities": {
                "alternatePANCoordinator": 0,
                "deviceType": (0x8eu8 & 0x02) >> 1,
                "powerSource": (0x8eu8 & 0x04) >> 2,
                "rxOnWhenIdle": (0x8eu8 & 0x08) >> 3,
                "reserved1": (0x8eu8 & 0x10) >> 4,
                "reserved2": (0x8eu8 & 0x20) >> 5,
                "securityCapability": (0x8eu8 & 0x40) >> 6,
                "allocateAddress": (0x8eu8 & 0x80) >> 7
            },
            "maxBufSize": 0xaa,
            "maxIncTxSize": 0xac,
            "serverMask": {
                "primaryTrustCenter": 0,
                "backupTrustCenter": 1,
                "deprecated1": 0,
                "deprecated2": 0,
                "deprecated3": 0,
                "deprecated4": 0,
                "networkManager": 0,
                "reserved1": 0,
                "reserved2": 0,
                "stackComplianceRevision": 1
            },
            "maxOutTxSize": 0xdc,
            "deprecated1": 0,
            "fragmentationSupported": Value::Null,
            "tlvs": []
        }
    ])
}

pub fn simple_descriptor(
    network_address: u16,
    endpoint: u8,
    profile_id: u16,
    device_id: u16,
    in_clusters: Vec<u16>,
    out_clusters: Vec<u16>,
) -> Value {
    json!([
        0,
        {
            "nwkAddress": network_address,
            "length": 14,
            "endpoint": endpoint,
            "profileId": profile_id,
            "deviceId": device_id,
            "deviceVersion": 1,
            "inClusterList": in_clusters,
            "outClusterList": out_clusters
        }
    ])
}

pub fn install_default_devices(adapter: &MockAdapter) {
    adapter.add_device(
        0,
        MockDevice {
            node_descriptor: Some(node_descriptor(0, 0, 0x0007)),
            active_endpoints: Some(json!([0, {"nwkAddress": 0, "endpointList": [1, 2]}])),
            simple_descriptor: HashMap::from([
                (1, simple_descriptor(0, 1, 2, 3, vec![10], vec![11])),
                (2, simple_descriptor(0, 2, 3, 5, vec![1], vec![0])),
            ]),
            ..MockDevice::default()
        },
    );

    adapter.add_device(
        140,
        MockDevice {
            node_descriptor: None,
            lqi_table: Some(json!([0, {
                "neighborTableEntries": 2,
                "startIndex": 0,
                "entryList": [
                    {"extendedPanId": [1,2,3,4,5,6,7,8], "eui64": "0x160", "nwkAddress": 160, "deviceType": 1, "rxOnWhenIdle": 0, "relationship": 2, "reserved1": 0, "permitJoining": 0, "reserved2": 0, "depth": 5, "lqi": 20},
                    {"extendedPanId": [1,2,3,4,5,6,7,8], "eui64": "0x170", "nwkAddress": 170, "deviceType": 1, "rxOnWhenIdle": 0, "relationship": 4, "reserved1": 0, "permitJoining": 0, "reserved2": 0, "depth": 8, "lqi": 21}
                ]
            }])),
            routing_table: Some(json!([0, {
                "routingTableEntries": 2,
                "startIndex": 0,
                "entryList": [
                    {"destinationAddress": 120, "status": "ACTIVE", "memoryConstrained": 0, "manyToOne": 0, "routeRecordRequired": 0, "reserved1": 0, "nextHopAddress": 1},
                    {"destinationAddress": 130, "status": "DISCOVERY_FAILED", "memoryConstrained": 0, "manyToOne": 0, "routeRecordRequired": 0, "reserved1": 0, "nextHopAddress": 2}
                ]
            }])),
            binding_table: Some(json!([0, {
                "bindingTableEntries": 2,
                "startIndex": 0,
                "entryList": [
                    {"sourceEui64": "0xf1f2f3f5f6f7f8", "sourceEndpoint": 1, "clusterId": 0, "destAddrMode": 3, "dest": "0xa1a2a3a4a5a6a7a8", "destEndpoint": 2},
                    {"sourceEui64": "0xe1e2e3e5e6e7e8", "sourceEndpoint": 3, "clusterId": 9, "destAddrMode": 1, "dest": 4660}
                ]
            }])),
            ..MockDevice::default()
        },
    );
    adapter.add_device(
        150,
        MockDevice {
            node_descriptor: None,
            key: Some("xiaomi".to_string()),
            ..MockDevice::default()
        },
    );
    adapter.add_device(
        151,
        MockDevice {
            node_descriptor: Some(node_descriptor(151, 2, 1219)),
            active_endpoints: None,
            key: Some("xiaomi".to_string()),
            ..MockDevice::default()
        },
    );
    adapter.add_device(
        170,
        MockDevice {
            node_descriptor: Some(node_descriptor(170, 2, 4619)),
            active_endpoints: Some(json!([0, {"nwkAddress": 170, "endpointList": [1]}])),
            simple_descriptor: HashMap::from([(
                1,
                simple_descriptor(170, 1, 99, 5, vec![0, 1280], vec![2]),
            )]),
            attributes: HashMap::from([(
                1,
                HashMap::from([
                    ("zoneState".to_string(), json!(0)),
                    ("iasCieAddr".to_string(), json!("0x0000012300000000")),
                    ("modelId".to_string(), json!("myIasDevice")),
                    ("manufacturerName".to_string(), json!("KoenAndCoSecurity")),
                    ("zclVersion".to_string(), json!(1)),
                    ("appVersion".to_string(), json!(2)),
                    ("hwVersion".to_string(), json!(3)),
                    ("dateCode".to_string(), json!("201901")),
                    ("swBuildId".to_string(), json!("1.01")),
                    ("powerSource".to_string(), json!(1)),
                    ("stackVersion".to_string(), json!(101)),
                ]),
            )]),
            ..MockDevice::default()
        },
    );
    adapter.add_device(
        171,
        MockDevice {
            node_descriptor: Some(node_descriptor(171, 2, 1212)),
            active_endpoints: Some(
                json!([0, {"nwkAddress": 171, "endpointList": [1, 2, 3, 4, 5, 6]}]),
            ),
            simple_descriptor: HashMap::from([(
                1,
                simple_descriptor(171, 1, 99, 5, vec![0, 1, 2], vec![2]),
            )]),
            attributes: HashMap::from([(
                1,
                HashMap::from([
                    ("modelId".to_string(), json!("lumi.remote.b286opcn01")),
                    ("manufacturerName".to_string(), json!("Xioami")),
                    ("zclVersion".to_string(), json!(1)),
                    ("appVersion".to_string(), json!(2)),
                    ("hwVersion".to_string(), json!(3)),
                    ("dateCode".to_string(), json!("201901")),
                    ("swBuildId".to_string(), json!("1.01")),
                    ("powerSource".to_string(), json!(1)),
                    ("stackVersion".to_string(), json!(101)),
                ]),
            )]),
            ..MockDevice::default()
        },
    );
    adapter.add_device(
        172,
        MockDevice {
            node_descriptor: Some(node_descriptor(172, 1, 1212)),
            active_endpoints: Some(json!([0, {"nwkAddress": 172, "endpointList": [12, 11, 13]}])),
            simple_descriptor: HashMap::from([
                (
                    11,
                    simple_descriptor(
                        172,
                        11,
                        99,
                        0x0210,
                        vec![0, 3, 4, 5, 6, 8, 768, 62301],
                        vec![2],
                    ),
                ),
                (
                    12,
                    simple_descriptor(172, 12, 99, 0xe15e, vec![0, 3, 4, 5, 6, 8, 768], vec![2]),
                ),
                (
                    13,
                    simple_descriptor(172, 13, 99, 0x0100, vec![0, 3, 4, 5, 6, 8, 768], vec![2]),
                ),
            ]),
            attributes: HashMap::from([(
                12,
                HashMap::from([
                    ("modelId".to_string(), json!("GL-C-008")),
                    ("manufacturerName".to_string(), json!("Gledopto")),
                    ("zclVersion".to_string(), json!(1)),
                    ("appVersion".to_string(), json!(2)),
                    ("hwVersion".to_string(), json!(3)),
                    ("dateCode".to_string(), json!("201901")),
                    ("swBuildId".to_string(), json!("1.01")),
                    ("powerSource".to_string(), json!(1)),
                    ("stackVersion".to_string(), json!(101)),
                ]),
            )]),
            ..MockDevice::default()
        },
    );
    adapter.add_device(
        173,
        MockDevice {
            node_descriptor: Some(node_descriptor(173, 2, 0)),
            active_endpoints: Some(json!([0, {"nwkAddress": 173, "endpointList": [1]}])),
            simple_descriptor: HashMap::new(),
            attributes: HashMap::new(),
            ..MockDevice::default()
        },
    );
    adapter.add_device(
        174,
        MockDevice {
            node_descriptor: Some(node_descriptor(174, 2, 1213)),
            active_endpoints: Some(json!([0, {"nwkAddress": 174, "endpointList": [1]}])),
            simple_descriptor: HashMap::from([(
                1,
                simple_descriptor(174, 1, 99, 5, vec![0, 32], vec![2]),
            )]),
            attributes: HashMap::from([(
                1,
                HashMap::from([("checkinInterval".to_string(), json!(50))]),
            )]),
            ..MockDevice::default()
        },
    );
    adapter.add_device(
        180,
        MockDevice {
            node_descriptor: Some(node_descriptor(180, 1, 0xabcd)),
            active_endpoints: Some(json!([0, {"nwkAddress": 180, "endpointList": [1, 197]}])),
            simple_descriptor: HashMap::from([(
                1,
                simple_descriptor(180, 1, 260, 257, vec![0, 6, 8], vec![]),
            )]),
            attributes: HashMap::new(),
            ..MockDevice::default()
        },
    );
    adapter.add_device(
        161,
        MockDevice {
            node_descriptor: Some(node_descriptor(161, 1, 1213)),
            active_endpoints: Some(json!([0, {"nwkAddress": 161, "endpointList": [4, 1]}])),
            simple_descriptor: HashMap::from([
                (1, simple_descriptor(161, 1, 99, 5, vec![0, 1], vec![2])),
                (4, simple_descriptor(161, 4, 99, 5, vec![1], vec![2])),
            ]),
            attributes: HashMap::from([
                (
                    1,
                    HashMap::from([
                        ("modelId".to_string(), json!("myDevice9123")),
                        ("manufacturerName".to_string(), json!("Boef")),
                        ("zclVersion".to_string(), json!(1)),
                        ("appVersion".to_string(), json!(2)),
                        ("hwVersion".to_string(), json!(3)),
                        ("dateCode".to_string(), json!("201901")),
                        ("swBuildId".to_string(), json!("1.01")),
                        ("powerSource".to_string(), json!(1)),
                        ("stackVersion".to_string(), json!(101)),
                    ]),
                ),
                (4, HashMap::new()),
            ]),
            ..MockDevice::default()
        },
    );
    adapter.add_device(
        162,
        MockDevice {
            node_descriptor: Some(node_descriptor(162, 1, 1213)),
            active_endpoints: Some(json!([0, {"nwkAddress": 162, "endpointList": [2, 1]}])),
            simple_descriptor: HashMap::from([
                (1, simple_descriptor(162, 1, 99, 5, vec![1], vec![2])),
                (2, simple_descriptor(162, 2, 99, 5, vec![0, 1], vec![2])),
            ]),
            attributes: HashMap::from([
                (
                    2,
                    HashMap::from([
                        ("modelId".to_string(), json!("myDevice9124")),
                        ("manufacturerName".to_string(), json!("Boef")),
                        ("zclVersion".to_string(), json!(1)),
                        ("appVersion".to_string(), json!(2)),
                        ("hwVersion".to_string(), json!(3)),
                        ("dateCode".to_string(), json!("201901")),
                        ("swBuildId".to_string(), json!("1.01")),
                        ("powerSource".to_string(), json!(1)),
                        ("stackVersion".to_string(), json!(101)),
                    ]),
                ),
                (1, HashMap::new()),
            ]),
            ..MockDevice::default()
        },
    );
    adapter.add_device(
        129,
        MockDevice {
            node_descriptor: Some(node_descriptor(129, 1, 1212)),
            active_endpoints: Some(json!([0, {"nwkAddress": 129, "endpointList": [1]}])),
            simple_descriptor: HashMap::from([(
                1,
                simple_descriptor(129, 1, 99, 5, vec![0, 1], vec![2]),
            )]),
            attributes: HashMap::from([(
                1,
                HashMap::from([
                    ("modelId".to_string(), json!("myModelID")),
                    ("manufacturerName".to_string(), json!("KoenAndCo")),
                    ("zclVersion".to_string(), json!(1)),
                    ("appVersion".to_string(), json!(2)),
                    ("hwVersion".to_string(), json!(3)),
                    ("dateCode".to_string(), json!("201901")),
                    ("swBuildId".to_string(), json!("1.01")),
                    ("powerSource".to_string(), json!(1)),
                    ("stackVersion".to_string(), json!(101)),
                ]),
            )]),
            ..MockDevice::default()
        },
    );
}
