// Original: .upstream/zigbee2mqtt/test/controller.test.ts (State coverage lives in
// the controller suite upstream; there is no dedicated state.test.ts).
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// Ported against `lib/state.ts` behaviour: load/save key handling,
// `CACHE_IGNORE_PROPERTIES`, deep merge, event payload and invalid-state
// cleanup on stop.

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use z2m_rs::event_bus::{Event, EventBus};
use z2m_rs::model::entity::{Entity, EntityId};
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;

struct TestEntity {
    id: EntityId,
    name: String,
    options: Value,
}

impl TestEntity {
    fn device(ieee: &str) -> Self {
        Self {
            id: EntityId::Ieee(ieee.to_string()),
            name: ieee.to_string(),
            options: json!({"friendly_name": ieee}),
        }
    }

    fn group(id: u64) -> Self {
        Self {
            id: EntityId::Group(id),
            name: id.to_string(),
            options: json!({"friendly_name": id.to_string()}),
        }
    }

    fn with_filtered_cache(mut self, properties: &[&str]) -> Self {
        self.options["filtered_cache"] = json!(properties);
        self
    }
}

impl Entity for TestEntity {
    fn entity_id(&self) -> EntityId {
        self.id.clone()
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn options(&self) -> Value {
        self.options.clone()
    }

    fn is_device(&self) -> bool {
        matches!(self.id, EntityId::Ieee(_))
    }

    fn event_value(&self) -> Value {
        json!({"ID": self.id.as_json(), "name": self.name})
    }
}

struct Harness {
    file: std::path::PathBuf,
    settings: Arc<Mutex<Settings>>,
    event_bus: Arc<Mutex<EventBus>>,
    logger: Arc<Mutex<Logger>>,
    state: Arc<Mutex<State>>,
}

fn harness(configuration: Value) -> Harness {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let file = std::env::temp_dir().join(format!(
        "z2m-state-{}-{}-{id}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let settings = Arc::new(Mutex::new(Settings::from_config(configuration)));
    let event_bus = Arc::new(Mutex::new(EventBus::new()));
    let mut logger = Logger::new();
    logger.level = "debug".to_string();
    let logger = Arc::new(Mutex::new(logger));
    let state = Arc::new(Mutex::new(State::with_file(
        settings.clone(),
        event_bus.clone(),
        logger.clone(),
        file.clone(),
    )));

    Harness {
        file,
        settings,
        event_bus,
        logger,
        state,
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.file);
    }
}

#[tokio::test]
async fn loads_state_from_file() {
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    std::fs::write(
        &harness.file,
        r#"{"0x0017880104e45519":{"state":"ON"},"1":{"brightness":10}}"#,
    )
    .unwrap();

    State::start(&harness.state);
    let state = harness.state.lock().unwrap();

    assert!(state.exists(&EntityId::Ieee("0x0017880104e45519".to_string())));
    assert_eq!(
        state.get(&EntityId::Ieee("0x0017880104e45519".to_string())),
        json!({"state": "ON"})
    );
    assert_eq!(state.get(&EntityId::Group(1)), json!({"brightness": 10}));
    assert_eq!(
        harness.logger.lock().unwrap().winston_logs.last(),
        Some(&(
            "debug".to_string(),
            format!("z2m: Loaded state from file {}", harness.file.display())
        ))
    );
}

#[tokio::test]
async fn logs_missing_state_file() {
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));

    State::start(&harness.state);

    assert_eq!(harness.state.lock().unwrap().entries().len(), 0);
    assert!(harness
        .logger
        .lock()
        .unwrap()
        .winston_logs
        .iter()
        .any(|(_, message)| {
            message
                == &format!(
                    "z2m: Can't load state from file {} (doesn't exist)",
                    harness.file.display()
                )
        }));
}

#[tokio::test]
async fn corrupt_state_file_is_ignored() {
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    std::fs::write(&harness.file, "{not json").unwrap();

    State::start(&harness.state);

    assert_eq!(harness.state.lock().unwrap().entries().len(), 0);
    assert!(harness
        .logger
        .lock()
        .unwrap()
        .winston_logs
        .iter()
        .any(|(_, message)| {
            message.starts_with(&format!(
                "z2m: Failed to load state from file {} (corrupt file?)",
                harness.file.display()
            ))
        }));
}

#[tokio::test]
async fn saves_state_with_javascript_indentation() {
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    State::start(&harness.state);

    let device = TestEntity::device("0x0017880104e45519");
    let group = TestEntity::group(1);
    harness
        .state
        .lock()
        .unwrap()
        .set(&device, json!({"state": "ON"}), None);
    harness
        .state
        .lock()
        .unwrap()
        .set(&group, json!({"brightness": 10}), None);
    harness.state.lock().unwrap().stop(None);

    let saved = std::fs::read_to_string(&harness.file).unwrap();
    assert_eq!(
        saved,
        "{\n    \"0x0017880104e45519\": {\n        \"state\": \"ON\"\n    },\n    \"1\": {\n        \"brightness\": 10\n    }\n}"
    );
}

#[tokio::test]
async fn does_not_save_when_persistent_cache_is_disabled() {
    let harness = harness(json!({
        "mqtt": {"server": "localhost"},
        "advanced": {"cache_state_persistent": false},
    }));
    State::start(&harness.state);

    harness.state.lock().unwrap().set(
        &TestEntity::device("0x0017880104e45519"),
        json!({"state": "ON"}),
        None,
    );
    harness.state.lock().unwrap().stop(None);

    assert!(!harness.file.exists());
    assert!(harness
        .logger
        .lock()
        .unwrap()
        .winston_logs
        .iter()
        .any(|(_, message)| message == "z2m: Not saving state"));
}

#[tokio::test]
async fn set_merges_deep_and_returns_unfiltered_state() {
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    let mut state = harness.state.lock().unwrap();
    let device = TestEntity::device("0x0017880104e45519");

    state.set(&device, json!({"state": "ON"}), None);
    let to_state = state.set(&device, json!({"brightness": 10}), None);

    assert_eq!(to_state, json!({"state": "ON", "brightness": 10}));
    assert_eq!(
        state.get(&EntityId::Ieee("0x0017880104e45519".to_string())),
        json!({"state": "ON", "brightness": 10})
    );
}

#[tokio::test]
async fn set_emits_state_change_event() {
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    let received = Arc::new(Mutex::new(Vec::new()));
    let sink = received.clone();
    harness.event_bus.lock().unwrap().on(
        Event::StateChange,
        1,
        Arc::new(move |data| {
            sink.lock().unwrap().push(data);
        }),
    );

    let device = TestEntity::device("0x0017880104e45519");
    let mut state = harness.state.lock().unwrap();
    state.set(&device, json!({"state": "ON"}), None);
    state.set(
        &device,
        json!({"brightness": 10}),
        Some("device_publish".to_string()),
    );

    let received = received.lock().unwrap();
    assert_eq!(received.len(), 2);
    assert_eq!(
        received[1]["entity"],
        json!({"ID": "0x0017880104e45519", "name": "0x0017880104e45519"})
    );
    assert_eq!(received[1]["from"], json!({"state": "ON"}));
    assert_eq!(received[1]["to"], json!({"state": "ON", "brightness": 10}));
    assert_eq!(received[1]["update"], json!({"brightness": 10}));
    assert_eq!(received[1]["reason"], json!("device_publish"));
    assert!(received[0].get("reason").is_none());
}

#[tokio::test]
async fn cache_ignores_actions_and_entity_filtered_cache() {
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    let device = TestEntity::device("0x0017880104e45519")
        .with_filtered_cache(&["custom_property", "no_occupancy_since"]);
    let mut state = harness.state.lock().unwrap();

    let to_state = state.set(
        &device,
        json!({
            "state": "ON",
            "action": "toggle",
            "action_single": "single",
            "button": "1",
            "transition_time": 1,
            "elapsed": 2,
            "custom_property": 3,
            "step_size": 4,
        }),
        None,
    );

    // `set` returns the unfiltered state, `get` the cached (filtered) one.
    assert_eq!(to_state["action"], json!("toggle"));
    assert_eq!(
        state.get(&EntityId::Ieee("0x0017880104e45519".to_string())),
        json!({"state": "ON"})
    );

    // `action_.*` is a regex, not a literal property name.
    state.set(&device, json!({"action_double": "double"}), None);
    assert!(state
        .get(&EntityId::Ieee("0x0017880104e45519".to_string()))
        .get("action_double")
        .is_none());
}

#[tokio::test]
async fn stop_removes_states_of_devices_that_left_the_network() {
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    std::fs::write(
        &harness.file,
        r#"{"0x0017880104e45519":{"state":"ON"},"0x0017880104e45520":{"state":"OFF"},"1":{"brightness":1}}"#,
    )
    .unwrap();

    State::start(&harness.state);
    let resolve = |ieee: &str| ieee == "0x0017880104e45519";
    harness.state.lock().unwrap().stop(Some(&resolve));

    let saved: Value =
        serde_json::from_str(&std::fs::read_to_string(&harness.file).unwrap()).unwrap();
    assert_eq!(
        saved,
        json!({"0x0017880104e45519": {"state": "ON"}, "1": {"brightness": 1}})
    );
}

#[tokio::test]
async fn clear_and_remove() {
    let harness = harness(json!({"mqtt": {"server": "localhost"}}));
    let mut state = harness.state.lock().unwrap();
    let device = TestEntity::device("0x0017880104e45519");
    state.set(&device, json!({"state": "ON"}), None);

    assert!(state.remove(&EntityId::Ieee("0x0017880104e45519".to_string())));
    assert!(!state.remove(&EntityId::Ieee("0x0017880104e45519".to_string())));

    state.set(&device, json!({"state": "ON"}), None);
    state.clear();
    assert_eq!(state.entries().len(), 0);

    // `settings`/`event_bus` handles stay alive for the harness lifetime.
    assert!(!harness.settings.lock().unwrap().get().is_null());
}
