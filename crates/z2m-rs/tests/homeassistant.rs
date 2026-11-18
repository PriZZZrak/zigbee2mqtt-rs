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
    published: Arc<Mutex<Vec<Value>>>,
    database_path: std::path::PathBuf,
    state_path: std::path::PathBuf,
}

fn temp(name: &str) -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir().join(format!("z2m-ha-{name}-{}-{id}", std::process::id()))
}

fn harness() -> Harness {
    harness_with(|_| {})
}
/// Same as [`harness`] but callable when a local variable shadows it.
fn fresh_harness() -> Harness {
    harness_with(|_| {})
}

fn harness_with(mutate: impl FnOnce(&mut Definition)) -> Harness {
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
    let zh_device = ZhDevice::create(
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
    zh_device.create_endpoint(1).unwrap();

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
    let _group = controller.create_group(1).unwrap();

    let settings = Arc::new(Mutex::new(Settings::from_config(json!({
        "mqtt": {"server": "localhost"},
        "homeassistant": {"enabled": true},
        "availability": {"enabled": true},
        "devices": {IEEE: {"friendly_name": "lamp"}},
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
    let device = zigbee.resolve_device(IEEE).unwrap();
    let state_path = temp("state.json");
    let state = Arc::new(Mutex::new(State::with_file(
        settings.clone(),
        event_bus.clone(),
        logger.clone(),
        state_path.clone(),
    )));
    let mqtt = Arc::new(tokio::sync::Mutex::new(Mqtt::new(event_bus.clone())));
    let published = Arc::new(Mutex::new(Vec::new()));
    let sink = published.clone();
    let publish: PublishEntityState = Arc::new(move |_entity, payload, _reason| {
        let sink = sink.clone();
        Box::pin(async move {
            sink.lock().unwrap().push(payload);
        })
    });

    let mut definition = Definition::new("TEST", "Acme", "Test device").with_zigbee_model("TEST");
    definition.exposes = zigbee_converters::Exposes::Static(vec![
        Expose::numeric("temperature", access::STATE),
        Expose::enumeration("mode", access::SET | access::STATE, &["auto", "manual"]),
        Expose::binary("contact", access::STATE),
        Expose::binary("power", access::SET | access::STATE),
        Expose {
            homeassistant: json!({
                "type": "sensor",
                "entityCategory": "diagnostic",
                "deviceClass": "voltage",
                "enabledByDefault": false,
                "icon": "mdi:flash",
            }),
            ..Expose::numeric("voltage", access::STATE)
        },
        Expose {
            homeassistant: json!({"name": null}),
            ..Expose::binary("tamper", access::STATE)
        },
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
    mutate(&mut definition);
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

async fn config(harness: &Harness, topic: &str) -> Option<Value> {
    for _ in 0..100 {
        if let Some(payload) = harness
            .mqtt
            .lock()
            .await
            .attempted
            .iter()
            .find(|(published, payload, _)| published == topic && !payload.is_empty())
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
    let availability = numeric["availability"].as_array().unwrap();
    assert_eq!(availability.len(), 2);
    assert_eq!(
        availability[0]["value_template"], "{{ value_json.state }}",
        "json availability payload template (case 103)"
    );

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
    assert!(siren["command_template"]
        .as_str()
        .unwrap()
        .contains("warning"));

    let text = config(&harness, &format!("homeassistant/text/{IEEE}/note/config"))
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
    let bridge = bridge.unwrap();
    assert_eq!(
        bridge["device"]["identifiers"][0],
        "zigbee2mqtt_bridge_0x00124b0024c16b95"
    );
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

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_publish_entity_state(json!({
            "entity": {"name": "lamp", "ieeeAddr": IEEE, "ID": IEEE},
            "message": {"action": "single"},
        }));
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mqtt = harness.mqtt.lock().await;
    let trigger = mqtt
        .attempted
        .iter()
        .find(|(topic, _, _)| {
            topic == &format!("homeassistant/device_automation/{IEEE}/action_single/config")
        })
        .expect("trigger discovery");
    let payload: Value = serde_json::from_str(&trigger.1).unwrap();
    assert_eq!(payload["automation_type"], "trigger");
    assert_eq!(payload["subtype"], "single");
    assert_eq!(payload["topic"], "zigbee2mqtt/lamp/action");
    assert!(mqtt
        .attempted
        .iter()
        .any(|(topic, payload, _)| topic == "zigbee2mqtt/lamp/action" && payload == "single"));
    drop(mqtt);
}

#[tokio::test]
async fn experimental_event_entities_prepare_wildcard_action_types() {
    let _guard = serial();
    let harness = harness();
    harness.context.settings.lock().unwrap().set(
        &["homeassistant", "experimental_event_entities"],
        json!(true),
    );
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
    assert!(event["value_template"]
        .as_str()
        .unwrap()
        .contains("patterns"));
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

    // Cases 74-75: only existing properties are copied, missing ones are not
    // invented (hue -> h and saturation -> s are independent).
    let mut missing_saturation = json!({"color": {"hue": 100}});
    ha.adjust_message_before_publish(device.as_ref(), &mut missing_saturation);
    assert_eq!(missing_saturation["color"]["h"], 100);
    assert!(missing_saturation["color"].get("s").is_none());
    assert!(missing_saturation["color"].get("saturation").is_none());

    let mut missing_color = json!({"state": "ON"});
    ha.adjust_message_before_publish(device.as_ref(), &mut missing_color);
    assert!(missing_color.get("color").is_none());
}

#[tokio::test]
async fn applies_expose_metadata_and_null_name() {
    let _guard = serial();
    let harness = harness();
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    // Upstream `Should apply expose-level Home Assistant discovery metadata`.
    let voltage = config(
        &harness,
        &format!("homeassistant/sensor/{IEEE}/voltage/config"),
    )
    .await
    .expect("voltage config");
    assert_eq!(voltage["device_class"], "voltage");
    assert_eq!(voltage["enabled_by_default"], false);
    assert_eq!(voltage["entity_category"], "diagnostic");
    assert_eq!(voltage["icon"], "mdi:flash");
    assert!(voltage.get("type").is_none());

    // Upstream `Should set discovery name to null when expose specifies
    // homeassistant name null`.
    let tamper = config(
        &harness,
        &format!("homeassistant/binary_sensor/{IEEE}/tamper/config"),
    )
    .await
    .expect("tamper config");
    assert!(tamper.as_object().unwrap().contains_key("name"));
    assert!(tamper["name"].is_null());
}

#[tokio::test]
async fn applies_device_option_overrides_and_null_filtering() {
    let _guard = serial();
    let harness = harness();
    {
        let mut settings = harness.context.settings.lock().unwrap();
        settings.set(
            &["devices", IEEE, "homeassistant"],
            json!({
                "temperature": null,
                "voltage": {"object_id": "custom_voltage", "type": "sensor"},
            }),
        );
    }
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    // A null per-object override removes that entity entirely.
    assert!(config(
        &harness,
        &format!("homeassistant/sensor/{IEEE}/temperature/config")
    )
    .await
    .is_none());

    // object_id override changes the discovery topic.
    let custom = config(
        &harness,
        &format!("homeassistant/sensor/{IEEE}/custom_voltage/config"),
    )
    .await
    .expect("overridden object id");
    // Upstream prefixes the payload object_id with the device name
    // (`homeassistant.ts:1836-1841`); the topic uses the overridden id.
    assert_eq!(custom["object_id"], "lamp_custom_voltage");
}

#[tokio::test]
async fn device_option_homeassistant_false_disables_discovery() {
    let _guard = serial();
    let harness = harness();
    harness
        .context
        .settings
        .lock()
        .unwrap()
        .set(&["devices", IEEE, "homeassistant"], json!(false));
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let mqtt = harness.mqtt.lock().await;
    assert!(
        !mqtt
            .attempted
            .iter()
            .any(|(topic, _, _)| topic.starts_with(&format!("homeassistant/sensor/{IEEE}/"))),
        "discovery ran despite homeassistant: false"
    );
}

#[tokio::test]
async fn discovers_last_seen_when_continuous_format_is_enabled() {
    let _guard = serial();
    let harness = harness();
    harness
        .context
        .settings
        .lock()
        .unwrap()
        .set(&["advanced", "last_seen"], json!("ISO_8601"));
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let payload = config(
        &harness,
        &format!("homeassistant/sensor/{IEEE}/last_seen/config"),
    )
    .await
    .expect("last_seen config");
    assert_eq!(payload["device_class"], "timestamp");
    assert_eq!(payload["enabled_by_default"], false);
    assert_eq!(payload["icon"], "mdi:clock");
}

#[tokio::test]
async fn discovers_ota_update_entity_when_definition_has_ota() {
    let _guard = serial();
    let harness = harness_with(|definition| definition.ota = Some(json!({})));
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let payload = config(
        &harness,
        &format!("homeassistant/update/{IEEE}/update/config"),
    )
    .await
    .expect("update config");
    assert_eq!(payload["device_class"], "firmware");
    assert_eq!(payload["entity_category"], "config");
    assert_eq!(
        payload["payload_install"],
        json!(format!("{{\"id\": \"{IEEE}\"}}"))
    );
}

#[tokio::test]
async fn clears_outdated_discovery_config_from_retained_messages() {
    let _guard = serial();
    let harness = harness();
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();
    harness.mqtt.lock().await.attempted.clear();

    let topic = "homeassistant/sensor/0xdeadbeefdeadbeef/temperature/config";
    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": topic,
        "message": json!({
            "state_topic": "zigbee2mqtt/removed",
            "availability": [{"topic": "zigbee2mqtt/bridge/state"}],
        })
        .to_string(),
    }));
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mqtt = harness.mqtt.lock().await;
    let publish = mqtt
        .attempted
        .iter()
        .find(|(published, _, _)| published == topic)
        .expect("stale config cleared");
    assert_eq!(publish.1, "");
    assert!(publish.2.retain);
    assert_eq!(publish.2.qos, 1);
}

#[tokio::test]
async fn keeps_discovery_topics_when_rename_is_not_a_homeassistant_rename() {
    let _guard = serial();
    let harness = harness();
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();
    harness.mqtt.lock().await.attempted.clear();

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_entity_renamed(json!({
            "entity": {"ID": IEEE, "ieeeAddr": IEEE, "name": "lamp"},
            "from": "lamp",
            "to": "lamp2",
            "homeAssisantRename": false,
        }));
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mqtt = harness.mqtt.lock().await;
    assert!(
        !mqtt
            .attempted
            .iter()
            .any(|(_, payload, options)| payload.is_empty() && options.retain),
        "discovery was cleared without homeAssisantRename"
    );
}

#[tokio::test]
async fn discovers_endpoint_entities_with_suffixed_topics() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        if let zigbee_converters::Exposes::Static(exposes) = &mut definition.exposes {
            exposes.push(Expose {
                r#type: "light".to_string(),
                name: Some("light".to_string()),
                endpoint: Some("l1".to_string()),
                access: access::ALL,
                features: vec![
                    Expose::binary("state", access::ALL),
                    Expose::numeric("brightness", access::ALL),
                ],
                ..Default::default()
            });
            exposes.push(Expose {
                r#type: "cover".to_string(),
                name: Some("cover".to_string()),
                endpoint: Some("left".to_string()),
                access: access::ALL,
                features: vec![
                    Expose::binary("state", access::ALL),
                    Expose::numeric("position", access::ALL),
                ],
                ..Default::default()
            });
            exposes.push(Expose {
                r#type: "cover".to_string(),
                name: Some("cover".to_string()),
                endpoint: Some("right".to_string()),
                access: access::ALL,
                features: vec![
                    Expose::binary("state", access::ALL),
                    Expose::numeric("position", access::ALL),
                ],
                ..Default::default()
            });
            exposes.push(Expose {
                r#type: "fan".to_string(),
                name: Some("fan".to_string()),
                endpoint: Some("f1".to_string()),
                access: access::ALL,
                features: vec![
                    Expose::binary("state", access::ALL),
                    Expose::numeric("speed", access::ALL),
                ],
                ..Default::default()
            });
            exposes.push(Expose {
                r#type: "switch".to_string(),
                name: Some("switch".to_string()),
                endpoint: Some("2".to_string()),
                access: access::ALL,
                features: vec![Expose {
                    property: Some("state_l2".to_string()),
                    ..Expose::binary("state", access::ALL)
                }],
                ..Default::default()
            });
        }
    });
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    // Upstream appends the endpoint name to object_id, name and topics.
    let light = config(
        &harness,
        &format!("homeassistant/light/{IEEE}/light_l1/config"),
    )
    .await
    .expect("endpoint light");
    assert_eq!(light["name"], "L1");
    assert_eq!(light["state_topic"], "zigbee2mqtt/lamp/l1");
    assert_eq!(light["command_topic"], "zigbee2mqtt/lamp/l1/set");

    let cover = config(
        &harness,
        &format!("homeassistant/cover/{IEEE}/cover_left/config"),
    )
    .await
    .expect("endpoint cover");
    assert_eq!(cover["state_topic"], "zigbee2mqtt/lamp/left");
    assert_eq!(cover["command_topic"], "zigbee2mqtt/lamp/left/set");

    let right_cover = config(
        &harness,
        &format!("homeassistant/cover/{IEEE}/cover_right/config"),
    )
    .await
    .expect("right endpoint cover");
    assert_eq!(right_cover["state_topic"], "zigbee2mqtt/lamp/right");
    assert_eq!(right_cover["command_topic"], "zigbee2mqtt/lamp/right/set");

    let fan = config(&harness, &format!("homeassistant/fan/{IEEE}/fan_f1/config"))
        .await
        .expect("endpoint fan");
    assert_eq!(fan["name"], "F1");
    assert_eq!(fan["command_topic"], "zigbee2mqtt/lamp/f1/set/state");

    let switch = config(
        &harness,
        &format!("homeassistant/switch/{IEEE}/switch_2/config"),
    )
    .await
    .expect("endpoint switch");
    assert_eq!(switch["command_topic"], "zigbee2mqtt/lamp/2/set");
    assert_eq!(
        switch["value_template"],
        "{% if \"state_l2\" in value_json %}{{ value_json[\"state_l2\"] }}{% endif %}"
    );
}

#[tokio::test]
async fn endpoint_state_publish_is_split_per_endpoint_topic() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        if let zigbee_converters::Exposes::Static(exposes) = &mut definition.exposes {
            exposes.push(Expose {
                r#type: "light".to_string(),
                name: Some("light".to_string()),
                endpoint: Some("l1".to_string()),
                access: access::ALL,
                features: vec![
                    Expose::binary("state", access::ALL),
                    Expose::numeric("brightness", access::ALL),
                ],
                ..Default::default()
            });
        }
    });
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();
    harness.mqtt.lock().await.attempted.clear();

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_publish_entity_state(json!({
            "entity": {"name": "lamp", "ieeeAddr": IEEE, "ID": IEEE},
            "message": {"state": "ON", "state_l1": "ON", "brightness_l1": 42},
        }));
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mqtt = harness.mqtt.lock().await;
    let published = mqtt
        .attempted
        .iter()
        .find(|(topic, _, _)| topic == "zigbee2mqtt/lamp/l1")
        .expect("endpoint state published");
    let payload: Value = serde_json::from_str(&published.1).unwrap();
    assert_eq!(payload, json!({"state": "ON", "brightness": 42}));
}

#[tokio::test]
async fn legacy_action_sensor_discovers_and_clears_action() {
    let _guard = serial();
    let harness = harness();
    harness
        .context
        .settings
        .lock()
        .unwrap()
        .set(&["homeassistant", "legacy_action_sensor"], json!(true));
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    // Upstream `Legacy action sensor`: the action enum is discovered as a sensor
    // instead of an event entity.
    let sensor = config(
        &harness,
        &format!("homeassistant/sensor/{IEEE}/action/config"),
    )
    .await
    .expect("legacy action sensor");
    assert_eq!(
        sensor["value_template"],
        "{% if \"action\" in value_json %}{{ value_json[\"action\"] }}{% endif %}"
    );
    assert!(config(
        &harness,
        &format!("homeassistant/event/{IEEE}/action/config")
    )
    .await
    .is_none());

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_publish_entity_state(json!({
            "entity": {"name": "lamp", "ieeeAddr": IEEE, "ID": IEEE},
            "message": {"action": "single"},
        }));
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(
        harness
            .published
            .lock()
            .unwrap()
            .iter()
            .any(|payload| payload == &json!({"action": ""})),
        "legacy action was not cleared: {:?}",
        harness.published.lock().unwrap()
    );
}

#[tokio::test]
async fn light_type_select_is_not_republished_as_endpoint_state() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        if let zigbee_converters::Exposes::Static(exposes) = &mut definition.exposes {
            exposes.push(Expose {
                r#type: "light".to_string(),
                name: Some("light".to_string()),
                endpoint: Some("l1".to_string()),
                access: access::ALL,
                features: vec![
                    Expose::binary("state", access::ALL),
                    Expose::numeric("brightness", access::ALL),
                ],
                ..Default::default()
            });
            exposes.push(Expose {
                r#type: "enum".to_string(),
                name: Some("light_type".to_string()),
                property: Some("light_type".to_string()),
                access: access::SET | access::STATE,
                values: vec!["light".to_string(), "switch".to_string()],
                ..Default::default()
            });
        }
    });
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    // The `light_type` select is discovered on the select platform.
    assert!(config(
        &harness,
        &format!("homeassistant/select/{IEEE}/light_type/config")
    )
    .await
    .is_some());
    harness.mqtt.lock().await.attempted.clear();

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_publish_entity_state(json!({
            "entity": {"name": "lamp", "ieeeAddr": IEEE, "ID": IEEE},
            "message": {"light_type": "switch", "state_l1": "ON"},
        }));
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mqtt = harness.mqtt.lock().await;
    assert!(
        mqtt.attempted
            .iter()
            .any(|(topic, payload, _)| topic == "zigbee2mqtt/lamp/l1"
                && payload == r#"{"state":"ON"}"#),
        "endpoint state missing: {:?}",
        mqtt.attempted
    );
    assert!(
        !mqtt
            .attempted
            .iter()
            .any(|(topic, _, _)| topic == "zigbee2mqtt/lamp/type"),
        "light_type select was republished as endpoint state"
    );
}

#[tokio::test]
async fn scenes_are_discovered_and_rediscovered_on_change() {
    let _guard = serial();
    let harness = harness();
    {
        let device = harness.context.zigbee.resolve_device(IEEE).unwrap();
        let endpoint = device.zh.endpoints()[0].clone();
        *endpoint.meta.lock().unwrap() = json!({"scenes": {"1_0": {"name": "Relax"}}});
    }
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let scene = config(
        &harness,
        &format!("homeassistant/scene/{IEEE}/scene_1/config"),
    )
    .await
    .expect("scene config");
    assert_eq!(scene["payload_on"], json!("{ \"scene_recall\": 1 }"));
    assert_eq!(scene["name"], "Relax");

    harness.mqtt.lock().await.attempted.clear();
    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_scenes_changed(json!({
            "entity": {"name": "lamp", "ieeeAddr": IEEE, "ID": IEEE},
        }));
    // The handler clears the scene topics, waits 2s (upstream) and rediscovers.
    tokio::time::sleep(Duration::from_millis(2600)).await;

    let mqtt = harness.mqtt.lock().await;
    assert!(
        mqtt.attempted.iter().any(|(topic, payload, options)| topic
            == &format!("homeassistant/scene/{IEEE}/scene_1/config")
            && payload.is_empty()
            && options.retain),
        "scene topic was not cleared: {:?}",
        mqtt.attempted
    );
    assert!(
        mqtt.attempted.iter().any(|(topic, payload, _)| topic
            == &format!("homeassistant/scene/{IEEE}/scene_1/config")
            && !payload.is_empty()),
        "scene was not rediscovered"
    );
}

#[tokio::test]
async fn exposes_changed_clears_removed_and_rediscovers_returning_entities() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        definition.exposes = zigbee_converters::Exposes::Dynamic(Arc::new(|_device, options| {
            let mut exposes = vec![Expose::numeric("temperature", access::STATE)];
            if options
                .get("expose_humidity")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                exposes.push(Expose::numeric("humidity", access::STATE));
            }
            exposes
        }));
    });
    {
        let mut settings = harness.context.settings.lock().unwrap();
        settings.set(&["devices", IEEE, "expose_humidity"], json!(true));
    }
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    assert!(config(
        &harness,
        &format!("homeassistant/sensor/{IEEE}/humidity/config")
    )
    .await
    .is_some());

    // Upstream `Should remove discovery entries for removed exposes when device
    // options change` + `exposes_changed` triggering rediscovery.
    {
        let mut settings = harness.context.settings.lock().unwrap();
        settings.set(&["devices", IEEE, "expose_humidity"], json!(false));
    }
    harness.mqtt.lock().await.attempted.clear();
    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_exposes_changed(json!({
            "device": {"ieeeAddr": IEEE, "friendly_name": "lamp"},
        }));
    tokio::time::sleep(Duration::from_millis(300)).await;

    {
        let mqtt = harness.mqtt.lock().await;
        let cleared = mqtt
            .attempted
            .iter()
            .find(|(topic, _, _)| topic == &format!("homeassistant/sensor/{IEEE}/humidity/config"))
            .expect("removed expose cleared");
        assert_eq!(cleared.1, "");
        assert!(cleared.2.retain);
    }

    // Upstream `Should rediscover an entity whose expose comes back after it was
    // removed`.
    {
        let mut settings = harness.context.settings.lock().unwrap();
        settings.set(&["devices", IEEE, "expose_humidity"], json!(true));
    }
    harness.mqtt.lock().await.attempted.clear();
    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_exposes_changed(json!({
            "device": {"ieeeAddr": IEEE, "friendly_name": "lamp"},
        }));
    tokio::time::sleep(Duration::from_millis(300)).await;

    let humidity = config(
        &harness,
        &format!("homeassistant/sensor/{IEEE}/humidity/config"),
    )
    .await
    .expect("humidity rediscovered");
    assert_eq!(
        humidity["value_template"],
        "{% if \"humidity\" in value_json %}{{ value_json[\"humidity\"] }}{% endif %}"
    );
}

#[tokio::test]
async fn group_members_changed_rediscovers_the_group() {
    let _guard = serial();
    let harness = harness();
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();
    assert!(
        !harness
            .mqtt
            .lock()
            .await
            .attempted
            .iter()
            .any(|(topic, _, _)| topic.starts_with("homeassistant/light/")
                && !topic.contains(IEEE)),
        "empty group must not be discovered"
    );

    let device = harness.context.zigbee.resolve_device(IEEE).unwrap();
    let group = harness.context.zigbee.group_by_id(1).unwrap();
    group.zh.add_member(&device.zh.endpoints()[0]);
    harness.mqtt.lock().await.attempted.clear();

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_group_members_changed(json!({"group": {"ID": 1, "name": "kitchen"}}));
    tokio::time::sleep(Duration::from_millis(300)).await;

    let mqtt = harness.mqtt.lock().await;
    let light = mqtt
        .attempted
        .iter()
        .find(|(topic, _, _)| {
            topic.starts_with("homeassistant/light/")
                && topic.ends_with("/light/config")
                && !topic.contains(IEEE)
        })
        .expect("group light discovered");
    let payload: Value = serde_json::from_str(&light.1).unwrap();
    let members = payload["group"].as_array().expect("group unique ids");
    assert!(!members.is_empty());
    assert_eq!(payload["device"]["name"], "kitchen");
}

#[tokio::test]
async fn entity_options_changed_republishes_discovery() {
    let _guard = serial();
    let harness = harness();
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();
    harness.mqtt.lock().await.attempted.clear();

    harness
        .context
        .settings
        .lock()
        .unwrap()
        .set(&["devices", IEEE, "friendly_name"], json!("lamp2"));
    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_entity_options_changed(json!({
            "entity": {"name": "lamp2", "ieeeAddr": IEEE, "ID": IEEE},
        }));
    tokio::time::sleep(Duration::from_millis(300)).await;

    let payload = config(
        &harness,
        &format!("homeassistant/sensor/{IEEE}/temperature/config"),
    )
    .await
    .expect("rediscovered after options change");
    assert_eq!(payload["device"]["name"], "lamp2");
}

#[tokio::test]
async fn group_state_publish_does_not_republish_endpoint_states() {
    let _guard = serial();
    let harness = harness();
    let device = harness.context.zigbee.resolve_device(IEEE).unwrap();
    harness
        .context
        .zigbee
        .group_by_id(1)
        .unwrap()
        .zh
        .add_member(&device.zh.endpoints()[0]);
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();
    harness.mqtt.lock().await.attempted.clear();

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_publish_entity_state(json!({
            "entity": {"name": "kitchen", "ID": 1},
            "message": {"state_l1": "ON"},
        }));
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mqtt = harness.mqtt.lock().await;
    assert!(
        !mqtt
            .attempted
            .iter()
            .any(|(topic, _, _)| topic.contains("kitchen/")),
        "group publish was split per endpoint: {:?}",
        mqtt.attempted
    );
}

#[tokio::test]
async fn disabled_device_is_discovered_as_offline() {
    let _guard = serial();
    let harness = harness();
    harness
        .context
        .settings
        .lock()
        .unwrap()
        .set(&["devices", IEEE, "disabled"], json!(true));
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let payload = config(
        &harness,
        &format!("homeassistant/sensor/{IEEE}/temperature/config"),
    )
    .await
    .expect("temperature config");
    let availability = payload["availability"].as_array().expect("availability");
    // A disabled device disables availability for the entity, so only the
    // bridge state topic remains - with the offline template.
    assert_eq!(availability.len(), 1, "{availability:?}");
    for entry in availability {
        assert_eq!(entry["value_template"], "{{ \"offline\" }}");
    }
    // Availability is disabled for disabled devices, so no availability_mode.
    assert!(payload["availability_mode"].is_null());
}

#[tokio::test]
async fn configuration_url_points_at_the_frontend() {
    let _guard = serial();
    let harness = harness();
    harness
        .context
        .settings
        .lock()
        .unwrap()
        .set(&["frontend", "url"], json!("http://example.com"));
    assert_eq!(
        harness.context.settings.lock().unwrap().get()["frontend"]["url"],
        "http://example.com"
    );
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let payload = config(
        &harness,
        &format!("homeassistant/sensor/{IEEE}/temperature/config"),
    )
    .await
    .expect("temperature config");
    assert_eq!(
        payload["device"]["configuration_url"],
        format!("http://example.com/#/device/0/{IEEE}/info")
    );
}

#[tokio::test]
async fn start_rejects_attribute_output_and_conflicting_discovery_topic() {
    let _guard = serial();
    let attribute = harness();
    attribute
        .context
        .settings
        .lock()
        .unwrap()
        .set(&["advanced", "output"], json!("attribute"));
    let error = HomeAssistant::new(attribute.context.clone())
        .start()
        .await
        .unwrap_err();
    assert_eq!(
        error,
        "Home Assistant integration requires 'output: json' under 'advanced'"
    );

    let conflicting = harness();
    conflicting
        .context
        .settings
        .lock()
        .unwrap()
        .set(&["homeassistant", "discovery_topic"], json!("zigbee2mqtt"));
    let error = HomeAssistant::new(conflicting.context.clone())
        .start()
        .await
        .unwrap_err();
    assert_eq!(
        error,
        "'homeassistant.discovery_topic' cannot not be equal to the 'mqtt.base_topic' (got 'zigbee2mqtt')"
    );
}

#[tokio::test]
async fn starting_without_cache_state_warns() {
    let _guard = serial();
    let harness = harness();
    harness
        .context
        .settings
        .lock()
        .unwrap()
        .set(&["advanced", "cache_state"], json!(false));
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    assert!(harness
        .context
        .logger
        .lock()
        .unwrap()
        .winston_logs
        .iter()
        .any(|(_, line)| line
            .contains("In order for Home Assistant integration to work properly, set `cache_state: true` under `advanced`")));
}

#[tokio::test(start_paused = true)]
async fn status_online_republishes_bridge_state_and_cached_states() {
    let _guard = serial();
    let harness = harness();
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();
    harness.mqtt.lock().await.attempted.clear();

    {
        let device = harness.context.zigbee.resolve_device(IEEE).unwrap();
        harness
            .context
            .state
            .lock()
            .unwrap()
            .set(device.as_ref(), json!({"state": "ON"}), None);
    }
    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": "homeassistant/status",
        "message": "online",
    }));
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    tokio::time::advance(Duration::from_secs(31)).await;
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }

    let mqtt = harness.mqtt.lock().await;
    let state = mqtt
        .attempted
        .iter()
        .find(|(topic, _, _)| topic == "zigbee2mqtt/bridge/state")
        .expect("bridge state republished");
    assert_eq!(state.1, r#"{"state":"online"}"#);
    assert!(state.2.retain);
    assert_eq!(state.2.qos, 1);
    drop(mqtt);
    assert!(harness
        .published
        .lock()
        .unwrap()
        .iter()
        .any(|payload| payload == &json!({"state": "ON"})));
}

#[tokio::test(start_paused = true)]
async fn status_offline_or_other_topics_do_not_republish() {
    let _guard = serial();
    let harness = harness();
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();
    harness.mqtt.lock().await.attempted.clear();

    for message in [
        json!({"topic": "homeassistant/status", "message": "offline"}),
        json!({"topic": "homeassistant/other", "message": "online"}),
    ] {
        harness.event_bus.lock().unwrap().emit_mqtt_message(message);
    }
    tokio::time::advance(Duration::from_secs(31)).await;
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }

    assert!(!harness
        .mqtt
        .lock()
        .await
        .attempted
        .iter()
        .any(|(topic, _, _)| topic == "zigbee2mqtt/bridge/state"));
}

#[tokio::test]
async fn discovery_types_are_unique_and_templates_guarded() {
    let _guard = serial();
    let harness = harness();
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let mqtt = harness.mqtt.lock().await;
    let mut seen = std::collections::HashSet::new();
    for (topic, payload, _) in &mqtt.attempted {
        let Some(rest) = topic.strip_prefix("homeassistant/") else {
            continue;
        };
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() != 4 || parts[3] != "config" || parts[1] == "0x00124b0024c16b95" {
            continue;
        }
        let key = (parts[0].to_string(), parts[2].to_string());
        assert!(
            seen.insert(key.clone()),
            "duplicate discovery entry {key:?}"
        );

        let payload: Value = serde_json::from_str(payload).unwrap();
        if let Some(template) = payload["value_template"].as_str() {
            if template.contains("value_json") {
                assert!(
                    template.starts_with("{% if \""),
                    "unguarded template {template}"
                );
            }
        }
    }
    assert!(!seen.is_empty());
}

#[tokio::test]
async fn configuration_switches_are_config_entities() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        if let zigbee_converters::Exposes::Static(exposes) = &mut definition.exposes {
            exposes.push(Expose {
                r#type: "switch".to_string(),
                name: Some("switch".to_string()),
                endpoint: Some("1".to_string()),
                access: access::ALL,
                features: vec![Expose {
                    property: Some("window_detection".to_string()),
                    ..Expose::binary("state", access::ALL)
                }],
                label: Some("Window detection".to_string()),
                ..Default::default()
            });
        }
    });
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let payload = config(
        &harness,
        &format!("homeassistant/switch/{IEEE}/window_detection/config"),
    )
    .await
    .expect("configuration switch");
    assert_eq!(payload["entity_category"], "config");
    assert_eq!(payload["icon"], "mdi:window-open-variant");
    assert_eq!(payload["name"], "Window detection");
}

#[tokio::test(start_paused = true)]
async fn homeassistant_rename_clears_and_republishes_discovery() {
    let _guard = serial();
    let harness = harness();
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();
    harness.mqtt.lock().await.attempted.clear();

    harness
        .context
        .settings
        .lock()
        .unwrap()
        .set(&["devices", IEEE, "friendly_name"], json!("lamp2"));
    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_entity_renamed(json!({
            "entity": {"ID": IEEE, "ieeeAddr": IEEE, "name": "lamp2"},
            "from": "lamp",
            "to": "lamp2",
            "homeAssisantRename": true,
        }));
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    tokio::time::advance(Duration::from_secs(3)).await;
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }

    {
        let mqtt = harness.mqtt.lock().await;
        let cleared = mqtt
            .attempted
            .iter()
            .find(|(topic, payload, options)| {
                topic == &format!("homeassistant/sensor/{IEEE}/temperature/config")
                    && payload.is_empty()
                    && options.retain
            })
            .expect("old discovery topic cleared");
        assert_eq!(cleared.1, "");
    }

    let republished = config(
        &harness,
        &format!("homeassistant/sensor/{IEEE}/temperature/config"),
    )
    .await
    .expect("rediscovered after rename");
    assert_eq!(republished["device"]["name"], "lamp2");
    assert_eq!(republished["object_id"], "lamp2_temperature");
}

#[tokio::test(start_paused = true)]
async fn group_rename_clears_and_republishes_discovery() {
    let _guard = serial();
    let harness = harness();
    {
        let device = harness.context.zigbee.resolve_device(IEEE).unwrap();
        harness
            .context
            .zigbee
            .group_by_id(1)
            .unwrap()
            .zh
            .add_member(&device.zh.endpoints()[0]);
    }
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();
    assert!(config(
        &harness,
        &format!("homeassistant/light/{IEEE}/light/config")
    )
    .await
    .is_some());
    harness.mqtt.lock().await.attempted.clear();

    harness
        .context
        .settings
        .lock()
        .unwrap()
        .set(&["groups", "1", "friendly_name"], json!("kitchen2"));
    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_entity_renamed(json!({
            "entity": {"ID": 1, "name": "kitchen2"},
            "from": "kitchen",
            "to": "kitchen2",
            "homeAssisantRename": true,
        }));
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    tokio::time::advance(Duration::from_secs(3)).await;
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }

    let mqtt = harness.mqtt.lock().await;
    assert!(
        mqtt.attempted
            .iter()
            .any(|(_, payload, options)| payload.is_empty() && options.retain),
        "group discovery topics were not cleared"
    );
    let rediscovered = mqtt
        .attempted
        .iter()
        .filter(|(topic, payload, _)| {
            topic.starts_with("homeassistant/light/") && !payload.is_empty()
        })
        .filter_map(|(_, payload, _)| serde_json::from_str::<Value>(payload).ok())
        .any(|payload| payload["device"]["name"] == "kitchen2");
    assert!(rediscovered, "group was not rediscovered with the new name");
}

#[tokio::test]
async fn precision_options_do_not_change_discovery_payloads() {
    let _guard = serial();
    let harness = harness();
    {
        let mut settings = harness.context.settings.lock().unwrap();
        settings.set(&["devices", IEEE, "temperature_precision"], json!(1));
        settings.set(&["devices", IEEE, "humidity_precision"], json!(0));
        settings.set(&["devices", IEEE, "pressure_precision"], json!(2));
    }
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let payload = config(
        &harness,
        &format!("homeassistant/sensor/{IEEE}/temperature/config"),
    )
    .await
    .expect("temperature config");
    assert_eq!(
        payload["value_template"],
        "{% if \"temperature\" in value_json %}{{ value_json[\"temperature\"] }}{% endif %}"
    );
    let template = payload["value_template"].as_str().unwrap();
    assert!(!template.contains("round"));
}

#[tokio::test]
async fn availability_entries_cover_bridge_and_entity_topics() {
    let _guard = serial();
    let harness = harness();
    {
        let device = harness.context.zigbee.resolve_device(IEEE).unwrap();
        harness
            .context
            .zigbee
            .group_by_id(1)
            .unwrap()
            .zh
            .add_member(&device.zh.endpoints()[0]);
    }
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let temperature = config(
        &harness,
        &format!("homeassistant/sensor/{IEEE}/temperature/config"),
    )
    .await
    .expect("temperature config");
    let availability = temperature["availability"].as_array().unwrap();
    assert_eq!(availability.len(), 2, "{availability:?}");
    assert_eq!(availability[0]["topic"], "zigbee2mqtt/bridge/state");
    assert_eq!(availability[1]["topic"], "zigbee2mqtt/lamp/availability");
    for entry in availability {
        assert_eq!(entry["value_template"], "{{ value_json.state }}");
    }
    assert_eq!(temperature["availability_mode"], "all");

    // Group discovery uses the group state topic for its availability entry.
    let mqtt = harness.mqtt.lock().await;
    let group_light = mqtt
        .attempted
        .iter()
        .filter(|(topic, payload, _)| {
            topic.starts_with("homeassistant/light/") && !payload.is_empty()
        })
        .filter_map(|(_, payload, _)| serde_json::from_str::<Value>(payload).ok())
        .find(|payload| payload["device"]["name"] == "kitchen")
        .expect("group light discovery");
    let availability = group_light["availability"].as_array().unwrap();
    assert_eq!(availability[1]["topic"], "zigbee2mqtt/kitchen/availability");
    assert_eq!(group_light["availability_mode"], "all");
}

/// Case 55: an override that panics is caught, logged and the original payload
/// is still published.
#[tokio::test]
async fn discovery_override_panic_is_caught() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        definition.override_ha_discovery_payload =
            Some(Arc::new(|payload: &mut Value, _options: &Value| {
                if payload["mode_command_topic"]
                    .as_str()
                    .is_some_and(|topic| topic.ends_with("/system_mode"))
                {
                    panic!("Failed");
                }
            }));
    });
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let climate = config(
        &harness,
        &format!("homeassistant/climate/{IEEE}/climate/config"),
    )
    .await
    .expect("climate discovery still published");
    assert_eq!(
        climate["mode_command_topic"],
        "zigbee2mqtt/lamp/set/system_mode"
    );
    ha.stop().await.unwrap();
}

/// Case 56: overrides receive the device options.
#[tokio::test]
async fn discovery_override_receives_device_options() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        definition.override_ha_discovery_payload =
            Some(Arc::new(|payload: &mut Value, options: &Value| {
                if payload["mode_command_topic"].is_string() {
                    payload["discovery_option_marker"] = options
                        .get("discovery_option_marker")
                        .cloned()
                        .unwrap_or(Value::Null);
                }
            }));
    });
    harness.context.settings.lock().unwrap().configuration["devices"][IEEE]
        ["discovery_option_marker"] = json!("passed");
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let climate = config(
        &harness,
        &format!("homeassistant/climate/{IEEE}/climate/config"),
    )
    .await
    .expect("climate discovery");
    assert_eq!(climate["discovery_option_marker"], "passed");
    ha.stop().await.unwrap();
}

/// Case 68: `homeassistant.discovery_topic` is used for the config topic.
#[tokio::test]
async fn custom_discovery_topic_is_used() {
    let _guard = serial();
    let harness = harness();
    harness.context.settings.lock().unwrap().configuration["homeassistant"]["discovery_topic"] =
        json!("my_custom_discovery_topic");
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let config_payload = config(
        &harness,
        &format!("my_custom_discovery_topic/sensor/{IEEE}/temperature/config"),
    )
    .await
    .expect("custom discovery topic");
    assert_eq!(
        config_payload["state_topic"], "zigbee2mqtt/lamp",
        "state topics stay on the base topic"
    );
    ha.stop().await.unwrap();
}

/// Case 76: an already discovered device is not discovered again on a message.
#[tokio::test]
async fn device_message_does_not_rediscover() {
    let _guard = serial();
    let harness = harness();
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();
    assert!(config(
        &harness,
        &format!("homeassistant/sensor/{IEEE}/temperature/config")
    )
    .await
    .is_some());
    harness.mqtt.lock().await.attempted.clear();

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_device_message(json!({
            "device": {"ieeeAddr": IEEE, "name": "lamp"},
            "endpoint": {"ID": 1},
            "cluster": "msTemperatureMeasurement",
            "type": "attributeReport",
            "data": {"measuredValue": -85},
            "linkquality": 10,
        }));
    tokio::time::sleep(Duration::from_millis(150)).await;

    assert!(
        !harness
            .mqtt
            .lock()
            .await
            .attempted
            .iter()
            .any(|(topic, _, _)| topic.ends_with("/config")),
        "rediscovered an already discovered device"
    );
    ha.stop().await.unwrap();
}

/// Case 77: a message from an undiscovered device triggers discovery.
#[tokio::test]
async fn device_message_discovers_when_not_discovered() {
    let _guard = serial();
    let harness = harness();
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();
    ha.test_mark_undiscovered(IEEE);
    harness.mqtt.lock().await.attempted.clear();

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_device_message(json!({
            "device": {"ieeeAddr": IEEE, "name": "lamp"},
            "endpoint": {"ID": 1},
            "cluster": "msTemperatureMeasurement",
            "type": "attributeReport",
            "data": {"measuredValue": -85},
            "linkquality": 10,
        }));
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert!(
        harness.mqtt.lock().await.attempted.iter().any(
            |(topic, _, _)| topic == &format!("homeassistant/sensor/{IEEE}/temperature/config")
        ),
        "discovery did not run for an undiscovered device"
    );
    ha.stop().await.unwrap();
}

/// Case 78: a device leaving does not publish discovery configs.
#[tokio::test]
async fn device_leave_does_not_publish_discovery() {
    let _guard = serial();
    let harness = harness();
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();
    harness.mqtt.lock().await.attempted.clear();

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_device_leave(json!({"ieeeAddr": IEEE}));
    tokio::time::sleep(Duration::from_millis(150)).await;

    assert!(
        !harness
            .mqtt
            .lock()
            .await
            .attempted
            .iter()
            .any(|(topic, _, _)| topic.ends_with("/config")),
        "unexpected discovery on leave"
    );
    ha.stop().await.unwrap();
}

/// Case 109: receiving back the bridge entity config does not clear it.
#[tokio::test]
async fn bridge_entity_config_echo_is_not_cleared() {
    let _guard = serial();
    let harness = harness();
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();
    let topic = "homeassistant/binary_sensor/0x00124b0024c16b95/connection_state/config";
    let payload = config(&harness, topic).await.expect("bridge entity");
    harness.mqtt.lock().await.attempted.clear();

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": topic,
        "message": payload.to_string(),
    }));
    tokio::time::sleep(Duration::from_millis(150)).await;

    assert!(
        !harness
            .mqtt
            .lock()
            .await
            .attempted
            .iter()
            .any(|(published, body, _)| published == topic && body.is_empty()),
        "bridge entity config was cleared after receiving it back"
    );
    ha.stop().await.unwrap();
}

/// Case 3: a numeric expose with SET access still yields a guarded number
/// entity template.
#[tokio::test]
async fn number_entity_template_is_guarded() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        definition.exposes = zigbee_converters::Exposes::Static(vec![Expose {
            value_min: Some(0.0),
            value_max: Some(255.0),
            ..Expose::numeric("effect_speed", access::ALL)
        }]);
    });
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let number = config(
        &harness,
        &format!("homeassistant/number/{IEEE}/effect_speed/config"),
    )
    .await
    .expect("number entity");
    assert_eq!(
        number["value_template"],
        "{% if \"effect_speed\" in value_json %}{{ value_json[\"effect_speed\"] }}{% endif %}"
    );
    ha.stop().await.unwrap();
}

/// Case 4: discovery templates must not use truthiness that drops 0, false or
/// null values, and must not use `default(x, True)` outside truncating
/// templates.
#[tokio::test]
async fn discovery_templates_are_falsy_safe() {
    let _guard = serial();
    let harness = harness();
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mqtt = harness.mqtt.lock().await;
    let mut offending = Vec::new();
    for (topic, payload, _) in &mqtt.attempted {
        if !topic.ends_with("/config") || payload.is_empty() {
            continue;
        }
        if payload.contains("default(")
            && payload.contains(", True)")
            && !payload.contains("truncate(")
        {
            offending.push(topic.clone());
        }
        let value: Value = serde_json::from_str(payload).unwrap_or(Value::Null);
        fn scan(value: &Value, topic: &str, offending: &mut Vec<String>) {
            match value {
                Value::String(text) => {
                    let mut rest = text.as_str();
                    while let Some(start) = rest.find("{% if value_json[\"") {
                        let tail = &rest[start..];
                        if !tail.contains(" in value_json") {
                            offending.push(format!("{topic}: {tail}"));
                        }
                        rest = &tail[3..];
                    }
                }
                Value::Array(items) => {
                    for item in items {
                        scan(item, topic, offending);
                    }
                }
                Value::Object(map) => {
                    for item in map.values() {
                        scan(item, topic, offending);
                    }
                }
                _ => {}
            }
        }
        scan(&value, topic, &mut offending);
    }
    assert!(
        offending.is_empty(),
        "falsy-sensitive templates: {offending:?}"
    );
}

/// Case 81: when Home Assistant comes online, bridge state and discovery are
/// republished after the 30 s debounce.
#[tokio::test(start_paused = true)]
async fn homeassistant_online_republishes_state() {
    let _guard = serial();
    let harness = harness();
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    harness.mqtt.lock().await.attempted.clear();

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": "homeassistant/status",
        "message": "online",
    }));
    tokio::time::sleep(Duration::from_secs(31)).await;
    for _ in 0..50 {
        tokio::task::yield_now().await;
    }

    let mqtt = harness.mqtt.lock().await;
    assert!(
        mqtt.attempted.iter().any(
            |(topic, payload, options)| topic == "zigbee2mqtt/bridge/state"
                && payload == r#"{"state":"online"}"#
                && options.retain
        ),
        "bridge state not republished: {:?}",
        mqtt.attempted
            .iter()
            .map(|(topic, _, _)| topic.clone())
            .collect::<Vec<_>>()
    );
    drop(mqtt);
    ha.stop().await.unwrap();
}

/// Case 54: user homeassistant option overrides are applied after the
/// converter compatibility mapping.
#[tokio::test]
async fn user_configuration_wins_over_the_mapping() {
    let _guard = serial();
    let harness = harness();
    harness.context.settings.lock().unwrap().configuration["devices"][IEEE]["homeassistant"] =
        json!({"temperature": {"name": "Custom temperature"}});
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let temperature = config(
        &harness,
        &format!("homeassistant/sensor/{IEEE}/temperature/config"),
    )
    .await
    .expect("temperature discovery");
    assert_eq!(temperature["name"], "Custom temperature");
    ha.stop().await.unwrap();
}

/// Case 87: removing a group clears its discovery config.
#[tokio::test]
async fn group_removal_clears_discovery() {
    let _guard = serial();
    let harness = harness();
    let device = harness.context.zigbee.resolve_device(IEEE).unwrap();
    let group = harness.context.zigbee.resolve_group(1).unwrap();
    group.zh.add_member(&device.zh.endpoints()[0]);
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();
    let topic = {
        let mqtt = harness.mqtt.lock().await;
        mqtt.attempted
            .iter()
            .map(|(topic, _, _)| topic.clone())
            .find(|topic| {
                topic.starts_with("homeassistant/")
                    && topic.contains("_1/")
                    && topic.ends_with("/config")
            })
            .expect("group discovery topic")
    };
    harness.mqtt.lock().await.attempted.clear();

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_entity_removed(json!({"entity": {"ID": 1, "type": "group"}}));
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(
        harness
            .mqtt
            .lock()
            .await
            .attempted
            .iter()
            .any(|(published, payload, options)| published == &topic
                && payload.is_empty()
                && options.retain),
        "group discovery was not cleared"
    );
    ha.stop().await.unwrap();
}

fn climate_expose(endpoint: Option<&str>, features: Vec<Expose>) -> Expose {
    Expose {
        r#type: "climate".to_string(),
        name: Some("climate".to_string()),
        endpoint: endpoint.map(str::to_string),
        access: access::ALL,
        features,
        ..Default::default()
    }
}

fn demand_expose(access: u32, min: Option<f64>, max: Option<f64>) -> Expose {
    Expose {
        value_min: min,
        value_max: max,
        ..Expose::numeric("pi_heating_demand", access)
    }
}

/// Case 48: thermostat discovery payload.
#[tokio::test]
async fn thermostat_discovery_payload() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        definition.exposes = zigbee_converters::Exposes::Static(vec![climate_expose(
            None,
            vec![
                Expose::numeric("local_temperature", access::STATE),
                Expose {
                    value_min: Some(5.0),
                    value_max: Some(30.0),
                    value_step: Some(0.5),
                    ..Expose::numeric("occupied_heating_setpoint", access::ALL)
                },
                Expose::enumeration("system_mode", access::ALL, &["off", "heat", "sleep"]),
                Expose::enumeration("running_state", access::STATE, &["idle", "heat"]),
                Expose::enumeration("fan_mode", access::ALL, &["low", "auto"]),
            ],
        )]);
    });
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let climate = config(
        &harness,
        &format!("homeassistant/climate/{IEEE}/climate/config"),
    )
    .await
    .expect("climate");
    assert_eq!(climate["min_temp"], "5");
    assert_eq!(climate["max_temp"], "30");
    assert_eq!(climate["temp_step"], 0.5);
    assert_eq!(climate["modes"], json!(["off", "heat"]));
    assert_eq!(
        climate["temperature_command_topic"],
        "zigbee2mqtt/lamp/set/occupied_heating_setpoint"
    );
    assert_eq!(
        climate["temperature_state_template"],
        "{% if \"occupied_heating_setpoint\" in value_json %}{{ value_json[\"occupied_heating_setpoint\"] }}{% endif %}"
    );
    assert_eq!(climate["action_topic"], "zigbee2mqtt/lamp");
    assert!(climate["action_template"]
        .as_str()
        .unwrap_or_default()
        .contains("heat:heating"));
    assert_eq!(climate["current_temperature_topic"], "zigbee2mqtt/lamp");
    assert_eq!(climate["fan_modes"], json!(["low", "auto"]));
    ha.stop().await.unwrap();
}

/// Case 49: read-only PI heating demand becomes a diagnostic sensor.
#[tokio::test]
async fn pi_heating_demand_read_only_is_a_sensor() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        definition.exposes = zigbee_converters::Exposes::Static(vec![climate_expose(
            None,
            vec![
                Expose::numeric("local_temperature", access::STATE),
                Expose {
                    value_min: Some(5.0),
                    value_max: Some(30.0),
                    ..Expose::numeric("occupied_heating_setpoint", access::ALL)
                },
                demand_expose(access::STATE, None, None),
            ],
        )]);
    });
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let sensor = config(
        &harness,
        &format!("homeassistant/sensor/{IEEE}/pi_heating_demand/config"),
    )
    .await
    .expect("read-only demand sensor");
    assert_eq!(sensor["entity_category"], "diagnostic");
    assert!(config(
        &harness,
        &format!("homeassistant/number/{IEEE}/pi_heating_demand/config")
    )
    .await
    .is_none());
    ha.stop().await.unwrap();
}

/// Case 50: writable PI heating demand becomes a number entity.
#[tokio::test]
async fn pi_heating_demand_writable_is_a_number() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        definition.exposes = zigbee_converters::Exposes::Static(vec![climate_expose(
            None,
            vec![
                Expose::numeric("local_temperature", access::STATE),
                Expose {
                    value_min: Some(5.0),
                    value_max: Some(30.0),
                    ..Expose::numeric("occupied_heating_setpoint", access::ALL)
                },
                demand_expose(access::ALL, Some(0.0), Some(100.0)),
            ],
        )]);
    });
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let number = config(
        &harness,
        &format!("homeassistant/number/{IEEE}/pi_heating_demand/config"),
    )
    .await
    .expect("writable demand number");
    assert_eq!(number["min"], 0.0);
    assert_eq!(number["max"], 100.0);
    assert_eq!(
        number["command_topic"],
        "zigbee2mqtt/lamp/set/pi_heating_demand"
    );
    ha.stop().await.unwrap();
}

/// Case 58: a separate temperature sensor is discovered for the thermostat.
#[tokio::test]
async fn thermostat_gets_a_local_temperature_sensor() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        definition.exposes = zigbee_converters::Exposes::Static(vec![climate_expose(
            None,
            vec![
                Expose {
                    unit: Some("°C".to_string()),
                    ..Expose::numeric("local_temperature", access::STATE)
                },
                Expose {
                    value_min: Some(5.0),
                    value_max: Some(30.0),
                    ..Expose::numeric("occupied_heating_setpoint", access::ALL)
                },
            ],
        )]);
    });
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let sensor = config(
        &harness,
        &format!("homeassistant/sensor/{IEEE}/local_temperature/config"),
    )
    .await
    .expect("local temperature sensor");
    assert_eq!(sensor["device_class"], "temperature");
    assert_eq!(sensor["unit_of_measurement"], "°C");
    assert_eq!(
        sensor["value_template"],
        "{% if \"local_temperature\" in value_json %}{{ value_json[\"local_temperature\"] }}{% endif %}"
    );
    ha.stop().await.unwrap();
}

/// Case 60: cooling-only climate uses the cooling setpoint.
#[tokio::test]
async fn cooling_only_climate_setpoint() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        definition.exposes = zigbee_converters::Exposes::Static(vec![climate_expose(
            None,
            vec![
                Expose::numeric("local_temperature", access::STATE),
                Expose {
                    value_min: Some(16.0),
                    value_max: Some(32.0),
                    value_step: Some(0.5),
                    ..Expose::numeric("occupied_cooling_setpoint", access::ALL)
                },
                Expose::enumeration("system_mode", access::ALL, &["off", "cool"]),
            ],
        )]);
    });
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let climate = config(
        &harness,
        &format!("homeassistant/climate/{IEEE}/climate/config"),
    )
    .await
    .expect("cooling climate");
    assert_eq!(
        climate["temperature_command_topic"],
        "zigbee2mqtt/lamp/set/occupied_cooling_setpoint"
    );
    assert_eq!(climate["min_temp"], "16");
    assert_eq!(climate["max_temp"], "32");
    ha.stop().await.unwrap();
}

/// Case 61: emergency_heating maps to heat in the climate mode template.
#[tokio::test]
async fn emergency_heating_maps_to_heat() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        definition.exposes = zigbee_converters::Exposes::Static(vec![climate_expose(
            None,
            vec![
                Expose::numeric("local_temperature", access::STATE),
                Expose {
                    value_min: Some(5.0),
                    value_max: Some(30.0),
                    ..Expose::numeric("occupied_heating_setpoint", access::ALL)
                },
                Expose::enumeration(
                    "system_mode",
                    access::ALL,
                    &["off", "heat", "emergency_heating"],
                ),
            ],
        )]);
    });
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let climate = config(
        &harness,
        &format!("homeassistant/climate/{IEEE}/climate/config"),
    )
    .await
    .expect("climate");
    let modes = climate["modes"].as_array().unwrap();
    assert!(!modes.contains(&json!("emergency_heating")), "{modes:?}");
    assert_eq!(modes, &vec![json!("off"), json!("heat")]);
    assert!(climate["mode_state_template"]
        .as_str()
        .unwrap_or_default()
        .contains("heat if mode == emergency_heating else mode"));
    ha.stop().await.unwrap();
}

/// Case 59: multi-endpoint climate prefers the humidity of its own endpoint.
#[tokio::test]
async fn climate_prefers_same_endpoint_humidity() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        let climate = |endpoint: &str| {
            climate_expose(
                Some(endpoint),
                vec![
                    Expose::numeric("local_temperature", access::STATE),
                    Expose {
                        value_min: Some(5.0),
                        value_max: Some(30.0),
                        ..Expose::numeric("occupied_heating_setpoint", access::ALL)
                    },
                ],
            )
        };
        definition.exposes = zigbee_converters::Exposes::Static(vec![
            climate("l1"),
            climate("l2"),
            Expose {
                name: Some("humidity".to_string()),
                property: Some("humidity_l1".to_string()),
                endpoint: Some("l1".to_string()),
                ..Expose::numeric("humidity", access::STATE)
            },
            Expose::numeric("humidity", access::STATE),
        ]);
    });
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let l1 = config(
        &harness,
        &format!("homeassistant/climate/{IEEE}/climate_l1/config"),
    )
    .await
    .expect("climate l1");
    assert_eq!(
        l1["current_humidity_template"],
        "{% if \"humidity_l1\" in value_json %}{{ value_json[\"humidity_l1\"] }}{% endif %}"
    );
    let l2 = config(
        &harness,
        &format!("homeassistant/climate/{IEEE}/climate_l2/config"),
    )
    .await
    .expect("climate l2");
    assert_eq!(
        l2["current_humidity_template"],
        "{% if \"humidity\" in value_json %}{{ value_json[\"humidity\"] }}{% endif %}"
    );
    ha.stop().await.unwrap();
}

fn cover_expose(endpoint: Option<&str>, features: Vec<Expose>) -> Expose {
    Expose {
        r#type: "cover".to_string(),
        name: Some("cover".to_string()),
        endpoint: endpoint.map(str::to_string),
        access: access::ALL,
        features,
        ..Default::default()
    }
}

/// Case 62: cover discovery with position.
#[tokio::test]
async fn cover_position_discovery() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        definition.exposes = zigbee_converters::Exposes::Static(vec![cover_expose(
            None,
            vec![
                Expose::binary("state", access::ALL),
                Expose::numeric("position", access::ALL),
            ],
        )]);
    });
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let cover = config(
        &harness,
        &format!("homeassistant/cover/{IEEE}/cover/config"),
    )
    .await
    .expect("cover");
    assert_eq!(
        cover["position_template"],
        "{% if \"position\" in value_json %}{{ value_json[\"position\"] }}{% endif %}"
    );
    assert_eq!(cover["position_topic"], "zigbee2mqtt/lamp");
    assert_eq!(cover["set_position_topic"], "zigbee2mqtt/lamp/set");
    assert_eq!(cover["state_open"], "OPEN");
    assert_eq!(cover["state_closed"], "CLOSE");
    ha.stop().await.unwrap();
}

/// Case 64: motor_state covers derive direction-aware states.
#[tokio::test]
async fn cover_motor_state_direction_discovery() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        definition.exposes = zigbee_converters::Exposes::Static(vec![
            cover_expose(
                None,
                vec![
                    Expose::binary("state", access::ALL),
                    Expose::numeric("position", access::ALL),
                ],
            ),
            Expose::enumeration(
                "motor_state",
                access::STATE,
                &["opening", "closing", "stopped"],
            ),
        ]);
    });
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let cover = config(
        &harness,
        &format!("homeassistant/cover/{IEEE}/cover/config"),
    )
    .await
    .expect("cover");
    assert_eq!(cover["state_opening"], "opening");
    assert_eq!(cover["state_closing"], "closing");
    assert_eq!(cover["state_stopped"], "stopped");
    let template = cover["value_template"].as_str().unwrap_or_default();
    assert!(
        template.contains("motor == \"opening\" and position != 100"),
        "{template}"
    );
    assert!(
        template.contains("motor == \"closing\" and position != 0"),
        "{template}"
    );
    ha.stop().await.unwrap();
}

/// Case 65: covers without position keep motor_state and the state fallback.
#[tokio::test]
async fn cover_without_position_keeps_state_fallback() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        definition.exposes = zigbee_converters::Exposes::Static(vec![
            cover_expose(None, vec![Expose::binary("state", access::ALL)]),
            Expose::enumeration(
                "motor_state",
                access::STATE,
                &["opening", "closing", "stopped"],
            ),
        ]);
    });
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let cover = config(
        &harness,
        &format!("homeassistant/cover/{IEEE}/cover/config"),
    )
    .await
    .expect("cover");
    assert_eq!(cover["state_opening"], "opening");
    assert_eq!(cover["state_closing"], "closing");
    let template = cover["value_template"].as_str().unwrap_or_default();
    assert!(template.contains("set position = none"), "{template}");
    assert!(
        template.contains("state in [\"OPEN\", \"CLOSE\"]"),
        "{template}"
    );
    assert_eq!(cover["optimistic"], true);
    ha.stop().await.unwrap();
}

/// Cases 45 and 47: speed-controlled fans, including a non-first endpoint.
#[tokio::test]
async fn fan_speed_on_non_first_endpoint() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        let fan = |endpoint: Option<&str>| Expose {
            r#type: "fan".to_string(),
            name: Some("fan".to_string()),
            endpoint: endpoint.map(str::to_string),
            access: access::ALL,
            features: vec![
                Expose::binary("state", access::ALL),
                Expose::enumeration("mode", access::ALL, &["off", "low", "high", "auto"]),
            ],
            ..Default::default()
        };
        definition.exposes = zigbee_converters::Exposes::Static(vec![fan(None), fan(Some("l2"))]);
    });
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let fan = config(&harness, &format!("homeassistant/fan/{IEEE}/fan/config"))
        .await
        .expect("fan");
    assert_eq!(fan["speed_range_max"], 2);
    assert_eq!(fan["percentage_command_topic"], "zigbee2mqtt/lamp/set/mode");
    assert_eq!(fan["preset_modes"], json!(["auto"]));

    let second = config(&harness, &format!("homeassistant/fan/{IEEE}/fan_l2/config"))
        .await
        .expect("second endpoint fan");
    assert_eq!(second["command_topic"], "zigbee2mqtt/lamp/l2/set/state");
    assert_eq!(second["state_topic"], "zigbee2mqtt/lamp");
    ha.stop().await.unwrap();
}

/// Cases 52-53: analog input/output exposes.
#[tokio::test]
async fn analog_input_and_output_discovery() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        definition.exposes = zigbee_converters::Exposes::Static(vec![
            Expose {
                name: Some("analog_in_temperature_1".to_string()),
                property: Some("analog_in_temperature_1".to_string()),
                label: Some("my_sensor_name".to_string()),
                unit: Some("°C".to_string()),
                homeassistant: json!({"name": "my_sensor_name", "deviceClass": "temperature", "stateClass": "measurement"}),
                ..Expose::numeric("analog_in_temperature_1", access::STATE)
            },
            Expose {
                name: Some("analog_output_2".to_string()),
                property: Some("analog_output_2".to_string()),
                label: Some("my_number_name".to_string()),
                endpoint: None,
                ..Expose::numeric("analog_output_2", access::ALL)
            },
        ]);
    });
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let sensor = config(
        &harness,
        &format!("homeassistant/sensor/{IEEE}/analog_in_temperature_1/config"),
    )
    .await
    .expect("analog input");
    assert_eq!(sensor["name"], "my_sensor_name");
    assert_eq!(sensor["device_class"], "temperature");
    assert_eq!(sensor["unit_of_measurement"], "°C");
    assert_eq!(
        sensor["value_template"],
        "{% if \"analog_in_temperature_1\" in value_json %}{{ value_json[\"analog_in_temperature_1\"] }}{% endif %}"
    );

    let number = config(
        &harness,
        &format!("homeassistant/number/{IEEE}/analog_output_2/config"),
    )
    .await
    .expect("analog output");
    assert_eq!(number["name"], "my_number_name");
    assert_eq!(
        number["command_topic"],
        "zigbee2mqtt/lamp/set/analog_output_2"
    );
    ha.stop().await.unwrap();
}

/// Cases 66-67: infrared emitter and receiver entities.
#[tokio::test]
async fn infrared_emitter_and_receiver_discovery() {
    let _guard = serial();
    let harness = harness_with(|definition| {
        definition.exposes = zigbee_converters::Exposes::Static(vec![
            Expose {
                r#type: "text".to_string(),
                name: Some("emitter".to_string()),
                property: Some("emitter".to_string()),
                label: Some("Emitter".to_string()),
                access: access::SET,
                homeassistant: json!({"type": "infrared", "schema": "emitter", "valueTemplate": null}),
                ..Default::default()
            },
            Expose {
                r#type: "text".to_string(),
                name: Some("receiver".to_string()),
                property: Some("receiver".to_string()),
                label: Some("Receiver".to_string()),
                access: access::STATE,
                homeassistant: json!({"type": "infrared", "schema": "receiver", "valueTemplate": "{{ json_value.emitter }}"}),
                ..Default::default()
            },
        ]);
    });
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();

    let emitter = config(
        &harness,
        &format!("homeassistant/infrared/{IEEE}/emitter/config"),
    )
    .await
    .expect("emitter");
    assert_eq!(emitter["schema"], "emitter");
    assert_eq!(emitter["name"], "Emitter");
    assert!(emitter.get("value_template").is_none());

    let receiver = config(
        &harness,
        &format!("homeassistant/infrared/{IEEE}/receiver/config"),
    )
    .await
    .expect("receiver");
    assert_eq!(receiver["schema"], "receiver");
    assert_eq!(receiver["name"], "Receiver");
    assert_eq!(receiver["value_template"], "{{ json_value.emitter }}");
    ha.stop().await.unwrap();
}

/// Case 96: device_automation: null disables trigger discovery.
#[tokio::test]
async fn device_automation_can_be_disabled() {
    let _guard = serial();
    let harness = harness();
    harness.context.settings.lock().unwrap().configuration["device_options"] =
        json!({"homeassistant": {"device_automation": null}});
    let ha = HomeAssistant::new(harness.context.clone());
    ha.clone().start().await.unwrap();
    harness.mqtt.lock().await.attempted.clear();

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_publish_entity_state(json!({
            "entity": {"name": "lamp", "ieeeAddr": IEEE, "ID": IEEE},
            "message": {"action": "single"},
        }));
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(
        !harness
            .mqtt
            .lock()
            .await
            .attempted
            .iter()
            .any(|(topic, _, _)| topic.starts_with("homeassistant/device_automation/")),
        "device triggers were discovered despite device_automation: null"
    );
    ha.stop().await.unwrap();
}

/// Case 82: existing update versions are preserved.
#[tokio::test]
async fn update_versions_are_preserved() {
    let _guard = serial();
    let harness = harness_with(|definition| definition.ota = Some(json!({})));
    let ha = HomeAssistant::new(harness.context.clone());
    let device = harness.context.zigbee.resolve_device(IEEE).unwrap();

    let mut with_versions =
        json!({"update": {"installed_version": "1.0", "latest_version": "1.2"}});
    ha.adjust_message_before_publish(device.as_ref(), &mut with_versions);
    assert_eq!(with_versions["update"]["installed_version"], "1.0");
    assert_eq!(with_versions["update"]["latest_version"], "1.2");

    let mut without_versions = json!({"update": {}});
    ha.adjust_message_before_publish(device.as_ref(), &mut without_versions);
    assert_eq!(without_versions["update"]["installed_version"], -1);
    assert_eq!(without_versions["update"]["latest_version"], -1);
}
