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

/// Build a definition whose `configure` counts calls and fails while
/// `fail_until` is greater than the current call count.
fn failing_registration(
    calls: Arc<Mutex<Vec<String>>>,
    fail_until: usize,
    delay_ms: u64,
) -> Definition {
    let mut definition = Definition::new("TEST", "Acme", "Test").with_zigbee_model("TEST");
    definition.version = "1.2.3".to_string();
    let counter = Arc::new(Mutex::new(0usize));
    definition.configure = Some(Arc::new(move |_device, _coordinator, _definition| {
        let calls = calls.clone();
        let counter = counter.clone();
        Box::pin(async move {
            if delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
            let mut count = counter.lock().unwrap();
            *count += 1;
            calls.lock().unwrap().push("configure".to_string());
            if *count <= fail_until {
                Err(format!("configure failure {count}"))
            } else {
                Ok(())
            }
        })
    }));
    definition
}

fn emit_interview(harness: &Harness) {
    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_device_interview(json!({
            "device": {"ieeeAddr": IEEE, "name": "lamp"},
            "status": "successful",
        }));
}

/// Case 17: at most three automatic configure attempts.
#[tokio::test]
async fn stops_after_three_failed_attempts() {
    let _guard = serial();
    let harness = harness();
    let calls = Arc::new(Mutex::new(Vec::new()));
    harness
        .zigbee
        .converters()
        .lock()
        .unwrap()
        .register(failing_registration(calls.clone(), usize::MAX, 0));
    harness
        .zigbee
        .resolve_device(IEEE)
        .unwrap()
        .resolve_definition(false);
    let configure = Configure::new(harness.context.clone());
    configure.clone().start().await.unwrap();

    for _ in 0..5 {
        emit_interview(&harness);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(
        calls.lock().unwrap().len(),
        3,
        "configure must stop after 3 failed attempts"
    );
    configure.stop().await.unwrap();
}

/// Case 18: a successful attempt before the limit is reached configures the
/// device and records the definition version.
#[tokio::test]
async fn configures_after_a_couple_of_failures() {
    let _guard = serial();
    let harness = harness();
    let calls = Arc::new(Mutex::new(Vec::new()));
    harness
        .zigbee
        .converters()
        .lock()
        .unwrap()
        .register(failing_registration(calls.clone(), 2, 0));
    harness
        .zigbee
        .resolve_device(IEEE)
        .unwrap()
        .resolve_definition(false);
    let configure = Configure::new(harness.context.clone());
    configure.clone().start().await.unwrap();

    for _ in 0..3 {
        emit_interview(&harness);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(calls.lock().unwrap().len(), 3);
    let zh = harness.zigbee.resolve_device(IEEE).unwrap().zh.clone();
    assert_eq!(zh.meta.lock().unwrap()["configured"], json!("1.2.3"));
    configure.stop().await.unwrap();
}

/// Case 16: a configure that is already running is not started a second time.
#[tokio::test]
async fn does_not_configure_twice_concurrently() {
    let _guard = serial();
    let harness = harness();
    let calls = Arc::new(Mutex::new(Vec::new()));
    harness
        .zigbee
        .converters()
        .lock()
        .unwrap()
        .register(failing_registration(calls.clone(), 0, 300));
    harness
        .zigbee
        .resolve_device(IEEE)
        .unwrap()
        .resolve_definition(false);
    let configure = Configure::new(harness.context.clone());
    configure.clone().start().await.unwrap();

    emit_interview(&harness);
    tokio::time::sleep(Duration::from_millis(20)).await;
    emit_interview(&harness);
    tokio::time::sleep(Duration::from_millis(600)).await;

    assert_eq!(
        calls.lock().unwrap().len(),
        1,
        "the in-flight guard must skip the second configure"
    );
    configure.stop().await.unwrap();
}

/// Case 9: a definition version of 0.0.0 never triggers a re-configure.
#[tokio::test]
async fn does_not_reconfigure_when_the_definition_version_is_zero() {
    let _guard = serial();
    let harness = harness();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut definition = failing_registration(calls.clone(), 0, 0);
    definition.version = "0.0.0".to_string();
    harness
        .zigbee
        .converters()
        .lock()
        .unwrap()
        .register(definition);
    harness
        .zigbee
        .resolve_device(IEEE)
        .unwrap()
        .resolve_definition(false);
    let configure = Configure::new(harness.context.clone());
    configure.clone().start().await.unwrap();

    emit_interview(&harness);
    tokio::time::sleep(Duration::from_millis(50)).await;
    emit_interview(&harness);
    tokio::time::sleep(Duration::from_millis(100)).await;

    // The device is configured once and never re-configured afterwards.
    assert_eq!(calls.lock().unwrap().len(), 1, "0.0.0 must not reconfigure");
    let zh = harness.zigbee.resolve_device(IEEE).unwrap().zh.clone();
    assert_eq!(zh.meta.lock().unwrap()["configured"], json!("0.0.0"));
    configure.stop().await.unwrap();
}

/// Case 4: disabled devices are skipped by the automatic configure path.
#[tokio::test]
async fn does_not_configure_disabled_devices() {
    let _guard = serial();
    let harness = harness();
    harness.context.settings.lock().unwrap().configuration["devices"][IEEE]["disabled"] =
        json!(true);
    let calls = Arc::new(Mutex::new(Vec::new()));
    harness
        .zigbee
        .converters()
        .lock()
        .unwrap()
        .register(failing_registration(calls.clone(), 0, 0));
    harness
        .zigbee
        .resolve_device(IEEE)
        .unwrap()
        .resolve_definition(false);
    let configure = Configure::new(harness.context.clone());
    configure.clone().start().await.unwrap();

    emit_interview(&harness);
    tokio::time::sleep(Duration::from_millis(150)).await;

    assert!(
        calls.lock().unwrap().is_empty(),
        "disabled device must not be configured"
    );
    configure.stop().await.unwrap();
}

const END_DEVICE: &str = "0x0017880104e45511";

/// Definition whose `configure` records the device IEEE and can fail.
fn recording_registration(
    calls: Arc<Mutex<Vec<String>>>,
    version: &str,
    fail: bool,
) -> Definition {
    let mut definition = Definition::new("TEST", "Acme", "Test").with_zigbee_model("TEST");
    definition.version = version.to_string();
    definition.configure = Some(Arc::new(move |device, _coordinator, _definition| {
        let calls = calls.clone();
        Box::pin(async move {
            calls.lock().unwrap().push(device.ieee_addr());
            if fail {
                Err("configure failure".to_string())
            } else {
                Ok(())
            }
        })
    }));
    definition
}

fn add_end_device(harness: &Harness) {
    ZhDevice::create(
        DeviceType::EndDevice,
        END_DEVICE,
        1235,
        None,
        Some("Test".to_string()),
        Some("Battery".to_string()),
        Some("TEST".to_string()),
        InterviewState::Successful,
        None,
    )
    .unwrap();
    harness.context.settings.lock().unwrap().configuration["devices"][END_DEVICE] =
        json!({"friendly_name": "sensor"});
    let _ = harness.zigbee.resolve_device(END_DEVICE);
}

fn emit_last_seen(harness: &Harness, ieee: &str) {
    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_last_seen_changed(json!({
            "device": {"ieeeAddr": ieee},
            "reason": "messageEmitted",
        }));
}

/// Cases 1 and 2: the startup pass configures routers but never end devices.
#[tokio::test(start_paused = true)]
async fn configures_routers_but_not_end_devices_on_startup() {
    let _guard = serial();
    let harness = harness();
    add_end_device(&harness);
    let calls = Arc::new(Mutex::new(Vec::new()));
    harness
        .zigbee
        .converters()
        .lock()
        .unwrap()
        .register(recording_registration(calls.clone(), "1.2.3", false));
    harness.zigbee.resolve_device(IEEE).unwrap().resolve_definition(false);
    harness
        .zigbee
        .resolve_device(END_DEVICE)
        .unwrap()
        .resolve_definition(false);

    let configure = Configure::new(harness.context.clone());
    configure.clone().start().await.unwrap();
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(30)).await;
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }

    let calls = calls.lock().unwrap().clone();
    assert!(calls.contains(&IEEE.to_string()), "router was not configured: {calls:?}");
    assert!(!calls.contains(&END_DEVICE.to_string()), "end device was configured: {calls:?}");
    configure.stop().await.unwrap();
}

/// Case 3: a rejoining device is configured again even when already configured.
#[tokio::test]
async fn reconfigures_when_the_device_rejoins() {
    let _guard = serial();
    let harness = harness();
    let calls = Arc::new(Mutex::new(Vec::new()));
    harness
        .zigbee
        .converters()
        .lock()
        .unwrap()
        .register(recording_registration(calls.clone(), "1.2.3", false));
    harness.zigbee.resolve_device(IEEE).unwrap().resolve_definition(false);
    let configure = Configure::new(harness.context.clone());
    configure.clone().start().await.unwrap();

    emit_interview(&harness);
    tokio::time::sleep(Duration::from_millis(80)).await;
    assert_eq!(calls.lock().unwrap().len(), 1);

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_device_joined(json!({"device": {"ieeeAddr": IEEE}}));
    tokio::time::sleep(Duration::from_millis(120)).await;
    assert_eq!(calls.lock().unwrap().len(), 2, "rejoin must re-configure");
    configure.stop().await.unwrap();
}

/// Case 5: a reconfigure event re-runs configure.
#[tokio::test]
async fn reconfigures_on_reconfigure_event() {
    let _guard = serial();
    let harness = harness();
    let calls = Arc::new(Mutex::new(Vec::new()));
    harness
        .zigbee
        .converters()
        .lock()
        .unwrap()
        .register(recording_registration(calls.clone(), "1.2.3", false));
    harness.zigbee.resolve_device(IEEE).unwrap().resolve_definition(false);
    let configure = Configure::new(harness.context.clone());
    configure.clone().start().await.unwrap();

    emit_interview(&harness);
    tokio::time::sleep(Duration::from_millis(80)).await;

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_reconfigure(json!({"device": {"ieeeAddr": IEEE}}));
    tokio::time::sleep(Duration::from_millis(120)).await;
    assert_eq!(calls.lock().unwrap().len(), 2, "reconfigure event must re-run");
    configure.stop().await.unwrap();
}

/// Case 6: an already configured device is not configured twice.
#[tokio::test]
async fn does_not_configure_twice_when_already_configured() {
    let _guard = serial();
    let harness = harness();
    let calls = Arc::new(Mutex::new(Vec::new()));
    harness
        .zigbee
        .converters()
        .lock()
        .unwrap()
        .register(recording_registration(calls.clone(), "1.2.3", false));
    harness.zigbee.resolve_device(IEEE).unwrap().resolve_definition(false);
    let configure = Configure::new(harness.context.clone());
    configure.clone().start().await.unwrap();

    emit_interview(&harness);
    tokio::time::sleep(Duration::from_millis(80)).await;
    emit_interview(&harness);
    tokio::time::sleep(Duration::from_millis(120)).await;

    assert_eq!(calls.lock().unwrap().len(), 1, "second configure must be skipped");
    configure.stop().await.unwrap();
}

/// Case 7: a zigbee message configures a device that has no configured marker.
#[tokio::test]
async fn configures_on_zigbee_message_when_not_configured_yet() {
    let _guard = serial();
    let harness = harness();
    let calls = Arc::new(Mutex::new(Vec::new()));
    harness
        .zigbee
        .converters()
        .lock()
        .unwrap()
        .register(recording_registration(calls.clone(), "1.2.3", false));
    harness.zigbee.resolve_device(IEEE).unwrap().resolve_definition(false);
    let configure = Configure::new(harness.context.clone());
    configure.clone().start().await.unwrap();

    emit_last_seen(&harness, IEEE);
    tokio::time::sleep(Duration::from_millis(80)).await;
    assert_eq!(calls.lock().unwrap().len(), 1, "message must configure once");

    emit_last_seen(&harness, IEEE);
    tokio::time::sleep(Duration::from_millis(120)).await;
    assert_eq!(calls.lock().unwrap().len(), 1, "configured device stays configured");
    configure.stop().await.unwrap();
}

/// Case 8: changing the definition version re-configures the device.
#[tokio::test]
async fn reconfigures_when_the_definition_version_changes() {
    let _guard = serial();
    let harness = harness();
    let calls = Arc::new(Mutex::new(Vec::new()));
    harness
        .zigbee
        .converters()
        .lock()
        .unwrap()
        .register(recording_registration(calls.clone(), "1.2.3", false));
    harness.zigbee.resolve_device(IEEE).unwrap().resolve_definition(false);
    let configure = Configure::new(harness.context.clone());
    configure.clone().start().await.unwrap();

    emit_interview(&harness);
    tokio::time::sleep(Duration::from_millis(80)).await;
    assert_eq!(calls.lock().unwrap().len(), 1);

    // Simulate a device configured with an older definition version.
    let zh = harness.zigbee.resolve_device(IEEE).unwrap().zh.clone();
    zh.meta.lock().unwrap()["configured"] = json!("0.9.0");
    emit_interview(&harness);
    tokio::time::sleep(Duration::from_millis(120)).await;

    assert_eq!(calls.lock().unwrap().len(), 2, "new version must re-configure");
    assert_eq!(zh.meta.lock().unwrap()["configured"], json!("1.2.3"));
    configure.stop().await.unwrap();
}

/// Case 12: a failing configure is reported through the MQTT response.
#[tokio::test]
async fn mqtt_request_reports_configure_failure() {
    let _guard = serial();
    let harness = harness();
    harness
        .zigbee
        .converters()
        .lock()
        .unwrap()
        .register(recording_registration(Arc::new(Mutex::new(Vec::new())), "1.2.3", true));
    harness.zigbee.resolve_device(IEEE).unwrap().resolve_definition(false);
    let configure = Configure::new(harness.context.clone());
    configure.clone().start().await.unwrap();

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": "zigbee2mqtt/bridge/request/device/configure",
        "message": format!(r#"{{"id":"{IEEE}"}}"#),
    }));
    let response = wait_for_publish(&harness.mqtt).await;
    assert_eq!(response["status"], "error");
    assert!(
        response["error"]
            .as_str()
            .unwrap_or_default()
            .contains("Failed to configure"),
        "{response}"
    );
    configure.stop().await.unwrap();
}

/// Case 15: a device that is still interviewing is not configured.
#[tokio::test]
async fn does_not_configure_when_interview_is_not_completed() {
    let _guard = serial();
    let harness = harness();
    ZhDevice::create(
        DeviceType::Router,
        END_DEVICE,
        1235,
        None,
        Some("Test".to_string()),
        None,
        Some("TEST".to_string()),
        InterviewState::InProgress,
        None,
    )
    .unwrap();
    harness.context.settings.lock().unwrap().configuration["devices"][END_DEVICE] =
        json!({"friendly_name": "pending"});
    let _ = harness.zigbee.resolve_device(END_DEVICE);
    let calls = Arc::new(Mutex::new(Vec::new()));
    harness
        .zigbee
        .converters()
        .lock()
        .unwrap()
        .register(recording_registration(calls.clone(), "1.2.3", false));
    harness
        .zigbee
        .resolve_device(END_DEVICE)
        .unwrap()
        .resolve_definition(false);
    let configure = Configure::new(harness.context.clone());
    configure.clone().start().await.unwrap();

    emit_interview(&harness);
    tokio::time::sleep(Duration::from_millis(120)).await;
    assert!(
        !calls.lock().unwrap().contains(&END_DEVICE.to_string()),
        "device still interviewing must not be configured"
    );
    configure.stop().await.unwrap();
}

/// Case 19: an MQTT configure request (force) runs after the automatic attempts
/// were exhausted.
#[tokio::test]
async fn force_configures_after_failed_attempts() {
    let _guard = serial();
    let harness = harness();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let failing = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let mut definition = Definition::new("TEST", "Acme", "Test").with_zigbee_model("TEST");
    definition.version = "1.2.3".to_string();
    {
        let calls = calls.clone();
        let failing = failing.clone();
        definition.configure = Some(Arc::new(move |device, _c, _d| {
            let calls = calls.clone();
            let failing = failing.clone();
            Box::pin(async move {
                calls.lock().unwrap().push(device.ieee_addr());
                if failing.load(std::sync::atomic::Ordering::SeqCst) {
                    Err("configure failure".to_string())
                } else {
                    Ok(())
                }
            })
        }));
    }
    harness
        .zigbee
        .converters()
        .lock()
        .unwrap()
        .register(definition);
    harness.zigbee.resolve_device(IEEE).unwrap().resolve_definition(false);
    let configure = Configure::new(harness.context.clone());
    configure.clone().start().await.unwrap();

    for _ in 0..4 {
        emit_interview(&harness);
        tokio::time::sleep(Duration::from_millis(60)).await;
    }
    assert_eq!(calls.lock().unwrap().len(), 3, "automatic attempts stop at 3");

    failing.store(false, std::sync::atomic::Ordering::SeqCst);
    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": "zigbee2mqtt/bridge/request/device/configure",
        "message": format!(r#"{{"id":"{IEEE}"}}"#),
    }));
    let response = wait_for_publish(&harness.mqtt).await;
    assert_eq!(response["status"], "ok", "{response}");
    assert_eq!(calls.lock().unwrap().len(), 4, "force must run a fourth attempt");
    configure.stop().await.unwrap();
}
