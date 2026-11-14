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
    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_mqtt_message(json!({
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

    let fan = config(
        &harness,
        &format!("homeassistant/fan/{IEEE}/fan_f1/config"),
    )
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
    assert_eq!(switch["value_template"], "{% if \"state_l2\" in value_json %}{{ value_json[\"state_l2\"] }}{% endif %}");
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

    harness.event_bus.lock().unwrap().emit_publish_entity_state(json!({
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
