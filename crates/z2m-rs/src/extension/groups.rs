// Original: .upstream/zigbee2mqtt/lib/extension/groups.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// R1 devices have no endpoint names without converter definitions, so the
// endpoint-suffix branches are exercised only when a definition provides them.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use serde_json::{json, Map, Value};

use crate::extension::extension::{Extension, ExtensionContext, ExtensionFuture};
use crate::model::entity::Entity;
use crate::util::stringify::stringify;
use crate::util::utils;

/// Upstream `STATE_PROPERTIES` (`groups.ts:14-25`).
fn state_property(key: &str, value: &str, exposes: &[zigbee_converters::Expose]) -> bool {
    let light_feature = |name: &str| {
        exposes.iter().any(|expose| {
            zigbee_converters::exposes::is_light_expose(expose)
                && expose.feature_names().contains(&name)
        })
    };

    match key {
        "state" => true,
        "brightness" => light_feature("brightness"),
        "color_temp" => light_feature("color_temp"),
        "color" => light_feature("color_xy") || light_feature("color_hs"),
        "color_mode" => {
            light_feature(&format!("color_{value}"))
                || (value == "color_temp" && light_feature("color_temp"))
        }
        _ => false,
    }
}

struct ParsedMessage {
    r#type: String,
    resolved_group: Option<Arc<zigbee_apiarist::controller::model::group::Group>>,
    resolved_device: Option<Arc<crate::model::Device>>,
    resolved_endpoint: Option<Arc<zigbee_apiarist::controller::model::endpoint::Endpoint>>,
    group_key: Option<Value>,
    device_key: Option<Value>,
    endpoint_key: Option<Value>,
    skip_disable_reporting: bool,
    raw: Value,
}

pub struct Groups {
    context: ExtensionContext,
    topic_regex: regex::Regex,
    last_optimistic_state: Mutex<Map<String, Value>>,
}

impl Groups {
    pub fn new(context: ExtensionContext) -> Arc<Self> {
        let base = context.settings_value()["mqtt"]["base_topic"]
            .as_str()
            .unwrap_or("zigbee2mqtt")
            .to_string();
        let topic_regex = regex::Regex::new(&format!(
            "^{}/bridge/request/group/members/(remove|add|remove_all)$",
            regex::escape(&base)
        ))
        .expect("valid group members topic regex");
        Arc::new(Self {
            context,
            topic_regex,
            last_optimistic_state: Mutex::new(Map::new()),
        })
    }

    /// Upstream `onStateChange` (`groups.ts:48-150`).
    async fn on_state_change(self: &Arc<Self>, data: Value) {
        let reason = data.get("reason").and_then(Value::as_str).unwrap_or("");
        if reason == "groupOptimistic" || reason == "publishCached" {
            return;
        }

        let Some(entity_json) = data.get("entity") else {
            return;
        };
        let entity_key = entity_json
            .get("ieeeAddr")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                entity_json.get("ID").map(|id| {
                    id.as_str()
                        .map(str::to_string)
                        .unwrap_or_else(|| id.to_string())
                })
            });
        let Some(entity) = entity_key.and_then(|key| self.context.zigbee.resolve_entity(&key))
        else {
            return;
        };

        let update = data.get("update").cloned().unwrap_or_else(|| json!({}));
        let Value::Object(update) = update else {
            return;
        };

        let endpoint_names = entity.endpoint_names();
        let mut payload = Map::new();
        let mut endpoint_name: Option<String> = None;
        for (mut prop, value) in update {
            if let Some(name) = endpoint_names
                .iter()
                .find(|name| prop.ends_with(&format!("_{name}")))
                .cloned()
            {
                prop = prop[..prop.len() - name.len() - 1].to_string();
                endpoint_name = Some(name);
            }
            if matches!(
                prop.as_str(),
                "state" | "brightness" | "color_temp" | "color" | "color_mode"
            ) {
                payload.insert(prop, value);
            }
        }

        if payload.is_empty() {
            return;
        }

        let groups: Vec<Arc<crate::model::Group>> = self
            .context
            .zigbee
            .groups_iterator(|_| true)
            .into_iter()
            .filter(|group| {
                let options = group.options();
                options
                    .get("optimistic")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
            })
            .collect();

        if entity.is_device() {
            let Some(endpoint) = entity.endpoint(endpoint_name.as_deref()) else {
                return;
            };
            for group in groups {
                let last = self
                    .last_optimistic_state
                    .lock()
                    .unwrap()
                    .get(&group.id().to_string())
                    .cloned();
                if group.has_member_endpoint(&endpoint)
                    && last.as_ref() != Some(&Value::Object(payload.clone()))
                    && self
                        .should_publish_payload_for_group(&group, &Value::Object(payload.clone()))
                {
                    self.last_optimistic_state
                        .lock()
                        .unwrap()
                        .insert(group.id().to_string(), Value::Object(payload.clone()));
                    (self.context.publish_entity_state)(
                        group as Arc<dyn Entity>,
                        Value::Object(payload.clone()),
                        Some("groupOptimistic".to_string()),
                    )
                    .await;
                }
            }
        } else {
            self.last_optimistic_state
                .lock()
                .unwrap()
                .remove(&entity.entity_id().key());

            let mut groups_to_publish: Vec<Arc<crate::model::Group>> = Vec::new();
            let Some(zh_group) = entity.zh_group() else {
                return;
            };
            for member in zh_group.members() {
                let device = self
                    .context
                    .zigbee
                    .resolve_device(&member.get_device().ieee_addr());
                let Some(device) = device else { continue };
                if device
                    .options()
                    .get("disabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    continue;
                }

                let exposes = device.exposes().unwrap_or_default();
                let mut member_payload = Map::new();
                for (key, value) in &payload {
                    let value_str = value.as_str().unwrap_or_default();
                    if state_property(key, value_str, &exposes) {
                        member_payload.insert(key.clone(), value.clone());
                    }
                }

                if let Some(name) = device.endpoint_name(&member) {
                    let keys: Vec<String> = member_payload.keys().cloned().collect();
                    for key in keys {
                        if let Some(value) = member_payload.remove(&key) {
                            member_payload.insert(format!("{key}_{name}"), value);
                        }
                    }
                }

                (self.context.publish_entity_state)(
                    device as Arc<dyn Entity>,
                    Value::Object(member_payload),
                    Some("groupOptimistic".to_string()),
                )
                .await;

                for group in &groups {
                    if group.has_member_endpoint(&member)
                        && self.should_publish_payload_for_group(
                            group,
                            &Value::Object(payload.clone()),
                        )
                        && group.id() != entity.entity_id().key().parse::<u64>().unwrap_or(0)
                    {
                        groups_to_publish.push(group.clone());
                    }
                }
            }

            let mut seen = HashSet::new();
            for group in groups_to_publish {
                if seen.insert(group.id()) {
                    (self.context.publish_entity_state)(
                        group as Arc<dyn Entity>,
                        Value::Object(payload.clone()),
                        Some("groupOptimistic".to_string()),
                    )
                    .await;
                }
            }
        }
    }

    fn should_publish_payload_for_group(
        &self,
        group: &Arc<crate::model::Group>,
        payload: &Value,
    ) -> bool {
        group.options().get("off_state").and_then(Value::as_str) == Some("last_member_state")
            || payload.get("state").and_then(Value::as_str).is_none()
            || !matches!(
                payload.get("state").and_then(Value::as_str),
                Some("OFF" | "CLOSE")
            )
            || self.are_all_members_off_or_closed(group)
    }

    fn are_all_members_off_or_closed(&self, group: &Arc<crate::model::Group>) -> bool {
        for member in group.zh.members() {
            let Some(device) = self
                .context
                .zigbee
                .resolve_device(&member.get_device().ieee_addr())
            else {
                continue;
            };
            let id = device.entity_id();
            if self.context.state.lock().unwrap().exists(&id) {
                let state = self.context.state.lock().unwrap().get(&id);
                if matches!(
                    state.get("state").and_then(Value::as_str),
                    Some("ON" | "OPEN")
                ) {
                    return false;
                }
            }
        }
        true
    }

    fn group_name(&self, group: &Arc<zigbee_apiarist::controller::model::group::Group>) -> String {
        self.context
            .zigbee
            .group_by_id(group.group_id as u64)
            .map(|group| group.name())
            .unwrap_or_default()
    }

    /// Upstream `parseMQTTMessage` (`groups.ts:184-252`).
    fn parse_mqtt_message(&self, data: &Value) -> Option<(Value, Result<ParsedMessage, String>)> {
        let topic = data.get("topic").and_then(Value::as_str)?;
        let captures = self.topic_regex.captures(topic)?;
        let r#type = captures.get(1)?.as_str().to_string();
        let text = data.get("message").and_then(Value::as_str).unwrap_or("");
        let raw = utils::parse_json(text, text);

        let parsed = (|| -> Result<ParsedMessage, String> {
            if !raw.is_object() || raw.get("device").is_none() {
                return Err("Invalid payload".to_string());
            }
            let device_key = raw["device"].clone();
            let skip_disable_reporting = raw
                .get("skip_disable_reporting")
                .and_then(Value::as_bool)
                .unwrap_or(false);

            let mut group_key = None;
            let mut resolved_group = None;
            if r#type != "remove_all" {
                let group = raw
                    .get("group")
                    .ok_or_else(|| "Invalid payload".to_string())?;
                group_key = Some(group.clone());
                let key = group
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| group.to_string());
                let entity = self.context.zigbee.resolve_entity(&key);
                match entity.and_then(|entity| entity.zh_group()) {
                    Some(group) => resolved_group = Some(group),
                    None => {
                        return Err(format!("Group '{}' does not exist", key));
                    }
                }
            }

            let device_name = device_key
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| device_key.to_string());
            let resolved_device = self
                .context
                .zigbee
                .resolve_entity(&device_name)
                .and_then(|entity| {
                    entity
                        .zh_device()
                        .and_then(|zh| self.context.zigbee.resolve_device(&zh.ieee_addr()))
                })
                .ok_or_else(|| format!("Device '{}' does not exist", device_name))?;

            let endpoint_param = raw.get("endpoint").and_then(Value::as_str);
            let endpoint_key = raw
                .get("endpoint")
                .cloned()
                .unwrap_or_else(|| json!("default"));
            let resolved_endpoint = resolved_device.endpoint(endpoint_param).ok_or_else(|| {
                format!(
                    "Device '{}' does not have endpoint '{}'",
                    resolved_device.name(),
                    endpoint_key.as_str().unwrap_or("default")
                )
            })?;

            Ok(ParsedMessage {
                r#type: r#type.clone(),
                resolved_group,
                resolved_device: Some(resolved_device),
                resolved_endpoint: Some(resolved_endpoint),
                group_key,
                device_key: Some(device_key),
                endpoint_key: Some(endpoint_key),
                skip_disable_reporting,
                raw: raw.clone(),
            })
        })();

        Some((raw, parsed))
    }

    async fn publish_response(
        &self,
        r#type: &str,
        request: &Value,
        data: Value,
        error: Option<String>,
    ) {
        let response = utils::get_response(request, data, error.clone());
        self.context
            .mqtt
            .lock()
            .await
            .publish(
                &format!("bridge/response/group/members/{type}"),
                stringify(&response),
                Default::default(),
            )
            .await;
        if let Some(error) = error {
            self.context.logger.lock().unwrap().error(&error, "z2m");
        }
    }

    /// Upstream `onMQTTMessage` (`groups.ts:254-313`).
    async fn on_mqtt_message(self: &Arc<Self>, data: Value) {
        let Some((raw, parsed)) = self.parse_mqtt_message(&data) else {
            return;
        };
        let parsed = match parsed {
            Ok(parsed) => parsed,
            Err(error) => {
                let r#type = self
                    .topic_regex
                    .captures(data.get("topic").and_then(Value::as_str).unwrap_or(""))
                    .and_then(|captures| captures.get(1).map(|value| value.as_str().to_string()))
                    .unwrap_or_default();
                self.publish_response(&r#type, &raw, json!({}), Some(error))
                    .await;
                return;
            }
        };

        let device = parsed.resolved_device.clone().expect("checked above");
        let endpoint = parsed.resolved_endpoint.clone().expect("checked above");
        let mut changed_groups: Vec<Arc<zigbee_apiarist::controller::model::group::Group>> =
            Vec::new();

        let result = match parsed.r#type.as_str() {
            "add" => {
                let group = parsed.resolved_group.clone().expect("checked above");
                self.context.logger.lock().unwrap().info(
                    &format!(
                        "Adding endpoint '{}' of device '{}' to group '{}'",
                        endpoint.ID,
                        device.name(),
                        self.group_name(&group)
                    ),
                    "z2m",
                );
                let result = endpoint.add_to_group(&group).await;
                changed_groups.push(group);
                result
            }
            "remove" => {
                let group = parsed.resolved_group.clone().expect("checked above");
                self.context.logger.lock().unwrap().info(
                    &format!(
                        "Removing endpoint '{}' of device '{}' from group '{}'",
                        endpoint.ID,
                        device.name(),
                        self.group_name(&group)
                    ),
                    "z2m",
                );
                let result = endpoint.remove_from_group(&group).await;
                changed_groups.push(group);
                result
            }
            _ => {
                self.context.logger.lock().unwrap().info(
                    &format!(
                        "Removing endpoint '{}' of device '{}' from all groups",
                        endpoint.ID,
                        device.name()
                    ),
                    "z2m",
                );
                changed_groups = self
                    .context
                    .zigbee
                    .groups_iterator(|group| {
                        group
                            .members()
                            .iter()
                            .any(|member| Arc::ptr_eq(member, &endpoint))
                    })
                    .into_iter()
                    .map(|group| group.zh.clone())
                    .collect();
                endpoint.remove_from_all_groups().await
            }
        };

        match result {
            Ok(()) => {
                let response_data = match parsed.r#type.as_str() {
                    "remove_all" => json!({
                        "device": parsed.device_key,
                        "endpoint": parsed.endpoint_key,
                    }),
                    _ => json!({
                        "device": parsed.device_key,
                        "endpoint": parsed.endpoint_key,
                        "group": parsed.group_key,
                    }),
                };
                self.publish_response(&parsed.r#type, &raw, response_data, None)
                    .await;
            }
            Err(error) => {
                let preposition = if parsed.r#type == "add" { "to" } else { "from" };
                let error = format!(
                    "Failed to {} {} group ({error})",
                    parsed.r#type, preposition
                );
                self.publish_response(&parsed.r#type, &raw, json!({}), Some(error))
                    .await;
                return;
            }
        }

        for group in changed_groups {
            self.context.event_bus.lock().unwrap().emit_group_members_changed(json!({
                "group": {"ID": group.group_id, "name": self.group_name(&group)},
                "action": parsed.r#type,
                "endpoint": {"ID": endpoint.ID, "deviceIeeeAddress": endpoint.device_ieee_address.lock().unwrap().clone()},
                "skipDisableReporting": parsed.skip_disable_reporting,
            }));
        }
    }
}

impl Extension for Groups {
    fn name(&self) -> &'static str {
        "Groups"
    }

    fn context(&self) -> &ExtensionContext {
        &self.context
    }

    fn start(self: Arc<Self>) -> ExtensionFuture {
        Box::pin(async move {
            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_state_change(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move { this.on_state_change(data).await });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_mqtt_message(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move { this.on_mqtt_message(data).await });
                }),
            );

            Ok(())
        })
    }
}
