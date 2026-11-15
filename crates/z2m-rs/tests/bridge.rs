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
    enable_calls: Arc<Mutex<Vec<(bool, String)>>>,
    restarts: Arc<Mutex<usize>>,
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
    let enable_calls = Arc::new(Mutex::new(Vec::new()));
    let restarts = Arc::new(Mutex::new(0usize));
    let restart_sink = restarts.clone();
    let enable_sink = enable_calls.clone();
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
            enable_disable_extension: Arc::new(move |enabled, name| {
                let sink = enable_sink.clone();
                Box::pin(async move {
                    sink.lock().unwrap().push((enabled, name));
                    Ok(())
                })
            }),
            restart: Arc::new(move || {
                let sink = restart_sink.clone();
                Box::pin(async move {
                    *sink.lock().unwrap() += 1;
                })
            }),
            add_extension: Arc::new(|_| Box::pin(async {})),
            key: 8,
        },
        mqtt,
        event_bus,
        published,
        enable_calls,
        restarts,
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

#[tokio::test]
async fn options_apply_arrays_null_and_report_restart_required() {
    let _guard = serial();
    let harness = harness();
    {
        let mut settings = harness.context.settings.lock().unwrap();
        settings.set(&["advanced", "ext_pan_id"], json!([221, 221, 221, 221, 221, 221, 221, 221]));
        settings.set(&["serial"], json!({"disable_led": false, "port": "/dev/dummy"}));
    }
    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    // Upstream `Change options array`.
    send(
        &harness,
        "options",
        json!({"options": {"advanced": {"ext_pan_id": [220, 221, 221, 221, 221, 221, 221, 221]}}}),
    )
    .await;
    let payload = response(&harness, "options").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    assert_eq!(payload["data"]["restart_required"], true);
    assert_eq!(
        harness.context.settings.lock().unwrap().get()["advanced"]["ext_pan_id"],
        json!([220, 221, 221, 221, 221, 221, 221, 221])
    );

    // Upstream `Change options with null`.
    send(
        &harness,
        "options",
        json!({"options": {"serial": {"disable_led": false, "port": null}}}),
    )
    .await;
    let payload = response(&harness, "options").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    assert_eq!(payload["data"]["restart_required"], true);
    assert_eq!(
        harness.context.settings.lock().unwrap().get()["serial"],
        json!({"disable_led": false})
    );

    // Upstream `Change options not valid against schema`.
    send(
        &harness,
        "options",
        json!({"options": {"advanced": {"log_level": "not-a-level"}}}),
    )
    .await;
    let payload = response(&harness, "options").await;
    assert_eq!(payload["status"], "error", "{payload}");
    assert!(!payload["error"].as_str().unwrap_or_default().is_empty());
}

#[tokio::test]
async fn logging_publishes_to_mqtt_with_dedupe_and_connection_guard() {
    let _guard = serial();
    let harness = harness();
    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    // Not connected: log lines stay local (upstream transport guard).
    harness
        .context
        .logger
        .lock()
        .unwrap()
        .info("not connected", "z2m");
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!harness
        .mqtt
        .lock()
        .await
        .attempted
        .iter()
        .any(|(topic, _, _)| topic == "zigbee2mqtt/bridge/logging"));

    harness.mqtt.lock().await.connected = true;

    {
        let mut logger = harness.context.logger.lock().unwrap();
        logger.info("this is a test", "z2m");
        logger.info("this is a test", "z2m"); // duplicate, must not publish twice
    }
    tokio::time::sleep(Duration::from_millis(150)).await;

    {
        let mqtt = harness.mqtt.lock().await;
        let published: Vec<&(String, String, z2m_rs::mqtt::MqttPublishOptions)> = mqtt
            .attempted
            .iter()
            .filter(|(topic, _, _)| topic == "zigbee2mqtt/bridge/logging")
            .collect();
        assert_eq!(published.len(), 1, "{:?}", published);
        let payload: Value = serde_json::from_str(&published[0].1).unwrap();
        assert_eq!(
            payload,
            json!({"message": "this is a test", "level": "info", "namespace": "z2m"})
        );
    }

    harness.mqtt.lock().await.attempted.clear();
    harness
        .context
        .logger
        .lock()
        .unwrap()
        .debug("this is a test", "z2m");
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!harness
        .mqtt
        .lock()
        .await
        .attempted
        .iter()
        .any(|(topic, _, _)| topic == "zigbee2mqtt/bridge/logging"));
}

#[tokio::test]
async fn bridge_info_reports_the_real_runtime_version() {
    let _guard = serial();
    let harness = harness();
    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    let mqtt = harness.mqtt.lock().await;
    let (_, payload, _) = mqtt
        .attempted
        .iter()
        .rev()
        .find(|(topic, _, _)| topic == "zigbee2mqtt/bridge/info")
        .expect("bridge info");
    let payload: Value = serde_json::from_str(payload).unwrap();
    let node_version = payload["os"]["node_version"]
        .as_str()
        .expect("os.node_version string");
    assert!(node_version.starts_with("rustc "), "{node_version}");
    assert!(
        node_version
            .split_whitespace()
            .nth(1)
            .is_some_and(|version| version.chars().next().is_some_and(|c| c.is_ascii_digit())),
        "{node_version}"
    );
}

#[tokio::test]
async fn options_apply_homeassistant_and_log_levels() {
    let _guard = serial();
    let harness = harness();
    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    // Upstream `Change options and apply - homeassistant`.
    send(
        &harness,
        "options",
        json!({"options": {"homeassistant": {"enabled": true}}}),
    )
    .await;
    let payload = response(&harness, "options").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    assert_eq!(
        harness.enable_calls.lock().unwrap().as_slice(),
        [(true, "HomeAssistant".to_string())]
    );

    // Upstream `Change options and apply - log_level / log_debug_namespace_ignore
    // / log_namespaced_levels`.
    send(
        &harness,
        "options",
        json!({"options": {"advanced": {
            "log_level": "debug",
            "log_namespaced_levels": {"zh:zstack": "warning"},
            "log_debug_namespace_ignore": "^zh:.*",
        }}}),
    )
    .await;
    let payload = response(&harness, "options").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    let logger = harness.context.logger.lock().unwrap();
    assert_eq!(logger.level, "debug");
    assert_eq!(
        logger.namespaced_levels.get("zh:zstack").map(String::as_str),
        Some("warning")
    );
    assert_eq!(logger.get_debug_namespace_ignore(), "^zh:.*");
}

#[tokio::test]
async fn interview_request_validates_payload_and_entity() {
    let _guard = serial();
    let harness = harness();
    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    for (message, error) in [
        (json!({"options": {}}), "Invalid payload".to_string()),
        (json!({"id": "missing"}), "Device 'missing' does not exist".to_string()),
        (
            json!({"id": format!("{IEEE}/1")}),
            format!("Device '{IEEE}/1' does not exist"),
        ),
        (json!({"id": "group_1"}), "Device 'group_1' does not exist".to_string()),
    ] {
        send(&harness, "device/interview", message).await;
        let payload = response(&harness, "device/interview").await;
        assert_eq!(payload["status"], "error", "{payload}");
        assert_eq!(payload["error"], error);
    }
}

#[tokio::test]
async fn device_and_group_options_apply_and_warn() {
    let _guard = serial();
    let harness = harness();
    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    // Upstream `Should allow change device options`.
    send(
        &harness,
        "device/options",
        json!({"id": "lamp", "options": {"retain": false, "transition": 1}}),
    )
    .await;
    let payload = response(&harness, "device/options").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    assert_eq!(payload["data"]["restart_required"], false);
    assert_eq!(payload["data"]["id"], "lamp");
    let settings = harness.context.settings.lock().unwrap();
    let device = settings.get_device("lamp").expect("device settings");
    assert_eq!(device["retain"], false);
    assert_eq!(device["transition"], 1);
    drop(settings);

    // Upstream `Should warn on unsupported device option`.
    send(
        &harness,
        "device/options",
        json!({"id": "lamp", "options": {"unsupported": true}}),
    )
    .await;
    let payload = response(&harness, "device/options").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    assert!(harness
        .context
        .logger
        .lock()
        .unwrap()
        .winston_logs
        .iter()
        .any(|(_, line)| line.contains("does not support option 'unsupported'")));

    // Invalid payloads and entity mismatches.
    send(&harness, "device/options", json!({"id": "lamp"})).await;
    let payload = response(&harness, "device/options").await;
    assert_eq!(payload["error"], "Invalid payload");

    send(
        &harness,
        "group/options",
        json!({"id": "1", "options": {"retain": true}}),
    )
    .await;
    let payload = response(&harness, "group/options").await;
    assert_eq!(payload["status"], "error", "{payload}");
    assert_eq!(payload["error"], "Group '1' does not exist");
}

#[tokio::test]
async fn reporting_configure_and_read_error_paths() {
    let _guard = serial();
    let harness = harness();
    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    // Upstream `Should throw error when configure reporting is called with
    // malformed payload / for non-existing device / endpoint`.
    for (message, error) in [
        (json!({"id": "lamp"}), "Invalid payload".to_string()),
        (
            json!({
                "id": "missing",
                "endpoint": 1,
                "cluster": "genOnOff",
                "attribute": "onOff",
                "minimum_report_interval": 0,
                "maximum_report_interval": 100,
            }),
            "Device 'missing' does not exist".to_string(),
        ),
        (
            json!({
                "id": "lamp",
                "endpoint": 9,
                "cluster": "genOnOff",
                "attribute": "onOff",
                "minimum_report_interval": 0,
                "maximum_report_interval": 100,
            }),
            "Device 'lamp' does not have endpoint '9'".to_string(),
        ),
    ] {
        send(&harness, "device/reporting/configure", message).await;
        let payload = response(&harness, "device/reporting/configure").await;
        assert_eq!(payload["status"], "error", "{payload}");
        assert_eq!(payload["error"], error);
    }

    // Upstream `Should throw error when read reporting config is called with
    // malformed payload / for non-existing device / endpoint`.
    for (message, error) in [
        (json!({"id": "lamp"}), "Invalid payload".to_string()),
        (
            json!({"id": "missing", "endpoint": 1, "cluster": "genOnOff", "configs": []}),
            "Device 'missing' does not exist".to_string(),
        ),
        (
            json!({"id": "lamp", "endpoint": 9, "cluster": "genOnOff", "configs": []}),
            "Device 'lamp' does not have endpoint '9'".to_string(),
        ),
    ] {
        send(&harness, "device/reporting/read", message).await;
        let payload = response(&harness, "device/reporting/read").await;
        assert_eq!(payload["status"], "error", "{payload}");
        assert_eq!(payload["error"], error);
    }
}

#[tokio::test]
async fn rename_trims_validates_and_reports_errors() {
    let _guard = serial();
    let harness = harness();
    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    // Upstream `Should trim input when renaming device`.
    send(
        &harness,
        "device/rename",
        json!({"from": "lamp", "to": "  lamp2  "}),
    )
    .await;
    let payload = response(&harness, "device/rename").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    assert_eq!(payload["data"]["to"], "lamp2");
    assert_eq!(
        harness.context.settings.lock().unwrap().get_device("lamp2").map(|device| device["friendly_name"].clone()),
        Some(json!("lamp2"))
    );

    // Upstream `Shouldnt allow rename device with to not allowed name containing
    // a wildcard`.
    send(
        &harness,
        "device/rename",
        json!({"from": "lamp2", "to": "living_room/blinds#"}),
    )
    .await;
    let payload = response(&harness, "device/rename").await;
    assert_eq!(payload["status"], "error");
    assert_eq!(
        payload["error"],
        "MQTT wildcard (+ and #) not allowed in friendly_name ('living_room/blinds#')"
    );

    // Invalid payload, unknown entity and `last` without a joined device.
    send(&harness, "device/rename", json!({"from": "lamp2"})).await;
    let payload = response(&harness, "device/rename").await;
    assert_eq!(payload["error"], "Invalid payload");

    send(
        &harness,
        "device/rename",
        json!({"from": "missing", "to": "x"}),
    )
    .await;
    let payload = response(&harness, "device/rename").await;
    assert_eq!(payload["error"], "Device 'missing' does not exist");

    send(&harness, "device/rename", json!({"last": true, "to": "x"})).await;
    let payload = response(&harness, "device/rename").await;
    assert_eq!(payload["error"], "No device has joined since start");
}

#[tokio::test]
async fn force_remove_device_blocks_and_keeps_configuration() {
    let _guard = serial();
    let harness = harness();
    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    send(
        &harness,
        "device/remove",
        json!({"id": "lamp", "force": true, "block": true, "keep_config": true}),
    )
    .await;
    let payload = response(&harness, "device/remove").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    assert_eq!(payload["data"]["force"], true);

    let settings = harness.context.settings.lock().unwrap();
    assert!(settings
        .get()
        .get("blocklist")
        .and_then(Value::as_array)
        .is_some_and(|list| list.contains(&json!(IEEE))));
    assert!(
        settings.get_device("lamp").is_some(),
        "keep_config removed the device settings"
    );
    drop(settings);
    assert!(harness.context.zigbee.resolve_device(IEEE).is_none());

    // Unknown device is rejected.
    send(
        &harness,
        "device/remove",
        json!({"id": "missing", "force": true}),
    )
    .await;
    let payload = response(&harness, "device/remove").await;
    assert_eq!(payload["error"], "Device 'missing' does not exist");
}

#[tokio::test]
async fn force_remove_group_removes_settings() {
    let _guard = serial();
    let harness = harness();
    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    send(
        &harness,
        "group/add",
        json!({"friendly_name": "kitchen"}),
    )
    .await;
    let payload = response(&harness, "group/add").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    let group_id = payload["data"]["id"].as_u64().expect("group id");

    send(
        &harness,
        "group/remove",
        json!({"id": group_id, "force": true}),
    )
    .await;
    let payload = response(&harness, "group/remove").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    assert!(harness.context.zigbee.group_by_id(group_id).is_none());
    assert!(harness
        .context
        .settings
        .lock()
        .unwrap()
        .get()["groups"]
        .get(&group_id.to_string())
        .is_none());
}

#[tokio::test]
async fn device_joined_publishes_event_and_enables_rename_last() {
    let _guard = serial();
    let harness = harness();
    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();
    harness.mqtt.lock().await.attempted.clear();

    harness.event_bus.lock().unwrap().emit_device_joined(json!({
        "device": {"ieeeAddr": IEEE, "name": "lamp"},
    }));
    tokio::time::sleep(Duration::from_millis(250)).await;

    {
        let mqtt = harness.mqtt.lock().await;
        let event = mqtt
            .attempted
            .iter()
            .find(|(topic, _, _)| topic == "zigbee2mqtt/bridge/event")
            .expect("device_joined event");
        let payload: Value = serde_json::from_str(&event.1).unwrap();
        assert_eq!(payload["type"], "device_joined");
        assert_eq!(payload["data"]["ieee_address"], IEEE);
        assert!(mqtt
            .attempted
            .iter()
            .any(|(topic, _, _)| topic == "zigbee2mqtt/bridge/devices"));
    }

    // Upstream `Should allow to rename last joined device`.
    send(&harness, "device/rename", json!({"last": true, "to": "lamp2"})).await;
    let payload = response(&harness, "device/rename").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    assert_eq!(payload["data"]["to"], "lamp2");
    assert!(harness
        .context
        .settings
        .lock()
        .unwrap()
        .get_device("lamp2")
        .is_some());
}

#[tokio::test]
async fn zigbee_events_publish_bridge_events_and_devices() {
    let _guard = serial();
    let harness = harness();
    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    let cases: Vec<(&str, Value, Value)> = vec![
        (
            "announce",
            json!({"device": {"ieeeAddr": IEEE, "name": "lamp"}}),
            json!({"type": "device_announce", "data": {"ieee_address": IEEE}}),
        ),
        (
            "interview",
            json!({"device": {"ieeeAddr": IEEE, "name": "lamp"}, "status": "started"}),
            json!({"type": "device_interview", "data": {"ieee_address": IEEE, "status": "started"}}),
        ),
        (
            "interview",
            json!({"device": {"ieeeAddr": IEEE, "name": "lamp"}, "status": "failed"}),
            json!({"type": "device_interview", "data": {"ieee_address": IEEE, "status": "failed"}}),
        ),
        (
            "interview",
            json!({"device": {"ieeeAddr": IEEE, "name": "lamp"}, "status": "successful"}),
            json!({"type": "device_interview", "data": {"ieee_address": IEEE, "status": "successful"}}),
        ),
        (
            "leave",
            json!({"device": {"ieeeAddr": IEEE, "name": "lamp"}}),
            json!({"type": "device_leave", "data": {"ieee_address": IEEE}}),
        ),
    ];

    for (event, data, expected) in cases {
        harness.mqtt.lock().await.attempted.clear();
        let mut bus = harness.event_bus.lock().unwrap();
        match event {
            "announce" => bus.emit_device_announce(data),
            "interview" => bus.emit_device_interview(data),
            _ => bus.emit_device_leave(data),
        }
        drop(bus);
        tokio::time::sleep(Duration::from_millis(200)).await;

        let mqtt = harness.mqtt.lock().await;
        let published = mqtt
            .attempted
            .iter()
            .find(|(topic, _, _)| topic == "zigbee2mqtt/bridge/event")
            .unwrap_or_else(|| panic!("missing bridge/event for {event}"));
        let payload: Value = serde_json::from_str(&published.1).unwrap();
        assert_eq!(payload["type"], expected["type"]);
        assert_eq!(payload["data"]["ieee_address"], expected["data"]["ieee_address"]);
        if let Some(status) = expected["data"]["status"].as_str() {
            assert_eq!(payload["data"]["status"], status);
            if status == "successful" {
                // The harness device has no registered definition, so the event
                // reports it as unsupported with a null definition.
                assert_eq!(payload["data"]["supported"], false);
                assert!(payload["data"]["definition"].is_null());
            }
        }
        assert!(
            mqtt
                .attempted
                .iter()
                .any(|(topic, _, _)| topic == "zigbee2mqtt/bridge/devices"),
            "devices not republished for {event}"
        );
    }
}

#[tokio::test]
async fn options_blocklist_and_device_option_variants() {
    let _guard = serial();
    let harness = harness();
    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    // Upstream `Should allow to add and remove from blocklist`.
    send(
        &harness,
        "options",
        json!({"options": {"blocklist": ["0x123", "0x1234"]}}),
    )
    .await;
    let payload = response(&harness, "options").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    assert_eq!(
        harness.context.settings.lock().unwrap().get()["blocklist"],
        json!(["0x123", "0x1234"])
    );
    send(
        &harness,
        "options",
        json!({"options": {"blocklist": ["0x123"]}}),
    )
    .await;
    let _ = response(&harness, "options").await;
    assert_eq!(
        harness.context.settings.lock().unwrap().get()["blocklist"],
        json!(["0x123"])
    );

    // Upstream `Should allow to remove device option`.
    harness
        .context
        .settings
        .lock()
        .unwrap()
        .set(&["devices", IEEE, "qos"], json!(1));
    send(
        &harness,
        "device/options",
        json!({"id": "lamp", "options": {"qos": null}}),
    )
    .await;
    let payload = response(&harness, "device/options").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    assert_eq!(payload["data"]["to"].get("qos"), None);
    assert_eq!(
        harness
            .context
            .settings
            .lock()
            .unwrap()
            .get_device("lamp")
            .unwrap()
            .get("qos"),
        None
    );

    // Upstream `Should allow change device options with restart required`.
    send(
        &harness,
        "device/options",
        json!({"id": "lamp", "options": {"disabled": true}}),
    )
    .await;
    let payload = response(&harness, "device/options").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    assert_eq!(payload["data"]["restart_required"], true);
    assert_eq!(
        harness
            .context
            .settings
            .lock()
            .unwrap()
            .get_device("lamp")
            .unwrap()["disabled"],
        true
    );

    // Upstream `Should save as image as file when changing device icon`.
    let icon = "data:image/png;base64,aGVsbG8=";
    send(
        &harness,
        "device/options",
        json!({"id": "lamp", "options": {"icon": icon}}),
    )
    .await;
    let payload = response(&harness, "device/options").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    let file = payload["data"]["to"]["icon"]
        .as_str()
        .expect("icon path")
        .to_string();
    assert!(file.starts_with("device_icons/"), "{file}");
    let path = z2m_rs::util::data::join_path(&file);
    assert!(path.exists(), "{}", path.display());
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(path.parent().unwrap());
}

#[tokio::test]
async fn restart_request_triggers_the_restart_callback() {
    let _guard = serial();
    let harness = harness();
    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    send(&harness, "restart", json!("")).await;
    let payload = response(&harness, "restart").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    assert!(harness
        .context
        .logger
        .lock()
        .unwrap()
        .winston_logs
        .iter()
        .any(|(_, line)| line.contains("Restarting Zigbee2MQTT")));
    // The callback fires after the upstream 500 ms response delay.
    tokio::time::sleep(Duration::from_millis(650)).await;
    assert_eq!(*harness.restarts.lock().unwrap(), 1);
}

#[tokio::test]
async fn backup_without_adapter_returns_an_error() {
    let _guard = serial();
    let harness = harness();
    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    send(&harness, "backup", json!("")).await;
    let payload = response(&harness, "backup").await;
    assert_eq!(payload["status"], "error", "{payload}");
    assert!(!payload["error"].as_str().unwrap_or_default().is_empty());
    assert!(payload["data"]["zip"].is_null());
}

#[tokio::test]
async fn remove_device_can_clear_the_entity_cache() {
    let _guard = serial();
    let harness = harness();
    let bridge = Bridge::new(harness.context.clone());
    bridge.clone().start().await.unwrap();

    // Upstream `Should allow to clear cache when removing device`.
    send(
        &harness,
        "device/remove",
        json!({"id": "lamp", "force": true, "clear_cache": true}),
    )
    .await;
    let payload = response(&harness, "device/remove").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    assert_eq!(payload["data"]["force"], true);
    assert!(harness.context.zigbee.resolve_device(IEEE).is_none());
    assert!(harness
        .context
        .settings
        .lock()
        .unwrap()
        .get_device("lamp")
        .is_none());
}
