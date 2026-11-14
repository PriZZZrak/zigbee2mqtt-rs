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
        mqtt: Arc::new(tokio::sync::Mutex::new(z2m_rs::mqtt::Mqtt::new(event_bus.clone()))),
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
