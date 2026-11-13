// Original: .upstream/zigbee2mqtt/test/extensions/availability.test.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;
use zigbee_apiarist::controller::controller::{Controller, ControllerOptions};
use zigbee_apiarist::controller::database::{Database, DeviceType};
use zigbee_apiarist::controller::model::device::{Device as ZhDevice, InterviewState};
use zigbee_apiarist::controller::model::entity::inject_database;
use zigbee_apiarist::controller::model::group::Group as ZhGroup;
use zigbee_apiarist::controller::model::test_support::serial;
use zigbee_apiarist::models::network_options::NetworkOptions;

use z2m_rs::event_bus::EventBus;
use z2m_rs::extension::availability::Availability;
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
    zigbee: Arc<Zigbee>,
    database_path: std::path::PathBuf,
    state_path: std::path::PathBuf,
}

fn temp(name: &str) -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir().join(format!("z2m-avail-{name}-{}-{id}", std::process::id()))
}

fn harness(availability_enabled: bool, device_availability: serde_json::Value) -> Harness {
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
        DeviceType::EndDevice,
        IEEE,
        1234,
        None,
        Some("Test".to_string()),
        Some("Battery".to_string()),
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
        "availability": {"enabled": availability_enabled},
        "devices": {IEEE: {"friendly_name": "lamp", "availability": device_availability}},
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
            logger,
            settings,
            enable_disable_extension: Arc::new(|_, _| Box::pin(async { Ok(()) })),
            restart: Arc::new(|| Box::pin(async {})),
            add_extension: Arc::new(|_| Box::pin(async {})),
            key: 5,
        },
        mqtt,
        event_bus,
        zigbee,
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
async fn publishes_offline_then_online_after_last_seen() {
    let _guard = serial();
    let harness = harness(true, json!({"timeout": 1}));
    let availability = Availability::new(harness.context.clone());
    // Device has no lastSeen yet.
    availability.clone().start().await.unwrap();

    {
        let mqtt = harness.mqtt.lock().await;
        let topic = format!("zigbee2mqtt/lamp/availability");
        let offline = mqtt
            .attempted
            .iter()
            .find(|(published, _, _)| published == &topic)
            .expect("availability published");
        assert_eq!(offline.1, r#"{"state":"offline"}"#);
        assert!(offline.2.retain);
        assert_eq!(offline.2.qos, 1);
    }

    let device = harness.zigbee.resolve_device(IEEE).unwrap();
    device.zh.update_last_seen();
    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_last_seen_changed(json!({
            "device": {"ieeeAddr": IEEE, "name": "lamp"},
            "reason": "messageEmitted",
        }));
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mqtt = harness.mqtt.lock().await;
    let topic = "zigbee2mqtt/lamp/availability";
    let last = mqtt
        .attempted
        .iter()
        .filter(|(published, _, _)| published == topic)
        .last()
        .expect("online published");
    assert_eq!(last.1, r#"{"state":"online"}"#);

    availability.clone().stop().await.unwrap();
}

#[tokio::test]
async fn availability_disabled_via_device_option() {
    let _guard = serial();
    let harness = harness(true, json!(false));
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();

    let mqtt = harness.mqtt.lock().await;
    assert!(
        mqtt.attempted.is_empty(),
        "unexpected: {:?}",
        mqtt.attempted
            .iter()
            .map(|(t, p, _)| (t.clone(), p.clone()))
            .collect::<Vec<_>>()
    );
    availability.stop().await.unwrap();
}

#[tokio::test]
async fn rename_publishes_empty_retained_availability() {
    let _guard = serial();
    let harness = harness(true, json!({"timeout": 1}));
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    harness.mqtt.lock().await.attempted.clear();

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_entity_renamed(json!({
            "entity": {"ID": IEEE, "ieeeAddr": IEEE, "name": "lamp"},
            "from": "old",
            "to": "lamp",
        }));
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mqtt = harness.mqtt.lock().await;
    assert!(mqtt.attempted.iter().any(|(topic, payload, options)| topic
        == "zigbee2mqtt/old/availability"
        && payload.is_empty()
        && options.retain
        && options.qos == 1));
    assert!(mqtt
        .attempted
        .iter()
        .any(|(topic, _, _)| topic == "zigbee2mqtt/lamp/availability"));

    availability.stop().await.unwrap();
}
