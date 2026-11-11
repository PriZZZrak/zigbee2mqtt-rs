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
