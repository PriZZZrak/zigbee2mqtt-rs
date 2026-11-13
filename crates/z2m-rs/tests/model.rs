// Original: .upstream/zigbee2mqtt/lib/model/device.ts, lib/model/group.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// R1 has no `zigbee-herdsman-converters`, so device tests cover the
// definition-independent surface (`AGENTS.md` §2/§4).

use std::sync::{Arc, Mutex};

use serde_json::json;
use zigbee_apiarist::controller::model::device::Device as ZhDevice;
use zigbee_apiarist::controller::model::test_support::serial;

use z2m_rs::model::{Device, Entity, EntityId};
use z2m_rs::util::settings::Settings;

fn zh_device(r#type: &str, ieee: &str) -> Arc<ZhDevice> {
    ZhDevice::from_database_entry(&json!({
        "id": 1,
        "type": r#type,
        "ieeeAddr": ieee,
        "nwkAddr": 1234,
        "manufId": 4476,
        "manufName": "Test",
        "powerSource": "Mains (single phase)",
        "modelId": "TEST",
        "interviewCompleted": true,
        "endpoints": {
            "1": {"epId": 1, "profId": 260, "devId": 5, "inClusterList": [0, 6], "outClusterList": []},
            "2": {"epId": 2, "profId": 260, "devId": 5, "inClusterList": [8], "outClusterList": []}
        },
    }))
}

fn settings(configuration: serde_json::Value) -> Arc<Mutex<Settings>> {
    Arc::new(Mutex::new(Settings::from_config(configuration)))
}

#[test]
fn device_options_fall_back_to_ieee_addr() {
    let _guard = serial();
    ZhDevice::reset_cache();

    let device = Device::without_converters(
        zh_device("Router", "0x0017880104e45519"),
        settings(json!({"mqtt": {"server": "localhost"}})),
    );

    assert_eq!(device.ieee_addr(), "0x0017880104e45519");
    assert_eq!(device.id(), "0x0017880104e45519");
    assert_eq!(
        device.options(),
        json!({"friendly_name": "0x0017880104e45519", "ID": "0x0017880104e45519"})
    );
    assert_eq!(device.name(), "0x0017880104e45519");
}

#[test]
fn device_options_merge_defaults_with_device_settings() {
    let _guard = serial();
    ZhDevice::reset_cache();

    let device = Device::without_converters(
        zh_device("Router", "0x0017880104e45519"),
        settings(json!({
            "mqtt": {"server": "localhost"},
            "device_options": {"retain": true, "qos": 1},
            "devices": {
                "0x0017880104e45519": {"friendly_name": "lamp", "qos": 0},
            },
        })),
    );

    assert_eq!(
        device.options(),
        json!({
            "retain": true,
            "qos": 0,
            "friendly_name": "lamp",
            "ID": "0x0017880104e45519",
        })
    );
    assert_eq!(device.name(), "lamp");
    assert_eq!(device.filtered_cache(), Vec::<String>::new());
}

#[test]
fn filtered_cache_is_read_from_options() {
    let _guard = serial();
    ZhDevice::reset_cache();

    let device = Device::without_converters(
        zh_device("Router", "0x0017880104e45519"),
        settings(json!({
            "mqtt": {"server": "localhost"},
            "devices": {
                "0x0017880104e45519": {
                    "friendly_name": "lamp",
                    "filtered_cache": ["battery", "voltage"],
                },
            },
        })),
    );

    assert_eq!(
        device.filtered_cache(),
        vec!["battery".to_string(), "voltage".to_string()]
    );
}

#[test]
fn coordinator_name_and_support() {
    let _guard = serial();
    ZhDevice::reset_cache();

    let device = Device::without_converters(
        zh_device("Coordinator", "0x00124b0024c16b95"),
        settings(json!({"mqtt": {"server": "localhost"}})),
    );

    assert_eq!(device.name(), "Coordinator");
    assert!(device.is_supported());
    assert_eq!(
        device.entity_id(),
        EntityId::Ieee("0x00124b0024c16b95".to_string())
    );
    assert!(device.is_device());
    assert!(!device.is_group());
}

#[test]
fn non_coordinator_is_not_supported_without_converters() {
    let _guard = serial();
    ZhDevice::reset_cache();

    let device = Device::without_converters(
        zh_device("Router", "0x0017880104e45519"),
        settings(json!({"mqtt": {"server": "localhost"}})),
    );

    assert!(!device.is_supported());
}

#[test]
fn ensure_in_settings_adds_non_coordinator_devices() {
    let _guard = serial();
    ZhDevice::reset_cache();

    let settings = settings(json!({"mqtt": {"server": "localhost"}}));
    let device =
        Device::without_converters(zh_device("Router", "0x0017880104e45519"), settings.clone());

    device.ensure_in_settings(&mut settings.lock().unwrap());
    assert_eq!(
        settings.lock().unwrap().get_device("0x0017880104e45519"),
        Some(json!({"ID": "0x0017880104e45519", "friendly_name": "0x0017880104e45519"}))
    );

    // Idempotent: a second call does not panic with "already exists".
    device.ensure_in_settings(&mut settings.lock().unwrap());

    let coordinator = Device::without_converters(
        zh_device("Coordinator", "0x00124b0024c16b95"),
        settings.clone(),
    );
    coordinator.ensure_in_settings(&mut settings.lock().unwrap());
    assert_eq!(
        settings.lock().unwrap().get_device("0x00124b0024c16b95"),
        None
    );
}

#[test]
fn endpoint_resolves_numeric_and_default_keys() {
    let _guard = serial();
    ZhDevice::reset_cache();

    let device = Device::without_converters(
        zh_device("Router", "0x0017880104e45519"),
        settings(json!({"mqtt": {"server": "localhost"}})),
    );

    assert_eq!(device.endpoint(None).map(|endpoint| endpoint.ID), Some(1));
    assert_eq!(
        device.endpoint(Some("default")).map(|endpoint| endpoint.ID),
        Some(1)
    );
    assert_eq!(
        device.endpoint(Some("2")).map(|endpoint| endpoint.ID),
        Some(2)
    );
    assert!(device.endpoint(Some("3")).is_none());
    // Endpoint names come from converter definitions, absent in R1.
    assert!(device.endpoint(Some("l1")).is_none());
}

#[test]
fn event_value_snapshot() {
    let _guard = serial();
    ZhDevice::reset_cache();

    let device = Device::without_converters(
        zh_device("Router", "0x0017880104e45519"),
        settings(json!({
            "mqtt": {"server": "localhost"},
            "devices": {"0x0017880104e45519": {"friendly_name": "lamp"}},
        })),
    );

    assert_eq!(
        device.event_value(),
        json!({
            "ID": "0x0017880104e45519",
            "ieeeAddr": "0x0017880104e45519",
            "name": "lamp",
            "type": "Router",
        })
    );
}
