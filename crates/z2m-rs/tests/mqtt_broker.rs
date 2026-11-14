// Original: .upstream/zigbee2mqtt/lib/mqtt.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// Real-broker differential test. Ignored by default; run with:
//   MQTT_TEST_SERVER=mqtt://127.0.0.1:18830 cargo test -p z2m-rs \
//     --test mqtt_broker -- --ignored --nocapture

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;

use z2m_rs::event_bus::EventBus;
use z2m_rs::mqtt::{Mqtt, MqttPublishOptions};
use z2m_rs::util::settings::Settings;

#[tokio::test]
#[ignore = "requires a real MQTT broker (MQTT_TEST_SERVER)"]
async fn real_broker_connect_publish_and_queue_stats() {
    let server = std::env::var("MQTT_TEST_SERVER").expect("MQTT_TEST_SERVER");
    let settings = json!({
        "mqtt": {"server": server, "base_topic": "z2m-rs-broker-test"},
    });
    let event_bus = Arc::new(Mutex::new(EventBus::new()));
    let mut mqtt = Mqtt::new(event_bus.clone());
    mqtt.connect(&settings).await.expect("connect");

    // Subscribe with a second raw client to prove broker delivery.
    let (host, port) = {
        let rest = settings["mqtt"]["server"].as_str().unwrap().strip_prefix("mqtt://").unwrap();
        let (host, port) = rest.split_once(':').unwrap();
        (host.to_string(), port.parse::<u16>().unwrap())
    };
    let mut options = rumqttc::MqttOptions::new("z2m-rs-broker-observer", host, port);
    options.set_keep_alive(Duration::from_secs(5));
    let (observer, mut eventloop) = rumqttc::AsyncClient::new(options, 10);
    observer
        .subscribe("z2m-rs-broker-test/state", rumqttc::QoS::AtLeastOnce)
        .await
        .unwrap();

    let mut delivered = None;
    for _ in 0..50 {
        let publish = mqtt
            .publish(
                "state",
                r#"{"state":"ON"}"#,
                MqttPublishOptions {
                    qos: 1,
                    ..Default::default()
                },
            )
            .await;
        assert!(publish);
        tokio::time::sleep(Duration::from_millis(100)).await;
        match tokio::time::timeout(Duration::from_millis(100), eventloop.poll()).await {
            Ok(Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(publish)))) => {
                delivered = Some(String::from_utf8_lossy(&publish.payload).to_string());
                break;
            }
            _ => {}
        }
    }
    assert_eq!(delivered.as_deref(), Some(r#"{"state":"ON"}"#));

    let stats = mqtt.stats();
    assert_eq!(stats["connected"], true);
    assert!(stats["queued"].is_number());
    println!("mqtt.stats = {stats}");

    mqtt.disconnect().await;
    std::fs::write(
        "/tmp/opencode/mqtt_broker_transcript.txt",
        format!("server={server}\nstats={stats}\ndelivered={delivered:?}\n"),
    )
    .unwrap();
    let _ = &event_bus;
}
