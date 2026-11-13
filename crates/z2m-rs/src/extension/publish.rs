// Original: .upstream/zigbee2mqtt/lib/extension/publish.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// Group `membersState` in converter meta is not populated yet (groups land
// with the groups extension); everything else follows the upstream flow.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use serde_json::{json, Map, Value};

use zigbee_converters::{DefinitionRef, ToMeta, ToTarget, ToZigbeeConverter};

use crate::event_bus::EventBus;
use crate::model::entity::{Entity, PublishEntityState};
use crate::state::State;
use crate::util::logger::Logger;
use crate::util::settings::Settings;
use crate::util::stringify::stringify;
use crate::util::utils;
use crate::zigbee::Zigbee;

/// Upstream `STATE_VALUES` (`publish.ts:20`).
const STATE_VALUES: [&str; 8] = [
    "on", "off", "toggle", "open", "close", "stop", "lock", "unlock",
];

/// Upstream `SCENE_CONVERTER_KEYS` (`publish.ts:21`).
const SCENE_CONVERTER_KEYS: [&str; 5] = [
    "scene_store",
    "scene_add",
    "scene_remove",
    "scene_remove_all",
    "scene_rename",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTopic {
    pub id: String,
    pub endpoint: Option<String>,
    pub attribute: Option<String>,
    pub r#type: String,
}

/// Upstream `Publish` (`lib/extension/publish.ts:31`).
pub struct Publish {
    event_bus: Arc<Mutex<EventBus>>,
    zigbee: Arc<Zigbee>,
    settings: Arc<Mutex<Settings>>,
    logger: Arc<Mutex<Logger>>,
    state: Arc<Mutex<State>>,
    publish_entity_state: PublishEntityState,
}

impl Publish {
    pub fn new(
        event_bus: Arc<Mutex<EventBus>>,
        zigbee: Arc<Zigbee>,
        settings: Arc<Mutex<Settings>>,
        logger: Arc<Mutex<Logger>>,
        state: Arc<Mutex<State>>,
        publish_entity_state: PublishEntityState,
    ) -> Arc<Self> {
        Arc::new(Self {
            event_bus,
            zigbee,
            settings,
            logger,
            state,
            publish_entity_state,
        })
    }

    /// Upstream `start()` (`publish.ts:39-42`).
    pub fn attach(self: &Arc<Self>, key: usize) {
        let bus = self.event_bus.clone();
        let this = self.clone();
        bus.lock().unwrap().on_mqtt_message(
            key,
            Arc::new(move |data| {
                let this = this.clone();
                tokio::spawn(async move { this.on_mqtt_message(data).await });
            }),
        );
    }

    /// Upstream `parseTopic` + `loadTopicGetSetRegex` (`publish.ts:16,44`).
    pub fn parse_topic(&self, topic: &str) -> Option<ParsedTopic> {
        let base_topic = self.settings.lock().unwrap().get()["mqtt"]["base_topic"]
            .as_str()
            .unwrap_or("zigbee2mqtt")
            .to_string();
        let pattern = format!("^{}/(.+?)/(get|set)(?:/(.+))?$", regex::escape(&base_topic));
        let regex = regex::Regex::new(&pattern).ok()?;
        let captures = regex.captures(topic)?;

        let raw_entity = captures.get(1)?.as_str();
        // Upstream negative lookahead `(?!bridge)`.
        if raw_entity.starts_with("bridge") {
            return None;
        }

        // Upstream `parseTopic` resolves `device_name[/endpoint_name]`.
        let entity = self.zigbee.resolve_entity_and_endpoint(raw_entity);

        Some(ParsedTopic {
            id: entity.id.clone(),
            endpoint: entity.endpoint_id.clone(),
            attribute: captures.get(3).map(|value| value.as_str().to_string()),
            r#type: captures.get(2)?.as_str().to_string(),
        })
    }

    /// Upstream `parseMessage` (`publish.ts:64-79`).
    pub fn parse_message(parsed_topic: &ParsedTopic, message: &str) -> Option<Value> {
        if let Some(attribute) = &parsed_topic.attribute {
            let value = serde_json::from_str::<Value>(message)
                .unwrap_or_else(|_| Value::String(message.to_string()));
            return Some(json!({attribute: value}));
        }

        match serde_json::from_str::<Value>(message) {
            Ok(value) => Some(value),
            Err(_) => STATE_VALUES
                .contains(&message.to_lowercase().as_str())
                .then(|| json!({"state": message})),
        }
    }

    /// Upstream `updateMessageHomeAssistant` (`publish.ts:81-96`).
    pub fn update_message_home_assistant(&self, message: &mut Value, entity_state: &Value) {
        if !self.settings.lock().unwrap().get()["homeassistant"]["enabled"]
            .as_bool()
            .unwrap_or(false)
        {
            return;
        }

        let Value::Object(map) = message else {
            return;
        };
        let has_color_temp = map.contains_key("color_temp");
        let has_color = map.contains_key("color");
        let has_brightness = map.contains_key("brightness");
        if entity_state.get("state").and_then(Value::as_str) == Some("ON")
            && (has_color_temp || has_color)
            && !has_brightness
        {
            map.remove("state");
            self.logger
                .lock()
                .unwrap()
                .debug("Skipping state because of Home Assistant", "z2m");
        }
    }

    /// Upstream `onMQTTMessage` (`publish.ts:98-280`).
    pub async fn on_mqtt_message(self: &Arc<Self>, data: Value) {
        let topic = data["topic"].as_str().unwrap_or_default();
        let Some(parsed) = self.parse_topic(topic) else {
            return;
        };

        let Some(entity) = self.zigbee.resolve_entity(&parsed.id) else {
            self.logger
                .lock()
                .unwrap()
                .error(&format!("Entity '{}' is unknown", parsed.id), "z2m");
            return;
        };
        entity.resolve_definition(false);

        let definitions: Vec<DefinitionRef> = if entity.is_device() {
            match entity.definition() {
                Some(definition) => vec![definition],
                None => {
                    self.logger.lock().unwrap().error(
                        &format!("Cannot publish to unsupported device '{}'", entity.name()),
                        "z2m",
                    );
                    return;
                }
            }
        } else {
            entity.members_definitions()
        };

        let target = if entity.is_device() {
            entity
                .endpoint(parsed.endpoint.as_deref())
                .map(ToTarget::Endpoint)
        } else {
            entity.zh_group().map(ToTarget::Group)
        };
        let Some(target) = target else {
            self.logger.lock().unwrap().error(
                &format!(
                    "Device '{}' has no endpoint '{}'",
                    entity.name(),
                    parsed
                        .endpoint
                        .clone()
                        .unwrap_or_else(|| "undefined".to_string())
                ),
                "z2m",
            );
            return;
        };

        let message_text = data["message"].as_str().unwrap_or_default();
        let Some(mut message) = Self::parse_message(&parsed, message_text) else {
            self.logger
                .lock()
                .unwrap()
                .error("Invalid message 'undefined', skipping...", "z2m");
            return;
        };
        if !message.is_object() {
            self.logger
                .lock()
                .unwrap()
                .error("Invalid message 'undefined', skipping...", "z2m");
            return;
        }

        let entity_state = self.state.lock().unwrap().get(&entity.entity_id());
        let converters = definition_converters(&definitions);
        self.update_message_home_assistant(&mut message, &entity_state);

        let mut entries: Vec<(String, Value)> = message
            .as_object()
            .map(|map| map.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        // Upstream sort: `state`/`brightness` first when the bulb goes off,
        // otherwise last (JS stable sort keeps the remaining order).
        let sorter = if message
            .get("state")
            .and_then(Value::as_str)
            .map(str::to_lowercase)
            == Some("off".to_string())
        {
            1
        } else {
            -1
        };
        let is_ordered = |key: &str| matches!(key, "state" | "brightness" | "brightness_percent");
        entries.sort_by_key(|(key, _)| {
            let selected = is_ordered(key);
            if sorter == 1 {
                if selected {
                    0
                } else {
                    1
                }
            } else if selected {
                1
            } else {
                0
            }
        });

        let endpoint_names = entity.endpoint_names();
        let endpoint_suffix = if endpoint_names.is_empty() {
            None
        } else {
            Some(format!(
                "^(.*?)_({})$",
                endpoint_names
                    .iter()
                    .map(|name| regex::escape(name))
                    .collect::<Vec<_>>()
                    .join("|")
            ))
        };

        let mut used_converters: std::collections::HashMap<String, Vec<usize>> =
            std::collections::HashMap::new();
        let mut to_publish: Map<String, Value> = Map::new();
        let mut to_publish_entity: std::collections::HashMap<String, Arc<dyn Entity>> =
            std::collections::HashMap::new();
        let mut scenes_changed = false;

        for (mut key, value) in entries {
            let mut endpoint_name = parsed.endpoint.clone();
            let mut local_target = target.clone();
            let mut target_id = match &local_target {
                ToTarget::Endpoint(endpoint) => endpoint.ID.to_string(),
                ToTarget::Group(group) => group.group_id.to_string(),
            };

            if entity.is_device() {
                if let Some(suffix) = &endpoint_suffix {
                    if let Ok(regex) = regex::Regex::new(suffix) {
                        if let Some(captures) = regex.captures(&key) {
                            endpoint_name = captures.get(2).map(|value| value.as_str().to_string());
                            key = captures
                                .get(1)
                                .map(|value| value.as_str().to_string())
                                .unwrap_or(key);
                            if let Some(endpoint) = entity.endpoint(endpoint_name.as_deref()) {
                                target_id = endpoint.ID.to_string();
                                local_target = ToTarget::Endpoint(endpoint);
                            }
                        }
                    }
                }
            }

            let converter_index = converters.iter().position(|converter| {
                converter_matches(
                    converter,
                    &key,
                    endpoint_name.as_deref(),
                    entity.is_device(),
                )
            });
            let Some(converter_index) = converter_index else {
                self.logger.lock().unwrap().error(
                    &format!(
                        "No converter available for '{key}' on '{}': ({})",
                        entity.name(),
                        stringify(message.get(&key).unwrap_or(&Value::Null))
                    ),
                    "z2m",
                );
                continue;
            };

            let used = used_converters.entry(target_id.clone()).or_default();
            if parsed.r#type == "set" && used.contains(&converter_index) {
                continue;
            }

            let converter = converters[converter_index].clone();
            let meta = ToMeta {
                endpoint_name: endpoint_name.clone(),
                options: entity.options(),
                message: message.clone(),
                device: entity.zh_device(),
                state: entity_state.clone(),
                mapped: definitions
                    .first()
                    .cloned()
                    .unwrap_or_else(|| Arc::new(zigbee_converters::Definition::new("", "", ""))),
                publish: Some(Arc::new({
                    let this = self.clone();
                    let entity = entity.clone();
                    move |payload: Value| -> zigbee_converters::ConverterFuture<()> {
                        let this = this.clone();
                        let entity = entity.clone();
                        Box::pin(async move {
                            (this.publish_entity_state)(entity, payload, None).await;
                        })
                    }
                })),
                device_exposes_changed: Some(Arc::new({
                    let this = self.clone();
                    move || {
                        this.event_bus
                            .lock()
                            .unwrap()
                            .emit_exposes_and_devices_changed()
                    }
                })),
            };

            let result = if parsed.r#type == "set" {
                match &converter.convert_set {
                    Some(convert) => {
                        self.logger.lock().unwrap().debug(
                            &format!("Publishing 'set' '{key}' to '{}'", entity.name()),
                            "z2m",
                        );
                        convert(local_target, key.clone(), value, meta).await
                    }
                    None => {
                        self.logger.lock().unwrap().error(
                            &format!(
                                "No converter available for 'set' '{key}' ({})",
                                stringify(&message[&key])
                            ),
                            "z2m",
                        );
                        continue;
                    }
                }
            } else {
                match &converter.convert_get {
                    Some(convert) => {
                        self.logger.lock().unwrap().debug(
                            &format!("Publishing get 'get' '{key}' to '{}'", entity.name()),
                            "z2m",
                        );
                        convert(local_target, key.clone(), value, meta).await
                    }
                    None => {
                        self.logger.lock().unwrap().error(
                            &format!(
                                "No converter available for 'get' '{key}' ({})",
                                stringify(&message[&key])
                            ),
                            "z2m",
                        );
                        continue;
                    }
                }
            };

            match result {
                Ok(result) => {
                    let optimistic = entity
                        .options()
                        .get("optimistic")
                        .and_then(Value::as_bool)
                        .unwrap_or(true);
                    if let Some(mut state) = result.state.filter(|_| optimistic) {
                        if let (Some(endpoint_name), Value::Object(map)) =
                            (&endpoint_name, &mut state)
                        {
                            let keys: Vec<String> = map.keys().cloned().collect();
                            for key in keys {
                                if let Some(value) = map.remove(&key) {
                                    map.insert(format!("{key}_{endpoint_name}"), value);
                                }
                            }
                        }
                        let mut payload = state;
                        utils::filter_properties(
                            entity
                                .options()
                                .get("filtered_optimistic")
                                .and_then(Value::as_array)
                                .map(|items| {
                                    items
                                        .iter()
                                        .filter_map(Value::as_str)
                                        .map(str::to_string)
                                        .collect::<Vec<_>>()
                                })
                                .as_deref(),
                            &mut payload,
                        );
                        merge_map(&mut to_publish, &entity.entity_id().key(), payload);
                        to_publish_entity.insert(entity.entity_id().key(), entity.clone());
                    }

                    if let Some(Value::Object(members)) =
                        result.members_state.filter(|_| optimistic)
                    {
                        for (ieee, state) in members {
                            if let Some(member) = self.zigbee.resolve_entity(&ieee) {
                                merge_map(&mut to_publish, &ieee, state);
                                to_publish_entity.insert(ieee, member);
                            }
                        }
                    }
                }
                Err(error) => {
                    self.logger.lock().unwrap().error(
                        &format!(
                            "Publish '{}' '{key}' to '{}' failed: '{error}'",
                            parsed.r#type,
                            entity.name()
                        ),
                        "z2m",
                    );
                }
            }

            used_converters
                .entry(target_id)
                .or_default()
                .push(converter_index);

            if !scenes_changed {
                scenes_changed = converter
                    .key
                    .iter()
                    .any(|key| SCENE_CONVERTER_KEYS.contains(&key.as_str()));
            }
        }

        for (id, payload) in to_publish {
            if !utils::object_is_empty(&payload) {
                if let Some(target) = to_publish_entity.get(&id).cloned() {
                    (self.publish_entity_state)(target, payload, None).await;
                }
            }
        }

        if scenes_changed {
            self.event_bus
                .lock()
                .unwrap()
                .emit_scenes_changed(json!({"entity": entity.event_value()}));
        }
    }
}

fn definition_converters(definitions: &[DefinitionRef]) -> Vec<ToZigbeeConverter> {
    let mut seen = HashSet::new();
    let mut converters = Vec::new();
    for definition in definitions {
        for converter in &definition.to_zigbee {
            let pointer = format!("{:p}", Arc::as_ptr(definition));
            if seen.insert((pointer, format!("{:p}", converter as *const _))) {
                converters.push(converter.clone());
            }
        }
    }
    converters
}

fn converter_matches(
    converter: &ToZigbeeConverter,
    key: &str,
    endpoint_name: Option<&str>,
    is_device: bool,
) -> bool {
    let key_matches = converter.key.is_empty() || converter.key.iter().any(|entry| entry == key);
    let endpoint_matches = !is_device
        || converter.endpoints.is_none()
        || endpoint_name.is_some_and(|name| {
            converter
                .endpoints
                .as_ref()
                .is_some_and(|endpoints| endpoints.iter().any(|endpoint| endpoint == name))
        });
    key_matches && endpoint_matches
}

fn merge_map(target: &mut Map<String, Value>, id: &str, payload: Value) {
    let entry = target.entry(id.to_string()).or_insert_with(|| json!({}));
    if let (Value::Object(target), Value::Object(payload)) = (entry, payload) {
        for (key, value) in payload {
            target.insert(key, value);
        }
    }
}
