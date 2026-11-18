// Original: .upstream/zigbee2mqtt/test/extensions/groups.test.ts and
// publish.test.ts — group command fan-out through Publish + Groups together.
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
use z2m_rs::extension::publish::Publish;
use z2m_rs::model::entity::PublishEntityState;
use z2m_rs::mqtt::Mqtt;
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::zigbee::Zigbee;
use zigbee_converters::{access, Definition, Expose, ToMeta, ToZigbeeConverter, ToZigbeeResult};

const IEEE: &str = "0x0017880104e45519";
const IEEE2: &str = "0x0017880104e45520";
const IEEE3: &str = "0x0017880104e45542";
const IEEE4: &str = "0x0017880104e45543";

type Recorded = Arc<Mutex<Vec<(String, Value, Option<String>)>>>;

struct Harness {
    context: ExtensionContext,
    event_bus: Arc<Mutex<EventBus>>,
    zigbee: Arc<Zigbee>,
    recorded: Recorded,
    database_path: std::path::PathBuf,
    state_path: std::path::PathBuf,
}

fn temp(name: &str) -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir().join(format!("z2m-gp-{name}-{}-{id}", std::process::id()))
}

fn harness(configuration: Value) -> Harness {
    harness_members(configuration, &[(IEEE, 1), (IEEE2, 1)])
}

fn harness_members(configuration: Value, members: &[(&str, u8)]) -> Harness {
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
    for (ieee, network, model, endpoints) in [
        (IEEE, 1234, "TEST", vec![1u8]),
        (IEEE2, 1235, "SWITCH", vec![1, 2, 3]),
        (IEEE3, 1236, "ZIGFRED", vec![7]),
        (IEEE4, 1237, "PLAIN", vec![1, 2]),
    ] {
        let device = ZhDevice::create(
            DeviceType::Router,
            ieee,
            network,
            None,
            Some("Test".to_string()),
            Some("Mains (single phase)".to_string()),
            Some(model.to_string()),
            InterviewState::Successful,
            None,
        )
        .unwrap();
        for endpoint in endpoints {
            device.create_endpoint(endpoint).unwrap();
        }
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
    let group = controller.create_group(1).unwrap();
    let settings = Arc::new(Mutex::new(Settings::from_config(configuration)));
    let event_bus = Arc::new(Mutex::new(EventBus::new()));
    let logger = Arc::new(Mutex::new(Logger::new()));
    let zigbee = Zigbee::with_controller(
        controller,
        event_bus.clone(),
        settings.clone(),
        logger.clone(),
    );
    for (ieee, endpoint_id) in members {
        if let Some(device) = zigbee.resolve_device(ieee) {
            if let Some(endpoint) = device.zh.get_endpoint(*endpoint_id) {
                group.add_member(&endpoint);
            }
        }
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
    let recorded: Recorded = Arc::new(Mutex::new(Vec::new()));
    let portable = recorded.clone();
    let portable_state = state.clone();
    let publish: PublishEntityState = Arc::new(move |entity, mut payload, reason| {
        let recorded = portable.clone();
        let state = portable_state.clone();
        Box::pin(async move {
            // Mirror `controller.publishEntityState`: the state cache receives
            // the unfiltered payload (and emits stateChange), the MQTT message
            // is filtered by `filtered_attributes`.
            let new_state =
                state
                    .lock()
                    .unwrap()
                    .set(entity.as_ref(), payload.clone(), reason.clone());
            // `advanced.cache_state` (default true) publishes the merged state.
            let mut message = new_state;
            z2m_rs::util::utils::filter_properties(
                entity
                    .options()
                    .get("filtered_attributes")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    })
                    .as_deref(),
                &mut message,
            );
            recorded
                .lock()
                .unwrap()
                .push((entity.name(), message, reason));
        })
    });

    let context = ExtensionContext {
        zigbee: zigbee.clone(),
        mqtt,
        state,
        publish_entity_state: publish,
        event_bus: event_bus.clone(),
        logger,
        settings,
        enable_disable_extension: Arc::new(|_, _| Box::pin(async { Ok(()) })),
        restart: Arc::new(|| Box::pin(async {})),
        add_extension: Arc::new(|_| Box::pin(async {})),
        key: 40,
    };

    Harness {
        context,
        event_bus,
        zigbee,
        recorded,
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

fn light_converters() -> Vec<ToZigbeeConverter> {
    let state = ToZigbeeConverter {
        key: vec!["state".to_string()],
        endpoints: None,
        convert_set: Some(Arc::new(|_target, _key, value, _meta: ToMeta| {
            Box::pin(async move {
                Ok(ToZigbeeResult {
                    state: Some(json!({"state": value})),
                    members_state: None,
                })
            })
        })),
        convert_get: None,
    };
    let brightness = ToZigbeeConverter {
        key: vec!["brightness".to_string()],
        endpoints: None,
        convert_set: Some(Arc::new(|_target, _key, value, _meta: ToMeta| {
            Box::pin(async move {
                Ok(ToZigbeeResult {
                    state: Some(json!({"brightness": value})),
                    members_state: None,
                })
            })
        })),
        convert_get: None,
    };
    let color_temp = ToZigbeeConverter {
        key: vec!["color_temp".to_string()],
        endpoints: None,
        convert_set: Some(Arc::new(|_target, _key, value, _meta: ToMeta| {
            Box::pin(async move {
                Ok(ToZigbeeResult {
                    state: Some(json!({"color_temp": value, "color_mode": "color_temp"})),
                    members_state: None,
                })
            })
        })),
        convert_get: None,
    };
    vec![state, brightness, color_temp]
}

fn register_model(zigbee: &Zigbee, model: &str, endpoints: Vec<(&str, u8)>) {
    register_model_with_meta(zigbee, model, endpoints, true);
}

fn register_model_with_meta(
    zigbee: &Zigbee,
    model: &str,
    endpoints: Vec<(&str, u8)>,
    multi_endpoint: bool,
) {
    let mut definition = Definition::new(model, "Acme", "Test")
        .with_zigbee_model(model)
        .with_exposes(vec![Expose {
            r#type: "light".to_string(),
            name: Some("light".to_string()),
            access: access::ALL,
            features: vec![
                Expose::binary("state", access::ALL),
                Expose::numeric("brightness", access::ALL),
                Expose::numeric("color_temp", access::ALL),
            ],
            ..Default::default()
        }]);
    if !endpoints.is_empty() {
        if multi_endpoint {
            definition.meta = json!({"multiEndpoint": true});
        }
        definition.endpoint = endpoints
            .iter()
            .map(|(name, id)| (name.to_string(), *id))
            .collect();
    }
    definition.to_zigbee.extend(light_converters());
    zigbee.converters().lock().unwrap().register(definition);
}

fn register_converter(zigbee: &Zigbee) {
    register_model(zigbee, "TEST", vec![]);
    register_model(zigbee, "SWITCH", vec![("left", 2), ("right", 3)]);
    register_model(zigbee, "ZIGFRED", vec![("l2", 7)]);
    register_model_with_meta(zigbee, "PLAIN", vec![("left", 2)], false);
    for ieee in [IEEE, IEEE2, IEEE3, IEEE4] {
        if let Some(device) = zigbee.resolve_device(ieee) {
            device.resolve_definition(true);
        }
    }
}

async fn start(harness: &Harness) -> (Arc<Publish>, Arc<Groups>) {
    let publish = Publish::new(harness.context.clone());
    publish.attach(1);
    let groups = Groups::new(harness.context.clone());
    groups.clone().start().await.unwrap();
    (publish, groups)
}

async fn send(harness: &Harness, topic: &str, message: Value) {
    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": topic,
        "message": message.to_string(),
    }));
    for _ in 0..100 {
        let recorded = harness.recorded.lock().unwrap();
        if !recorded.is_empty() {
            break;
        }
        drop(recorded);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
}

fn baseline_configuration() -> Value {
    json!({
        "mqtt": {"server": "localhost"},
        "devices": {
            IEEE: {"friendly_name": "lamp"},
            IEEE2: {"friendly_name": "bulb"},
        },
        "groups": {"1": {"friendly_name": "kitchen"}},
    })
}

fn baseline() -> Harness {
    harness(baseline_configuration())
}

/// Cases 4-5 and publish 25: a group command is published for the group and
/// fans out to all enabled members.
#[tokio::test]
async fn group_set_fans_out_to_members() {
    let _guard = serial();
    let harness = baseline();
    register_converter(&harness.zigbee);
    let (_publish, groups) = start(&harness).await;

    send(&harness, "zigbee2mqtt/kitchen/set", json!({"state": "ON"})).await;

    let recorded = harness.recorded.lock().unwrap();
    let names: Vec<&str> = recorded.iter().map(|(name, _, _)| name.as_str()).collect();
    assert!(names.contains(&"kitchen"), "{recorded:?}");
    assert!(names.contains(&"lamp"), "{recorded:?}");
    assert!(names.contains(&"bulb"), "{recorded:?}");
    assert!(recorded
        .iter()
        .filter(|(name, _, _)| name != "kitchen")
        .all(|(_, payload, _)| payload == &json!({"state": "ON"})));
    drop(recorded);
    groups.stop().await.unwrap();
}

/// Case 4: disabled members are skipped while the group is still published.
#[tokio::test]
async fn group_set_skips_disabled_members() {
    let _guard = serial();
    let mut configuration = json!({
        "mqtt": {"server": "localhost"},
        "devices": {
            IEEE: {"friendly_name": "lamp", "disabled": true},
            IEEE2: {"friendly_name": "bulb", "disabled": true},
        },
        "groups": {"1": {"friendly_name": "kitchen"}},
    });
    configuration.as_object_mut().unwrap();
    let harness = harness(configuration);
    register_converter(&harness.zigbee);
    let (_publish, groups) = start(&harness).await;

    send(&harness, "zigbee2mqtt/kitchen/set", json!({"state": "ON"})).await;

    let recorded = harness.recorded.lock().unwrap();
    assert!(
        recorded.iter().all(|(name, _, _)| name == "kitchen"),
        "disabled members published: {recorded:?}"
    );
    drop(recorded);
    groups.stop().await.unwrap();
}

/// Case 8: `filtered_attributes` removes properties from the group publish but
/// members still receive the full payload.
#[tokio::test]
async fn group_filtered_attributes_are_applied() {
    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "devices": {
            IEEE: {"friendly_name": "lamp"},
            IEEE2: {"friendly_name": "bulb"},
        },
        "groups": {"1": {"friendly_name": "kitchen", "filtered_attributes": ["brightness"]}},
    }));
    register_converter(&harness.zigbee);
    let (_publish, groups) = start(&harness).await;

    send(
        &harness,
        "zigbee2mqtt/kitchen/set",
        json!({"state": "ON", "brightness": 100}),
    )
    .await;

    let recorded = harness.recorded.lock().unwrap();
    let group = recorded
        .iter()
        .find(|(name, _, _)| name == "kitchen")
        .expect("group publish");
    assert_eq!(group.1, json!({"state": "ON"}), "{recorded:?}");
    let member = recorded
        .iter()
        .find(|(name, _, _)| name == "lamp")
        .expect("member publish");
    assert_eq!(member.1, json!({"state": "ON", "brightness": 100}));
    drop(recorded);
    groups.stop().await.unwrap();
}

fn entity(harness: &Harness, ieee: &str) -> Arc<dyn z2m_rs::model::entity::Entity> {
    harness
        .zigbee
        .resolve_device(ieee)
        .map(|device| device as Arc<dyn z2m_rs::model::entity::Entity>)
        .expect("device")
}

fn report(harness: &Harness, ieee: &str, payload: Value) {
    let device = entity(harness, ieee);
    harness.context.state.lock().unwrap().set(
        device.as_ref(),
        payload,
        Some("devicePublish".to_string()),
    );
}

fn last_group_state(harness: &Harness) -> Option<Value> {
    harness
        .recorded
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find(|(name, _, _)| name == "kitchen")
        .map(|(_, payload, _)| payload.clone())
}

/// Case 11: turning one member off does not turn the group off while another
/// member is still on.
#[tokio::test]
async fn group_stays_on_while_a_member_is_on() {
    let _guard = serial();
    let harness = baseline();
    register_converter(&harness.zigbee);
    let (_publish, groups) = start(&harness).await;
    send(&harness, "zigbee2mqtt/kitchen/set", json!({"state": "ON"})).await;
    harness.recorded.lock().unwrap().clear();

    report(&harness, IEEE, json!({"state": "OFF"}));
    tokio::time::sleep(Duration::from_millis(150)).await;

    assert_eq!(last_group_state(&harness), None, "group reported OFF");
    groups.stop().await.unwrap();
}

/// Case 20: the group turns off only after all members are off.
#[tokio::test]
async fn group_turns_off_when_all_members_are_off() {
    let _guard = serial();
    let harness = baseline();
    register_converter(&harness.zigbee);
    let (_publish, groups) = start(&harness).await;
    send(&harness, "zigbee2mqtt/kitchen/set", json!({"state": "ON"})).await;
    harness.recorded.lock().unwrap().clear();

    report(&harness, IEEE, json!({"state": "OFF"}));
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(last_group_state(&harness), None);
    report(&harness, IEEE2, json!({"state": "OFF"}));
    tokio::time::sleep(Duration::from_millis(150)).await;

    assert_eq!(last_group_state(&harness), Some(json!({"state": "OFF"})));
    groups.stop().await.unwrap();
}

/// Case 12: off_state last_member_state always follows the member change.
#[tokio::test]
async fn last_member_state_publishes_the_member_state() {
    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "devices": {
            IEEE: {"friendly_name": "lamp"},
            IEEE2: {"friendly_name": "bulb"},
        },
        "groups": {"1": {"friendly_name": "kitchen", "off_state": "last_member_state"}},
    }));
    register_converter(&harness.zigbee);
    let (_publish, groups) = start(&harness).await;
    send(&harness, "zigbee2mqtt/kitchen/set", json!({"state": "ON"})).await;
    harness.recorded.lock().unwrap().clear();

    report(&harness, IEEE, json!({"state": "OFF"}));
    tokio::time::sleep(Duration::from_millis(150)).await;

    assert_eq!(last_group_state(&harness), Some(json!({"state": "OFF"})));
    groups.stop().await.unwrap();
}

/// Case 21: the group publish merges with the cached group state.
#[tokio::test]
async fn group_state_merges_cached_properties() {
    let _guard = serial();
    let harness = baseline();
    register_converter(&harness.zigbee);
    let (_publish, groups) = start(&harness).await;
    send(&harness, "zigbee2mqtt/kitchen/set", json!({"state": "ON"})).await;
    harness.recorded.lock().unwrap().clear();

    report(&harness, IEEE, json!({"brightness": 100}));
    tokio::time::sleep(Duration::from_millis(150)).await;

    assert_eq!(
        last_group_state(&harness),
        Some(json!({"state": "ON", "brightness": 100}))
    );
    groups.stop().await.unwrap();
}

/// Case 22: an OFF transition is published even when the cache was cleared.
#[tokio::test]
async fn group_off_publishes_with_missing_current_state() {
    let _guard = serial();
    let harness = baseline();
    register_converter(&harness.zigbee);
    let (_publish, groups) = start(&harness).await;
    send(&harness, "zigbee2mqtt/kitchen/set", json!({"state": "ON"})).await;
    harness.recorded.lock().unwrap().clear();
    harness.context.state.lock().unwrap().clear();

    report(&harness, IEEE, json!({"state": "OFF"}));
    tokio::time::sleep(Duration::from_millis(150)).await;

    assert!(
        last_group_state(&harness).is_some(),
        "group OFF was suppressed without cached state"
    );
    groups.stop().await.unwrap();
}

/// Publish case 36: `optimistic: false` on a group suppresses the publish.
#[tokio::test]
async fn non_optimistic_group_does_not_publish() {
    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "devices": {
            IEEE: {"friendly_name": "lamp"},
            IEEE2: {"friendly_name": "bulb"},
        },
        "groups": {"1": {"friendly_name": "kitchen", "optimistic": false}},
    }));
    register_converter(&harness.zigbee);
    let (_publish, groups) = start(&harness).await;

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": "zigbee2mqtt/kitchen/set",
        "message": r#"{"brightness":200}"#,
    }));
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert!(
        harness.recorded.lock().unwrap().is_empty(),
        "non-optimistic group published {:?}",
        harness.recorded.lock().unwrap()
    );
    groups.stop().await.unwrap();
}

/// Case 13: a non-default endpoint that is still on keeps the group on.
#[tokio::test]
async fn non_default_endpoint_on_keeps_group_on() {
    let _guard = serial();
    let harness = harness_members(baseline_configuration(), &[(IEEE, 1), (IEEE2, 2)]);
    register_converter(&harness.zigbee);
    let (_publish, groups) = start(&harness).await;
    send(&harness, "zigbee2mqtt/kitchen/set", json!({"state": "ON"})).await;
    harness.recorded.lock().unwrap().clear();

    send(&harness, "zigbee2mqtt/lamp/set", json!({"state": "OFF"})).await;

    assert_eq!(last_group_state(&harness), None, "group turned off early");
    groups.stop().await.unwrap();
}

/// Case 14: turning only the left endpoint off keeps the group on because the
/// right endpoint is still on.
#[tokio::test]
async fn non_default_endpoints_keep_group_on() {
    let _guard = serial();
    let harness = harness_members(
        baseline_configuration(),
        &[(IEEE, 1), (IEEE2, 2), (IEEE2, 3)],
    );
    register_converter(&harness.zigbee);
    let (_publish, groups) = start(&harness).await;
    send(&harness, "zigbee2mqtt/kitchen/set", json!({"state": "ON"})).await;
    harness.recorded.lock().unwrap().clear();

    send(
        &harness,
        "zigbee2mqtt/bulb/set",
        json!({"state_left": "OFF"}),
    )
    .await;

    assert_eq!(last_group_state(&harness), None, "group turned off early");
    let member = harness
        .recorded
        .lock()
        .unwrap()
        .iter()
        .find(|(name, _, _)| name == "bulb")
        .cloned();
    assert_eq!(
        member.map(|(_, payload, _)| payload),
        Some(json!({"state_left": "OFF", "state_right": "ON"}))
    );
    groups.stop().await.unwrap();
}

/// Case 15: the group turns off once all endpoints are off.
#[tokio::test]
async fn all_endpoints_off_publishes_group_off() {
    let _guard = serial();
    let harness = harness_members(baseline_configuration(), &[(IEEE, 1), (IEEE2, 2)]);
    register_converter(&harness.zigbee);
    let (_publish, groups) = start(&harness).await;
    send(&harness, "zigbee2mqtt/kitchen/set", json!({"state": "ON"})).await;
    harness.recorded.lock().unwrap().clear();

    send(&harness, "zigbee2mqtt/lamp/set", json!({"state": "OFF"})).await;
    assert_eq!(last_group_state(&harness), None);
    send(
        &harness,
        "zigbee2mqtt/bulb/set",
        json!({"state_left": "OFF"}),
    )
    .await;

    assert_eq!(last_group_state(&harness), Some(json!({"state": "OFF"})));
    groups.stop().await.unwrap();
}

/// Cases 17-18: the member endpoint is looked up by name, not by position.
#[tokio::test]
async fn endpoint_state_is_resolved_by_name() {
    for (state_l2, expect_off) in [("ON", false), ("OFF", true)] {
        let _guard = serial();
        let harness = harness_members(baseline_configuration(), &[(IEEE, 1), (IEEE3, 7)]);
        register_converter(&harness.zigbee);
        let (_publish, groups) = start(&harness).await;
        send(&harness, "zigbee2mqtt/kitchen/set", json!({"state": "ON"})).await;
        {
            let device = entity(&harness, IEEE3);
            harness.context.state.lock().unwrap().set(
                device.as_ref(),
                json!({"state_l2": state_l2}),
                Some("devicePublish".to_string()),
            );
        }
        harness.recorded.lock().unwrap().clear();

        send(&harness, "zigbee2mqtt/lamp/set", json!({"state": "OFF"})).await;

        let group_off = last_group_state(&harness) == Some(json!({"state": "OFF"}));
        assert_eq!(group_off, expect_off, "state_l2={state_l2}");
        groups.stop().await.unwrap();
    }
}

/// Case 16: with a device whose state does not use the endpoint, the group off
/// is published even though an endpoint-suffixed state exists.
#[tokio::test]
async fn group_off_when_device_state_does_not_use_the_endpoint() {
    let _guard = serial();
    let harness = harness_members(baseline_configuration(), &[(IEEE, 1), (IEEE4, 2)]);
    register_converter(&harness.zigbee);
    let (_publish, groups) = start(&harness).await;
    send(&harness, "zigbee2mqtt/kitchen/set", json!({"state": "ON"})).await;
    harness.recorded.lock().unwrap().clear();

    send(&harness, "zigbee2mqtt/lamp/set", json!({"state": "OFF"})).await;

    assert_eq!(
        last_group_state(&harness),
        Some(json!({"state": "OFF"})),
        "group off was suppressed by an endpoint-suffixed state"
    );
    groups.stop().await.unwrap();
}
