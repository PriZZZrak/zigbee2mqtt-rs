// Original: .upstream/zigbee2mqtt/lib/mqtt.ts
//
// Thin async wrapper around `rumqttc` (pure Rust, no default features) that
// keeps the upstream publish/retain/subscribe contract used by the bridge.

use std::collections::BTreeMap;
use std::sync::Arc;

use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::event_bus::EventBus;

/// Upstream `MqttPublishOptions`.
#[derive(Debug, Clone, Default)]
pub struct MqttPublishOptions {
    pub retain: bool,
    pub qos: u8,
    pub skip_log: bool,
    pub skip_receive: bool,
    pub is_entity_state: bool,
}

#[derive(Debug, Clone)]
pub struct RetainedMessage {
    pub topic: String,
    pub payload: String,
    pub options: MqttPublishOptions,
}

pub struct Mqtt {
    pub event_bus: Arc<Mutex<EventBus>>,
    pub base_topic: String,
    pub server: String,
    pub connected: bool,
    pub published_topics: std::collections::HashSet<String>,
    pub retained_messages: BTreeMap<String, RetainedMessage>,
    client: Option<AsyncClient>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl Mqtt {
    pub fn new(event_bus: Arc<Mutex<EventBus>>) -> Self {
        Self {
            event_bus,
            base_topic: "zigbee2mqtt".to_string(),
            server: "mqtt://localhost:1883".to_string(),
            connected: false,
            published_topics: Default::default(),
            retained_messages: BTreeMap::new(),
            client: None,
            task: None,
        }
    }

    pub fn with_settings(event_bus: Arc<Mutex<EventBus>>, settings: &Value) -> Self {
        let mut mqtt = Self::new(event_bus);
        mqtt.base_topic = settings["mqtt"]["base_topic"]
            .as_str()
            .unwrap_or("zigbee2mqtt")
            .to_string();
        mqtt.server = settings["mqtt"]["server"]
            .as_str()
            .unwrap_or("mqtt://localhost:1883")
            .to_string();
        mqtt
    }

    /// Upstream `Mqtt.connect()`.
    pub async fn connect(&mut self, settings: &Value) -> Result<(), String> {
        let mqtt_settings = &settings["mqtt"];
        let server = mqtt_settings["server"]
            .as_str()
            .unwrap_or("mqtt://localhost:1883");
        let (host, port) = parse_server(server)?;
        self.base_topic = mqtt_settings["base_topic"]
            .as_str()
            .unwrap_or("zigbee2mqtt")
            .to_string();
        self.server = server.to_string();

        let client_id = mqtt_settings["client_id"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| format!("zigbee2mqtt-{}", std::process::id()));
        let mut options = MqttOptions::new(client_id, host, port);
        if let Some(keepalive) = mqtt_settings["keepalive"].as_u64() {
            options.set_keep_alive(std::time::Duration::from_secs(keepalive));
        }
        if let (Some(user), Some(password)) = (
            mqtt_settings["user"].as_str(),
            mqtt_settings["password"].as_str(),
        ) {
            options.set_credentials(user, password);
        } else if let Some(user) = mqtt_settings["user"].as_str() {
            options.set_credentials(user, "");
        }
        if let Some(id) = mqtt_settings["client_id"].as_str() {
            options.set_clean_session(false);
            let _ = id;
        }

        let (client, mut eventloop) = AsyncClient::new(options, 64);
        let base_topic = self.base_topic.clone();
        let event_bus = self.event_bus.clone();
        let task = tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::Publish(publish))) => {
                        let payload = match std::str::from_utf8(&publish.payload) {
                            Ok(payload) => payload.to_string(),
                            Err(_) => continue,
                        };
                        event_bus.lock().await.emit_mqtt_message(json!({
                            "topic": publish.topic,
                            "message": payload,
                        }));
                    }
                    Ok(Event::Incoming(Packet::ConnAck(_))) => {}
                    Ok(_) => {}
                    Err(_) => {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
                let _ = &base_topic;
            }
        });

        self.client = Some(client);
        self.task = Some(task);
        self.connected = true;
        Ok(())
    }

    /// Upstream `Mqtt.disconnect()`.
    pub async fn disconnect(&mut self) {
        if let Some(client) = self.client.as_mut() {
            let _ = client
                .publish(
                    format!("{}/bridge/state", self.base_topic),
                    QoS::AtLeastOnce,
                    true,
                    serde_json::to_vec(&json!({"state": "offline"})).unwrap_or_default(),
                )
                .await;
            let _ = client.disconnect().await;
        }
        self.connected = false;
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }

    /// Upstream `Mqtt.publish(topic, payload, options)`.
    pub async fn publish(
        &mut self,
        topic: &str,
        payload: impl Into<Vec<u8>>,
        options: MqttPublishOptions,
    ) -> bool {
        let payload = payload.into();
        let qos = match options.qos {
            1 => QoS::AtLeastOnce,
            2 => QoS::ExactlyOnce,
            _ => QoS::AtMostOnce,
        };
        let Some(client) = self.client.as_mut() else {
            return false;
        };
        let topic = if topic.contains('/') {
            topic.to_string()
        } else {
            format!("{}/{}", self.base_topic, topic)
        };
        if client
            .publish(&topic, qos, options.retain, payload.clone())
            .await
            .is_err()
        {
            return false;
        }
        self.published_topics.insert(topic.clone());
        if options.retain {
            self.retained_messages.insert(
                topic.clone(),
                RetainedMessage {
                    topic: topic.clone(),
                    payload: String::from_utf8_lossy(&payload).to_string(),
                    options: options.clone(),
                },
            );
        }
        self.event_bus
            .lock()
            .await
            .emit_mqtt_message_published(json!({"topic": topic}));
        true
    }

    /// Upstream `Mqtt.subscribe(topic, callback)`.
    pub async fn subscribe(&mut self, topic: &str, qos: u8) -> bool {
        let Some(client) = self.client.as_mut() else {
            return false;
        };
        let qos = match qos {
            1 => QoS::AtLeastOnce,
            2 => QoS::ExactlyOnce,
            _ => QoS::AtMostOnce,
        };
        client.subscribe(topic, qos).await.is_ok()
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn info(&self) -> Value {
        json!({
            "version": 4,
            "server": self.server,
        })
    }

    pub fn stats(&self) -> Value {
        json!({
            "connected": self.is_connected(),
        })
    }

    /// Upstream `Mqtt.onMessage`.
    pub async fn on_message(&mut self, topic: &str, payload: Value) {
        self.event_bus.lock().await.emit_mqtt_message(json!({
            "topic": topic,
            "message": payload,
        }));
    }
}

fn parse_server(server: &str) -> Result<(String, u16), String> {
    let rest = server
        .strip_prefix("mqtt://")
        .or_else(|| server.strip_prefix("tcp://"))
        .ok_or_else(|| format!("Unsupported MQTT server '{server}'"))?;
    let (host, port) = rest
        .rsplit_once(':')
        .ok_or_else(|| format!("Invalid MQTT server '{server}', expected mqtt://host:port"))?;
    let port = port
        .parse::<u16>()
        .map_err(|_| format!("Invalid MQTT port in '{server}'"))?;
    Ok((host.to_string(), port))
}
