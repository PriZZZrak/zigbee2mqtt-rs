// Original: .upstream/zigbee2mqtt/lib/controller.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// Lifecycle/extension wiring lands with the bridge extension; this slice ports
// the state-publishing pipeline (`publishEntityState`, controller.ts:382-490).

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::event_bus::EventBus;
use crate::extension::extension::Extension;
use crate::model::entity::Entity;
use crate::mqtt::{Mqtt, MqttPublishOptions};
use crate::state::State;
use crate::util::logger::Logger;
use crate::util::settings::Settings;
use crate::util::stringify::stringify;
use crate::util::utils;
use crate::zigbee::Zigbee;

pub struct Controller {
    pub event_bus: Arc<Mutex<EventBus>>,
    pub zigbee: Arc<Zigbee>,
    pub state: Arc<Mutex<State>>,
    pub mqtt: Arc<tokio::sync::Mutex<Mqtt>>,
    pub settings: Arc<Mutex<Settings>>,
    pub logger: Arc<Mutex<Logger>>,
    extensions: Mutex<Vec<Arc<dyn Extension>>>,
}

impl Controller {
    pub fn new(
        event_bus: Arc<Mutex<EventBus>>,
        zigbee: Arc<Zigbee>,
        state: Arc<Mutex<State>>,
        mqtt: Arc<tokio::sync::Mutex<Mqtt>>,
        settings: Arc<Mutex<Settings>>,
        logger: Arc<Mutex<Logger>>,
    ) -> Self {
        Self {
            event_bus,
            zigbee,
            state,
            mqtt,
            settings,
            logger,
            extensions: Mutex::new(Vec::new()),
        }
    }

    pub fn add_extension(&self, extension: Arc<dyn Extension>) {
        self.extensions.lock().unwrap().push(extension);
    }

    /// Upstream `publishEntityState` (`controller.ts:382-460`).
    pub async fn publish_entity_state(
        &self,
        entity: Arc<dyn Entity>,
        payload: Value,
        state_change_reason: Option<String>,
    ) {
        let applied = self.settings.lock().unwrap().get();
        let mut message = payload.clone();

        let new_state = self.state.lock().unwrap().set(
            entity.as_ref(),
            payload.clone(),
            state_change_reason.clone(),
        );

        if applied["advanced"]["cache_state"].as_bool().unwrap_or(true) {
            message = new_state;
        }

        let entity_options = entity.options();
        let mut options = MqttPublishOptions {
            retain: entity_options
                .get("retain")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            qos: entity_options
                .get("qos")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u8,
            is_entity_state: true,
            ..Default::default()
        };
        if let Some(retention) = entity_options.get("retention").and_then(Value::as_u64) {
            options.message_expiry_interval = Some(retention as u32);
        }

        if entity.is_device()
            && applied["mqtt"]["include_device_information"]
                .as_bool()
                .unwrap_or(false)
        {
            if let Some(device) = entity.device_info() {
                message["device"] = device;
            }
        }

        let last_seen = applied["advanced"]["last_seen"]
            .as_str()
            .unwrap_or("disable");
        if entity.is_device() && last_seen != "disable" {
            if let Some(timestamp) = entity.last_seen() {
                message["last_seen"] = json!(utils::format_date(timestamp as i64, last_seen));
            }
        }

        if entity.is_device() {
            if let Some(linkquality) = entity.linkquality() {
                message["linkquality"] = json!(linkquality);
            }
        }

        for extension in self.extensions.lock().unwrap().iter() {
            extension.adjust_message_before_publish(entity.as_ref(), &mut message);
        }

        utils::filter_properties(
            entity_options
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

        if !utils::object_is_empty(&message) {
            let output = applied["advanced"]["output"].as_str().unwrap_or("json");
            if output == "attribute_and_json" || output == "json" {
                self.mqtt
                    .lock()
                    .await
                    .publish(&entity.name(), stringify(&message), options.clone())
                    .await;
            }

            if output == "attribute_and_json" || output == "attribute" {
                self.iterate_payload_attribute_output(
                    &format!("{}/", entity.name()),
                    &message,
                    options.clone(),
                )
                .await;
            }
        }

        self.event_bus
            .lock()
            .unwrap()
            .emit_publish_entity_state(json!({
                "entity": entity.event_value(),
                "message": message,
                "stateChangeReason": state_change_reason,
                "payload": payload,
            }));
    }

    /// Upstream `iteratePayloadAttributeOutput` (`controller.ts:462-490`).
    pub async fn iterate_payload_attribute_output(
        &self,
        topic_root: &str,
        payload: &Value,
        options: MqttPublishOptions,
    ) {
        let mut messages = Vec::new();
        collect_attribute_messages(topic_root, payload, &mut messages);

        for (topic, message) in messages {
            self.mqtt
                .lock()
                .await
                .publish(&topic, message, options.clone())
                .await;
        }
    }
}

/// Upstream `iteratePayloadAttributeOutput` recursion (`controller.ts:462-490`).
fn collect_attribute_messages(
    topic_root: &str,
    payload: &Value,
    messages: &mut Vec<(String, String)>,
) {
    let Value::Object(entries) = payload else {
        return;
    };

    for (key, value) in entries {
        let mut sub_payload = value.clone();
        if key == "color" && utils::object_has_properties(&sub_payload, &["r", "g", "b"]) {
            sub_payload = json!([
                sub_payload["r"].clone(),
                sub_payload["g"].clone(),
                sub_payload["b"].clone()
            ]);
        }

        if sub_payload.is_null() {
            messages.push((format!("{topic_root}{key}"), String::new()));
        } else if let Value::Array(items) = &sub_payload {
            messages.push((
                format!("{topic_root}{key}"),
                items
                    .iter()
                    .map(|item| {
                        item.as_str()
                            .map(str::to_string)
                            .unwrap_or_else(|| stringify(item))
                    })
                    .collect::<Vec<_>>()
                    .join(","),
            ));
        } else if sub_payload.is_object() {
            collect_attribute_messages(&format!("{topic_root}{key}-"), &sub_payload, messages);
        } else if let Some(text) = sub_payload.as_str() {
            messages.push((format!("{topic_root}{key}"), text.to_string()));
        } else {
            messages.push((format!("{topic_root}{key}"), stringify(&sub_payload)));
        }
    }
}
