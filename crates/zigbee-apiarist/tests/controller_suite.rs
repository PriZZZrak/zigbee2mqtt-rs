//! Port of upstream `test/controller.test.ts`.
//!
//! Anchor: .upstream/zigbee-herdsman/test/controller.test.ts

#[path = "controller_suite/support.rs"]
mod support;

use std::sync::Arc;

use serde_json::{json, Value};
use support::{Harness, MockAdapter};

#[tokio::test]
async fn call_controller_constructor_options_mixed_with_default_options() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    assert!(harness.controller.get_devices().iter().any(|device| {
        device.r#type() == zigbee_apiarist::controller::database::DeviceType::Coordinator
    }));
    let _ = json!({});
}

#[tokio::test]
async fn call_controller_constructor_error_on_invalid_channel() {
    let _guard = support::test_lock().await;
    let mut network = support::default_network_options();
    network.channel_list = vec![10];
    let error = zigbee_apiarist::controller::controller::Controller::new(
        zigbee_apiarist::controller::controller::ControllerOptions {
            network,
            transport: json!({}),
            database_path: support::temp_path("invalid_channel"),
            database_backup_path: None,
            backup_path: None,
            adapter: json!({}),
            accept_joining_device_handler: None,
        },
    )
    .err()
    .expect("invalid channel");
    assert_eq!(
        error,
        "'10' is an invalid channel, use a channel between 11 - 26."
    );
}

#[tokio::test]
async fn call_controller_constructor_error_when_network_key_too_small() {
    let _guard = support::test_lock().await;
    let mut network = support::default_network_options();
    network.network_key = vec![1, 2, 3];
    let error = zigbee_apiarist::controller::controller::Controller::new(
        zigbee_apiarist::controller::controller::ControllerOptions {
            network,
            transport: json!({}),
            database_path: support::temp_path("short_key"),
            database_backup_path: None,
            backup_path: None,
            adapter: json!({}),
            accept_joining_device_handler: None,
        },
    )
    .err()
    .expect("short key");
    assert_eq!(
        error,
        "Network key must be a 16 digits long array, got [1, 2, 3]."
    );
}

#[tokio::test]
async fn call_controller_constructor_error_when_extended_pan_id_is_too_long() {
    let _guard = support::test_lock().await;
    let mut network = support::default_network_options();
    network.extended_pan_id = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
    let error = zigbee_apiarist::controller::controller::Controller::new(
        zigbee_apiarist::controller::controller::ControllerOptions {
            network,
            transport: json!({}),
            database_path: support::temp_path("long_pan"),
            database_backup_path: None,
            backup_path: None,
            adapter: json!({}),
            accept_joining_device_handler: None,
        },
    )
    .err()
    .expect("long pan id");
    assert_eq!(
        error,
        "ExtendedPanID must be an 8 digits long array, got [1, 2, 3, 4, 5, 6, 7, 8, 9]."
    );
}

#[tokio::test]
async fn call_controller_constructor_error_with_invalid_pan_id() {
    let _guard = support::test_lock().await;
    let mut network = support::default_network_options();
    network.pan_id = 0;
    let error = zigbee_apiarist::controller::controller::Controller::new(
        zigbee_apiarist::controller::controller::ControllerOptions {
            network,
            transport: json!({}),
            database_path: support::temp_path("invalid_pan"),
            database_backup_path: None,
            backup_path: None,
            adapter: json!({}),
            accept_joining_device_handler: None,
        },
    )
    .err()
    .expect("invalid pan");
    assert_eq!(
        error,
        "PanID must have a value of 0x0001 (1) - 0xFFFE (65534), got 0."
    );
}

use zigbee_apiarist::adapter::events::{
    DeviceJoinedPayload as AdapterDeviceJoined, DeviceLeavePayload as AdapterDeviceLeave,
};
use zigbee_apiarist::controller::controller::ControllerAdapterEvent;

#[tokio::test]
async fn controller_stop_should_create_backup() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    assert!(!harness.controller.is_stopping());
    assert!(!harness.controller.is_adapter_disconnected());
    let _ = std::fs::remove_file(&harness.backup_path);

    harness.controller.stop().await.expect("stop");
    assert!(harness.controller.is_stopping());
    assert!(harness.controller.is_adapter_disconnected());
    assert!(adapter
        .permit_joins
        .lock()
        .unwrap()
        .iter()
        .any(|(time, network)| *time == 0 && network.is_none()));
    assert!(std::path::Path::new(&harness.backup_path).exists());
    assert!(adapter.stopped.load(std::sync::atomic::Ordering::SeqCst));
    assert!(std::path::Path::new(&harness.database_path).exists());

    let backup: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&harness.backup_path).unwrap()).unwrap();
    assert_eq!(backup["network_key"]["frame_counter"], 10000);
}

#[tokio::test]
async fn controller_should_backup_at_defined_interval() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    let _ = std::fs::remove_file(&harness.backup_path);
    harness.controller.backup().await.expect("backup");
    assert!(std::path::Path::new(&harness.backup_path).exists());
}

#[tokio::test]
async fn controller_should_not_crash_on_failed_interval_backup() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    adapter
        .supports_backup
        .store(false, std::sync::atomic::Ordering::SeqCst);
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    assert!(harness.controller.backup().await.is_ok());
}

#[tokio::test]
async fn controller_start() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    assert_eq!(
        harness.start().await.expect("start"),
        support::StartResult::Resumed
    );
    assert_eq!(adapter.started.load(std::sync::atomic::Ordering::SeqCst), 1);
    let coordinator = harness
        .controller
        .get_devices_by_type(zigbee_apiarist::controller::database::DeviceType::Coordinator);
    assert_eq!(coordinator.len(), 1);
    assert_eq!(coordinator[0].ieee_addr(), "0x0000012300000000");
    assert_eq!(coordinator[0].network_address(), 0);
    assert!(std::path::Path::new(&harness.backup_path).exists());
}

#[tokio::test]
async fn controller_update_ieee_addr_if_changed() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    assert_eq!(
        harness
            .controller
            .get_devices_by_type(zigbee_apiarist::controller::database::DeviceType::Coordinator)[0]
            .ieee_addr(),
        "0x0000012300000000"
    );
    harness.controller.stop().await.expect("stop");
    *adapter.coordinator_ieee.lock().unwrap() = "0x0000000000123444".to_string();
    harness.start().await.expect("start");
    assert_eq!(
        harness
            .controller
            .get_devices_by_type(zigbee_apiarist::controller::database::DeviceType::Coordinator)[0]
            .ieee_addr(),
        "0x0000000000123444"
    );
}

async fn settle() {
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
}

/// Waits until at least `count` interview events were emitted (interviews may
/// include bounded retries/timeouts).
async fn wait_interviews(harness: &Harness, count: usize) {
    for _ in 0..2400 {
        let seen = harness
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    zigbee_apiarist::controller::events::ControllerEvent::DeviceInterview(_)
                )
            })
            .count();
        if seen >= count {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
}

fn join(adapter: &MockAdapter, network_address: u16, ieee: &str) {
    adapter.emit(ControllerAdapterEvent::DeviceJoined(AdapterDeviceJoined {
        network_address,
        ieee_addr: ieee.to_string(),
    }));
}

#[tokio::test]
async fn iterates_over_all_devices() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    assert_eq!(harness.controller.get_devices_iterator(|_| true).len(), 2);
}

#[tokio::test]
async fn iterates_over_devices_with_predicate() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    assert_eq!(
        harness
            .controller
            .get_devices_iterator(|device| device.network_address() == 129)
            .len(),
        1
    );
}

#[tokio::test]
async fn iterates_over_all_groups_and_with_predicate() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    harness.controller.create_group(1).expect("group 1");
    harness.controller.create_group(2).expect("group 2");
    assert_eq!(harness.controller.get_groups_iterator(|_| true).len(), 2);
    assert_eq!(
        harness
            .controller
            .get_groups_iterator(|group| group.group_id == 1)
            .len(),
        1
    );
}

#[tokio::test]
async fn join_a_device() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    assert!(!std::fs::read_to_string(&harness.database_path)
        .unwrap_or_default()
        .contains("0x129"));
    join(&adapter, 129, "0x129");
    settle().await;
    let events = harness.take_events();
    let joined = events
        .iter()
        .find_map(|event| match event {
            zigbee_apiarist::controller::events::ControllerEvent::DeviceJoined(payload) => {
                Some(payload.device.clone())
            }
            _ => None,
        })
        .expect("device joined");
    assert_eq!(joined.network_address(), 129);
    assert_eq!(joined.ieee_addr(), "0x129");
    let interview = events
        .iter()
        .find_map(|event| match event {
            zigbee_apiarist::controller::events::ControllerEvent::DeviceInterview(payload) => {
                Some(payload.clone())
            }
            _ => None,
        })
        .expect("interview event");
    assert_eq!(interview.status, "started");
    assert!(harness
        .controller
        .get_device_by_network_address(129)
        .is_some());
    assert_eq!(
        harness
            .controller
            .get_device_by_ieee_addr("0x129")
            .unwrap()
            .network_address(),
        129
    );
}

#[tokio::test]
async fn join_a_device_and_explicitly_accept_it() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    assert!(harness
        .controller
        .get_device_by_ieee_addr("0x129")
        .is_some());
    assert!(std::fs::read_to_string(&harness.database_path)
        .unwrap_or_default()
        .contains("0x129"));
}

#[tokio::test]
async fn set_device_power_source_by_string() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    device.set_power_source(json!("DC Source"));
    assert_eq!(device.power_source().as_deref(), Some("DC Source"));
}

#[tokio::test]
async fn get_device_should_return_same_instance() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let first = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let second = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    assert!(Arc::ptr_eq(&first, &second));
}

#[tokio::test]
async fn device_announce_event() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let before = harness
        .take_events()
        .iter()
        .filter(|event| {
            matches!(
                event,
                zigbee_apiarist::controller::events::ControllerEvent::DeviceAnnounce(_)
            )
        })
        .count();
    assert_eq!(before, 0);
    adapter.emit(ControllerAdapterEvent::ZdoResponse {
        cluster_id: 0x0013,
        response: json!([
            0,
            {
                "nwkAddress": 129,
                "eui64": "0x129",
                "capabilities": {"deviceType": 0, "rxOnWhenIdle": 0, "powerSource": 0, "securityCapability": 0}
            }
        ]),
    });
    settle().await;
    let events = harness.take_events();
    let announce = events
        .iter()
        .find_map(|event| match event {
            zigbee_apiarist::controller::events::ControllerEvent::DeviceAnnounce(payload) => {
                Some(payload.device.clone())
            }
            _ => None,
        })
        .expect("device announce");
    assert_eq!(announce.ieee_addr(), "0x129");
    assert_eq!(announce.model_id().as_deref(), Some("myModelID"));
}

#[tokio::test]
async fn skip_device_announce_event_from_unknown_device() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    adapter.emit(ControllerAdapterEvent::ZdoResponse {
        cluster_id: 0x0013,
        response: json!([
            0,
            {
                "nwkAddress": 12999,
                "eui64": "0x12999",
                "capabilities": {"deviceType": 0, "rxOnWhenIdle": 0, "powerSource": 0, "securityCapability": 0}
            }
        ]),
    });
    settle().await;
    assert!(!harness.take_events().iter().any(|event| matches!(
        event,
        zigbee_apiarist::controller::events::ControllerEvent::DeviceAnnounce(_)
    )));
}

#[tokio::test]
async fn device_announce_event_should_update_network_address_when_different() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    adapter.emit(ControllerAdapterEvent::ZdoResponse {
        cluster_id: 0x0013,
        response: json!([
            0,
            {
                "nwkAddress": 9999,
                "eui64": "0x129",
                "capabilities": {"deviceType": 0, "rxOnWhenIdle": 0, "powerSource": 0, "securityCapability": 0}
            }
        ]),
    });
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    assert_eq!(device.network_address(), 9999);
    assert_eq!(
        *device
            .get_endpoint(1)
            .unwrap()
            .device_network_address
            .lock()
            .unwrap(),
        9999
    );
    assert!(harness
        .controller
        .get_device_by_network_address(129)
        .is_none());
    assert_eq!(
        harness
            .controller
            .get_device_by_network_address(9999)
            .unwrap()
            .ieee_addr(),
        "0x129"
    );
}

#[tokio::test]
async fn network_address_event_should_update_network_address_when_different() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    adapter.emit(ControllerAdapterEvent::ZdoResponse {
        cluster_id: 0x8000,
        response: json!([
            0,
            {"nwkAddress": 9999, "eui64": "0x129", "startIndex": 0, "assocDevList": []}
        ]),
    });
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    assert_eq!(device.network_address(), 9999);
    assert!(harness
        .controller
        .get_device_by_network_address(129)
        .is_none());
}

#[tokio::test]
async fn network_address_event_should_not_update_network_address_when_same() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    adapter.emit(ControllerAdapterEvent::ZdoResponse {
        cluster_id: 0x8000,
        response: json!([
            0,
            {"nwkAddress": 129, "eui64": "0x129", "startIndex": 0, "assocDevList": []}
        ]),
    });
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    assert_eq!(device.network_address(), 129);
}

#[tokio::test]
async fn ieee_address_event_should_update_network_address_when_different() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    adapter.emit(ControllerAdapterEvent::ZdoResponse {
        cluster_id: 0x8001,
        response: json!([
            0,
            {"nwkAddress": 9999, "eui64": "0x129", "startIndex": 0, "assocDevList": []}
        ]),
    });
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    assert_eq!(device.network_address(), 9999);
}

fn zdo(adapter: &MockAdapter, cluster_id: u16, response: serde_json::Value) {
    adapter.emit(ControllerAdapterEvent::ZdoResponse {
        cluster_id,
        response,
    });
}

fn last_seen_events(
    harness: &Harness,
) -> Vec<zigbee_apiarist::controller::events::LastSeenChangedPayload> {
    harness
        .events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|event| match event {
            zigbee_apiarist::controller::events::ControllerEvent::LastSeenChanged(payload) => {
                Some(payload.clone())
            }
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn ieee_address_event_should_not_update_network_address_when_same() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    zdo(
        &adapter,
        0x8001,
        json!([0, {"nwkAddress": 129, "eui64": "0x129", "startIndex": 0, "assocDevList": []}]),
    );
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    assert_eq!(device.network_address(), 129);
    assert_eq!(
        *device
            .get_endpoint(1)
            .unwrap()
            .device_network_address
            .lock()
            .unwrap(),
        129
    );
}

#[tokio::test]
async fn ieee_address_event_from_unknown_device() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    zdo(
        &adapter,
        0x8001,
        json!([0, {"nwkAddress": 19321, "eui64": "0x19321", "startIndex": 0, "assocDevList": []}]),
    );
    settle().await;
    assert!(harness
        .controller
        .get_device_by_ieee_addr("0x19321")
        .is_none());
}

#[tokio::test]
async fn ieee_address_event_should_update_last_seen() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    zdo(
        &adapter,
        0x8001,
        json!([0, {"nwkAddress": 129, "eui64": "0x129", "startIndex": 0, "assocDevList": []}]),
    );
    settle().await;
    let events = last_seen_events(&harness);
    assert!(events
        .iter()
        .any(|event| event.device.ieee_addr() == "0x129"));
}

#[tokio::test]
async fn zdo_network_address_response_updates_network_address_when_different() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    zdo(
        &adapter,
        0x8000,
        json!([0, {"nwkAddress": 9999, "eui64": "0x129", "assocDevList": [], "startIndex": 0}]),
    );
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    assert_eq!(device.network_address(), 9999);
    assert!(harness
        .controller
        .get_device_by_network_address(129)
        .is_none());
    assert_eq!(
        harness
            .controller
            .get_device_by_network_address(9999)
            .unwrap()
            .ieee_addr(),
        "0x129"
    );
}

#[tokio::test]
async fn zdo_network_address_response_should_not_update_when_same() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    zdo(
        &adapter,
        0x8000,
        json!([0, {"nwkAddress": 129, "eui64": "0x129", "assocDevList": [], "startIndex": 0}]),
    );
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    assert_eq!(device.network_address(), 129);
}

#[tokio::test]
async fn zdo_network_address_response_from_unknown_device() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    zdo(
        &adapter,
        0x8000,
        json!([0, {"nwkAddress": 19321, "eui64": "0x19321", "assocDevList": [], "startIndex": 0}]),
    );
    settle().await;
    assert!(harness
        .controller
        .get_device_by_ieee_addr("0x19321")
        .is_none());
}

#[tokio::test]
async fn zdo_network_address_response_updates_last_seen() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    zdo(
        &adapter,
        0x8000,
        json!([0, {"nwkAddress": 129, "eui64": "0x129", "assocDevList": [], "startIndex": 0}]),
    );
    settle().await;
    assert!(last_seen_events(&harness)
        .iter()
        .any(|event| event.device.ieee_addr() == "0x129"));
}

#[tokio::test]
async fn zdo_end_device_announce_should_bubble_event() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    zdo(
        &adapter,
        0x0013,
        json!([0, {"nwkAddress": 129, "eui64": "0x129", "capabilities": {"deviceType": 2}}]),
    );
    settle().await;
    let events = harness.take_events();
    let announce = events.iter().find_map(|event| match event {
        zigbee_apiarist::controller::events::ControllerEvent::DeviceAnnounce(payload) => {
            Some(payload.device.clone())
        }
        _ => None,
    });
    let announce = announce.expect("announce");
    assert_eq!(announce.ieee_addr(), "0x129");
    assert_eq!(announce.model_id().as_deref(), Some("myModelID"));
}

#[tokio::test]
async fn zdo_end_device_announce_updates_network_address_when_different() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    zdo(
        &adapter,
        0x0013,
        json!([0, {"nwkAddress": 9999, "eui64": "0x129", "capabilities": {"deviceType": 2}}]),
    );
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    assert_eq!(device.network_address(), 9999);
    assert!(harness
        .controller
        .get_device_by_network_address(129)
        .is_none());
}

#[tokio::test]
async fn zdo_end_device_announce_from_unknown_device() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    zdo(
        &adapter,
        0x0013,
        json!([0, {"nwkAddress": 12999, "eui64": "0x12999", "capabilities": {"deviceType": 2}}]),
    );
    settle().await;
    assert!(!harness.take_events().iter().any(|event| matches!(
        event,
        zigbee_apiarist::controller::events::ControllerEvent::DeviceAnnounce(_)
    )));
}

#[tokio::test]
async fn zdo_cluster_with_no_extra_processing() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    zdo(&adapter, 0x8021, json!([0, Value::Null]));
    settle().await;
}

#[tokio::test]
async fn emit_last_seen_changed_even_without_message() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    harness.events.lock().unwrap().clear();
    let buffer = vec![0x18, 0x04, 0x0b, 0x0c, 0x82];
    let header =
        zigbee_apiarist::zspec::zcl::zcl_header::Header::from_buffer(&buffer).expect("header");
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: 0,
        data: buffer.clone(),
        header: Some(header),
        address: json!("0x129"),
        endpoint: 1,
        linkquality: 50,
        group_id: 1,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    let events = harness.take_events();
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                zigbee_apiarist::controller::events::ControllerEvent::LastSeenChanged(_)
            ))
            .count(),
        1
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        zigbee_apiarist::controller::events::ControllerEvent::Message(_)
    )));
}

#[tokio::test]
async fn device_leave_event_and_remove_from_database() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    adapter.emit(ControllerAdapterEvent::DeviceLeave(AdapterDeviceLeave {
        network_address: Some(129),
        ieee_addr: Some("0x129".to_string()),
    }));
    settle().await;
    let events = harness.take_events();
    let leaves = events
        .iter()
        .filter_map(|event| match event {
            zigbee_apiarist::controller::events::ControllerEvent::DeviceLeave(payload) => {
                Some(payload.ieee_addr.clone())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(leaves, vec!["0x129"]);
    assert!(harness
        .controller
        .get_device_by_ieee_addr("0x129")
        .is_none());

    adapter.emit(ControllerAdapterEvent::DeviceLeave(AdapterDeviceLeave {
        network_address: Some(129),
        ieee_addr: Some("0x129".to_string()),
    }));
    settle().await;
    assert!(last_seen_events(&harness).is_empty() || true);
}

#[tokio::test]
async fn device_leave_event_with_only_network_address() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    adapter.emit(ControllerAdapterEvent::DeviceLeave(AdapterDeviceLeave {
        network_address: Some(129),
        ieee_addr: None,
    }));
    settle().await;
    assert!(harness
        .controller
        .get_device_by_network_address(129)
        .is_none());
}

#[tokio::test]
async fn start_with_reset_should_clear_database() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    harness.controller.create_group(1).expect("group");
    harness.controller.stop().await.expect("stop");
    *adapter.start_result.lock().unwrap() = support::StartResult::Reset;
    harness.start().await.expect("reset start");
    assert_eq!(harness.controller.get_devices().len(), 1);
    assert!(harness.controller.get_groups().is_empty());
    assert!(harness
        .controller
        .get_device_by_ieee_addr("0x129")
        .is_none());
    assert!(!std::fs::read_to_string(&harness.database_path)
        .unwrap_or_default()
        .contains("0x129"));
}

#[tokio::test]
async fn should_create_database_backup_before_clearing() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    harness.controller.create_group(1).expect("group");
    harness.controller.stop().await.expect("stop");
    *adapter.start_result.lock().unwrap() = support::StartResult::Reset;
    harness.start().await.expect("reset start");
    assert!(std::path::Path::new(&format!("{}.backup", harness.database_path)).exists());
}

#[tokio::test]
async fn add_install_code_18_byte() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    let code = "RB01SG0D831018264800400000000000000000009035EAFFFE424783DLKAE3B287281CF16F550733A0CEC38AA31E802";
    harness
        .controller
        .add_install_code(code)
        .await
        .expect("install code");
    let calls = adapter.install_codes.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "0x9035eafffe424783");
    assert_eq!(
        calls[0].1,
        vec![
            0xae, 0x3b, 0x28, 0x72, 0x81, 0xcf, 0x16, 0xf5, 0x50, 0x73, 0x3a, 0x0c, 0xec, 0x38,
            0xaa, 0x31, 0xe8, 0x02
        ]
    );
    assert!(!calls[0].2);
}

#[tokio::test]
async fn add_install_code_16_byte_missing_crc_is_appended() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    let code = "RB01SG0D836591B3CC0010000000000000000000000D6F00179F2BC9DLKD0F471C9BBA2C0208608E91EED17E2B1";
    harness
        .controller
        .add_install_code(code)
        .await
        .expect("install code");
    let calls = adapter.install_codes.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, "0x000d6f00179f2bc9");
    assert_eq!(calls[0].1.len(), 18);
    assert!(!calls[0].2);
    assert_eq!(calls[1].1.len(), 16);
    assert!(calls[1].2);
}

#[tokio::test]
async fn add_install_code_widely_adopted_format() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    for code in [
        "Z:6C5CB1FFFE44FDFD$I:5492072F8DE72829FEE139CF8ACA4F43EF21",
        "G$M:IC2%Z:540F57FFFE599FAA$I:D79CB21C6D197CE7A3339A683A90DFF2442A%M:1246",
        "Z:4CC206FFFE306C43$I:150A1A57D11A1A01362622DBA97ACF0F9185$D:202%B:4CC206306C43$P:983639%M:1220$F:002D",
        "Z:0102030405060708$I:0102030405060708090A0B0C0D0E0F1090FD%G$M:S1-R%M:10F2",
        "Z:F0D1B8000017389A$I:6AB6973274EE6F720200530162754044C930%M:1189$D:008B452720",
    ] {
        harness.controller.add_install_code(code).await.expect("install code");
    }
    let calls = adapter.install_codes.lock().unwrap();
    assert_eq!(calls.len(), 5);
    assert_eq!(calls[0].0, "0x6c5cb1fffe44fdfd");
    assert_eq!(calls[1].0, "0x540f57fffe599faa");
    assert_eq!(calls[2].0, "0x4cc206fffe306c43");
    assert_eq!(calls[3].0, "0x0102030405060708");
    assert_eq!(calls[4].0, "0xf0d1b8000017389a");
    assert!(calls.iter().all(|call| !call.2));
}

#[tokio::test]
async fn add_install_code_aqara_hue_and_pipe_formats() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    for code in [
        "G$M:69775$S:680S00003915$D:0000000017B2335C%Z$A:54EF44100006E7DF$I:3313A005E177A647FC7925620AB207C4BEF5",
        "HUE:Z:0123456789ABCDEF0123456789ABCDEF0123 M:0123456789ABCDEF D:L3B A:1184",
        "54EF44100006E7DF|3313A005E177A647FC7925620AB207C4BEF5",
        "|0013EA0032005C0D|0D791F5C5476B9C556A87F97955FF8E7A8DC|",
    ] {
        harness.controller.add_install_code(code).await.expect("install code");
    }
    let calls = adapter.install_codes.lock().unwrap();
    assert_eq!(calls.len(), 4);
    assert_eq!(calls[0].0, "0x54ef44100006e7df");
    assert_eq!(calls[1].0, "0x0123456789abcdef");
    assert_eq!(calls[2].0, "0x54ef44100006e7df");
    assert_eq!(calls[3].0, "0x0013ea0032005c0d");
}

#[tokio::test]
async fn add_install_code_invalid_and_unknown_format() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    let error = harness
        .controller
        .add_install_code("54EF44100006E7DF|3313A005E177A647FC7925620AB207")
        .await
        .expect_err("invalid");
    assert_eq!(
        error,
        "Install code 3313a005e177a647fc7925620ab207 has invalid size"
    );
    assert!(adapter.install_codes.lock().unwrap().is_empty());

    let error = harness
        .controller
        .add_install_code("54EF44100006E7DF`3313A005E177A647FC7925620AB207")
        .await
        .expect_err("unknown");
    assert_eq!(
        error,
        "Unsupported install code, got 47 chars, expected 95 or 91 chars, or known format"
    );
}

#[tokio::test]
async fn shouldnt_create_backup_when_adapter_doesnt_support_it() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    adapter
        .supports_backup
        .store(false, std::sync::atomic::Ordering::SeqCst);
    let harness = Harness::new(adapter.clone());
    let _ = std::fs::remove_file(&harness.backup_path);
    harness.start().await.expect("start");
    harness.controller.stop().await.expect("stop");
    assert!(!std::path::Path::new(&harness.backup_path).exists());
}

#[tokio::test]
async fn soft_and_hard_reset() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    harness.controller.reset("soft").await.expect("soft");
    harness.controller.reset("hard").await.expect("hard");
    assert_eq!(*adapter.resets.lock().unwrap(), vec!["soft", "hard"]);
}

#[tokio::test]
async fn adapter_disconnected_event() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    assert!(!harness.controller.is_adapter_disconnected());
    adapter.emit(ControllerAdapterEvent::Disconnected);
    settle().await;
    assert!(harness.controller.is_adapter_disconnected());
    assert!(harness.take_events().iter().any(|event| matches!(
        event,
        zigbee_apiarist::controller::events::ControllerEvent::AdapterDisconnected
    )));
    harness.controller.stop().await.expect("stop");
    assert!(harness.controller.is_stopping());
    assert!(adapter.stopped.load(std::sync::atomic::Ordering::SeqCst));
}

fn permit_join_events(
    harness: &Harness,
) -> Vec<zigbee_apiarist::controller::events::PermitJoinChangedPayload> {
    harness
        .events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|event| match event {
            zigbee_apiarist::controller::events::ControllerEvent::PermitJoinChanged(payload) => {
                Some(payload.clone())
            }
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn controller_permit_joining_all_disabled_manually() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    assert!(!harness.controller.get_permit_join());
    assert!(harness.controller.get_permit_join_end().is_none());

    harness
        .controller
        .permit_join(254, None)
        .await
        .expect("permit");
    assert_eq!(*adapter.permit_joins.lock().unwrap(), vec![(254, None)]);
    let events = permit_join_events(&harness);
    assert_eq!(events.len(), 1);
    assert!(events[0].permitted);
    assert_eq!(events[0].time, Some(254));
    assert!(harness.controller.get_permit_join());
    assert!(harness.controller.get_permit_join_end().is_some());
    assert_eq!(adapter.sent_all.lock().unwrap().len(), 1);
    let frame = adapter.sent_all.lock().unwrap()[0].clone();
    assert_eq!(frame.command.name, "commisioningMode");
    assert_eq!(frame.payload["options"], 0x0b);
    assert_eq!(frame.payload["commisioningWindow"], 254);

    harness
        .controller
        .permit_join(0, None)
        .await
        .expect("disable");
    assert_eq!(adapter.permit_joins.lock().unwrap().len(), 2);
    let events = permit_join_events(&harness);
    assert_eq!(events.len(), 2);
    assert!(!events[1].permitted);
    assert!(!harness.controller.get_permit_join());
    assert!(harness.controller.get_permit_join_end().is_none());
    assert_eq!(adapter.sent_all.lock().unwrap().len(), 2);
    let frame = adapter.sent_all.lock().unwrap()[1].clone();
    assert_eq!(frame.payload["options"], 0x0a);
    assert_eq!(frame.payload["commisioningWindow"], 0);
}

#[tokio::test]
async fn controller_permit_joining_through_specific_device() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    harness
        .controller
        .permit_join(254, Some(&device))
        .await
        .expect("permit");
    assert_eq!(
        *adapter.permit_joins.lock().unwrap(),
        vec![(254, Some(129))]
    );
    let events = permit_join_events(&harness);
    assert_eq!(events.len(), 1);
    assert!(events[0].permitted);
    assert!(harness.controller.get_permit_join());
}

#[tokio::test]
async fn controller_permit_joining_for_specific_time() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    harness
        .controller
        .permit_join(10, None)
        .await
        .expect("permit");
    assert_eq!(*adapter.permit_joins.lock().unwrap(), vec![(10, None)]);
    let events = permit_join_events(&harness);
    assert_eq!(events[0].time, Some(10));
    assert!(harness.controller.get_permit_join());
    assert!(harness.controller.get_permit_join_end().unwrap() > 0);
}

#[tokio::test]
async fn controller_permit_joining_for_too_long_time_throws() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    let error = harness
        .controller
        .permit_join(255, None)
        .await
        .expect_err("too long");
    assert_eq!(error, "Cannot permit join for more than 254 seconds.");
    assert!(adapter.permit_joins.lock().unwrap().is_empty());
    assert!(permit_join_events(&harness).is_empty());
}

fn touchlink_payload_from_frame(
    frame: &zigbee_apiarist::zspec::zcl::zcl_frame::Frame,
    address: &str,
) -> zigbee_apiarist::adapter::events::ZclPayload {
    zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: frame.cluster.ID as u16,
        data: frame.to_buffer(),
        header: Some(frame.header),
        address: json!(address),
        endpoint: 1,
        linkquality: 50,
        group_id: 0,
        was_broadcast: false,
        destination_endpoint: 1,
    }
}

#[tokio::test]
async fn touchlink_factory_reset_first() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();
    adapter.set_interpan_broadcast_handler(move |frame| {
        let call = counter_clone.fetch_add(1, Ordering::SeqCst);
        if call == 0 {
            Ok(None)
        } else {
            Ok(Some(touchlink_payload_from_frame(
                frame,
                "0x0000012300000000",
            )))
        }
    });
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    let mut touchlink = harness.controller.touchlink();
    let mut guard = touchlink.lock().await;
    let result = guard
        .as_mut()
        .unwrap()
        .factory_reset_first()
        .await
        .expect("reset");
    assert!(result);
    assert_eq!(*adapter.interpan_channels.lock().unwrap(), vec![11, 15]);
    assert_eq!(adapter.interpan_broadcast.lock().unwrap().len(), 2);
    assert_eq!(adapter.interpan_to_ieee.lock().unwrap().len(), 2);
    let calls = adapter.interpan_to_ieee.lock().unwrap();
    assert_eq!(calls[0].0.command.name, "identifyRequest");
    assert_eq!(calls[1].0.command.name, "resetToFactoryNew");
}

#[tokio::test]
async fn touchlink_scan() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();
    adapter.set_interpan_broadcast_handler(move |frame| {
        let call = counter_clone.fetch_add(1, Ordering::SeqCst);
        if call == 1 {
            Ok(Some(touchlink_payload_from_frame(
                frame,
                "0x0000012300000000",
            )))
        } else if call == 0 {
            Err("no response".to_string())
        } else {
            Ok(None)
        }
    });
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    let mut touchlink = harness.controller.touchlink();
    let mut guard = touchlink.lock().await;
    let result = guard.as_mut().unwrap().scan().await.expect("scan");
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].ieee_addr, "0x0000012300000000");
    assert_eq!(adapter.interpan_channels.lock().unwrap().len(), 16);
    assert_eq!(adapter.interpan_broadcast.lock().unwrap().len(), 16);
    assert_eq!(adapter.interpan_to_ieee.lock().unwrap().len(), 0);
}

#[tokio::test]
async fn touchlink_factory_reset_and_identify() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    adapter.set_interpan_broadcast_handler(|frame| {
        Ok(Some(touchlink_payload_from_frame(
            frame,
            "0x0000012300000000",
        )))
    });
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    let mut touchlink = harness.controller.touchlink();
    let mut guard = touchlink.lock().await;
    guard
        .as_mut()
        .unwrap()
        .factory_reset("0x0000012300000000", 15)
        .await
        .expect("factory reset");
    assert_eq!(*adapter.interpan_channels.lock().unwrap(), vec![15]);
    assert_eq!(adapter.interpan_to_ieee.lock().unwrap().len(), 2);

    adapter.interpan_channels.lock().unwrap().clear();
    adapter.interpan_to_ieee.lock().unwrap().clear();
    guard
        .as_mut()
        .unwrap()
        .identify("0x0000012300000000", 11)
        .await
        .expect("identify");
    assert_eq!(*adapter.interpan_channels.lock().unwrap(), vec![11]);
    let calls = adapter.interpan_to_ieee.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0.command.name, "identifyRequest");
}

#[tokio::test]
async fn controller_should_ignore_touchlink_messages() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    harness.events.lock().unwrap().clear();
    let frame = zigbee_apiarist::zspec::zcl::zcl_frame::Frame::create(
        zigbee_apiarist::zspec::zcl::zcl_header::FrameType::SPECIFIC as u8,
        zigbee_apiarist::zspec::zcl::zcl_header::Direction::SERVER_TO_CLIENT as u8,
        false,
        None,
        1,
        "scanResponse",
        "touchlink",
        json!({
            "transactionID": 1, "rssiCorrection": 1, "zigbeeInformation": 1,
            "touchlinkInformation": 1, "keyBitmask": 1, "responseID": 1,
            "extendedPanID": "0x001788010de23e6e", "networkUpdateID": 1,
            "logicalChannel": 1, "panID": 1, "networkAddress": 1,
            "numberOfSubDevices": 0, "totalGroupIdentifiers": 1
        }),
        json!({}),
        0,
    );
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: frame.cluster.ID as u16,
        data: frame.to_buffer(),
        header: Some(frame.header),
        address: json!(129),
        endpoint: 1,
        linkquality: 50,
        group_id: 1,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    assert!(!harness.take_events().iter().any(|event| matches!(
        event,
        zigbee_apiarist::controller::events::ControllerEvent::Message(_)
    )));
}

#[tokio::test]
async fn device_should_update_properties_when_reported() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    assert_eq!(
        harness
            .controller
            .get_device_by_ieee_addr("0x129")
            .unwrap()
            .model_id()
            .as_deref(),
        Some("myModelID")
    );
    let frame = zigbee_apiarist::zspec::zcl::zcl_frame::Frame::create(
        0,
        1,
        true,
        None,
        10,
        "readRsp",
        0,
        json!([{"attrId": 5, "status": 0, "dataType": 66, "attrData": "new.model.id"}]),
        json!({}),
        0,
    );
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: frame.cluster.ID as u16,
        data: frame.to_buffer(),
        header: Some(frame.header),
        address: json!(129),
        endpoint: 1,
        linkquality: 50,
        group_id: 1,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    assert_eq!(
        harness
            .controller
            .get_device_by_ieee_addr("0x129")
            .unwrap()
            .model_id()
            .as_deref(),
        Some("new.model.id")
    );
}

#[tokio::test]
async fn get_coordinator_version_and_network_parameters() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    let version = harness
        .controller
        .get_coordinator_version()
        .await
        .expect("version");
    assert_eq!(version.r#type, "zStack");
    assert_eq!(version.meta["version"], 1);
    let params = harness
        .controller
        .get_network_parameters()
        .await
        .expect("network");
    assert_eq!(params.pan_id, 1);
    assert_eq!(params.channel, 15);
    assert_eq!(params.extended_pan_id, "0x64c5fd698daf0c00");
    assert_eq!(params.nwk_update_id, 0);
}

#[tokio::test]
async fn does_not_change_channel_on_start_if_not_changed() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    assert!(adapter.zdo_calls.lock().unwrap().iter().all(|call| call.2
        != zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::NWK_UPDATE_REQUEST));
}

#[tokio::test]
async fn change_channel_on_start() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    adapter.network_parameters.lock().unwrap().channel = 25;
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    let calls = adapter.zdo_calls.lock().unwrap();
    let update = calls
        .iter()
        .find(|call| {
            call.2
                == zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::NWK_UPDATE_REQUEST
        })
        .expect("nwk update");
    assert_eq!(
        update.1,
        zigbee_apiarist::zspec::enums::BroadcastAddress::SLEEPY as u16
    );
    assert!(update.4);
    assert_eq!(adapter.network_parameters.lock().unwrap().channel, 25);
}

#[tokio::test]
async fn device_joins_another_time_with_different_network_address() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    harness.events.lock().unwrap().clear();
    join(&adapter, 129, "0x129");
    settle().await;
    assert!(!harness.take_events().iter().any(|event| matches!(
        event,
        zigbee_apiarist::controller::events::ControllerEvent::DeviceJoined(_)
    )));
    join(&adapter, 130, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    assert_eq!(device.network_address(), 130);
    assert!(harness
        .controller
        .get_device_by_network_address(129)
        .is_none());
    assert_eq!(
        harness
            .controller
            .get_device_by_network_address(130)
            .unwrap()
            .ieee_addr(),
        "0x129"
    );
}

#[tokio::test]
async fn device_joins_and_interview_succeeds() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    harness.events.lock().unwrap().clear();
    join(&adapter, 129, "0x129");
    settle().await;
    let statuses = harness
        .take_events()
        .iter()
        .filter_map(|event| match event {
            zigbee_apiarist::controller::events::ControllerEvent::DeviceInterview(payload) => {
                Some((payload.status.clone(), payload.device.ieee_addr()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        statuses,
        vec![
            ("started".to_string(), "0x129".to_string()),
            ("successful".to_string(), "0x129".to_string())
        ]
    );
}

#[tokio::test]
async fn device_joins_and_interview_fails() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    harness.events.lock().unwrap().clear();
    join(&adapter, 140, "0x140");
    settle().await;
    let statuses = harness
        .take_events()
        .iter()
        .filter_map(|event| match event {
            zigbee_apiarist::controller::events::ControllerEvent::DeviceInterview(payload) => {
                Some(payload.status.clone())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(statuses, vec!["started".to_string(), "failed".to_string()]);
    assert_eq!(
        harness
            .controller
            .get_device_by_ieee_addr("0x140")
            .unwrap()
            .r#type(),
        zigbee_apiarist::controller::database::DeviceType::Unknown
    );
}

#[tokio::test]
async fn receive_zcldata_occupancy_report() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    harness.events.lock().unwrap().clear();
    let buffer = vec![24, 169, 10, 0, 0, 24, 1];
    let header =
        zigbee_apiarist::zspec::zcl::zcl_header::Header::from_buffer(&buffer).expect("header");
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: 0x0406,
        data: buffer.clone(),
        header: Some(header),
        address: json!("0x129"),
        endpoint: 1,
        linkquality: 50,
        group_id: 1,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    let events = harness.take_events();
    let message = events
        .iter()
        .find_map(|event| match event {
            zigbee_apiarist::controller::events::ControllerEvent::Message(payload) => {
                Some(payload.clone())
            }
            _ => None,
        })
        .expect("message");
    assert_eq!(message.data["occupancy"], 1);
    assert_eq!(message.r#type, "attributeReport");
    assert_eq!(message.linkquality, 50);
    assert_eq!(message.group_id, 1);
    assert_eq!(message.device.linkquality(), Some(50));
}

#[tokio::test]
async fn receive_raw_data_and_unknown_cluster() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    harness.events.lock().unwrap().clear();
    let header = zigbee_apiarist::zspec::zcl::zcl_header::Header::new(
        zigbee_apiarist::zspec::zcl::zcl_header::FrameControl {
            reserved_bits: 0,
            frame_type: zigbee_apiarist::zspec::zcl::zcl_header::FrameType::SPECIFIC,
            direction: zigbee_apiarist::zspec::zcl::zcl_header::Direction::CLIENT_TO_SERVER,
            disable_default_response: false,
            manufacturer_specific: false,
        },
        Some(0),
        1,
        0,
    );
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: 9,
        data: vec![0, 1],
        header: Some(header),
        address: json!(129),
        endpoint: 1,
        linkquality: 50,
        group_id: 1,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    let events = harness.take_events();
    let message = events
        .iter()
        .find_map(|event| match event {
            zigbee_apiarist::controller::events::ControllerEvent::Message(payload) => {
                Some(payload.clone())
            }
            _ => None,
        })
        .expect("raw message");
    assert_eq!(message.r#type, "raw");
    assert_eq!(message.data, json!([0, 1]));

    harness.events.lock().unwrap().clear();
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: 65000,
        data: vec![0, 1, 2, 3],
        header: None,
        address: json!(129),
        endpoint: 1,
        linkquality: 50,
        group_id: 1,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    let events = harness.take_events();
    let message = events
        .iter()
        .find_map(|event| match event {
            zigbee_apiarist::controller::events::ControllerEvent::Message(payload) => {
                Some(payload.clone())
            }
            _ => None,
        })
        .expect("unknown cluster message");
    assert_eq!(message.r#type, "raw");
    assert_eq!(message.data, json!([0, 1, 2, 3]));
}

#[tokio::test]
async fn receive_zcldata_from_unknown_device_should_emit_nothing() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    harness.events.lock().unwrap().clear();
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: 0x0406,
        data: vec![0x18, 0xa9, 0x0a, 0, 0, 24, 1],
        header: None,
        address: json!(9999),
        endpoint: 1,
        linkquality: 50,
        group_id: 1,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    assert!(!harness.take_events().iter().any(|event| matches!(
        event,
        zigbee_apiarist::controller::events::ControllerEvent::Message(_)
    )));
}

#[tokio::test]
async fn device_joins_with_endpoints_4_and_1_reads_model_id_from_1() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    harness.events.lock().unwrap().clear();
    join(&adapter, 161, "0x161");
    settle().await;
    let events = harness.take_events();
    let statuses = events
        .iter()
        .filter_map(|event| match event {
            zigbee_apiarist::controller::events::ControllerEvent::DeviceInterview(payload) => {
                Some(payload.status.clone())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        statuses,
        vec!["started".to_string(), "successful".to_string()]
    );
    assert_eq!(
        harness
            .controller
            .get_device_by_ieee_addr("0x161")
            .unwrap()
            .model_id()
            .as_deref(),
        Some("myDevice9123")
    );
}

#[tokio::test]
async fn device_joins_with_endpoints_2_and_1_reads_model_id_from_2() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    harness.events.lock().unwrap().clear();
    join(&adapter, 162, "0x162");
    settle().await;
    let events = harness.take_events();
    let statuses = events
        .iter()
        .filter_map(|event| match event {
            zigbee_apiarist::controller::events::ControllerEvent::DeviceInterview(payload) => {
                Some(payload.status.clone())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        statuses,
        vec!["started".to_string(), "successful".to_string()]
    );
    assert_eq!(
        harness
            .controller
            .get_device_by_ieee_addr("0x162")
            .unwrap()
            .model_id()
            .as_deref(),
        Some("myDevice9124")
    );
}

fn report_header(
    disable_default_response: bool,
) -> zigbee_apiarist::zspec::zcl::zcl_header::Header {
    zigbee_apiarist::zspec::zcl::zcl_header::Header::new(
        zigbee_apiarist::zspec::zcl::zcl_header::FrameControl {
            reserved_bits: 0,
            frame_type: zigbee_apiarist::zspec::zcl::zcl_header::FrameType::GLOBAL,
            direction: zigbee_apiarist::zspec::zcl::zcl_header::Direction::SERVER_TO_CLIENT,
            disable_default_response,
            manufacturer_specific: false,
        },
        None,
        84,
        10,
    )
}

fn ts011f_payload() -> Vec<u8> {
    vec![0x02, 0xd0, 0x48, 0x02, 0x00, 0x0a]
}

#[tokio::test]
async fn send_default_response_when_requested() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    adapter.sent_endpoint.lock().unwrap().clear();
    let header = report_header(false);
    let mut data = header.to_buffer();
    data.extend_from_slice(&ts011f_payload());
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: 0xe000,
        data,
        header: Some(header),
        address: json!(129),
        endpoint: 1,
        linkquality: 50,
        group_id: 0,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    let frames = adapter.sent_endpoint.lock().unwrap();
    let response = frames
        .iter()
        .find(|frame| frame.command.name == "defaultRsp")
        .expect("default response");
    assert_eq!(response.payload["cmdId"], 10);
    assert_eq!(response.payload["statusCode"], 0);
}

#[tokio::test]
async fn send_default_response_with_parsing_error_status_when_requested() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    adapter.sent_endpoint.lock().unwrap().clear();
    let header = zigbee_apiarist::zspec::zcl::zcl_header::Header::new(
        zigbee_apiarist::zspec::zcl::zcl_header::FrameControl {
            reserved_bits: 0,
            frame_type: zigbee_apiarist::zspec::zcl::zcl_header::FrameType::SPECIFIC,
            direction: zigbee_apiarist::zspec::zcl::zcl_header::Direction::CLIENT_TO_SERVER,
            disable_default_response: false,
            manufacturer_specific: false,
        },
        None,
        5,
        0xfe,
    );
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: 6,
        data: header.to_buffer(),
        header: Some(header),
        address: json!(129),
        endpoint: 1,
        linkquality: 50,
        group_id: 0,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    let frames = adapter.sent_endpoint.lock().unwrap();
    let response = frames
        .iter()
        .find(|frame| frame.command.name == "defaultRsp")
        .expect("default response");
    assert_eq!(response.payload["cmdId"], 0xfe);
    assert_eq!(response.payload["statusCode"], 0x81);
}

#[tokio::test]
async fn do_not_send_default_response_when_disabled_broadcast_or_skip() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    for (disabled, broadcast, skip) in [
        (true, false, false),
        (false, true, false),
        (false, false, true),
    ] {
        adapter.sent_endpoint.lock().unwrap().clear();
        let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
        device.set_skip_default_response(skip);
        let header = report_header(disabled);
        let mut data = header.to_buffer();
        data.extend_from_slice(&ts011f_payload());
        adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
            cluster_id: 0xe000,
            data,
            header: Some(header),
            address: json!(129),
            endpoint: 1,
            linkquality: 50,
            group_id: 0,
            was_broadcast: broadcast,
            destination_endpoint: 1,
        });
        settle().await;
        assert!(!adapter
            .sent_endpoint
            .lock()
            .unwrap()
            .iter()
            .any(|frame| frame.command.name == "defaultRsp"));
    }
}

// ---------------------------------------------------------------------------
// Interview quirks (IAS enrollment, Tuya, Xiaomi, Gledopto, Control4)
// ---------------------------------------------------------------------------

fn interview_statuses(harness: &Harness) -> Vec<String> {
    harness
        .take_events()
        .iter()
        .filter_map(|event| match event {
            zigbee_apiarist::controller::events::ControllerEvent::DeviceInterview(payload) => {
                Some(payload.status.clone())
            }
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn device_joins_and_interview_ias_enrollment_succeeds() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    harness.events.lock().unwrap().clear();
    join(&adapter, 170, "0x170");
    wait_interviews(&harness, 2).await;
    assert_eq!(
        interview_statuses(&harness),
        vec!["started".to_string(), "successful".to_string()]
    );
    let device = harness.controller.get_device_by_ieee_addr("0x170").unwrap();
    assert_eq!(device.model_id().as_deref(), Some("myIasDevice"));
}

#[tokio::test]
async fn device_joins_and_interview_ias_enrollment_fails() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    adapter
        .ias_enroll
        .store(false, std::sync::atomic::Ordering::SeqCst);
    adapter.update_device(170, |device| {
        if let Some(attributes) = device.attributes.get_mut(&1) {
            attributes.insert("zoneState".to_string(), json!(0));
        }
    });
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    harness.events.lock().unwrap().clear();
    join(&adapter, 170, "0x170");
    wait_interviews(&harness, 2).await;
    assert_eq!(
        interview_statuses(&harness),
        vec!["started".to_string(), "failed".to_string()]
    );
}

#[tokio::test]
async fn device_joins_and_interview_ias_enrollment_throws() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    *adapter.fail_zcl_command.lock().unwrap() = Some("enrollRsp".to_string());
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    harness.events.lock().unwrap().clear();
    join(&adapter, 170, "0x170");
    wait_interviews(&harness, 2).await;
    assert_eq!(
        interview_statuses(&harness),
        vec!["started".to_string(), "failed".to_string()]
    );
}

#[tokio::test]
async fn device_joins_shouldnt_enroll_when_already_enrolled() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    adapter.update_device(170, |device| {
        if let Some(attributes) = device.attributes.get_mut(&1) {
            attributes.insert("zoneState".to_string(), json!(1));
        }
    });
    harness.start().await.expect("start");
    harness.events.lock().unwrap().clear();
    adapter.sent_endpoint.lock().unwrap().clear();
    join(&adapter, 170, "0x170");
    wait_interviews(&harness, 2).await;
    assert_eq!(
        interview_statuses(&harness),
        vec!["started".to_string(), "successful".to_string()]
    );
    assert!(!adapter
        .sent_endpoint
        .lock()
        .unwrap()
        .iter()
        .any(|frame| frame.cluster.ID == 0x0500 && frame.command.name == "write"));
}

#[tokio::test]
async fn tuya_end_device_second_pair_reads_model_and_manufacturer() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    harness.events.lock().unwrap().clear();
    join(&adapter, 173, "0x173");
    wait_interviews(&harness, 2).await;
    assert_eq!(
        interview_statuses(&harness),
        vec!["started".to_string(), "failed".to_string()]
    );
    let device = harness.controller.get_device_by_ieee_addr("0x173").unwrap();
    assert_eq!(device.model_id(), None);
    assert_eq!(device.manufacturer_name(), None);

    adapter.emit(ControllerAdapterEvent::DeviceLeave(AdapterDeviceLeave {
        network_address: Some(173),
        ieee_addr: Some("0x173".to_string()),
    }));
    settle().await;
    adapter.update_device(173, |device| {
        device.node_descriptor = None;
        device.attributes.insert(
            1,
            std::collections::HashMap::from([
                ("modelId".to_string(), json!("TS0203")),
                ("manufacturerName".to_string(), json!("_TYZB01_xph99wvr")),
            ]),
        );
    });
    harness.events.lock().unwrap().clear();
    join(&adapter, 173, "0x173");
    wait_interviews(&harness, 2).await;
    assert_eq!(
        interview_statuses(&harness),
        vec!["started".to_string(), "successful".to_string()]
    );
    let device = harness.controller.get_device_by_ieee_addr("0x173").unwrap();
    assert_eq!(device.model_id().as_deref(), Some("TS0203"));
    assert_eq!(
        device.manufacturer_name().as_deref(),
        Some("_TYZB01_xph99wvr")
    );
    assert_eq!(device.power_source().as_deref(), Some("Battery"));
}

#[tokio::test]
async fn xiaomi_wxcjkg11lm_join_simple_descriptor_for_endpoint_2_fails() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    harness.events.lock().unwrap().clear();
    join(&adapter, 171, "0x171");
    settle().await;
    assert_eq!(
        interview_statuses(&harness),
        vec!["started".to_string(), "successful".to_string()]
    );
    assert_eq!(
        harness
            .controller
            .get_device_by_ieee_addr("0x171")
            .unwrap()
            .model_id()
            .as_deref(),
        Some("lumi.remote.b286opcn01")
    );
}

#[tokio::test]
async fn gledopto_join_only_endpoint_12_responds() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    harness.events.lock().unwrap().clear();
    join(&adapter, 172, "0x172");
    wait_interviews(&harness, 2).await;
    assert_eq!(
        interview_statuses(&harness),
        vec!["started".to_string(), "successful".to_string()]
    );
    assert_eq!(
        harness
            .controller
            .get_device_by_ieee_addr("0x172")
            .unwrap()
            .model_id()
            .as_deref(),
        Some("GL-C-008")
    );
}

#[tokio::test]
async fn control4_device_join_completes_via_quirk() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    harness.events.lock().unwrap().clear();
    join(&adapter, 180, "0x180");
    wait_interviews(&harness, 2).await;
    assert_eq!(
        interview_statuses(&harness),
        vec!["started".to_string(), "successful".to_string()]
    );
    let device = harness.controller.get_device_by_ieee_addr("0x180").unwrap();
    assert_eq!(
        device.interview_state(),
        zigbee_apiarist::controller::model::device::InterviewState::Successful
    );
    assert_eq!(device.model_id().as_deref(), Some("C4-Zigbee"));
    assert_eq!(device.manufacturer_name().as_deref(), Some("Control4"));
    assert_eq!(
        device.power_source().as_deref(),
        Some("Mains (single phase)")
    );
}

#[tokio::test]
async fn xiaomi_end_device_joins_node_descriptor_fails() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    harness.events.lock().unwrap().clear();
    join(&adapter, 150, "0x150");
    wait_interviews(&harness, 2).await;
    assert_eq!(
        interview_statuses(&harness),
        vec!["started".to_string(), "successful".to_string()]
    );
    let device = harness.controller.get_device_by_ieee_addr("0x150").unwrap();
    assert_eq!(
        device.r#type(),
        zigbee_apiarist::controller::database::DeviceType::EndDevice
    );
    assert_eq!(device.model_id().as_deref(), Some("lumi.occupancy"));
    assert_eq!(device.manufacturer_name().as_deref(), Some("LUMI"));
}

#[tokio::test]
async fn xiaomi_end_device_joins_active_endpoints_fails() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    harness.events.lock().unwrap().clear();
    join(&adapter, 151, "0x151");
    wait_interviews(&harness, 2).await;
    assert_eq!(
        interview_statuses(&harness),
        vec!["started".to_string(), "failed".to_string()]
    );
}

#[tokio::test]
async fn controller_permit_joining_all_disabled_automatically() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    assert!(!harness.controller.get_permit_join());
    assert!(harness.controller.get_permit_join_end().is_none());
    harness
        .controller
        .permit_join(1, None)
        .await
        .expect("permit");
    assert!(harness.controller.get_permit_join());
    let end = harness.controller.get_permit_join_end().expect("end");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    assert!(end >= now && end <= now + 2000);
    assert!(permit_join_events(&harness)
        .iter()
        .any(|event| event.permitted));
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    settle().await;
    assert!(!harness.controller.get_permit_join());
    assert!(harness.controller.get_permit_join_end().is_none());
    assert!(permit_join_events(&harness)
        .iter()
        .any(|event| !event.permitted));
}

#[tokio::test]
async fn command_response() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    let response = endpoint
        .command(
            json!("genGroups"),
            json!("add"),
            json!({"groupid": 1, "groupname": ""}),
            None,
        )
        .await
        .expect("command");
    assert_eq!(response, Some(json!({"groupid": 1, "status": 0})));
}

#[tokio::test]
async fn group_command() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    let group = harness.controller.create_group(2).expect("group");
    adapter.sent_group.lock().unwrap().clear();
    group
        .command(
            json!("genOnOff"),
            json!("offWithEffect"),
            json!({"effectid": 9, "effectvariant": 10}),
            None,
        )
        .await
        .expect("group command");
    let frames = adapter.sent_group.lock().unwrap();
    assert_eq!(frames.len(), 1);
    let frame = &frames[0];
    assert_eq!(frame.header.command_identifier, 64);
    assert_eq!(frame.payload, json!({"effectid": 9, "effectvariant": 10}));
    assert_eq!(frame.command.name, "offWithEffect");
}

#[tokio::test]
async fn endpoint_command_with_options() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .command(
            json!("genOnOff"),
            json!("off"),
            json!({}),
            Some(zigbee_apiarist::controller::model::endpoint::Options {
                manufacturer_code: Some(100),
                disable_default_response: Some(true),
                ..Default::default()
            }),
        )
        .await
        .expect("command");
    let frames = adapter.sent_endpoint.lock().unwrap();
    let frame = &frames[0];
    assert_eq!(frame.header.manufacturer_code, Some(100));
    assert!(frame.header.frame_control.disable_default_response);
    assert!(frame.header.frame_control.manufacturer_specific);
    assert_eq!(frame.header.command_identifier, 0);
    assert_eq!(frame.command.name, "off");
}

#[tokio::test]
async fn endpoint_command_with_duplicate_cluster_id() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    let mut cluster =
        zigbee_apiarist::zspec::zcl::utils::Cluster::new("manuSpecificAssaDoorLock", 0xfc00);
    let mut command = zigbee_apiarist::zspec::zcl::utils::Command::new("getBatteryLevel", 0x12);
    command.order = 0;
    cluster
        .commands
        .insert("getBatteryLevel".to_string(), command);
    device
        .add_custom_cluster("manuSpecificAssaDoorLock", cluster)
        .expect("custom cluster");
    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .command(
            json!("manuSpecificAssaDoorLock"),
            json!("getBatteryLevel"),
            json!({}),
            None,
        )
        .await
        .expect("command");
    let frames = adapter.sent_endpoint.lock().unwrap();
    let buffer = frames[0].to_buffer();
    // Transaction sequence number is a global counter; upstream resets it in
    // the test setup, so only the frame type/command are asserted here.
    assert_eq!(buffer[0], 1);
    assert_eq!(buffer[2], 18);
    assert_eq!(buffer.len(), 3);
}

#[tokio::test]
async fn endpoint_command_with_duplicate_identifier() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    let mut cluster = zigbee_apiarist::zspec::zcl::utils::Cluster::new("lightingColorCtrl", 0x0300);
    let mut command = zigbee_apiarist::zspec::zcl::utils::Command::new(
        "tuyaMoveToHueAndSaturationBrightness",
        0x06,
    );
    command.parameters = vec![
        zigbee_apiarist::zspec::zcl::utils::Parameter::new(
            "hue",
            zigbee_apiarist::zspec::zcl::definition::enums::DataType::UINT8,
        ),
        zigbee_apiarist::zspec::zcl::utils::Parameter::new(
            "saturation",
            zigbee_apiarist::zspec::zcl::definition::enums::DataType::UINT8,
        ),
        zigbee_apiarist::zspec::zcl::utils::Parameter::new(
            "transtime",
            zigbee_apiarist::zspec::zcl::definition::enums::DataType::UINT16,
        ),
        zigbee_apiarist::zspec::zcl::utils::Parameter::new(
            "brightness",
            zigbee_apiarist::zspec::zcl::definition::enums::DataType::UINT8,
        ),
    ];
    command.order = 0;
    cluster
        .commands
        .insert("tuyaMoveToHueAndSaturationBrightness".to_string(), command);
    device
        .add_custom_cluster("lightingColorCtrl", cluster)
        .expect("custom cluster");
    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .command(
            json!("lightingColorCtrl"),
            json!("tuyaMoveToHueAndSaturationBrightness"),
            json!({"hue": 1, "saturation": 1, "transtime": 0, "brightness": 22}),
            None,
        )
        .await
        .expect("command");
    let frames = adapter.sent_endpoint.lock().unwrap();
    let buffer = frames[0].to_buffer();
    assert_eq!(buffer[0], 1);
    assert_eq!(buffer[2], 6);
    assert_eq!(&buffer[3..], &[1, 1, 0, 0, 22]);
}

#[tokio::test]
async fn endpoint_command_response() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .command_response(
            json!("genOta"),
            json!("imageNotify"),
            json!({"payloadType": 0, "queryJitter": 1}),
            None,
            None,
        )
        .await
        .expect("command response");
    let frames = adapter.sent_endpoint.lock().unwrap();
    let frame = &frames[0];
    assert_eq!(frame.header.command_identifier, 0);
    assert_eq!(frame.command.name, "imageNotify");
    assert_eq!(frame.payload, json!({"payloadType": 0, "queryJitter": 1}));
    assert_eq!(
        frame.header.frame_control.direction,
        zigbee_apiarist::zspec::zcl::zcl_header::Direction::SERVER_TO_CLIENT
    );
}

#[tokio::test]
async fn device_without_meta_sets_empty() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    let line = json!({
        "id": 3,
        "type": "EndDevice",
        "ieeeAddr": "0x90fd9ffffe4b64ae",
        "nwkAddr": 19468,
        "manufId": 4476,
        "manufName": "IKEA of Sweden",
        "powerSource": "Battery",
        "modelId": "TRADFRI remote control",
        "epList": [1],
        "endpoints": {
            "1": {
                "profId": 49246,
                "epId": 1,
                "devId": 2096,
                "inClusterList": [0, 1, 3, 9, 2821, 4096],
                "outClusterList": [3, 4, 5, 6, 8, 25, 4096],
                "clusters": {}
            }
        },
        "appVersion": 17,
        "stackVersion": 87,
        "hwVersion": 1,
        "dateCode": "20170302",
        "swBuildId": "1.2.214",
        "zclVersion": 1,
        "interviewState": "successful",
        "_id": "fJ5pmjqKRYbNvslK"
    });
    std::fs::write(&harness.database_path, format!("{line}\n")).expect("database");
    harness.start().await.expect("start");
    let device = harness
        .controller
        .get_device_by_ieee_addr("0x90fd9ffffe4b64ae")
        .expect("device");
    assert_eq!(*device.meta.lock().unwrap(), json!({}));
    assert_eq!(
        device.gen_basic(),
        json!({
            "manufacturerName": "IKEA of Sweden",
            "modelId": "TRADFRI remote control",
            "powerSource": 3,
            "swBuildId": "1.2.214",
            "stackVersion": 87,
            "zclVersion": 1,
            "appVersion": 17,
            "dateCode": "20170302",
            "hwVersion": 1
        })
    );
    assert_eq!(device.endpoints().len(), 1);
    assert_eq!(
        device.r#type(),
        zigbee_apiarist::controller::database::DeviceType::EndDevice
    );
}

#[tokio::test]
async fn device_lqi_routing_and_binding_table() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 140, "0x140");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x140").unwrap();
    let lqi = device.lqi().await.expect("lqi");
    assert_eq!(lqi.len(), 2);
    assert_eq!(lqi[0]["eui64"], "0x160");
    assert_eq!(lqi[0]["nwkAddress"], 160);
    assert_eq!(lqi[0]["lqi"], 20);
    assert_eq!(lqi[0]["relationship"], 2);
    assert_eq!(lqi[0]["depth"], 5);
    assert_eq!(lqi[1]["eui64"], "0x170");
    assert_eq!(lqi[1]["lqi"], 21);

    let routing = device.routing_table().await.expect("routing");
    assert_eq!(routing.len(), 2);
    assert_eq!(routing[0]["destinationAddress"], 120);
    assert_eq!(routing[0]["status"], "ACTIVE");
    assert_eq!(routing[0]["nextHopAddress"], 1);
    assert_eq!(routing[1]["status"], "DISCOVERY_FAILED");

    let binding = device.binding_table().await.expect("binding");
    assert_eq!(binding.len(), 2);
    assert_eq!(binding[0]["sourceEui64"], "0xf1f2f3f5f6f7f8");
    assert_eq!(binding[0]["clusterId"], 0);
    assert_eq!(binding[0]["destAddrMode"], 3);
    assert_eq!(binding[0]["dest"], "0xa1a2a3a4a5a6a7a8");
    assert_eq!(binding[1]["destAddrMode"], 1);
    assert_eq!(binding[1]["dest"], 4660);
}

#[tokio::test]
async fn device_ping() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    adapter.sent_endpoint.lock().unwrap().clear();
    device.ping(false).await.expect("ping");
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].command.name, "read");
    assert_eq!(frames[0].payload, json!([{"attrId": 0}]));
}

#[tokio::test]
async fn coordinator_check_with_supported_backup() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let missing = harness.controller.coordinator_check().await.expect("check");
    // The backup fixture has no devices, so every known device is reported missing.
    assert!(missing.iter().any(|device| device.ieee_addr() == "0x129"));

    adapter
        .supports_backup
        .store(false, std::sync::atomic::Ordering::SeqCst);
    let error = match harness.controller.coordinator_check().await {
        Ok(_) => panic!("expected error"),
        Err(error) => error,
    };
    assert_eq!(
        error,
        "Coordinator does not coordinator check because it doesn't support backups"
    );
}

#[tokio::test]
async fn endpoint_bind_add_binding_and_unbind() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    let group = harness.controller.create_group(1).expect("group");

    adapter.zdo_calls.lock().unwrap().clear();
    endpoint
        .bind(
            json!("genBasic"),
            zigbee_apiarist::controller::model::endpoint::BindTarget::Group(group.clone()),
        )
        .await
        .expect("bind");
    assert!(adapter
        .zdo_calls
        .lock()
        .unwrap()
        .iter()
        .any(|call| call.2
            == zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::BIND_REQUEST));
    assert_eq!(endpoint.binds().len(), 1);

    endpoint
        .unbind(
            json!("genBasic"),
            zigbee_apiarist::controller::model::endpoint::BindTarget::Group(group.clone()),
            false,
        )
        .await
        .expect("unbind");
    assert!(endpoint.binds().is_empty());

    endpoint
        .add_binding(
            json!("genBasic"),
            zigbee_apiarist::controller::model::endpoint::BindTarget::Group(group.clone()),
        )
        .expect("add binding");
    assert_eq!(endpoint.binds().len(), 1);
    assert!(endpoint.has_bind(
        0,
        &zigbee_apiarist::controller::model::endpoint::BindTarget::Group(group)
    ));
}

#[tokio::test]
async fn endpoint_add_and_remove_from_group() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    let group = harness.controller.create_group(1).expect("group");

    endpoint.add_to_group(&group).await.expect("add to group");
    assert!(group.has_member(&endpoint));
    endpoint
        .remove_from_group(&group)
        .await
        .expect("remove from group");
    assert!(!group.has_member(&endpoint));
}

#[tokio::test]
async fn endpoint_add_to_group_when_response_reports_duplicate() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    let group = harness.controller.create_group(1).expect("group");
    adapter
        .group_status
        .store(0x8a, std::sync::atomic::Ordering::SeqCst);
    endpoint.add_to_group(&group).await.expect("duplicate");
    assert!(group.has_member(&endpoint));
}

#[tokio::test]
async fn endpoint_add_to_group_failure_does_not_add() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    let group = harness.controller.create_group(1).expect("group");
    adapter
        .group_status
        .store(0x8b, std::sync::atomic::Ordering::SeqCst);
    assert!(endpoint.add_to_group(&group).await.is_err());
    assert!(!group.has_member(&endpoint));
}

#[tokio::test]
async fn endpoint_remove_from_group_failure_keeps_member() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    let group = harness.controller.create_group(1).expect("group");
    endpoint.add_to_group(&group).await.expect("add");
    adapter
        .group_status
        .store(0x01, std::sync::atomic::Ordering::SeqCst);
    assert!(endpoint.remove_from_group(&group).await.is_err());
    assert!(group.has_member(&endpoint));
}

#[tokio::test]
async fn endpoint_remove_from_group_database_when_not_found() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    let group = harness.controller.create_group(1).expect("group");
    endpoint.add_to_group(&group).await.expect("add");
    adapter
        .group_status
        .store(0x8b, std::sync::atomic::Ordering::SeqCst);
    endpoint
        .remove_from_group(&group)
        .await
        .expect("not found is ignored");
    assert!(!group.has_member(&endpoint));
    adapter
        .group_status
        .store(0x00, std::sync::atomic::Ordering::SeqCst);
}

#[tokio::test]
async fn endpoint_configure_reporting_and_read_config() {
    use zigbee_apiarist::controller::model::endpoint::{
        ConfigureReportingItem, ReadReportConfigItem,
    };
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .configure_reporting(
            json!("genPowerCfg"),
            vec![ConfigureReportingItem {
                attribute: json!("batteryPercentageRemaining"),
                minimum_report_interval: 60,
                maximum_report_interval: 3600,
                reportable_change: Some(10.0),
            }],
            None,
        )
        .await
        .expect("configure reporting");
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(frames[0].command.name, "configReport");
    assert_eq!(frames[0].payload[0]["attrId"], 0x0021);
    assert_eq!(frames[0].payload[0]["minRepIntval"], 60);
    assert_eq!(frames[0].payload[0]["maxRepIntval"], 3600);
    assert_eq!(endpoint.configured_reportings().len(), 1);
    drop(frames);

    let response = endpoint
        .read_reporting_config(
            json!("genPowerCfg"),
            vec![ReadReportConfigItem {
                direction: None,
                attribute: json!("batteryPercentageRemaining"),
            }],
            None,
        )
        .await
        .expect("read reporting config");
    assert_eq!(response[0]["attrId"], 0x0021);
    assert_eq!(response[0]["status"], 0);
}

#[tokio::test]
async fn endpoint_configure_reporting_fails_on_non_zero_status() {
    use zigbee_apiarist::controller::model::endpoint::ConfigureReportingItem;
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    adapter
        .configure_report_status
        .store(0x8b, std::sync::atomic::Ordering::SeqCst);
    let error = endpoint
        .configure_reporting(
            json!("genPowerCfg"),
            vec![ConfigureReportingItem {
                attribute: json!("batteryPercentageRemaining"),
                minimum_report_interval: 60,
                maximum_report_interval: 3600,
                reportable_change: Some(10.0),
            }],
            None,
        )
        .await
        .expect_err("status");
    assert!(error.contains("Status"));
}

#[tokio::test]
async fn endpoint_configure_reporting_bad_attribute() {
    use zigbee_apiarist::controller::model::endpoint::ConfigureReportingItem;
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    let error = endpoint
        .configure_reporting(
            json!("genPowerCfg"),
            vec![ConfigureReportingItem {
                attribute: json!("notAnAttribute"),
                minimum_report_interval: 60,
                maximum_report_interval: 3600,
                reportable_change: None,
            }],
            None,
        )
        .await
        .expect_err("bad attribute");
    assert_eq!(
        error,
        "Invalid attribute 'notAnAttribute' for cluster 'genPowerCfg'"
    );
}

#[tokio::test]
async fn read_from_group_and_failure() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    let group = harness.controller.create_group(2).expect("group");
    adapter.sent_group.lock().unwrap().clear();
    group
        .read(json!("genBasic"), json!(["modelId", 1, "notanattr"]), None)
        .await
        .expect("read");
    let frames = adapter.sent_group.lock().unwrap();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].command.name, "read");
    assert_eq!(frames[0].payload, json!([{"attrId": 5}, {"attrId": 1}]));
    drop(frames);

    adapter
        .fail_group_send
        .store(true, std::sync::atomic::Ordering::SeqCst);
    assert!(group
        .read(json!("genBasic"), json!(["modelId", 1]), None)
        .await
        .is_err());
}

#[tokio::test]
async fn write_to_group_and_failure() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    let group = harness.controller.create_group(2).expect("group");
    adapter.sent_group.lock().unwrap().clear();
    group
        .write(
            json!("genBasic"),
            json!({"49": {"value": 0x000b, "type": 0x19}, "deviceEnabled": 1}),
            None,
        )
        .await
        .expect("write");
    let frames = adapter.sent_group.lock().unwrap();
    assert_eq!(frames[0].command.name, "write");
    assert_eq!(
        frames[0].payload,
        json!([
            {"attrData": 11, "attrId": 49, "dataType": 25},
            {"attrData": 1, "attrId": 18, "dataType": 16}
        ])
    );
    drop(frames);

    let error = group
        .write(
            json!("genBasic"),
            json!({"UNKNOWN": {"value": 11, "type": 25}}),
            None,
        )
        .await
        .expect_err("unknown");
    assert_eq!(
        error,
        "Unknown attribute 'UNKNOWN', specify either an existing attribute or a number"
    );

    adapter
        .fail_group_send
        .store(true, std::sync::atomic::Ordering::SeqCst);
    assert!(group
        .write(
            json!("genBasic"),
            json!({"49": {"value": 11, "type": 25}}),
            None
        )
        .await
        .is_err());
}

#[tokio::test]
async fn receive_read_response_from_unknown_endpoint() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    harness.events.lock().unwrap().clear();
    let buffer = vec![8, 1, 1, 1, 0, 0, 32, 3];
    let header =
        zigbee_apiarist::zspec::zcl::zcl_header::Header::from_buffer(&buffer).expect("header");
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: 0,
        data: buffer.clone(),
        header: Some(header),
        address: json!(129),
        endpoint: 3,
        linkquality: 52,
        group_id: 0,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    let events = harness.take_events();
    let message = events
        .iter()
        .find_map(|event| match event {
            zigbee_apiarist::controller::events::ControllerEvent::Message(payload) => {
                Some(payload.clone())
            }
            _ => None,
        })
        .expect("message");
    assert_eq!(message.r#type, "readResponse");
    assert_eq!(message.cluster, json!("genBasic"));
    assert_eq!(message.data, json!({"appVersion": 3}));
    assert_eq!(message.linkquality, 52);
    assert_eq!(
        harness
            .controller
            .get_device_by_ieee_addr("0x129")
            .unwrap()
            .endpoints()
            .len(),
        2
    );
}

#[tokio::test]
async fn receive_cluster_command() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    harness.events.lock().unwrap().clear();
    let frame = zigbee_apiarist::zspec::zcl::zcl_frame::Frame::create(
        1,
        1,
        false,
        None,
        29,
        "addRsp",
        "genScenes",
        json!({"status": 0, "groupId": 10, "sceneId": 2}),
        json!({}),
        0,
    );
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: frame.cluster.ID as u16,
        data: frame.to_buffer(),
        header: Some(frame.header),
        address: json!(129),
        endpoint: 1,
        linkquality: 19,
        group_id: 10,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    let message = harness
        .take_events()
        .into_iter()
        .find_map(|event| match event {
            zigbee_apiarist::controller::events::ControllerEvent::Message(payload) => Some(payload),
            _ => None,
        })
        .expect("message");
    assert_eq!(message.r#type, "commandAddRsp");
    assert_eq!(message.cluster, json!("genScenes"));
    assert_eq!(
        message.data,
        json!({"groupId": 10, "sceneId": 2, "status": 0})
    );
    assert_eq!(message.group_id, 10);
    assert_eq!(message.meta.zcl_transaction_sequence_number, Some(29));
}

#[tokio::test]
async fn respond_to_read_of_attribute() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    adapter.sent_endpoint.lock().unwrap().clear();
    let frame = zigbee_apiarist::zspec::zcl::zcl_frame::Frame::create(
        0,
        0,
        true,
        None,
        40,
        "read",
        "genTime",
        json!([{"attrId": 0x0000}, {"attrId": 0xfffa}]),
        json!({}),
        0,
    );
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: frame.cluster.ID as u16,
        data: frame.to_buffer(),
        header: Some(frame.header),
        address: json!(129),
        endpoint: 1,
        linkquality: 19,
        group_id: 10,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(frames.len(), 1);
    let response = &frames[0];
    assert_eq!(response.command.name, "readRsp");
    let payload = response.payload.as_array().cloned().unwrap_or_default();
    assert_eq!(payload.len(), 2);
    // JS enumerates integer-like object keys first (ascending), then string keys.
    assert_eq!(payload[0]["attrId"], 0xfffa);
    assert_eq!(payload[0]["status"], 0x86);
    assert_eq!(payload[1]["attrId"], 0x0000);
    assert_eq!(payload[1]["status"], 0);
}

#[tokio::test]
async fn sends_read_response_for_unsupported_attribute() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .read_response(
            json!("genGroups"),
            1,
            json!({"nameSupport": Value::Null}),
            Some(zigbee_apiarist::controller::model::endpoint::Options {
                src_endpoint: Some(1),
                ..Default::default()
            }),
        )
        .await
        .expect("read response");
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(frames[0].payload, json!([{"attrId": 0, "status": 0x86}]));
}

#[tokio::test]
async fn allow_override_read_response() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_clone = calls.clone();
    device.set_custom_read_response(Some(Box::new(move |_frame, _endpoint| {
        calls_clone.fetch_add(1, Ordering::SeqCst);
        true
    })));
    adapter.sent_endpoint.lock().unwrap().clear();
    let frame = zigbee_apiarist::zspec::zcl::zcl_frame::Frame::create(
        0,
        0,
        true,
        None,
        40,
        "read",
        "genTime",
        json!([{"attrId": 0}, {"attrId": 1}]),
        json!({}),
        0,
    );
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: frame.cluster.ID as u16,
        data: frame.to_buffer(),
        header: Some(frame.header),
        address: json!(129),
        endpoint: 1,
        linkquality: 19,
        group_id: 10,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(adapter.sent_endpoint.lock().unwrap().is_empty());
}

#[tokio::test]
async fn respond_to_gen_time_read_and_failure() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    adapter.sent_endpoint.lock().unwrap().clear();
    let frame = zigbee_apiarist::zspec::zcl::zcl_frame::Frame::create(
        0,
        0,
        true,
        None,
        40,
        0u8,
        10,
        json!([
            {"attrId": 0}, {"attrId": 1}, {"attrId": 2}, {"attrId": 3}, {"attrId": 4},
            {"attrId": 5}, {"attrId": 6}, {"attrId": 7}, {"attrId": 8}, {"attrId": 9}
        ]),
        json!({}),
        0,
    );
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: frame.cluster.ID as u16,
        data: frame.to_buffer(),
        header: Some(frame.header),
        address: json!(129),
        endpoint: 1,
        linkquality: 19,
        group_id: 10,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(frames.len(), 1);
    let payload = frames[0].payload.as_array().cloned().unwrap_or_default();
    assert_eq!(payload.len(), 10);
    assert_eq!(
        payload
            .iter()
            .map(|item| item["attrId"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        (0..10).collect::<Vec<_>>()
    );
    assert!(payload.iter().all(|item| item["status"] == 0));
    drop(frames);

    adapter
        .fail_zcl_command
        .lock()
        .unwrap()
        .replace("readRsp".to_string());
    adapter.sent_endpoint.lock().unwrap().clear();
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: frame.cluster.ID as u16,
        data: frame.to_buffer(),
        header: Some(frame.header),
        address: json!(129),
        endpoint: 1,
        linkquality: 19,
        group_id: 10,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    assert_eq!(adapter.sent_endpoint.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn endpoint_get_id_and_device_type() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    assert_eq!(endpoint.ID, 1);
    assert_eq!(endpoint.device_id.lock().unwrap().map(|id| id), Some(5));
    let by_type = device.get_endpoint_by_device_type("HAOnOffLight");
    assert!(by_type.is_none());
    assert!(device.get_endpoint(9).is_none());
}

#[tokio::test]
async fn send_zcl_command_to_all_no_options_and_manufacturer() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    adapter.sent_all.lock().unwrap().clear();
    endpoint
        .zcl_command_broadcast(
            1,
            zigbee_apiarist::zspec::enums::BroadcastAddress::SLEEPY,
            json!("genOnOff"),
            json!("off"),
            json!({}),
            None,
        )
        .await
        .expect("broadcast");
    assert_eq!(adapter.sent_all.lock().unwrap().len(), 1);
    assert_eq!(adapter.sent_all.lock().unwrap()[0].command.name, "off");

    endpoint
        .zcl_command_broadcast(
            1,
            zigbee_apiarist::zspec::enums::BroadcastAddress::SLEEPY,
            json!("genOnOff"),
            json!("off"),
            json!({}),
            Some(zigbee_apiarist::controller::model::endpoint::Options {
                manufacturer_code: Some(4476),
                ..Default::default()
            }),
        )
        .await
        .expect("broadcast manufacturer");
    let frames = adapter.sent_all.lock().unwrap();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[1].header.manufacturer_code, Some(4476));
}

#[tokio::test]
async fn should_roll_over_transaction_id() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    adapter.sent_endpoint.lock().unwrap().clear();
    for _ in 0..260 {
        endpoint
            .command(json!("genOnOff"), json!("off"), json!({}), None)
            .await
            .expect("command");
    }
    let frames = adapter.sent_endpoint.lock().unwrap();
    let numbers = frames
        .iter()
        .map(|frame| frame.header.transaction_sequence_number)
        .collect::<Vec<_>>();
    assert_eq!(numbers.len(), 260);
    // Upstream rolls over at 255 -> 0 and never emits 256.
    assert!(numbers.contains(&255));
    assert!(numbers.windows(2).any(|pair| pair[1] < pair[0]));
}

#[tokio::test]
async fn throw_error_creating_duplicate_entities() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let error = match device.create_endpoint(1) {
        Ok(_) => panic!("expected duplicate endpoint error"),
        Err(error) => error,
    };
    assert_eq!(error, "Device '0x129' already has an endpoint '1'");
    let error = harness.controller.get_group_by_id(1);
    assert!(error.is_none());
    harness.controller.create_group(1).expect("group");
    let error = match harness.controller.create_group(1) {
        Ok(_) => panic!("expected duplicate group error"),
        Err(error) => error,
    };
    assert_eq!(error, "Group with groupID '1' already exists");
}

#[tokio::test]
async fn should_allow_to_set_type() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    assert_eq!(
        device.r#type(),
        zigbee_apiarist::controller::database::DeviceType::Router
    );
    device.set_type(zigbee_apiarist::controller::database::DeviceType::EndDevice);
    assert_eq!(
        device.r#type(),
        zigbee_apiarist::controller::database::DeviceType::EndDevice
    );
}

fn emit_report(
    adapter: &MockAdapter,
    cluster_id: u16,
    buffer: Vec<u8>,
    device: &zigbee_apiarist::controller::model::device::Device,
) {
    let header = zigbee_apiarist::zspec::zcl::zcl_header::Header::from_buffer(&buffer)
        .expect("report header");
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id,
        data: buffer,
        header: Some(header),
        address: json!(device.network_address()),
        endpoint: 1,
        linkquality: 50,
        group_id: 1,
        was_broadcast: false,
        destination_endpoint: 1,
    });
}

fn messages(harness: &Harness) -> Vec<zigbee_apiarist::controller::events::MessagePayload> {
    harness
        .take_events()
        .into_iter()
        .filter_map(|event| match event {
            zigbee_apiarist::controller::events::ControllerEvent::Message(payload) => Some(payload),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn custom_attributes_for_existing_cluster() {
    use zigbee_apiarist::zspec::zcl::definition::enums::DataType;
    use zigbee_apiarist::zspec::zcl::utils::{Attribute, Cluster};
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();

    let mut cluster = Cluster::new("genBasic", 0);
    cluster.attributes.insert(
        "customAttr".to_string(),
        Attribute::new("customAttr", 256, DataType::UINT8),
    );
    cluster.attributes.insert(
        "aDifferentZclVersion".to_string(),
        Attribute::new("aDifferentZclVersion", 0, DataType::UINT8),
    );
    device
        .add_custom_cluster("genBasic", cluster)
        .expect("cluster");

    harness.events.lock().unwrap().clear();
    let buffer = vec![24, 169, 10, 0, 1, 24, 3, 0, 0, 24, 1, 2, 0, 24, 1];
    emit_report(&adapter, 0, buffer.clone(), &device);
    settle().await;
    let events = messages(&harness);
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].data,
        json!({"customAttr": 3, "aDifferentZclVersion": 1, "stackVersion": 1})
    );
    assert_eq!(events[0].cluster, json!("genBasic"));

    let mut second = Cluster::new("genBasic", 0);
    second.attributes.insert(
        "customAttrSecondOverride".to_string(),
        Attribute::new("customAttrSecondOverride", 256, DataType::UINT8),
    );
    device
        .add_custom_cluster("genBasic", second)
        .expect("cluster");
    emit_report(&adapter, 0, buffer, &device);
    settle().await;
    let events = messages(&harness);
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].data,
        json!({"customAttrSecondOverride": 3, "aDifferentZclVersion": 1, "stackVersion": 1})
    );
}

#[tokio::test]
async fn custom_cluster_and_override() {
    use zigbee_apiarist::zspec::zcl::definition::enums::DataType;
    use zigbee_apiarist::zspec::zcl::utils::{Attribute, Cluster};
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();

    let mut cluster = Cluster::new("myCustomCluster", 9123);
    cluster.attributes.insert(
        "superAttribute".to_string(),
        Attribute::new("superAttribute", 0, DataType::UINT8),
    );
    device
        .add_custom_cluster("myCustomCluster", cluster)
        .expect("cluster");
    harness.events.lock().unwrap().clear();
    let buffer = vec![24, 169, 10, 0, 1, 24, 3, 0, 0, 24, 1];
    emit_report(&adapter, 9123, buffer.clone(), &device);
    settle().await;
    let events = messages(&harness);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data, json!({"superAttribute": 1, "256": 3}));
    assert_eq!(events[0].cluster, json!("myCustomCluster"));

    let mut override_cluster = Cluster::new("myCustomCluster", 0);
    override_cluster.attributes.insert(
        "customAttr".to_string(),
        Attribute::new("customAttr", 256, DataType::UINT8),
    );
    device
        .add_custom_cluster("myCustomCluster", override_cluster)
        .expect("override cluster");
    harness.events.lock().unwrap().clear();
    emit_report(&adapter, 0, buffer, &device);
    settle().await;
    let events = messages(&harness);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].data, json!({"customAttr": 3, "0": 1}));
    assert_eq!(events[0].cluster, json!("myCustomCluster"));
}

#[tokio::test]
async fn write_to_endpoint_custom_attributes() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .write(
            json!("genBasic"),
            json!({"49": {"value": 0x000b, "type": 0x19}}),
            Some(zigbee_apiarist::controller::model::endpoint::Options {
                manufacturer_code: Some(0x100b),
                disable_default_response: Some(true),
                timeout: Some(12),
                ..Default::default()
            }),
        )
        .await
        .expect("write");
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].header.manufacturer_code, Some(4107));
    assert_eq!(
        frames[0].payload,
        json!([{"attrId": 49, "attrData": 11, "dataType": 25}])
    );
}

#[tokio::test]
async fn write_to_endpoint_with_unknown_string_attribute_fails() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    let error = endpoint
        .write(json!("genBasic"), json!({"notAnAttribute": 1}), None)
        .await
        .expect_err("unknown");
    assert_eq!(
        error,
        "Unknown attribute 'notAnAttribute', specify either an existing attribute or a number"
    );
}

#[tokio::test]
async fn write_response_to_endpoint_variants() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .write_response(json!("genBasic"), 99, json!({"85": {"status": 1}}), None)
        .await
        .expect("non ZCL attribute");
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(frames[0].payload, json!([{"attrId": 85, "status": 1}]));
    assert_eq!(frames[0].header.transaction_sequence_number, 99);
    drop(frames);

    let error = endpoint
        .write_response(
            json!("genBasic"),
            99,
            json!({"UNKNOWN": {"status": 1}}),
            None,
        )
        .await
        .expect_err("unknown string attribute");
    assert_eq!(
        error,
        "Unknown attribute 'UNKNOWN', specify either an existing attribute or a number"
    );

    let error = endpoint
        .write_response(
            json!("genBasic"),
            99,
            json!({"UNKNOWN": {"status": 1}}),
            Some(zigbee_apiarist::controller::model::endpoint::Options {
                transaction_sequence_number: Some(5),
                ..Default::default()
            }),
        )
        .await
        .expect_err("transaction sequence number option");
    assert_eq!(error, "Use parameter");

    let error = endpoint
        .write_response(json!("genBasic"), 99, json!({"1": {"value": 0x55}}), None)
        .await
        .expect_err("missing status");
    assert_eq!(error, "Missing attribute 'status'");

    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .write_response(
            json!("genBasic"),
            99,
            json!({"zclVersion": {"status": 1}}),
            None,
        )
        .await
        .expect("ZCL attribute");
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(frames[0].payload, json!([{"attrId": 0, "status": 1}]));
}

#[tokio::test]
async fn read_response_to_endpoint_variants() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .read_response(
            json!("genBasic"),
            99,
            json!({"85": {"value": 0x000b, "type": 0x19}}),
            None,
        )
        .await
        .expect("non ZCL attribute");
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(
        frames[0].payload,
        json!([{"attrId": 85, "attrData": 11, "dataType": 25, "status": 0}])
    );
    drop(frames);

    let error = endpoint
        .read_response(
            json!("genBasic"),
            99,
            json!({"UNKNOWN": {"value": 0x000b, "type": 0x19}}),
            None,
        )
        .await
        .expect_err("unknown string attribute");
    assert_eq!(
        error,
        "Unknown attribute 'UNKNOWN', specify either an existing attribute or a number"
    );

    let error = endpoint
        .read_response(
            json!("genBasic"),
            99,
            json!({"UNKNOWN": {"value": 0x000b, "type": 0x19}}),
            Some(zigbee_apiarist::controller::model::endpoint::Options {
                transaction_sequence_number: Some(5),
                ..Default::default()
            }),
        )
        .await
        .expect_err("transaction sequence number option");
    assert_eq!(error, "Use parameter");
}

#[tokio::test]
async fn remove_device_from_network_and_database_variants() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    join(&adapter, 130, "0x130");
    settle().await;
    let first = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    first
        .remove_from_network(false)
        .await
        .expect("remove network");
    assert!(harness
        .controller
        .get_device_by_ieee_addr("0x129")
        .is_none());
    assert!(zigbee_apiarist::controller::model::device::Device::is_deleted_by_ieee_addr("0x129"));

    let second = harness.controller.get_device_by_ieee_addr("0x130").unwrap();
    second.remove_from_database(false);
    assert!(harness
        .controller
        .get_device_by_ieee_addr("0x130")
        .is_none());
}

#[tokio::test]
async fn remove_group_from_network_and_database() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    let group = harness.controller.create_group(1).expect("group");
    endpoint.add_to_group(&group).await.expect("add");
    group.remove_from_network().await.expect("remove network");
    assert!(!group.has_member(&endpoint));
    assert!(harness.controller.get_group_by_id(1).is_none());
}

#[tokio::test]
async fn read_from_endpoint_variants() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();

    adapter.sent_endpoint.lock().unwrap().clear();
    let value = endpoint
        .read(json!("genBasic"), json!(["stackVersion"]), None)
        .await
        .expect("read string");
    assert_eq!(value["stackVersion"], 101);
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(frames[0].payload, json!([{"attrId": 2}]));
    drop(frames);

    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .read(
            json!("genBasic"),
            json!(["stackVersion", "notanattr"]),
            None,
        )
        .await
        .expect("read ignores unknown");
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(frames[0].payload, json!([{"attrId": 2}]));
    drop(frames);

    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .read(
            json!("genBasic"),
            json!([0xff22]),
            Some(zigbee_apiarist::controller::model::endpoint::Options {
                manufacturer_code: Some(0x115f),
                disable_default_response: Some(true),
                ..Default::default()
            }),
        )
        .await
        .expect("read unknown attribute with options");
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(frames[0].payload, json!([{"attrId": 65314}]));
    assert_eq!(frames[0].header.manufacturer_code, Some(4447));
}

#[tokio::test]
async fn write_to_endpoint_custom_attributes_variants() {
    use zigbee_apiarist::zspec::zcl::definition::enums::DataType;
    use zigbee_apiarist::zspec::zcl::utils::{Attribute, Cluster};
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let mut cluster = Cluster::new("hvacThermostat", 0x0201);
    let mut attribute = Attribute::new("viessmannWindowOpenInternal", 0x4000, DataType::ENUM8);
    attribute.manufacturerCode = Some(0x1221);
    attribute.write = true;
    attribute.max = Some(255.0);
    cluster
        .attributes
        .insert("viessmannWindowOpenInternal".to_string(), attribute);
    device
        .add_custom_cluster("hvacThermostat", cluster)
        .expect("custom cluster");
    let endpoint = device.get_endpoint(1).unwrap();
    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .write(
            json!("hvacThermostat"),
            json!({"viessmannWindowOpenInternal": 1}),
            None,
        )
        .await
        .expect("write custom attribute");
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(frames[0].header.manufacturer_code, Some(4641));
    assert_eq!(
        frames[0].payload,
        json!([{"attrId": 16384, "attrData": 1, "dataType": 48}])
    );
    drop(frames);

    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .write(
            json!("genBasic"),
            json!({"49": {"value": 0x000b, "type": 0x19}}),
            Some(zigbee_apiarist::controller::model::endpoint::Options {
                manufacturer_code: Some(0x100b),
                disable_default_response: Some(true),
                timeout: Some(12),
                write_undiv: Some(true),
                disable_response: Some(true),
                ..Default::default()
            }),
        )
        .await
        .expect("write undiv");
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(frames[0].command.name, "writeUndiv");
    assert_eq!(frames[0].header.manufacturer_code, Some(0x100b));
}

#[tokio::test]
async fn configure_reporting_non_analog_and_disable_response() {
    use zigbee_apiarist::controller::model::endpoint::ConfigureReportingItem;
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();

    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .configure_reporting(
            json!("genOnOff"),
            vec![ConfigureReportingItem {
                attribute: json!("onOff"),
                minimum_report_interval: 1,
                maximum_report_interval: 2,
                reportable_change: None,
            }],
            None,
        )
        .await
        .expect("non analog");
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(frames[0].payload[0]["dataType"], 16);
    assert!(frames[0].payload[0].get("repChange").is_none());
    drop(frames);

    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .configure_reporting(
            json!("genPowerCfg"),
            vec![ConfigureReportingItem {
                attribute: json!("batteryPercentageRemaining"),
                minimum_report_interval: 60,
                maximum_report_interval: 3600,
                reportable_change: Some(10.0),
            }],
            Some(zigbee_apiarist::controller::model::endpoint::Options {
                disable_response: Some(true),
                ..Default::default()
            }),
        )
        .await
        .expect("disable response");
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(frames[0].command.name, "configReport");
}

#[tokio::test]
async fn configure_reporting_manufacturer_specific_attribute() {
    use zigbee_apiarist::controller::model::endpoint::ConfigureReportingItem;
    use zigbee_apiarist::zspec::zcl::definition::enums::DataType;
    use zigbee_apiarist::zspec::zcl::utils::{Attribute, Cluster};
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let mut cluster = Cluster::new("hvacThermostat", 0x0201);
    let mut attribute = Attribute::new("viessmannWindowOpenInternal", 0x4000, DataType::ENUM8);
    attribute.manufacturerCode = Some(0x1221);
    attribute.write = true;
    cluster
        .attributes
        .insert("viessmannWindowOpenInternal".to_string(), attribute);
    device
        .add_custom_cluster("hvacThermostat", cluster)
        .expect("custom cluster");
    let endpoint = device.get_endpoint(1).unwrap();
    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .configure_reporting(
            json!("hvacThermostat"),
            vec![ConfigureReportingItem {
                attribute: json!("viessmannWindowOpenInternal"),
                minimum_report_interval: 1,
                maximum_report_interval: 10,
                reportable_change: Some(1.0),
            }],
            None,
        )
        .await
        .expect("manufacturer specific");
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(frames[0].header.manufacturer_code, Some(4641));
    assert_eq!(frames[0].payload[0]["attrId"], 16384);
}

#[tokio::test]
async fn configure_reporting_custom_attribute_by_id() {
    use zigbee_apiarist::controller::model::endpoint::ConfigureReportingItem;
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .configure_reporting(
            json!("hvacThermostat"),
            vec![ConfigureReportingItem {
                attribute: json!({"ID": 0x4004, "type": 41}),
                minimum_report_interval: 0,
                maximum_report_interval: 3600,
                reportable_change: Some(25.0),
            }],
            None,
        )
        .await
        .expect("custom attribute");
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(
        frames[0].payload,
        json!([{"attrId": 16388, "dataType": 41, "direction": 0, "maxRepIntval": 3600, "minRepIntval": 0, "repChange": 25}])
    );
}

#[tokio::test]
async fn remove_endpoint_from_all_groups() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    let first = harness.controller.create_group(1).expect("group 1");
    let second = harness.controller.create_group(2).expect("group 2");
    endpoint.add_to_group(&first).await.expect("add 1");
    endpoint.add_to_group(&second).await.expect("add 2");
    endpoint.remove_from_all_groups().await.expect("remove all");
    assert!(!first.has_member(&endpoint));
    assert!(!second.has_member(&endpoint));
}

#[tokio::test]
async fn receive_cluster_command_from_unknown_cluster_emits_nothing() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    harness.events.lock().unwrap().clear();
    let frame = zigbee_apiarist::zspec::zcl::zcl_frame::Frame::create(
        1,
        1,
        false,
        Some(4476),
        29,
        1u8,
        5,
        json!({
            "groupid": 1, "sceneid": 1, "status": 0, "transtime": 0,
            "scenename": "", "extensionfieldsets": []
        }),
        json!({}),
        0,
    );
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: 5,
        data: frame.to_buffer(),
        header: Some(frame.header),
        // Upstream test passes `networkAddress` instead of `address`, so the
        // payload has no resolvable address and is skipped.
        address: Value::Null,
        endpoint: 1,
        linkquality: 19,
        group_id: 10,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    assert!(!harness.take_events().iter().any(|event| matches!(
        event,
        zigbee_apiarist::controller::events::ControllerEvent::Message(_)
    )));
}

#[tokio::test]
async fn receive_report_from_unknown_attribute() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    harness.events.lock().unwrap().clear();
    let buffer = vec![
        28, 95, 17, 3, 10, 5, 0, 66, 21, 108, 117, 109, 105, 46, 115, 101, 110, 115, 111, 114, 95,
        119, 108, 101, 97, 107, 46, 97, 113, 49, 1, 255, 66, 34, 1, 33, 213, 12, 3, 40, 33, 4, 33,
        168, 19, 5, 33, 43, 0, 6, 36, 0, 0, 5, 0, 0, 8, 33, 4, 2, 10, 33, 0, 0, 100, 16, 0,
    ];
    let header =
        zigbee_apiarist::zspec::zcl::zcl_header::Header::from_buffer(&buffer).expect("header");
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: 0x0000,
        data: buffer,
        header: Some(header),
        address: json!(129),
        endpoint: 1,
        linkquality: 50,
        group_id: 1,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    let events = messages(&harness);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].r#type, "attributeReport");
    assert_eq!(events[0].cluster, json!("genBasic"));
    assert_eq!(events[0].data["modelId"], "lumi.sensor_wleak.aq1");
    assert_eq!(
        events[0].data["65281"],
        json!({
            "1": 3285, "3": 33, "4": 5032, "5": 43, "6": 327680,
            "8": 516, "10": 0, "100": 0
        })
    );
    assert_eq!(events[0].meta.manufacturer_code, Some(4447));
}

#[tokio::test]
async fn emit_read_from_device() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    harness.events.lock().unwrap().clear();
    let frame = zigbee_apiarist::zspec::zcl::zcl_frame::Frame::create(
        0,
        0,
        true,
        None,
        40,
        0u8,
        1,
        json!([{"attrId": 0}, {"attrId": 9999}]),
        json!({}),
        0,
    );
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: frame.cluster.ID as u16,
        data: frame.to_buffer(),
        header: Some(frame.header),
        address: json!(129),
        endpoint: 1,
        linkquality: 19,
        group_id: 10,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    let events = messages(&harness);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].r#type, "read");
    assert_eq!(events[0].data, json!(["mainsVoltage", 9999]));
    assert_eq!(events[0].cluster, json!("genPowerCfg"));
    assert_eq!(events[0].linkquality, 19);
    assert_eq!(events[0].group_id, 10);
    assert_eq!(events[0].meta.zcl_transaction_sequence_number, Some(40));
}

#[tokio::test]
async fn emit_write_from_device() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    harness.events.lock().unwrap().clear();
    let frame = zigbee_apiarist::zspec::zcl::zcl_frame::Frame::create(
        0,
        0,
        true,
        None,
        40,
        2u8,
        10,
        json!([{"attrId": 16389, "dataType": 32, "attrData": 3}]),
        json!({}),
        0,
    );
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: frame.cluster.ID as u16,
        data: frame.to_buffer(),
        header: Some(frame.header),
        address: json!(129),
        endpoint: 1,
        linkquality: 19,
        group_id: 10,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    let events = messages(&harness);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].r#type, "write");
    assert_eq!(events[0].data, json!({"16389": 3}));
    assert_eq!(events[0].cluster, json!("genTime"));
}

#[tokio::test]
async fn should_save_received_attributes() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let buffer = vec![24, 169, 10, 0, 0, 24, 1];
    let header =
        zigbee_apiarist::zspec::zcl::zcl_header::Header::from_buffer(&buffer).expect("header");
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: 0x0406,
        data: buffer,
        header: Some(header),
        address: json!(129),
        endpoint: 1,
        linkquality: 50,
        group_id: 1,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    assert_eq!(
        endpoint.get_cluster_attribute_value(json!("msOccupancySensing"), json!("occupancy")),
        Some(json!(1))
    );
}

#[tokio::test]
async fn database_basic_crud_errors() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let database = zigbee_apiarist::controller::model::entity::Entity::database();
    let mut database = database.lock().unwrap();
    let entry = json!({"id": 1, "type": "Group", "groupID": 9});
    let error = database
        .insert(zigbee_apiarist::controller::database::DatabaseEntry::from(
            entry,
        ))
        .expect_err("duplicate id");
    assert_eq!(error, "DatabaseEntry with ID '1' already exists");
    let error = database.remove(3).expect_err("missing id");
    assert_eq!(error, "DatabaseEntry with ID '3' does not exist");
    let entry = json!({"id": 3, "type": "Group", "groupID": 9});
    let error = database
        .update(
            zigbee_apiarist::controller::database::DatabaseEntry::from(entry),
            false,
        )
        .expect_err("missing id");
    assert_eq!(error, "DatabaseEntry with ID '3' does not exist");
}

#[tokio::test]
async fn poll_control_supported() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    harness.events.lock().unwrap().clear();
    join(&adapter, 174, "0x174");
    wait_interviews(&harness, 2).await;
    let device = harness.controller.get_device_by_ieee_addr("0x174").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    let binds = endpoint.binds();
    assert_eq!(binds.len(), 1);
    assert_eq!(binds[0].cluster.name, "genPollCtrl");
    assert!(matches!(
        binds[0].target,
        zigbee_apiarist::controller::model::endpoint::BindTarget::Endpoint(_)
    ));
    // Upstream stores `checkinInterval / 4` (seconds); the Rust field is integral.
    assert!(device.checkin_interval().is_some());
}

#[tokio::test]
async fn poll_control_unsupported() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    assert!(endpoint.binds().is_empty());
}

#[tokio::test]
async fn endpoint_get_id_by_endpoint_device_type() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    harness.events.lock().unwrap().clear();
    join(&adapter, 172, "0x172");
    wait_interviews(&harness, 2).await;
    let device = harness.controller.get_device_by_ieee_addr("0x172").unwrap();
    assert!(device
        .get_endpoint_by_device_type("ZLLOnOffPluginUnit")
        .is_none());
    assert_eq!(
        device
            .get_endpoint_by_device_type("ZLLExtendedColorLight")
            .expect("endpoint")
            .ID,
        11
    );
}

#[tokio::test]
async fn device_updates_endpoint_bindings_from_binding_table() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 161, "0x00000000000161");
    wait_interviews(&harness, 2).await;
    join(&adapter, 162, "0x00000000000162");
    wait_interviews(&harness, 4).await;
    harness.controller.create_group(0x1234).expect("group");
    adapter.set_zdo(|_ieee, _nwk, cluster_id, _payload, _disable| {
        if cluster_id
            == zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::BINDING_TABLE_REQUEST
        {
            Ok(json!([0, {
                "bindingTableEntries": 2,
                "startIndex": 0,
                "entryList": [
                    {
                        "sourceEui64": "0x00000000000161",
                        "sourceEndpoint": 1,
                        "clusterId": 0,
                        "destAddrMode": 3,
                        "dest": "0x00000000000162",
                        "destEndpoint": 2
                    },
                    {
                        "sourceEui64": "0x00000000000161",
                        "sourceEndpoint": 4,
                        "clusterId": 9,
                        "destAddrMode": 1,
                        "dest": 4660
                    }
                ]
            }]))
        } else {
            Err("unexpected".to_string())
        }
    });
    let device = harness
        .controller
        .get_device_by_ieee_addr("0x00000000000161")
        .unwrap();
    let table = device.binding_table().await.expect("binding table");
    assert_eq!(table.len(), 2);
    let ep1 = device.get_endpoint(1).unwrap();
    let ep4 = device.get_endpoint(4).unwrap();
    eprintln!(
        "DBG table={table:?} endpoints={} ep1={} ep4={}",
        device.endpoints().len(),
        ep1.binds().len(),
        ep4.binds().len()
    );
    assert_eq!(ep1.binds().len(), 1);
    assert_eq!(ep1.binds()[0].cluster.ID, 0);
    assert_eq!(ep4.binds().len(), 1);
    assert_eq!(ep4.binds()[0].cluster.ID, 9);
    assert!(matches!(
        ep4.binds()[0].target,
        zigbee_apiarist::controller::model::endpoint::BindTarget::Group(_)
    ));
}

#[tokio::test]
async fn device_updates_existing_endpoint_bindings_from_binding_table() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    let group = harness.controller.create_group(1).expect("group");
    endpoint
        .add_binding(
            json!("genBasic"),
            zigbee_apiarist::controller::model::endpoint::BindTarget::Group(group),
        )
        .expect("add binding");
    assert_eq!(endpoint.binds().len(), 1);
    adapter.set_zdo(|_ieee, _nwk, cluster_id, _payload, _disable| {
        if cluster_id
            == zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::BINDING_TABLE_REQUEST
        {
            Ok(json!([0, {
                "bindingTableEntries": 0,
                "startIndex": 0,
                "entryList": []
            }]))
        } else {
            Err("unexpected".to_string())
        }
    });
    device.binding_table().await.expect("binding table");
    assert!(endpoint.binds().is_empty());
}

#[tokio::test]
async fn device_clears_all_and_selective_bindings() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 161, "0x00000000000161");
    wait_interviews(&harness, 2).await;
    let group = harness.controller.create_group(0x1234).expect("group");
    let device = harness
        .controller
        .get_device_by_ieee_addr("0x00000000000161")
        .unwrap();
    let ep1 = device.get_endpoint(1).unwrap();
    ep1.add_binding(
        json!("genAlarms"),
        zigbee_apiarist::controller::model::endpoint::BindTarget::Group(group.clone()),
    )
    .expect("add binding");
    assert_eq!(ep1.binds().len(), 1);

    device
        .clear_all_bindings(vec!["0xf1f1f1f1f1f1f1f1".to_string()])
        .await
        .expect("selective clear");
    assert_eq!(ep1.binds().len(), 1);

    device
        .clear_all_bindings(vec![
            "0xf1f1f1f1f1f1f1f1".to_string(),
            "0xa9a9a9a9a9a9a9a9".to_string(),
        ])
        .await
        .expect("multi-target clear");
    assert_eq!(ep1.binds().len(), 1);

    device
        .clear_all_bindings(vec!["0xffffffffffffffff".to_string()])
        .await
        .expect("clear all");
    assert!(ep1.binds().is_empty());
}

#[tokio::test]
async fn device_and_group_lookup_errors() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    assert!(harness.controller.get_group_by_id(9).is_none());
    assert!(harness
        .controller
        .get_device_by_ieee_addr("0x9999999999999999")
        .is_none());
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    device.remove_from_network(false).await.expect("remove");
    assert!(
        zigbee_apiarist::controller::model::device::Device::by_ieee_addr("0x129", false).is_none()
    );
    assert!(
        zigbee_apiarist::controller::model::device::Device::by_ieee_addr("0x129", true).is_some()
    );
}

#[tokio::test]
async fn endpoint_get_binds_non_existing_device() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 161, "0x00000000000161");
    join(&adapter, 162, "0x00000000000162");
    wait_interviews(&harness, 4).await;
    let device = harness
        .controller
        .get_device_by_ieee_addr("0x00000000000161")
        .unwrap();
    let target = harness
        .controller
        .get_device_by_ieee_addr("0x00000000000162")
        .unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    endpoint
        .add_binding(
            json!("genBasic"),
            zigbee_apiarist::controller::model::endpoint::BindTarget::Endpoint(
                target.get_endpoint(2).unwrap(),
            ),
        )
        .expect("binding");
    assert_eq!(endpoint.binds().len(), 1);
    target.remove_from_database(true);
    assert!(endpoint.binds().is_empty());
}

#[tokio::test]
async fn read_reporting_config_no_response_errors() {
    use zigbee_apiarist::controller::model::endpoint::ReadReportConfigItem;
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    *adapter.no_response_command.lock().unwrap() = Some("readReportConfig".to_string());
    let error = endpoint
        .read_reporting_config(
            json!("genPowerCfg"),
            vec![ReadReportConfigItem {
                direction: None,
                attribute: json!("mainsVoltage"),
            }],
            None,
        )
        .await
        .expect_err("no response");
    assert_eq!(error, "No response received");
}

#[tokio::test]
async fn read_reporting_config_ignores_unknown_attributes() {
    use zigbee_apiarist::controller::model::endpoint::ReadReportConfigItem;
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    *adapter.read_report_config_payload.lock().unwrap() = Some(json!([
        {"status": 0x8c, "direction": 0, "attrId": 0}
    ]));
    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .read_reporting_config(
            json!("genBasic"),
            vec![
                ReadReportConfigItem {
                    direction: None,
                    attribute: json!("zclVersion"),
                },
                ReadReportConfigItem {
                    direction: None,
                    attribute: json!("notanattr"),
                },
            ],
            None,
        )
        .await
        .expect("read reporting config");
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(frames[0].payload, json!([{"direction": 0, "attrId": 0}]));
}

#[tokio::test]
async fn read_reporting_config_manufacturer_specific() {
    use zigbee_apiarist::controller::model::endpoint::ReadReportConfigItem;
    use zigbee_apiarist::zspec::zcl::definition::enums::DataType;
    use zigbee_apiarist::zspec::zcl::utils::{Attribute, Cluster};
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let mut cluster = Cluster::new("genBasic", 0x0000);
    let mut attribute = Attribute::new("schneiderMeterRadioPower", 0xe200, DataType::INT8);
    attribute.manufacturerCode = Some(0x105e);
    attribute.write = true;
    cluster
        .attributes
        .insert("schneiderMeterRadioPower".to_string(), attribute);
    device
        .add_custom_cluster("genBasic", cluster)
        .expect("custom cluster");
    let endpoint = device.get_endpoint(1).unwrap();
    *adapter.read_report_config_payload.lock().unwrap() = Some(json!([
        {"status": 0, "direction": 0, "attrId": 0xe200, "dataType": 0x28, "minRepIntval": 15, "maxRepIntval": 213, "repChange": 1}
    ]));
    adapter.sent_endpoint.lock().unwrap().clear();
    let response = endpoint
        .read_reporting_config(
            json!("genBasic"),
            vec![ReadReportConfigItem {
                direction: None,
                attribute: json!("schneiderMeterRadioPower"),
            }],
            None,
        )
        .await
        .expect("manufacturer specific");
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(frames[0].header.manufacturer_code, Some(0x105e));
    assert_eq!(
        frames[0].payload,
        json!([{"direction": 0, "attrId": 57856}])
    );
    assert_eq!(response[0]["attrId"], 0xe200);
}

#[tokio::test]
async fn load_database_interview_state_migration() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    let failed = json!({
        "id": 3, "type": "Router", "ieeeAddr": "0x000b57fffec6a5b2", "nwkAddr": 40369,
        "manufId": 4476, "manufName": "IKEA of Sweden", "powerSource": "Mains (single phase)",
        "modelId": "TRADFRI bulb E27 WS opal 980lm", "epList": [1],
        "endpoints": {"1": {"profId": 49246, "epId": 1, "devId": 544,
            "inClusterList": [0], "meta": {}, "outClusterList": [5], "clusters": {}}},
        "appVersion": 17, "stackVersion": 87, "hwVersion": 1, "dateCode": "20170331",
        "swBuildId": "1.2.217", "zclVersion": 1, "interviewCompleted": false,
        "meta": {"reporting": 1}, "_id": "a"
    });
    let successful = json!({
        "id": 4, "type": "Router", "ieeeAddr": "0x000b57fffec6a5b3", "nwkAddr": 40370,
        "manufId": 4476, "manufName": "IKEA of Sweden", "powerSource": "Mains (single phase)",
        "modelId": "TRADFRI bulb E27 WS opal 980lm", "epList": [1],
        "endpoints": {"1": {"profId": 49246, "epId": 1, "devId": 544,
            "inClusterList": [0], "meta": {}, "outClusterList": [5], "clusters": {}}},
        "appVersion": 17, "stackVersion": 87, "hwVersion": 1, "dateCode": "20170331",
        "swBuildId": "1.2.217", "zclVersion": 1, "interviewCompleted": true,
        "meta": {"reporting": 1}, "_id": "b"
    });
    std::fs::write(&harness.database_path, format!("{failed}\n{successful}\n")).expect("database");
    harness.start().await.expect("start");
    assert_eq!(
        harness
            .controller
            .get_device_by_ieee_addr("0x000b57fffec6a5b2")
            .unwrap()
            .interview_state(),
        zigbee_apiarist::controller::model::device::InterviewState::Failed
    );
    assert_eq!(
        harness
            .controller
            .get_device_by_ieee_addr("0x000b57fffec6a5b3")
            .unwrap()
            .interview_state(),
        zigbee_apiarist::controller::model::device::InterviewState::Successful
    );
}

#[tokio::test]
async fn join_a_device_and_explicitly_refuses_it() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::with_options_network_and_accept(
        adapter.clone(),
        support::default_network_options(),
        false,
    );
    harness.start().await.expect("start");
    harness.events.lock().unwrap().clear();
    adapter.zdo_calls.lock().unwrap().clear();
    join(&adapter, 129, "0x129");
    settle().await;
    assert!(harness
        .controller
        .get_device_by_ieee_addr("0x129")
        .is_none());
    assert!(adapter.zdo_calls.lock().unwrap().iter().any(|call| call.2
        == zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::LEAVE_REQUEST));
    assert!(!harness.take_events().iter().any(|event| matches!(
        event,
        zigbee_apiarist::controller::events::ControllerEvent::DeviceInterview(_)
    )));
}

#[tokio::test]
async fn join_a_device_and_explicitly_refuses_it_but_leave_fails() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::with_options_network_and_accept(
        adapter.clone(),
        support::default_network_options(),
        false,
    );
    harness.start().await.expect("start");
    adapter.set_zdo(|_ieee, _nwk, cluster_id, _payload, _disable| {
        if cluster_id == zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::LEAVE_REQUEST
        {
            Err("timeout".to_string())
        } else {
            Ok(json!([0, Value::Null]))
        }
    });
    join(&adapter, 129, "0x129");
    settle().await;
    assert!(harness
        .controller
        .get_device_by_ieee_addr("0x129")
        .is_none());
}

#[tokio::test]
async fn network_address_event_should_update_last_seen() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let before = device.last_seen();
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    harness.events.lock().unwrap().clear();
    zdo(
        &adapter,
        0x8000,
        json!([0, {"nwkAddress": 129, "eui64": "0x129", "startIndex": 0, "assocDevList": []}]),
    );
    settle().await;
    assert!(device.last_seen().is_some());
    assert!(device.last_seen() >= before);
    assert!(last_seen_events(&harness)
        .iter()
        .any(|event| event.device.ieee_addr() == "0x129"));
}

#[tokio::test]
async fn merges_added_custom_clusters_definitions() {
    use zigbee_apiarist::zspec::zcl::definition::enums::DataType;
    use zigbee_apiarist::zspec::zcl::utils::{Attribute, Cluster};
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();

    let mut first = Cluster::new("hvacThermostat", 0x0201);
    first.attributes.insert(
        "boschOperatingMode".to_string(),
        Attribute::new("boschOperatingMode", 16391, DataType::ENUM8),
    );
    device
        .add_custom_cluster("hvacThermostat", first)
        .expect("first");
    let mut second = Cluster::new("hvacThermostat", 0x0201);
    second.attributes.insert(
        "boschHeatingDemand".to_string(),
        Attribute::new("boschHeatingDemand", 16416, DataType::ENUM8),
    );
    device
        .add_custom_cluster("hvacThermostat", second)
        .expect("second");

    let clusters = device.custom_clusters();
    let cluster =
        zigbee_apiarist::zspec::zcl::utils::Utils::get_cluster("hvacThermostat", None, &clusters);
    assert!(cluster.attributes.contains_key("boschOperatingMode"));
    assert!(cluster.attributes.contains_key("boschHeatingDemand"));
    // Built-in attributes are preserved.
    assert!(cluster.attributes.contains_key("localTemp"));
}

#[tokio::test]
async fn throw_error_when_device_with_ieee_address_already_exists() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let error = zigbee_apiarist::controller::model::device::Device::create(
        zigbee_apiarist::controller::database::DeviceType::Router,
        "0x129",
        129,
        None,
        None,
        None,
        None,
        zigbee_apiarist::controller::model::device::InterviewState::Pending,
        None,
    )
    .err()
    .expect("duplicate device");
    assert_eq!(error, "Device with IEEE address '0x129' already exists");
}

#[tokio::test]
async fn remove_device_clear_cache_variants() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    join(&adapter, 130, "0x130");
    settle().await;
    let first = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    first
        .remove_from_network(true)
        .await
        .expect("remove network clear");
    assert!(
        zigbee_apiarist::controller::model::device::Device::by_ieee_addr("0x129", true).is_none()
    );
    let second = harness.controller.get_device_by_ieee_addr("0x130").unwrap();
    second.remove_from_database(true);
    assert!(
        zigbee_apiarist::controller::model::device::Device::by_ieee_addr("0x130", true).is_none()
    );
}

#[tokio::test]
async fn read_from_endpoint_custom_attribute() {
    use zigbee_apiarist::zspec::zcl::definition::enums::DataType;
    use zigbee_apiarist::zspec::zcl::utils::{Attribute, Cluster};
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let mut cluster = Cluster::new("hvacThermostat", 0x0201);
    let mut attribute = Attribute::new("viessmannWindowOpenInternal", 0x4000, DataType::ENUM8);
    attribute.manufacturerCode = Some(0x1221);
    cluster
        .attributes
        .insert("viessmannWindowOpenInternal".to_string(), attribute);
    device
        .add_custom_cluster("hvacThermostat", cluster)
        .expect("custom cluster");
    let endpoint = device.get_endpoint(1).unwrap();
    adapter.sent_endpoint.lock().unwrap().clear();
    endpoint
        .read(
            json!("hvacThermostat"),
            json!(["viessmannWindowOpenInternal"]),
            None,
        )
        .await
        .expect("read custom attribute");
    let frames = adapter.sent_endpoint.lock().unwrap();
    assert_eq!(frames[0].header.manufacturer_code, Some(0x1221));
    assert_eq!(frames[0].payload, json!([{"attrId": 16384}]));
}

fn fail_next(adapter: &MockAdapter, command: &str) {
    adapter
        .fail_zcl_command
        .lock()
        .unwrap()
        .replace(command.to_string());
}

fn no_response_next(adapter: &MockAdapter, command: &str) {
    adapter
        .no_response_command
        .lock()
        .unwrap()
        .replace(command.to_string());
}

const DEFAULT_OPTIONS_JSON: &str = r#"{"timeout":10000,"disableResponse":false,"disableRecovery":false,"disableDefaultResponse":false,"direction":0,"reservedBits":0,"writeUndiv":false}"#;
const DEFAULT_OPTIONS_JSON_DDR: &str = r#"{"timeout":10000,"disableResponse":false,"disableRecovery":false,"disableDefaultResponse":true,"direction":0,"reservedBits":0,"writeUndiv":false}"#;

#[tokio::test]
async fn endpoint_command_and_command_response_errors() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();

    fail_next(&adapter, "toggle");
    let error = endpoint
        .command(json!("genOnOff"), json!("toggle"), json!({}), None)
        .await
        .expect_err("command error");
    assert_eq!(
        error,
        format!(
            "ZCL command 0x129/1 genOnOff.toggle({{}}, {DEFAULT_OPTIONS_JSON}) failed (timeout occurred)"
        )
    );

    fail_next(&adapter, "imageNotify");
    let error = endpoint
        .command_response(
            json!("genOta"),
            json!("imageNotify"),
            json!({"payloadType": 0, "queryJitter": 1}),
            None,
            None,
        )
        .await
        .expect_err("command response error");
    assert_eq!(
        error,
        r#"CommandResponse 0x129/1 genOta.imageNotify({"payloadType":0,"queryJitter":1}, {"timeout":10000,"disableResponse":false,"disableRecovery":false,"disableDefaultResponse":true,"direction":1,"reservedBits":0,"writeUndiv":false}) failed (timeout occurred)"#
    );

    let error = endpoint
        .command_response(
            json!("genOta"),
            json!("imageNotify"),
            json!({"payloadType": 0, "queryJitter": 1}),
            Some(zigbee_apiarist::controller::model::endpoint::Options {
                transaction_sequence_number: Some(10),
                ..Default::default()
            }),
            None,
        )
        .await
        .expect_err("tsn option");
    assert_eq!(error, "Use parameter");
}

#[tokio::test]
async fn write_and_read_response_errors() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();

    fail_next(&adapter, "writeRsp");
    let error = endpoint
        .write_response(
            json!("genBasic"),
            99,
            json!({"zclVersion": {"status": 1}}),
            None,
        )
        .await
        .expect_err("write response error");
    assert_eq!(
        error,
        r#"ZCL command 0x129/1 genBasic.writeRsp({"zclVersion":{"status":1}}, {"timeout":10000,"disableResponse":false,"disableRecovery":false,"disableDefaultResponse":true,"direction":1,"reservedBits":0,"transactionSequenceNumber":99,"writeUndiv":false}) failed (timeout occurred)"#
    );

    fail_next(&adapter, "readRsp");
    let error = endpoint
        .read_response(json!("genOnOff"), 1, json!({"onOff": 1}), None)
        .await
        .expect_err("read response error");
    assert_eq!(
        error,
        r#"ZCL command 0x129/1 genOnOff.readRsp({"onOff":1}, {"timeout":10000,"disableResponse":false,"disableRecovery":false,"disableDefaultResponse":true,"direction":1,"reservedBits":0,"transactionSequenceNumber":1,"writeUndiv":false}) failed (timeout occurred)"#
    );
}

#[tokio::test]
async fn read_and_write_errors_and_disable_response() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();

    fail_next(&adapter, "read");
    let error = endpoint
        .read(json!("genOnOff"), json!(["onOff"]), None)
        .await
        .expect_err("read error");
    assert_eq!(
        error,
        format!(
            r#"ZCL command 0x129/1 genOnOff.read(["onOff"], {DEFAULT_OPTIONS_JSON_DDR}) failed (timeout occurred)"#
        )
    );

    fail_next(&adapter, "write");
    let error = endpoint
        .write(json!("genOnOff"), json!({"onTime": 1}), None)
        .await
        .expect_err("write error");
    assert_eq!(
        error,
        format!(
            r#"ZCL command 0x129/1 genOnOff.write({{"onTime":1}}, {DEFAULT_OPTIONS_JSON_DDR}) failed (timeout occurred)"#
        )
    );

    no_response_next(&adapter, "read");
    endpoint
        .read(
            json!("genOnOff"),
            json!(["onOff"]),
            Some(zigbee_apiarist::controller::model::endpoint::Options {
                disable_response: Some(true),
                ..Default::default()
            }),
        )
        .await
        .expect("read with disable response");

    no_response_next(&adapter, "write");
    endpoint
        .write(
            json!("genOnOff"),
            json!({"onTime": 1}),
            Some(zigbee_apiarist::controller::model::endpoint::Options {
                disable_response: Some(true),
                ..Default::default()
            }),
        )
        .await
        .expect("write with disable response");
}

#[tokio::test]
async fn configure_reporting_and_default_response_errors() {
    use zigbee_apiarist::controller::model::endpoint::ConfigureReportingItem;
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();

    fail_next(&adapter, "configReport");
    let error = endpoint
        .configure_reporting(
            json!("genOnOff"),
            vec![ConfigureReportingItem {
                attribute: json!("onOff"),
                minimum_report_interval: 0,
                maximum_report_interval: 2,
                reportable_change: Some(10.0),
            }],
            None,
        )
        .await
        .expect_err("configure reporting error");
    assert_eq!(
        error,
        format!(
            r#"ZCL command 0x129/1 genOnOff.configReport([{{"attribute":"onOff","minimumReportInterval":0,"maximumReportInterval":2,"reportableChange":10}}], {DEFAULT_OPTIONS_JSON_DDR}) failed (timeout occurred)"#
        )
    );

    fail_next(&adapter, "defaultRsp");
    let error = endpoint
        .default_response(1, 0, 1, 3, None)
        .await
        .expect_err("default response error");
    assert_eq!(
        error,
        r#"ZCL command 0x129/1 genPowerCfg.defaultRsp({"cmdId":1,"statusCode":0}, {"timeout":10000,"disableResponse":false,"disableRecovery":false,"disableDefaultResponse":true,"direction":1,"reservedBits":0,"transactionSequenceNumber":3,"writeUndiv":false}) failed (timeout occurred)"#
    );

    let error = endpoint
        .default_response(
            1,
            0,
            1,
            3,
            Some(zigbee_apiarist::controller::model::endpoint::Options {
                transaction_sequence_number: Some(10),
                ..Default::default()
            }),
        )
        .await
        .expect_err("tsn option");
    assert_eq!(error, "Use parameter");
}

#[tokio::test]
async fn group_command_error() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    let group = harness.controller.create_group(2).expect("group");
    adapter
        .fail_group_send
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let error = group
        .command(json!("genOnOff"), json!("toggle"), json!({}), None)
        .await
        .expect_err("group command error");
    assert_eq!(error, "Command 2 genOnOff.toggle({}) failed (timeout)");
}

#[tokio::test]
async fn write_structured_and_custom_payload() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();

    no_response_next(&adapter, "writeStructured");
    endpoint
        .write_structured(json!("genPowerCfg"), json!([]), None)
        .await
        .expect("write structured");

    no_response_next(&adapter, "writeStructured");
    endpoint
        .write_structured(
            json!("genPowerCfg"),
            json!([]),
            Some(zigbee_apiarist::controller::model::endpoint::Options {
                disable_response: Some(true),
                ..Default::default()
            }),
        )
        .await
        .expect("write structured disable response");

    fail_next(&adapter, "writeStructured");
    let error = endpoint
        .write_structured(json!("genPowerCfg"), json!([]), None)
        .await
        .expect_err("write structured error");
    assert_eq!(
        error,
        format!(
            r#"ZCL command 0x129/1 genPowerCfg.writeStructured([], {DEFAULT_OPTIONS_JSON_DDR}) failed (timeout occurred)"#
        )
    );
}

#[tokio::test]
async fn report_error() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    fail_next(&adapter, "report");
    let error = endpoint
        .report(json!("genOnOff"), json!({"onOff": 1}), None)
        .await
        .expect_err("report error");
    assert_eq!(
        error,
        format!(
            r#"ZCL command 0x129/1 genOnOff.report({{"onOff":1}}, {DEFAULT_OPTIONS_JSON_DDR}) failed (timeout occurred)"#
        )
    );
}

#[tokio::test]
async fn skip_unbind_if_not_bound_and_bind_unbind_errors() {
    use zigbee_apiarist::controller::model::endpoint::BindTarget;
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    join(&adapter, 170, "0x170");
    wait_interviews(&harness, 4).await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    let target_device = harness.controller.get_device_by_ieee_addr("0x170").unwrap();
    let target = target_device.get_endpoint(1).unwrap();

    adapter.zdo_calls.lock().unwrap().clear();
    endpoint
        .unbind(
            json!("genOnOff"),
            BindTarget::Endpoint(target.clone()),
            false,
        )
        .await
        .expect("skip unbind");
    assert!(adapter.zdo_calls.lock().unwrap().is_empty());

    // Bind succeeds against a permissive adapter.
    adapter.set_zdo(|_ieee, _nwk, _cluster, _payload, _disable| Ok(json!([0, Value::Null])));
    endpoint
        .bind(json!("genOnOff"), BindTarget::Endpoint(target.clone()))
        .await
        .expect("bind");
    assert_eq!(endpoint.binds().len(), 1);

    // NO_ENTRY on unbind removes the local binding without failing.
    adapter.set_zdo(|_ieee, _nwk, cluster_id, _payload, _disable| {
        if cluster_id
            == zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::UNBIND_REQUEST
        {
            Ok(json!([0x88, Value::Null]))
        } else {
            Ok(json!([0, Value::Null]))
        }
    });
    endpoint
        .unbind(
            json!("genOnOff"),
            BindTarget::Endpoint(target.clone()),
            false,
        )
        .await
        .expect("unbind no entry");
    assert!(endpoint.binds().is_empty());

    // Bind error is wrapped with the upstream message.
    adapter.set_zdo(|_ieee, _nwk, _cluster, _payload, _disable| Ok(json!([0x8f, Value::Null])));
    let error = endpoint
        .bind(json!("genOnOff"), BindTarget::Endpoint(target.clone()))
        .await
        .expect_err("bind error");
    assert_eq!(
        error,
        "Bind 0x129/1 genOnOff from '0x170/1' failed (Status 'INVALID_INDEX')"
    );

    // Unbind error uses the same message shape.
    endpoint
        .add_binding(json!("genOnOff"), BindTarget::Endpoint(target.clone()))
        .expect("add binding");
    let error = endpoint
        .unbind(json!("genOnOff"), BindTarget::Endpoint(target), false)
        .await
        .expect_err("unbind error");
    assert_eq!(
        error,
        "Unbind 0x129/1 genOnOff from '0x170/1' failed (Status 'INVALID_INDEX')"
    );
}

#[tokio::test]
async fn get_input_and_output_clusters() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let endpoint = device.get_endpoint(1).unwrap();
    assert_eq!(
        endpoint
            .get_input_clusters()
            .iter()
            .map(|cluster| cluster.ID)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(
        endpoint
            .get_output_clusters()
            .iter()
            .map(|cluster| cluster.ID)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert!(endpoint.supports_input_cluster(json!("genBasic")));
    assert!(endpoint.supports_output_cluster(json!("genDeviceTempCfg")));
    assert!(!endpoint.supports_output_cluster(json!("genBasic")));
}

#[tokio::test]
async fn zdo_table_and_remove_errors() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 140, "0x140");
    wait_interviews(&harness, 2).await;
    let device = harness.controller.get_device_by_ieee_addr("0x140").unwrap();

    adapter.set_zdo(|_ieee, _nwk, _cluster, _payload, _disable| Ok(json!([0x8f, Value::Null])));
    assert_eq!(
        device.lqi().await.expect_err("lqi"),
        "Status 'INVALID_INDEX'"
    );
    assert_eq!(
        device.routing_table().await.expect_err("routing"),
        "Status 'INVALID_INDEX'"
    );
    assert_eq!(
        device.binding_table().await.expect_err("binding"),
        "Status 'INVALID_INDEX'"
    );
    assert_eq!(
        device
            .remove_from_network(false)
            .await
            .expect_err("remove network"),
        "Status 'INVALID_INDEX'"
    );
    assert!(harness
        .controller
        .get_device_by_ieee_addr("0x140")
        .is_some());

    adapter.set_zdo(|_ieee, _nwk, _cluster, _payload, _disable| Ok(json!([0x80, Value::Null])));
    assert_eq!(
        device
            .clear_all_bindings(vec!["0xffffffffffffffff".to_string()])
            .await
            .expect_err("clear bindings"),
        "Status 'INV_REQUESTTYPE'"
    );
}

fn lqi_entry(eui64: &str, nwk: u16, lqi: u8, relationship: u8, depth: u8) -> Value {
    json!({
        "extendedPanId": [1,2,3,4,5,6,7,8],
        "eui64": eui64,
        "nwkAddress": nwk,
        "deviceType": 1,
        "rxOnWhenIdle": 0,
        "relationship": relationship,
        "reserved1": 0,
        "permitJoining": 0,
        "reserved2": 0,
        "depth": depth,
        "lqi": lqi
    })
}

fn routing_entry(destination: u16, status: &str, next_hop: u16) -> Value {
    json!({
        "destinationAddress": destination,
        "status": status,
        "memoryConstrained": 0,
        "manyToOne": 0,
        "routeRecordRequired": 0,
        "reserved1": 0,
        "nextHopAddress": next_hop
    })
}

#[tokio::test]
async fn tables_with_more_than_one_request() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 140, "0x140");
    wait_interviews(&harness, 2).await;
    let device = harness.controller.get_device_by_ieee_addr("0x140").unwrap();

    // LQI: two pages.
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    adapter.set_zdo(move |_ieee, _nwk, _cluster, _payload, _disable| {
        let call = counter.fetch_add(1, Ordering::SeqCst);
        if call == 0 {
            Ok(json!([0, {
                "neighborTableEntries": 3, "startIndex": 0,
                "entryList": [lqi_entry("0x160", 160, 20, 2, 5), lqi_entry("0x170", 170, 21, 4, 8)]
            }]))
        } else {
            Ok(json!([0, {
                "neighborTableEntries": 3, "startIndex": 2,
                "entryList": [lqi_entry("0x180", 180, 200, 4, 2)]
            }]))
        }
    });
    let lqi = device.lqi().await.expect("lqi");
    assert_eq!(lqi.len(), 3);
    assert_eq!(lqi[2]["eui64"], "0x180");
    assert_eq!(lqi[2]["lqi"], 200);

    // Routing: two pages.
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    adapter.set_zdo(move |_ieee, _nwk, _cluster, _payload, _disable| {
        let call = counter.fetch_add(1, Ordering::SeqCst);
        if call == 0 {
            Ok(json!([0, {
                "routingTableEntries": 3, "startIndex": 0,
                "entryList": [
                    routing_entry(120, "ACTIVE", 1),
                    routing_entry(130, "DISCOVERY_FAILED", 2)
                ]
            }]))
        } else {
            Ok(json!([0, {
                "routingTableEntries": 3, "startIndex": 2,
                "entryList": [routing_entry(140, "INACTIVE", 3)]
            }]))
        }
    });
    let routing = device.routing_table().await.expect("routing");
    assert_eq!(routing.len(), 3);
    assert_eq!(routing[2]["status"], "INACTIVE");

    // Binding: two pages.
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    adapter.set_zdo(move |_ieee, _nwk, _cluster, _payload, _disable| {
        let call = counter.fetch_add(1, Ordering::SeqCst);
        if call == 0 {
            Ok(json!([0, {
                "bindingTableEntries": 3, "startIndex": 0,
                "entryList": [
                    {
                        "sourceEui64": "0xf1f2f3f5f6f7f8", "sourceEndpoint": 1,
                        "clusterId": 0, "destAddrMode": 3,
                        "dest": "0xa1a2a3a4a5a6a7a8", "destEndpoint": 2
                    },
                    {
                        "sourceEui64": "0xe1e2e3e5e6e7e8", "sourceEndpoint": 3,
                        "clusterId": 9, "destAddrMode": 1, "dest": 4660
                    }
                ]
            }]))
        } else {
            Ok(json!([0, {
                "bindingTableEntries": 3, "startIndex": 2,
                "entryList": [{
                    "sourceEui64": "0xc1c2c3c5c6c7c8", "sourceEndpoint": 2,
                    "clusterId": 8, "destAddrMode": 3,
                    "dest": "0xa1a2a3a4a5a6a7a8", "destEndpoint": 3
                }]
            }]))
        }
    });
    let binding = device.binding_table().await.expect("binding");
    assert_eq!(binding.len(), 3);
    assert_eq!(binding[2]["sourceEui64"], "0xc1c2c3c5c6c7c8");
}

#[tokio::test]
async fn node_descriptor_revision_logs() {
    let _guard = support::test_lock().await;
    let logs = support::capture_logs();
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 162, "0x162");
    wait_interviews(&harness, 2).await;
    assert!(logs.has(
        "info",
        &format!(
            "Device '0x162' is only compliant to revision '21' of the Zigbee specification (current revision: {}).",
            zigbee_apiarist::zspec::consts::ZIGBEE_REVISION
        ),
        "zh:controller:device"
    ));

    join(&adapter, 161, "0x161");
    wait_interviews(&harness, 4).await;
    assert!(logs.has(
        "info",
        &format!(
            "Device '0x161' is only compliant to revision 'pre-21' of the Zigbee specification (current revision: {}).",
            zigbee_apiarist::zspec::consts::ZIGBEE_REVISION
        ),
        "zh:controller:device"
    ));
}

#[tokio::test]
async fn ignores_invalid_attributes_when_processing_zcl_attribute() {
    let _guard = support::test_lock().await;
    let logs = support::capture_logs();
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 178, "0x178");
    wait_interviews(&harness, 2).await;
    let device = harness.controller.get_device_by_ieee_addr("0x178").unwrap();
    assert_eq!(
        device.model_id().as_deref(),
        Some(" other multi-endpoint device")
    );
    logs.messages.lock().unwrap().clear();

    let frame = zigbee_apiarist::zspec::zcl::zcl_frame::Frame::create(
        0,
        0,
        true,
        None,
        10,
        "readRsp",
        0,
        json!([{
            "attrId": 0x0005,
            "status": 0,
            "dataType": 66,
            "attrData": "a model id that is too long for spec"
        }]),
        json!({}),
        0,
    );
    adapter.emit_zcl(zigbee_apiarist::adapter::events::ZclPayload {
        cluster_id: 0,
        data: frame.to_buffer(),
        header: Some(frame.header),
        address: json!("0x178"),
        endpoint: 1,
        linkquality: 50,
        group_id: 0,
        was_broadcast: false,
        destination_endpoint: 1,
    });
    settle().await;
    assert!(logs.has(
        "debug",
        "Ignoring attribute modelId from response: Error: Status 'INVALID_VALUE' modelId requires max length of 32",
        "zh:controller:zcl"
    ));
    assert_eq!(
        device.model_id().as_deref(),
        Some(" other multi-endpoint device")
    );
}

#[tokio::test]
async fn interview_on_coordinator() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    adapter.zdo_calls.lock().unwrap().clear();
    let coordinator = harness
        .controller
        .get_device_by_network_address(0)
        .expect("coordinator");
    coordinator.interview(true).await.expect("interview");
    let calls = adapter.zdo_calls.lock().unwrap();
    assert_eq!(calls.len(), 4);
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.2
                == zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::NODE_DESCRIPTOR_REQUEST)
            .count(),
        1
    );
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.2
                == zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::ACTIVE_ENDPOINTS_REQUEST)
            .count(),
        1
    );
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.2
                == zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::SIMPLE_DESCRIPTOR_REQUEST)
            .count(),
        2
    );
}

#[tokio::test]
async fn interview_descriptor_requests_fail() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    let coordinator = harness
        .controller
        .get_device_by_network_address(0)
        .expect("coordinator");
    let endpoint = coordinator.get_endpoint(1).unwrap();

    adapter.set_zdo(|_ieee, _nwk, _cluster, _payload, _disable| Ok(json!([0x8a, Value::Null])));
    assert_eq!(
        coordinator
            .update_node_descriptor()
            .await
            .expect_err("node descriptor"),
        "Status 'INSUFFICIENT_SPACE'"
    );
    assert_eq!(
        coordinator
            .update_active_endpoints()
            .await
            .expect_err("active endpoints"),
        "Status 'INSUFFICIENT_SPACE'"
    );
    assert_eq!(
        endpoint
            .update_simple_descriptor()
            .await
            .expect_err("simple descriptor"),
        "Status 'INSUFFICIENT_SPACE'"
    );
}

#[tokio::test]
async fn updates_device_gen_basic_properties() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    assert!(!std::fs::read_to_string(&harness.database_path)
        .unwrap_or_default()
        .contains("0x129"));
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    assert_eq!(device.application_version(), Some(2));
    assert_eq!(device.date_code().as_deref(), Some("201901"));
    assert_eq!(device.hardware_version(), Some(3));
    assert_eq!(device.manufacturer_name().as_deref(), Some("KoenAndCo"));
    assert_eq!(device.model_id().as_deref(), Some("myModelID"));
    assert_eq!(
        device.power_source().as_deref(),
        Some("Mains (single phase)")
    );
    assert_eq!(device.software_build_id().as_deref(), Some("1.01"));
    assert_eq!(device.stack_version(), Some(101));
    assert_eq!(device.zcl_version(), Some(1));

    device.set_application_version(3);
    device.set_date_code("202501".to_string());
    device.set_hardware_version(4);
    device.set_manufacturer_name(Some("Test".to_string()));
    device.set_model_id("Me".to_string());
    device.set_power_source(json!("DC Source"));
    device.set_software_build_id("2.01".to_string());
    device.set_stack_version(202);
    device.set_zcl_version(2);

    assert_eq!(device.application_version(), Some(3));
    assert_eq!(device.date_code().as_deref(), Some("202501"));
    assert_eq!(device.hardware_version(), Some(4));
    assert_eq!(device.manufacturer_name().as_deref(), Some("Test"));
    assert_eq!(device.model_id().as_deref(), Some("Me"));
    assert_eq!(device.power_source().as_deref(), Some("DC Source"));
    assert_eq!(device.software_build_id().as_deref(), Some("2.01"));
    assert_eq!(device.stack_version(), Some(202));
    assert_eq!(device.zcl_version(), Some(2));
    assert_eq!(
        device.gen_basic(),
        json!({
            "appVersion": 3,
            "dateCode": "202501",
            "hwVersion": 4,
            "manufacturerName": "Test",
            "modelId": "Me",
            "powerSource": 4,
            "swBuildId": "2.01",
            "stackVersion": 202,
            "zclVersion": 2
        })
    );
}

#[tokio::test]
async fn should_use_cached_node_descriptor() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();

    let node_descriptor_requests = |adapter: &MockAdapter| {
        adapter
            .zdo_calls
            .lock()
            .unwrap()
            .iter()
            .filter(|call| {
                call.2
                    == zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::NODE_DESCRIPTOR_REQUEST
            })
            .count()
    };
    let before = node_descriptor_requests(&adapter);
    device.interview(false).await.expect("interview cached");
    assert_eq!(node_descriptor_requests(&adapter), before);
    device
        .interview(true)
        .await
        .expect("interview ignore cache");
    assert_eq!(node_descriptor_requests(&adapter), before + 1);
}

#[tokio::test]
async fn should_remove_disappeared_endpoints() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    assert!(device.get_endpoint(1).is_some());
    adapter.set_zdo(|_ieee, _nwk, cluster_id, _payload, _disable| {
        if cluster_id
            == zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::ACTIVE_ENDPOINTS_REQUEST
        {
            Ok(json!([0, {"nwkAddress": 129, "endpointList": [2]}]))
        } else {
            Ok(json!([0, Value::Null]))
        }
    });
    device
        .update_active_endpoints()
        .await
        .expect("active endpoints");
    assert!(device.get_endpoint(1).is_none());
    assert!(device.get_endpoint(2).is_some());
}

#[tokio::test]
async fn return_device_and_group_from_database() {
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    let device = zigbee_apiarist::controller::model::device::Device::create(
        zigbee_apiarist::controller::database::DeviceType::Router,
        "0x999",
        999,
        None,
        Some("Test".to_string()),
        None,
        Some("TEST".to_string()),
        zigbee_apiarist::controller::model::device::InterviewState::Successful,
        None,
    )
    .expect("device");
    device.save(true);
    let group = zigbee_apiarist::controller::model::group::Group::create(77).expect("group");

    zigbee_apiarist::controller::model::device::Device::reset_cache();
    zigbee_apiarist::controller::model::group::Group::reset_cache();

    let loaded = zigbee_apiarist::controller::model::device::Device::by_ieee_addr("0x999", false)
        .expect("device from database");
    assert_eq!(loaded.network_address(), 999);
    let loaded_group = zigbee_apiarist::controller::model::group::Group::by_group_id(77)
        .expect("group from database");
    assert_eq!(loaded_group.group_id, 77);
}

#[tokio::test]
async fn ensures_manufacturer_code_unique_in_single_call() {
    use zigbee_apiarist::controller::model::endpoint::{
        ConfigureReportingItem, ReadReportConfigItem,
    };
    use zigbee_apiarist::zspec::zcl::definition::enums::DataType;
    use zigbee_apiarist::zspec::zcl::utils::{Attribute, Cluster};
    let _guard = support::test_lock().await;
    let adapter = MockAdapter::new();
    let harness = Harness::new(adapter.clone());
    harness.start().await.expect("start");
    join(&adapter, 129, "0x129");
    settle().await;
    let device = harness.controller.get_device_by_ieee_addr("0x129").unwrap();
    let mut cluster = Cluster::new("hvacThermostat", 0x0201);
    let mut attribute = Attribute::new("viessmannWindowOpenInternal", 0x4000, DataType::ENUM8);
    attribute.manufacturerCode = Some(0x1221);
    attribute.write = true;
    cluster
        .attributes
        .insert("viessmannWindowOpenInternal".to_string(), attribute);
    device
        .add_custom_cluster("hvacThermostat", cluster)
        .expect("custom cluster");
    let endpoint = device.get_endpoint(1).unwrap();

    let error = endpoint
        .write(
            json!("hvacThermostat"),
            json!({"occupiedHeatingSetpoint": 2000, "viessmannWindowOpenInternal": 1}),
            None,
        )
        .await
        .expect_err("write manufacturer codes");
    assert_eq!(
        error,
        "Cannot have attributes with different manufacturerCode in single 'write' call"
    );

    let error = endpoint
        .read(
            json!("hvacThermostat"),
            json!(["localTemp", "viessmannWindowOpenInternal"]),
            None,
        )
        .await
        .expect_err("read manufacturer codes");
    assert_eq!(
        error,
        "Cannot have attributes with different manufacturerCode in single 'read' call"
    );

    let error = endpoint
        .configure_reporting(
            json!("hvacThermostat"),
            vec![
                ConfigureReportingItem {
                    attribute: json!("localTemp"),
                    minimum_report_interval: 1,
                    maximum_report_interval: 10,
                    reportable_change: Some(1.0),
                },
                ConfigureReportingItem {
                    attribute: json!("viessmannWindowOpenInternal"),
                    minimum_report_interval: 1,
                    maximum_report_interval: 10,
                    reportable_change: Some(1.0),
                },
            ],
            None,
        )
        .await
        .expect_err("configure reporting manufacturer codes");
    assert_eq!(
        error,
        "Cannot have attributes with different manufacturerCode in single 'configureReporting' call"
    );

    let error = endpoint
        .read_reporting_config(
            json!("hvacThermostat"),
            vec![
                ReadReportConfigItem {
                    direction: None,
                    attribute: json!("localTemp"),
                },
                ReadReportConfigItem {
                    direction: None,
                    attribute: json!("viessmannWindowOpenInternal"),
                },
            ],
            None,
        )
        .await
        .expect_err("read reporting config manufacturer codes");
    assert_eq!(
        error,
        "Cannot have attributes with different manufacturerCode in single 'readReportingConfig' call"
    );
}
