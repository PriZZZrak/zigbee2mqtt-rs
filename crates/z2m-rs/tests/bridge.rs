// Original: .upstream/zigbee2mqtt/test/extensions/bridge.test.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};
use zigbee_apiarist::controller::controller::{Controller, ControllerOptions};
use zigbee_apiarist::controller::database::{Database, DeviceType};
use zigbee_apiarist::controller::model::device::{Device as ZhDevice, InterviewState};
use zigbee_apiarist::controller::model::entity::inject_database;
use zigbee_apiarist::controller::model::group::Group as ZhGroup;
use zigbee_apiarist::controller::model::test_support::serial;
use zigbee_apiarist::zspec::zcl::definition::enums::DataType;
use zigbee_apiarist::zspec::zcl::utils::{Attribute, Cluster};
use zigbee_apiarist::models::network_options::NetworkOptions;

use z2m_rs::event_bus::EventBus;
use z2m_rs::extension::bridge::Bridge;
use z2m_rs::extension::extension::{Extension, ExtensionContext};
use z2m_rs::model::entity::PublishEntityState;
use z2m_rs::mqtt::Mqtt;
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::zigbee::Zigbee;

const IEEE: &str = "0x0017880104e45519";

struct Harness {
    context: ExtensionContext,
    mqtt: Arc<tokio::sync::Mutex<Mqtt>>,
    event_bus: Arc<Mutex<EventBus>>,
    published: Arc<Mutex<Vec<(String, Value)>>>,
    database_path: std::path::PathBuf,
    state_path: std::path::PathBuf,
}

fn temp(name: &str) -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir().join(format!("z2m-bridge-{name}-{}-{id}", std::process::id()))
}

fn harness() -> Harness {
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

    let settings = Arc::new(Mutex::new(Settings::from_config(json!({
        "mqtt": {"server": "localhost"},
        "devices": {IEEE: {"friendly_name": "lamp"}},
        "advanced": {"log_level": "info"},
    }))));
    let event_bus = Arc::new(Mutex::new(EventBus::new()));
    let logger = Arc::new(Mutex::new(Logger::new()));
    let zigbee = Zigbee::with_controller(
        controller,
        event_bus.clone(),
        settings.clone(),
        logger.clone(),
    );
    let _ = zigbee.resolve_device(IEEE);
    let state = Arc::new(Mutex::new(State::with_file(
        settings.clone(),
        event_bus.clone(),
        logger.clone(),
        state_path.clone(),
    )));
    let mqtt = Arc::new(tokio::sync::Mutex::new(Mqtt::new(event_bus.clone())));
    let published = Arc::new(Mutex::new(Vec::new()));
    let sink = published.clone();
    let publish: PublishEntityState = Arc::new(move |entity, payload, _reason| {
        let sink = sink.clone();
        Box::pin(async move {
            sink.lock().unwrap().push((entity.name(), payload));
        })
    });

    Harness {
        context: ExtensionContext {
            zigbee,
            mqtt: mqtt.clone(),
            state,
            publish_entity_state: publish,
            event_bus: event_bus.clone(),
            logger,
            settings,
            enable_disable_extension: Arc::new(|_, _| Box::pin(async { Ok(()) })),
            restart: Arc::new(|| Box::pin(async {})),
            add_extension: Arc::new(|_| Box::pin(async {})),
            key: 8,
        },
        mqtt,
        event_bus,
        published,
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

async fn send(harness: &Harness, request: &str, message: Value) {
    harness.mqtt.lock().await.attempted.clear();
    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": format!("zigbee2mqtt/bridge/request/{request}"),
        "message": message.to_string(),
    }));
}

async fn response(harness: &Harness, request: &str) -> Value {
    let topic = format!("zigbee2mqtt/bridge/response/{request}");
    for _ in 0..100 {
        if let Some(payload) = harness
            .mqtt
            .lock()
            .await
            .attempted
            .iter()
            .rev()
            .find(|(published, _, _)| published == &topic)
            .map(|(_, payload, _)| payload.clone())
        {
            return serde_json::from_str(&payload).unwrap();
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("no response for {request}");
}

#[tokio::test]
async fn start_publishes_bridge_topics() {
    let _guard = serial();
    let harness = harness();

    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    let mqtt = harness.mqtt.lock().await;
    for topic in [
        "zigbee2mqtt/bridge/info",
        "zigbee2mqtt/bridge/devices",
        "zigbee2mqtt/bridge/groups",
        "zigbee2mqtt/bridge/definitions",
    ] {
        let entry = mqtt
            .attempted
            .iter()
            .find(|(published, _, _)| published == topic)
            .unwrap_or_else(|| panic!("missing {topic}"));
        assert!(entry.2.retain);
        assert!(entry.2.skip_log);
    }

    let devices: Value = serde_json::from_str(
        &mqtt
            .attempted
            .iter()
            .find(|(topic, _, _)| topic == "zigbee2mqtt/bridge/devices")
            .unwrap()
            .1,
    )
    .unwrap();
    let device = devices
        .as_array()
        .unwrap()
        .iter()
        .find(|device| device["ieee_address"] == IEEE)
        .expect("device in bridge/devices");
    assert_eq!(device["friendly_name"], "lamp");
    assert_eq!(device["supported"], false);

    let info: Value = serde_json::from_str(
        &mqtt
            .attempted
            .iter()
            .find(|(topic, _, _)| topic == "zigbee2mqtt/bridge/info")
            .unwrap()
            .1,
    )
    .unwrap();
    assert_eq!(info["config"]["mqtt"]["password"], Value::Null);
    assert_eq!(info["restart_required"], false);
}

#[tokio::test]
async fn health_check_and_invalid_options() {
    let _guard = serial();
    let harness = harness();
    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    send(&harness, "health_check", json!("")).await;
    let payload = response(&harness, "health_check").await;
    assert_eq!(payload["status"], "ok");
    assert_eq!(payload["data"]["healthy"], true);

    send(&harness, "options", json!({"bad": true})).await;
    let payload = response(&harness, "options").await;
    assert_eq!(payload["status"], "error");
    assert_eq!(payload["error"], "Invalid payload");

    send(
        &harness,
        "options",
        json!({"options": {"advanced": {"log_level": "debug"}}}),
    )
    .await;
    let payload = response(&harness, "options").await;
    if payload["status"] != "ok" {
        panic!("{payload}");
    }
    assert_eq!(payload["data"]["restart_required"], false);
}

#[tokio::test]
async fn group_add_and_rename_device() {
    let _guard = serial();
    let harness = harness();
    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    send(
        &harness,
        "group/add",
        json!({"friendly_name": "kitchen", "id": 1}),
    )
    .await;
    let payload = response(&harness, "group/add").await;
    assert_eq!(payload["status"], "ok");
    assert_eq!(payload["data"]["friendly_name"], "kitchen");
    assert_eq!(payload["data"]["id"], 1);

    send(
        &harness,
        "device/rename",
        json!({"from": IEEE, "to": "desk_lamp"}),
    )
    .await;
    let payload = response(&harness, "device/rename").await;
    assert_eq!(payload["status"], "ok");
    assert_eq!(payload["data"]["from"], "lamp");
    assert_eq!(payload["data"]["to"], "desk_lamp");

    assert_eq!(
        harness
            .context
            .settings
            .lock()
            .unwrap()
            .get_device(IEEE)
            .unwrap()["friendly_name"],
        "desk_lamp"
    );
    assert!(harness
        .mqtt
        .lock()
        .await
        .attempted
        .iter()
        .any(|(topic, payload, options)| topic == "zigbee2mqtt/lamp"
            && payload.is_empty()
            && options.retain));
}

#[tokio::test]
async fn definitions_include_custom_clusters() {
    let _guard = serial();
    let harness = harness();
    let device = harness.context.zigbee.resolve_device(IEEE).unwrap();
    let mut cluster = Cluster::new("manuSpecificTest", 0xfc00);
    cluster
        .attributes
        .insert("mode".to_string(), Attribute::new("mode", 0x0000, DataType::UINT8));
    device
        .zh
        .add_custom_cluster("manuSpecificTest", cluster)
        .unwrap();

    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    let mqtt = harness.mqtt.lock().await;
    let payload: Value = serde_json::from_str(
        &mqtt
            .attempted
            .iter()
            .find(|(topic, _, _)| topic == "zigbee2mqtt/bridge/definitions")
            .unwrap()
            .1,
    )
    .unwrap();
    let cluster = &payload["custom_clusters"][IEEE]["manuSpecificTest"];
    assert_eq!(cluster["name"], "manuSpecificTest", "payload={}", payload["custom_clusters"]);
    assert_eq!(cluster["ID"], 0xfc00);
    assert_eq!(cluster["attributes"]["mode"]["name"], "mode");
    assert_eq!(cluster["attributes"]["mode"]["ID"], 0);
    assert_eq!(cluster["attributes"]["mode"]["type"], 32);
    assert_eq!(cluster["attributes"]["mode"]["read"], true);
}

#[tokio::test]
async fn force_remove_device_without_adapter() {
    let _guard = serial();
    let harness = harness();
    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    send(
        &harness,
        "device/remove",
        json!({"id": IEEE, "force": true, "keep_config": false}),
    )
    .await;
    let payload = response(&harness, "device/remove").await;
    assert_eq!(payload["status"], "ok");
    assert_eq!(payload["data"]["force"], true);
    assert_eq!(
        harness.context.settings.lock().unwrap().get_device(IEEE),
        None
    );
}
