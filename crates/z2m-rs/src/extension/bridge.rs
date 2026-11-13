// Original: .upstream/zigbee2mqtt/lib/extension/bridge.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// OMITTED request handlers (AGENTS §5 Q5): touchlink/scan, touchlink/identify,
// touchlink/factory_reset, device/generate_external_definition and action.
// `bridge/request/backup` uses the `zip` crate with deflate level 6.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine as _;
use serde_json::{json, Map, Value};
use zigbee_apiarist::controller::database::DeviceType;
use zigbee_apiarist::controller::model::endpoint::{
    ConfigureReportingItem, Options, ReadReportConfigItem,
};

use crate::extension::extension::{Extension, ExtensionContext, ExtensionFuture};
use crate::model::entity::Entity;
use crate::mqtt::MqttPublishOptions;
use crate::util::stringify::stringify;
use crate::util::utils;
use crate::util::{data, settings_schema, system};

/// Upstream `DEFAULT_BIND_GROUP_ID` (`utils.ts:15`).
const DEFAULT_BIND_GROUP_ID: u64 = 901;

pub struct Bridge {
    context: ExtensionContext,
    request_regex: regex::Regex,
    os_info: Mutex<Value>,
    zigbee2mqtt_version: Mutex<Value>,
    coordinator_version: Mutex<Value>,
    restart_required: AtomicBool,
    last_joined_device_ieee_addr: Mutex<Option<String>>,
    last_bridge_logging_payload: Mutex<Option<String>>,
}

impl Bridge {
    pub fn new(context: ExtensionContext) -> Arc<Self> {
        let base = context.settings_value()["mqtt"]["base_topic"]
            .as_str()
            .unwrap_or("zigbee2mqtt")
            .to_string();
        Arc::new(Self {
            context,
            request_regex: regex::Regex::new(&format!(
                "{}/bridge/request/(.*)",
                regex::escape(&base)
            ))
            .expect("valid bridge request regex"),
            os_info: Mutex::new(Value::Null),
            zigbee2mqtt_version: Mutex::new(Value::Null),
            coordinator_version: Mutex::new(Value::Null),
            restart_required: AtomicBool::new(false),
            last_joined_device_ieee_addr: Mutex::new(None),
            last_bridge_logging_payload: Mutex::new(None),
        })
    }

    fn set_restart_required(&self, value: bool) {
        if value {
            self.restart_required.store(true, Ordering::SeqCst);
        }
    }

    /// Upstream `onMQTTMessage` (`bridge.ts:204-227`).
    async fn on_mqtt_message(self: &Arc<Self>, data: Value) {
        let topic = data.get("topic").and_then(Value::as_str).unwrap_or("");
        let Some(captures) = self.request_regex.captures(topic) else {
            return;
        };
        let Some(key_match) = captures.get(1) else {
            return;
        };
        let key = key_match.as_str().to_lowercase();
        let text = data.get("message").and_then(Value::as_str).unwrap_or("");
        let message = utils::parse_json(text, text);

        let response = match key.as_str() {
            "device/options" => self.change_entity_options("device", &message).await,
            "group/options" => self.change_entity_options("group", &message).await,
            "device/configure_reporting" | "device/reporting/configure" => {
                self.device_reporting_configure(&message).await
            }
            "device/reporting/read" => self.device_reporting_read(&message).await,
            "device/remove" => self.remove_entity("device", &message).await,
            "device/rename" => self.rename_entity("device", &message).await,
            "device/interview" => self.device_interview(&message).await,
            "group/add" => self.group_add(&message).await,
            "group/remove" => self.remove_entity("group", &message).await,
            "group/rename" => self.rename_entity("group", &message).await,
            "permit_join" => self.permit_join(&message).await,
            "restart" => self.restart(&message).await,
            "backup" => self.backup(&message).await,
            "install_code/add" => self.install_code_add(&message).await,
            "health_check" => Ok(utils::get_response(
                &message,
                json!({"healthy": true}),
                None,
            )),
            "coordinator_check" => self.coordinator_check(&message).await,
            "options" => self.bridge_options(&message).await,
            _ => return,
        };

        let response = match response {
            Ok(response) => response,
            Err(error) => {
                self.context.logger.lock().unwrap().error(
                    &format!("Request '{topic}' failed with error: '{error}'"),
                    "z2m",
                );
                utils::get_response(&message, json!({}), Some(error))
            }
        };

        self.context
            .mqtt
            .lock()
            .await
            .publish(
                &format!("bridge/response/{}", key_match.as_str()),
                stringify(&response),
                Default::default(),
            )
            .await;
    }

    /// Upstream `bridgeOptions` (`bridge.ts:241-276`).
    async fn bridge_options(&self, message: &Value) -> Result<Value, String> {
        if !message.is_object() || !message.get("options").is_some_and(Value::is_object) {
            return Err("Invalid payload".to_string());
        }
        let new_settings = message["options"].clone();
        let new_restart_required = {
            let mut settings = self.context.settings.lock().unwrap();
            settings
                .apply(new_settings.clone())
                .map_err(|error| error)?
        };
        self.set_restart_required(new_restart_required);

        if new_settings.get("homeassistant").is_some() {
            let enabled = self.context.settings.lock().unwrap().get()["homeassistant"]["enabled"]
                .as_bool()
                .unwrap_or(false);
            (self.context.enable_disable_extension)(enabled, "HomeAssistant".to_string()).await?;
        }

        if new_settings
            .get("advanced")
            .and_then(|advanced| advanced.get("log_level"))
            .is_some()
        {
            let level = self.context.settings.lock().unwrap().get()["advanced"]["log_level"]
                .as_str()
                .unwrap_or("info")
                .to_string();
            self.context.logger.lock().unwrap().set_level(&level);
        }

        if new_settings
            .get("advanced")
            .and_then(|advanced| advanced.get("log_namespaced_levels"))
            .is_some()
        {
            let levels = self.context.settings.lock().unwrap().get()["advanced"]
                ["log_namespaced_levels"]
                .as_object()
                .map(|levels| {
                    levels
                        .iter()
                        .filter_map(|(key, value)| {
                            value.as_str().map(|value| (key.clone(), value.to_string()))
                        })
                        .collect::<std::collections::BTreeMap<_, _>>()
                })
                .unwrap_or_default();
            self.context
                .logger
                .lock()
                .unwrap()
                .set_namespaced_levels(levels);
        }

        if new_settings
            .get("advanced")
            .and_then(|advanced| advanced.get("log_debug_namespace_ignore"))
            .is_some()
        {
            let ignore = self.context.settings.lock().unwrap().get()["advanced"]
                ["log_debug_namespace_ignore"]
                .as_str()
                .unwrap_or("")
                .to_string();
            self.context
                .logger
                .lock()
                .unwrap()
                .set_debug_namespace_ignore(&ignore);
        }

        self.context.logger.lock().unwrap().info(
            if new_restart_required {
                "Changes require restart to take effect"
            } else {
                "Successfully changed options"
            },
            "z2m",
        );
        self.publish_info().await;
        Ok(utils::get_response(
            message,
            json!({"restart_required": self.restart_required.load(Ordering::SeqCst)}),
            None,
        ))
    }

    /// Upstream `changeEntityOptions` (`bridge.ts:450-507`).
    async fn change_entity_options(
        &self,
        entity_type: &str,
        message: &Value,
    ) -> Result<Value, String> {
        if !message.is_object() || message.get("id").is_none() || message.get("options").is_none() {
            return Err("Invalid payload".to_string());
        }
        let id_value = message["id"].clone();
        let id = id_value
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| id_value.to_string());
        let entity = self.get_entity(entity_type, &id)?;

        if entity.is_device() {
            let supported: std::collections::HashSet<String> =
                settings_schema::definition_property_names("device")
                    .into_iter()
                    .chain(
                        entity
                            .definition()
                            .map(|definition| {
                                definition
                                    .options
                                    .iter()
                                    .filter_map(|option| {
                                        option.get("property").and_then(Value::as_str)
                                    })
                                    .map(str::to_string)
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default(),
                    )
                    .collect();
            if let Value::Object(options) = &message["options"] {
                for option in options.keys() {
                    if !supported.contains(option) {
                        self.context.logger.lock().unwrap().warning(
                            &format!("Device '{id}' does not support option '{option}'"),
                            "z2m",
                        );
                    }
                }
            }
        }

        let cleanup = |options: Value| -> Value {
            let mut options = options;
            if let Value::Object(map) = &mut options {
                for key in ["friendlyName", "friendly_name", "ID", "type", "devices"] {
                    map.remove(key);
                }
            }
            options
        };
        let old_options = cleanup(entity.options());

        let mut new_options = message["options"].clone();
        if let Some(icon) = new_options.get("icon").and_then(Value::as_str) {
            if let Some((extension, data)) = utils::match_base64_file(icon) {
                let file = utils::save_base64_device_icon(&extension, &data)?;
                new_options["icon"] = Value::String(file.clone());
                self.context
                    .logger
                    .lock()
                    .unwrap()
                    .debug(&format!("Saved base64 image as file to '{file}'"), "z2m");
            }
        }

        let restart = self
            .context
            .settings
            .lock()
            .unwrap()
            .change_entity_options_with_restart(entity_type, &id, new_options)
            .map_err(|error| error)?;
        self.set_restart_required(restart);
        let entity = self.get_entity(entity_type, &id)?;
        let new_options = cleanup(entity.options());
        self.publish_info().await;

        self.context.logger.lock().unwrap().info(
            &format!(
                "{} config for {entity_type} {id}",
                if restart {
                    "New config requires restart to take effect for"
                } else {
                    "Successfully changed"
                }
            ),
            "z2m",
        );

        self.context
            .event_bus
            .lock()
            .unwrap()
            .emit_entity_options_changed(json!({
                "from": old_options,
                "to": new_options,
                "entity": entity.event_value(),
            }));
        Ok(utils::get_response(
            message,
            json!({
                "from": old_options,
                "to": new_options,
                "id": id_value,
                "restart_required": self.restart_required.load(Ordering::SeqCst),
            }),
            None,
        ))
    }

    async fn device_reporting_configure(&self, message: &Value) -> Result<Value, String> {
        let valid = message.is_object()
            && message.get("id").is_some()
            && message.get("endpoint").is_some()
            && message.get("cluster").is_some()
            && message.get("attribute").is_some()
            && message
                .get("maximum_report_interval")
                .and_then(Value::as_i64)
                .is_some()
            && message
                .get("minimum_report_interval")
                .and_then(Value::as_i64)
                .is_some()
            && (message.get("reportable_change").is_none()
                || message
                    .get("reportable_change")
                    .and_then(Value::as_f64)
                    .is_some());
        if !valid {
            return Err("Invalid payload".to_string());
        }
        let id = message["id"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| message["id"].to_string());
        let device = self.get_entity("device", &id)?;
        let endpoint_name = message["endpoint"].as_str();
        let endpoint = device.endpoint(endpoint_name).ok_or_else(|| {
            format!(
                "Device '{id}' does not have endpoint '{}'",
                message["endpoint"]
            )
        })?;
        let coordinator = self
            .context
            .zigbee
            .first_coordinator_endpoint()
            .ok_or_else(|| "Coordinator not available".to_string())?;
        endpoint
            .bind(
                message["cluster"].clone(),
                zigbee_apiarist::controller::model::endpoint::BindTarget::Endpoint(coordinator),
            )
            .await?;
        endpoint
            .configure_reporting(
                message["cluster"].clone(),
                vec![ConfigureReportingItem {
                    attribute: message["attribute"].clone(),
                    minimum_report_interval: message["minimum_report_interval"]
                        .as_u64()
                        .unwrap_or(0) as u32,
                    maximum_report_interval: message["maximum_report_interval"]
                        .as_u64()
                        .unwrap_or(0) as u32,
                    reportable_change: message.get("reportable_change").and_then(Value::as_f64),
                }],
                None,
            )
            .await?;
        self.publish_devices().await;
        self.context.logger.lock().unwrap().info(
            &format!(
                "Configured reporting for '{}', '{}.{}'",
                message["id"], message["cluster"], message["attribute"]
            ),
            "z2m",
        );
        Ok(utils::get_response(
            message,
            json!({
                "id": message["id"],
                "endpoint": message["endpoint"],
                "cluster": message["cluster"],
                "maximum_report_interval": message["maximum_report_interval"],
                "minimum_report_interval": message["minimum_report_interval"],
                "reportable_change": message.get("reportable_change").cloned().unwrap_or(Value::Null),
                "attribute": message["attribute"],
            }),
            None,
        ))
    }

    async fn device_reporting_read(&self, message: &Value) -> Result<Value, String> {
        if !message.is_object()
            || message.get("id").is_none()
            || message.get("endpoint").is_none()
            || message.get("cluster").is_none()
            || message.get("configs").is_none()
        {
            return Err("Invalid payload".to_string());
        }
        let id = message["id"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| message["id"].to_string());
        let device = self.get_entity("device", &id)?;
        let endpoint = device
            .endpoint(message["endpoint"].as_str())
            .ok_or_else(|| {
                format!(
                    "Device '{id}' does not have endpoint '{}'",
                    message["endpoint"]
                )
            })?;
        let configs = message["configs"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .map(|item| ReadReportConfigItem {
                        direction: item
                            .get("direction")
                            .and_then(Value::as_u64)
                            .map(|value| value as u8),
                        attribute: item.get("attribute").cloned().unwrap_or(Value::Null),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let options = message
            .get("manufacturer_code")
            .and_then(Value::as_u64)
            .map(|code| Options {
                manufacturer_code: Some(code as u16),
                ..Default::default()
            });
        let response = endpoint
            .read_reporting_config(message["cluster"].clone(), configs, options)
            .await?;
        self.publish_devices().await;
        let mut response_data = json!({
            "id": message["id"],
            "endpoint": message["endpoint"],
            "cluster": message["cluster"],
            "configs": response,
        });
        if let Some(code) = message.get("manufacturer_code") {
            response_data["manufacturer_code"] = code.clone();
        }
        Ok(utils::get_response(message, response_data, None))
    }

    async fn device_interview(&self, message: &Value) -> Result<Value, String> {
        if !message.is_object() || message.get("id").is_none() {
            return Err("Invalid payload".to_string());
        }
        let id = message["id"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| message["id"].to_string());
        let device = self.get_entity("device", &id)?;
        let device = self
            .context
            .zigbee
            .resolve_device(&device.entity_id().key())
            .ok_or_else(|| format!("Device '{}' does not exist", id))?;
        device.re_interview(self.context.event_bus.clone()).await?;
        Ok(utils::get_response(
            message,
            json!({"id": message["id"]}),
            None,
        ))
    }

    async fn group_add(&self, message: &Value) -> Result<Value, String> {
        if message.is_object() && message.get("friendly_name").is_none() {
            return Err("Invalid payload".to_string());
        }
        let friendly_name = if message.is_object() {
            message["friendly_name"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        } else {
            message.as_str().unwrap_or_default().to_string()
        };
        let id = if message.is_object() {
            message.get("id").map(|id| {
                id.as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| id.to_string())
            })
        } else {
            None
        };
        let group_id = {
            let mut settings = self.context.settings.lock().unwrap();
            settings.add_group(&friendly_name, id.as_deref())
        };
        self.context.zigbee.create_group(group_id);
        self.publish_groups().await;
        let name = self
            .context
            .settings
            .lock()
            .unwrap()
            .get_group(&group_id.to_string())
            .and_then(|group| group.get("friendly_name").cloned())
            .unwrap_or(Value::Null);
        Ok(utils::get_response(
            message,
            json!({"friendly_name": name, "id": group_id}),
            None,
        ))
    }

    async fn rename_entity(
        self: &Arc<Self>,
        entity_type: &str,
        message: &Value,
    ) -> Result<Value, String> {
        let device_and_last =
            entity_type == "device" && message.get("last").and_then(Value::as_bool) == Some(true);
        if !message.is_object()
            || (message.get("from").is_none() && !device_and_last)
            || message.get("to").is_none()
        {
            return Err("Invalid payload".to_string());
        }
        if device_and_last && self.last_joined_device_ieee_addr.lock().unwrap().is_none() {
            return Err("No device has joined since start".to_string());
        }

        let from = if device_and_last {
            self.last_joined_device_ieee_addr
                .lock()
                .unwrap()
                .clone()
                .unwrap()
        } else {
            message["from"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| message["from"].to_string())
        };
        let to = message["to"]
            .as_str()
            .ok_or_else(|| "to is not a string".to_string())?
            .trim()
            .to_string();
        let homeassistant_rename = message
            .get("homeassistant_rename")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let entity = self.get_entity(entity_type, &from)?;
        let old_friendly_name = entity
            .options()
            .get("friendly_name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        self.context
            .settings
            .lock()
            .unwrap()
            .change_friendly_name(&from, &to)
            .map_err(|error| error)?;

        self.context
            .mqtt
            .lock()
            .await
            .publish(
                &old_friendly_name,
                "",
                MqttPublishOptions {
                    retain: true,
                    ..Default::default()
                },
            )
            .await;

        self.context
            .event_bus
            .lock()
            .unwrap()
            .emit_entity_renamed(json!({
                "entity": entity.event_value(),
                "homeAssisantRename": homeassistant_rename,
                "from": old_friendly_name,
                "to": to,
            }));

        if entity.is_device() {
            self.publish_devices().await;
        } else {
            self.publish_groups().await;
            self.publish_info().await;
        }

        (self.context.publish_entity_state)(entity, json!({}), None).await;

        Ok(utils::get_response(
            message,
            json!({
                "from": old_friendly_name,
                "to": to,
                "homeassistant_rename": homeassistant_rename,
            }),
            None,
        ))
    }

    async fn remove_entity(
        self: &Arc<Self>,
        entity_type: &str,
        message: &Value,
    ) -> Result<Value, String> {
        let message_is_object = message.is_object();
        let id_value = if message_is_object {
            message.get("id").cloned().unwrap_or(Value::Null)
        } else {
            Value::String(message.as_str().unwrap_or_default().trim().to_string())
        };
        let id = id_value
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| id_value.to_string());
        let entity = self.get_entity(entity_type, &id)?;
        let friendly_name = entity.name();

        let mut block = false;
        let mut force = false;
        let mut keep_config = false;
        let mut clear_cache = false;
        if entity_type == "device" && message_is_object {
            block = message
                .get("block")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            force = message
                .get("force")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            keep_config = message
                .get("keep_config")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            clear_cache = message
                .get("clear_cache")
                .and_then(Value::as_bool)
                .unwrap_or(false);
        } else if entity_type == "group" && message_is_object {
            force = message
                .get("force")
                .and_then(Value::as_bool)
                .unwrap_or(false);
        }
        let block_force_log = if entity_type == "device" {
            format!(" (block: {block}, force: {force}, keep config: {keep_config}, clear cache: {clear_cache})")
        } else {
            format!(" (force: {force})")
        };

        self.context.logger.lock().unwrap().info(
            &format!("Removing {entity_type} '{friendly_name}'{block_force_log}"),
            "z2m",
        );

        if entity.is_device() {
            let device = self
                .context
                .zigbee
                .resolve_device(&entity.entity_id().key())
                .ok_or_else(|| format!("Device '{id}' does not exist"))?;
            if block {
                self.context
                    .settings
                    .lock()
                    .unwrap()
                    .block_device(&device.ieee_addr());
            }
            if force {
                device.zh.remove_from_database(clear_cache);
            } else {
                device
                    .zh
                    .remove_from_network(clear_cache)
                    .await
                    .map_err(|error| {
                        format!(
                            "Failed to remove device '{friendly_name}'{block_force_log} ({error})"
                        )
                    })?;
            }
            if clear_cache {
                self.context
                    .zigbee
                    .remove_device_from_lookup(&device.ieee_addr());
            }
            if !keep_config {
                self.context
                    .settings
                    .lock()
                    .unwrap()
                    .remove_device(&device.ieee_addr())
                    .map_err(|error| error)?;
            }
        } else {
            let group = self
                .context
                .zigbee
                .group_by_id(entity.entity_id().key().parse::<u64>().unwrap_or(0))
                .ok_or_else(|| format!("Group '{id}' does not exist"))?;
            if force {
                group.zh.remove_from_database();
            } else {
                group.zh.remove_from_network().await.map_err(|error| {
                    format!("Failed to remove group '{friendly_name}'{block_force_log} ({error})")
                })?;
            }
            self.context.zigbee.remove_group_from_lookup(group.id());
            self.context
                .settings
                .lock()
                .unwrap()
                .remove_group(&id)
                .map_err(|error| error)?;
        }

        self.context
            .event_bus
            .lock()
            .unwrap()
            .emit_entity_removed(json!({"entity": entity.event_value(), "name": friendly_name}));

        self.context
            .state
            .lock()
            .unwrap()
            .remove(&entity.entity_id());

        self.context
            .mqtt
            .lock()
            .await
            .publish(
                &friendly_name,
                "",
                MqttPublishOptions {
                    retain: true,
                    ..Default::default()
                },
            )
            .await;

        self.context.logger.lock().unwrap().info(
            &format!("Successfully removed {entity_type} '{friendly_name}'{block_force_log}"),
            "z2m",
        );
        self.publish_groups().await;

        if entity.is_device() {
            self.publish_devices().await;
            self.publish_definitions().await;
            return Ok(utils::get_response(
                message,
                json!({
                    "id": id_value,
                    "block": block,
                    "force": force,
                    "keep_config": keep_config,
                    "clear_cache": clear_cache,
                }),
                None,
            ));
        }

        Ok(utils::get_response(
            message,
            json!({"id": id_value, "force": force}),
            None,
        ))
    }

    async fn permit_join(&self, message: &Value) -> Result<Value, String> {
        let (time, device) = if message.is_object() {
            let Some(time) = message.get("time") else {
                return Err("Invalid payload".to_string());
            };
            let time = time
                .as_u64()
                .or_else(|| time.as_str().and_then(|text| text.parse::<u64>().ok()))
                .ok_or_else(|| "Invalid payload".to_string())?;
            let device = if let Some(device) = message.get("device") {
                let key = device
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| device.to_string());
                let resolved = self.context.zigbee.resolve_entity(&key);
                let resolved = resolved
                    .and_then(|entity| {
                        entity
                            .zh_device()
                            .and_then(|zh| self.context.zigbee.resolve_device(&zh.ieee_addr()))
                    })
                    .ok_or_else(|| format!("Device '{key}' does not exist"))?;
                Some(resolved)
            } else {
                None
            };
            (time, device)
        } else {
            let time = message
                .as_str()
                .and_then(|text| text.parse::<u64>().ok())
                .unwrap_or(0);
            (time, None)
        };

        self.context
            .zigbee
            .permit_join(time as u16, device.as_ref())
            .await?;
        let mut response = json!({"time": time});
        if let Some(device) = device {
            response["device"] = Value::String(device.name());
        }
        Ok(utils::get_response(message, response, None))
    }

    async fn restart(&self, message: &Value) -> Result<Value, String> {
        let restart = self.context.restart.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            restart().await;
        });
        self.context
            .logger
            .lock()
            .unwrap()
            .info("Restarting Zigbee2MQTT", "z2m");
        Ok(utils::get_response(message, json!({}), None))
    }

    async fn backup(&self, message: &Value) -> Result<Value, String> {
        self.context.zigbee.backup().await?;
        let data_path = data::get_path();
        let files = utils::get_all_files(&data_path);
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .compression_level(Some(6));
            for file in files {
                let name = file
                    .strip_prefix(&data_path)
                    .map(|path| path.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();
                if name.starts_with("log/") || name.starts_with("ota/") {
                    continue;
                }
                let bytes = std::fs::read(&file).map_err(|error| error.to_string())?;
                writer
                    .start_file(name, options)
                    .map_err(|error| error.to_string())?;
                use std::io::Write as _;
                writer
                    .write_all(&bytes)
                    .map_err(|error| error.to_string())?;
            }
            writer.finish().map_err(|error| error.to_string())?;
        }
        let zip = base64::engine::general_purpose::STANDARD.encode(buffer.into_inner());
        Ok(utils::get_response(message, json!({"zip": zip}), None))
    }

    async fn install_code_add(&self, message: &Value) -> Result<Value, String> {
        if message.is_object() && message.get("value").is_none() {
            return Err("Invalid payload".to_string());
        }
        let value = if message.is_object() {
            message["value"].clone()
        } else {
            message.clone()
        };
        let install_code = value.as_str().unwrap_or_default();
        self.context.zigbee.add_install_code(install_code).await?;
        self.context
            .logger
            .lock()
            .unwrap()
            .info("Successfully added new install code", "z2m");
        Ok(utils::get_response(message, json!({"value": value}), None))
    }

    async fn coordinator_check(&self, message: &Value) -> Result<Value, String> {
        let missing = self.context.zigbee.coordinator_check().await?;
        let missing_routers: Vec<Value> = missing
            .iter()
            .map(|device| {
                json!({"ieee_address": device.ieee_addr(), "friendly_name": device.name()})
            })
            .collect();
        Ok(utils::get_response(
            message,
            json!({"missing_routers": missing_routers}),
            None,
        ))
    }

    fn get_entity(&self, entity_type: &str, id: &str) -> Result<Arc<dyn Entity>, String> {
        let entity =
            self.context.zigbee.resolve_entity(id).ok_or_else(|| {
                format!("{} '{id}' does not exist", utils::capitalize(entity_type))
            })?;
        if entity.is_device() != (entity_type == "device") {
            return Err(format!(
                "{} '{id}' does not exist",
                utils::capitalize(entity_type)
            ));
        }
        Ok(entity)
    }

    /// Upstream `publishInfo` (`bridge.ts:794-827`).
    pub async fn publish_info(&self) {
        let mut config = self.context.settings.lock().unwrap().get();
        if let Value::Object(advanced) = &mut config["advanced"] {
            advanced.remove("network_key");
        }
        if let Value::Object(mqtt) = &mut config["mqtt"] {
            mqtt.remove("password");
        }
        if let Value::Object(frontend) = &mut config["frontend"] {
            frontend.remove("auth_token");
        }

        let network = self
            .context
            .zigbee
            .get_network_parameters()
            .await
            .unwrap_or_else(|_| json!({}));
        let coordinator_ieee = self
            .context
            .zigbee
            .first_coordinator_endpoint()
            .map(|endpoint| endpoint.device_ieee_address.lock().unwrap().clone())
            .unwrap_or_default();
        let mut coordinator = self.coordinator_version.lock().unwrap().clone();
        if !coordinator.is_object() {
            coordinator = json!({});
        }
        coordinator["ieee_address"] = Value::String(coordinator_ieee);

        let payload = json!({
            "os": self.os_info.lock().unwrap().clone(),
            "mqtt": self.context.mqtt.lock().await.info(),
            "version": self.zigbee2mqtt_version.lock().unwrap()["version"].clone(),
            "commit": self.zigbee2mqtt_version.lock().unwrap().get("commitHash").cloned().unwrap_or(Value::Null),
            "zigbee_herdsman_converters": {"version": zigbee_converters::VERSION},
            "zigbee_herdsman": {"version": zigbee_apiarist::VERSION},
            "coordinator": coordinator,
            "network": {
                "pan_id": network.get("panID").cloned().unwrap_or(Value::Null),
                "extended_pan_id": network.get("extendedPanID").cloned().unwrap_or(Value::Null),
                "channel": network.get("channel").cloned().unwrap_or(Value::Null),
            },
            "log_level": self.context.logger.lock().unwrap().get_level(),
            "permit_join": self.context.zigbee.get_permit_join(),
            "permit_join_end": self.context.zigbee.get_permit_join_end(),
            "restart_required": self.restart_required.load(Ordering::SeqCst),
            "config": config,
            "config_schema": settings_schema::schema(),
        });

        self.context
            .mqtt
            .lock()
            .await
            .publish(
                "bridge/info",
                stringify(&payload),
                MqttPublishOptions {
                    retain: true,
                    skip_log: true,
                    ..Default::default()
                },
            )
            .await;
    }

    /// Upstream `publishDevices` (`bridge.ts:829-890`).
    pub async fn publish_devices(&self) {
        let mut devices = Vec::new();
        for device in self.context.zigbee.devices_iterator(|_| true) {
            let mut endpoints = Map::new();
            for endpoint in device.zh.endpoints() {
                let mut bindings = Vec::new();
                for bind in endpoint.binds() {
                    let target = match &bind.target {
                        zigbee_apiarist::controller::model::endpoint::BindTarget::Endpoint(
                            endpoint,
                        ) => json!({
                            "type": "endpoint",
                            "ieee_address": endpoint.device_ieee_address.lock().unwrap().clone(),
                            "endpoint": endpoint.ID,
                        }),
                        zigbee_apiarist::controller::model::endpoint::BindTarget::Group(group) => {
                            json!({"type": "group", "id": group.group_id})
                        }
                        zigbee_apiarist::controller::model::endpoint::BindTarget::GroupId(id) => {
                            json!({"type": "group", "id": id})
                        }
                    };
                    bindings.push(json!({"cluster": bind.cluster.name, "target": target}));
                }

                let mut configured_reportings = Vec::new();
                for reporting in endpoint.configured_reportings() {
                    let attribute = reporting
                        .attribute
                        .get("name")
                        .cloned()
                        .or_else(|| reporting.attribute.get("ID").cloned())
                        .unwrap_or(Value::Null);
                    configured_reportings.push(json!({
                        "cluster": reporting.cluster.name,
                        "attribute": attribute,
                        "minimum_report_interval": reporting.minimum_report_interval,
                        "maximum_report_interval": reporting.maximum_report_interval,
                        "reportable_change": reporting.reportable_change,
                    }));
                }

                endpoints.insert(
                    endpoint.ID.to_string(),
                    json!({
                        "name": device.endpoint_name(&endpoint),
                        "scenes": utils::get_scenes_endpoint(&endpoint),
                        "bindings": bindings,
                        "configured_reportings": configured_reportings,
                        "clusters": {
                            "input": endpoint.get_input_clusters().iter().map(|cluster| cluster.name.clone()).collect::<Vec<_>>(),
                            "output": endpoint.get_output_clusters().iter().map(|cluster| cluster.name.clone()).collect::<Vec<_>>(),
                        },
                    }),
                );
            }

            devices.push(json!({
                "ieee_address": device.ieee_addr(),
                "type": device.zh.r#type().as_str(),
                "network_address": device.zh.network_address(),
                "supported": device.is_supported(),
                "friendly_name": device.name(),
                "disabled": device.options().get("disabled").and_then(Value::as_bool).unwrap_or(false),
                "description": device.options().get("description").cloned().unwrap_or(Value::Null),
                "definition": self.get_definition_payload(&device),
                "power_source": device.zh.power_source(),
                "software_build_id": device.zh.software_build_id(),
                "date_code": device.zh.date_code(),
                "model_id": device.zh.model_id(),
                "interviewing": device.zh.interview_state() == zigbee_apiarist::controller::model::device::InterviewState::InProgress,
                "interview_completed": device.zh.interview_state() == zigbee_apiarist::controller::model::device::InterviewState::Successful,
                "interview_state": device.zh.interview_state().as_str(),
                "manufacturer": device.zh.manufacturer_name(),
                "endpoints": Value::Object(endpoints),
            }));
        }

        self.context
            .mqtt
            .lock()
            .await
            .publish(
                "bridge/devices",
                stringify(&Value::Array(devices)),
                MqttPublishOptions {
                    retain: true,
                    skip_log: true,
                    ..Default::default()
                },
            )
            .await;
    }

    /// Upstream `publishGroups` (`bridge.ts:892-912`).
    pub async fn publish_groups(&self) {
        let mut groups = Vec::new();
        for group in self.context.zigbee.groups_iterator(|_| true) {
            let members: Vec<Value> = group
                .zh
                .members()
                .iter()
                .map(|member| {
                    json!({
                        "ieee_address": member.device_ieee_address.lock().unwrap().clone(),
                        "endpoint": member.ID,
                    })
                })
                .collect();
            groups.push(json!({
                "id": group.id(),
                "friendly_name": if group.id() == DEFAULT_BIND_GROUP_ID { "default_bind_group".to_string() } else { group.name() },
                "description": group.options().get("description").cloned().unwrap_or(Value::Null),
                "scenes": utils::get_scenes_group(&group.zh),
                "members": members,
            }));
        }

        self.context
            .mqtt
            .lock()
            .await
            .publish(
                "bridge/groups",
                stringify(&Value::Array(groups)),
                MqttPublishOptions {
                    retain: true,
                    skip_log: true,
                    ..Default::default()
                },
            )
            .await;
    }

    /// Upstream `publishDefinitions` (`bridge.ts:914-926`).
    pub async fn publish_definitions(&self) {
        let mut custom_clusters = Map::new();
        for device in self
            .context
            .zigbee
            .devices_iterator(|device| device.custom_clusters().len() > 0)
        {
            custom_clusters.insert(device.ieee_addr(), json!({}));
        }

        let payload = json!({
            "clusters": zigbee_apiarist::zspec::zcl::utils::clusters_json(),
            "custom_clusters": custom_clusters,
            "actions": [],
        });

        self.context
            .mqtt
            .lock()
            .await
            .publish(
                "bridge/definitions",
                stringify(&payload),
                MqttPublishOptions {
                    retain: true,
                    skip_log: true,
                    ..Default::default()
                },
            )
            .await;
    }

    /// Upstream `getDefinitionPayload` (`bridge.ts:928-956`).
    pub fn get_definition_payload(&self, device: &Arc<crate::model::Device>) -> Value {
        let Some(definition) = device.definition() else {
            return Value::Null;
        };

        let mut icon = device
            .options()
            .get("icon")
            .and_then(Value::as_str)
            .map(str::to_string);
        if let Some(value) = &icon {
            let replaced = value
                .replace(
                    "$zigbeeModel",
                    &utils::sanitize_image_parameter(device.zh.model_id().as_deref().unwrap_or("")),
                )
                .replace(
                    "$model",
                    &utils::sanitize_image_parameter(&definition.model),
                );
            icon = Some(replaced);
        }

        json!({
            "source": if definition.generated { "generated" } else { "native" },
            "model": definition.model,
            "vendor": definition.vendor,
            "description": definition.description,
            "exposes": device.exposes().unwrap_or_default().iter().map(zigbee_converters::Expose::to_json).collect::<Vec<_>>(),
            "supports_ota": definition.ota.is_some(),
            "options": definition.options,
            "version": definition.version,
            "icon": icon,
        })
    }
}

impl Extension for Bridge {
    fn name(&self) -> &'static str {
        "Bridge"
    }

    fn context(&self) -> &ExtensionContext {
        &self.context
    }

    fn start(self: Arc<Self>) -> ExtensionFuture {
        Box::pin(async move {
            let settings_value = self.context.settings_value();
            let debug_to_frontend = settings_value["advanced"]["log_debug_to_mqtt_frontend"]
                .as_bool()
                .unwrap_or(false);

            let mqtt = self.context.mqtt.clone();
            let last_payload = Arc::new(Mutex::new(None::<String>));
            let last_payload_for_listener = last_payload.clone();
            self.context
                .logger
                .lock()
                .unwrap()
                .set_log_listener(Arc::new(move |level, message, namespace| {
                    if !debug_to_frontend && level == "debug" {
                        return;
                    }
                    let payload = stringify(
                        &json!({"message": message, "level": level, "namespace": namespace}),
                    );
                    {
                        let mut last = last_payload_for_listener.lock().unwrap();
                        if last.as_deref() == Some(payload.as_str()) {
                            return;
                        }
                        *last = Some(payload.clone());
                    }
                    let mqtt = mqtt.clone();
                    tokio::spawn(async move {
                        mqtt.lock()
                            .await
                            .publish(
                                "bridge/logging",
                                payload,
                                MqttPublishOptions {
                                    skip_log: true,
                                    ..Default::default()
                                },
                            )
                            .await;
                    });
                }));

            let (cpu_models, cpu_count) = system::cpu_summary();
            *self.os_info.lock().unwrap() = json!({
                "version": system::os_version(),
                "node_version": "rust",
                "cpus": format!("{cpu_models} (x{cpu_count})"),
                "memory_mb": system::total_memory_mb(),
            });
            *self.zigbee2mqtt_version.lock().unwrap() = utils::get_zigbee2mqtt_version().await;
            *self.coordinator_version.lock().unwrap() = self
                .context
                .zigbee
                .get_coordinator_version()
                .await
                .unwrap_or(Value::Null);

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_entity_renamed(
                self.context.key,
                Arc::new(move |_| {
                    let this = this.clone();
                    tokio::spawn(async move { this.publish_info().await });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_group_members_changed(
                self.context.key,
                Arc::new(move |_| {
                    let this = this.clone();
                    tokio::spawn(async move { this.publish_groups().await });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_devices_changed(
                self.context.key,
                Arc::new(move |_| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        this.publish_devices().await;
                        this.publish_info().await;
                        this.publish_definitions().await;
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_permit_join_changed(
                self.context.key,
                Arc::new(move |_| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        if !this.context.zigbee.is_stopping() {
                            this.publish_info().await;
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_scenes_changed(
                self.context.key,
                Arc::new(move |_| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        this.publish_devices().await;
                        this.publish_groups().await;
                    });
                }),
            );

            // Zigbee events.
            for event in [
                "joined",
                "leave",
                "announce",
                "networkAddressChanged",
                "interview",
            ] {
                let bus = self.context.event_bus.clone();
                let this = self.clone();
                let key = this.context.key;
                let event_name = event.to_string();
                let callback: crate::event_bus::Listener = Arc::new(move |data| {
                    let this = this.clone();
                    let event_name = event_name.clone();
                    tokio::spawn(async move {
                        this.on_zigbee_event(&event_name, data).await;
                    });
                });
                let mut bus = bus.lock().unwrap();
                match event {
                    "joined" => bus.on_device_joined(key, callback),
                    "leave" => bus.on_device_leave(key, callback),
                    "announce" => bus.on_device_announce(key, callback),
                    "networkAddressChanged" => bus.on_device_network_address_changed(key, callback),
                    "interview" => bus.on_device_interview(key, callback),
                    _ => {}
                }
            }

            self.publish_info().await;
            self.publish_devices().await;
            self.publish_groups().await;
            self.publish_definitions().await;

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

    fn stop(self: Arc<Self>) -> ExtensionFuture {
        let logger = self.context.logger.clone();
        let event_bus = self.context.event_bus.clone();
        let key = self.context.key;
        Box::pin(async move {
            logger.lock().unwrap().clear_log_listener();
            event_bus.lock().unwrap().remove_listeners(key);
            Ok(())
        })
    }
}

impl Bridge {
    /// `bridge/event` payloads (`bridge.ts:132-189`).
    async fn on_zigbee_event(self: &Arc<Self>, event: &str, data: Value) {
        let ieee = data
            .get("device")
            .and_then(|device| device.get("ieeeAddr"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                data.get("ieeeAddr")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            });
        let device = ieee
            .as_deref()
            .and_then(|ieee| self.context.zigbee.resolve_device(ieee));

        match event {
            "joined" => {
                if let Some(device) = &device {
                    *self.last_joined_device_ieee_addr.lock().unwrap() = Some(device.ieee_addr());
                }
                self.publish_devices().await;
                let payload = json!({
                    "type": "device_joined",
                    "data": {
                        "friendly_name": device.as_ref().map(|device| device.name()),
                        "ieee_address": ieee,
                    },
                });
                self.context
                    .mqtt
                    .lock()
                    .await
                    .publish("bridge/event", stringify(&payload), Default::default())
                    .await;
            }
            "leave" => {
                self.publish_groups().await;
                self.publish_devices().await;
                self.publish_definitions().await;
                let payload = json!({
                    "type": "device_leave",
                    "data": {
                        "ieee_address": ieee,
                        "friendly_name": data.get("name").cloned().unwrap_or(Value::Null),
                    },
                });
                self.context
                    .mqtt
                    .lock()
                    .await
                    .publish("bridge/event", stringify(&payload), Default::default())
                    .await;
            }
            "networkAddressChanged" => self.publish_devices().await,
            "interview" => {
                self.publish_devices().await;
                let status = data
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("started");
                let payload = if status == "successful" {
                    json!({
                        "type": "device_interview",
                        "data": {
                            "friendly_name": device.as_ref().map(|device| device.name()),
                            "status": status,
                            "ieee_address": ieee,
                            "supported": device.as_ref().map(|device| device.is_supported()).unwrap_or(false),
                            "definition": device.as_ref().map(|device| self.get_definition_payload(device)).unwrap_or(Value::Null),
                        },
                    })
                } else {
                    json!({
                        "type": "device_interview",
                        "data": {
                            "friendly_name": device.as_ref().map(|device| device.name()),
                            "status": status,
                            "ieee_address": ieee,
                        },
                    })
                };
                self.context
                    .mqtt
                    .lock()
                    .await
                    .publish("bridge/event", stringify(&payload), Default::default())
                    .await;
            }
            "announce" => {
                self.publish_devices().await;
                let payload = json!({
                    "type": "device_announce",
                    "data": {
                        "friendly_name": device.as_ref().map(|device| device.name()),
                        "ieee_address": ieee,
                    },
                });
                self.context
                    .mqtt
                    .lock()
                    .await
                    .publish("bridge/event", stringify(&payload), Default::default())
                    .await;
            }
            _ => {}
        }
    }
}
