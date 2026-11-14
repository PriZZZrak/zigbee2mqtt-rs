// Original: .upstream/zigbee2mqtt/lib/controller.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// Lifecycle/extension wiring lands with the bridge extension; this slice ports
// the state-publishing pipeline (`publishEntityState`, controller.ts:382-490).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::event_bus::EventBus;
use crate::extension::extension::{Extension, ExtensionContext};
use crate::extension::{
    availability::Availability, bind::Bind, bridge::Bridge, configure::Configure,
    frontend::Frontend, groups::Groups, health::Health, homeassistant::HomeAssistant,
    network_map::NetworkMap, on_event::OnEvent, publish::Publish, receive::Receive,
};
use crate::util::abort::AbortSignal;
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
    next_extension_key: AtomicUsize,
    abort: Mutex<Option<AbortSignal>>,
    restart_callback: Mutex<Option<Arc<dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync>>>,
    exit_callback: Mutex<Option<Arc<dyn Fn(i32, bool) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync>>>,
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
            next_extension_key: AtomicUsize::new(1),
            abort: Mutex::new(None),
            restart_callback: Mutex::new(None),
            exit_callback: Mutex::new(None),
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

impl Controller {
    pub fn set_restart_callback(
        &self,
        callback: Arc<
            dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
                + Send
                + Sync,
        >,
    ) {
        *self.restart_callback.lock().unwrap() = Some(callback);
    }

    pub fn set_exit_callback(
        &self,
        callback: Arc<
            dyn Fn(i32, bool) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
                + Send
                + Sync,
        >,
    ) {
        *self.exit_callback.lock().unwrap() = Some(callback);
    }

    fn extension_context(self: &Arc<Self>) -> ExtensionContext {
        let key = self.next_extension_key.fetch_add(1, Ordering::SeqCst);
        let weak = Arc::downgrade(self);
        let enable_weak = weak.clone();
        let enable_disable_extension = Arc::new(move |enable: bool, name: String| {
            let weak = enable_weak.clone();
            Box::pin(async move {
                match weak.upgrade() {
                    Some(controller) => controller.enable_disable_extension(enable, name).await,
                    None => Err("Controller dropped".to_string()),
                }
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>>
        });
        let restart_weak = weak.clone();
        let restart = Arc::new(move || {
            let weak = restart_weak.clone();
            Box::pin(async move {
                if let Some(controller) = weak.upgrade() {
                    let callback = controller.restart_callback.lock().unwrap().clone();
                    if let Some(callback) = callback {
                        callback().await;
                    }
                }
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        });
        let add_weak = weak;
        let add_extension = Arc::new(move |extension: Arc<dyn Extension>| {
            let weak = add_weak.clone();
            Box::pin(async move {
                if let Some(controller) = weak.upgrade() {
                    controller.add_extension_runtime(extension).await;
                }
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        });

        ExtensionContext {
            zigbee: self.zigbee.clone(),
            mqtt: self.mqtt.clone(),
            state: self.state.clone(),
            publish_entity_state: self.publish_callback(),
            event_bus: self.event_bus.clone(),
            logger: self.logger.clone(),
            settings: self.settings.clone(),
            enable_disable_extension,
            restart,
            add_extension,
            key,
        }
    }

    fn publish_callback(
        self: &Arc<Self>,
    ) -> crate::model::entity::PublishEntityState {
        let weak = Arc::downgrade(self);
        Arc::new(move |entity, payload, reason| {
            let weak = weak.clone();
            Box::pin(async move {
                if let Some(controller) = weak.upgrade() {
                    controller.publish_entity_state(entity, payload, reason).await;
                }
            })
        })
    }

    /// Upstream extension constructor switch (`controller.ts:54-87`).
    pub fn make_extension(self: &Arc<Self>, name: &str) -> Option<Arc<dyn Extension>> {
        let context = self.extension_context();
        Some(match name {
            "OnEvent" => OnEvent::new(context) as Arc<dyn Extension>,
            "Bridge" => Bridge::new(context) as Arc<dyn Extension>,
            "Publish" => Publish::new(context) as Arc<dyn Extension>,
            "Receive" => Receive::new(context) as Arc<dyn Extension>,
            "Configure" => Configure::new(context) as Arc<dyn Extension>,
            "NetworkMap" => NetworkMap::new(context) as Arc<dyn Extension>,
            "Groups" => Groups::new(context) as Arc<dyn Extension>,
            "Bind" => Bind::new(context) as Arc<dyn Extension>,
            "Availability" => Availability::new(context) as Arc<dyn Extension>,
            "Health" => Health::new(context) as Arc<dyn Extension>,
            "HomeAssistant" => HomeAssistant::new(context) as Arc<dyn Extension>,
            "Frontend" => Frontend::new(context) as Arc<dyn Extension>,
            _ => return None,
        })
    }

    pub async fn add_extension_runtime(self: &Arc<Self>, extension: Arc<dyn Extension>) {
        let name = extension.name().to_string();
        if self
            .extensions
            .lock()
            .unwrap()
            .iter()
            .any(|existing| existing.name() == name)
        {
            return;
        }
        self.extensions.lock().unwrap().push(extension.clone());
        self.start_extension(extension).await;
    }

    async fn start_extension(self: &Arc<Self>, extension: Arc<dyn Extension>) {
        if let Err(error) = extension.clone().start().await {
            self.logger.lock().unwrap().error(
                &format!("Failed to start '{}' ({error})", extension.name()),
                "z2m",
            );
        }
    }

    async fn stop_extension(self: &Arc<Self>, extension: Arc<dyn Extension>) {
        if let Err(error) = extension.clone().stop().await {
            self.logger.lock().unwrap().error(
                &format!("Failed to stop '{}' ({error})", extension.name()),
                "z2m",
            );
        }
    }

    /// Upstream `enableDisableExtension` (`controller.ts:218-300`).
    pub async fn enable_disable_extension(
        self: &Arc<Self>,
        enable: bool,
        name: String,
    ) -> Result<(), String> {
        const BUILT_INS: [&str; 12] = [
            "Availability",
            "Bind",
            "Bridge",
            "Configure",
            "Groups",
            "Health",
            "NetworkMap",
            "OnEvent",
            "Publish",
            "Receive",
            "HomeAssistant",
            "Frontend",
        ];
        if enable {
            match name.as_str() {
                "HomeAssistant" | "Frontend" => {
                    let allowed = {
                        let settings = self.settings.lock().unwrap().get();
                        if name == "HomeAssistant" {
                            settings["homeassistant"]["enabled"].as_bool().unwrap_or(false)
                        } else {
                            settings["frontend"]["enabled"].as_bool().unwrap_or(false)
                        }
                    };
                    if !allowed {
                        return Err(format!(
                            "Tried to enable {name} extension disabled in settings"
                        ));
                    }
                    let extension = self
                        .make_extension(&name)
                        .ok_or_else(|| format!("Extension {name} does not exist"))?;
                    self.remove_extension_by_name(&name).await;
                    self.extensions.lock().unwrap().push(extension.clone());
                    self.start_extension(extension).await;
                    Ok(())
                }
                _ => {
                    if BUILT_INS.contains(&name.as_str()) {
                        Err(format!(
                            "Extension {name} does not exist (should be added with 'addExtension') or is built-in that cannot be enabled at runtime"
                        ))
                    } else {
                        Err(format!(
                            "Extension {name} does not exist (should be added with 'addExtension') or is built-in that cannot be enabled at runtime"
                        ))
                    }
                }
            }
        } else {
            match name.as_str() {
                "HomeAssistant" | "Frontend" => {
                    let enabled = {
                        let settings = self.settings.lock().unwrap().get();
                        if name == "HomeAssistant" {
                            settings["homeassistant"]["enabled"].as_bool().unwrap_or(false)
                        } else {
                            settings["frontend"]["enabled"].as_bool().unwrap_or(false)
                        }
                    };
                    if enabled {
                        return Err(format!(
                            "Tried to disable {name} extension enabled in settings"
                        ));
                    }
                    self.remove_extension_by_name(&name).await;
                    Ok(())
                }
                other if BUILT_INS.contains(&other) => {
                    Err(format!("Built-in extension {name} cannot be disabled at runtime"))
                }
                _ => {
                    self.remove_extension_by_name(&name).await;
                    Ok(())
                }
            }
        }
    }

    async fn remove_extension_by_name(self: &Arc<Self>, name: &str) {
        let extension = self
            .extensions
            .lock()
            .unwrap()
            .iter()
            .find(|extension| extension.name() == name)
            .cloned();
        if let Some(extension) = extension {
            self.extensions
                .lock()
                .unwrap()
                .retain(|existing| existing.name() != name);
            self.stop_extension(extension).await;
        }
    }

    /// Upstream `start()` (`controller.ts:89-216`).
    pub async fn start(self: &Arc<Self>) -> Result<(), String> {
        let abort = AbortSignal::new();
        *self.abort.lock().unwrap() = Some(abort.clone());
        State::start(&self.state);

        let settings = self.settings.lock().unwrap().get();
        logger_init(&self.logger, &settings);
        logger_info(
            &self.logger,
            &format!(
                "Starting Zigbee2MQTT version {} (commit #{})",
                utils::get_zigbee2mqtt_version().await["version"]
                    .as_str()
                    .unwrap_or("unknown"),
                utils::get_zigbee2mqtt_version().await["commitHash"]
                    .as_str()
                    .unwrap_or("unknown")
            ),
        );

        let started = self.zigbee.start(&abort).await?;
        if !started || abort.aborted() {
            return Ok(());
        }

        let mut device_count = 0;
        for device in self
            .zigbee
            .devices_iterator(utils::device_not_coordinator)
        {
            device_count += 1;
            let model = if device.is_supported() {
                device
                    .definition()
                    .map(|definition| {
                        format!(
                            "{} - {} {}",
                            definition.model, definition.vendor, definition.description
                        )
                    })
                    .unwrap_or_else(|| "Not supported".to_string())
            } else {
                "Not supported".to_string()
            };
            logger_info(
                &self.logger,
                &format!(
                    "{} ({}): {} ({})",
                    device.name(),
                    device.ieee_addr(),
                    model,
                    device.zh.r#type().as_str()
                ),
            );
        }
        logger_info(
            &self.logger,
            &format!("Currently {device_count} devices are joined."),
        );

        if let Err(error) = self.mqtt.lock().await.connect(&settings).await {
            let _ = self.zigbee.stop().await;
            return Err(format!("MQTT failed to connect, exiting... ({error})"));
        }
        if abort.aborted() {
            return Ok(());
        }

        if settings["advanced"]["enable_external_js"].as_bool().unwrap_or(false) {
            logger_info(
                &self.logger,
                "External JS (converters/extensions) is disabled",
            );
        }
        for name in [
            "OnEvent",
            "Bridge",
            "Publish",
            "Receive",
            "Configure",
            "NetworkMap",
            "Groups",
            "Bind",
            "Availability",
            "Health",
        ] {
            if let Some(extension) = self.make_extension(name) {
                self.extensions.lock().unwrap().push(extension.clone());
                self.start_extension(extension).await;
            }
            if abort.aborted() {
                return Ok(());
            }
        }
        if settings["frontend"]["enabled"].as_bool().unwrap_or(false) {
            if let Some(extension) = self.make_extension("Frontend") {
                self.extensions.lock().unwrap().push(extension.clone());
                self.start_extension(extension).await;
            }
        }
        if settings["homeassistant"]["enabled"].as_bool().unwrap_or(false) {
            if let Some(extension) = self.make_extension("HomeAssistant") {
                self.extensions.lock().unwrap().push(extension.clone());
                self.start_extension(extension).await;
            }
        }

        if settings["advanced"]["cache_state_send_on_startup"]
            .as_bool()
            .unwrap_or(true)
            && settings["advanced"]["cache_state"].as_bool().unwrap_or(true)
        {
            for entity in self.zigbee.devices_and_groups_iterator() {
                let id = entity.entity_id();
                if self.state.lock().unwrap().exists(&id) {
                    let payload = self.state.lock().unwrap().get(&id);
                    self.publish_entity_state(entity, payload, Some("publishCached".to_string()))
                        .await;
                }
            }
        }

        let weak = Arc::downgrade(self);
        self.event_bus.lock().unwrap().on_last_seen_changed(
            self.next_extension_key.fetch_add(1, Ordering::SeqCst),
            Arc::new(move |data| {
                let weak = weak.clone();
                tokio::spawn(async move {
                    if let Some(controller) = weak.upgrade() {
                        if let Some(ieee) = data
                            .get("device")
                            .and_then(|device| device.get("ieeeAddr"))
                            .and_then(Value::as_str)
                        {
                            if let Some(device) = controller.zigbee.resolve_device(ieee) {
                                let applied = controller.settings.lock().unwrap().get();
                                let publish = controller.publish_callback();
                                utils::publish_last_seen(
                                    &data,
                                    device as Arc<dyn Entity>,
                                    &applied,
                                    false,
                                    &publish,
                                )
                                .await;
                            }
                        }
                    }
                });
            }),
        );

        utils::sleep(0).await;
        logger_info(&self.logger, "Zigbee2MQTT started!");
        self.settings.lock().unwrap().set_onboarding(false);
        *self.abort.lock().unwrap() = None;
        Ok(())
    }

    /// Upstream `stop()` (`controller.ts:343-375`).
    pub async fn stop(self: &Arc<Self>, restart: bool, code: i32) {
        logger_info(
            &self.logger,
            &format!("Stopping Zigbee2MQTT (restart={restart}, code={code}, signal=None)"),
        );
        if let Some(abort) = self.abort.lock().unwrap().take() {
            abort.abort();
        }
        let extensions: Vec<Arc<dyn Extension>> = self.extensions.lock().unwrap().clone();
        for extension in extensions {
            self.stop_extension(extension).await;
        }
        self.state.lock().unwrap().stop(Some(&|ieee| self.zigbee.resolve_entity(ieee).is_some()));
        self.mqtt.lock().await.disconnect().await;
        let _ = self.zigbee.stop().await;
        logger_info(&self.logger, "Stopped Zigbee2MQTT");
        let exit = self.exit_callback.lock().unwrap().clone();
        if let Some(exit) = exit {
            exit(code, restart).await;
        }
    }
}

fn logger_info(logger: &Arc<Mutex<Logger>>, message: &str) {
    logger.lock().unwrap().info(message, "z2m");
}

fn logger_init(logger: &Arc<Mutex<Logger>>, settings: &Value) {
    logger.lock().unwrap().advanced = settings.clone();
    logger.lock().unwrap().init();
}
