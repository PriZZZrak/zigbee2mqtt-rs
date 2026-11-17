// Original: .upstream/zigbee2mqtt/test/extensions/groups.test.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::sync::{Arc, Mutex};

#[path = "support/mock_adapter.rs"]
mod mock_adapter;
use mock_adapter::MockAdapter;
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
const IEEE2: &str = "0x0017880104e45520";

type Published = Arc<Mutex<Vec<(String, Value, Option<String>)>>>;

struct Harness {
    adapter: Arc<MockAdapter>,
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
    let adapter = MockAdapter::new();
    zigbee_apiarist::controller::model::entity::Entity::inject_adapter(adapter.clone());
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
    let bulb = ZhDevice::create(
        DeviceType::Router,
        IEEE2,
        5678,
        None,
        Some("Bulb".to_string()),
        Some("Mains (single phase)".to_string()),
        Some("BULB".to_string()),
        InterviewState::Successful,
        None,
    )
    .unwrap();
    let bulb_endpoint = bulb.create_endpoint(1).unwrap();
    let group = controller.create_group(1).unwrap();
    group.add_member(&device.endpoints()[0]);
    group.add_member(&bulb_endpoint);

    let settings = Arc::new(Mutex::new(Settings::from_config(json!({
        "mqtt": {"server": "localhost"},
        "devices": {
            IEEE: {"friendly_name": "lamp"},
            IEEE2: {"friendly_name": "bulb"},
        },
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
    let bulb = zigbee.resolve_device(IEEE2).unwrap();
    let mut definition =
        zigbee_converters::Definition::new("BULB", "Acme", "Bulb").with_zigbee_model("BULB");
    definition.exposes = zigbee_converters::Exposes::Static(vec![zigbee_converters::Expose {
        r#type: "light".to_string(),
        name: Some("light".to_string()),
        access: zigbee_converters::access::ALL,
        features: vec![
            zigbee_converters::Expose::binary("state", zigbee_converters::access::ALL),
            zigbee_converters::Expose::numeric("brightness", zigbee_converters::access::ALL),
            zigbee_converters::Expose::numeric("color_temp", zigbee_converters::access::ALL),
        ],
        ..Default::default()
    }]);
    zigbee.converters().lock().unwrap().register(definition);
    let _ = bulb.resolve_definition(false);
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
        adapter,
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

#[tokio::test]
async fn member_state_filtering_uses_member_exposes() {
    let _guard = serial();
    let harness = harness();
    let groups = Groups::new(harness.context.clone());
    groups.clone().start().await.unwrap();

    // The group payload carries color_temp; the definition-less lamp must not
    // receive it, the bulb (which exposes color_temp) must.
    harness.event_bus.lock().unwrap().emit_state_change(json!({
        "entity": {"ID": 1, "name": "kitchen"},
        "from": {},
        "to": {"state": "ON", "color_temp": 50},
        "update": {"state": "ON", "color_temp": 50},
        "reason": "devicePublish",
    }));
    tokio::time::sleep(Duration::from_millis(150)).await;

    let published = harness.published.lock().unwrap();
    let bulb = published
        .iter()
        .find(|(name, _, _)| name == "bulb")
        .expect("bulb member state");
    assert_eq!(bulb.1, json!({"state": "ON", "color_temp": 50}));
    let lamp = published
        .iter()
        .find(|(name, _, _)| name == "lamp")
        .expect("lamp member state");
    assert_eq!(lamp.1, json!({"state": "ON"}));
}

const SWITCH: &str = "0x0017880104e45542";

fn group_members(harness: &Harness, id: u16) -> Vec<(String, u8)> {
    harness
        .zigbee
        .resolve_group(id as u64)
        .unwrap()
        .zh
        .members()
        .iter()
        .map(|member| {
            (
                member.device_ieee_address.lock().unwrap().clone(),
                member.ID,
            )
        })
        .collect()
}

/// Create a group that is not part of the default harness setup.
fn create_named_group(harness: &Harness, id: u16, name: &str) -> Arc<ZhGroup> {
    harness.context.settings.lock().unwrap().configuration["groups"][id.to_string()] =
        json!({"friendly_name": name});
    let group = harness
        .zigbee
        .zh_controller()
        .unwrap()
        .create_group(id)
        .unwrap();
    let _ = harness.zigbee.resolve_group(id as u64);
    group
}

/// Multi-endpoint device with a named `right` endpoint (upstream QBKG03LM).
fn add_multi_endpoint_device(
    harness: &Harness,
) -> Arc<zigbee_apiarist::controller::model::endpoint::Endpoint> {
    harness.context.settings.lock().unwrap().configuration["devices"][SWITCH] =
        json!({"friendly_name": "switch"});
    let device = ZhDevice::create(
        DeviceType::Router,
        SWITCH,
        1235,
        None,
        Some("Test".to_string()),
        Some("Mains (single phase)".to_string()),
        Some("SWITCH".to_string()),
        InterviewState::Successful,
        None,
    )
    .unwrap();
    device.create_endpoint(1).unwrap();
    device.create_endpoint(2).unwrap();
    let endpoint = device.create_endpoint(3).unwrap();
    let mut definition =
        zigbee_converters::Definition::new("SWITCH", "Acme", "Switch").with_zigbee_model("SWITCH");
    definition.endpoint = vec![("right".to_string(), 3)];
    harness
        .zigbee
        .converters()
        .lock()
        .unwrap()
        .register(definition);
    let resolved = harness.zigbee.resolve_device(SWITCH).unwrap();
    let _ = resolved.resolve_definition(false);
    endpoint
}

fn members_changed_events(harness: &Harness) -> Arc<Mutex<Vec<Value>>> {
    let events: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    harness
        .event_bus
        .lock()
        .unwrap()
        .on_group_members_changed(900, Arc::new(move |data| sink.lock().unwrap().push(data)));
    events
}

async fn members_request(harness: &Harness, r#type: &str, payload: Value) -> Value {
    let groups = Groups::new(harness.context.clone());
    groups.clone().start().await.unwrap();
    let response_topic = format!("zigbee2mqtt/bridge/response/group/members/{type}");
    let seen = {
        let mqtt = harness.mqtt.lock().await;
        mqtt.attempted
            .iter()
            .filter(|(published, _, _)| published == &response_topic)
            .count()
    };
    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": format!("zigbee2mqtt/bridge/request/group/members/{type}"),
        "message": payload.to_string(),
    }));
    for _ in 0..150 {
        let body = {
            let mqtt = harness.mqtt.lock().await;
            let mut matches = mqtt
                .attempted
                .iter()
                .filter(|(published, _, _)| published == &response_topic);
            if mqtt
                .attempted
                .iter()
                .filter(|(published, _, _)| published == &response_topic)
                .count()
                > seen
            {
                matches.next_back().map(|(_, body, _)| body.clone())
            } else {
                None
            }
        };
        if let Some(body) = body {
            groups.stop().await.unwrap();
            return serde_json::from_str(&body).unwrap_or(Value::Null);
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    groups.stop().await.unwrap();
    panic!("no response for {type}");
}

async fn wait_for_events(events: &Arc<Mutex<Vec<Value>>>, expected: usize) {
    for _ in 0..50 {
        if events.lock().unwrap().len() >= expected {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn count_topic(harness: &Harness, topic: &str) -> usize {
    harness
        .mqtt
        .lock()
        .await
        .attempted
        .iter()
        .filter(|(published, _, _)| published == topic)
        .count()
}

/// Case 23: add a device endpoint to a group via MQTT, update the members and
/// emit the group-members-changed event.
#[tokio::test]
async fn add_to_group_via_mqtt() {
    let _guard = serial();
    let harness = harness();
    let group = create_named_group(&harness, 2, "office");
    assert!(group.members().is_empty());
    let events = members_changed_events(&harness);

    let response = members_request(
        &harness,
        "add",
        json!({"transaction": "123", "group": "office", "device": "lamp"}),
    )
    .await;

    assert_eq!(response["status"], "ok", "{response}");
    assert_eq!(response["transaction"], "123");
    assert_eq!(
        response["data"],
        json!({"device": "lamp", "endpoint": "default", "group": "office"})
    );
    assert_eq!(group_members(&harness, 2), vec![(IEEE.to_string(), 1)]);
    wait_for_events(&events, 1).await;
    let events = events.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["action"], "add");
    assert_eq!(events[0]["group"]["name"], "office");
    assert_eq!(events[0]["endpoint"]["deviceIeeeAddress"], IEEE);
}

/// Case 24: a coordinator failure is reported and the group is not modified.
#[tokio::test]
async fn add_to_group_via_mqtt_fails() {
    let _guard = serial();
    let harness = harness();
    let group = create_named_group(&harness, 2, "office");
    harness
        .adapter
        .fail_zcl
        .store(true, std::sync::atomic::Ordering::SeqCst);

    let response = members_request(
        &harness,
        "add",
        json!({"group": "office", "device": "lamp"}),
    )
    .await;

    assert_eq!(response["status"], "error", "{response}");
    let error = response["error"].as_str().unwrap_or_default();
    assert!(
        error.starts_with("Failed to add to group (") && error.contains("ZCL failed"),
        "{response}"
    );
    assert!(group.members().is_empty(), "group must stay unchanged");
}

/// Case 25: group friendly names may contain slashes.
#[tokio::test]
async fn add_to_group_with_slashes_via_mqtt() {
    let _guard = serial();
    let harness = harness();
    let group = create_named_group(&harness, 99, "group/with/slashes");

    let response = members_request(
        &harness,
        "add",
        json!({"group": "group/with/slashes", "device": "lamp"}),
    )
    .await;

    assert_eq!(response["status"], "ok", "{response}");
    assert_eq!(
        response["data"],
        json!({"device": "lamp", "endpoint": "default", "group": "group/with/slashes"})
    );
    assert_eq!(group_members(&harness, 99), vec![(IEEE.to_string(), 1)]);
}

/// Case 26: add a named endpoint (postfix) of a multi-endpoint device.
#[tokio::test]
async fn add_to_group_via_mqtt_with_endpoint_name() {
    let _guard = serial();
    let harness = harness();
    let group = create_named_group(&harness, 2, "office");
    add_multi_endpoint_device(&harness);

    let response = members_request(
        &harness,
        "add",
        json!({"group": "office", "device": "switch", "endpoint": "right"}),
    )
    .await;

    assert_eq!(response["status"], "ok", "{response}");
    assert_eq!(
        response["data"],
        json!({"device": "switch", "endpoint": "right", "group": "office"})
    );
    assert_eq!(group_members(&harness, 2), vec![(SWITCH.to_string(), 3)]);
}

/// Case 27: adding the same endpoint again (IEEE + ID after name) is a no-op.
#[tokio::test]
async fn add_to_group_via_mqtt_does_not_add_twice() {
    let _guard = serial();
    let harness = harness();
    let group = create_named_group(&harness, 2, "office");
    add_multi_endpoint_device(&harness);

    let first = members_request(
        &harness,
        "add",
        json!({"group": "office", "device": "switch", "endpoint": "right"}),
    )
    .await;
    assert_eq!(first["status"], "ok", "{first}");

    let second = members_request(
        &harness,
        "add",
        json!({"group": "office", "device": SWITCH, "endpoint": "3"}),
    )
    .await;
    assert_eq!(second["status"], "ok", "{second}");
    assert_eq!(
        second["data"],
        json!({"device": SWITCH, "endpoint": "3", "group": "office"})
    );
    assert_eq!(group_members(&harness, 2), vec![(SWITCH.to_string(), 3)]);
}

/// Case 28: remove a device endpoint from a group via MQTT.
#[tokio::test]
async fn remove_from_group_via_mqtt() {
    let _guard = serial();
    let harness = harness();
    let events = members_changed_events(&harness);

    let response = members_request(
        &harness,
        "remove",
        json!({"group": "kitchen", "device": "lamp"}),
    )
    .await;

    assert_eq!(response["status"], "ok", "{response}");
    assert_eq!(
        response["data"],
        json!({"device": "lamp", "endpoint": "default", "group": "kitchen"})
    );
    let members = group_members(&harness, 1);
    assert!(!members.contains(&(IEEE.to_string(), 1)), "{members:?}");
    assert!(members.contains(&(IEEE2.to_string(), 1)), "{members:?}");
    wait_for_events(&events, 1).await;
    let events = events.lock().unwrap();
    assert_eq!(events[0]["action"], "remove");
    assert_eq!(events[0]["group"]["name"], "kitchen");
}

/// Case 29: the coordinator error is reported and the member is kept.
#[tokio::test]
async fn remove_from_group_via_mqtt_fails() {
    let _guard = serial();
    let harness = harness();
    harness
        .adapter
        .fail_zcl
        .store(true, std::sync::atomic::Ordering::SeqCst);

    let response = members_request(
        &harness,
        "remove",
        json!({"group": "kitchen", "device": "lamp"}),
    )
    .await;

    assert_eq!(response["status"], "error", "{response}");
    let error = response["error"].as_str().unwrap_or_default();
    assert!(
        error.starts_with("Failed to remove from group (") && error.contains("ZCL failed"),
        "{response}"
    );
    assert!(group_members(&harness, 1).contains(&(IEEE.to_string(), 1)));
}

/// Case 30: `skip_disable_reporting` removes the member without sending a
/// `configureReporting` frame that would disable device reporting.
#[tokio::test]
async fn remove_from_group_keeping_reporting() {
    let _guard = serial();
    let harness = harness();
    harness.adapter.endpoint_frames.lock().unwrap().clear();

    let response = members_request(
        &harness,
        "remove",
        json!({"group": "kitchen", "device": "lamp", "skip_disable_reporting": true}),
    )
    .await;

    assert_eq!(response["status"], "ok", "{response}");
    assert!(!group_members(&harness, 1).contains(&(IEEE.to_string(), 1)));
    let frames = harness.adapter.endpoint_frames.lock().unwrap().clone();
    assert!(
        frames.iter().any(|frame| frame.command.name == "remove"),
        "no genGroups remove frame"
    );
    assert!(
        !frames
            .iter()
            .any(|frame| frame.command.name == "configureReporting"),
        "reporting must not be disabled with skip_disable_reporting"
    );
}

/// Cases 31-33: remove with the endpoint addressed by id, name and both.
#[tokio::test]
async fn remove_from_group_with_postfix_variants() {
    let _guard = serial();
    let harness = harness();
    let endpoint = add_multi_endpoint_device(&harness);
    let group = harness.zigbee.resolve_group(1).unwrap();
    group.zh.add_member(&endpoint);

    // Variant 1: IEEE address + endpoint id.
    let response = members_request(
        &harness,
        "remove",
        json!({"group": "kitchen", "device": SWITCH, "endpoint": "3"}),
    )
    .await;
    assert_eq!(response["status"], "ok", "{response}");
    assert_eq!(
        response["data"],
        json!({"device": SWITCH, "endpoint": "3", "group": "kitchen"})
    );
    assert!(!group_members(&harness, 1).contains(&(SWITCH.to_string(), 3)));

    // Variant 2: friendly name + endpoint id.
    group.zh.add_member(&endpoint);
    let response = members_request(
        &harness,
        "remove",
        json!({"group": "kitchen", "device": "switch", "endpoint": "3"}),
    )
    .await;
    assert_eq!(response["status"], "ok", "{response}");
    assert_eq!(
        response["data"],
        json!({"device": "switch", "endpoint": "3", "group": "kitchen"})
    );
    assert!(!group_members(&harness, 1).contains(&(SWITCH.to_string(), 3)));

    // Variant 3: IEEE address + endpoint name.
    group.zh.add_member(&endpoint);
    let response = members_request(
        &harness,
        "remove",
        json!({"group": "kitchen", "device": SWITCH, "endpoint": "right"}),
    )
    .await;
    assert_eq!(response["status"], "ok", "{response}");
    assert_eq!(
        response["data"],
        json!({"device": SWITCH, "endpoint": "right", "group": "kitchen"})
    );
    assert!(!group_members(&harness, 1).contains(&(SWITCH.to_string(), 3)));
}

/// Case 34: `remove_all` removes the endpoint from every group.
#[tokio::test]
async fn remove_from_group_all() {
    let _guard = serial();
    let harness = harness();
    let endpoint = add_multi_endpoint_device(&harness);
    let group = harness.zigbee.resolve_group(1).unwrap();
    group.zh.add_member(&endpoint);
    harness.adapter.endpoint_frames.lock().unwrap().clear();

    let response = members_request(
        &harness,
        "remove_all",
        json!({"device": SWITCH, "endpoint": "right"}),
    )
    .await;

    assert_eq!(response["status"], "ok", "{response}");
    assert_eq!(
        response["data"],
        json!({"device": SWITCH, "endpoint": "right"})
    );
    assert!(!group_members(&harness, 1).contains(&(SWITCH.to_string(), 3)));
    assert!(
        harness
            .adapter
            .endpoint_frames
            .lock()
            .unwrap()
            .iter()
            .any(|frame| frame.command.name == "removeAll"),
        "remove_all must send a genGroups removeAll frame"
    );
}

/// Case 37: a non-existing endpoint is rejected before any coordinator call.
#[tokio::test]
async fn error_adding_to_non_existing_endpoint() {
    let _guard = serial();
    let harness = harness();
    harness.adapter.endpoint_frames.lock().unwrap().clear();

    let response = members_request(
        &harness,
        "add",
        json!({"group": "kitchen", "device": "lamp", "endpoint": "9"}),
    )
    .await;

    assert_eq!(response["status"], "error", "{response}");
    assert_eq!(
        response["error"], "Device 'lamp' does not have endpoint '9'",
        "{response}"
    );
    assert!(
        harness.adapter.endpoint_frames.lock().unwrap().is_empty(),
        "no coordinator call for an unknown endpoint"
    );
}

/// The `groupMembersChanged` event emitted by `Groups` makes the Bridge
/// extension republish `bridge/groups` (upstream groups.test.ts asserts the
/// `bridge/groups` publish; here the bridge listener is exercised end to end).
#[tokio::test]
async fn group_members_changed_republishes_bridge_groups() {
    let _guard = serial();
    let harness = harness();
    create_named_group(&harness, 2, "office");
    let mut bridge_context = harness.context.clone();
    bridge_context.key = 98;
    let bridge = z2m_rs::extension::bridge::Bridge::new(bridge_context);
    bridge.clone().start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(150)).await;
    let before = count_topic(&harness, "zigbee2mqtt/bridge/groups").await;

    let response = members_request(
        &harness,
        "add",
        json!({"group": "office", "device": "lamp"}),
    )
    .await;
    assert_eq!(response["status"], "ok", "{response}");

    let mut published = before;
    for _ in 0..50 {
        published = count_topic(&harness, "zigbee2mqtt/bridge/groups").await;
        if published > before {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        published > before,
        "bridge/groups was not republished after the membership change (before={before}, topics={:?})",
        harness
            .mqtt
            .lock()
            .await
            .attempted
            .iter()
            .map(|(topic, _, _)| topic.clone())
            .collect::<Vec<_>>()
    );
    bridge.stop().await.unwrap();
}
