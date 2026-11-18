//! Recording `ControllerRuntimeAdapter` for bridge tests (audit CP-07).
//!
//! These are test doubles only: production code never sees this module.

#![allow(dead_code)]

use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tokio::sync::mpsc;

use zigbee_apiarist::adapter::events::ZclPayload;
use zigbee_apiarist::controller::controller::{
    ControllerAdapterEvent, ControllerCoordinatorVersion, ControllerFuture,
    ControllerNetworkParameters, ControllerRuntimeAdapter, StartResult,
};
use zigbee_apiarist::controller::green_power::{GreenPowerAdapter, GreenPowerNetworkParameters};
use zigbee_apiarist::controller::model::entity::ControllerAdapter;
use zigbee_apiarist::controller::touchlink::{AdapterFuture, TouchlinkAdapter};
use zigbee_apiarist::models::backup::Backup;
use zigbee_apiarist::models::backup::BackupDevice;

use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;

pub struct MockRuntimeAdapter {
    pub permit_joins: Mutex<Vec<(u8, Option<u16>)>>,
    pub backup_devices: Mutex<Vec<BackupDevice>>,
    pub install_codes: Mutex<Vec<(String, Vec<u8>, bool)>>,
    pub supports_backup: AtomicBool,
    pub backup_calls: Mutex<usize>,
    pub fail_permit_join: AtomicBool,
    pub fail_install_code: AtomicBool,
}

impl Default for MockRuntimeAdapter {
    fn default() -> Self {
        Self {
            permit_joins: Mutex::new(Vec::new()),
            backup_devices: Mutex::new(Vec::new()),
            install_codes: Mutex::new(Vec::new()),
            supports_backup: AtomicBool::new(true),
            backup_calls: Mutex::new(0),
            fail_permit_join: AtomicBool::new(false),
            fail_install_code: AtomicBool::new(false),
        }
    }
}

impl MockRuntimeAdapter {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

impl ControllerAdapter for MockRuntimeAdapter {
    fn has_zdo_message_overhead(&self) -> bool {
        false
    }

    fn send_zdo(
        &self,
        _ieee_address: &str,
        _network_address: u16,
        _cluster_id: u16,
        _payload: &[u8],
        _disable_response: bool,
    ) -> AdapterFuture<'_, Value> {
        Box::pin(async { Ok(json!([0])) })
    }

    fn send_zcl_frame_to_endpoint(
        &self,
        _ieee_address: &str,
        _network_address: u16,
        _endpoint: u8,
        _frame: &Frame,
        _timeout: u64,
        _disable_response: bool,
        _disable_recovery: bool,
        _source_endpoint: Option<u8>,
        _profile_id: Option<u16>,
    ) -> AdapterFuture<'_, Option<ZclPayload>> {
        Box::pin(async { Ok(None) })
    }

    fn send_zcl_frame_to_group(
        &self,
        _group_id: u16,
        _frame: &Frame,
        _source_endpoint: Option<u8>,
        _profile_id: Option<u16>,
    ) -> AdapterFuture<'_, ()> {
        Box::pin(async { Ok(()) })
    }

    fn send_zcl_frame_to_all(
        &self,
        _endpoint: u8,
        _frame: &Frame,
        _source_endpoint: u8,
        _destination: u16,
        _profile_id: Option<u16>,
    ) -> AdapterFuture<'_, ()> {
        Box::pin(async { Ok(()) })
    }
}

impl GreenPowerAdapter for MockRuntimeAdapter {
    fn get_coordinator_ieee(
        &self,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + '_>> {
        Box::pin(async { Ok("0x00124b0024c16b95".to_string()) })
    }

    fn get_network_parameters(
        &self,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = Result<GreenPowerNetworkParameters, String>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async { Ok(GreenPowerNetworkParameters { channel: 11 }) })
    }

    fn send_zcl_frame_to_all(
        &self,
        _endpoint: u8,
        _frame: &Frame,
        _source_endpoint: u8,
        _destination: u16,
        _profile_id: Option<u16>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }

    fn send_zcl_frame_to_endpoint(
        &self,
        _ieee_address: &str,
        _network_address: u16,
        _endpoint: u8,
        _frame: &Frame,
        _timeout: u64,
        _disable_response: bool,
        _disable_recovery: bool,
        _source_endpoint: Option<u8>,
        _profile_id: Option<u16>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Option<ZclPayload>, String>> + Send + '_>>
    {
        Box::pin(async { Ok(None) })
    }
}

impl TouchlinkAdapter for MockRuntimeAdapter {
    fn set_channel_inter_pan(&self, _channel: u8) -> AdapterFuture<'_, ()> {
        Box::pin(async { Ok(()) })
    }

    fn restore_channel_inter_pan(&self) -> AdapterFuture<'_, ()> {
        Box::pin(async { Ok(()) })
    }

    fn send_zcl_frame_inter_pan_broadcast(
        &self,
        _frame: &Frame,
        _timeout: u64,
        _disable_response: bool,
    ) -> AdapterFuture<'_, Option<ZclPayload>> {
        Box::pin(async { Ok(None) })
    }

    fn send_zcl_frame_inter_pan_to_ieee_addr(
        &self,
        _frame: &Frame,
        _ieee_address: &str,
    ) -> AdapterFuture<'_, ()> {
        Box::pin(async { Ok(()) })
    }
}

impl ControllerRuntimeAdapter for MockRuntimeAdapter {
    fn start(&self) -> ControllerFuture<'_, StartResult> {
        Box::pin(async { Ok(StartResult::Resumed) })
    }

    fn stop(&self) -> ControllerFuture<'_, ()> {
        Box::pin(async { Ok(()) })
    }

    fn subscribe(&self) -> mpsc::UnboundedReceiver<ControllerAdapterEvent> {
        let (_tx, rx) = mpsc::unbounded_channel();
        rx
    }

    fn permit_join(&self, time: u8, network_address: Option<u16>) -> ControllerFuture<'_, ()> {
        self.permit_joins
            .lock()
            .unwrap()
            .push((time, network_address));
        let fail = self.fail_permit_join.load(Ordering::SeqCst);
        Box::pin(async move {
            if fail {
                Err("permit join failed".to_string())
            } else {
                Ok(())
            }
        })
    }

    fn get_network_parameters_full(&self) -> ControllerFuture<'_, ControllerNetworkParameters> {
        Box::pin(async {
            Ok(ControllerNetworkParameters {
                pan_id: 0x1a62,
                extended_pan_id: "0xdddddddddddddddd".to_string(),
                channel: 11,
                nwk_update_id: 0,
            })
        })
    }

    fn get_coordinator_version(&self) -> ControllerFuture<'_, ControllerCoordinatorVersion> {
        Box::pin(async {
            Ok(ControllerCoordinatorVersion {
                r#type: "zStack".to_string(),
                meta: json!({"version": "test"}),
            })
        })
    }

    fn supports_backup(&self) -> ControllerFuture<'_, bool> {
        let supported = self.supports_backup.load(Ordering::SeqCst);
        Box::pin(async move { Ok(supported) })
    }

    fn backup(&self, _ieee_addresses: &[String]) -> ControllerFuture<'_, Backup> {
        *self.backup_calls.lock().unwrap() += 1;
        let devices = self.backup_devices.lock().unwrap().clone();
        Box::pin(async move {
            Ok(Backup {
                devices,
                ..Default::default()
            })
        })
    }

    fn reset(&self, _ty: &str) -> ControllerFuture<'_, ()> {
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
        let fail = self.fail_install_code.load(Ordering::SeqCst);
        Box::pin(async move {
            if fail {
                Err("install code failed".to_string())
            } else {
                Ok(())
            }
        })
    }

    fn manufacturer_id(&self) -> Option<u16> {
        Some(4476)
    }
}
