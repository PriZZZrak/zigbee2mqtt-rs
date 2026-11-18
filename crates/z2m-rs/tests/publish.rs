// Original: .upstream/zigbee2mqtt/test/extensions/publish.test.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// R1 fallback: definition-less devices log `Cannot publish to unsupported
// device`. Registered toZigbee converters cover set/get dispatch.

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
use z2m_rs::extension::extension::ExtensionContext;
use z2m_rs::extension::publish::{ParsedTopic, Publish};
use z2m_rs::model::entity::PublishEntityState;
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::zigbee::Zigbee;
use zigbee_converters::{Definition, Expose, ToMeta, ToZigbeeConverter, ToZigbeeResult};

const IEEE: &str = "0x0017880104e45519";

type Recorded = Arc<Mutex<Vec<(Value, Option<String>)>>>;

struct Harness {
    event_bus: Arc<Mutex<EventBus>>,
    settings: Arc<Mutex<Settings>>,
    zigbee: Arc<Zigbee>,
    publish: Arc<Publish>,
    logger: Arc<Mutex<Logger>>,
    recorded: Recorded,
    database_path: std::path::PathBuf,
    state_path: std::path::PathBuf,
}

fn temp(name: &str) -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir().join(format!("z2m-publish-{name}-{}-{id}", std::process::id()))
}

fn harness(configuration: Value) -> Harness {
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
        None,
        Some("TEST".to_string()),
        InterviewState::Successful,
        None,
    )
    .unwrap();
    device.create_endpoint(1).unwrap();
    device.create_endpoint(2).unwrap();

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

    let settings = Arc::new(Mutex::new(Settings::from_config(configuration)));
    let event_bus = Arc::new(Mutex::new(EventBus::new()));
    let mut logger = Logger::new();
    logger.level = "debug".to_string();
    let logger = Arc::new(Mutex::new(logger));
    let zigbee = Zigbee::with_controller(
        controller,
        event_bus.clone(),
        settings.clone(),
        logger.clone(),
    );

    // Startup resolves devices, which ensures them in settings (`zigbee.ts:290`).
    let _ = zigbee.resolve_device(IEEE);

    let state_path = temp("state.json");
    let state = Arc::new(Mutex::new(State::with_file(
        settings.clone(),
        event_bus.clone(),
        logger.clone(),
        state_path.clone(),
    )));

    let recorded: Recorded = Arc::new(Mutex::new(Vec::new()));
    let sink = recorded.clone();
    let publish_entity_state: PublishEntityState = Arc::new(move |_entity, payload, reason| {
        let sink = sink.clone();
        Box::pin(async move {
            sink.lock().unwrap().push((payload, reason));
        })
    });

    let context = ExtensionContext {
        zigbee: zigbee.clone(),
        mqtt: Arc::new(tokio::sync::Mutex::new(z2m_rs::mqtt::Mqtt::new(
            event_bus.clone(),
        ))),
        state: state.clone(),
        publish_entity_state: publish_entity_state.clone(),
        event_bus: event_bus.clone(),
        logger: logger.clone(),
        settings: settings.clone(),
        enable_disable_extension: Arc::new(|_, _| Box::pin(async { Ok(()) })),
        restart: Arc::new(|| Box::pin(async {})),
        add_extension: Arc::new(|_| Box::pin(async {})),
        key: 1,
    };
    let publish = Publish::new(context);
    publish.attach(1);

    Harness {
        event_bus,
        settings,
        zigbee: zigbee.clone(),
        publish,
        logger,
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

fn register_converter(zigbee: &Zigbee, calls: Arc<Mutex<Vec<(String, Value)>>>) {
    let converter = ToZigbeeConverter {
        key: vec!["state".to_string()],
        endpoints: None,
        convert_set: Some(Arc::new(move |_target, key, value, _meta: ToMeta| {
            let calls = calls.clone();
            Box::pin(async move {
                calls.lock().unwrap().push((key, value));
                Ok(ToZigbeeResult {
                    state: Some(json!({"state": "ON"})),
                    members_state: None,
                })
            })
        })),
        convert_get: Some(Arc::new(|_target, _key, _value, _meta| {
            Box::pin(async move { Ok(ToZigbeeResult::default()) })
        })),
    };
    let mut definition = Definition::new("TEST", "Acme", "Test")
        .with_zigbee_model("TEST")
        .with_exposes(vec![Expose::binary(
            "state",
            zigbee_converters::access::ALL,
        )]);
    definition.to_zigbee.push(converter);
    zigbee.converters().lock().unwrap().register(definition);
}

async fn wait_for_publishes(recorded: &Recorded, count: usize) {
    for _ in 0..100 {
        if recorded.lock().unwrap().len() >= count {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("expected {count} publishes");
}

#[test]
fn parse_message_variants() {
    let attribute = |name: &str| ParsedTopic {
        id: "lamp".to_string(),
        endpoint: None,
        attribute: Some(name.to_string()),
        r#type: "set".to_string(),
    };
    let plain = || ParsedTopic {
        id: "lamp".to_string(),
        endpoint: None,
        attribute: None,
        r#type: "set".to_string(),
    };

    assert_eq!(
        Publish::parse_message(&attribute("brightness"), "42"),
        Some(json!({"brightness": 42}))
    );
    assert_eq!(
        Publish::parse_message(&attribute("effect"), "rainbow"),
        Some(json!({"effect": "rainbow"}))
    );
    assert_eq!(
        Publish::parse_message(&plain(), r#"{"state":"ON"}"#),
        Some(json!({"state": "ON"}))
    );
    assert_eq!(
        Publish::parse_message(&plain(), "toggle"),
        Some(json!({"state": "toggle"}))
    );
    assert_eq!(Publish::parse_message(&plain(), "not-a-state"), None);
}

#[tokio::test]
async fn parse_topic_resolves_entity_and_endpoint() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));

    let set = harness
        .publish
        .parse_topic("zigbee2mqtt/0x0017880104e45519/set")
        .unwrap();
    assert_eq!(set.id, IEEE);
    assert_eq!(set.r#type, "set");
    assert_eq!(set.attribute, None);

    let with_attribute = harness
        .publish
        .parse_topic("zigbee2mqtt/0x0017880104e45519/set/brightness")
        .unwrap();
    assert_eq!(with_attribute.attribute.as_deref(), Some("brightness"));

    let with_endpoint = harness
        .publish
        .parse_topic("zigbee2mqtt/0x0017880104e45519/2/get")
        .unwrap();
    assert_eq!(with_endpoint.endpoint.as_deref(), Some("2"));
    assert_eq!(with_endpoint.r#type, "get");

    assert!(harness
        .publish
        .parse_topic("zigbee2mqtt/bridge/request/options")
        .is_none());
    assert!(harness.publish.parse_topic("other/topic/set").is_none());
}

#[tokio::test]
async fn unsupported_device_is_rejected() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": format!("zigbee2mqtt/{IEEE}/set"),
        "message": r#"{"state":"ON"}"#,
    }));

    for _ in 0..50 {
        if harness
            .logger
            .lock()
            .unwrap()
            .winston_logs
            .iter()
            .any(|(_, line)| line.contains("Cannot publish to unsupported device"))
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    assert!(harness
        .logger
        .lock()
        .unwrap()
        .winston_logs
        .iter()
        .any(|(_, line)| line == &format!("z2m: Cannot publish to unsupported device '{IEEE}'")));
    assert!(harness.recorded.lock().unwrap().is_empty());
}

#[tokio::test]
async fn set_and_get_dispatch_to_converters() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    let calls: Arc<Mutex<Vec<(String, Value)>>> = Arc::new(Mutex::new(Vec::new()));
    register_converter(&harness.zigbee, calls.clone());

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": format!("zigbee2mqtt/{IEEE}/set"),
        "message": r#"{"state":"ON"}"#,
    }));
    wait_for_publishes(&harness.recorded, 1).await;

    assert_eq!(
        calls.lock().unwrap().as_slice(),
        [("state".to_string(), json!("ON"))]
    );
    assert_eq!(
        harness.recorded.lock().unwrap()[0].0,
        json!({"state": "ON"})
    );

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": format!("zigbee2mqtt/{IEEE}/get"),
        "message": r#"{"state":""}"#,
    }));
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(harness.recorded.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn missing_endpoint_and_converter_are_logged() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    let calls: Arc<Mutex<Vec<(String, Value)>>> = Arc::new(Mutex::new(Vec::new()));
    register_converter(&harness.zigbee, calls);
    harness
        .zigbee
        .resolve_device(IEEE)
        .unwrap()
        .resolve_definition(true);
    assert!(
        harness
            .zigbee
            .resolve_device(IEEE)
            .unwrap()
            .definition()
            .is_some(),
        "definition not resolved"
    );

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": format!("zigbee2mqtt/{IEEE}/center/get"),
        "message": r#"{"state":""}"#,
    }));
    for _ in 0..50 {
        if harness
            .logger
            .lock()
            .unwrap()
            .winston_logs
            .iter()
            .any(|(_, line)| line.contains("has no endpoint 'center'"))
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        harness
            .logger
            .lock()
            .unwrap()
            .winston_logs
            .iter()
            .any(|(_, line)| line.contains("has no endpoint 'center'")),
        "logs={:?}",
        harness.logger.lock().unwrap().winston_logs
    );

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": format!("zigbee2mqtt/{IEEE}/set"),
        "message": r#"{"unknown_property": 5}"#,
    }));
    for _ in 0..50 {
        if harness
            .logger
            .lock()
            .unwrap()
            .winston_logs
            .iter()
            .any(|(_, line)| line.contains("No converter available for 'set' 'unknown_property'"))
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(harness
        .logger
        .lock()
        .unwrap()
        .winston_logs
        .iter()
        .any(|(_, line)| line.contains("No converter available for 'unknown_property' on")));
}

fn register_state_converter(
    zigbee: &Zigbee,
    calls: Arc<Mutex<Vec<(String, Value)>>>,
    state: Value,
) {
    let get_calls = calls.clone();
    let converter = ToZigbeeConverter {
        key: vec!["state".to_string(), "brightness".to_string()],
        endpoints: None,
        convert_set: Some(Arc::new(move |_target, key, value, _meta: ToMeta| {
            let calls = calls.clone();
            let state = state.clone();
            Box::pin(async move {
                calls.lock().unwrap().push((key, value));
                Ok(ToZigbeeResult {
                    state: Some(state),
                    members_state: None,
                })
            })
        })),
        convert_get: Some(Arc::new(move |_target, key, _value, _meta| {
            let calls = get_calls.clone();
            Box::pin(async move {
                calls
                    .lock()
                    .unwrap()
                    .push((format!("get:{key}"), Value::Null));
                Ok(ToZigbeeResult::default())
            })
        })),
    };
    let mut definition = Definition::new("TEST", "Acme", "Test")
        .with_zigbee_model("TEST")
        .with_exposes(vec![
            Expose::binary("state", zigbee_converters::access::ALL),
            Expose::numeric("brightness", zigbee_converters::access::ALL),
        ]);
    definition.to_zigbee.push(converter);
    zigbee.converters().lock().unwrap().register(definition);
}

/// Cases 37-41: topics that are not `<base>/<entity>/<get|set>` are ignored.
#[tokio::test]
async fn invalid_topics_are_ignored() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    for topic in [
        format!("zigbee2mqtt1/{IEEE}/set"),
        format!("zigbee2mqtt1/{IEEE}/sett"),
        format!("zigbee2mqtt/{IEEE}/write"),
        "zigbee2mqtt/set".to_string(),
        "set".to_string(),
    ] {
        assert!(
            harness.publish.parse_topic(&topic).is_none(),
            "topic '{topic}' must not parse"
        );
        harness
            .event_bus
            .lock()
            .unwrap()
            .emit_mqtt_message(json!({"topic": topic, "message": r#"{"state":"ON"}"#}));
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(harness.recorded.lock().unwrap().is_empty());
}

/// Cases 48-50, 59 and 60: slashes in the base topic, friendly name and
/// numeric name suffixes still resolve the entity and endpoint.
#[tokio::test]
async fn topic_parsing_handles_slashes() {
    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost", "base_topic": "zigbee2mqtt/at/my/home"},
    }));
    harness.settings.lock().unwrap().configuration["devices"][IEEE]["friendly_name"] =
        json!("floor0/basement/lamp");

    let parsed = harness
        .publish
        .parse_topic("zigbee2mqtt/at/my/home/floor0/basement/lamp/get")
        .expect("slashed base and device name");
    assert_eq!(parsed.id, "floor0/basement/lamp");
    assert_eq!(parsed.r#type, "get");

    let parsed = harness
        .publish
        .parse_topic("zigbee2mqtt/at/my/home/floor0/basement/lamp/2/set")
        .expect("endpoint after a numeric name part");
    assert_eq!(parsed.endpoint.as_deref(), Some("2"));
    assert_eq!(parsed.r#type, "set");

    // A friendly name that ends with a number and has a named endpoint postfix.
    harness.settings.lock().unwrap().configuration["devices"][IEEE]["friendly_name"] =
        json!("ground_floor/kitchen/switch/2");
    let parsed = harness
        .publish
        .parse_topic("zigbee2mqtt/at/my/home/ground_floor/kitchen/switch/2/1/get")
        .expect("numeric name suffix with endpoint");
    assert_eq!(parsed.id, "ground_floor/kitchen/switch/2");
    assert_eq!(parsed.endpoint.as_deref(), Some("1"));
}

/// Cases 43-44: get dispatches per endpoint for multi-endpoint devices.
#[tokio::test]
async fn get_dispatches_multiple_endpoints() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    let calls: Arc<Mutex<Vec<(String, Value)>>> = Arc::new(Mutex::new(Vec::new()));
    register_state_converter(&harness.zigbee, calls.clone(), json!({"state": "ON"}));
    harness
        .zigbee
        .resolve_device(IEEE)
        .unwrap()
        .resolve_definition(true);

    for endpoint in ["1", "2"] {
        harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
            "topic": format!("zigbee2mqtt/{IEEE}/{endpoint}/get"),
            "message": r#"{"state":""}"#,
        }));
    }
    for _ in 0..50 {
        if calls.lock().unwrap().len() >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(calls.lock().unwrap().len(), 2, "both endpoints were read");
}

/// Cases 51, 52, 56 and 58: attribute/postfix set topics reach the converter.
#[tokio::test]
async fn set_attribute_and_postfix_topics_dispatch() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    let calls: Arc<Mutex<Vec<(String, Value)>>> = Arc::new(Mutex::new(Vec::new()));
    register_state_converter(&harness.zigbee, calls.clone(), json!({"state": "ON"}));
    harness
        .zigbee
        .resolve_device(IEEE)
        .unwrap()
        .resolve_definition(true);

    for topic in [
        format!("zigbee2mqtt/{IEEE}/set/state"),
        format!("zigbee2mqtt/{IEEE}/1/set/state"),
        // IEEE addresses work as the entity id as well (case 53).
        format!("zigbee2mqtt/{IEEE}/set"),
    ] {
        harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
            "topic": topic,
            "message": "ON",
        }));
    }
    for _ in 0..50 {
        if calls.lock().unwrap().len() >= 3 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0], ("state".to_string(), json!("ON")));
    assert_eq!(calls[1], ("state".to_string(), json!("ON")));
    assert_eq!(calls[2], ("state".to_string(), json!("ON")));
}

/// Case 54: an unknown endpoint postfix is not dispatched.
#[tokio::test]
async fn non_existing_postfix_is_not_dispatched() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    let calls: Arc<Mutex<Vec<(String, Value)>>> = Arc::new(Mutex::new(Vec::new()));
    register_state_converter(&harness.zigbee, calls.clone(), json!({"state": "ON"}));
    harness
        .zigbee
        .resolve_device(IEEE)
        .unwrap()
        .resolve_definition(true);

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": format!("zigbee2mqtt/{IEEE}/center/set"),
        "message": r#"{"state":"ON"}"#,
    }));
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(calls.lock().unwrap().is_empty());
}

/// Case 57: non-JSON values other than on/off/toggle are rejected.
#[tokio::test]
async fn invalid_state_message_is_not_dispatched() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    let calls: Arc<Mutex<Vec<(String, Value)>>> = Arc::new(Mutex::new(Vec::new()));
    register_state_converter(&harness.zigbee, calls.clone(), json!({"state": "ON"}));
    harness
        .zigbee
        .resolve_device(IEEE)
        .unwrap()
        .resolve_definition(true);

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": format!("zigbee2mqtt/{IEEE}/set"),
        "message": "ON_RANDOM",
    }));
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(calls.lock().unwrap().is_empty());
}

/// Case 34: `optimistic: false` suppresses the optimistic state publish.
#[tokio::test]
async fn optimistic_false_suppresses_the_publish() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    harness.settings.lock().unwrap().configuration["devices"][IEEE]["optimistic"] = json!(false);
    let calls: Arc<Mutex<Vec<(String, Value)>>> = Arc::new(Mutex::new(Vec::new()));
    register_state_converter(&harness.zigbee, calls.clone(), json!({"state": "ON"}));
    harness
        .zigbee
        .resolve_device(IEEE)
        .unwrap()
        .resolve_definition(true);

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": format!("zigbee2mqtt/{IEEE}/set"),
        "message": r#"{"state":"ON"}"#,
    }));
    for _ in 0..50 {
        if !calls.lock().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(calls.lock().unwrap().len(), 1, "the converter still runs");
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        harness.recorded.lock().unwrap().is_empty(),
        "optimistic:false published {:?}",
        harness.recorded.lock().unwrap()
    );
}

/// Case 35: `filtered_optimistic` removes the listed properties from the
/// optimistic publish.
#[tokio::test]
async fn filtered_optimistic_filters_the_publish() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    harness.settings.lock().unwrap().configuration["devices"][IEEE]["filtered_optimistic"] =
        json!(["brightness"]);
    let calls: Arc<Mutex<Vec<(String, Value)>>> = Arc::new(Mutex::new(Vec::new()));
    register_state_converter(
        &harness.zigbee,
        calls,
        json!({"state": "ON", "brightness": 100}),
    );
    harness
        .zigbee
        .resolve_device(IEEE)
        .unwrap()
        .resolve_definition(true);

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": format!("zigbee2mqtt/{IEEE}/set"),
        "message": r#"{"brightness":100}"#,
    }));
    wait_for_publishes(&harness.recorded, 1).await;
    assert_eq!(
        harness.recorded.lock().unwrap()[0].0,
        json!({"state": "ON"}),
        "brightness must be filtered out"
    );
}

/// Case 44: get with multiple cover endpoints dispatches per endpoint.
#[tokio::test]
async fn get_dispatches_multiple_cover_endpoints() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    let calls: Arc<Mutex<Vec<(String, Value)>>> = Arc::new(Mutex::new(Vec::new()));
    let get_calls = calls.clone();
    let converter = ToZigbeeConverter {
        key: vec!["position".to_string()],
        endpoints: Some(vec!["1".to_string(), "2".to_string()]),
        convert_set: None,
        convert_get: Some(Arc::new(move |_target, key, _value, _meta| {
            let calls = get_calls.clone();
            Box::pin(async move {
                calls
                    .lock()
                    .unwrap()
                    .push((format!("get:{key}"), Value::Null));
                Ok(ToZigbeeResult::default())
            })
        })),
    };
    let mut definition = Definition::new("TEST", "Acme", "Test")
        .with_zigbee_model("TEST")
        .with_exposes(vec![Expose::numeric(
            "position",
            zigbee_converters::access::ALL,
        )]);
    definition.to_zigbee.push(converter);
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
        .resolve_definition(true);

    for endpoint in ["1", "2"] {
        harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
            "topic": format!("zigbee2mqtt/{IEEE}/{endpoint}/get"),
            "message": r#"{"position":""}"#,
        }));
    }
    for _ in 0..50 {
        if calls.lock().unwrap().len() >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(calls.lock().unwrap().len(), 2);
}

/// Case 47: a property with an unknown endpoint postfix logs an error.
#[tokio::test]
async fn unknown_endpoint_property_is_logged() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    let calls: Arc<Mutex<Vec<(String, Value)>>> = Arc::new(Mutex::new(Vec::new()));
    register_state_converter(&harness.zigbee, calls.clone(), json!({"state": "ON"}));
    harness
        .zigbee
        .resolve_device(IEEE)
        .unwrap()
        .resolve_definition(true);

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": format!("zigbee2mqtt/{IEEE}/get"),
        "message": r#"{"state_left":""}"#,
    }));
    for _ in 0..50 {
        if harness
            .logger
            .lock()
            .unwrap()
            .winston_logs
            .iter()
            .any(|(_, line)| line.contains("No converter available for 'state_left'"))
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        harness
            .logger
            .lock()
            .unwrap()
            .winston_logs
            .iter()
            .any(|(_, line)| { line.contains("No converter available for 'state_left'") }),
        "logs={:?}",
        harness.logger.lock().unwrap().winston_logs
    );
    assert!(calls.lock().unwrap().is_empty());
}
