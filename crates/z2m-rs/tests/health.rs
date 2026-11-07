// Port of .upstream/zigbee2mqtt/test/extensions/health.test.ts
//
// Upstream uses the controller event bus and fake timers. The Rust port drives
// an explicit `Health` model and advances time through `advance_*`.

use serde_json::{json, Value};
use z2m_rs::extension::health::Health;

const BULB_COLOR: &str = "0x000b57fffec6a5b3";
const BULB_COLOR_2: &str = "0x000b57fffec6a5b4";

fn assert_health_shape(value: &Value) {
    assert!(value["response_time"].is_number());
    let load_average = value["os"]["load_average"].as_array().unwrap();
    assert_eq!(load_average.len(), 3);
    assert!(load_average.iter().all(Value::is_number));
    assert!(value["os"]["memory_used_mb"].is_number());
    assert!(value["os"]["memory_percent"].is_number());
    assert!(value["process"]["uptime_sec"].is_number());
    assert!(value["process"]["memory_used_mb"].is_number());
    assert!(value["process"]["memory_percent"].is_number());
    assert_eq!(value["mqtt"]["connected"], json!(true));
    assert_eq!(value["mqtt"]["queued"], json!(0));
    assert!(value["mqtt"]["published"].is_number());
}

fn last(health: &Health) -> &Value {
    health.published.last().expect("health must be published")
}

#[test]
#[ignore = "not implemented yet"]
fn checks_health_at_default_interval() {
    let mut health = Health::new(10, false);

    health.on_last_seen_changed(BULB_COLOR);
    health.on_last_seen_changed(BULB_COLOR_2);
    health.on_device_leave(BULB_COLOR);
    health.on_device_joined(BULB_COLOR);
    health.on_device_network_address_changed(BULB_COLOR);
    health.advance_seconds(1);
    health.on_last_seen_changed(BULB_COLOR);
    health.advance_seconds(1);
    health.on_last_seen_changed(BULB_COLOR);
    health.advance_seconds(1);
    health.on_last_seen_changed(BULB_COLOR);
    health.on_mqtt_message("zigbee2mqtt/mock", "mocked");
    health.advance_minutes(11);

    assert_eq!(health.published.len(), 1);
    let value = last(&health);
    assert_health_shape(value);
    assert_eq!(
        value,
        &json!({
            "response_time": value["response_time"],
            "os": value["os"],
            "process": value["process"],
            "mqtt": {"connected": true, "queued": 0, "published": value["mqtt"]["published"], "received": 1},
            "devices": {
                "0x000b57fffec6a5b3": {
                    "leave_count": 1,
                    "messages": 4,
                    "messages_per_sec": 0.0067,
                    "network_address_changes": 1,
                },
                "0x000b57fffec6a5b4": {
                    "leave_count": 0,
                    "messages": 1,
                    "messages_per_sec": 0.0017,
                    "network_address_changes": 0,
                },
            },
        })
    );
    assert_eq!(health.publish_options, vec![(true, 1)]);

    health.published.clear();
    health.publish_options.clear();
    health.on_mqtt_message("zigbee2mqtt/mock2", "mocked2");
    health.advance_minutes(11);

    assert_eq!(health.published.len(), 1);
    let value = last(&health);
    assert_eq!(value["mqtt"]["received"], json!(2));
    assert_eq!(
        value["devices"],
        json!({
            "0x000b57fffec6a5b3": {
                "leave_count": 1,
                "messages": 4,
                "messages_per_sec": 0.0033,
                "network_address_changes": 1,
            },
            "0x000b57fffec6a5b4": {
                "leave_count": 0,
                "messages": 1,
                "messages_per_sec": 0.0008,
                "network_address_changes": 0,
            },
        })
    );
    assert_eq!(health.publish_options, vec![(true, 1)]);
}

#[test]
#[ignore = "not implemented yet"]
fn checks_health_at_given_interval() {
    let mut health = Health::new(20, false);

    health.advance_minutes(11);
    assert_eq!(health.published.len(), 0);

    health.advance_minutes(10);
    assert_eq!(health.published.len(), 1);
    let value = last(&health);
    assert_health_shape(value);
    assert_eq!(value["mqtt"]["received"], json!(0));
    assert_eq!(value["devices"], json!({}));
    assert_eq!(health.publish_options, vec![(true, 1)]);
}

#[test]
#[ignore = "not implemented yet"]
fn init_device_health_from_leave() {
    let mut health = Health::new(10, false);

    health.on_device_leave(BULB_COLOR);
    health.on_device_joined(BULB_COLOR);
    health.advance_minutes(11);

    assert_eq!(health.published.len(), 1);
    let value = last(&health);
    assert_health_shape(value);
    assert_eq!(value["mqtt"]["received"], json!(0));
    assert_eq!(
        value["devices"],
        json!({
            "0x000b57fffec6a5b3": {
                "leave_count": 1,
                "messages": 0,
                "messages_per_sec": 0,
                "network_address_changes": 0,
            },
        })
    );
    assert_eq!(health.publish_options, vec![(true, 1)]);
}

#[test]
#[ignore = "not implemented yet"]
fn init_device_health_from_network_address_change() {
    let mut health = Health::new(10, false);

    health.on_device_network_address_changed(BULB_COLOR);
    health.advance_minutes(11);

    assert_eq!(health.published.len(), 1);
    let value = last(&health);
    assert_health_shape(value);
    assert_eq!(value["mqtt"]["received"], json!(0));
    assert_eq!(
        value["devices"],
        json!({
            "0x000b57fffec6a5b3": {
                "leave_count": 0,
                "messages": 0,
                "messages_per_sec": 0,
                "network_address_changes": 1,
            },
        })
    );
    assert_eq!(health.publish_options, vec![(true, 1)]);
}

#[test]
#[ignore = "not implemented yet"]
fn checks_health_then_resets_possible_stats() {
    let mut health = Health::new(10, true);

    health.on_last_seen_changed(BULB_COLOR);
    // coverage: no time diff between first/last
    health.on_last_seen_changed(BULB_COLOR);
    health.on_last_seen_changed(BULB_COLOR_2);
    health.advance_minutes(11);

    assert_eq!(health.published.len(), 1);
    let value = last(&health);
    assert_health_shape(value);
    assert_eq!(value["mqtt"]["received"], json!(0));
    assert_eq!(
        value["devices"],
        json!({
            "0x000b57fffec6a5b3": {
                "leave_count": 0,
                "messages": 2,
                "messages_per_sec": 0.0033,
                "network_address_changes": 0,
            },
            "0x000b57fffec6a5b4": {
                "leave_count": 0,
                "messages": 1,
                "messages_per_sec": 0.0017,
                "network_address_changes": 0,
            },
        })
    );
    assert_eq!(health.publish_options, vec![(true, 1)]);

    health.published.clear();
    health.publish_options.clear();
    health.advance_minutes(11);

    assert_eq!(health.published.len(), 1);
    let value = last(&health);
    assert_health_shape(value);
    assert_eq!(value["mqtt"]["received"], json!(0));
    assert_eq!(value["devices"], json!({}));
    assert_eq!(health.publish_options, vec![(true, 1)]);
}
