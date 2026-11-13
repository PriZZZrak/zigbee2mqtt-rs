// Original: .upstream/zigbee2mqtt/lib/extension/receive.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// R1 registers no converter database, so devices stay definition-less and the
// upstream fallback branch (`receive.ts:130-134`) logs "Skipping message, still
// interviewing" and publishes last-seen only. Registered definitions use the
// shared `fromZigbee` converters from `zigbee-converters`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use zigbee_converters::{ConvertData, ConvertMeta, DefinitionRef, FromZigbeeConverter};

use crate::event_bus::EventBus;
use crate::model::entity::{Entity, PublishEntityState};
use crate::state::State;
use crate::util::logger::Logger;
use crate::util::settings::Settings;
use crate::util::stringify::stringify;
use crate::util::utils;
use crate::zigbee::Zigbee;

struct DebounceState {
    payload: Value,
    generation: u64,
}

/// Upstream `Receive` (`lib/extension/receive.ts:14`).
pub struct Receive {
    event_bus: Arc<Mutex<EventBus>>,
    zigbee: Arc<Zigbee>,
    settings: Arc<Mutex<Settings>>,
    logger: Arc<Mutex<Logger>>,
    state: Arc<Mutex<State>>,
    publish_entity_state: PublishEntityState,
    elapsed: Mutex<HashMap<String, u64>>,
    debouncers: Mutex<HashMap<String, Arc<Mutex<DebounceState>>>>,
    throttlers: Mutex<HashMap<String, Instant>>,
}

impl Receive {
    #[allow(clippy::too_many_arguments)]
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
            elapsed: Mutex::new(HashMap::new()),
            debouncers: Mutex::new(HashMap::new()),
            throttlers: Mutex::new(HashMap::new()),
        })
    }

    /// Upstream `start()` (`receive.ts:21-25`).
    pub fn attach(self: &Arc<Self>, key: usize) {
        let bus = self.event_bus.clone();
        let this = self.clone();
        bus.lock().unwrap().on_device_message(
            key,
            Arc::new(move |data| {
                let this = this.clone();
                tokio::spawn(async move { this.on_device_message(data).await });
            }),
        );

        let bus = self.event_bus.clone();
        let this = self.clone();
        bus.lock().unwrap().on_last_seen_changed(
            key,
            Arc::new(move |data| {
                let this = this.clone();
                tokio::spawn(async move { this.on_last_seen_changed(data).await });
            }),
        );

        let bus = self.event_bus.clone();
        let this = self.clone();
        bus.lock().unwrap().on_publish_entity_state(
            key,
            Arc::new(move |data| this.on_publish_entity_state(&data)),
        );
    }

    /// Upstream `onLastSeenChanged` (`receive.ts:33-47`).
    pub async fn on_last_seen_changed(self: &Arc<Self>, data: Value) {
        let last_seen = self.applied()["advanced"]["last_seen"]
            .as_str()
            .unwrap_or("disable")
            .to_string();
        if data.get("reason").and_then(Value::as_str) != Some("messageNonEmitted")
            || last_seen == "disable"
        {
            return;
        }

        let Some(device) = self.device_from(&data) else {
            return;
        };
        let options = device.options();
        if let Some(debounce) = options.get("debounce").and_then(Value::as_u64) {
            let ignore = options
                .get("debounce_ignore")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                });
            self.publish_debounce(device, json!({}), debounce, ignore)
                .await;
        } else if let Some(throttle) = options.get("throttle").and_then(Value::as_u64) {
            self.publish_throttle(device, json!({}), throttle).await;
        }
    }

    /// Upstream `onPublishEntityState` (`receive.ts:49-65`).
    pub fn on_publish_entity_state(&self, data: &Value) {
        let Some(ieee) = data
            .get("entity")
            .and_then(|entity| entity.get("ieeeAddr"))
            .and_then(Value::as_str)
        else {
            return;
        };

        let reason = data.get("stateChangeReason").and_then(Value::as_str);
        if reason == Some("publishDebounce") || reason == Some("lastSeenChanged") {
            return;
        }

        let debouncers = self.debouncers.lock().unwrap();
        let Some(debouncer) = debouncers.get(ieee) else {
            return;
        };
        let mut state = debouncer.lock().unwrap();
        if let Some(payload) = data.get("payload").and_then(Value::as_object) {
            if let Value::Object(current) = &mut state.payload {
                for key in payload.keys() {
                    current.remove(key);
                }
            }
        }
    }

    /// Upstream `publishDebounce` (`receive.ts:67-93`).
    pub async fn publish_debounce(
        self: &Arc<Self>,
        device: Arc<dyn Entity>,
        payload: Value,
        time: u64,
        debounce_ignore: Option<Vec<String>>,
    ) {
        let key = device.entity_id().key();
        let debouncer = {
            let mut debouncers = self.debouncers.lock().unwrap();
            debouncers
                .entry(key.clone())
                .or_insert_with(|| {
                    Arc::new(Mutex::new(DebounceState {
                        payload: json!({}),
                        generation: 0,
                    }))
                })
                .clone()
        };

        let (flush, generation) = {
            let mut state = debouncer.lock().unwrap();
            let mut flush = None;
            if is_payload_conflicted(&payload, &state.payload, debounce_ignore.as_deref()) {
                let previous = std::mem::take(&mut state.payload);
                state.generation += 1;
                flush = Some(previous);
            }
            if let (Value::Object(current), Value::Object(update)) = (&mut state.payload, &payload)
            {
                for (key, value) in update {
                    current.insert(key.clone(), value.clone());
                }
            }
            state.generation += 1;
            (flush, state.generation)
        };

        if let Some(previous) = flush {
            self.publish_now(
                device.clone(),
                previous,
                Some("publishDebounce".to_string()),
            )
            .await;
        }

        // Update the state cache right away (`receive.ts:88-90`).
        self.state
            .lock()
            .unwrap()
            .set(device.as_ref(), payload, None);

        let this = self.clone();
        let device_for_task = device.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(time * 1000)).await;
            let payload = {
                let mut state = debouncer.lock().unwrap();
                if state.generation != generation {
                    return;
                }
                std::mem::take(&mut state.payload)
            };
            this.publish_now(
                device_for_task,
                payload,
                Some("publishDebounce".to_string()),
            )
            .await;
        });
    }

    /// Upstream `publishThrottle` (`receive.ts:95-107`).
    pub async fn publish_throttle(
        self: &Arc<Self>,
        device: Arc<dyn Entity>,
        payload: Value,
        time: u64,
    ) {
        self.state
            .lock()
            .unwrap()
            .set(device.as_ref(), payload.clone(), None);

        let key = device.entity_id().key();
        let allowed = {
            let mut throttlers = self.throttlers.lock().unwrap();
            let wait = Duration::from_millis(time * 1000);
            let allowed = throttlers
                .get(&key)
                .map(|last| last.elapsed() >= wait)
                .unwrap_or(true);
            if allowed {
                throttlers.insert(key, Instant::now());
            }
            allowed
        };

        if allowed {
            self.publish_now(device, payload, Some("publishThrottle".to_string()))
                .await;
        }
    }

    /// Upstream `isPayloadConflicted` (`receive.ts:113-123`).
    pub fn is_payload_conflicted(
        new_payload: &Value,
        old_payload: &Value,
        debounce_ignore: Option<&[String]>,
    ) -> bool {
        is_payload_conflicted(new_payload, old_payload, debounce_ignore)
    }

    /// Upstream `onDeviceMessage` (`receive.ts:125-213`).
    pub async fn on_device_message(self: &Arc<Self>, data: Value) {
        let Some(device) = self.device_from(&data) else {
            return;
        };
        device.resolve_definition(false);

        let definition = device.definition();
        if definition.is_none() || !device.interviewed() {
            self.logger
                .lock()
                .unwrap()
                .debug("Skipping message, still interviewing", "z2m");
            utils::publish_last_seen(
                &json!({"device": device.event_value(), "reason": "messageEmitted"}),
                device.clone(),
                &self.applied(),
                true,
                &self.publish_entity_state,
            )
            .await;
            return;
        }
        let definition = definition.unwrap();

        let r#type = data["type"].as_str().unwrap_or_default().to_string();
        let cluster = data["cluster"].as_str().unwrap_or_default().to_string();
        let converters = matching_converters(&definition, &cluster, &r#type);

        let ignore_clusters = ["genOta", "genTime", "genBasic", "genPollCtrl"];
        if converters.is_empty() && !ignore_clusters.contains(&cluster.as_str()) {
            self.logger.lock().unwrap().debug(
                &format!(
                    "No converter available for '{}' with cluster '{cluster}' and type '{}' and data '{}'",
                    definition.model,
                    r#type,
                    stringify(&data["data"]),
                ),
                "z2m",
            );
            utils::publish_last_seen(
                &json!({"device": device.event_value(), "reason": "messageEmitted"}),
                device.clone(),
                &self.applied(),
                true,
                &self.publish_entity_state,
            )
            .await;
            return;
        }

        let mut payload = json!({});
        for converter in converters {
            let convert_data = build_convert_data(self, &device, &data, &r#type, &cluster);
            let meta = ConvertMeta {
                device: device.zh_device().expect("receive only converts devices"),
                state: self.state.lock().unwrap().get(&device.entity_id()),
                device_exposes_changed: Arc::new({
                    let this = self.clone();
                    move || {
                        this.event_bus
                            .lock()
                            .unwrap()
                            .emit_exposes_and_devices_changed();
                    }
                }),
            };
            let options = device.options();
            let publish_callback: zigbee_converters::PublishCallback = Arc::new({
                let this = self.clone();
                let device = device.clone();
                move |payload: Value| -> zigbee_converters::ConverterFuture<()> {
                    let this = this.clone();
                    let device = device.clone();
                    Box::pin(async move {
                        this.publish(device, payload, None).await;
                    })
                }
            });

            match (converter.convert)(
                definition.clone(),
                convert_data,
                publish_callback,
                options,
                meta,
            )
            .await
            {
                Ok(Some(converted)) => {
                    merge(&mut payload, converted);
                }
                Ok(None) => {}
                Err(error) => {
                    self.logger.lock().unwrap().error(
                        &format!("Exception while calling fromZigbee converter: {error}}}"),
                        "z2m",
                    );
                }
            }
        }

        if !utils::object_is_empty(&payload) {
            self.publish(device, payload, None).await;
        } else if self.applied()["advanced"]["last_seen"]
            .as_str()
            .is_some_and(|value| value != "disable" && !value.is_empty())
        {
            self.publish(device, json!({}), None).await;
        }
    }

    /// Upstream `publish` closure (`receive.ts:157-179`).
    async fn publish(
        self: &Arc<Self>,
        device: Arc<dyn Entity>,
        payload: Value,
        reason: Option<String>,
    ) {
        let elapsed_enabled = self.applied()["advanced"]["elapsed"]
            .as_bool()
            .unwrap_or(false);
        let mut payload = payload;
        if elapsed_enabled {
            let now = zigbee_apiarist::utils::datetime::now_ms() as u64;
            let key = device.entity_id().key();
            let mut elapsed = self.elapsed.lock().unwrap();
            if let Some(previous) = elapsed.get(&key) {
                payload["elapsed"] = json!(now.saturating_sub(*previous));
            }
            elapsed.insert(key, now);
        }

        let options = device.options();
        if let Some(debounce) = options.get("debounce").and_then(Value::as_u64) {
            let ignore = options
                .get("debounce_ignore")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                });
            self.publish_debounce(device, payload, debounce, ignore)
                .await;
        } else if let Some(throttle) = options.get("throttle").and_then(Value::as_u64) {
            self.publish_throttle(device, payload, throttle).await;
        } else {
            self.publish_now(device, payload, reason).await;
        }
    }

    /// Upstream debounce/throttle callbacks publish through `publishEntityState`.
    async fn publish_now(&self, device: Arc<dyn Entity>, payload: Value, reason: Option<String>) {
        (self.publish_entity_state)(device, payload, reason).await;
    }

    fn applied(&self) -> Value {
        self.settings.lock().unwrap().get()
    }

    fn device_from(&self, data: &Value) -> Option<Arc<dyn Entity>> {
        let ieee = data
            .get("device")
            .and_then(|device| device.get("ieeeAddr"))
            .and_then(Value::as_str)?;
        self.zigbee
            .resolve_device(ieee)
            .map(|device| device as Arc<dyn Entity>)
    }
}

fn matching_converters(
    definition: &DefinitionRef,
    cluster: &str,
    r#type: &str,
) -> Vec<FromZigbeeConverter> {
    definition
        .from_zigbee
        .iter()
        .filter(|converter| {
            converter.cluster == cluster && converter.types.iter().any(|kind| kind == r#type)
        })
        .cloned()
        .collect()
}

fn build_convert_data(
    receive: &Receive,
    device: &Arc<dyn Entity>,
    data: &Value,
    r#type: &str,
    cluster: &str,
) -> ConvertData {
    let endpoint_id = data
        .get("endpoint")
        .and_then(|endpoint| endpoint.get("ID"))
        .and_then(Value::as_u64)
        .unwrap_or(1) as u8;
    let zh_device = receive
        .zigbee
        .resolve_device(&device.entity_id().key())
        .map(|device| device.zh.clone())
        .expect("device resolved above");
    let endpoint = zh_device
        .get_endpoint(endpoint_id)
        .or_else(|| zh_device.endpoints().first().cloned())
        .expect("device has no endpoint");

    ConvertData {
        r#type: r#type.to_string(),
        device: zh_device,
        endpoint,
        cluster: cluster.to_string(),
        data: data["data"].clone(),
        group_id: data
            .get("groupID")
            .and_then(Value::as_u64)
            .map(|id| id as u16),
        linkquality: data.get("linkquality").and_then(Value::as_u64).unwrap_or(0) as u8,
        meta: data.get("meta").cloned().unwrap_or(Value::Null),
    }
}

fn is_payload_conflicted(
    new_payload: &Value,
    old_payload: &Value,
    debounce_ignore: Option<&[String]>,
) -> bool {
    let (Value::Object(old), Value::Object(new)) = (old_payload, new_payload) else {
        return false;
    };
    let Some(debounce_ignore) = debounce_ignore else {
        return false;
    };

    for (key, old_value) in old {
        if debounce_ignore.iter().any(|ignore| ignore == key)
            && new.get(key).is_some_and(|new_value| new_value != old_value)
        {
            return true;
        }
    }
    false
}

fn merge(target: &mut Value, update: Value) {
    if let (Value::Object(target), Value::Object(update)) = (target, update) {
        for (key, value) in update {
            target.insert(key, value);
        }
    }
}
