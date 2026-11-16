//! MQTT reconnect / retained-republish / delivery tests (audit CP-02).
//!
//! A minimal in-process MQTT 3.1.1 broker drives `z2m_rs::mqtt::Mqtt` without
//! external services: it accepts CONNECT/SUBSCRIBE/PUBLISH/PINGREQ, stores
//! retained messages, records every publish (topic, payload, retain, qos) and
//! can drop the client connections to force a reconnect.
//!
//! Upstream anchors: `lib/mqtt.ts` `onConnect` (state publish, subscribe, 2 s
//! retained republish and the `bridge/info` cancel) and `publish`
//! (`force_disable_retain`, retained tracking).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use z2m_rs::event_bus::EventBus;
use z2m_rs::mqtt::Mqtt;

#[derive(Debug, Clone)]
struct Published {
    topic: String,
    payload: Vec<u8>,
    retain: bool,
    qos: u8,
}

struct Broker {
    port: u16,
    retained: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    publishes: Arc<Mutex<Vec<Published>>>,
    subscriptions: Arc<Mutex<Vec<String>>>,
    connections: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<()>>>>,
}

impl Broker {
    async fn start() -> Arc<Broker> {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = listener.local_addr().unwrap().port();
        let retained: Arc<Mutex<HashMap<String, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
        let publishes: Arc<Mutex<Vec<Published>>> = Arc::new(Mutex::new(Vec::new()));
        let subscriptions: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let connections: Arc<Mutex<Vec<tokio::sync::oneshot::Sender<()>>>> =
            Arc::new(Mutex::new(Vec::new()));

        let broker = Arc::new(Broker {
            port,
            retained: retained.clone(),
            publishes: publishes.clone(),
            subscriptions: subscriptions.clone(),
            connections: connections.clone(),
        });

        let state = (retained, publishes, subscriptions, connections);
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let (retained, publishes, subscriptions, connections) = state.clone();
                let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
                connections.lock().unwrap().push(stop_tx);
                tokio::spawn(async move {
                    let mut buffer: Vec<u8> = Vec::new();
                    loop {
                        let mut chunk = [0u8; 4096];
                        let read = tokio::select! {
                            _ = &mut stop_rx => return,
                            read = socket.read(&mut chunk) => read,
                        };
                        let Ok(read) = read else { return };
                        if read == 0 {
                            return;
                        }
                        buffer.extend_from_slice(&chunk[..read]);
                        loop {
                            let Some((packet_type, flags, header_len, body_len)) =
                                frame(&buffer)
                            else {
                                break;
                            };
                            if buffer.len() < header_len + body_len {
                                break;
                            }
                            let body = buffer[header_len..header_len + body_len].to_vec();
                            buffer.drain(..header_len + body_len);
                            if !handle(
                                &mut socket,
                                packet_type,
                                flags,
                                &body,
                                &retained,
                                &publishes,
                                &subscriptions,
                            )
                            .await
                            {
                                return;
                            }
                        }
                    }
                });
            }
        });

        broker
    }

    fn server(&self) -> String {
        format!("mqtt://127.0.0.1:{}", self.port)
    }

    fn seen(&self, topic: &str) -> Vec<Published> {
        self.publishes
            .lock()
            .unwrap()
            .iter()
            .filter(|publish| publish.topic == topic)
            .cloned()
            .collect()
    }

    fn preload_retained(&self, topic: &str, payload: &str) {
        self.retained
            .lock()
            .unwrap()
            .insert(topic.to_string(), payload.as_bytes().to_vec());
    }

    fn subscriptions(&self) -> Vec<String> {
        self.subscriptions.lock().unwrap().clone()
    }

    fn drop_connections(&self) {
        let mut connections = self.connections.lock().unwrap();
        for sender in connections.drain(..) {
            let _ = sender.send(());
        }
    }
}

fn frame(buffer: &[u8]) -> Option<(u8, u8, usize, usize)> {
    if buffer.is_empty() {
        return None;
    }
    let packet_type = buffer[0] >> 4;
    let flags = buffer[0] & 0x0f;
    let mut index = 1;
    let mut length = 0usize;
    let mut multiplier = 1usize;
    loop {
        let byte = *buffer.get(index)?;
        index += 1;
        length += (byte & 0x7f) as usize * multiplier;
        if byte & 0x80 == 0 {
            break;
        }
        multiplier *= 128;
        if multiplier > 128 * 128 * 128 {
            return None;
        }
    }
    Some((packet_type, flags, index, length))
}

async fn handle(
    socket: &mut tokio::net::TcpStream,
    packet_type: u8,
    flags: u8,
    body: &[u8],
    retained: &Arc<Mutex<HashMap<String, Vec<u8>>>>,
    publishes: &Arc<Mutex<Vec<Published>>>,
    subscriptions: &Arc<Mutex<Vec<String>>>,
) -> bool {
    match packet_type {
        1 => {
            // CONNECT -> CONNACK
            let _ = socket.write_all(&[0x20, 0x02, 0x00, 0x00]).await;
        }
        8 => {
            // SUBSCRIBE -> SUBACK, then retained delivery for matching filters
            let qos = 0u8;
            let packet_id = u16::from_be_bytes([body[0], body[1]]);
            let mut index = 2;
            let mut filters = Vec::new();
            while index + 2 <= body.len() {
                let len = u16::from_be_bytes([body[index], body[index + 1]]) as usize;
                index += 2;
                if index + len > body.len() {
                    break;
                }
                filters.push(String::from_utf8_lossy(&body[index..index + len]).to_string());
                index += len + 1; // requested qos byte
            }
            subscriptions.lock().unwrap().extend(filters.clone());
            let mut suback = vec![0x90, 0x03, packet_id.to_be_bytes()[0], packet_id.to_be_bytes()[1], qos];
            let _ = socket.write_all(&suback).await;
            let retained = retained.lock().unwrap().clone();
            for (topic, payload) in retained {
                let matches = filters.iter().any(|filter| {
                    filter.ends_with("/#")
                        && topic.starts_with(filter.trim_end_matches('#'))
                        || filter == &topic
                });
                if !matches {
                    continue;
                }
                let mut packet = vec![0x31];
                let mut remaining = Vec::new();
                remaining.extend_from_slice(&(topic.len() as u16).to_be_bytes());
                remaining.extend_from_slice(topic.as_bytes());
                remaining.extend_from_slice(&payload);
                packet.push(remaining.len() as u8);
                packet.extend_from_slice(&remaining);
                let _ = socket.write_all(&packet).await;
            }
        }
        3 => {
            // PUBLISH
            let retain = flags & 0x01 != 0;
            let qos = (flags >> 1) & 0x03;
            let topic_len = u16::from_be_bytes([body[0], body[1]]) as usize;
            let topic = String::from_utf8_lossy(&body[2..2 + topic_len]).to_string();
            let mut index = 2 + topic_len;
            let packet_id = if qos > 0 {
                let id = u16::from_be_bytes([body[index], body[index + 1]]);
                index += 2;
                Some(id)
            } else {
                None
            };
            let payload = body[index..].to_vec();
            publishes.lock().unwrap().push(Published {
                topic: topic.clone(),
                payload: payload.clone(),
                retain,
                qos,
            });
            if retain {
                retained.lock().unwrap().insert(topic, payload);
            }
            if let Some(packet_id) = packet_id {
                let _ = socket
                    .write_all(&[0x40, 0x02, packet_id.to_be_bytes()[0], packet_id.to_be_bytes()[1]])
                    .await;
            }
        }
        12 => {
            let _ = socket.write_all(&[0xd0, 0x00]).await;
        }
        14 => return false,
        _ => {}
    }
    true
}

fn connected_now(mqtt: &Arc<tokio::sync::Mutex<Mqtt>>) -> bool {
    mqtt.try_lock().map(|guard| guard.is_connected()).unwrap_or(false)
}

fn queued_now(mqtt: &Arc<tokio::sync::Mutex<Mqtt>>) -> u64 {
    mqtt.try_lock()
        .map(|guard| guard.stats()["queued"].as_u64().unwrap_or(1))
        .unwrap_or(1)
}

async fn wait_for<F: Fn() -> bool>(what: &str, timeout: Duration, predicate: F) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if predicate() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("timeout waiting for {what}");
}

fn settings(server: &str, force_disable_retain: bool) -> serde_json::Value {
    json!({
        "mqtt": {
            "base_topic": "z2m",
            "server": server,
            "force_disable_retain": force_disable_retain,
        }
    })
}

async fn connect(server: &str, force_disable_retain: bool) -> Arc<tokio::sync::Mutex<Mqtt>> {
    let event_bus = Arc::new(Mutex::new(EventBus::new()));
    let mut mqtt = Mqtt::new(event_bus);
    mqtt.connect(&settings(server, force_disable_retain))
        .await
        .expect("connect");
    Arc::new(tokio::sync::Mutex::new(mqtt))
}

#[tokio::test]
async fn publishes_bridge_state_and_subscribes_on_connect() {
    let broker = Broker::start().await;
    let mqtt = connect(&broker.server(), false).await;

    wait_for("bridge/state online", Duration::from_secs(5), || {
        broker
            .seen("z2m/bridge/state")
            .iter()
            .any(|publish| publish.retain && publish.payload == b"{\"state\":\"online\"}".to_vec())
    })
    .await;
    wait_for("subscription", Duration::from_secs(5), || {
        broker.subscriptions().iter().any(|filter| filter == "z2m/#")
    })
    .await;
    assert!(mqtt.lock().await.is_connected());
}

#[tokio::test]
async fn reconnects_and_republishes_state_after_the_broker_drops_the_connection() {
    let broker = Broker::start().await;
    let mqtt = connect(&broker.server(), false).await;
    wait_for("first connect", Duration::from_secs(5), || {
        connected_now(&mqtt)
    })
    .await;

    broker.drop_connections();
    wait_for("disconnect", Duration::from_secs(5), || {
        !connected_now(&mqtt)
    })
    .await;
    wait_for("second connect", Duration::from_secs(8), || {
        connected_now(&mqtt)
    })
    .await;
    wait_for("state published twice", Duration::from_secs(8), || {
        broker.seen("z2m/bridge/state").len() >= 2
    })
    .await;
    wait_for("resubscribed", Duration::from_secs(5), || {
        broker
            .subscriptions()
            .iter()
            .filter(|filter| *filter == "z2m/#")
            .count()
            >= 2
    })
    .await;
}

#[tokio::test]
async fn republishes_tracked_retained_messages_two_seconds_after_connect() {
    let broker = Broker::start().await;
    let mqtt = connect(&broker.server(), false).await;
    wait_for("connected", Duration::from_secs(5), || {
        connected_now(&mqtt)
    })
    .await;

    mqtt.lock()
        .await
        .publish(
            "device/state",
            b"{\"state\":\"ON\"}".to_vec(),
            z2m_rs::mqtt::MqttPublishOptions {
                retain: true,
                qos: 1,
                ..Default::default()
            },
        )
        .await;

    // The 2 s timer republishes everything tracked at fire time.
    wait_for("retained republish", Duration::from_secs(6), || {
        broker.seen("z2m/device/state").len() >= 2
    })
    .await;
    assert!(broker.seen("z2m/device/state").iter().all(|publish| publish.retain));
}

#[tokio::test]
async fn cancels_the_republish_when_a_retained_bridge_info_arrives() {
    let broker = Broker::start().await;
    broker.preload_retained("z2m/bridge/info", "{\"version\":1}");
    let mqtt = connect(&broker.server(), false).await;
    wait_for("connected", Duration::from_secs(5), || {
        connected_now(&mqtt)
    })
    .await;
    // Give the retained `bridge/info` time to arrive and cancel the timer.
    tokio::time::sleep(Duration::from_millis(300)).await;

    mqtt.lock()
        .await
        .publish(
            "device/state",
            b"{\"state\":\"ON\"}".to_vec(),
            z2m_rs::mqtt::MqttPublishOptions {
                retain: true,
                qos: 1,
                ..Default::default()
            },
        )
        .await;
    tokio::time::sleep(Duration::from_millis(2500)).await;
    assert_eq!(
        broker.seen("z2m/device/state").len(),
        1,
        "republish must be cancelled by the retained bridge/info"
    );
}

#[tokio::test]
async fn force_disable_retain_publishes_without_the_retain_flag() {
    let broker = Broker::start().await;
    let mqtt = connect(&broker.server(), true).await;
    wait_for("connected", Duration::from_secs(5), || {
        connected_now(&mqtt)
    })
    .await;

    let mut guard = mqtt.lock().await;
    guard
        .publish(
            "device/state",
            b"{\"state\":\"ON\"}".to_vec(),
            z2m_rs::mqtt::MqttPublishOptions {
                retain: true,
                qos: 1,
                ..Default::default()
            },
        )
        .await;
    assert!(!guard.retained_messages.lock().unwrap().contains_key("z2m/device/state"));
    drop(guard);

    wait_for("publish without retain", Duration::from_secs(5), || {
        broker
            .seen("z2m/device/state")
            .iter()
            .any(|publish| !publish.retain)
    })
    .await;
}

#[tokio::test]
async fn qos1_publish_is_acknowledged_and_leaves_no_inflight_work() {
    let broker = Broker::start().await;
    let mqtt = connect(&broker.server(), false).await;
    wait_for("connected", Duration::from_secs(5), || {
        connected_now(&mqtt)
    })
    .await;

    let mut guard = mqtt.lock().await;
    guard
        .publish(
            "device/state",
            b"{}".to_vec(),
            z2m_rs::mqtt::MqttPublishOptions {
                retain: false,
                qos: 1,
                ..Default::default()
            },
        )
        .await;
    drop(guard);

    wait_for("publish delivered", Duration::from_secs(5), || {
        !broker.seen("z2m/device/state").is_empty()
    })
    .await;
    // The PUBACK frees the in-flight counter.
    wait_for("queue drained", Duration::from_secs(5), || {
        queued_now(&mqtt) == 0
    })
    .await;
}

#[tokio::test]
async fn buffers_publish_while_the_broker_is_down_and_delivers_after_reconnect() {
    let broker = Broker::start().await;
    let mqtt = connect(&broker.server(), false).await;
    wait_for("connected", Duration::from_secs(5), || connected_now(&mqtt)).await;

    // Stop the broker's accepted connections; the client must report the loss.
    broker.drop_connections();
    wait_for("disconnected", Duration::from_secs(5), || !connected_now(&mqtt)).await;

    // While disconnected the publish is buffered by the client (mqtt.js does the
    // same) and must be delivered once the event loop reconnects.
    let queued = mqtt
        .lock()
        .await
        .publish("device/queued", b"{\"buffered\":true}".to_vec(), Default::default())
        .await;
    assert!(queued, "the client buffers publishes while reconnecting");

    wait_for("reconnected and flushed", Duration::from_secs(10), || {
        broker
            .seen("z2m/device/queued")
            .iter()
            .any(|publish| publish.payload == b"{\"buffered\":true}".to_vec())
    })
    .await;
}
