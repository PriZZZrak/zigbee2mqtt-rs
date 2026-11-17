//! Minimal recording `ControllerAdapter` for the extension tests.
//!
//! Upstream mocks `Zigbee`/the zh controller with vitest spies; the Rust model
//! reaches the adapter through `Entity::adapter()`, so injecting this mock makes
//! group/bind/ping paths deterministic without a coordinator.

#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use zigbee_apiarist::adapter::events::ZclPayload;
use zigbee_apiarist::controller::model::entity::ControllerAdapter;
use zigbee_apiarist::controller::touchlink::AdapterFuture;
use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;

#[derive(Default)]
pub struct MockAdapter {
    pub zdo_calls: Mutex<Vec<(String, u16, u16, Vec<u8>, bool)>>,
    pub endpoint_frames: Mutex<Vec<Frame>>,
    pub group_frames: Mutex<Vec<(u16, Frame)>>,
    pub broadcast_frames: Mutex<Vec<(u8, u16, Frame)>>,
    pub fail_zdo: AtomicBool,
    pub fail_zcl: AtomicBool,
    pub zdo_response: Mutex<Value>,
    pub zcl_response: Mutex<Option<ZclPayload>>,
}

impl MockAdapter {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
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
    ) -> AdapterFuture<'_, Value> {
        self.zdo_calls.lock().unwrap().push((
            ieee_address.to_string(),
            network_address,
            cluster_id,
            payload.to_vec(),
            disable_response,
        ));
        let fail = self.fail_zdo.load(Ordering::SeqCst);
        let response = self.zdo_response.lock().unwrap().clone();
        Box::pin(async move {
            if fail {
                Err("ZDO failed".to_string())
            } else {
                Ok(response)
            }
        })
    }

    fn send_zcl_frame_to_endpoint(
        &self,
        _ieee_address: &str,
        _network_address: u16,
        _endpoint: u8,
        frame: &Frame,
        _timeout: u64,
        _disable_response: bool,
        _disable_recovery: bool,
        _source_endpoint: Option<u8>,
        _profile_id: Option<u16>,
    ) -> AdapterFuture<'_, Option<ZclPayload>> {
        self.endpoint_frames.lock().unwrap().push(frame.clone());
        let fail = self.fail_zcl.load(Ordering::SeqCst);
        let response = self.zcl_response.lock().unwrap().clone();
        Box::pin(async move {
            if fail {
                Err("ZCL failed".to_string())
            } else {
                Ok(response)
            }
        })
    }

    fn send_zcl_frame_to_group(
        &self,
        group_id: u16,
        frame: &Frame,
        _source_endpoint: Option<u8>,
        _profile_id: Option<u16>,
    ) -> AdapterFuture<'_, ()> {
        self.group_frames
            .lock()
            .unwrap()
            .push((group_id, frame.clone()));
        let fail = self.fail_zcl.load(Ordering::SeqCst);
        Box::pin(async move {
            if fail {
                Err("ZCL group failed".to_string())
            } else {
                Ok(())
            }
        })
    }

    fn send_zcl_frame_to_all(
        &self,
        endpoint: u8,
        frame: &Frame,
        _source_endpoint: u8,
        destination: u16,
        _profile_id: Option<u16>,
    ) -> AdapterFuture<'_, ()> {
        self.broadcast_frames
            .lock()
            .unwrap()
            .push((endpoint, destination, frame.clone()));
        let fail = self.fail_zcl.load(Ordering::SeqCst);
        Box::pin(async move {
            if fail {
                Err("ZCL broadcast failed".to_string())
            } else {
                Ok(())
            }
        })
    }
}

/// Convenience: a response payload for an empty ZDO success.
pub fn zdo_ok() -> Value {
    json!({"status": 0})
}
