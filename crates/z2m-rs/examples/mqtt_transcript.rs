//! Broker transcript helper (audit CP-02).
//!
//! Connects the Rust `Mqtt` client to the broker started by
//! `tools/oracle/mqtt-transcript.mjs`, publishes one retained and one plain
//! message, waits for the 2 s retained-republish timer and disconnects. The Node
//! oracle runs the same scenario with the upstream `dist/mqtt.js` client and the
//! two broker-side transcripts are compared.
//!
//! Usage: cargo run -p z2m-rs --example mqtt_transcript -- mqtt://127.0.0.1:PORT

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;

use z2m_rs::event_bus::EventBus;
use z2m_rs::mqtt::{Mqtt, MqttPublishOptions};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let server = std::env::args().nth(1).expect("server url");
    let settings = json!({
        "mqtt": {
            "base_topic": "z2m",
            "server": server,
            "force_disable_retain": false,
        }
    });

    let event_bus = Arc::new(Mutex::new(EventBus::new()));
    let mut mqtt = Mqtt::new(event_bus);
    mqtt.connect(&settings).await.expect("connect");

    // Wait for CONNACK + bridge/state + subscribe.
    tokio::time::sleep(Duration::from_millis(500)).await;

    mqtt.publish(
        "device/state",
        b"{\"state\":\"ON\"}".to_vec(),
        MqttPublishOptions {
            retain: true,
            qos: 1,
            ..Default::default()
        },
    )
    .await;
    mqtt.publish(
        "device/plain",
        b"{\"plain\":true}".to_vec(),
        MqttPublishOptions {
            retain: false,
            qos: 0,
            ..Default::default()
        },
    )
    .await;

    // Let the 2 s retained-republish timer fire.
    tokio::time::sleep(Duration::from_millis(2600)).await;
    mqtt.disconnect().await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    println!("ok");
}
