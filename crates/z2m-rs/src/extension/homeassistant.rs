// Original: .upstream/zigbee2mqtt/lib/extension/homeassistant.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// First slice: discovery infrastructure, bridge entities (connection state /
// restart required / version), and numeric/binary/enum/switch expose mapping.
// Remaining expose types (light, climate, lock, cover, fan, text, composite,
// list, device_automation/event) and the remaining bridge buttons/sensors are
// being ported next; see docs/parity-matrix.md.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::{json, Map, Value};

use zigbee_converters::{access, Expose};

use crate::extension::extension::{Extension, ExtensionContext, ExtensionFuture};
use crate::model::entity::Entity;
use crate::mqtt::MqttPublishOptions;
use crate::util::stringify::stringify;
use crate::util::utils;

struct Discovered {
    messages: HashMap<String, (String, bool)>,
    triggers: HashSet<String>,
    discovered: bool,
}

#[derive(Clone)]
struct Entry {
    r#type: String,
    object_id: String,
    payload: Map<String, Value>,
}

pub struct HomeAssistant {
    context: ExtensionContext,
    discovery_topic: String,
    status_topic: String,
    legacy_action_sensor: bool,
    experimental_event_entities: bool,
    discovered: Mutex<HashMap<String, Arc<Mutex<Discovered>>>>,
    bridge_identifier: Mutex<String>,
    origin: Mutex<Value>,
    version: Mutex<String>,
}

fn lookups() -> &'static Value {
    static LOOKUPS: OnceLock<Value> = OnceLock::new();
    LOOKUPS.get_or_init(|| {
        serde_json::from_str(include_str!("homeassistant_lookups.json"))
            .expect("embedded homeassistant lookups are valid JSON")
    })
}

fn lookup(table: &str, key: &str) -> Option<&'static Map<String, Value>> {
    lookups()[table].get(key).and_then(Value::as_object)
}

fn merge_lookup(target: &mut Map<String, Value>, table: &str, key: &str) {
    if let Some(extra) = lookup(table, key) {
        for (name, value) in extra {
            target.insert(name.clone(), value.clone());
        }
    }
}

fn merge(target: &mut Map<String, Value>, extra: Map<String, Value>) {
    for (key, value) in extra {
        target.insert(key, value);
    }
}

fn clean_name(name: &str) -> String {
    name.strip_prefix("analog_in_")
        .or_else(|| name.strip_prefix("analog_out_"))
        .unwrap_or(name)
        .to_string()
}

fn state_value_template(property: &str) -> String {
    format!(
        "{{% if \"{property}\" in value_json %}}{{{{ value_json[\"{property}\"] }}}}{{% endif %}}"
    )
}

fn capitalize(value: &str) -> String {
    utils::capitalize(value)
}

impl HomeAssistant {
    pub fn new(context: ExtensionContext) -> Arc<Self> {
        let settings = context.settings_value();
        let discovery_topic = settings["homeassistant"]["discovery_topic"]
            .as_str()
            .unwrap_or("homeassistant")
            .to_string();
        Arc::new(Self {
            context,
            discovery_topic,
            status_topic: settings["homeassistant"]["status_topic"]
                .as_str()
                .unwrap_or("homeassistant/status")
                .to_string(),
            legacy_action_sensor: settings["homeassistant"]["legacy_action_sensor"]
                .as_bool()
                .unwrap_or(false),
            experimental_event_entities: settings["homeassistant"]["experimental_event_entities"]
                .as_bool()
                .unwrap_or(false),
            discovered: Mutex::new(HashMap::new()),
            bridge_identifier: Mutex::new(String::new()),
            origin: Mutex::new(Value::Null),
            version: Mutex::new("unknown".to_string()),
        })
    }

    fn get_discovered(&self, id: &str) -> Arc<Mutex<Discovered>> {
        let mut map = self.discovered.lock().unwrap();
        map.entry(id.to_string())
            .or_insert_with(|| {
                Arc::new(Mutex::new(Discovered {
                    messages: HashMap::new(),
                    triggers: HashSet::new(),
                    discovered: false,
                }))
            })
            .clone()
    }

    fn get_configs(&self, entity: &Arc<dyn Entity>) -> Vec<Entry> {
        let exposes: Vec<Expose> = if entity.is_device() {
            entity
                .zh_device()
                .and_then(|zh| self.context.zigbee.resolve_device(&zh.ieee_addr()))
                .map(|device| device.exposes().unwrap_or_default())
                .unwrap_or_default()
        } else {
            entity
                .members_devices()
                .iter()
                .flat_map(|member| {
                    member
                        .zh_device()
                        .and_then(|zh| self.context.zigbee.resolve_device(&zh.ieee_addr()))
                        .and_then(|device| device.exposes().ok())
                        .unwrap_or_default()
                })
                .collect()
        };

        let mut entries = Vec::new();
        for expose in exposes {
            match expose.r#type.as_str() {
                "switch" => {
                    let Some(state) = expose.features.iter().find(|feature| {
                        feature.r#type == "binary" && feature.name.as_deref() == Some("state")
                    }) else {
                        continue;
                    };
                    let property = state.property.clone().unwrap_or_default();
                    let mut payload = Map::new();
                    payload.insert("name".into(), Value::Null);
                    payload.insert("payload_off".into(), state.value_off.clone());
                    payload.insert("payload_on".into(), state.value_on.clone());
                    payload.insert(
                        "value_template".into(),
                        Value::String(state_value_template(&property)),
                    );
                    payload.insert("command_topic".into(), Value::Bool(true));
                    if [
                        "auto_lock",
                        "away_mode",
                        "comfort_smiley",
                        "enable_display",
                        "indicator",
                        "tilt_mode",
                        "valve_detection",
                        "window_detection",
                    ]
                    .contains(&property.as_str())
                    {
                        payload.insert(
                            "name".into(),
                            expose
                                .label
                                .clone()
                                .map(Value::String)
                                .unwrap_or(Value::Null),
                        );
                        payload.insert(
                            "command_topic_postfix".into(),
                            Value::String(property.clone()),
                        );
                        payload.insert("state_off".into(), state.value_off.clone());
                        payload.insert("state_on".into(), state.value_on.clone());
                        merge_lookup(&mut payload, "CONFIG_SWITCH_DISCOVERY_LOOKUP", &property);
                    }
                    entries.push(Entry {
                        r#type: "switch".into(),
                        object_id: "switch".into(),
                        payload,
                    });
                }
                "binary" => {
                    let property = expose.property.clone().unwrap_or_default();
                    let label = expose
                        .label
                        .clone()
                        .or_else(|| expose.name.clone())
                        .unwrap_or_else(|| property.clone());
                    if expose.access & access::SET != 0 {
                        let mut payload = Map::new();
                        payload.insert("name".into(), Value::String(label));
                        payload.insert(
                            "value_template".into(),
                            Value::String(state_value_template(&property)),
                        );
                        payload.insert(
                            "payload_on".into(),
                            Value::String(expose.value_on.to_string()),
                        );
                        payload.insert(
                            "payload_off".into(),
                            Value::String(expose.value_off.to_string()),
                        );
                        payload.insert("command_topic".into(), Value::Bool(true));
                        payload.insert(
                            "command_topic_postfix".into(),
                            Value::String(property.clone()),
                        );
                        merge_lookup(&mut payload, 
                                "BINARY_DISCOVERY_LOOKUP",
                                expose.name.as_deref().unwrap_or(""),
                            );
                        entries.push(Entry {
                            r#type: "switch".into(),
                            object_id: format!("switch_{property}"),
                            payload,
                        });
                    } else {
                        let mut payload = Map::new();
                        payload.insert("name".into(), Value::String(label));
                        payload.insert(
                            "value_template".into(),
                            Value::String(state_value_template(&property)),
                        );
                        payload.insert("payload_on".into(), expose.value_on.clone());
                        payload.insert("payload_off".into(), expose.value_off.clone());
                        merge_lookup(&mut payload, 
                                "BINARY_DISCOVERY_LOOKUP",
                                expose.name.as_deref().unwrap_or(""),
                            );
                        entries.push(Entry {
                            r#type: "binary_sensor".into(),
                            object_id: expose.name.clone().unwrap_or(property),
                            payload,
                        });
                    }
                }
                "numeric" => {
                    let property = expose.property.clone().unwrap_or_default();
                    let label = expose
                        .label
                        .clone()
                        .or_else(|| expose.name.clone())
                        .unwrap_or_else(|| property.clone());
                    let allows_set = expose.access & access::SET != 0;
                    let mut payload = Map::new();
                    payload.insert("name".into(), Value::String(label.clone()));
                    payload.insert(
                        "value_template".into(),
                        Value::String(state_value_template(&property)),
                    );
                    if allows_set {
                        payload.insert("command_topic".into(), Value::Bool(true));
                        payload.insert(
                            "command_topic_postfix".into(),
                            Value::String(property.clone()),
                        );
                    } else {
                        payload.insert("enabled_by_default".into(), Value::Bool(true));
                    }
                    if let Some(unit) = &expose.unit {
                        payload.insert("unit_of_measurement".into(), Value::String(unit.clone()));
                    }
                    if let Some(step) = expose.value_step {
                        payload.insert("step".into(), Value::from(step));
                    }
                    let mut lookup_extra = if allows_set {
                        lookup(
                            "NUMERIC_DISCOVERY_LOOKUP",
                            &clean_name(expose.name.as_deref().unwrap_or("")),
                        )
                        .cloned()
                        .unwrap_or_default()
                    } else {
                        let mut key = clean_name(expose.name.as_deref().unwrap_or(""));
                        if expose.name.as_deref() == Some("voc")
                            && expose
                                .unit
                                .as_deref()
                                .is_some_and(|unit| unit == "ppb" || unit == "ppm")
                        {
                            key = "voc_parts".to_string();
                        }
                        lookup("NUMERIC_DISCOVERY_LOOKUP", &key)
                            .cloned()
                            .unwrap_or_default()
                    };
                    if allows_set {
                        if lookup_extra.get("device_class").and_then(Value::as_str)
                            != Some("temperature")
                        {
                            lookup_extra.remove("device_class");
                        }
                    } else if let Some(unit) = &expose.unit {
                        if ["Wh", "kWh"].contains(&unit.as_str()) {
                            lookup_extra.insert("device_class".into(), json!("energy"));
                            lookup_extra.insert("state_class".into(), json!("total_increasing"));
                        } else if ["A", "mA"].contains(&unit.as_str()) {
                            lookup_extra.insert("device_class".into(), json!("current"));
                            lookup_extra.insert("state_class".into(), json!("measurement"));
                        } else if ["mW", "W", "kW"].contains(&unit.as_str()) {
                            lookup_extra.insert("device_class".into(), json!("power"));
                            lookup_extra.insert("state_class".into(), json!("measurement"));
                        }
                    }
                    merge(&mut payload, lookup_extra);
                    if payload.get("device_class").is_some()
                        && !payload.contains_key("unit_of_measurement")
                    {
                        payload.remove("device_class");
                    }
                    if payload.get("entity_category").and_then(Value::as_str) == Some("config") {
                        payload.insert("entity_category".into(), json!("diagnostic"));
                    }
                    if allows_set {
                        if let Some(min) = expose.value_min {
                            payload.insert("min".into(), Value::from(min));
                        }
                        if let Some(max) = expose.value_max {
                            payload.insert("max".into(), Value::from(max));
                        }
                        entries.push(Entry {
                            r#type: "number".into(),
                            object_id: expose.name.clone().unwrap_or(property.clone()),
                            payload,
                        });
                    } else {
                        entries.push(Entry {
                            r#type: "sensor".into(),
                            object_id: expose.name.clone().unwrap_or(property.clone()),
                            payload,
                        });
                    }
                }
                "enum" => {
                    let property = expose.property.clone().unwrap_or_default();
                    if property == "action" {
                        if self.experimental_event_entities
                            && expose.access & access::STATE != 0
                            && expose.access & access::SET == 0
                        {
                            let mut payload = Map::new();
                            payload.insert("state_topic".into(), Value::Bool(true));
                            payload.insert(
                                "event_types".into(),
                                Value::Array(
                                    expose.values.iter().cloned().map(Value::String).collect(),
                                ),
                            );
                            merge_lookup(&mut payload, 
                                    "ENUM_DISCOVERY_LOOKUP",
                                    expose.name.as_deref().unwrap_or(""),
                                );
                            entries.push(Entry {
                                r#type: "event".into(),
                                object_id: property.clone(),
                                payload,
                            });
                        }
                        if !self.legacy_action_sensor {
                            continue;
                        }
                    }
                    let label = expose
                        .label
                        .clone()
                        .or_else(|| expose.name.clone())
                        .unwrap_or_else(|| property.clone());
                    let mut payload = Map::new();
                    payload.insert("name".into(), Value::String(label));
                    if expose.access & access::STATE != 0 {
                        payload.insert(
                            "value_template".into(),
                            Value::String(state_value_template(&property)),
                        );
                    }
                    merge_lookup(&mut payload, 
                            "ENUM_DISCOVERY_LOOKUP",
                            expose.name.as_deref().unwrap_or(""),
                        );
                    if expose.access & access::SET != 0 && expose.values.len() == 1 {
                        payload.insert("state_topic".into(), Value::Bool(false));
                        payload.insert("command_topic".into(), Value::Bool(true));
                        payload.insert(
                            "command_topic_postfix".into(),
                            Value::String(property.clone()),
                        );
                        payload.insert(
                            "payload_press".into(),
                            Value::String(expose.values[0].clone()),
                        );
                        entries.push(Entry {
                            r#type: "button".into(),
                            object_id: property,
                            payload,
                        });
                    } else if expose.access & access::SET != 0 {
                        payload.insert("command_topic".into(), Value::Bool(true));
                        payload.insert(
                            "command_topic_postfix".into(),
                            Value::String(property.clone()),
                        );
                        payload.insert(
                            "options".into(),
                            Value::Array(
                                expose.values.iter().cloned().map(Value::String).collect(),
                            ),
                        );
                        entries.push(Entry {
                            r#type: "select".into(),
                            object_id: property,
                            payload,
                        });
                    } else if expose.access & access::STATE != 0 {
                        entries.push(Entry {
                            r#type: "sensor".into(),
                            object_id: property,
                            payload,
                        });
                    }
                }
                _ => {}
            }
        }
        entries
    }

    fn device_payload(&self, entity: &Arc<dyn Entity>) -> Value {
        let mut payload = Map::new();
        let identifier = if entity.is_group() {
            format!("zigbee2mqtt_group_{}", entity.entity_id().key())
        } else {
            format!("zigbee2mqtt_{}", entity.entity_id().key())
        };
        payload.insert("identifiers".into(), json!([identifier]));
        let settings = self.context.settings_value();
        let mut name = entity.name();
        if let Some(overridden) = entity
            .options()
            .get("homeassistant")
            .and_then(|ha| ha.get("name"))
            .and_then(Value::as_str)
        {
            name = overridden.to_string();
        }
        payload.insert("name".into(), Value::String(name));
        payload.insert(
            "sw_version".into(),
            json!(format!("Zigbee2MQTT {}", self.version.lock().unwrap())),
        );
        if entity.is_device() {
            if let Some(definition) = entity.definition() {
                payload.insert("model".into(), json!(definition.description));
                payload.insert("model_id".into(), json!(definition.model));
                payload.insert("manufacturer".into(), json!(definition.vendor));
            }
            if let Some(build) = entity.zh_device().and_then(|zh| zh.software_build_id()) {
                payload.insert("sw_version".into(), json!(build));
            }
        } else if entity.is_group() {
            payload.insert("model".into(), json!("Group"));
            payload.insert("manufacturer".into(), json!("Zigbee2MQTT"));
        }
        let url = settings["frontend"]["url"].as_str().unwrap_or("");
        if !url.is_empty() {
            let postfix = if entity.is_device() {
                format!("/#/device/0/{}/info", entity.entity_id().key())
            } else {
                format!("/#/group/0/{}", entity.entity_id().key())
            };
            payload.insert("configuration_url".into(), json!(format!("{url}{postfix}")));
        }
        let bridge = self.bridge_identifier.lock().unwrap().clone();
        if !bridge.is_empty() && bridge != identifier {
            payload.insert("via_device".into(), json!(bridge));
        }
        Value::Object(payload)
    }

    fn discovery_topic_for(&self, entry: &Entry, entity: &Arc<dyn Entity>) -> String {
        let key = if entity.is_device() {
            entity.entity_id().key()
        } else {
            format!("group_{}", entity.entity_id().key())
        };
        format!("{}/{}/{}/config", entry.r#type, key, entry.object_id)
    }

    async fn discover(&self, entity: Arc<dyn Entity>, publish: bool) {
        if entity.is_group() && entity.members_devices().is_empty() {
            return;
        }
        if entity.is_device()
            && (entity.definition().is_none()
                || !entity.interviewed()
                || entity
                    .options()
                    .get("homeassistant")
                    .map(|value| !value.as_bool().unwrap_or(false))
                    .unwrap_or(false))
        {
            return;
        }

        let id = entity.entity_id().key();
        let discovered = self.get_discovered(&id);
        let last_topics: Vec<String> = {
            let mut state = discovered.lock().unwrap();
            state.discovered = true;
            state.messages.keys().cloned().collect()
        };
        let mut new_topics = HashSet::new();
        let mut to_publish: Vec<(String, String)> = Vec::new();

        let configs = self.get_configs(&entity);
        let device_payload = self.device_payload(&entity);
        let settings = self.context.settings_value();
        let base_topic = format!(
            "{}/{}",
            settings["mqtt"]["base_topic"]
                .as_str()
                .unwrap_or("zigbee2mqtt"),
            entity.name()
        );
        let availability_enabled =
            utils::is_availability_enabled_for_entity(entity.as_ref(), &settings);

        for config in configs {
            let mut payload = config.payload.clone();
            let mut state_topic = base_topic.clone();
            if let Some(postfix) = payload
                .remove("state_topic_postfix")
                .and_then(|value| value.as_str().map(str::to_string))
            {
                state_topic = format!("{state_topic}/{postfix}");
            }
            if payload
                .get("state_topic")
                .map(|value| !value.is_boolean())
                .unwrap_or(false)
                || payload.get("state_topic").and_then(Value::as_bool) == Some(true)
                || !payload.contains_key("state_topic")
            {
                payload.insert("state_topic".into(), Value::String(state_topic.clone()));
            } else {
                payload.remove("state_topic");
            }

            payload.insert("device".into(), device_payload.clone());
            payload.insert("origin".into(), self.origin.lock().unwrap().clone());

            let mut object_id = device_payload["name"]
                .as_str()
                .unwrap_or_default()
                .replace(' ', "_")
                .to_lowercase();
            if config.object_id.starts_with(&config.r#type) && config.object_id.contains('_') {
                if let Some((_, suffix)) = config.object_id.split_once('_') {
                    object_id = format!("{object_id}_{suffix}");
                }
            } else if !config.object_id.starts_with(&config.r#type) {
                object_id = format!("{object_id}_{}", config.object_id);
            }
            payload.insert("object_id".into(), json!(object_id));
            payload.insert(
                "default_entity_id".into(),
                json!(format!("{}.{}", config.r#type, object_id)),
            );
            payload.insert(
                "unique_id".into(),
                json!(format!(
                    "{}_{}_{}",
                    entity
                        .options()
                        .get("ID")
                        .and_then(Value::as_str)
                        .unwrap_or(&id),
                    config.object_id,
                    settings["mqtt"]["base_topic"]
                        .as_str()
                        .unwrap_or("zigbee2mqtt")
                )),
            );

            if payload.get("availability").is_none()
                || payload.get("availability").and_then(Value::as_bool) != Some(false)
            {
                let mut availability = vec![
                    json!({"topic": format!("{}/bridge/state", settings["mqtt"]["base_topic"].as_str().unwrap_or("zigbee2mqtt")), "value_template": "{{ value_json.state }}"}),
                ];
                if availability_enabled && !entity.is_device()
                    || availability_enabled && entity.is_device()
                {
                    availability.push(json!({"topic": format!("{base_topic}/availability"), "value_template": "{{ value_json.state }}"}));
                }
                payload.insert("availability".into(), Value::Array(availability));
                payload.insert("availability_mode".into(), json!("all"));
            }

            let command_prefix = payload
                .remove("command_topic_prefix")
                .and_then(|value| value.as_str().map(|value| format!("{value}/")))
                .unwrap_or_default();
            let command_postfix = payload
                .remove("command_topic_postfix")
                .and_then(|value| value.as_str().map(|value| format!("/{value}")))
                .unwrap_or_default();
            if payload.get("command_topic").and_then(Value::as_bool) == Some(true) {
                payload.insert(
                    "command_topic".into(),
                    json!(format!("{base_topic}/{command_prefix}set{command_postfix}")),
                );
            }

            let topic = self.discovery_topic_for(&config, &entity);
            let payload_str = stringify(&Value::Object(payload));
            new_topics.insert(topic.clone());
            let should_publish = {
                let mut state = discovered.lock().unwrap();
                let should_publish = state
                    .messages
                    .get(&topic)
                    .map(|(previous, published)| !*published || previous != &payload_str)
                    .unwrap_or(true);
                if should_publish {
                    state
                        .messages
                        .insert(topic.clone(), (payload_str.clone(), publish));
                }
                should_publish
            };
            if should_publish && publish {
                to_publish.push((topic, payload_str));
            }
        }

        let mut cleared = Vec::new();
        {
            let mut state = discovered.lock().unwrap();
            for topic in last_topics {
                if !new_topics.contains(&topic) {
                    state.messages.remove(&topic);
                    if publish && !topic.starts_with("device_automation") {
                        cleared.push(topic);
                    }
                }
            }
        }

        let mqtt = self.context.mqtt.clone();
        let discovery_topic = self.discovery_topic.clone();
        for (topic, payload) in to_publish {
            mqtt.lock()
                .await
                .publish(
                    &topic,
                    payload,
                    MqttPublishOptions {
                        retain: true,
                        qos: 1,
                        base_topic: Some(discovery_topic.clone()),
                        ..Default::default()
                    },
                )
                .await;
        }
        for topic in cleared {
            mqtt.lock()
                .await
                .publish(
                    &topic,
                    "",
                    MqttPublishOptions {
                        retain: true,
                        qos: 1,
                        base_topic: Some(discovery_topic.clone()),
                        ..Default::default()
                    },
                )
                .await;
        }
    }

    async fn clear_entity(&self, entity: &Arc<dyn Entity>) {
        let id = entity.entity_id().key();
        let discovered = self.discovered.lock().unwrap().remove(&id);
        if let Some(discovered) = discovered {
            let topics: Vec<String> = discovered
                .lock()
                .unwrap()
                .messages
                .keys()
                .cloned()
                .collect();
            for topic in topics {
                self.context
                    .mqtt
                    .lock()
                    .await
                    .publish(
                        &topic,
                        "",
                        MqttPublishOptions {
                            retain: true,
                            qos: 1,
                            base_topic: Some(self.discovery_topic.clone()),
                            ..Default::default()
                        },
                    )
                    .await;
            }
        }
    }
}

impl Extension for HomeAssistant {
    fn name(&self) -> &'static str {
        "HomeAssistant"
    }

    fn context(&self) -> &ExtensionContext {
        &self.context
    }

    fn start(self: Arc<Self>) -> ExtensionFuture {
        Box::pin(async move {
            let settings = self.context.settings_value();
            if !settings["homeassistant"]["enabled"]
                .as_bool()
                .unwrap_or(false)
            {
                return Err(
                    "Home Assistant extension created with setting 'enabled: false'".to_string(),
                );
            }
            if settings["advanced"]["output"].as_str() == Some("attribute") {
                return Err(
                    "Home Assistant integration requires 'output: json' under 'advanced'"
                        .to_string(),
                );
            }
            if settings["homeassistant"]["discovery_topic"].as_str()
                == settings["mqtt"]["base_topic"].as_str()
            {
                return Err(format!(
                    "'homeassistant.discovery_topic' cannot not be equal to the 'mqtt.base_topic' (got '{}')",
                    settings["mqtt"]["base_topic"].as_str().unwrap_or("zigbee2mqtt")
                ));
            }

            *self.version.lock().unwrap() = utils::get_zigbee2mqtt_version().await["version"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            *self.origin.lock().unwrap() = json!({
                "name": "Zigbee2MQTT",
                "sw": self.version.lock().unwrap().clone(),
                "url": "https://www.zigbee2mqtt.io",
            });

            let coordinator = self
                .context
                .zigbee
                .first_coordinator_endpoint()
                .map(|endpoint| endpoint.device_ieee_address.lock().unwrap().clone())
                .unwrap_or_default();
            let bridge_id = if coordinator.is_empty() {
                String::new()
            } else {
                format!("zigbee2mqtt_bridge_{coordinator}")
            };
            *self.bridge_identifier.lock().unwrap() = bridge_id;

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_entity_removed(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(ieee) = data
                            .get("entity")
                            .and_then(|entity| entity.get("ieeeAddr"))
                            .and_then(Value::as_str)
                        {
                            if let Some(device) = this.context.zigbee.resolve_device(ieee) {
                                this.clear_entity(&(device as Arc<dyn Entity>)).await;
                            }
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_entity_renamed(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(ieee) = data
                            .get("entity")
                            .and_then(|entity| entity.get("ieeeAddr"))
                            .and_then(Value::as_str)
                        {
                            if let Some(device) = this.context.zigbee.resolve_device(ieee) {
                                let device = device as Arc<dyn Entity>;
                                this.clear_entity(&device).await;
                                this.discover(device, true).await;
                            }
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_device_joined(
                self.context.key,
                Arc::new(move |data| {
                    let ieee = data
                        .get("device")
                        .and_then(|device| device.get("ieeeAddr"))
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(device) =
                            ieee.and_then(|ieee| this.context.zigbee.resolve_device(&ieee))
                        {
                            let id = device.entity_id().key();
                            let discovered = this.get_discovered(&id).lock().unwrap().discovered;
                            if !discovered {
                                this.discover(device as Arc<dyn Entity>, true).await;
                            }
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_device_message(
                self.context.key,
                Arc::new(move |data| {
                    let ieee = data
                        .get("device")
                        .and_then(|device| device.get("ieeeAddr"))
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(device) =
                            ieee.and_then(|ieee| this.context.zigbee.resolve_device(&ieee))
                        {
                            let id = device.entity_id().key();
                            let discovered = this.get_discovered(&id).lock().unwrap().discovered;
                            if !discovered {
                                this.discover(device as Arc<dyn Entity>, true).await;
                            }
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_mqtt_message(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        if data.get("topic").and_then(Value::as_str)
                            == Some(this.status_topic.as_str())
                            && data
                                .get("message")
                                .and_then(Value::as_str)
                                .map(str::to_lowercase)
                                .as_deref()
                                == Some("online")
                        {
                            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                            this.context
                                .mqtt
                                .lock()
                                .await
                                .publish(
                                    "bridge/state",
                                    stringify(&json!({"state": "online"})),
                                    MqttPublishOptions {
                                        retain: true,
                                        qos: 1,
                                        ..Default::default()
                                    },
                                )
                                .await;
                            for entity in this.context.zigbee.devices_and_groups_iterator() {
                                let id = entity.entity_id();
                                let exists = this.context.state.lock().unwrap().exists(&id);
                                if entity.is_device() && exists {
                                    let payload = this.context.state.lock().unwrap().get(&id);
                                    (this.context.publish_entity_state)(
                                        entity,
                                        payload,
                                        Some("publishCached".to_string()),
                                    )
                                    .await;
                                }
                            }
                        }
                    });
                }),
            );

            self.context
                .mqtt
                .lock()
                .await
                .subscribe(&self.status_topic, 0)
                .await;

            for device in self
                .context
                .zigbee
                .devices_iterator(utils::device_not_coordinator)
            {
                self.discover(device as Arc<dyn Entity>, false).await;
            }
            for group in self.context.zigbee.groups_iterator(|_| true) {
                self.discover(group as Arc<dyn Entity>, false).await;
            }
            for device in self
                .context
                .zigbee
                .devices_iterator(utils::device_not_coordinator)
            {
                self.discover(device as Arc<dyn Entity>, true).await;
            }
            for group in self.context.zigbee.groups_iterator(|_| true) {
                self.discover(group as Arc<dyn Entity>, true).await;
            }

            Ok(())
        })
    }

    fn adjust_message_before_publish(&self, entity: &dyn Entity, message: &mut Value) {
        self.adjust_color(message);
        if entity.is_device()
            && entity.definition().is_some_and(|definition| definition.ota.is_some())
            && message.get("update").and_then(|update| update.get("latest_version")).is_none()
        {
            message["update"] = json!({"installed_version": -1, "latest_version": -1});
        }
    }
}

impl HomeAssistant {
    fn discovery_regex(&self) -> Option<regex::Regex> {
        regex::Regex::new(&format!(
            "{}/(.*)/(.*)/(.*)/config",
            regex::escape(&self.discovery_topic)
        ))
        .ok()
    }

    fn adjust_color(&self, message: &mut Value) {
        if let Some(color) = message.get_mut("color").and_then(Value::as_object_mut) {
            if let Some(hue) = color.get("hue").cloned() {
                color.insert("h".to_string(), hue);
            }
            if let Some(saturation) = color.get("saturation").cloned() {
                color.insert("s".to_string(), saturation);
            }
        }
    }
}
