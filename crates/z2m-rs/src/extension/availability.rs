// Original: .upstream/zigbee2mqtt/lib/extension/availability.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};
use zigbee_apiarist::controller::database::DeviceType;

use crate::extension::extension::{Extension, ExtensionContext, ExtensionFuture};
use crate::model::entity::Entity;
use crate::mqtt::MqttPublishOptions;
use crate::util::utils;

/// Upper bound for a `setTimeout` delay (`availability.ts:16`).
const MAX_TIMEOUT: u64 = 2_147_483_647;

/// Upstream `RETRIEVE_ON_RECONNECT` (`availability.ts:18-22`).
const RETRIEVE_ON_RECONNECT: [(&str, Option<&str>, Option<&str>); 3] = [
    ("state", None, None),
    ("brightness", Some("state"), Some("ON")),
    ("color", Some("state"), Some("ON")),
];

pub struct Availability {
    context: ExtensionContext,
    timers: Mutex<HashMap<String, tokio::task::JoinHandle<()>>>,
    last_published_availabilities: Mutex<HashMap<String, bool>>,
    ping_backoffs: Mutex<HashMap<String, f64>>,
    backoff_paused_devices: Mutex<HashSet<String>>,
    retrieve_state_debouncers: Mutex<HashMap<String, u64>>,
    ping_queue: Mutex<Vec<Arc<dyn Entity>>>,
    ping_queue_executing: AtomicBool,
    stopped: AtomicBool,
}

impl Availability {
    pub fn new(context: ExtensionContext) -> Arc<Self> {
        Arc::new(Self {
            context,
            timers: Mutex::new(HashMap::new()),
            last_published_availabilities: Mutex::new(HashMap::new()),
            ping_backoffs: Mutex::new(HashMap::new()),
            backoff_paused_devices: Mutex::new(HashSet::new()),
            retrieve_state_debouncers: Mutex::new(HashMap::new()),
            ping_queue: Mutex::new(Vec::new()),
            ping_queue_executing: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
        })
    }

    fn availability_option<'a>(&self, device: &'a Arc<dyn Entity>, key: &str) -> Option<Value> {
        device
            .options()
            .get("availability")
            .and_then(|availability| availability.get(key))
            .cloned()
    }

    fn get_timeout(&self, device: &Arc<dyn Entity>) -> u64 {
        if let Some(Value::Number(timeout)) = self.availability_option(device, "timeout") {
            return utils::minutes(timeout.as_u64().unwrap_or(0));
        }
        let settings = self.context.settings_value();
        let minutes = if self.is_active_device(device) {
            settings["availability"]["active"]["timeout"].as_u64()
        } else {
            settings["availability"]["passive"]["timeout"].as_u64()
        }
        .unwrap_or(10);
        utils::minutes(minutes)
    }

    fn get_max_jitter(&self, device: &Arc<dyn Entity>) -> u64 {
        if let Some(Value::Number(jitter)) = self.availability_option(device, "max_jitter") {
            return jitter.as_u64().unwrap_or(0);
        }
        self.context.settings_value()["availability"]["active"]["max_jitter"]
            .as_u64()
            .unwrap_or(30000)
    }

    fn get_backoff(&self, device: &Arc<dyn Entity>) -> bool {
        if let Some(Value::Bool(backoff)) = self.availability_option(device, "backoff") {
            return backoff;
        }
        self.context.settings_value()["availability"]["active"]["backoff"]
            .as_bool()
            .unwrap_or(true)
    }

    fn get_pause_on_backoff_gt(&self, device: &Arc<dyn Entity>) -> f64 {
        if let Some(value) = self.availability_option(device, "pause_on_backoff_gt") {
            return value.as_f64().unwrap_or(0.0);
        }
        self.context.settings_value()["availability"]["active"]["pause_on_backoff_gt"]
            .as_f64()
            .unwrap_or(0.0)
    }

    /// Upstream `isActiveDevice` (`availability.ts:71-76`).
    fn is_active_device(&self, device: &Arc<dyn Entity>) -> bool {
        let Some(zh) = device.zh_device() else {
            return false;
        };
        let power_source = zh.power_source();
        (zh.r#type() == DeviceType::Router && power_source.as_deref() != Some("Battery"))
            || power_source
                .as_deref()
                .is_some_and(|source| source != "Unknown" && source != "Battery")
    }

    /// Upstream `isAvailable` (`availability.ts:78-92`).
    fn is_available(&self, entity: &Arc<dyn Entity>) -> bool {
        if entity.is_device() {
            let last_seen = entity.last_seen().unwrap_or(0) as i64;
            let now = zigbee_apiarist::utils::datetime::now_ms() as i64;
            return now - last_seen < self.get_timeout(entity) as i64;
        }

        entity.members_devices().iter().any(|member| {
            self.last_published_availabilities
                .lock()
                .unwrap()
                .get(&member.entity_id().key())
                .copied()
                == Some(true)
        })
    }

    /// Upstream `resetTimer` (`availability.ts:94-129`).
    fn reset_timer(self: &Arc<Self>, device: Arc<dyn Entity>, reset_backoff: bool) {
        let ieee = device.entity_id().key();
        if let Some(timer) = self.timers.lock().unwrap().remove(&ieee) {
            timer.abort();
        }
        self.remove_from_ping_queue(&device);

        if self.is_active_device(&device) {
            let backoff_enabled = self.get_backoff(&device);
            let jitter = (utils::random_f64() * self.get_max_jitter(&device) as f64) as u64;
            let mut backoff = 1.0;

            if reset_backoff {
                self.backoff_paused_devices.lock().unwrap().remove(&ieee);
                self.ping_backoffs.lock().unwrap().remove(&ieee);
            } else if backoff_enabled {
                backoff = self
                    .ping_backoffs
                    .lock()
                    .unwrap()
                    .get(&ieee)
                    .copied()
                    .unwrap_or(1.0);
            }

            let paused = self.backoff_paused_devices.lock().unwrap().contains(&ieee);
            if !backoff_enabled || reset_backoff || !paused {
                let delay = (((self.get_timeout(&device) + utils::seconds(1) + jitter) as f64)
                    * backoff)
                    .min(MAX_TIMEOUT as f64) as u64;
                let this = self.clone();
                let handle = tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    this.add_to_ping_queue(device);
                });
                self.timers.lock().unwrap().insert(ieee, handle);
            }
        } else {
            let delay = self.get_timeout(&device) + utils::seconds(1);
            let this = self.clone();
            let handle = tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(delay)).await;
                this.publish_availability(device, true, false, false).await;
            });
            self.timers.lock().unwrap().insert(ieee, handle);
        }
    }

    fn clear_timer(&self, id: &str) {
        if let Some(timer) = self.timers.lock().unwrap().remove(id) {
            timer.abort();
        }
    }

    /// Upstream `addToPingQueue` (`availability.ts:136-139`).
    fn add_to_ping_queue(self: &Arc<Self>, device: Arc<dyn Entity>) {
        self.ping_queue.lock().unwrap().push(device);
        let this = self.clone();
        tokio::spawn(async move { this.ping_queue_execute_next().await });
    }

    fn remove_from_ping_queue(&self, device: &Arc<dyn Entity>) {
        let ieee = device.entity_id().key();
        self.ping_queue
            .lock()
            .unwrap()
            .retain(|queued| queued.entity_id().key() != ieee);
    }

    /// Upstream `pingQueueExecuteNext` (`availability.ts:148-209`).
    async fn ping_queue_execute_next(self: Arc<Self>) {
        if self.ping_queue.lock().unwrap().is_empty()
            || self.ping_queue_executing.swap(true, Ordering::SeqCst)
        {
            return;
        }

        loop {
            let device = match self.ping_queue.lock().unwrap().first().cloned() {
                Some(device) => device,
                None => break,
            };
            let ieee = device.entity_id().key();
            let mut ping_success = false;
            let available = self
                .last_published_availabilities
                .lock()
                .unwrap()
                .get(&ieee)
                .copied()
                .unwrap_or(false)
                || self.is_available(&device);
            let attempts = if available { 2 } else { 1 };

            for attempt in 1..=attempts {
                let Some(zh) = device.zh_device() else {
                    break;
                };
                match zh.ping(!available || attempt != 2).await {
                    Ok(()) => {
                        ping_success = true;
                        self.context.logger.lock().unwrap().debug(
                            &format!(
                                "Successfully pinged '{}' (attempt {attempt}/{attempts})",
                                device.name()
                            ),
                            "z2m",
                        );
                        break;
                    }
                    Err(error) => {
                        self.context.logger.lock().unwrap().warning(
                            &format!(
                                "Failed to ping '{}' (attempt {attempt}/{attempts}, {error})",
                                device.name()
                            ),
                            "z2m",
                        );
                        if attempt != attempts {
                            utils::sleep(3).await;
                        }
                    }
                }
            }

            if self.stopped.load(Ordering::SeqCst) {
                return;
            }

            if !ping_success && self.get_backoff(&device) {
                let current = self
                    .ping_backoffs
                    .lock()
                    .unwrap()
                    .get(&ieee)
                    .copied()
                    .unwrap_or(1.0);
                let pause_on = self.get_pause_on_backoff_gt(&device);
                if pause_on > 0.0 && current >= pause_on {
                    self.backoff_paused_devices
                        .lock()
                        .unwrap()
                        .insert(ieee.clone());
                } else {
                    self.ping_backoffs
                        .lock()
                        .unwrap()
                        .insert(ieee.clone(), current * if available { 1.5 } else { 2.0 });
                }
            }

            self.publish_availability(device.clone(), !ping_success, false, false)
                .await;
            self.reset_timer(device.clone(), ping_success);
            self.remove_from_ping_queue(&device);
            utils::sleep(2).await;
        }

        self.ping_queue_executing.store(false, Ordering::SeqCst);
    }

    /// Upstream `publishAvailabilityForAllEntities` (`availability.ts:244-250`).
    async fn publish_availability_for_all_entities(self: &Arc<Self>) {
        let mut entities: Vec<Arc<dyn Entity>> = self
            .context
            .zigbee
            .devices_iterator(utils::device_not_coordinator)
            .into_iter()
            .map(|device| device as Arc<dyn Entity>)
            .collect();
        entities.extend(
            self.context
                .zigbee
                .groups_iterator(|_| true)
                .into_iter()
                .map(|group| group as Arc<dyn Entity>),
        );
        for entity in entities {
            if utils::is_availability_enabled_for_entity(
                entity.as_ref(),
                &self.context.settings_value(),
            ) {
                self.publish_availability(entity, true, false, true).await;
            }
        }
    }

    /// Upstream `publishAvailability` (`availability.ts:252-288`).
    async fn publish_single(
        self: &Arc<Self>,
        entity: Arc<dyn Entity>,
        log_last_seen: bool,
        force_publish: bool,
        skip_groups: bool,
    ) {
        if log_last_seen && entity.is_device() {
            let now = zigbee_apiarist::utils::datetime::now_ms() as i64;
            let ago = now - entity.last_seen().unwrap_or(0) as i64;
            if self.is_active_device(&entity) {
                self.context.logger.lock().unwrap().debug(
                    &format!(
                        "Active device '{}' was last seen '{:.2}' minutes ago.",
                        entity.name(),
                        ago as f64 / utils::minutes(1) as f64
                    ),
                    "z2m",
                );
            } else {
                self.context.logger.lock().unwrap().debug(
                    &format!(
                        "Passive device '{}' was last seen '{:.2}' hours ago.",
                        entity.name(),
                        ago as f64 / utils::hours(1) as f64
                    ),
                    "z2m",
                );
            }
        }

        let available = self.is_available(&entity);
        let key = entity.entity_id().key();

        if !force_publish
            && self
                .last_published_availabilities
                .lock()
                .unwrap()
                .get(&key)
                .copied()
                == Some(available)
        {
            return;
        }

        if entity.is_device()
            && available
            && self
                .last_published_availabilities
                .lock()
                .unwrap()
                .get(&key)
                .copied()
                == Some(false)
        {
            self.context
                .logger
                .lock()
                .unwrap()
                .debug(&format!("Device '{}' reconnected", entity.name()), "z2m");
            self.retrieve_state(entity.clone());
        }

        self.last_published_availabilities
            .lock()
            .unwrap()
            .insert(key, available);
        self.context
            .mqtt
            .lock()
            .await
            .publish(
                &format!("{}/availability", entity.name()),
                serde_json::to_string(
                    &json!({"state": if available { "online" } else { "offline" }}),
                )
                .unwrap_or_default(),
                MqttPublishOptions {
                    retain: true,
                    qos: 1,
                    ..Default::default()
                },
            )
            .await;
    }

    /// Upstream group fan-out, kept non-recursive for the async type.
    async fn publish_availability(
        self: &Arc<Self>,
        entity: Arc<dyn Entity>,
        log_last_seen: bool,
        force_publish: bool,
        skip_groups: bool,
    ) {
        self.publish_single(entity.clone(), log_last_seen, force_publish, skip_groups)
            .await;

        if !skip_groups && entity.is_device() {
            let Some(zh) = entity.zh_device() else {
                return;
            };
            for group in self.context.zigbee.groups_iterator(|_| true) {
                if group.has_member_zh(&zh)
                    && utils::is_availability_enabled_for_entity(
                        group.as_ref(),
                        &self.context.settings_value(),
                    )
                {
                    self.publish_single(group as Arc<dyn Entity>, false, force_publish, true)
                        .await;
                }
            }
        }
    }

    /// Upstream `onLastSeenChanged` (`availability.ts:290-297`).
    async fn on_last_seen_changed(self: &Arc<Self>, data: Value) {
        let Some(ieee) = data
            .get("device")
            .and_then(|device| device.get("ieeeAddr"))
            .and_then(Value::as_str)
        else {
            return;
        };
        let Some(device) = self.context.zigbee.resolve_device(ieee) else {
            return;
        };
        if utils::is_availability_enabled_for_entity(
            device.as_ref(),
            &self.context.settings_value(),
        ) {
            self.remove_from_ping_queue(&(device.clone() as Arc<dyn Entity>));
            self.reset_timer(device.clone(), true);
            self.publish_availability(device, false, false, false).await;
        }
    }

    /// Upstream `retrieveState` (`availability.ts:310-361`).
    fn retrieve_state(self: &Arc<Self>, device: Arc<dyn Entity>) {
        let ieee = device.entity_id().key();
        if !device.interviewed() || device.definition().is_none() {
            return;
        }

        let generation = {
            let mut debouncers = self.retrieve_state_debouncers.lock().unwrap();
            if debouncers.contains_key(&ieee) {
                let next = debouncers.get(&ieee).copied().unwrap_or(0) + 1;
                debouncers.insert(ieee.clone(), next);
                return;
            }
            debouncers.insert(ieee.clone(), 0);
            0
        };

        let this = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(utils::seconds(2))).await;
            if this
                .retrieve_state_debouncers
                .lock()
                .unwrap()
                .get(&ieee)
                .copied()
                != Some(generation)
            {
                return;
            }
            this.retrieve_state_debouncers.lock().unwrap().remove(&ieee);

            this.context.logger.lock().unwrap().debug(
                &format!("Retrieving state of '{}' after reconnect", device.name()),
                "z2m",
            );

            let Some(definition) = device.definition() else {
                return;
            };
            let state = this.context.state.lock().unwrap().get(&device.entity_id());

            for (key, condition_key, condition_value) in RETRIEVE_ON_RECONNECT {
                if let (Some(condition_key), Some(condition_value)) =
                    (condition_key, condition_value)
                {
                    if state.get(condition_key).and_then(Value::as_str) != Some(condition_value) {
                        continue;
                    }
                }

                let Some(convert_get) = definition
                    .to_zigbee
                    .iter()
                    .find(|converter| {
                        converter.key.is_empty()
                            || converter
                                .key
                                .iter()
                                .any(|converter_key| converter_key == key)
                    })
                    .and_then(|converter| converter.convert_get.clone())
                else {
                    continue;
                };
                let Some(endpoint) = device.endpoint(None) else {
                    continue;
                };
                let meta = zigbee_converters::ToMeta {
                    endpoint_name: None,
                    options: device.options(),
                    message: state.clone(),
                    device: device.zh_device(),
                    state: state.clone(),
                    mapped: definition.clone(),
                    publish: Some(Arc::new({
                        let this = this.clone();
                        let device = device.clone();
                        move |payload: Value| -> zigbee_converters::ConverterFuture<()> {
                            let this = this.clone();
                            let device = device.clone();
                            Box::pin(async move {
                                (this.context.publish_entity_state)(device, payload, None).await;
                            })
                        }
                    })),
                    device_exposes_changed: Some(Arc::new({
                        let this = this.clone();
                        move || {
                            this.context
                                .event_bus
                                .lock()
                                .unwrap()
                                .emit_exposes_and_devices_changed();
                        }
                    })),
                };

                if let Err(error) = convert_get(
                    zigbee_converters::ToTarget::Endpoint(endpoint),
                    key.to_string(),
                    state.get(key).cloned().unwrap_or(Value::Null),
                    meta,
                )
                .await
                {
                    this.context.logger.lock().unwrap().error(
                        &format!(
                            "Failed to read state of '{}' after reconnect ({error})",
                            device.name()
                        ),
                        "z2m",
                    );
                }

                utils::sleep(0).await;
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });
    }

    fn devices(&self) -> Vec<Arc<dyn Entity>> {
        self.context
            .zigbee
            .devices_iterator(utils::device_not_coordinator)
            .into_iter()
            .map(|device| device as Arc<dyn Entity>)
            .collect()
    }
}

impl Extension for Availability {
    fn name(&self) -> &'static str {
        "Availability"
    }

    fn context(&self) -> &ExtensionContext {
        &self.context
    }

    fn start(self: Arc<Self>) -> ExtensionFuture {
        Box::pin(async move {
            if self.stopped.load(Ordering::SeqCst) {
                return Err("This extension cannot be restarted.".to_string());
            }

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_entity_renamed(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        let entity = data.get("entity").cloned().unwrap_or(Value::Null);
                        let from = data.get("from").and_then(Value::as_str).unwrap_or_default();
                        if let Some(ieee) = entity.get("ieeeAddr").and_then(Value::as_str) {
                            if let Some(device) = this.context.zigbee.resolve_device(ieee) {
                                if utils::is_availability_enabled_for_entity(
                                    device.as_ref(),
                                    &this.context.settings_value(),
                                ) {
                                    this.context
                                        .mqtt
                                        .lock()
                                        .await
                                        .publish(
                                            &format!("{from}/availability"),
                                            "",
                                            MqttPublishOptions {
                                                retain: true,
                                                qos: 1,
                                                ..Default::default()
                                            },
                                        )
                                        .await;
                                    this.publish_availability(
                                        device as Arc<dyn Entity>,
                                        false,
                                        true,
                                        false,
                                    )
                                    .await;
                                }
                            }
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_entity_removed(
                self.context.key,
                Arc::new(move |data| {
                    if let Some(ieee) = data
                        .get("entity")
                        .and_then(|entity| entity.get("ieeeAddr"))
                        .and_then(Value::as_str)
                    {
                        this.clear_timer(ieee);
                    }
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_device_leave(
                self.context.key,
                Arc::new(move |data| {
                    if let Some(ieee) = data.get("ieeeAddr").and_then(Value::as_str) {
                        this.clear_timer(ieee);
                    }
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_device_announce(
                self.context.key,
                Arc::new(move |data| {
                    if let Some(ieee) = data
                        .get("device")
                        .and_then(|device| device.get("ieeeAddr"))
                        .and_then(Value::as_str)
                    {
                        if let Some(device) = this.context.zigbee.resolve_device(ieee) {
                            this.retrieve_state(device as Arc<dyn Entity>);
                        }
                    }
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_last_seen_changed(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move { this.on_last_seen_changed(data).await });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_group_members_changed(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(group_id) = data
                            .get("group")
                            .and_then(|group| group.get("ID"))
                            .and_then(Value::as_u64)
                        {
                            if let Some(group) = this.context.zigbee.group_by_id(group_id) {
                                this.publish_availability(
                                    group as Arc<dyn Entity>,
                                    false,
                                    false,
                                    false,
                                )
                                .await;
                            }
                        }
                    });
                }),
            );

            self.publish_availability_for_all_entities().await;

            for device in self.devices() {
                if utils::is_availability_enabled_for_entity(
                    device.as_ref(),
                    &self.context.settings_value(),
                ) {
                    self.reset_timer(device.clone(), false);
                    if self.is_active_device(&device) && !self.is_available(&device) {
                        self.add_to_ping_queue(device);
                    }
                }
            }

            Ok(())
        })
    }

    fn stop(self: Arc<Self>) -> ExtensionFuture {
        Box::pin(async move {
            self.stopped.store(true, Ordering::SeqCst);
            self.ping_queue.lock().unwrap().clear();
            for (_, timer) in self.timers.lock().unwrap().drain() {
                timer.abort();
            }
            self.context
                .event_bus
                .lock()
                .unwrap()
                .remove_listeners(self.context.key);
            Ok(())
        })
    }
}
