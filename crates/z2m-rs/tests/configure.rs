// Original: .upstream/zigbee2mqtt/test/extensions/configure.test.ts
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
use z2m_rs::extension::configure::Configure;
use z2m_rs::extension::extension::{Extension, ExtensionContext};
use z2m_rs::model::entity::PublishEntityState;
use z2m_rs::mqtt::Mqtt;
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::zigbee::Zigbee;
use zigbee_converters::Definition;

const IEEE: &str = "0x0017880104e45519";

struct Harness {
    context: ExtensionContext,
    mqtt: Arc<tokio::sync::Mutex<Mqtt>>,
    event_bus: Arc<Mutex<EventBus>>,
    zigbee: Arc<Zigbee>,
    logger: Arc<Mutex<Logger>>,
    database_path: std::path::PathBuf,
    state_path: std::path::PathBuf,
}

fn temp(name: &str) -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir().join(format!("z2m-configure-{name}-{}-{id}", std::process::id()))
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
            zigbee: zigbee.clone(),
            mqtt: mqtt.clone(),
            state,
            publish_entity_state: publish,
            event_bus: event_bus.clone(),
            logger: logger.clone(),
            settings,
            enable_disable_extension: Arc::new(|_, _| Box::pin(async { Ok(()) })),
            restart: Arc::new(|| Box::pin(async {})),
            add_extension: Arc::new(|_| Box::pin(async {})),
            key: 9,
        },
        mqtt,
        event_bus,
        zigbee,
        logger,
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

fn registration(harness: &Harness, calls: Arc<Mutex<Vec<String>>>) -> Definition {
    let mut definition = Definition::new("TEST", "Acme", "Test").with_zigbee_model("TEST");
    definition.version = "1.2.3".to_string();
    definition.configure = Some(Arc::new(move |_device, _coordinator, _definition| {
        let calls = calls.clone();
        Box::pin(async move {
            calls.lock().unwrap().push("configure".to_string());
            Ok(())
        })
    }));
    let _ = harness;
    definition
}

async fn wait_for_publish(mqtt: &Arc<tokio::sync::Mutex<Mqtt>>) -> Value {
    for _ in 0..100 {
        if !mqtt.lock().await.attempted.is_empty() {
            let mqtt = mqtt.lock().await;
            return serde_json::from_str(&mqtt.attempted[0].1).unwrap();
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("no response published");
}

#[tokio::test]
async fn mqtt_request_configures_and_responds() {
    let _guard = serial();
    let harness = harness();
    let calls = Arc::new(Mutex::new(Vec::new()));
    harness
        .zigbee
        .converters()
        .lock()
        .unwrap()
        .register(registration(&harness, calls.clone()));
    let configure = Configure::new(harness.context.clone());
    configure.clone().start().await.unwrap();

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": "zigbee2mqtt/bridge/request/device/configure",
        "message": format!(r#"{{"id":"{IEEE}"}}"#),
    }));

    let response = wait_for_publish(&harness.mqtt).await;
    assert_eq!(response["status"], "ok");
    assert_eq!(response["data"]["id"], IEEE);
    assert_eq!(calls.lock().unwrap().as_slice(), ["configure".to_string()]);

    let zh = harness.zigbee.resolve_device(IEEE).unwrap().zh.clone();
    assert_eq!(zh.meta.lock().unwrap()["configured"], json!("1.2.3"));
}

#[tokio::test]
async fn unknown_device_returns_error() {
    let _guard = serial();
    let harness = harness();
    let configure = Configure::new(harness.context.clone());
    configure.clone().start().await.unwrap();

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": "zigbee2mqtt/bridge/request/device/configure",
        "message": r#"{"id":"0xdeadbeefdeadbeef"}"#,
    }));

    let response = wait_for_publish(&harness.mqtt).await;
    assert_eq!(response["status"], "error");
    assert_eq!(response["data"], json!({}));
    assert_eq!(
        response["error"],
        "Device '0xdeadbeefdeadbeef' does not exist"
    );
}

#[tokio::test]
async fn device_without_configure_returns_error() {
    let _guard = serial();
    let harness = harness();
    let mut definition = Definition::new("TEST", "Acme", "Test").with_zigbee_model("TEST");
    definition.version = "1.2.3".to_string();
    harness
        .zigbee
        .converters()
        .lock()
        .unwrap()
        .register(definition);
    let configure = Configure::new(harness.context.clone());
    configure.clone().start().await.unwrap();

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": "zigbee2mqtt/bridge/request/device/configure",
        "message": format!(r#"{{"id":"{IEEE}"}}"#),
    }));

    let response = wait_for_publish(&harness.mqtt).await;
    assert_eq!(response["status"], "error");
    assert_eq!(response["error"], "Device 'lamp' cannot be configured");
}
