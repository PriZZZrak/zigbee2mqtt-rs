// Original: .upstream/zigbee2mqtt/test/controller.test.ts (publishEntityState cases)
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::sync::{Arc, Mutex};

use serde_json::json;
use zigbee_apiarist::controller::model::device::Device as ZhDevice;
use zigbee_apiarist::controller::model::test_support::serial;

use z2m_rs::controller::Controller;
use z2m_rs::event_bus::EventBus;
use z2m_rs::model::Device;
use z2m_rs::mqtt::{Mqtt, MqttPublishOptions};
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;

fn temp(name: &str) -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir().join(format!("z2m-ctl-{name}-{}-{id}", std::process::id()))
}

fn zh_device() -> Arc<ZhDevice> {
    ZhDevice::from_database_entry(&json!({
        "id": 2,
        "type": "Router",
        "ieeeAddr": "0x0017880104e45519",
        "nwkAddr": 1234,
        "manufId": 4476,
        "manufName": "Test",
        "modelId": "TEST",
        "interviewCompleted": true,
        "endpoints": {"1": {"epId": 1, "profId": 260, "devId": 5, "inClusterList": [6], "outClusterList": []}},
    }))
}

fn harness(
    advanced: serde_json::Value,
) -> (
    Arc<Controller>,
    Arc<tokio::sync::Mutex<Mqtt>>,
    std::path::PathBuf,
) {
    let settings = Arc::new(Mutex::new(Settings::from_config(json!({
        "mqtt": {"server": "localhost", "base_topic": "zigbee2mqtt"},
        "advanced": advanced,
        "devices": {"0x0017880104e45519": {"friendly_name": "lamp"}},
    }))));
    let event_bus = Arc::new(Mutex::new(EventBus::new()));
    let logger = Arc::new(Mutex::new(Logger::new()));
    let state_path = temp("state.json");
    let state = Arc::new(Mutex::new(State::with_file(
        settings.clone(),
        event_bus.clone(),
        logger.clone(),
        state_path.clone(),
    )));
    let mqtt = Arc::new(tokio::sync::Mutex::new(Mqtt::new(event_bus.clone())));
    let zigbee = z2m_rs::zigbee::Zigbee::new(event_bus.clone(), settings.clone(), logger.clone());
    let controller = Arc::new(Controller::new(
        event_bus,
        zigbee,
        state,
        mqtt.clone(),
        settings,
        logger,
    ));
    (controller, mqtt, state_path)
}

fn device() -> Arc<Device> {
    Arc::new(Device::without_converters(
        zh_device(),
        Arc::new(Mutex::new(Settings::from_config(
            json!({"mqtt": {"server": "localhost"}, "devices": {"0x0017880104e45519": {"friendly_name": "lamp"}}}),
        ))),
    ))
}

#[tokio::test]
async fn publishes_state_with_entity_options_and_cache() {
    let _guard = serial();
    let (controller, mqtt, state_path) = harness(json!({"output": "json", "last_seen": "disable"}));

    controller
        .publish_entity_state(device(), json!({"state": "ON"}), None)
        .await;

    let mqtt = mqtt.lock().await;
    assert_eq!(mqtt.attempted.len(), 1);
    assert_eq!(mqtt.attempted[0].0, "zigbee2mqtt/lamp");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&mqtt.attempted[0].1).unwrap(),
        json!({"state": "ON"})
    );
    assert_eq!(mqtt.attempted[0].2.retain, false);
    assert_eq!(mqtt.attempted[0].2.qos, 0);
    assert!(mqtt.attempted[0].2.is_entity_state);
    let _ = std::fs::remove_file(state_path);
}

#[tokio::test]
async fn attribute_output_publishes_nested_topics() {
    let _guard = serial();
    let (controller, mqtt, state_path) =
        harness(json!({"output": "attribute", "last_seen": "disable"}));

    controller
        .publish_entity_state(
            device(),
            json!({"state": "ON", "color": {"r": 1, "g": 2, "b": 3}}),
            None,
        )
        .await;

    let mqtt = mqtt.lock().await;
    let topics: Vec<&str> = mqtt
        .attempted
        .iter()
        .map(|(topic, _, _)| topic.as_str())
        .collect();
    assert!(topics.contains(&"zigbee2mqtt/lamp/state"));
    assert!(topics.contains(&"zigbee2mqtt/lamp/color"));
    let color = mqtt
        .attempted
        .iter()
        .find(|(topic, _, _)| topic == "zigbee2mqtt/lamp/color")
        .map(|(_, payload, _)| payload.as_str())
        .unwrap();
    assert_eq!(color, "1,2,3");
    let _ = std::fs::remove_file(state_path);
}

#[test]
fn publish_options_default_is_empty() {
    assert_eq!(MqttPublishOptions::default().message_expiry_interval, None);
}
