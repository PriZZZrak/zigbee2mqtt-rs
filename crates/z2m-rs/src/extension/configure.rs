// Original: .upstream/zigbee2mqtt/lib/extension/configure.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};
use zigbee_apiarist::controller::database::DeviceType;

use crate::extension::extension::{Extension, ExtensionContext, ExtensionFuture};
use crate::model::entity::Entity;
use crate::util::stringify::stringify;
use crate::util::utils;

pub struct Configure {
    context: ExtensionContext,
    configuring: Mutex<HashSet<String>>,
    attempts: Mutex<HashMap<String, u32>>,
    topic: String,
}

impl Configure {
    pub fn new(context: ExtensionContext) -> Arc<Self> {
        let topic = format!(
            "{}/bridge/request/device/configure",
            context.settings_value()["mqtt"]["base_topic"]
                .as_str()
                .unwrap_or("zigbee2mqtt")
        );
        Arc::new(Self {
            context,
            configuring: Mutex::new(HashSet::new()),
            attempts: Mutex::new(HashMap::new()),
            topic,
        })
    }

    /// Upstream `onMQTTMessage` (`configure.ts:28-56`).
    async fn on_mqtt_message(self: &Arc<Self>, data: Value) {
        if data.get("topic").and_then(Value::as_str) != Some(self.topic.as_str()) {
            return;
        }

        let text = data.get("message").and_then(Value::as_str).unwrap_or("");
        let message = utils::parse_json(text, text);
        let id = if message.is_object() {
            message.get("id").cloned()
        } else {
            Some(message.clone())
        };

        let mut error = None;
        if id.is_none() || id == Some(Value::Null) {
            error = Some("Invalid payload".to_string());
        } else if let Some(id) = &id {
            let key = id
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| id.to_string());
            let entity = self.context.zigbee.resolve_entity(&key);
            match entity {
                Some(entity) if entity.is_device() => {
                    entity.resolve_definition(false);
                    if entity
                        .definition()
                        .and_then(|definition| definition.configure.clone())
                        .is_none()
                    {
                        error = Some(format!("Device '{}' cannot be configured", entity.name()));
                    } else if let Err(configure_error) =
                        self.configure(entity, "mqtt_message", true, true).await
                    {
                        error = Some(format!("Failed to configure ({configure_error})"));
                    }
                }
                _ => error = Some(format!("Device '{key}' does not exist")),
            }
        }

        let response = utils::get_response(&message, json!({"id": id}), error);
        self.context
            .mqtt
            .lock()
            .await
            .publish(
                "bridge/response/device/configure",
                stringify(&response),
                Default::default(),
            )
            .await;
    }

    async fn clear_configured(&self, device: &Arc<dyn Entity>) {
        if let Some(zh) = device.zh_device() {
            let mut meta = zh.meta.lock().unwrap();
            let removed = meta
                .as_object_mut()
                .and_then(|map| map.remove("configured"))
                .is_some();
            drop(meta);
            if removed {
                zh.save(true);
            }
        }
    }

    /// Upstream `configure()` (`configure.ts:86-154`).
    async fn configure(
        self: &Arc<Self>,
        device: Arc<dyn Entity>,
        event: &str,
        force: bool,
        throw_error: bool,
    ) -> Result<(), String> {
        let Some(definition) = device.definition() else {
            return Ok(());
        };
        let Some(configure) = definition.configure.clone() else {
            return Ok(());
        };
        let version = definition.version.clone();

        if !force {
            let options = device.options();
            if options
                .get("disabled")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || !device.interviewed()
            {
                return Ok(());
            }

            let is_end_device = device
                .zh_device()
                .is_some_and(|zh| zh.r#type() == DeviceType::EndDevice);
            if is_end_device && event != "zigbee_event" {
                return Ok(());
            }

            let configured = device
                .zh_device()
                .and_then(|zh| zh.meta.lock().unwrap().get("configured").cloned());
            let should_reconfigure = configured.is_none()
                || (version != "0.0.0" && configured != Some(Value::String(version.clone())));
            if !should_reconfigure {
                return Ok(());
            }
        }

        let ieee = device.entity_id().key();
        if self.configuring.lock().unwrap().contains(&ieee) {
            return Ok(());
        }

        let attempts = self
            .attempts
            .lock()
            .unwrap()
            .get(&ieee)
            .copied()
            .unwrap_or(0);
        if attempts >= 3 && !force {
            return Ok(());
        }

        self.configuring.lock().unwrap().insert(ieee.clone());
        let name = device.name();
        self.context
            .logger
            .lock()
            .unwrap()
            .info(&format!("Configuring '{name}'"), "z2m");

        let result = match device.zh_device() {
            Some(zh) => {
                configure(
                    zh,
                    self.context.zigbee.first_coordinator_endpoint(),
                    definition.clone(),
                )
                .await
            }
            None => Ok(()),
        };

        match result {
            Ok(()) => {
                self.attempts.lock().unwrap().remove(&ieee);
                self.context.logger.lock().unwrap().info(
                    &format!("Successfully configured '{name}' (definition v{version})"),
                    "z2m",
                );
                if let Some(zh) = device.zh_device() {
                    {
                        let mut meta = zh.meta.lock().unwrap();
                        if !meta.is_object() {
                            *meta = json!({});
                        }
                        meta.as_object_mut()
                            .unwrap()
                            .insert("configured".to_string(), Value::String(version.clone()));
                    }
                    zh.save(true);
                }
                self.context
                    .event_bus
                    .lock()
                    .unwrap()
                    .emit_exposes_and_devices_changed();
                Ok(())
            }
            Err(error) => {
                let new_attempts = attempts + 1;
                self.attempts
                    .lock()
                    .unwrap()
                    .insert(ieee.clone(), new_attempts);
                self.context.logger.lock().unwrap().error(
                    &format!("Failed to configure '{name}', attempt {new_attempts} ({error})"),
                    "z2m",
                );
                if throw_error {
                    Err(error)
                } else {
                    Ok(())
                }
            }
        }
        .inspect(|_| {
            self.configuring.lock().unwrap().remove(&ieee);
        })
    }

    fn device_from(&self, data: &Value) -> Option<Arc<dyn Entity>> {
        let ieee = data
            .get("device")
            .and_then(|device| device.get("ieeeAddr"))
            .and_then(Value::as_str)?;
        self.context
            .zigbee
            .resolve_device(ieee)
            .map(|device| device as Arc<dyn Entity>)
    }
}

impl Extension for Configure {
    fn name(&self) -> &'static str {
        "Configure"
    }

    fn context(&self) -> &ExtensionContext {
        &self.context
    }

    fn start(self: Arc<Self>) -> ExtensionFuture {
        Box::pin(async move {
            // Upstream `setImmediate` + 10s delay between routers.
            let this = self.clone();
            tokio::spawn(async move {
                let routers = this
                    .context
                    .zigbee
                    .devices_iterator(|device| device.r#type() == DeviceType::Router);
                for device in routers {
                    tokio::time::sleep(Duration::from_secs(10)).await;
                    let _ = this.configure(device, "started", false, false).await;
                }
            });

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_device_joined(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(device) = this.device_from(&data) {
                            this.clear_configured(&device).await;
                            let _ = this.configure(device, "zigbee_event", false, false).await;
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_device_interview(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(device) = this.device_from(&data) {
                            let _ = this.configure(device, "zigbee_event", false, false).await;
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_last_seen_changed(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(device) = this.device_from(&data) {
                            let _ = this.configure(device, "zigbee_event", false, false).await;
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
                    tokio::spawn(async move { this.on_mqtt_message(data).await });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_reconfigure(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(device) = this.device_from(&data) {
                            this.clear_configured(&device).await;
                            let _ = this
                                .configure(device, "reporting_disabled", false, false)
                                .await;
                        }
                    });
                }),
            );

            Ok(())
        })
    }
}
