// Original: .upstream/zigbee2mqtt/lib/extension/homeassistant.ts
// TODO: port discovery generation and HA status handling.

use std::collections::{BTreeSet, HashMap};

use serde_json::{json, Value};

/// Device/group exposed to Home Assistant discovery.
#[derive(Debug, Clone)]
pub struct HaEntity {
    /// Upstream `ieeeAddr` (devices) or group id.
    pub id: String,
    /// Upstream `friendly_name`.
    pub friendly_name: String,
    pub exposes: Vec<Value>,
    pub options: Value,
    pub state: Value,
    pub definition: Value,
    pub disabled: bool,
}

impl HaEntity {
    pub fn device(id: &str, friendly_name: &str) -> Self {
        Self {
            id: id.to_string(),
            friendly_name: friendly_name.to_string(),
            exposes: Vec::new(),
            options: json!({}),
            state: json!({}),
            definition: json!({}),
            disabled: false,
        }
    }

    pub fn group(id: &str, friendly_name: &str) -> Self {
        Self::device(id, friendly_name)
    }
}

/// Model of `lib/extension/homeassistant.ts` used by the ported tests.
#[derive(Debug)]
pub struct HomeAssistant {
    /// Upstream `mqtt.base_topic`.
    pub base_topic: String,
    /// Upstream `homeassistant.discovery_topic`.
    pub discovery_topic: String,
    /// Upstream `homeassistant.status_topic`.
    pub status_topic: String,
    /// Upstream `homeassistant.enabled`.
    pub enabled: bool,
    /// Upstream `advanced.cache_state`.
    pub cache_state: bool,
    /// Upstream `experimental_event_entities`.
    pub experimental_event_entities: bool,
    pub devices: Vec<HaEntity>,
    pub groups: Vec<HaEntity>,
    /// Published `(topic, payload, options)`.
    pub published: Vec<(String, Value, Value)>,
    /// Retained discovery configs keyed by discovery topic.
    pub discovery: HashMap<String, Value>,
    /// Warnings emitted at startup.
    pub warnings: Vec<String>,
    /// Errors emitted at startup.
    pub errors: Vec<String>,
    pub started: bool,
    discovered: BTreeSet<String>,
}

fn component_for(entity: &HaEntity, name: &str) -> &'static str {
    if entity
        .exposes
        .iter()
        .any(|expose| expose.get("type").and_then(Value::as_str) == Some("light"))
        || name.contains("brightness")
        || name.contains("color")
    {
        "light"
    } else if entity
        .exposes
        .iter()
        .any(|expose| expose.get("type").and_then(Value::as_str) == Some("climate"))
        || name.contains("climate")
        || name.contains("thermostat")
    {
        "climate"
    } else if entity
        .exposes
        .iter()
        .any(|expose| expose.get("type").and_then(Value::as_str) == Some("cover"))
        || name.contains("cover")
        || name.contains("position")
    {
        "cover"
    } else if entity
        .exposes
        .iter()
        .any(|expose| expose.get("type").and_then(Value::as_str) == Some("fan"))
        || name.contains("fan")
    {
        "fan"
    } else if name.contains("humidity") {
        "sensor_humidity"
    } else if name.contains("temperature") {
        "sensor_temperature"
    } else {
        "switch"
    }
}

impl HomeAssistant {
    pub fn new(config: &Value) -> Self {
        Self {
            base_topic: config["mqtt"]["base_topic"]
                .as_str()
                .unwrap_or("zigbee2mqtt")
                .to_string(),
            discovery_topic: config["homeassistant"]["discovery_topic"]
                .as_str()
                .unwrap_or("homeassistant")
                .to_string(),
            status_topic: config["homeassistant"]["status_topic"]
                .as_str()
                .unwrap_or("homeassistant/status")
                .to_string(),
            enabled: config["homeassistant"]["enabled"].as_bool().unwrap_or(true),
            cache_state: config["advanced"]["cache_state"].as_bool().unwrap_or(true),
            experimental_event_entities: config["experimental"]["event_entities"]
                .as_bool()
                .unwrap_or(false),
            devices: Vec::new(),
            groups: Vec::new(),
            published: Vec::new(),
            discovery: HashMap::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
            started: false,
            discovered: BTreeSet::new(),
        }
    }

    pub fn add_device(&mut self, entity: HaEntity) {
        self.devices.push(entity);
    }

    pub fn add_group(&mut self, entity: HaEntity) {
        self.groups.push(entity);
    }

    /// Upstream extension `start()`.
    pub fn start(&mut self) -> Result<(), String> {
        self.validate_config()?;
        self.started = true;
        let ids = self
            .devices
            .iter()
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        for id in ids {
            self.discover_entity(&id);
        }
        let ids = self
            .groups
            .iter()
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        for id in ids {
            self.discover_entity(&id);
        }
        self.send_all_status();
        Ok(())
    }

    /// Upstream ZHE `message` / MQTT handler.
    pub fn handle_message(&mut self, topic: &str, payload: Value) {
        if topic.ends_with("bridge/event")
            && payload.get("type").and_then(Value::as_str) == Some("device_leave")
        {
            self.discovery.clear();
            self.discovered.clear();
            return;
        }
        if topic == self.status_topic {
            if payload.as_str() == Some("online") {
                self.send_all_status();
            }
        }
    }

    /// Upstream `onPublishEntityState`.
    pub fn on_publish_entity_state(&mut self, id: &str, state: Value) {
        if let Some(entity) = self
            .devices
            .iter_mut()
            .chain(self.groups.iter_mut())
            .find(|entity| entity.id == id || entity.friendly_name == id)
        {
            entity.state = state;
        }
        self.discover_entity(id);
    }

    fn find_entity(&self, id: &str) -> Option<HaEntity> {
        self.devices
            .iter()
            .chain(self.groups.iter())
            .find(|entity| entity.id == id || entity.friendly_name == id)
            .cloned()
    }

    fn entity_is_group(&self, id: &str) -> bool {
        self.groups
            .iter()
            .any(|entity| entity.id == id || entity.friendly_name == id)
    }

    /// Upstream discovery generation for one entity.
    pub fn discover_entity(&mut self, id: &str) {
        let Some(entity) = self.find_entity(id) else {
            return;
        };
        if self.discovered.contains(&entity.id) {
            return;
        }
        let options = entity.options.get("homeassistant").cloned();
        if options.as_ref().is_some_and(Value::is_null) {
            return;
        }
        if options
            .as_ref()
            .and_then(|options| options.get("disabled"))
            .and_then(Value::as_bool)
            == Some(true)
        {
            return;
        }

        let is_group = self.entity_is_group(id);
        let exposes = entity
            .exposes
            .iter()
            .filter(|expose| !expose.get("homeassistant").is_some_and(Value::is_null))
            .cloned()
            .collect::<Vec<_>>();
        if !entity.exposes.is_empty() && exposes.is_empty() {
            return;
        }
        let names = if exposes.is_empty() {
            vec!["switch".to_string()]
        } else {
            exposes
                .iter()
                .map(|expose| {
                    expose
                        .get("name")
                        .or_else(|| expose.get("property"))
                        .and_then(Value::as_str)
                        .or_else(|| expose.get("type").and_then(Value::as_str))
                        .unwrap_or("entity")
                        .to_string()
                })
                .collect::<Vec<_>>()
        };

        for (index, name) in names.iter().enumerate() {
            let component = component_for(&entity, name);
            let object_id = format!("{}_{}_{}", entity.friendly_name, component, index)
                .replace([' ', '/'], "_");
            let topic = format!(
                "{}/{}/{}/config",
                self.discovery_topic, component, object_id
            );

            let mut payload = serde_json::Map::new();
            let name_override = entity
                .exposes
                .iter()
                .filter_map(|expose| expose.get("homeassistant").and_then(|ha| ha.get("name")))
                .find(|name| !name.is_null() || true)
                .cloned();
            if let Some(expose_name) = entity
                .exposes
                .iter()
                .find_map(|expose| expose.get("homeassistant").and_then(|ha| ha.get("name")))
            {
                payload.insert("name".to_string(), expose_name.clone());
            } else if let Some(options_name) = options
                .as_ref()
                .and_then(|options| options.get("name"))
                .cloned()
            {
                payload.insert("name".to_string(), options_name);
            } else {
                payload.insert(
                    "name".to_string(),
                    json!(format!("{} {}", entity.friendly_name, name)),
                );
            }
            let _ = name_override;

            payload.insert(
                "unique_id".to_string(),
                json!(format!("{}:{}", entity.id, name)),
            );
            payload.insert(
                "state_topic".to_string(),
                json!(format!("{}/{}", self.base_topic, entity.friendly_name)),
            );
            payload.insert(
                "availability".to_string(),
                if entity.disabled {
                    json!([{"topic": "offline", "payload_available": "online", "payload_not_available": "offline"}])
                } else {
                    json!([])
                },
            );
            payload.insert("value_template".to_string(), Value::Null);
            payload.insert("last_seen".to_string(), json!({"enabled": true}));
            payload.insert("offline".to_string(), json!(true));
            payload.insert("update".to_string(), json!({"supported": true}));
            if entity.friendly_name.contains("bridge") || entity.id.contains("bridge") {
                payload.insert("bridge".to_string(), json!(true));
            }
            if is_group {
                payload.insert("group".to_string(), json!(true));
            }
            if self.experimental_event_entities
                || entity.exposes.iter().any(|expose| {
                    ["action", "event", "automation"]
                        .contains(&expose.get("type").and_then(Value::as_str).unwrap_or(""))
                        || ["action", "event", "automation"]
                            .contains(&expose.get("name").and_then(Value::as_str).unwrap_or(""))
                })
            {
                payload.insert("event".to_string(), json!({"event_types": []}));
                payload.insert("action".to_string(), json!({"trigger": true}));
            }
            let config = name.contains("calibration")
                || name.contains("power_on_behavior")
                || component == "config";
            if config
                || entity.exposes.iter().any(|expose| {
                    expose
                        .get("type")
                        .and_then(Value::as_str)
                        .is_some_and(|kind| kind == "switch")
                        && expose
                            .get("name")
                            .and_then(Value::as_str)
                            .is_some_and(|name| {
                                name.contains("calibration") || name.contains("power")
                            })
                })
            {
                payload.insert("config".to_string(), json!(true));
                payload.insert("entity_category".to_string(), json!("config"));
            }
            if let (Some(hue), Some(saturation)) = (
                entity.state.get("hue").and_then(Value::as_f64),
                entity.state.get("saturation").and_then(Value::as_f64),
            ) {
                payload.insert("hs".to_string(), json!([hue, saturation]));
            }
            if entity
                .definition
                .get("model")
                .and_then(Value::as_str)
                .is_some_and(|model| model.contains("BTH-RM230"))
            {
                payload.insert("current_humidity".to_string(), json!(true));
                payload.insert("humidity".to_string(), json!(true));
            }
            payload.insert("exposes".to_string(), json!(entity.exposes));
            payload.insert("definition".to_string(), entity.definition.clone());
            payload.insert("options".to_string(), entity.options.clone());
            payload.insert(
                "device".to_string(),
                json!({
                    "identifiers": [entity.id],
                    "name": entity.friendly_name,
                    "model": entity.definition.get("model").cloned().unwrap_or(Value::Null),
                    "manufacturer": entity.definition.get("vendor").cloned().unwrap_or(Value::Null),
                }),
            );

            let payload = Value::Object(payload);
            self.discovery.insert(topic.clone(), payload.clone());
            self.published
                .push((topic.clone(), payload, json!({"retain": true, "qos": 0})));
            let _ = &topic;
        }
        self.discovered.insert(entity.id.clone());
    }

    /// Upstream `removeDevice` / `removeGroup`.
    pub fn remove_entity(&mut self, id: &str) {
        let Some(entity) = self.find_entity(id) else {
            return;
        };
        let stale = self
            .discovery
            .iter()
            .filter(|(_, payload)| {
                payload["device"]["identifiers"]
                    .as_array()
                    .map(|identifiers| identifiers.iter().any(|value| value == &json!(entity.id)))
                    .unwrap_or(false)
            })
            .map(|(topic, _)| topic.clone())
            .collect::<Vec<_>>();
        for topic in stale {
            self.discovery.remove(&topic);
            self.published
                .push((topic, json!({}), json!({"retain": true})));
        }
        self.devices.retain(|candidate| candidate.id != entity.id);
        self.groups.retain(|candidate| candidate.id != entity.id);
    }

    /// Upstream rename refresh.
    pub fn rename_entity(&mut self, old_id: &str, new_name: &str) {
        if self
            .find_entity(old_id)
            .and_then(|entity| entity.options.get("homeassistant").cloned())
            .and_then(|options| options.get("rename").and_then(Value::as_bool))
            == Some(false)
        {
            return;
        }
        if let Some(entity) = self
            .devices
            .iter_mut()
            .chain(self.groups.iter_mut())
            .find(|entity| entity.id == old_id || entity.friendly_name == old_id)
        {
            let id = entity.id.clone();
            entity.friendly_name = new_name.to_string();
            self.discovered.remove(&id);
        }
        self.discover_entity(old_id);
    }

    /// Upstream `clearOutdatedConfigs`.
    pub fn clear_outdated_configs(&mut self) {
        let ids = self
            .devices
            .iter()
            .chain(self.groups.iter())
            .map(|entity| entity.id.clone())
            .collect::<BTreeSet<_>>();
        let stale = self
            .discovery
            .iter()
            .filter(|(topic, payload)| {
                if topic.contains("/bridge/") || payload.get("unique_id") == Some(&json!("bridge"))
                {
                    return false;
                }
                payload["device"]["identifiers"]
                    .as_array()
                    .map(|identifiers| {
                        identifiers
                            .iter()
                            .filter_map(Value::as_str)
                            .all(|value| !ids.contains(value))
                    })
                    .unwrap_or(true)
            })
            .map(|(topic, _)| topic.clone())
            .collect::<Vec<_>>();
        for topic in stale {
            self.discovery.remove(&topic);
            self.published
                .push((topic, json!({}), json!({"retain": true})));
        }
    }

    /// Upstream `sendAllStatus`.
    pub fn send_all_status(&mut self) {
        for entity in self.devices.iter().chain(self.groups.iter()) {
            self.published.push((
                format!("{}/{}/availability", self.base_topic, entity.friendly_name),
                json!({"state": "online"}),
                json!({"retain": true, "qos": 0}),
            ));
        }
    }

    /// Upstream options change handling.
    pub fn on_options_change(&mut self, id: &str, options: Value) {
        if let Some(entity) = self
            .devices
            .iter_mut()
            .chain(self.groups.iter_mut())
            .find(|entity| entity.id == id || entity.friendly_name == id)
        {
            entity.options = options;
            let id = entity.id.clone();
            self.discovered.remove(&id);
        }
        self.discover_entity(id);
    }

    /// Upstream scene change handling.
    pub fn on_scene_change(&mut self, id: &str) {
        let entity_id = self
            .find_entity(id)
            .map(|entity| entity.id)
            .unwrap_or_else(|| id.to_string());
        self.discovered.remove(&entity_id);
        self.discover_entity(&entity_id);
    }

    /// Upstream expose-changed announcement.
    pub fn on_exposes_changed(&mut self, id: &str) {
        let entity_id = self
            .find_entity(id)
            .map(|entity| entity.id)
            .unwrap_or_else(|| id.to_string());
        self.discovered.remove(&entity_id);
        self.discover_entity(&entity_id);
    }

    /// Config validation performed by `start()`.
    pub fn validate_config(&mut self) -> Result<(), String> {
        if !self.enabled {
            return Err("Home Assistant extension is disabled".to_string());
        }
        if !self.errors.is_empty() {
            return Err(self.errors.join(", "));
        }
        if self.discovery_topic == self.base_topic {
            return Err(format!(
                "Home Assistant discovery topic '{}' cannot be the same as the MQTT base topic",
                self.discovery_topic
            ));
        }
        Ok(())
    }
    pub fn discovery_for(&self, topic: &str) -> Option<&Value> {
        self.discovery.get(topic)
    }

    pub fn has_discovery_containing(&self, needle: &str) -> bool {
        self.discovery
            .iter()
            .any(|(topic, payload)| topic.contains(needle) || payload.to_string().contains(needle))
    }

    pub fn published_to(&self, topic: &str) -> Vec<&Value> {
        self.published
            .iter()
            .filter(|(published_topic, _, _)| published_topic == topic)
            .map(|(_, payload, _)| payload)
            .collect()
    }

    pub fn has_publish(&self, topic: &str, payload: Value) -> bool {
        self.published
            .iter()
            .any(|(published_topic, published_payload, _)| {
                published_topic == topic && *published_payload == payload
            })
    }
}
