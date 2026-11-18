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

#[path = "support/mock_adapter.rs"]
mod mock_adapter;
use mock_adapter::MockAdapter;

const IEEE: &str = "0x0017880104e45519";
const IEEE2: &str = "0x0017880104e45520";

struct Harness {
    adapter: Arc<MockAdapter>,
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
    let coordinator = ZhDevice::create(
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
    coordinator.create_endpoint(1).unwrap();
    for (ieee, network, model, manufacturer) in [
        (IEEE, 1234u16, "TEST", "Test"),
        (IEEE2, 1235, "BULB", "GLEDOPTO"),
    ] {
        let device = ZhDevice::create(
            DeviceType::Router,
            ieee,
            network,
            None,
            Some(manufacturer.to_string()),
            None,
            Some(model.to_string()),
            InterviewState::Successful,
            None,
        )
        .unwrap();
        let endpoint = device.create_endpoint(1).unwrap();
        endpoint.input_clusters.lock().unwrap().extend([6u16, 8]);
        endpoint.output_clusters.lock().unwrap().extend([6u16, 8]);
    }

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
        "devices": {
            IEEE: {"friendly_name": "lamp"},
            IEEE2: {"friendly_name": "bulb"},
        },
        "groups": {"1": {"friendly_name": "kitchen"}},
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
    let _ = zigbee.resolve_device(IEEE2);
    let group = zigbee.zh_controller().unwrap().create_group(1).unwrap();
    if let Some(bulb) = zigbee.resolve_device(IEEE2) {
        group.add_member(&bulb.zh.endpoints()[0]);
    }
    let _ = zigbee.resolve_group(1);
    let state_path = temp("state.json");
    let state = Arc::new(Mutex::new(State::with_file(
        settings.clone(),
        event_bus.clone(),
        logger.clone(),
        state_path.clone(),
    )));
    let mqtt = Arc::new(tokio::sync::Mutex::new(Mqtt::new(event_bus.clone())));
    let publish: PublishEntityState = Arc::new(|_e, _p, _r| Box::pin(async {}));

    let adapter = MockAdapter::new();
    zigbee_apiarist::controller::model::entity::Entity::inject_adapter(adapter.clone());
    *adapter.zdo_response.lock().unwrap() = json!([0]);

    Harness {
        adapter,
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

    send(
        &harness,
        "bind",
        json!({"from": IEEE, "to": IEEE, "clusters": ["genBasic"]}),
    )
    .await;
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

/// Cases 21-24: bind rejects missing source/target entities and endpoints.
#[tokio::test]
async fn bind_reports_missing_source_and_target() {
    let _guard = serial();
    let harness = harness();
    let bind = Bind::new(harness.context.clone());
    bind.clone().start().await.unwrap();

    send(&harness, "bind", json!({"from": "missing", "to": "lamp"})).await;
    let payload = response(&harness, "bind").await;
    assert_eq!(
        payload["error"], "Source device 'missing' does not exist",
        "{payload}"
    );

    harness.mqtt.lock().await.attempted.clear();
    send(
        &harness,
        "bind",
        json!({"from": "lamp", "from_endpoint": "9", "to": "lamp"}),
    )
    .await;
    let payload = response(&harness, "bind").await;
    assert_eq!(
        payload["error"], "Source device 'lamp' does not have endpoint '9'",
        "{payload}"
    );

    harness.mqtt.lock().await.attempted.clear();
    send(&harness, "bind", json!({"from": "lamp", "to": "missing"})).await;
    let payload = response(&harness, "bind").await;
    assert_eq!(
        payload["error"], "Target device or group 'missing' does not exist",
        "{payload}"
    );

    harness.mqtt.lock().await.attempted.clear();
    send(
        &harness,
        "bind",
        json!({"from": "lamp", "to": "lamp", "to_endpoint": "9"}),
    )
    .await;
    let payload = response(&harness, "bind").await;
    assert_eq!(
        payload["error"], "Target device 'lamp' does not have endpoint '9'",
        "{payload}"
    );
    bind.stop().await.unwrap();
}

fn bind_calls(harness: &Harness, cluster_id: u16) -> usize {
    harness
        .adapter
        .zdo_calls
        .lock()
        .unwrap()
        .iter()
        .filter(|(_, _, cluster, _, _)| *cluster == cluster_id)
        .count()
}

const BIND_REQUEST: u16 = 0x0021;
const UNBIND_REQUEST: u16 = 0x0022;

/// Case 1: bind to a device and configure reporting.
#[tokio::test]
async fn binds_to_device_and_configures_reporting() {
    let _guard = serial();
    let harness = harness();
    let bind = Bind::new(harness.context.clone());
    bind.clone().start().await.unwrap();

    send(
        &harness,
        "bind",
        json!({"from": "bulb", "to": "lamp", "clusters": ["genOnOff"]}),
    )
    .await;
    let payload = response(&harness, "bind").await;

    assert_eq!(payload["status"], "ok", "{payload}");
    assert_eq!(payload["data"]["clusters"], json!(["genOnOff"]));
    // one user bind plus the reporting bind to the coordinator
    assert_eq!(bind_calls(&harness, BIND_REQUEST), 2);
    assert!(
        harness
            .adapter
            .endpoint_frames
            .lock()
            .unwrap()
            .iter()
            .any(|frame| frame.command.name.eq_ignore_ascii_case("configReport")),
        "no reporting configuration frame"
    );
    bind.stop().await.unwrap();
}

/// Case 5: only the requested clusters are bound.
#[tokio::test]
async fn binds_only_the_requested_clusters() {
    let _guard = serial();
    let harness = harness();
    let bind = Bind::new(harness.context.clone());
    bind.clone().start().await.unwrap();

    send(
        &harness,
        "bind",
        json!({"from": "bulb", "to": "lamp", "clusters": ["genOnOff", "genLevelCtrl"]}),
    )
    .await;
    let payload = response(&harness, "bind").await;

    assert_eq!(payload["status"], "ok", "{payload}");
    // two user binds plus two reporting binds
    assert_eq!(bind_calls(&harness, BIND_REQUEST), 4);
    bind.stop().await.unwrap();
}

/// Case 6: binding to the coordinator by IEEE address.
#[tokio::test]
async fn binds_to_the_coordinator() {
    let _guard = serial();
    let harness = harness();
    let bind = Bind::new(harness.context.clone());
    bind.clone().start().await.unwrap();

    send(
        &harness,
        "bind",
        json!({"from": "bulb", "to": "0x00124b0024c16b95", "clusters": ["genOnOff"]}),
    )
    .await;
    let payload = response(&harness, "bind").await;

    assert_eq!(payload["status"], "ok", "{payload}");
    assert_eq!(bind_calls(&harness, BIND_REQUEST), 1);
    bind.stop().await.unwrap();
}

/// Case 8: unbinding an existing binding.
#[tokio::test]
async fn unbinds_a_device_binding() {
    let _guard = serial();
    let harness = harness();
    let bind = Bind::new(harness.context.clone());
    bind.clone().start().await.unwrap();

    send(
        &harness,
        "bind",
        json!({"from": "bulb", "to": "lamp", "clusters": ["genOnOff"]}),
    )
    .await;
    assert_eq!(response(&harness, "bind").await["status"], "ok");

    send(
        &harness,
        "unbind",
        json!({"from": "bulb", "to": "lamp", "clusters": ["genOnOff"]}),
    )
    .await;
    let payload = response(&harness, "unbind").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    // the user unbind plus the reporting cleanup
    assert_eq!(bind_calls(&harness, UNBIND_REQUEST), 2);
    bind.stop().await.unwrap();
}

/// Case 9: unbinding the coordinator.
#[tokio::test]
async fn unbinds_the_coordinator() {
    let _guard = serial();
    let harness = harness();
    let bind = Bind::new(harness.context.clone());
    bind.clone().start().await.unwrap();

    send(
        &harness,
        "bind",
        json!({"from": "bulb", "to": "0x00124b0024c16b95", "clusters": ["genOnOff"]}),
    )
    .await;
    assert_eq!(response(&harness, "bind").await["status"], "ok");
    send(
        &harness,
        "unbind",
        json!({"from": "bulb", "to": "0x00124b0024c16b95", "clusters": ["genOnOff"]}),
    )
    .await;
    let payload = response(&harness, "unbind").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    assert!(bind_calls(&harness, UNBIND_REQUEST) >= 1);
    bind.stop().await.unwrap();
}

/// Cases 10/14: binding to a group by name or number uses a multicast bind.
#[tokio::test]
async fn binds_to_a_group_by_name_and_number() {
    for target in [json!("kitchen"), json!(1)] {
        let _guard = serial();
        let harness = harness();
        let bind = Bind::new(harness.context.clone());
        bind.clone().start().await.unwrap();

        send(
            &harness,
            "bind",
            json!({"from": "bulb", "to": target, "clusters": ["genOnOff"]}),
        )
        .await;
        let payload = response(&harness, "bind").await;
        assert_eq!(payload["status"], "ok", "{target}: {payload}");
        // user bind plus reporting bind for the group member
        assert_eq!(bind_calls(&harness, BIND_REQUEST), 2, "{target}");
        bind.stop().await.unwrap();
    }
}

/// Cases 11-13: unbinding from a group, with and without disabling reporting.
#[tokio::test]
async fn unbinds_from_a_group() {
    for skip in [false, true] {
        let _guard = serial();
        let harness = harness();
        let bind = Bind::new(harness.context.clone());
        bind.clone().start().await.unwrap();
        send(
            &harness,
            "bind",
            json!({"from": "bulb", "to": "0x00124b0024c16b95", "clusters": ["genOnOff"]}),
        )
        .await;
        assert_eq!(response(&harness, "bind").await["status"], "ok");
        send(
            &harness,
            "bind",
            json!({"from": "bulb", "to": "kitchen", "clusters": ["genOnOff"]}),
        )
        .await;
        assert_eq!(response(&harness, "bind").await["status"], "ok");
        // let any asynchronous reporting setup settle first
        tokio::time::sleep(Duration::from_millis(300)).await;
        harness.adapter.endpoint_frames.lock().unwrap().clear();

        send(
            &harness,
            "unbind",
            json!({
                "from": "bulb",
                "to": "kitchen",
                "clusters": ["genOnOff"],
                "skip_disable_reporting": skip,
            }),
        )
        .await;
        let payload = response(&harness, "unbind").await;
        assert_eq!(payload["status"], "ok", "{payload}");
        assert!(bind_calls(&harness, UNBIND_REQUEST) >= 1);
        tokio::time::sleep(Duration::from_millis(200)).await;
        let frames = harness.adapter.endpoint_frames.lock().unwrap().clone();
        let reporting_frames = frames
            .iter()
            .filter(|frame| frame.command.name.eq_ignore_ascii_case("configReport"))
            .count();
        assert_eq!(
            reporting_frames == 0,
            skip,
            "skip_disable_reporting={skip}: {reporting_frames} reporting frames, all={:?}",
            frames
                .iter()
                .map(|frame| frame.command.name.clone())
                .collect::<Vec<_>>()
        );
        bind.stop().await.unwrap();
    }
}

/// Case 15: a failing bind is reported and logged.
#[tokio::test]
async fn bind_failure_is_reported() {
    let _guard = serial();
    let harness = harness();
    harness
        .adapter
        .fail_zdo
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let bind = Bind::new(harness.context.clone());
    bind.clone().start().await.unwrap();

    send(
        &harness,
        "bind",
        json!({"from": "bulb", "to": "lamp", "clusters": ["genOnOff"]}),
    )
    .await;
    let payload = response(&harness, "bind").await;

    assert_eq!(payload["status"], "error", "{payload}");
    assert_eq!(payload["error"], "Failed to bind");
    bind.stop().await.unwrap();
}

/// Cases 16/17/19: source endpoints by id and the default endpoint.
#[tokio::test]
async fn binds_from_named_and_default_endpoints() {
    let _guard = serial();
    let harness = harness();
    {
        let bulb = harness.context.zigbee.resolve_device(IEEE2).unwrap();
        let endpoint = bulb.zh.create_endpoint(2).unwrap();
        endpoint.input_clusters.lock().unwrap().extend([6u16, 8]);
        endpoint.output_clusters.lock().unwrap().extend([6u16, 8]);
    }
    let bind = Bind::new(harness.context.clone());
    bind.clone().start().await.unwrap();

    // Numeric endpoint id.
    send(
        &harness,
        "bind",
        json!({"from": "bulb", "from_endpoint": "2", "to": "lamp", "clusters": ["genOnOff"]}),
    )
    .await;
    assert_eq!(response(&harness, "bind").await["status"], "ok");

    // Default endpoint when omitted.
    send(
        &harness,
        "bind",
        json!({"from": "bulb", "to": "lamp", "clusters": ["genOnOff"]}),
    )
    .await;
    let payload = response(&harness, "bind").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    assert_eq!(bind_calls(&harness, BIND_REQUEST), 2);
    bind.stop().await.unwrap();
}

/// Case 20: unbinding the default bind group.
#[tokio::test]
async fn unbinds_the_default_bind_group() {
    let _guard = serial();
    let harness = harness();
    let bind = Bind::new(harness.context.clone());
    bind.clone().start().await.unwrap();

    send(
        &harness,
        "bind",
        json!({"from": "bulb", "to": "default_bind_group", "clusters": ["genOnOff"]}),
    )
    .await;
    assert_eq!(response(&harness, "bind").await["status"], "ok");

    send(
        &harness,
        "unbind",
        json!({"from": "bulb", "to": "default_bind_group", "clusters": ["genOnOff"]}),
    )
    .await;
    let payload = response(&harness, "unbind").await;
    assert_eq!(payload["status"], "ok", "{payload}");
    assert_eq!(bind_calls(&harness, UNBIND_REQUEST), 1);
    bind.stop().await.unwrap();
}

fn read_frames(harness: &Harness) -> usize {
    harness
        .adapter
        .endpoint_frames
        .lock()
        .unwrap()
        .iter()
        .filter(|frame| frame.command.name.eq_ignore_ascii_case("read"))
        .count()
}

/// Case 4: reporting setup is not repeated when the binding already exists.
#[tokio::test]
async fn reporting_setup_is_not_repeated() {
    let _guard = serial();
    let harness = harness();
    let bind = Bind::new(harness.context.clone());
    bind.clone().start().await.unwrap();

    for _ in 0..2 {
        send(
            &harness,
            "bind",
            json!({"from": "bulb", "to": "lamp", "clusters": ["genOnOff"]}),
        )
        .await;
        assert_eq!(response(&harness, "bind").await["status"], "ok");
    }

    assert!(
        bind_calls(&harness, BIND_REQUEST) >= 2,
        "user + reporting binds"
    );
    let reporting = harness
        .adapter
        .endpoint_frames
        .lock()
        .unwrap()
        .iter()
        .filter(|frame| frame.command.name.eq_ignore_ascii_case("configReport"))
        .count();
    assert_eq!(reporting, 1, "reporting configured more than once");
    bind.stop().await.unwrap();
}

fn emit_command(harness: &Harness, cluster: &str, r#type: &str, data: Value, group: Option<u64>) {
    let mut payload = json!({
        "type": r#type,
        "device": {"ID": IEEE, "ieeeAddr": IEEE, "name": "lamp"},
        "endpoint": {"ID": 1},
        "cluster": cluster,
        "data": data,
        "linkquality": 100,
        "meta": {"zclTransactionSequenceNumber": 1},
    });
    if let Some(group) = group {
        payload["groupID"] = json!(group);
    }
    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_device_message(payload);
}

/// Cases 25-27: Hue commands poll the bound bulb / group.
#[tokio::test]
async fn hue_commands_poll_bound_devices() {
    let _guard = serial();
    let harness = harness();
    let bind = Bind::new(harness.context.clone());
    bind.clone().start().await.unwrap();
    send(
        &harness,
        "bind",
        json!({"from": "lamp", "to": "bulb", "clusters": ["genOnOff", "genLevelCtrl"]}),
    )
    .await;
    assert_eq!(response(&harness, "bind").await["status"], "ok");
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Case 25: Hue button 2 reads the current level.
    harness.adapter.endpoint_frames.lock().unwrap().clear();
    emit_command(
        &harness,
        "manuSpecificPhilips",
        "commandHueNotification",
        json!({"button": 2}),
        None,
    );
    tokio::time::sleep(Duration::from_millis(1_200)).await;
    assert!(read_frames(&harness) >= 1, "button 2 did not poll the bulb");

    // Case 26: Hue button 1 reads the on/off state.
    harness.adapter.endpoint_frames.lock().unwrap().clear();
    emit_command(
        &harness,
        "manuSpecificPhilips",
        "commandHueNotification",
        json!({"button": 1}),
        None,
    );
    tokio::time::sleep(Duration::from_millis(1_200)).await;
    assert!(read_frames(&harness) >= 1, "button 1 did not poll the bulb");

    // Case 27: a group scene recall polls the group members.
    harness
        .context
        .zigbee
        .resolve_group(1)
        .unwrap()
        .zh
        .add_member(
            &harness
                .context
                .zigbee
                .resolve_device(IEEE2)
                .unwrap()
                .zh
                .endpoints()[0],
        );
    harness.adapter.endpoint_frames.lock().unwrap().clear();
    emit_command(
        &harness,
        "genScenes",
        "commandRecall",
        json!({"groupid": 1, "sceneid": 1}),
        Some(1),
    );
    tokio::time::sleep(Duration::from_millis(1_200)).await;
    assert!(
        read_frames(&harness) >= 1,
        "scene recall did not poll the group"
    );
    bind.stop().await.unwrap();
}
