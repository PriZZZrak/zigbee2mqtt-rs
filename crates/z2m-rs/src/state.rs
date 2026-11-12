// Original: .upstream/zigbee2mqtt/lib/state.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Map, Value};

use crate::event_bus::EventBus;
use crate::model::entity::{Entity, EntityId};
use crate::util::logger::Logger;
use crate::util::object_assign_deep::object_assign_deep;
use crate::util::settings::Settings;
use crate::util::{data, utils};

/// Upstream `SAVE_INTERVAL` (`state.ts:9`).
pub const SAVE_INTERVAL_MS: u64 = 1000 * 60 * 5;

/// Upstream `CACHE_IGNORE_PROPERTIES` (`state.ts:10-29`, duplicate
/// `transition_time` entry included on purpose).
pub const CACHE_IGNORE_PROPERTIES: [&str; 17] = [
    "action",
    "action_.*",
    "button",
    "button_left",
    "button_right",
    "forgotten",
    "keyerror",
    "step_size",
    "transition_time",
    "group_list",
    "group_capacity",
    "no_occupancy_since",
    "step_mode",
    "transition_time",
    "elapsed",
    "from_side",
    "to_side",
];

/// Upstream `State` (`lib/state.ts:31`).
pub struct State {
    state: Map<String, Value>,
    file: PathBuf,
    timer: Option<tokio::task::JoinHandle<()>>,
    settings: Arc<Mutex<Settings>>,
    event_bus: Arc<Mutex<EventBus>>,
    logger: Arc<Mutex<Logger>>,
}

impl State {
    pub fn new(
        settings: Arc<Mutex<Settings>>,
        event_bus: Arc<Mutex<EventBus>>,
        logger: Arc<Mutex<Logger>>,
    ) -> Self {
        Self::with_file(settings, event_bus, logger, data::join_path("state.json"))
    }

    /// Test/embedding hook: upstream derives the path from `data.getPath()`.
    pub fn with_file(
        settings: Arc<Mutex<Settings>>,
        event_bus: Arc<Mutex<EventBus>>,
        logger: Arc<Mutex<Logger>>,
        file: PathBuf,
    ) -> Self {
        Self {
            state: Map::new(),
            file,
            timer: None,
            settings,
            event_bus,
            logger,
        }
    }

    /// Upstream `start()` (`state.ts:44-49`). Rust needs the owning `Arc` to
    /// schedule the periodic save, so the port takes it explicitly.
    pub fn start(state: &Arc<Mutex<Self>>) {
        state.lock().unwrap().load();

        let weak = Arc::downgrade(state);
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(SAVE_INTERVAL_MS));
            interval.tick().await;
            loop {
                interval.tick().await;
                let Some(state) = std::sync::Weak::upgrade(&weak) else {
                    break;
                };
                state.lock().unwrap().save();
            }
        });
        state.lock().unwrap().timer = Some(handle);
    }

    /// Upstream `stop()` (`state.ts:51-65`). `resolve_entity` mirrors
    /// `this.zigbee.resolveEntity(key)` and is passed as `None` when the
    /// coordinator was never created (`zhController === undefined`).
    pub fn stop(&mut self, resolve_entity: Option<&dyn Fn(&str) -> bool>) {
        if let Some(resolve_entity) = resolve_entity {
            self.state
                .retain(|key, _| !(key.starts_with("0x") && !resolve_entity(key)));
        }

        if let Some(timer) = self.timer.take() {
            timer.abort();
        }
        self.save();
    }

    /// Upstream `clear()` (`state.ts:67-69`).
    pub fn clear(&mut self) {
        self.state.clear();
    }

    fn load(&mut self) {
        self.state.clear();

        if !self.file.exists() {
            self.logger.lock().unwrap().debug(
                &format!(
                    "Can't load state from file {} (doesn't exist)",
                    self.file.display()
                ),
                "z2m",
            );
            return;
        }

        match std::fs::read_to_string(&self.file) {
            Ok(content) => match serde_json::from_str::<Value>(&content) {
                Ok(Value::Object(entries)) => {
                    for (key, value) in entries {
                        let id = EntityId::from_state_key(&key);
                        self.state.insert(id.key(), value);
                    }
                    self.logger.lock().unwrap().debug(
                        &format!("Loaded state from file {}", self.file.display()),
                        "z2m",
                    );
                }
                Ok(_) | Err(_) => {
                    let error = match serde_json::from_str::<Value>(&content) {
                        Err(error) => error.to_string(),
                        Ok(_) => "state.json must contain an object".to_string(),
                    };
                    self.logger.lock().unwrap().debug(
                        &format!(
                            "Failed to load state from file {} (corrupt file?) ({error})",
                            self.file.display()
                        ),
                        "z2m",
                    );
                }
            },
            Err(error) => {
                self.logger.lock().unwrap().debug(
                    &format!(
                        "Failed to load state from file {} (corrupt file?) ({error})",
                        self.file.display()
                    ),
                    "z2m",
                );
            }
        }
    }

    fn save(&mut self) {
        let persistent = self
            .settings
            .lock()
            .unwrap()
            .get()
            .get("advanced")
            .and_then(|advanced| advanced.get("cache_state_persistent"))
            .and_then(Value::as_bool)
            .unwrap_or(true);

        if !persistent {
            self.logger.lock().unwrap().debug("Not saving state", "z2m");
            return;
        }

        self.logger.lock().unwrap().debug(
            &format!("Saving state to file {}", self.file.display()),
            "z2m",
        );

        let json = stringify_pretty_4(&Value::Object(self.state.clone()));
        if let Err(error) = std::fs::write(&self.file, json) {
            self.logger.lock().unwrap().error(
                &format!(
                    "Failed to write state to '{}' ({error})",
                    self.file.display()
                ),
                "z2m",
            );
        }
    }

    /// Upstream `exists(entity)` (`state.ts:107-109`).
    pub fn exists(&self, id: &EntityId) -> bool {
        self.state.contains_key(&id.key())
    }

    /// Upstream `get(entity)` (`state.ts:111-113`).
    pub fn get(&self, id: &EntityId) -> Value {
        self.state
            .get(&id.key())
            .cloned()
            .unwrap_or_else(|| json!({}))
    }

    /// Upstream `set(entity, update, reason)` (`state.ts:115-126`).
    pub fn set(&mut self, entity: &dyn Entity, update: Value, reason: Option<String>) -> Value {
        let id = entity.entity_id();
        let key = id.key();
        let from_state = self.state.get(&key).cloned().unwrap_or_else(|| json!({}));

        let mut to_state = json!({});
        object_assign_deep(&mut to_state, &[from_state.clone(), update.clone()]);

        let mut new_cache = to_state.clone();
        let mut filter = CACHE_IGNORE_PROPERTIES
            .iter()
            .map(|property| property.to_string())
            .collect::<Vec<_>>();
        filter.extend(entity.filtered_cache());
        utils::filter_properties(Some(&filter), &mut new_cache);

        self.state.insert(key, new_cache);

        let mut data = json!({
            "entity": entity.event_value(),
            "from": from_state,
            "to": to_state,
            "update": update,
        });
        if let Some(reason) = reason {
            data["reason"] = Value::String(reason);
        }
        self.event_bus.lock().unwrap().emit_state_change(data);

        to_state
    }

    /// Upstream `remove(id)` (`state.ts:128-130`).
    pub fn remove(&mut self, id: &EntityId) -> bool {
        self.state.remove(&id.key()).is_some()
    }

    /// Raw cache contents (test/diagnostics helper, equivalent to iterating the
    /// upstream private `state` map).
    pub fn entries(&self) -> Map<String, Value> {
        self.state.clone()
    }
}

/// `JSON.stringify(value, null, 4)` (`state.ts:95`).
fn stringify_pretty_4(value: &Value) -> String {
    fn write(value: &Value, indent: usize, output: &mut String) {
        match value {
            Value::Object(map) if !map.is_empty() => {
                output.push_str("{\n");
                for (index, (key, entry)) in map.iter().enumerate() {
                    output.push_str(&" ".repeat(indent + 4));
                    output.push_str(&serde_json::to_string(key).expect("keys serialize"));
                    output.push_str(": ");
                    write(entry, indent + 4, output);
                    if index + 1 < map.len() {
                        output.push(',');
                    }
                    output.push('\n');
                }
                output.push_str(&" ".repeat(indent));
                output.push('}');
            }
            Value::Object(_) => output.push_str("{}"),
            Value::Array(items) if !items.is_empty() => {
                output.push_str("[\n");
                for (index, entry) in items.iter().enumerate() {
                    output.push_str(&" ".repeat(indent + 4));
                    write(entry, indent + 4, output);
                    if index + 1 < items.len() {
                        output.push(',');
                    }
                    output.push('\n');
                }
                output.push_str(&" ".repeat(indent));
                output.push(']');
            }
            Value::Array(_) => output.push_str("[]"),
            other => output.push_str(&serde_json::to_string(other).expect("values serialize")),
        }
    }

    let mut output = String::new();
    write(value, 0, &mut output);
    output
}
