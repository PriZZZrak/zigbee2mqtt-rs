// Original: .upstream/zigbee2mqtt/test/extensions/homeassistant.test.ts
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
use z2m_rs::extension::homeassistant::HomeAssistant;
use z2m_rs::model::entity::PublishEntityState;
use z2m_rs::mqtt::Mqtt;
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::zigbee::Zigbee;
use zigbee_converters::{access, Definition, Expose};

const IEEE: &str = "0x0017880104e45519";

struct Harness {
    context: ExtensionContext,
    mqtt: Arc<tokio::sync::Mutex<Mqtt>>,
    event_bus: Arc<Mutex<EventBus>>,
    database_path: std::path::PathBuf,
    state_path: std::path::PathBuf,
}

fn temp(name: &str) -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir().join(format!("z2m-ha-{name}-{}-{id}", std::process::id()))
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
        "homeassistant": {"enabled": true},
        "availability": {"enabled": true},
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
    let device = zigbee.resolve_device(IEEE).unwrap();
    let state_path = temp("state.json");
    let state = Arc::new(Mutex::new(State::with_file(
        settings.clone(),
        event_bus.clone(),
        logger.clone(),
        state_path.clone(),
    )));
    let mqtt = Arc::new(tokio::sync::Mutex::new(Mqtt::new(event_bus.clone())));
    let publish: PublishEntityState = Arc::new(|_e, _p, _r| Box::pin(async {}));

    let mut definition = Definition::new("TEST", "Acme", "Test device").with_zigbee_model("TEST");
    definition.exposes = zigbee_converters::Exposes::Static(vec![
        Expose::numeric("temperature", access::STATE),
        Expose::enumeration("mode", access::SET | access::STATE, &["auto", "manual"]),
        Expose::binary("contact", access::STATE),
        Expose::binary("power", access::SET | access::STATE),
        Expose::enumeration(
            "action",
            access::STATE,
            &["single", "recall_*", "button_1_press", "brightness_step"],
        ),
        Expose {
            r#type: "composite".to_string(),
            name: Some("warning".to_string()),
            access: access::SET,
            features: vec![
                Expose::enumeration("mode", access::SET, &["stop", "emergency", "fire"]),
                Expose::enumeration("level", access::SET, &["low", "high"]),
                Expose::numeric("duration", access::SET),
            ],
            ..Default::default()
        },
        Expose {
            r#type: "text".to_string(),
            property: Some("note".to_string()),
            access: access::SET | access::STATE,
            label: Some("Note".to_string()),
            ..Default::default()
        },
        Expose {
            r#type: "climate".to_string(),
            name: Some("climate".to_string()),
            access: access::ALL,
            features: vec![
                Expose::numeric("local_temperature", access::STATE),
                Expose {
                    value_min: Some(5.0),
                    value_max: Some(30.0),
                    value_step: Some(0.5),
                    ..Expose::numeric("occupied_heating_setpoint", access::SET | access::STATE)
                },
                Expose::enumeration(
                    "system_mode",
                    access::SET | access::STATE,
                    &["off", "heat", "sleep"],
                ),
                Expose::enumeration("running_state", access::STATE, &["idle", "heat"]),
                Expose::enumeration("fan_mode", access::SET | access::STATE, &["low", "auto"]),
                Expose {
                    value_min: Some(-5.0),
                    value_max: Some(5.0),
                    ..Expose::numeric("local_temperature_calibration", access::ALL)
                },
            ],
            ..Default::default()
        },
        Expose {
            r#type: "light".to_string(),
            name: Some("light".to_string()),
            access: access::ALL,
            features: vec![
                Expose::binary("state", access::ALL),
                Expose::numeric("brightness", access::ALL),
                Expose::numeric("color_temp", access::ALL),
            ],
            ..Default::default()
        },
        Expose {
            r#type: "cover".to_string(),
            name: Some("cover".to_string()),
            access: access::ALL,
            features: vec![
                Expose::binary("state", access::ALL),
                Expose::numeric("position", access::ALL),
            ],
            ..Default::default()
        },
        Expose {
            r#type: "lock".to_string(),
            name: Some("lock".to_string()),
            access: access::ALL,
            features: vec![Expose::binary("state", access::ALL)],
            ..Default::default()
        },
        Expose {
            r#type: "fan".to_string(),
            name: Some("fan".to_string()),
            access: access::ALL,
            features: vec![
                Expose::binary("state", access::ALL),
                Expose::enumeration("mode", access::ALL, &["off", "low", "high", "auto"]),
            ],
            ..Default::default()
        },
    ]);
    zigbee.converters().lock().unwrap().register(definition);
    device.resolve_definition(false);

    Harness {
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
            key: 30,
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

async fn config(harness: &Harness, topic: &str) -> Option<Value> {
    for _ in 0..100 {
        if let Some(payload) = harness
            .mqtt
            .lock()
            .await
            .attempted
            .iter()
            .find(|(published, _, _)| published == topic)
            .map(|(_, payload, _)| payload.clone())
        {
            return Some(serde_json::from_str(&payload).unwrap());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    None
}

#[tokio::test]
async fn discovers_numeric_binary_enum_and_switch_entities() {
    let _guard = serial();
    let harness = harness();
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let numeric = config(
        &harness,
        &format!("homeassistant/sensor/{IEEE}/temperature/config"),
    )
    .await;
    if numeric.is_none() {
        panic!(
            "topics: {:?}",
            harness
                .mqtt
                .lock()
                .await
                .attempted
                .iter()
                .map(|(t, _, _)| t.clone())
                .collect::<Vec<_>>()
        );
    }
    let numeric = numeric.unwrap();
    assert_eq!(
        numeric["device"]["identifiers"][0],
        format!("zigbee2mqtt_{IEEE}")
    );
    assert_eq!(numeric["state_topic"], format!("zigbee2mqtt/lamp"));
    assert_eq!(
        numeric["value_template"],
        "{% if \"temperature\" in value_json %}{{ value_json[\"temperature\"] }}{% endif %}"
    );
    assert_eq!(
        numeric["unique_id"],
        format!("{IEEE}_temperature_zigbee2mqtt")
    );
    assert_eq!(numeric["availability"].as_array().unwrap().len(), 2);

    let contact = config(
        &harness,
        &format!("homeassistant/binary_sensor/{IEEE}/contact/config"),
    )
    .await
    .expect("binary sensor");
    assert_eq!(contact["payload_on"], true);
    assert_eq!(contact["device"]["model_id"], "TEST");

    let select = config(
        &harness,
        &format!("homeassistant/select/{IEEE}/mode/config"),
    )
    .await
    .expect("enum select");
    assert_eq!(select["options"], json!(["auto", "manual"]));
    assert_eq!(
        select["command_topic"],
        format!("zigbee2mqtt/lamp/set/mode")
    );

    let switch = config(
        &harness,
        &format!("homeassistant/switch/{IEEE}/switch_power/config"),
    )
    .await
    .expect("binary switch");
    assert_eq!(switch["command_topic"], "zigbee2mqtt/lamp/set/power");

    let siren = config(
        &harness,
        &format!("homeassistant/siren/{IEEE}/siren/config"),
    )
    .await
    .expect("siren");
    assert_eq!(siren["available_tones"], json!(["emergency", "fire"]));
    assert_eq!(siren["support_volume_set"], true);
    assert_eq!(siren["support_duration"], true);
    assert!(siren["command_template"].as_str().unwrap().contains("warning"));

    let text = config(
        &harness,
        &format!("homeassistant/text/{IEEE}/note/config"),
    )
    .await
    .expect("text");
    assert_eq!(text["command_topic"], "zigbee2mqtt/lamp/set/note");
    assert_eq!(text["state_topic"], "zigbee2mqtt/lamp");

    let bridge = config(
        &harness,
        "homeassistant/binary_sensor/0x00124b0024c16b95/connection_state/config",
    )
    .await;
    if bridge.is_none() {
        panic!("topics: {:?}", harness.mqtt.lock().await.attempted.iter().map(|(t,_,_)| t.clone()).collect::<Vec<_>>());
    }
    let bridge = bridge.unwrap();
    assert_eq!(bridge["device"]["identifiers"][0], "zigbee2mqtt_bridge_0x00124b0024c16b95");
    assert_eq!(bridge["payload_on"], "online");

    let climate = config(
        &harness,
        &format!("homeassistant/climate/{IEEE}/climate/config"),
    )
    .await
    .expect("climate");
    assert_eq!(climate["min_temp"], "5");
    assert_eq!(climate["max_temp"], "30");
    assert_eq!(climate["modes"], json!(["off", "heat"]));
    assert_eq!(
        climate["temperature_command_topic"],
        "zigbee2mqtt/lamp/set/occupied_heating_setpoint"
    );
    assert_eq!(climate["fan_modes"], json!(["low", "auto"]));
    assert_eq!(climate["current_temperature_topic"], "zigbee2mqtt/lamp");
    assert_eq!(
        config(
            &harness,
            &format!("homeassistant/number/{IEEE}/local_temperature_calibration/config")
        )
        .await
        .expect("calibration")["device_class"],
        "temperature_delta"
    );

    let light = config(
        &harness,
        &format!("homeassistant/light/{IEEE}/light/config"),
    )
    .await
    .expect("light");
    assert_eq!(light["schema"], "json");
    assert_eq!(light["brightness"], true);
    assert_eq!(light["supported_color_modes"], json!(["color_temp"]));

    let cover = config(
        &harness,
        &format!("homeassistant/cover/{IEEE}/cover/config"),
    )
    .await
    .expect("cover");
    assert_eq!(cover["state_closed"], "CLOSE");
    assert_eq!(cover["position_topic"], "zigbee2mqtt/lamp");
    assert_eq!(cover["set_position_topic"], format!("zigbee2mqtt/lamp/set"));

    let lock = config(&harness, &format!("homeassistant/lock/{IEEE}/lock/config"))
        .await
        .expect("lock");
    assert_eq!(lock["state_locked"], true);

    let fan = config(&harness, &format!("homeassistant/fan/{IEEE}/fan/config"))
        .await
        .expect("fan");
    assert_eq!(fan["preset_modes"], json!(["auto"]));
    assert_eq!(fan["speed_range_max"], 2);

    let mqtt = harness.mqtt.lock().await;
    let entry = mqtt
        .attempted
        .iter()
        .find(|(topic, _, _)| topic.ends_with("/config"))
        .unwrap();
    assert!(entry.2.retain);
    assert_eq!(entry.2.qos, 1);
}

#[tokio::test]
async fn clears_discovery_on_entity_removed() {
    let _guard = serial();
    let harness = harness();
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_entity_removed(json!({"entity": {"ID": IEEE, "ieeeAddr": IEEE, "name": "lamp"}}));
    tokio::time::sleep(Duration::from_millis(200)).await;

    let cleared: Vec<String> = harness
        .mqtt
        .lock()
        .await
        .attempted
        .iter()
        .filter(|(topic, payload, options)| {
            topic.starts_with("homeassistant/") && payload.is_empty() && options.retain
        })
        .map(|(topic, _, _)| topic.clone())
        .collect();
    assert!(cleared.contains(&format!("homeassistant/sensor/{IEEE}/temperature/config")));
}

#[tokio::test]
async fn action_publishes_device_trigger_and_republication() {
    let _guard = serial();
    let harness = harness();
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();
    harness.mqtt.lock().await.attempted.clear();

    harness.event_bus.lock().unwrap().emit_publish_entity_state(json!({
        "entity": {"name": "lamp", "ieeeAddr": IEEE, "ID": IEEE},
        "message": {"action": "single"},
    }));
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mqtt = harness.mqtt.lock().await;
    let trigger = mqtt
        .attempted
        .iter()
        .find(|(topic, _, _)| topic == &format!("homeassistant/device_automation/{IEEE}/action_single/config"))
        .expect("trigger discovery");
    let payload: Value = serde_json::from_str(&trigger.1).unwrap();
    assert_eq!(payload["automation_type"], "trigger");
    assert_eq!(payload["subtype"], "single");
    assert_eq!(payload["topic"], "zigbee2mqtt/lamp/action");
    assert!(mqtt
        .attempted
        .iter()
        .any(|(topic, payload, _)| topic == "zigbee2mqtt/lamp/action" && payload == "single"));
}

#[tokio::test]
async fn experimental_event_entities_prepare_wildcard_action_types() {
    let _guard = serial();
    let harness = harness();
    harness
        .context
        .settings
        .lock()
        .unwrap()
        .set(&["homeassistant", "experimental_event_entities"], json!(true));
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let event = config(
        &harness,
        &format!("homeassistant/event/{IEEE}/action/config"),
    )
    .await
    .expect("event entity");
    assert_eq!(
        event["event_types"],
        json!(["single", "recall", "press", "brightness_step"])
    );
    assert!(event["value_template"].as_str().unwrap().contains("patterns"));
}

#[test]
fn adjust_message_copies_hue_and_saturation() {
    let _guard = serial();
    let harness = harness();
    let ha = HomeAssistant::new(harness.context.clone());
    let device = harness.context.zigbee.resolve_device(IEEE).unwrap();
    let mut message = json!({"color": {"hue": 100, "saturation": 50}});
    ha.adjust_message_before_publish(device.as_ref(), &mut message);
    assert_eq!(message["color"]["h"], 100);
    assert_eq!(message["color"]["s"], 50);
}
