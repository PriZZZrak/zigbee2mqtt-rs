// Original: .upstream/zigbee2mqtt/lib/controller.ts (publishEntityState, last_seen field)
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// Covers the upstream receive.test.ts cases
// "Should publish last_seen epoch / ISO_8601 / ISO_8601_local".

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use zigbee_apiarist::controller::controller::{Controller, ControllerOptions};
use zigbee_apiarist::controller::database::{Database, DeviceType};
use zigbee_apiarist::controller::model::device::{Device as ZhDevice, InterviewState};
use zigbee_apiarist::controller::model::entity::inject_database;
use zigbee_apiarist::controller::model::group::Group as ZhGroup;
use zigbee_apiarist::controller::model::test_support::serial;
use zigbee_apiarist::models::network_options::NetworkOptions;

use z2m_rs::controller::Controller as BridgeController;
use z2m_rs::event_bus::EventBus;
use z2m_rs::mqtt::Mqtt;
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::zigbee::Zigbee;

const IEEE: &str = "0x0017880104e45519";

fn temp(name: &str) -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir().join(format!("z2m-lastseen-{name}-{}-{id}", std::process::id()))
}

struct Harness {
    controller: Arc<BridgeController>,
    zigbee: Arc<Zigbee>,
    mqtt: Arc<tokio::sync::Mutex<Mqtt>>,
    settings: Arc<Mutex<Settings>>,
    database_path: std::path::PathBuf,
    state_path: std::path::PathBuf,
}

fn harness(last_seen_format: &str) -> Harness {
    let database_path = temp("db");
    let state_path = temp("state.json");
    inject_database(Arc::new(Mutex::new(Database::open(&database_path))));
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
        IEEE,
        1234,
        None,
        Some("Test".to_string()),
        None,
        Some("TEST".to_string()),
        InterviewState::Successful,
        None,
    )
    .unwrap();

    let zh_controller = Arc::new(
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

    let settings = Arc::new(Mutex::new(Settings::from_config(json!({
        "mqtt": {"server": "localhost"},
        "advanced": {"last_seen": last_seen_format, "cache_state": false},
        "devices": {IEEE: {"friendly_name": "lamp"}},
    }))));
    let event_bus = Arc::new(Mutex::new(EventBus::new()));
    let logger = Arc::new(Mutex::new(Logger::new()));
    let zigbee = Zigbee::with_controller(
        zh_controller,
        event_bus.clone(),
        settings.clone(),
        logger.clone(),
    );
    let state = Arc::new(Mutex::new(State::with_file(
        settings.clone(),
        event_bus.clone(),
        logger.clone(),
        state_path.clone(),
    )));
    let mqtt = Arc::new(tokio::sync::Mutex::new(Mqtt::new(event_bus.clone())));
    let controller = Arc::new(BridgeController::new(
        event_bus,
        zigbee.clone(),
        state,
        mqtt.clone(),
        settings.clone(),
        logger,
    ));
    let _ = zigbee.resolve_device(IEEE);

    Harness {
        controller,
        zigbee,
        mqtt,
        settings,
        database_path,
        state_path,
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.database_path);
        let _ = std::fs::remove_file(&self.state_path);
    }
}

async fn last_seen_payload(harness: &Harness) -> Value {
    let mqtt = harness.mqtt.lock().await;
    let (_, payload, _) = mqtt
        .attempted
        .iter()
        .rev()
        .find(|(topic, _, _)| topic == "zigbee2mqtt/lamp")
        .expect("state published");
    let payload: Value = serde_json::from_str(payload).unwrap();
    payload["last_seen"].clone()
}

const TIMESTAMP_MS: u64 = 1_700_000_000_000;

#[tokio::test]
async fn last_seen_epoch_is_published_as_a_number() {
    let _guard = serial();
    let harness = harness("epoch");
    let entity = harness.zigbee.resolve_device(IEEE).unwrap();
    entity.zh.set_last_seen(Some(TIMESTAMP_MS));

    harness
        .controller
        .publish_entity_state(
            entity.clone(),
            json!({"state": "ON"}),
            Some("devicePublish".to_string()),
        )
        .await;
    assert_eq!(last_seen_payload(&harness).await, json!(TIMESTAMP_MS));
}

#[tokio::test]
async fn last_seen_iso_8601_is_published_as_a_utc_string() {
    let _guard = serial();
    let harness = harness("ISO_8601");
    let entity = harness.zigbee.resolve_device(IEEE).unwrap();
    entity.zh.set_last_seen(Some(TIMESTAMP_MS));

    harness
        .controller
        .publish_entity_state(
            entity.clone(),
            json!({"state": "ON"}),
            Some("devicePublish".to_string()),
        )
        .await;
    assert_eq!(
        last_seen_payload(&harness).await,
        json!("2023-11-14T22:13:20.000Z")
    );
}

#[tokio::test]
async fn last_seen_iso_8601_local_is_published_with_an_offset() {
    let _guard = serial();
    let previous = std::env::var("TZ").ok();
    std::env::set_var("TZ", "Europe/Berlin");
    let harness = harness("ISO_8601_local");
    let entity = harness.zigbee.resolve_device(IEEE).unwrap();
    entity.zh.set_last_seen(Some(TIMESTAMP_MS));

    harness
        .controller
        .publish_entity_state(
            entity.clone(),
            json!({"state": "ON"}),
            Some("devicePublish".to_string()),
        )
        .await;
    let payload = last_seen_payload(&harness).await;
    let text = payload.as_str().expect("local ISO string");
    assert!(
        text.starts_with("2023-11-14T23:13:20.000"),
        "unexpected local timestamp {text}"
    );
    assert!(text.ends_with("+01:00"), "unexpected offset {text}");

    match previous {
        Some(value) => std::env::set_var("TZ", value),
        None => std::env::remove_var("TZ"),
    }
}

#[tokio::test]
async fn last_seen_is_omitted_when_disabled() {
    let _guard = serial();
    let harness = harness("disable");
    assert_eq!(harness.settings.lock().unwrap().get()["advanced"]["last_seen"], "disable");
    let entity = harness.zigbee.resolve_device(IEEE).unwrap();
    entity.zh.set_last_seen(Some(TIMESTAMP_MS));

    harness
        .controller
        .publish_entity_state(
            entity.clone(),
            json!({"state": "ON"}),
            Some("devicePublish".to_string()),
        )
        .await;
    assert!(last_seen_payload(&harness).await.is_null());
}
