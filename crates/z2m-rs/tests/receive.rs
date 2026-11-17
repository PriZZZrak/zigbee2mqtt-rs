// Original: .upstream/zigbee2mqtt/test/extensions/receive.test.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// R1 has no converter database, so definition-less devices follow the upstream
// fallback (`receive.ts:130-134`). Registered fromZigbee converters cover the
// conversion path.

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
use z2m_rs::extension::receive::Receive;
use z2m_rs::model::entity::PublishEntityState;
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::zigbee::Zigbee;
use zigbee_converters::{Definition, Expose, FromZigbeeConverter};

const IEEE: &str = "0x0017880104e45519";

type Recorded = Arc<Mutex<Vec<(Value, Option<String>)>>>;

struct Harness {
    zigbee: Arc<Zigbee>,
    event_bus: Arc<Mutex<EventBus>>,
    receive: Arc<Receive>,
    logger: Arc<Mutex<Logger>>,
    recorded: Recorded,
    state: Arc<Mutex<State>>,
    database_path: std::path::PathBuf,
    state_path: std::path::PathBuf,
}

fn temp(name: &str) -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir().join(format!("z2m-receive-{name}-{}-{id}", std::process::id()))
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
        Some(4476),
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

    let state_path = temp("state.json");
    let state = Arc::new(Mutex::new(State::with_file(
        settings.clone(),
        event_bus.clone(),
        logger.clone(),
        state_path.clone(),
    )));

    let recorded: Recorded = Arc::new(Mutex::new(Vec::new()));
    let sink = recorded.clone();
    let publish: PublishEntityState = Arc::new(move |_entity, payload, reason| {
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
        publish_entity_state: publish.clone(),
        event_bus: event_bus.clone(),
        logger: logger.clone(),
        settings,
        enable_disable_extension: Arc::new(|_, _| Box::pin(async { Ok(()) })),
        restart: Arc::new(|| Box::pin(async {})),
        add_extension: Arc::new(|_| Box::pin(async {})),
        key: 1,
    };
    let receive = Receive::new(context);
    receive.attach(1);

    Harness {
        zigbee,
        event_bus,
        receive,
        logger,
        recorded,
        state,
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

fn message(cluster: &str, data: Value) -> Value {
    json!({
        "type": "attributeReport",
        "device": {"ID": IEEE, "ieeeAddr": IEEE, "name": IEEE},
        "endpoint": {"ID": 1},
        "cluster": cluster,
        "data": data,
        "linkquality": 100,
        "meta": {"zclTransactionSequenceNumber": 1},
    })
}

async fn wait_for_publishes(recorded: &Recorded, count: usize) {
    for _ in 0..100 {
        if recorded.lock().unwrap().len() >= count {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!(
        "expected {count} publishes, got {:?}",
        recorded.lock().unwrap()
    );
}

fn register_on_off_converter(zigbee: &Zigbee) {
    let mut definition = Definition::new("TEST", "Acme", "Test device")
        .with_zigbee_model("TEST")
        .with_exposes(vec![Expose::binary(
            "state",
            zigbee_converters::access::ALL,
        )]);
    definition.from_zigbee.push(FromZigbeeConverter {
        cluster: "genOnOff".to_string(),
        types: vec!["attributeReport".to_string()],
        convert: Arc::new(|_definition, _data, _publish, _options, _meta| {
            Box::pin(async move { Ok(Some(json!({"state": "ON"}))) })
        }),
    });
    zigbee.converters().lock().unwrap().register(definition);
}

#[tokio::test]
async fn definition_less_device_skips_message_and_publishes_last_seen() {
    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "advanced": {"last_seen": "ISO_8601"},
    }));

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_device_message(message("genOnOff", json!({"onOff": 1})));

    wait_for_publishes(&harness.recorded, 1).await;

    let recorded = harness.recorded.lock().unwrap();
    assert_eq!(recorded[0].0, json!({}));
    assert_eq!(recorded[0].1, Some("lastSeenChanged".to_string()));
    assert!(harness
        .logger
        .lock()
        .unwrap()
        .winston_logs
        .iter()
        .any(|(_, line)| line == "z2m: Skipping message, still interviewing"));
}

#[tokio::test]
async fn registered_converter_publishes_converted_payload() {
    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "advanced": {"last_seen": "ISO_8601"},
    }));
    register_on_off_converter(&harness.zigbee);

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_device_message(message("genOnOff", json!({"onOff": 1})));

    wait_for_publishes(&harness.recorded, 1).await;

    let recorded = harness.recorded.lock().unwrap();
    assert_eq!(recorded[0].0, json!({"state": "ON"}));
    assert_eq!(recorded[0].1, None);
    assert!(!harness
        .logger
        .lock()
        .unwrap()
        .winston_logs
        .iter()
        .any(|(_, line)| line.starts_with("z2m: Skipping message")));
}

#[tokio::test]
async fn unsupported_cluster_publishes_last_seen_only() {
    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "advanced": {"last_seen": "ISO_8601"},
    }));
    register_on_off_converter(&harness.zigbee);

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_device_message(message(
            "msTemperatureMeasurement",
            json!({"measuredValue": 2100}),
        ));

    wait_for_publishes(&harness.recorded, 1).await;

    let recorded = harness.recorded.lock().unwrap();
    assert_eq!(recorded[0].0, json!({}));
    assert_eq!(recorded[0].1, Some("lastSeenChanged".to_string()));
    assert!(harness
        .logger
        .lock()
        .unwrap()
        .winston_logs
        .iter()
        .any(|(_, line)| line.contains(
            "No converter available for 'TEST' with cluster 'msTemperatureMeasurement' and type 'attributeReport'"
        )));
}

#[tokio::test]
async fn debounce_publishes_once_with_debounce_reason_and_updates_cache() {
    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "advanced": {"last_seen": "ISO_8601"},
        "devices": {IEEE: {"friendly_name": "lamp", "debounce": 1}},
    }));
    register_on_off_converter(&harness.zigbee);

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_device_message(message("genOnOff", json!({"onOff": 1})));

    // Cached state is updated before the debounce timer fires.
    for _ in 0..50 {
        let cached = harness
            .state
            .lock()
            .unwrap()
            .get(&z2m_rs::model::EntityId::Ieee(IEEE.to_string()));
        if cached == json!({"state": "ON"}) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(
        harness
            .state
            .lock()
            .unwrap()
            .get(&z2m_rs::model::EntityId::Ieee(IEEE.to_string())),
        json!({"state": "ON"})
    );

    wait_for_publishes(&harness.recorded, 1).await;
    let recorded = harness.recorded.lock().unwrap();
    assert_eq!(recorded[0].0, json!({"state": "ON"}));
    assert_eq!(recorded[0].1, Some("publishDebounce".to_string()));
    let _ = &harness.receive;
}

#[test]
fn payload_conflict_uses_debounce_ignore() {
    let ignore = vec!["state".to_string()];
    assert!(z2m_rs::extension::receive::Receive::is_payload_conflicted(
        &json!({"state": "OFF"}),
        &json!({"state": "ON"}),
        Some(&ignore)
    ));
    assert!(!z2m_rs::extension::receive::Receive::is_payload_conflicted(
        &json!({"brightness": 1}),
        &json!({"state": "ON"}),
        Some(&ignore)
    ));
    assert!(!z2m_rs::extension::receive::Receive::is_payload_conflicted(
        &json!({"state": "OFF"}),
        &json!({"state": "ON"}),
        None
    ));
}

#[tokio::test]
async fn last_seen_from_non_emitted_frame_publishes_immediately() {
    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "advanced": {"last_seen": "ISO_8601"},
        "devices": {IEEE: {"friendly_name": "lamp"}},
    }));
    let device = harness.zigbee.resolve_device(IEEE).unwrap();

    // Controller-level handler: `publishLastSeen` must publish every
    // `messageNonEmitted` event when the device has no debounce/throttle.
    let published: Recorded = Arc::new(Mutex::new(Vec::new()));
    let sink = published.clone();
    let publish: z2m_rs::model::entity::PublishEntityState =
        Arc::new(move |_entity, payload, reason| {
            let sink = sink.clone();
            Box::pin(async move {
                sink.lock().unwrap().push((payload, reason));
            })
        });

    for _ in 0..2 {
        z2m_rs::util::utils::publish_last_seen(
            &json!({
                "device": {"ieeeAddr": IEEE, "name": "lamp"},
                "reason": "messageNonEmitted",
            }),
            device.clone() as Arc<dyn z2m_rs::model::entity::Entity>,
            &json!({"advanced": {"last_seen": "ISO_8601"}}),
            false,
            &publish,
        )
        .await;
    }

    let published = published.lock().unwrap();
    assert_eq!(published.len(), 2, "{published:?}");
    assert_eq!(published[0].0, json!({}));
    assert_eq!(published[0].1.as_deref(), Some("lastSeenChanged"));
}

#[tokio::test]
async fn last_seen_from_non_emitted_frame_is_debounced() {
    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "advanced": {"last_seen": "ISO_8601"},
        "devices": {IEEE: {"friendly_name": "lamp", "debounce": 1}},
    }));

    for _ in 0..2 {
        harness
            .event_bus
            .lock()
            .unwrap()
            .emit_last_seen_changed(json!({
                "device": {"ieeeAddr": IEEE, "name": "lamp"},
                "reason": "messageNonEmitted",
            }));
    }
    tokio::time::sleep(Duration::from_millis(1300)).await;

    let recorded = harness.recorded.lock().unwrap();
    assert_eq!(recorded.len(), 1, "{recorded:?}");
    assert_eq!(recorded[0].1.as_deref(), Some("publishDebounce"));
}

#[tokio::test]
async fn last_seen_from_non_emitted_frame_is_throttled() {
    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "advanced": {"last_seen": "ISO_8601"},
        "devices": {IEEE: {"friendly_name": "lamp", "throttle": 1}},
    }));

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_last_seen_changed(json!({
            "device": {"ieeeAddr": IEEE, "name": "lamp"},
            "reason": "messageNonEmitted",
        }));
    wait_for_publishes(&harness.recorded, 1).await;
    assert_eq!(
        harness.recorded.lock().unwrap()[0].1.as_deref(),
        Some("publishThrottle")
    );

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_last_seen_changed(json!({
            "device": {"ieeeAddr": IEEE, "name": "lamp"},
            "reason": "messageNonEmitted",
        }));
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(harness.recorded.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn elapsed_is_added_from_the_second_message_onwards() {
    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "advanced": {"last_seen": "ISO_8601", "elapsed": true},
    }));
    register_on_off_converter(&harness.zigbee);

    for _ in 0..2 {
        harness
            .event_bus
            .lock()
            .unwrap()
            .emit_device_message(message("genOnOff", json!({"onOff": 1})));
        tokio::time::sleep(Duration::from_millis(60)).await;
    }
    wait_for_publishes(&harness.recorded, 2).await;

    let recorded = harness.recorded.lock().unwrap();
    assert_eq!(recorded[0].0, json!({"state": "ON"}));
    assert_eq!(recorded[1].0["state"], "ON");
    let elapsed = recorded[1].0["elapsed"]
        .as_u64()
        .expect("elapsed on the second message");
    assert!(elapsed >= 40, "elapsed {elapsed} too small");
}

#[tokio::test]
async fn debounce_merges_payloads_and_updates_the_state_cache() {
    use z2m_rs::model::entity::Entity;

    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "advanced": {"last_seen": "ISO_8601"},
        "devices": {IEEE: {"friendly_name": "lamp", "debounce": 1}},
    }));
    let device = harness.zigbee.resolve_device(IEEE).unwrap();

    harness
        .receive
        .publish_debounce(device.clone(), json!({"a": 1}), 1, None)
        .await;
    harness
        .receive
        .publish_debounce(device.clone(), json!({"b": 2}), 1, None)
        .await;

    // State cache is updated right away so a cached republish cannot send the
    // old value (upstream receive.ts:88-90).
    assert_eq!(
        harness.state.lock().unwrap().get(&device.entity_id()),
        json!({"a": 1, "b": 2})
    );

    tokio::time::sleep(Duration::from_millis(1300)).await;
    let recorded = harness.recorded.lock().unwrap();
    assert_eq!(recorded.len(), 1, "{recorded:?}");
    assert_eq!(recorded[0].0, json!({"a": 1, "b": 2}));
    assert_eq!(recorded[0].1.as_deref(), Some("publishDebounce"));
}

fn message_at(cluster: &str, data: Value, endpoint: u8) -> Value {
    json!({
        "type": "attributeReport",
        "device": {"ID": IEEE, "ieeeAddr": IEEE, "name": IEEE},
        "endpoint": {"ID": endpoint},
        "cluster": cluster,
        "data": data,
        "linkquality": 100,
        "meta": {"zclTransactionSequenceNumber": 1},
    })
}

/// Cases 2-3: converters receive the message endpoint and can act per endpoint.
#[tokio::test]
async fn endpoint_specific_converter_actions() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    harness
        .zigbee
        .resolve_device(IEEE)
        .unwrap()
        .zh
        .create_endpoint(2)
        .unwrap();
    let mut definition = Definition::new("TEST", "Acme", "Test device")
        .with_zigbee_model("TEST")
        .with_exposes(vec![Expose::enumeration(
            "action",
            zigbee_converters::access::STATE,
            &["single_left", "single_right"],
        )]);
    definition.from_zigbee.push(FromZigbeeConverter {
        cluster: "genOnOff".to_string(),
        types: vec!["attributeReport".to_string()],
        convert: Arc::new(|_definition, data, _publish, _options, _meta| {
            Box::pin(async move {
                let action = if data.endpoint.ID == 1 {
                    "single_left"
                } else {
                    "single_right"
                };
                Ok(Some(json!({"action": action})))
            })
        }),
    });
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

    for endpoint in [1, 2] {
        harness
            .event_bus
            .lock()
            .unwrap()
            .emit_device_message(message_at("genOnOff", json!({"onOff": 1}), endpoint));
    }
    wait_for_publishes(&harness.recorded, 2).await;

    let recorded = harness.recorded.lock().unwrap();
    let actions: Vec<Value> = recorded
        .iter()
        .map(|(payload, _)| payload["action"].clone())
        .collect();
    assert!(actions.contains(&json!("single_left")), "{recorded:?}");
    assert!(actions.contains(&json!("single_right")), "{recorded:?}");
}

/// Case 8: a message that converts to no payload still goes through the
/// debounce instead of being published immediately.
#[tokio::test(start_paused = true)]
async fn empty_payload_does_not_bypass_the_debounce() {
    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "advanced": {"last_seen": "ISO_8601"},
        "devices": {IEEE: {"friendly_name": "lamp", "debounce": 1}},
    }));
    let mut definition = Definition::new("TEST", "Acme", "Test device").with_zigbee_model("TEST");
    definition.from_zigbee.push(FromZigbeeConverter {
        cluster: "genOnOff".to_string(),
        types: vec!["attributeReport".to_string()],
        convert: Arc::new(|_definition, _data, _publish, _options, _meta| {
            Box::pin(async move { Ok(None) })
        }),
    });
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

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_device_message(message("genOnOff", json!({"onOff": 1})));
    tokio::time::advance(Duration::from_millis(50)).await;
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    assert!(
        harness.recorded.lock().unwrap().is_empty(),
        "empty payload bypassed the debounce"
    );

    tokio::time::advance(Duration::from_secs(1)).await;
    for _ in 0..30 {
        tokio::task::yield_now().await;
    }
    let recorded = harness.recorded.lock().unwrap();
    assert!(!recorded.is_empty(), "debounced empty payload was dropped");
    assert_eq!(recorded[0].0, json!({}));
    assert_eq!(recorded[0].1.as_deref(), Some("publishDebounce"));
}

/// Case 12: `device_options` debounce applies to devices without their own
/// option.
#[tokio::test(start_paused = true)]
async fn device_options_debounce_is_applied() {
    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "device_options": {"debounce": 1},
    }));
    let mut definition = Definition::new("TEST", "Acme", "Test device").with_zigbee_model("TEST");
    definition.from_zigbee.push(FromZigbeeConverter {
        cluster: "genOnOff".to_string(),
        types: vec!["attributeReport".to_string()],
        convert: Arc::new(|_definition, data, _publish, _options, _meta| {
            let on = data.data.get("onOff").and_then(Value::as_u64);
            Box::pin(async move {
                Ok(match on {
                    Some(1) => Some(json!({"state": "ON"})),
                    Some(0) => Some(json!({"state": "OFF"})),
                    _ => None,
                })
            })
        }),
    });
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

    for on in [1, 0] {
        harness
            .event_bus
            .lock()
            .unwrap()
            .emit_device_message(message("genOnOff", json!({"onOff": on})));
        for _ in 0..20 {
            tokio::task::yield_now().await;
        }
    }
    assert!(harness.recorded.lock().unwrap().is_empty());
    tokio::time::advance(Duration::from_secs(1)).await;
    for _ in 0..30 {
        tokio::task::yield_now().await;
    }
    let recorded = harness.recorded.lock().unwrap();
    assert_eq!(recorded.len(), 1, "{recorded:?}");
    assert_eq!(recorded[0].0, json!({"state": "OFF"}));
}

/// Case 15: throttled devices only publish at the configured rate.
#[tokio::test]
async fn device_options_throttle_is_applied() {
    let _guard = serial();
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "devices": {IEEE: {"friendly_name": "lamp"}},
        "device_options": {"throttle": 1},
    }));
    register_on_off_converter(&harness.zigbee);
    harness
        .zigbee
        .resolve_device(IEEE)
        .unwrap()
        .resolve_definition(true);

    for _ in 0..2 {
        harness
            .event_bus
            .lock()
            .unwrap()
            .emit_device_message(message("genOnOff", json!({"onOff": 1})));
        for _ in 0..20 {
            tokio::task::yield_now().await;
        }
    }
    assert_eq!(harness.recorded.lock().unwrap().len(), 1);

    tokio::time::sleep(Duration::from_millis(1_100)).await;
    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_device_message(message("genOnOff", json!({"onOff": 1})));
    wait_for_publishes(&harness.recorded, 2).await;
}

/// Cases 22-23: two matching converters produce one merged publish.
#[tokio::test]
async fn multiple_converters_publish_once() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    let mut definition = Definition::new("TEST", "Acme", "Test device")
        .with_zigbee_model("TEST")
        .with_exposes(vec![
            Expose::binary("state", zigbee_converters::access::ALL),
            Expose::numeric("brightness", zigbee_converters::access::ALL),
        ]);
    definition.from_zigbee.push(FromZigbeeConverter {
        cluster: "genOnOff".to_string(),
        types: vec!["attributeReport".to_string()],
        convert: Arc::new(|_definition, _data, _publish, _options, _meta| {
            Box::pin(async move { Ok(Some(json!({"state": "ON"}))) })
        }),
    });
    definition.from_zigbee.push(FromZigbeeConverter {
        cluster: "genOnOff".to_string(),
        types: vec!["attributeReport".to_string()],
        convert: Arc::new(|_definition, _data, _publish, _options, _meta| {
            Box::pin(async move { Ok(Some(json!({"brightness": 100}))) })
        }),
    });
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

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_device_message(message("genOnOff", json!({"onOff": 1})));
    wait_for_publishes(&harness.recorded, 1).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let recorded = harness.recorded.lock().unwrap();
    assert_eq!(
        recorded.len(),
        1,
        "two converters must merge into one publish"
    );
    assert_eq!(recorded[0].0, json!({"state": "ON", "brightness": 100}));
}

/// Case 28: messages from the coordinator are ignored.
#[tokio::test]
async fn coordinator_messages_are_ignored() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));

    harness.event_bus.lock().unwrap().emit_device_message(json!({
        "type": "attributeReport",
        "device": {"ID": "0x00124b0024c16b95", "ieeeAddr": "0x00124b0024c16b95", "name": "Coordinator"},
        "endpoint": {"ID": 1},
        "cluster": "genOnOff",
        "data": {"onOff": 1},
        "linkquality": 100,
        "meta": {"zclTransactionSequenceNumber": 1},
    }));
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(harness.recorded.lock().unwrap().is_empty());
}

/// Case 30: command messages that no converter matches publish nothing.
#[tokio::test]
async fn unmatched_command_messages_publish_nothing() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    register_on_off_converter(&harness.zigbee);
    harness
        .zigbee
        .resolve_device(IEEE)
        .unwrap()
        .resolve_definition(true);

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_device_message(json!({
            "type": "commandOn",
            "device": {"ID": IEEE, "ieeeAddr": IEEE, "name": IEEE},
            "endpoint": {"ID": 1},
            "cluster": "genOnOff",
            "data": {"onOff": 1},
            "linkquality": 100,
            "meta": {"zclTransactionSequenceNumber": 1},
        }));
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(harness.recorded.lock().unwrap().is_empty());
}

/// Case 34: a converter that announces changed exposes emits the
/// devices/exposes-changed events.
#[tokio::test]
async fn converter_exposes_change_emits_events() {
    let _guard = serial();
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    let fired = Arc::new(Mutex::new(0usize));
    let sink = fired.clone();
    harness
        .event_bus
        .lock()
        .unwrap()
        .on_exposes_changed(77, Arc::new(move |_| *sink.lock().unwrap() += 1));

    let mut definition = Definition::new("TEST", "Acme", "Test device").with_zigbee_model("TEST");
    definition.from_zigbee.push(FromZigbeeConverter {
        cluster: "genOnOff".to_string(),
        types: vec!["attributeReport".to_string()],
        convert: Arc::new(|_definition, _data, _publish, _options, meta| {
            Box::pin(async move {
                (meta.device_exposes_changed)();
                Ok(Some(json!({"state": "ON"})))
            })
        }),
    });
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

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_device_message(message("genOnOff", json!({"onOff": 1})));
    wait_for_publishes(&harness.recorded, 1).await;
    assert_eq!(*fired.lock().unwrap(), 1, "exposes changed event missing");
}
