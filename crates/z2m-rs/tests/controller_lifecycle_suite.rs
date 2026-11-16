//! Port of upstream `test/controller.test.ts` (z2m lifecycle/state cases).
//!
//! Anchor: `.upstream/zigbee2mqtt/test/controller.test.ts` (zigbee2mqtt 2.14.3,
//! `3c841fdf`), 90 `it` cases. Per-case mapping:
//! `docs/z2m-controller-test-parity.md`.
//!
//! Upstream mocks `Zigbee`/`Mqtt`/`Controller` with vitest spies; the cases
//! that need a running coordinator or an MQTT broker cannot run here and are
//! mapped to existing extension suites or the HIL run instead. The cases that
//! do not need either (join acceptance, extension registry, topic validation,
//! state cache loading) are ported below.

use std::sync::{Arc, Mutex};

use serde_json::json;

use z2m_rs::controller::Controller;
use z2m_rs::event_bus::EventBus;
use z2m_rs::mqtt::Mqtt;
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::zigbee::Zigbee;

struct Harness {
    settings: Arc<Mutex<Settings>>,
    event_bus: Arc<Mutex<EventBus>>,
    logger: Arc<Mutex<Logger>>,
    zigbee: Arc<Zigbee>,
    mqtt: Arc<tokio::sync::Mutex<Mqtt>>,
    state_path: std::path::PathBuf,
}

fn harness(settings: serde_json::Value) -> Harness {
    let settings = Arc::new(Mutex::new(Settings::from_config(settings)));
    let event_bus = Arc::new(Mutex::new(EventBus::new()));
    let logger = Arc::new(Mutex::new(Logger::new()));
    let zigbee = Zigbee::new(event_bus.clone(), settings.clone(), logger.clone());
    let state_path = std::env::temp_dir().join(format!(
        "z2m-controller-suite-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mqtt = Arc::new(tokio::sync::Mutex::new(Mqtt::new(event_bus.clone())));
    let _ = std::fs::remove_file(&state_path);
    Harness {
        settings,
        event_bus,
        logger,
        zigbee,
        mqtt,
        state_path,
    }
}

fn controller(harness: &Harness) -> Arc<Controller> {
    let state = Arc::new(Mutex::new(State::with_file(
        harness.settings.clone(),
        harness.event_bus.clone(),
        harness.logger.clone(),
        harness.state_path.clone(),
    )));
    Arc::new(Controller::new(
        harness.event_bus.clone(),
        harness.zigbee.clone(),
        state,
        harness.mqtt.clone(),
        harness.settings.clone(),
        harness.logger.clone(),
    ))
}

fn with_lists(ieee: &str, passlist: Vec<&str>, blocklist: Vec<&str>) -> Harness {
    harness(json!({
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "mqtt://localhost"},
        "passlist": passlist,
        "blocklist": blocklist,
        "devices": {ieee: {"friendly_name": "device"}},
    }))
}

// --- acceptJoiningDeviceHandler (cases 44-49) ------------------------------

#[tokio::test]
async fn accept_joining_rejects_device_on_blocklist() {
    let harness = with_lists("0x0017880104e45519", vec![], vec!["0x0017880104e45519"]);
    assert!(!harness.zigbee.accept_joining_device_handler("0x0017880104e45519"));
}

#[tokio::test]
async fn accept_joining_accepts_device_not_on_blocklist() {
    let harness = with_lists("0x0017880104e45519", vec![], vec!["0x0017880104e45511"]);
    assert!(harness.zigbee.accept_joining_device_handler("0x0017880104e45519"));
}

#[tokio::test]
async fn accept_joining_accepts_device_on_passlist() {
    let harness = with_lists("0x0017880104e45519", vec!["0x0017880104e45519"], vec![]);
    assert!(harness.zigbee.accept_joining_device_handler("0x0017880104e45519"));
}

#[tokio::test]
async fn accept_joining_rejects_device_not_in_passlist() {
    let harness = with_lists("0x0017880104e45519", vec!["0x0017880104e45511"], vec![]);
    assert!(!harness.zigbee.accept_joining_device_handler("0x0017880104e45519"));
}

#[tokio::test]
async fn accept_joining_prefers_passlist_above_blocklist() {
    let harness = with_lists(
        "0x0017880104e45519",
        vec!["0x0017880104e45519"],
        vec!["0x0017880104e45519"],
    );
    assert!(harness.zigbee.accept_joining_device_handler("0x0017880104e45519"));
}

#[tokio::test]
async fn accept_joining_accepts_when_not_on_blocklist_and_passlist() {
    let harness = with_lists("0x0017880104e45519", vec![], vec![]);
    assert!(harness.zigbee.accept_joining_device_handler("0x0017880104e45519"));
}

// --- topic validation (case 10) -------------------------------------------

#[tokio::test]
async fn does_not_publish_wildcard_topics() {
    let harness = harness(json!({"mqtt": {"base_topic": "z2m", "server": "mqtt://localhost"}}));
    let payload = serde_json::to_vec(&json!({"a": 1})).unwrap();
    let mut mqtt = harness.mqtt.lock().await;
    // Wildcard topics are rejected before any publish attempt (upstream logs
    // and skips them); a normal topic would go through the publish path.
    assert!(!mqtt.publish("z2m/+/state", payload.clone(), Default::default()).await);
    assert!(!mqtt.publish("z2m/#", payload.clone(), Default::default()).await);
    assert!(!mqtt.is_connected(), "no broker in this unit test");
    assert_eq!(
        mqtt.attempted
            .iter()
            .filter(|(topic, _, _)| topic.contains('+') || topic.contains('#'))
            .count(),
        0,
        "wildcard topics must not reach the publish attempt log"
    );
}

// --- state cache loading (cases 11, 72, 75) --------------------------------

#[tokio::test]
async fn loads_empty_state_when_state_file_does_not_exist() {
    let harness = harness(json!({"mqtt": {"server": "mqtt://localhost"}}));
    let state = Arc::new(Mutex::new(State::with_file(
        harness.settings.clone(),
        harness.event_bus.clone(),
        harness.logger.clone(),
        harness.state_path.clone(),
    )));
    State::start(&state);
    assert!(state.lock().unwrap().entries().is_empty());
}

#[tokio::test]
async fn starts_when_state_file_is_corrupted() {
    let harness = harness(json!({"mqtt": {"server": "mqtt://localhost"}}));
    std::fs::write(&harness.state_path, "{not json").unwrap();
    let state = Arc::new(Mutex::new(State::with_file(
        harness.settings.clone(),
        harness.event_bus.clone(),
        harness.logger.clone(),
        harness.state_path.clone(),
    )));
    State::start(&state);
    assert!(state.lock().unwrap().entries().is_empty());
    let _ = std::fs::remove_file(&harness.state_path);
}

// --- extension registry (cases 89, 90) -------------------------------------

#[tokio::test]
async fn prevents_interacting_with_invalid_extensions() {
    let harness = harness(json!({"mqtt": {"server": "mqtt://localhost"}}));
    let controller = controller(&harness);
    assert!(controller.make_extension("Bridge").is_some());
    assert!(controller.make_extension("NotAnExtension").is_none());
    let error = controller
        .enable_disable_extension(false, "Bridge".to_string())
        .await
        .unwrap_err();
    assert!(error.contains("Built-in extension Bridge cannot be disabled"));
}

#[tokio::test]
async fn throws_when_adding_an_extension_twice() {
    let harness = harness(json!({"mqtt": {"server": "mqtt://localhost"}}));
    let controller = controller(&harness);
    let first = controller.make_extension("Bridge").expect("bridge");
    controller.add_extension(first.clone()).expect("first add");
    let error = controller.add_extension(first).unwrap_err();
    assert_eq!(error, "Extension 'Bridge' already exists");
}
