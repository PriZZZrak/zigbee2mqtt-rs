// Original: .upstream/zigbee2mqtt/test/extensions/groups.test.ts
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
use z2m_rs::extension::extension::{Extension, ExtensionContext};
use z2m_rs::extension::groups::Groups;
use z2m_rs::model::entity::PublishEntityState;
use z2m_rs::mqtt::Mqtt;
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::zigbee::Zigbee;

const IEEE: &str = "0x0017880104e45519";

type Published = Arc<Mutex<Vec<(String, Value, Option<String>)>>>;

struct Harness {
    context: ExtensionContext,
    mqtt: Arc<tokio::sync::Mutex<Mqtt>>,
    event_bus: Arc<Mutex<EventBus>>,
    zigbee: Arc<Zigbee>,
    published: Published,
    database_path: std::path::PathBuf,
    state_path: std::path::PathBuf,
}

fn temp(name: &str) -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir().join(format!("z2m-groups-{name}-{}-{id}", std::process::id()))
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
        Some("Mains (single phase)".to_string()),
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
    let group = controller.create_group(1).unwrap();
    group.add_member(&device.endpoints()[0]);

    let settings = Arc::new(Mutex::new(Settings::from_config(json!({
        "mqtt": {"server": "localhost"},
        "devices": {IEEE: {"friendly_name": "lamp"}},
        "groups": {"1": {"friendly_name": "kitchen", "optimistic": true}},
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
    let _ = zigbee.resolve_group(1);
    let state_path = temp("state.json");
    let state = Arc::new(Mutex::new(State::with_file(
        settings.clone(),
        event_bus.clone(),
        logger.clone(),
        state_path.clone(),
    )));
    let mqtt = Arc::new(tokio::sync::Mutex::new(Mqtt::new(event_bus.clone())));
    let published: Published = Arc::new(Mutex::new(Vec::new()));
    let sink = published.clone();
    let publish: PublishEntityState = Arc::new(move |entity, payload, reason| {
        let sink = sink.clone();
        Box::pin(async move {
            sink.lock().unwrap().push((entity.name(), payload, reason));
        })
    });

    Harness {
        context: ExtensionContext {
            zigbee: zigbee.clone(),
            mqtt: mqtt.clone(),
            state,
            publish_entity_state: publish,
            event_bus: event_bus.clone(),
            logger,
            settings,
            enable_disable_extension: Arc::new(|_, _| Box::pin(async { Ok(()) })),
            restart: Arc::new(|| Box::pin(async {})),
            add_extension: Arc::new(|_| Box::pin(async {})),
            key: 3,
        },
        mqtt,
        event_bus,
        zigbee,
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

#[tokio::test]
async fn device_state_change_publishes_optimistic_group_state() {
    let _guard = serial();
    let harness = harness();
    let groups = Groups::new(harness.context.clone());
    groups.clone().start().await.unwrap();

    harness.event_bus.lock().unwrap().emit_state_change(json!({
        "entity": {"ID": IEEE, "ieeeAddr": IEEE, "name": "lamp"},
        "from": {},
        "to": {"state": "ON"},
        "update": {"state": "ON"},
        "reason": "devicePublish",
    }));
    tokio::time::sleep(Duration::from_millis(150)).await;

    let published = harness.published.lock().unwrap();
    assert_eq!(published.len(), 1);
    assert_eq!(published[0].0, "kitchen");
    assert_eq!(published[0].1, json!({"state": "ON"}));
    assert_eq!(published[0].2.as_deref(), Some("groupOptimistic"));
}

#[tokio::test]
async fn optimistic_group_state_is_deduplicated() {
    let _guard = serial();
    let harness = harness();
    let groups = Groups::new(harness.context.clone());
    groups.clone().start().await.unwrap();

    for _ in 0..2 {
        harness.event_bus.lock().unwrap().emit_state_change(json!({
            "entity": {"ID": IEEE, "ieeeAddr": IEEE, "name": "lamp"},
            "from": {},
            "to": {"state": "ON"},
            "update": {"state": "ON"},
            "reason": "devicePublish",
        }));
    }
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert_eq!(harness.published.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn invalid_member_request_returns_error_response() {
    let _guard = serial();
    let harness = harness();
    let groups = Groups::new(harness.context.clone());
    groups.clone().start().await.unwrap();

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": "zigbee2mqtt/bridge/request/group/members/add",
        "message": r#"{"group":"1"}"#,
    }));
    tokio::time::sleep(Duration::from_millis(150)).await;

    let mqtt = harness.mqtt.lock().await;
    let response = mqtt
        .attempted
        .iter()
        .find(|(topic, _, _)| topic == "zigbee2mqtt/bridge/response/group/members/add")
        .expect("response published");
    let payload: Value = serde_json::from_str(&response.1).unwrap();
    assert_eq!(payload["status"], "error");
    assert_eq!(payload["error"], "Invalid payload");
}

#[tokio::test]
async fn unknown_group_and_device_errors() {
    let _guard = serial();
    let harness = harness();
    let groups = Groups::new(harness.context.clone());
    groups.clone().start().await.unwrap();

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": "zigbee2mqtt/bridge/request/group/members/add",
        "message": r#"{"group":"99","device":"0x0017880104e45519"}"#,
    }));
    tokio::time::sleep(Duration::from_millis(150)).await;

    {
        let mqtt = harness.mqtt.lock().await;
        let response = mqtt
            .attempted
            .iter()
            .find(|(topic, _, _)| topic == "zigbee2mqtt/bridge/response/group/members/add")
            .unwrap();
        let payload: Value = serde_json::from_str(&response.1).unwrap();
        assert_eq!(payload["error"], "Group '99' does not exist");
    }

    harness.mqtt.lock().await.attempted.clear();
    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": "zigbee2mqtt/bridge/request/group/members/add",
        "message": r#"{"group":"1","device":"0xdeadbeefdeadbeef"}"#,
    }));
    tokio::time::sleep(Duration::from_millis(150)).await;

    let mqtt = harness.mqtt.lock().await;
    let response = mqtt
        .attempted
        .iter()
        .find(|(topic, _, _)| topic == "zigbee2mqtt/bridge/response/group/members/add")
        .unwrap();
    let payload: Value = serde_json::from_str(&response.1).unwrap();
    assert_eq!(
        payload["error"],
        "Device '0xdeadbeefdeadbeef' does not exist"
    );
}
