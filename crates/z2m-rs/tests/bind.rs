// Original: .upstream/zigbee2mqtt/test/extensions/bind.test.ts
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
use zigbee_apiarist::models::network_options::NetworkOptions;

use z2m_rs::event_bus::EventBus;
use z2m_rs::extension::bind::Bind;
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
    database_path: std::path::PathBuf,
    state_path: std::path::PathBuf,
}

fn temp(name: &str) -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir().join(format!("z2m-bind-{name}-{}-{id}", std::process::id()))
}

fn harness() -> Harness {
    let database_path = temp("db");
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
    let device = ZhDevice::create(
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
    device.create_endpoint(1).unwrap();

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
    let state_path = temp("state.json");
    let state = Arc::new(Mutex::new(State::with_file(
        settings.clone(),
        event_bus.clone(),
        logger.clone(),
        state_path.clone(),
    )));
    let mqtt = Arc::new(tokio::sync::Mutex::new(Mqtt::new(event_bus.clone())));
    let publish: PublishEntityState = Arc::new(|_e, _p, _r| Box::pin(async {}));

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
            key: 4,
        },
        mqtt,
        event_bus,
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

async fn send(harness: &Harness, topic: &str, message: Value) {
    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": format!("zigbee2mqtt/bridge/request/device/{topic}"),
        "message": message.to_string(),
    }));
}

async fn response(harness: &Harness, topic: &str) -> Value {
    let full = format!("zigbee2mqtt/bridge/response/device/{topic}");
    for _ in 0..100 {
        if let Some(payload) = harness
            .mqtt
            .lock()
            .await
            .attempted
            .iter()
            .find(|(published, _, _)| published == &full)
            .map(|(_, payload, _)| payload.clone())
        {
            return serde_json::from_str(&payload).unwrap();
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("no response for {topic}");
}

#[tokio::test]
async fn invalid_payload_errors() {
    let _guard = serial();
    let harness = harness();
    let bind = Bind::new(harness.context.clone());
    bind.clone().start().await.unwrap();

    send(&harness, "bind", json!({"from": IEEE})).await;
    let payload = response(&harness, "bind").await;
    assert_eq!(payload["status"], "error");
    assert_eq!(payload["error"], "Invalid payload");

    harness.mqtt.lock().await.attempted.clear();
    send(
        &harness,
        "bind",
        json!({"from": "0xdeadbeefdeadbeef", "to": IEEE}),
    )
    .await;
    let payload = response(&harness, "bind").await;
    assert_eq!(
        payload["error"],
        "Source device '0xdeadbeefdeadbeef' does not exist"
    );

    harness.mqtt.lock().await.attempted.clear();
    send(
        &harness,
        "bind",
        json!({"from": IEEE, "to": "0xdeadbeefdeadbeef"}),
    )
    .await;
    let payload = response(&harness, "bind").await;
    assert_eq!(
        payload["error"],
        "Target device or group '0xdeadbeefdeadbeef' does not exist"
    );

    harness.mqtt.lock().await.attempted.clear();
    send(
        &harness,
        "bind",
        json!({"from": IEEE, "from_endpoint": "9", "to": IEEE}),
    )
    .await;
    let payload = response(&harness, "bind").await;
    assert_eq!(
        payload["error"],
        "Source device 'lamp' does not have endpoint '9'"
    );
}

#[tokio::test]
async fn bind_without_supported_clusters_reports_nothing_to_bind() {
    let _guard = serial();
    let harness = harness();
    let bind = Bind::new(harness.context.clone());
    bind.clone().start().await.unwrap();

    send(&harness, "bind", json!({"from": IEEE, "to": IEEE})).await;
    let payload = response(&harness, "bind").await;
    assert_eq!(payload["status"], "error");
    assert_eq!(payload["error"], "Nothing to bind");
}

#[tokio::test]
async fn binds_clear_validates_payload_and_target() {
    let _guard = serial();
    let harness = harness();
    let bind = Bind::new(harness.context.clone());
    bind.clone().start().await.unwrap();

    send(&harness, "binds/clear", json!({"target": 123})).await;
    let payload = response(&harness, "binds/clear").await;
    assert_eq!(payload["error"], "Invalid payload");

    harness.mqtt.lock().await.attempted.clear();
    send(
        &harness,
        "binds/clear",
        json!({"target": "0xdeadbeefdeadbeef"}),
    )
    .await;
    let payload = response(&harness, "binds/clear").await;
    assert_eq!(payload["error"], "Invalid target");
}
