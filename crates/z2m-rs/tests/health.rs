// Original: .upstream/zigbee2mqtt/test/extensions/health.test.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::sync::{Arc, Mutex};

use serde_json::json;

use z2m_rs::event_bus::{DeviceStats, EventBus, LastSeenChanges};
use z2m_rs::extension::extension::ExtensionContext;
use z2m_rs::extension::health::Health;
use z2m_rs::model::entity::PublishEntityState;
use z2m_rs::mqtt::Mqtt;
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::zigbee::Zigbee;

fn context(
    reset_on_check: bool,
) -> (
    ExtensionContext,
    Arc<tokio::sync::Mutex<Mqtt>>,
    Arc<Mutex<EventBus>>,
) {
    let settings = Arc::new(Mutex::new(Settings::from_config(json!({
        "mqtt": {"server": "localhost"},
        "health": {"interval": 10, "reset_on_check": reset_on_check},
    }))));
    let event_bus = Arc::new(Mutex::new(EventBus::new()));
    let settings_handle = settings.clone();
    let logger = Arc::new(Mutex::new(Logger::new()));
    let state_path = std::env::temp_dir().join(format!(
        "z2m-health-state-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let state = Arc::new(Mutex::new(State::with_file(
        settings.clone(),
        event_bus.clone(),
        logger.clone(),
        state_path,
    )));
    let mqtt = Arc::new(tokio::sync::Mutex::new(Mqtt::new(event_bus.clone())));
    let zigbee = Zigbee::new(event_bus.clone(), settings, logger.clone());
    let publish: PublishEntityState = Arc::new(|_e, _p, _r| Box::pin(async {}));

    (
        ExtensionContext {
            zigbee,
            mqtt: mqtt.clone(),
            state,
            publish_entity_state: publish,
            event_bus: event_bus.clone(),
            logger,
            settings: settings_handle,
            enable_disable_extension: Arc::new(|_, _| Box::pin(async { Ok(()) })),
            restart: Arc::new(|| Box::pin(async {})),
            add_extension: Arc::new(|_| Box::pin(async {})),
            key: 1,
        },
        mqtt,
        event_bus,
    )
}

#[tokio::test]
async fn publishes_bridge_health_payload() {
    let (context, mqtt, event_bus) = context(false);
    {
        let mut bus = event_bus.lock().unwrap();
        bus.stats.mqtt.published = 5;
        bus.stats.mqtt.received = 2;
        bus.stats.devices.insert(
            "0x0017880104e45519".to_string(),
            DeviceStats {
                last_seen_changes: Some(LastSeenChanges {
                    messages: 10,
                    first: zigbee_apiarist::utils::datetime::now_ms() - 1000,
                }),
                leave_counts: 1,
                network_address_changes: 3,
            },
        );
    }

    let health = Health::new(context);
    health.check_health().await;

    let mqtt = mqtt.lock().await;
    assert_eq!(mqtt.attempted.len(), 1);
    assert_eq!(mqtt.attempted[0].0, "zigbee2mqtt/bridge/health");
    assert!(mqtt.attempted[0].2.retain);
    assert_eq!(mqtt.attempted[0].2.qos, 1);

    let payload: serde_json::Value = serde_json::from_str(&mqtt.attempted[0].1).unwrap();
    assert!(payload["response_time"].is_number());
    assert!(payload["os"]["memory_used_mb"].is_number());
    assert!(payload["os"]["memory_percent"].is_number());
    assert!(payload["process"]["uptime_sec"].is_number());
    assert_eq!(payload["mqtt"]["published"], 5);
    assert_eq!(payload["mqtt"]["received"], 2);
    assert!(payload["mqtt"]["queued"].is_number());
    assert_eq!(payload["devices"]["0x0017880104e45519"]["messages"], 10);
    assert_eq!(payload["devices"]["0x0017880104e45519"]["leave_count"], 1);
    assert_eq!(
        payload["devices"]["0x0017880104e45519"]["network_address_changes"],
        3
    );
    let mps = payload["devices"]["0x0017880104e45519"]["messages_per_sec"]
        .as_f64()
        .unwrap();
    assert!(mps > 0.0);
}

#[tokio::test]
async fn reset_on_check_clears_stats() {
    let (context, _mqtt, event_bus) = context(true);
    event_bus.lock().unwrap().stats.mqtt.published = 7;

    let health = Health::new(context);
    health.check_health().await;

    assert_eq!(event_bus.lock().unwrap().stats.mqtt.published, 0);
}

#[test]
fn clear_stats_clears_devices_and_mqtt() {
    let (context, _mqtt, event_bus) = context(false);
    event_bus.lock().unwrap().stats.mqtt.received = 4;
    let health = Health::new(context);
    health.clear_stats();
    let bus = event_bus.lock().unwrap();
    assert!(bus.stats.devices.is_empty());
    assert_eq!(bus.stats.mqtt.received, 0);
}
