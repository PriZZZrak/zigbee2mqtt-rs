// Original: .upstream/zigbee2mqtt/test/extensions/availability.test.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::sync::{Arc, Mutex};

#[path = "support/mock_adapter.rs"]
mod mock_adapter;
use mock_adapter::MockAdapter;
use std::time::Duration;

use serde_json::json;
use zigbee_apiarist::controller::controller::{Controller, ControllerOptions};
use zigbee_apiarist::controller::database::{Database, DeviceType};
use zigbee_apiarist::controller::model::device::{Device as ZhDevice, InterviewState};
use zigbee_apiarist::controller::model::entity::inject_database;
use zigbee_apiarist::controller::model::group::Group as ZhGroup;
use zigbee_apiarist::controller::model::test_support::serial;
use zigbee_apiarist::models::network_options::NetworkOptions;

use z2m_rs::event_bus::EventBus;
use z2m_rs::extension::availability::Availability;
use z2m_rs::extension::extension::{Extension, ExtensionContext};
use z2m_rs::model::entity::PublishEntityState;
use z2m_rs::mqtt::Mqtt;
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::zigbee::Zigbee;

const IEEE: &str = "0x0017880104e45519";

struct Harness {
    adapter: Arc<MockAdapter>,
    context: ExtensionContext,
    mqtt: Arc<tokio::sync::Mutex<Mqtt>>,
    event_bus: Arc<Mutex<EventBus>>,
    zigbee: Arc<Zigbee>,
    database_path: std::path::PathBuf,
    state_path: std::path::PathBuf,
}

fn temp(name: &str) -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir().join(format!("z2m-avail-{name}-{}-{id}", std::process::id()))
}

fn harness(availability_enabled: bool, device_availability: serde_json::Value) -> Harness {
    harness_with(
        availability_enabled,
        device_availability,
        DeviceType::EndDevice,
        "Battery",
        1234,
    )
}

fn ping_harness(device_availability: serde_json::Value) -> Harness {
    harness_with(
        true,
        device_availability,
        DeviceType::Router,
        "Mains (single phase)",
        1234,
    )
}

fn harness_with(
    availability_enabled: bool,
    device_availability: serde_json::Value,
    r#type: DeviceType,
    power_source: &str,
    network_address: u16,
) -> Harness {
    let database_path = temp("db");
    inject_database(Arc::new(Mutex::new(Database::open(&database_path))));
    let adapter = MockAdapter::new();
    zigbee_apiarist::controller::model::entity::Entity::inject_adapter(adapter.clone());
    ZhDevice::reset_cache();
    ZhGroup::reset_cache();
    ZhDevice::create(
        DeviceType::Coordinator,
        "0x00124b0024c16b95",
        0,
        None,
        None,
        None,
        None,
        InterviewState::Successful,
        None,
    )
    .unwrap();
    let device = ZhDevice::create(
        r#type,
        IEEE,
        network_address,
        None,
        Some("Test".to_string()),
        Some(power_source.to_string()),
        Some("TEST".to_string()),
        InterviewState::Successful,
        None,
    )
    .unwrap();
    device.create_endpoint(1).unwrap();

    let controller = Arc::new(
        Controller::new(ControllerOptions {
            network: NetworkOptions {
                pan_id: 0x1a62,
                extended_pan_id: vec![0xdd; 8],
                channel_list: vec![11],
                network_key: vec![1, 3, 5, 7, 9, 11, 13, 15, 0, 2, 4, 6, 8, 10, 12, 13],
                network_key_distribute: true,
                has_default_extended_pan_id: false,
            },
            transport: json!({}),
            database_path: database_path.display().to_string(),
            database_backup_path: None,
            backup_path: None,
            adapter: json!({}),
            accept_joining_device_handler: None,
        })
        .unwrap(),
    );

    let settings = Arc::new(Mutex::new(Settings::from_config(json!({
        "mqtt": {"server": "localhost"},
        "availability": {"enabled": availability_enabled},
        "devices": {IEEE: {"friendly_name": "lamp", "availability": device_availability}},
    }))));
    let event_bus = Arc::new(Mutex::new(EventBus::new()));
    let logger = Arc::new(Mutex::new(Logger::new()));
    let zigbee = Zigbee::with_controller(
        controller,
        event_bus.clone(),
        settings.clone(),
        logger.clone(),
    );
    let _ = zigbee.resolve_device(IEEE);
    let state_path = temp("state.json");
    let state = Arc::new(Mutex::new(State::with_file(
        settings.clone(),
        event_bus.clone(),
        logger.clone(),
        state_path.clone(),
    )));
    let mqtt = Arc::new(tokio::sync::Mutex::new(Mqtt::new(event_bus.clone())));
    let publish: PublishEntityState = Arc::new(|_e, _p, _r| Box::pin(async {}));

    Harness {
        adapter,
        context: ExtensionContext {
            zigbee: zigbee.clone(),
            mqtt: mqtt.clone(),
            state,
            publish_entity_state: publish,
            event_bus: event_bus.clone(),
            logger,
            settings,
            enable_disable_extension: Arc::new(|_, _| Box::pin(async { Ok(()) })),
            restart: Arc::new(|| Box::pin(async {})),
            add_extension: Arc::new(|_| Box::pin(async {})),
            key: 5,
        },
        mqtt,
        event_bus,
        zigbee,
        database_path,
        state_path,
    }
}

fn add_router(harness: &Harness, ieee: &str, name: &str, network_address: u16) {
    harness.context.settings.lock().unwrap().configuration["devices"][ieee] =
        serde_json::json!({"friendly_name": name});
    let device = ZhDevice::create(
        DeviceType::Router,
        ieee,
        network_address,
        None,
        Some("Test".to_string()),
        Some("Mains (single phase)".to_string()),
        Some("TEST".to_string()),
        InterviewState::Successful,
        None,
    )
    .unwrap();
    device.create_endpoint(1).unwrap();
    let _ = harness.zigbee.resolve_device(ieee);
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.database_path);
        let _ = std::fs::remove_file(&self.state_path);
    }
}

#[tokio::test]
async fn publishes_offline_then_online_after_last_seen() {
    let _guard = serial();
    let harness = harness(true, json!({"timeout": 1}));
    let availability = Availability::new(harness.context.clone());
    // Device has no lastSeen yet.
    availability.clone().start().await.unwrap();

    {
        let mqtt = harness.mqtt.lock().await;
        let topic = format!("zigbee2mqtt/lamp/availability");
        let offline = mqtt
            .attempted
            .iter()
            .find(|(published, _, _)| published == &topic)
            .expect("availability published");
        assert_eq!(offline.1, r#"{"state":"offline"}"#);
        assert!(offline.2.retain);
        assert_eq!(offline.2.qos, 1);
    }

    let device = harness.zigbee.resolve_device(IEEE).unwrap();
    device.zh.update_last_seen();
    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_last_seen_changed(json!({
            "device": {"ieeeAddr": IEEE, "name": "lamp"},
            "reason": "messageEmitted",
        }));
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mqtt = harness.mqtt.lock().await;
    let topic = "zigbee2mqtt/lamp/availability";
    let last = mqtt
        .attempted
        .iter()
        .filter(|(published, _, _)| published == topic)
        .last()
        .expect("online published");
    assert_eq!(last.1, r#"{"state":"online"}"#);

    availability.clone().stop().await.unwrap();
}

#[tokio::test]
async fn availability_disabled_via_device_option() {
    let _guard = serial();
    let harness = harness(true, json!(false));
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();

    let mqtt = harness.mqtt.lock().await;
    assert!(
        mqtt.attempted.is_empty(),
        "unexpected: {:?}",
        mqtt.attempted
            .iter()
            .map(|(t, p, _)| (t.clone(), p.clone()))
            .collect::<Vec<_>>()
    );
    availability.stop().await.unwrap();
}

#[tokio::test]
async fn rename_publishes_empty_retained_availability() {
    let _guard = serial();
    let harness = harness(true, json!({"timeout": 1}));
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    harness.mqtt.lock().await.attempted.clear();

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_entity_renamed(json!({
            "entity": {"ID": IEEE, "ieeeAddr": IEEE, "name": "lamp"},
            "from": "old",
            "to": "lamp",
        }));
    tokio::time::sleep(Duration::from_millis(200)).await;

    let mqtt = harness.mqtt.lock().await;
    assert!(mqtt.attempted.iter().any(|(topic, payload, options)| topic
        == "zigbee2mqtt/old/availability"
        && payload.is_empty()
        && options.retain
        && options.qos == 1));
    assert!(mqtt
        .attempted
        .iter()
        .any(|(topic, _, _)| topic == "zigbee2mqtt/lamp/availability"));

    availability.stop().await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn passive_device_publishes_offline_after_timeout_and_online_on_last_seen() {
    let _guard = serial();
    let harness = harness(true, json!({"timeout": 1}));
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    tokio::task::yield_now().await;

    let device = harness.zigbee.resolve_device(IEEE).unwrap();
    device.zh.update_last_seen();
    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_last_seen_changed(json!({
            "device": {"ieeeAddr": IEEE, "name": "lamp"},
            "reason": "messageEmitted",
        }));
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    device.zh.update_last_seen();

    {
        let mqtt = harness.mqtt.lock().await;
        let last = mqtt
            .attempted
            .iter()
            .filter(|(topic, _, _)| topic == "zigbee2mqtt/lamp/availability")
            .last()
            .expect("availability published");
        assert_eq!(last.1, r#"{"state":"online"}"#);
    }
    harness.mqtt.lock().await.attempted.clear();

    // Make the device stale on the wall clock; the clock below is paused, so
    // only the timer's own scheduled deadline can fire.
    device.zh.set_last_seen(Some(
        zigbee_apiarist::utils::datetime::now_ms() - 2 * 60 * 1000,
    ));

    // Passive timeout is 1 minute plus the upstream 1s grace.
    tokio::time::advance(Duration::from_secs(63)).await;
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }

    let mqtt = harness.mqtt.lock().await;
    let last = mqtt
        .attempted
        .iter()
        .filter(|(topic, _, _)| topic == "zigbee2mqtt/lamp/availability")
        .last()
        .expect("offline republished after timeout");
    assert_eq!(last.1, r#"{"state":"offline"}"#);
    assert!(last.2.retain);
    assert_eq!(last.2.qos, 1);
    drop(mqtt);

    availability.stop().await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn device_timeout_option_controls_the_timer() {
    let _guard = serial();
    let harness = harness(true, json!({"timeout": 5}));
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    tokio::task::yield_now().await;

    let device = harness.zigbee.resolve_device(IEEE).unwrap();
    device.zh.update_last_seen();
    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_last_seen_changed(json!({
            "device": {"ieeeAddr": IEEE, "name": "lamp"},
            "reason": "messageEmitted",
        }));
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    device.zh.update_last_seen();
    harness.mqtt.lock().await.attempted.clear();

    // The 5 minute device option wins over the 25h passive default: nothing is
    // published within the first 2 minutes.
    tokio::time::advance(Duration::from_secs(2 * 60)).await;
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    assert!(
        harness.mqtt.lock().await.attempted.is_empty(),
        "published before the configured timeout"
    );

    device.zh.set_last_seen(Some(
        zigbee_apiarist::utils::datetime::now_ms() - 6 * 60 * 1000,
    ));
    tokio::time::advance(Duration::from_secs(4 * 60)).await;
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    let mqtt = harness.mqtt.lock().await;
    assert!(mqtt.attempted.iter().any(|(topic, payload, _)| topic
        == "zigbee2mqtt/lamp/availability"
        && payload == r#"{"state":"offline"}"#));
    drop(mqtt);

    availability.stop().await.unwrap();
}

/// Case 16: `availability.enabled: false` publishes nothing (the per-entity
/// filter, not an early start-return, is the upstream behaviour).
#[tokio::test]
async fn availability_disabled_globally_publishes_nothing() {
    let _guard = serial();
    let harness = harness(false, json!(null));
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_last_seen_changed(json!({
            "device": {"ieeeAddr": IEEE, "name": "lamp"},
            "reason": "messageEmitted",
        }));
    tokio::time::sleep(Duration::from_millis(150)).await;

    let mqtt = harness.mqtt.lock().await;
    assert!(
        mqtt.attempted.is_empty(),
        "unexpected publishes: {:?}",
        mqtt.attempted
            .iter()
            .map(|(topic, payload, _)| (topic.clone(), payload.clone()))
            .collect::<Vec<_>>()
    );
    drop(mqtt);
    availability.stop().await.unwrap();
}

/// Case 24: a stopped availability instance cannot be restarted.
#[tokio::test]
async fn stopped_availability_cannot_be_restarted() {
    let _guard = serial();
    let harness = harness(true, json!({}));
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    availability.clone().stop().await.unwrap();

    let error = availability.start().await.expect_err("restart is refused");
    assert!(
        error.contains("cannot be restarted"),
        "unexpected error: {error}"
    );
}

/// Case 17: a device option enables availability even when it is globally off.
#[tokio::test]
async fn availability_can_be_enabled_for_a_single_device() {
    let _guard = serial();
    let harness = harness(false, json!(true));
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    tokio::task::yield_now().await;

    let mqtt = harness.mqtt.lock().await;
    assert!(
        mqtt.attempted.iter().any(
            |(topic, payload, _)| topic == "zigbee2mqtt/lamp/availability"
                && payload == r#"{"state":"offline"}"#
        ),
        "per-device override did not publish: {:?}",
        mqtt.attempted
            .iter()
            .map(|(topic, payload, _)| (topic.clone(), payload.clone()))
            .collect::<Vec<_>>()
    );
    drop(mqtt);
    availability.stop().await.unwrap();
}

const IEEE_B: &str = "0x0017880104e45521";
const IEEE_C: &str = "0x0017880104e45522";

fn read_count(harness: &Harness) -> usize {
    harness
        .adapter
        .endpoint_frames
        .lock()
        .unwrap()
        .iter()
        .filter(|frame| frame.command.name == "read")
        .count()
}

fn mark_seen(harness: &Harness, ieee: &str) {
    harness
        .zigbee
        .resolve_device(ieee)
        .unwrap()
        .zh
        .set_last_seen(Some(zigbee_apiarist::utils::datetime::now_ms()));
}

fn emit_last_seen(harness: &Harness, ieee: &str, name: &str) {
    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_last_seen_changed(json!({
            "device": {"ieeeAddr": ieee, "name": name},
            "reason": "messageEmitted",
        }));
}

async fn settle() {
    for _ in 0..40 {
        tokio::task::yield_now().await;
    }
}

/// Case 2: an enabled and unavailable active device is pinged on startup.
#[tokio::test]
async fn startup_pings_unavailable_active_device() {
    let _guard = serial();
    let harness = ping_harness(json!({"max_jitter": 0}));
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    assert!(
        read_count(&harness) >= 1,
        "the startup ping must read genBasic"
    );
    availability.stop().await.unwrap();
}

/// Case 3a: passive (battery) devices are not pinged on startup.
#[tokio::test]
async fn startup_does_not_ping_passive_devices() {
    let _guard = serial();
    let harness = harness(true, json!({"max_jitter": 0}));
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    assert_eq!(read_count(&harness), 0, "passive devices are not pinged");
    availability.stop().await.unwrap();
}

/// Case 3b: an available active device is not pinged on startup.
#[tokio::test]
async fn startup_does_not_ping_available_devices() {
    let _guard = serial();
    let harness = ping_harness(json!({"max_jitter": 0}));
    mark_seen(&harness, IEEE);
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    assert_eq!(read_count(&harness), 0, "available devices are not pinged");
    availability.stop().await.unwrap();
}

/// Case 12: disabled devices are not pinged.
#[tokio::test]
async fn startup_does_not_ping_disabled_devices() {
    let _guard = serial();
    let harness = ping_harness(json!({"max_jitter": 0}));
    harness.context.settings.lock().unwrap().configuration["devices"][IEEE]["disabled"] =
        json!(true);
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    assert_eq!(read_count(&harness), 0, "disabled devices are not pinged");
    assert!(harness.mqtt.lock().await.attempted.is_empty());
    availability.stop().await.unwrap();
}

/// Case 8: an available device whose ping fails is tried a second time before
/// it is reported offline.
#[tokio::test(start_paused = true)]
async fn ping_retries_once_for_available_devices() {
    let _guard = serial();
    let harness = ping_harness(json!({"timeout": 1, "max_jitter": 0}));
    mark_seen(&harness, IEEE);
    harness
        .adapter
        .fail_zcl
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    emit_last_seen(&harness, IEEE, "lamp");
    settle().await;
    tokio::time::advance(Duration::from_millis(61_000)).await;
    settle().await;
    // The retry waits 3 s between attempts.
    tokio::time::advance(Duration::from_secs(3)).await;
    settle().await;

    assert_eq!(read_count(&harness), 2, "two ping attempts");
    availability.stop().await.unwrap();
}

/// Case 7: a last-seen change restarts the active ping timer.
#[tokio::test(start_paused = true)]
async fn last_seen_resets_the_active_ping_timer() {
    let _guard = serial();
    let harness = ping_harness(json!({"timeout": 1, "max_jitter": 0}));
    mark_seen(&harness, IEEE);
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    emit_last_seen(&harness, IEEE, "lamp");
    settle().await;
    tokio::time::advance(Duration::from_secs(30)).await;
    settle().await;
    emit_last_seen(&harness, IEEE, "lamp");
    settle().await;

    // 70 s after start: the original timer (61 s) would have fired already.
    tokio::time::advance(Duration::from_secs(40)).await;
    settle().await;
    assert_eq!(read_count(&harness), 0, "timer was not reset");

    // 95 s after start / 65 s after the reset: the pinged timer fired.
    tokio::time::advance(Duration::from_secs(25)).await;
    settle().await;
    assert_eq!(read_count(&harness), 1, "reset timer did not ping");
    availability.stop().await.unwrap();
}

/// Case 14: leaving stops the ping timer.
#[tokio::test(start_paused = true)]
async fn device_leave_stops_the_ping_timer() {
    let _guard = serial();
    let harness = ping_harness(json!({"timeout": 1, "max_jitter": 0}));
    mark_seen(&harness, IEEE);
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_device_leave(json!({"ieeeAddr": IEEE}));
    settle().await;
    tokio::time::advance(Duration::from_secs(120)).await;
    settle().await;

    assert_eq!(read_count(&harness), 0);
    availability.stop().await.unwrap();
}

/// Case 15: removing the entity stops the ping timer.
#[tokio::test(start_paused = true)]
async fn entity_removed_stops_the_ping_timer() {
    let _guard = serial();
    let harness = ping_harness(json!({"timeout": 1, "max_jitter": 0}));
    mark_seen(&harness, IEEE);
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_entity_removed(json!({"entity": {"ieeeAddr": IEEE}}));
    settle().await;
    tokio::time::advance(Duration::from_secs(120)).await;
    settle().await;

    assert_eq!(read_count(&harness), 0);
    availability.stop().await.unwrap();
}

/// Case 6 (paused-clock variant): the passive timeout comes from the global
/// availability settings and schedules the offline publish.
#[tokio::test(start_paused = true)]
async fn global_passive_timeout_controls_the_timer() {
    let _guard = serial();
    let harness = harness(true, json!(null));
    harness.context.settings.lock().unwrap().configuration["availability"]["passive"] =
        json!({"timeout": 1});
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    emit_last_seen(&harness, IEEE, "lamp");
    settle().await;

    assert_eq!(
        availability.test_ping_delay(IEEE),
        Some(61_000),
        "global passive timeout of 1 minute plus 1 s"
    );
    availability.stop().await.unwrap();
}

/// Case 21: device availability is also published for groups.
#[tokio::test]
async fn publishes_availability_for_groups() {
    let _guard = serial();
    let harness = ping_harness(json!({"max_jitter": 0}));
    harness.context.settings.lock().unwrap().configuration["groups"]["1"] =
        json!({"friendly_name": "kitchen"});
    let group = harness
        .zigbee
        .zh_controller()
        .unwrap()
        .create_group(1)
        .unwrap();
    let device = harness.zigbee.resolve_device(IEEE).unwrap();
    group.add_member(&device.zh.endpoints()[0]);
    let _ = harness.zigbee.resolve_group(1);

    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    let mqtt = harness.mqtt.lock().await;
    assert!(
        mqtt.attempted.iter().any(|(topic, payload, _)| topic
            == "zigbee2mqtt/kitchen/availability"
            && payload == r#"{"state":"offline"}"#),
        "group availability missing: {:?}",
        mqtt.attempted
            .iter()
            .map(|(topic, payload, _)| (topic.clone(), payload.clone()))
            .collect::<Vec<_>>()
    );
    drop(mqtt);
    availability.stop().await.unwrap();
}

/// Case 22: stopping clears the ping queue and interrupts the running ping.
#[tokio::test(start_paused = true)]
async fn stop_clears_the_ping_queue() {
    let _guard = serial();
    let harness = ping_harness(json!({"timeout": 1, "max_jitter": 0}));
    for (ieee, name, network) in [
        (IEEE, "lamp", 1234),
        (IEEE_B, "lamp2", 1235),
        (IEEE_C, "lamp3", 1236),
    ] {
        if ieee != IEEE {
            add_router(&harness, ieee, name, network);
        }
        mark_seen(&harness, ieee);
    }
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    harness
        .adapter
        .delay_ms
        .store(1_000, std::sync::atomic::Ordering::SeqCst);
    for ieee in [IEEE_B, IEEE_C] {
        let device = harness.zigbee.resolve_device(ieee).unwrap();
        availability.test_add_to_ping_queue(device as Arc<dyn z2m_rs::model::entity::Entity>);
    }
    settle().await;
    availability.clone().stop().await.unwrap();
    tokio::time::advance(Duration::from_secs(60)).await;
    settle().await;

    assert_eq!(
        availability.test_ping_queue_len(),
        0,
        "queue was not cleared"
    );
    assert_eq!(read_count(&harness), 1, "only the in-flight ping started");
}

/// Case 23: the queue drains in a single run instead of re-entering per device.
#[tokio::test(start_paused = true)]
async fn ping_queue_drains_in_one_run() {
    let _guard = serial();
    let harness = ping_harness(json!({"timeout": 1, "max_jitter": 0}));
    for (ieee, name, network) in [
        (IEEE, "lamp", 1234),
        (IEEE_B, "lamp2", 1235),
        (IEEE_C, "lamp3", 1236),
    ] {
        if ieee != IEEE {
            add_router(&harness, ieee, name, network);
        }
        mark_seen(&harness, ieee);
    }
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    for ieee in [IEEE, IEEE_B, IEEE_C] {
        let device = harness.zigbee.resolve_device(ieee).unwrap();
        availability.test_add_to_ping_queue(device as Arc<dyn z2m_rs::model::entity::Entity>);
    }
    settle().await;
    for _ in 0..2 {
        tokio::time::advance(Duration::from_secs(3)).await;
        settle().await;
    }

    assert_eq!(read_count(&harness), 3, "each queued device pinged once");
    assert_eq!(availability.test_ping_queue_len(), 0, "queue not drained");
    assert_eq!(
        availability.test_execute_count(),
        3,
        "one executeNext per addToPingQueue"
    );
    availability.stop().await.unwrap();
}

/// Case 25: pings are jittered within the configured maximum.
#[tokio::test]
async fn pings_are_jittered() {
    let _guard = serial();
    let harness = ping_harness(json!({"timeout": 1, "max_jitter": 60_000}));
    mark_seen(&harness, IEEE);
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    emit_last_seen(&harness, IEEE, "lamp");
    settle().await;

    let delay = availability
        .test_ping_delay(IEEE)
        .expect("ping delay scheduled");
    assert!(
        (61_000..121_000).contains(&delay),
        "jittered delay out of range: {delay}"
    );
    availability.stop().await.unwrap();
}

/// Case 26: a successful ping does not start a backoff.
#[tokio::test(start_paused = true)]
async fn ping_success_does_not_trigger_backoff() {
    let _guard = serial();
    let harness = ping_harness(json!({"timeout": 1, "max_jitter": 0}));
    mark_seen(&harness, IEEE);
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    emit_last_seen(&harness, IEEE, "lamp");
    settle().await;
    assert_eq!(availability.test_ping_delay(IEEE), Some(61_000));
    tokio::time::advance(Duration::from_millis(61_000)).await;
    settle().await;

    assert_eq!(read_count(&harness), 1);
    assert_eq!(
        availability.test_ping_backoff(IEEE),
        None,
        "successful pings must not back off"
    );
    availability.stop().await.unwrap();
}

/// Case 27: successive ping failures grow the backoff.
#[tokio::test(start_paused = true)]
async fn ping_failures_grow_the_backoff() {
    let _guard = serial();
    let harness = ping_harness(json!({"timeout": 1, "max_jitter": 0}));
    mark_seen(&harness, IEEE);
    harness
        .adapter
        .fail_zcl
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    emit_last_seen(&harness, IEEE, "lamp");
    settle().await;
    tokio::time::advance(Duration::from_millis(61_000)).await;
    settle().await;
    tokio::time::advance(Duration::from_secs(3)).await;
    settle().await;

    // Available device: two attempts, then the multiplier becomes 1.5.
    assert_eq!(read_count(&harness), 2);
    assert_eq!(availability.test_ping_backoff(IEEE), Some(1.5));
    assert_eq!(availability.test_ping_delay(IEEE), Some(91_500));

    tokio::time::advance(Duration::from_millis(91_500)).await;
    settle().await;
    tokio::time::advance(Duration::from_secs(3)).await;
    settle().await;
    assert_eq!(read_count(&harness), 4);
    assert_eq!(
        availability.test_ping_backoff(IEEE),
        Some(2.25),
        "a second failure multiplies the backoff by 1.5"
    );
    availability.stop().await.unwrap();
}

/// Case 28: a last-seen change resets the backoff.
#[tokio::test(start_paused = true)]
async fn last_seen_resets_the_backoff() {
    let _guard = serial();
    let harness = ping_harness(json!({"timeout": 1, "max_jitter": 0}));
    mark_seen(&harness, IEEE);
    harness
        .adapter
        .fail_zcl
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    emit_last_seen(&harness, IEEE, "lamp");
    settle().await;
    tokio::time::advance(Duration::from_millis(61_000)).await;
    settle().await;
    tokio::time::advance(Duration::from_secs(3)).await;
    settle().await;
    assert_eq!(availability.test_ping_backoff(IEEE), Some(1.5));

    mark_seen(&harness, IEEE);
    emit_last_seen(&harness, IEEE, "lamp");
    settle().await;
    assert_eq!(availability.test_ping_backoff(IEEE), None);
    assert_eq!(availability.test_ping_delay(IEEE), Some(61_000));
    availability.stop().await.unwrap();
}

/// Case 29: reaching the configured backoff threshold pauses pings and a
/// last-seen change unpauses them again.
#[tokio::test(start_paused = true)]
async fn backoff_pauses_at_the_configured_threshold() {
    let _guard = serial();
    let harness = ping_harness(json!({
        "timeout": 1,
        "max_jitter": 0,
        "pause_on_backoff_gt": 1.5,
    }));
    mark_seen(&harness, IEEE);
    harness
        .adapter
        .fail_zcl
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    emit_last_seen(&harness, IEEE, "lamp");
    settle().await;
    tokio::time::advance(Duration::from_millis(61_000)).await;
    settle().await;
    tokio::time::advance(Duration::from_secs(3)).await;
    settle().await;
    assert_eq!(availability.test_ping_backoff(IEEE), Some(1.5));
    assert!(!availability.test_backoff_paused(IEEE));

    tokio::time::advance(Duration::from_millis(91_500)).await;
    settle().await;
    tokio::time::advance(Duration::from_secs(3)).await;
    settle().await;
    assert!(
        availability.test_backoff_paused(IEEE),
        "device must be paused at the threshold"
    );

    mark_seen(&harness, IEEE);
    emit_last_seen(&harness, IEEE, "lamp");
    settle().await;
    assert!(!availability.test_backoff_paused(IEEE));
    assert_eq!(availability.test_ping_delay(IEEE), Some(61_000));
    availability.stop().await.unwrap();
}

/// Case 30: the ping delay is clamped to the maximum supported timeout.
#[tokio::test(start_paused = true)]
async fn ping_delay_is_clamped_to_max_timeout() {
    let _guard = serial();
    let harness = ping_harness(json!({"timeout": 40_000, "max_jitter": 0}));
    mark_seen(&harness, IEEE);
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    assert_eq!(
        availability.test_ping_delay(IEEE),
        Some(2_147_483_647),
        "40 000 minutes must clamp to the 32-bit timer maximum"
    );
    tokio::time::advance(Duration::from_secs(1)).await;
    settle().await;
    assert_eq!(read_count(&harness), 0, "clamped delay must not loop");
    availability.stop().await.unwrap();
}

/// Case 31: backoff can be disabled per device.
#[tokio::test(start_paused = true)]
async fn backoff_can_be_disabled() {
    let _guard = serial();
    let harness = ping_harness(json!({
        "timeout": 1,
        "max_jitter": 0,
        "backoff": false,
    }));
    mark_seen(&harness, IEEE);
    harness
        .adapter
        .fail_zcl
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    emit_last_seen(&harness, IEEE, "lamp");
    settle().await;
    tokio::time::advance(Duration::from_millis(61_000)).await;
    settle().await;
    tokio::time::advance(Duration::from_secs(3)).await;
    settle().await;

    assert_eq!(availability.test_ping_backoff(IEEE), None);
    assert_eq!(availability.test_ping_delay(IEEE), Some(61_000));
    tokio::time::advance(Duration::from_millis(61_000)).await;
    settle().await;
    assert!(
        read_count(&harness) >= 2,
        "pings continue at the normal rate"
    );
    availability.stop().await.unwrap();
}

/// Case 6: the passive default timeout is 25 hours.
#[tokio::test(start_paused = true)]
async fn default_passive_timeout_is_25_hours() {
    let _guard = serial();
    let harness = harness(true, json!(null));
    let availability = Availability::new(harness.context.clone());
    availability.clone().start().await.unwrap();
    settle().await;

    emit_last_seen(&harness, IEEE, "lamp");
    settle().await;

    assert_eq!(
        availability.test_ping_delay(IEEE),
        Some(25 * 60 * 60 * 1_000 + 1_000),
        "default passive timeout (1500 minutes) plus 1 s"
    );
    availability.stop().await.unwrap();
}
