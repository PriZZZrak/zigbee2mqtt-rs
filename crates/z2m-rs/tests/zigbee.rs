// Original: .upstream/zigbee2mqtt/test/controller.test.ts (upstream exercises the
// Zigbee wrapper through the controller suite; no dedicated zigbee.test.ts).
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// Exercises the definition-independent `Zigbee` surface against the real
// zigbee-herdsman model caches (no adapter is started).

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use zigbee_apiarist::controller::controller::{Controller, ControllerOptions};
use zigbee_apiarist::controller::database::Database;
use zigbee_apiarist::controller::database::DeviceType;
use zigbee_apiarist::controller::model::device::{Device as ZhDevice, InterviewState};
use zigbee_apiarist::controller::model::entity::inject_database;
use zigbee_apiarist::controller::model::group::Group as ZhGroup;
use zigbee_apiarist::controller::model::test_support::serial;
use zigbee_apiarist::models::network_options::NetworkOptions;

use z2m_rs::event_bus::EventBus;
use z2m_rs::model::entity::EntityId;
use z2m_rs::model::Entity;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::zigbee::Zigbee;

struct Harness {
    zigbee: Arc<Zigbee>,
    settings: Arc<Mutex<Settings>>,
    logger: Arc<Mutex<Logger>>,
    database_path: std::path::PathBuf,
}

fn temp_path(name: &str) -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir().join(format!("z2m-zigbee-{name}-{}-{id}.db", std::process::id()))
}

fn harness(configuration: Value) -> Harness {
    let database_path = temp_path("db");
    let database = Arc::new(Mutex::new(Database::open(&database_path)));
    inject_database(database.clone());

    ZhDevice::reset_cache();
    ZhGroup::reset_cache();

    ZhDevice::create(
        DeviceType::Coordinator,
        "0x00124b0024c16b95",
        0,
        None,
        None,
        None,
        None,
        InterviewState::Successful,
        None,
    )
    .unwrap();
    ZhDevice::create(
        DeviceType::Router,
        "0x0017880104e45519",
        1234,
        Some(4476),
        Some("Test".to_string()),
        Some("Mains (single phase)".to_string()),
        Some("TEST".to_string()),
        InterviewState::Successful,
        None,
    )
    .unwrap();

    let controller = Arc::new(
        Controller::new(ControllerOptions {
            network: NetworkOptions {
                pan_id: 0x1a62,
                extended_pan_id: vec![0xdd; 8],
                channel_list: vec![11],
                network_key: vec![1, 3, 5, 7, 9, 11, 13, 15, 0, 2, 4, 6, 8, 10, 12, 13],
                network_key_distribute: true,
                has_default_extended_pan_id: false,
            },
            transport: json!({}),
            database_path: database_path.display().to_string(),
            database_backup_path: None,
            backup_path: None,
            adapter: json!({}),
            accept_joining_device_handler: None,
        })
        .unwrap(),
    );

    let settings = Arc::new(Mutex::new(Settings::from_config(configuration)));
    let mut logger = Logger::new();
    logger.level = "debug".to_string();
    let logger = Arc::new(Mutex::new(logger));

    let zigbee = Zigbee::with_controller(
        controller,
        Arc::new(Mutex::new(EventBus::new())),
        settings.clone(),
        logger.clone(),
    );

    Harness {
        zigbee,
        settings,
        logger,
        database_path,
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.database_path);
    }
}

#[test]
fn resolve_device_adds_devices_to_settings() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));

    let device = harness
        .zigbee
        .resolve_device("0x0017880104e45519")
        .expect("router");

    assert_eq!(device.name(), "0x0017880104e45519");
    assert_eq!(device.zh.r#type(), DeviceType::Router);
    assert_eq!(
        harness
            .settings
            .lock()
            .unwrap()
            .get_device("0x0017880104e45519")
            .and_then(|device| device.get("friendly_name").cloned()),
        Some(json!("0x0017880104e45519"))
    );
    assert!(harness
        .zigbee
        .resolve_device("0x0017880104e45520")
        .is_none());
}

#[test]
fn resolve_device_uses_friendly_name_from_settings() {
    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "devices": {"0x0017880104e45519": {"friendly_name": "lamp"}},
    }));

    let device = harness
        .zigbee
        .resolve_device("0x0017880104e45519")
        .expect("router");

    assert_eq!(device.name(), "lamp");
    assert_eq!(
        device.entity_id(),
        EntityId::Ieee("0x0017880104e45519".to_string())
    );
}

#[test]
fn resolve_entity_by_name_coordinator_and_group() {
    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "devices": {"0x0017880104e45519": {"friendly_name": "lamp"}},
        "groups": {"1": {"friendly_name": "kitchen"}},
    }));

    assert_eq!(
        harness
            .zigbee
            .resolve_entity("lamp")
            .map(|entity| entity.name()),
        Some("lamp".to_string())
    );
    assert_eq!(
        harness
            .zigbee
            .resolve_entity("Coordinator")
            .map(|entity| entity.name()),
        Some("Coordinator".to_string())
    );

    let group = harness
        .zigbee
        .resolve_entity("kitchen")
        .expect("group created from settings");
    assert_eq!(group.entity_id(), EntityId::Group(1));
    assert_eq!(group.name(), "kitchen");

    // Resolves through the herdsman group lookup from now on.
    assert!(harness.zigbee.group_by_id(1).is_some());
    assert!(harness.zigbee.resolve_entity("missing").is_none());
}

#[test]
fn resolve_entity_and_endpoint_splits_on_last_slash() {
    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "devices": {"0x0017880104e45519": {"friendly_name": "lamp"}},
    }));

    let zh_device = harness.zigbee.resolve_device("0x0017880104e45519").unwrap();
    zh_device.zh.create_endpoint(1).unwrap();
    zh_device.zh.create_endpoint(2).unwrap();

    let exact = harness.zigbee.resolve_entity_and_endpoint("lamp");
    assert_eq!(exact.ID(), "lamp");
    assert!(exact.endpoint_id.is_none());
    assert_eq!(exact.endpoint.as_ref().map(|endpoint| endpoint.ID), Some(1));

    let with_endpoint = harness.zigbee.resolve_entity_and_endpoint("lamp/2");
    assert_eq!(with_endpoint.ID(), "lamp");
    assert_eq!(with_endpoint.endpoint_id.as_deref(), Some("2"));
    assert_eq!(
        with_endpoint.endpoint.as_ref().map(|endpoint| endpoint.ID),
        Some(2)
    );

    let unknown_endpoint = harness.zigbee.resolve_entity_and_endpoint("lamp/9");
    assert!(unknown_endpoint.entity.is_some());
    assert!(unknown_endpoint.endpoint.is_none());

    let missing = harness.zigbee.resolve_entity_and_endpoint("missing/2");
    assert!(missing.entity.is_none());
    assert_eq!(missing.endpoint_id.as_deref(), Some("2"));
}

#[test]
fn passlist_and_blocklist_handle_joining_devices() {
    let _guard = serial();
    let passlist_harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "passlist": ["0x0017880104e45519"],
    }));

    assert!(passlist_harness
        .zigbee
        .accept_joining_device_handler("0x0017880104e45519"));
    assert!(!passlist_harness
        .zigbee
        .accept_joining_device_handler("0x0017880104e45520"));

    let blocklist_harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "blocklist": ["0x0017880104e45520"],
        "passlist": [],
    }));
    assert!(blocklist_harness
        .zigbee
        .accept_joining_device_handler("0x0017880104e45519"));
    assert!(!blocklist_harness
        .zigbee
        .accept_joining_device_handler("0x0017880104e45520"));

    let messages: Vec<String> = blocklist_harness
        .logger
        .lock()
        .unwrap()
        .winston_logs
        .iter()
        .map(|(_, message)| message.clone())
        .collect();
    assert!(messages.contains(
        &"z2m: Accepting joining not in blocklist device '0x0017880104e45519'".to_string()
    ));
    assert!(messages.contains(
        &"z2m: Rejecting joining device which is on blocklist '0x0017880104e45520'".to_string()
    ));
}

#[test]
fn lookups_and_iterators() {
    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "devices": {"0x0017880104e45519": {"friendly_name": "lamp"}},
        "groups": {"1": {"friendly_name": "kitchen"}},
    }));
    let _ = harness.zigbee.resolve_entity("kitchen").unwrap();

    let devices = harness.zigbee.devices_iterator(|_| true);
    assert_eq!(devices.len(), 2);
    let non_coordinators = harness
        .zigbee
        .devices_iterator(|device| device.r#type() != DeviceType::Coordinator);
    assert_eq!(non_coordinators.len(), 1);
    assert_eq!(
        harness
            .zigbee
            .devices_and_groups_iterator()
            .into_iter()
            .filter(|entity| entity.is_group())
            .count(),
        1
    );

    assert!(harness
        .zigbee
        .remove_device_from_lookup("0x0017880104e45519"));
    assert!(!harness
        .zigbee
        .remove_device_from_lookup("0x0017880104e45519"));
    assert!(harness.zigbee.remove_group_from_lookup(1));
    assert!(!harness.zigbee.remove_group_from_lookup(1));
}

#[test]
fn default_state_of_unstarted_controller() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));

    assert!(!harness.zigbee.is_stopping());
    assert!(!harness.zigbee.get_permit_join());
    assert!(harness.zigbee.get_permit_join_end().is_none());
}
