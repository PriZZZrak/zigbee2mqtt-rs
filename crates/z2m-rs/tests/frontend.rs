// Original: .upstream/zigbee2mqtt/test/extensions/frontend.test.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;

use z2m_rs::event_bus::{Event, EventBus};
use z2m_rs::extension::extension::{Extension, ExtensionContext};
use z2m_rs::extension::frontend::Frontend;
use z2m_rs::model::entity::PublishEntityState;
use z2m_rs::mqtt::{Mqtt, MqttPublishOptions, RetainedMessage};
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::zigbee::Zigbee;

struct Harness {
    context: ExtensionContext,
    mqtt: Arc<tokio::sync::Mutex<Mqtt>>,
    event_bus: Arc<Mutex<EventBus>>,
    state_path: std::path::PathBuf,
}

fn harness(auth_token: Option<&str>) -> Harness {
    harness_with_host(auth_token, "127.0.0.1")
}

fn harness_with_host(auth_token: Option<&str>, host: &str) -> Harness {
    let mut frontend = json!({"enabled": true, "port": 0, "base_url": "/", "host": host});
    if let Some(token) = auth_token {
        frontend["auth_token"] = json!(token);
    }
    let settings = Arc::new(Mutex::new(Settings::from_config(json!({
        "mqtt": {"server": "localhost", "base_topic": "zigbee2mqtt"},
        "frontend": frontend,
    }))));
    let event_bus = Arc::new(Mutex::new(EventBus::new()));
    let logger = Arc::new(Mutex::new(Logger::new()));
    let zigbee = Zigbee::new(event_bus.clone(), settings.clone(), logger.clone());
    let state_path = std::env::temp_dir().join(format!(
        "z2m-frontend-state-{}-{}",
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
        state_path.clone(),
    )));
    let mqtt = Arc::new(tokio::sync::Mutex::new(Mqtt::new(event_bus.clone())));
    let publish: PublishEntityState = Arc::new(|_e, _p, _r| Box::pin(async {}));

    Harness {
        context: ExtensionContext {
            zigbee,
            mqtt: mqtt.clone(),
            state,
            publish_entity_state: publish,
            event_bus: event_bus.clone(),
            logger,
            settings,
            enable_disable_extension: Arc::new(|_, _| Box::pin(async { Ok(()) })),
            restart: Arc::new(|| Box::pin(async {})),
            add_extension: Arc::new(|_| Box::pin(async {})),
            key: 20,
        },
        mqtt,
        event_bus,
        state_path,
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.state_path);
    }
}

async fn next_json(socket: &mut tokio_tungstenite::WebSocketStream<TcpStream>) -> Value {
    loop {
        let message = tokio::time::timeout(Duration::from_secs(3), socket.next())
            .await
            .expect("websocket frame timeout")
            .expect("socket closed")
            .expect("websocket error");
        match message {
            Message::Text(text) => return serde_json::from_str(&text).unwrap(),
            Message::Close(_) => panic!("closed"),
            _ => continue,
        }
    }
}

#[tokio::test]
async fn auth_token_is_enforced() {
    let harness = harness(Some("secret"));
    let frontend = Frontend::new(harness.context.clone());
    frontend.clone().start().await.unwrap();
    let addr = frontend.bound_addr().unwrap();

    let stream = TcpStream::connect(addr).await.unwrap();
    let result =
        tokio_tungstenite::client_async(format!("ws://{addr}/api?token=wrong"), stream).await;
    assert!(result.is_err());

    let stream = TcpStream::connect(addr).await.unwrap();
    let (socket, _) =
        tokio_tungstenite::client_async(format!("ws://{addr}/api?token=secret"), stream)
            .await
            .unwrap();
    drop(socket);
}

#[tokio::test]
async fn replay_retained_and_receive_messages() {
    let harness = harness(None);
    harness.mqtt.lock().await.retained_messages.insert(
        "zigbee2mqtt/bridge/info".to_string(),
        RetainedMessage {
            topic: "zigbee2mqtt/bridge/info".to_string(),
            payload: r#"{"version":"2.14.3"}"#.to_string(),
            options: MqttPublishOptions::default(),
        },
    );
    let received = Arc::new(Mutex::new(Vec::new()));
    let sink = received.clone();
    harness.event_bus.lock().unwrap().on(
        Event::MqttMessage,
        99,
        Arc::new(move |data| sink.lock().unwrap().push(data)),
    );

    let frontend = Frontend::new(harness.context.clone());
    frontend.clone().start().await.unwrap();
    let addr = frontend.bound_addr().unwrap();
    let stream = TcpStream::connect(addr).await.unwrap();
    let (mut socket, _) = tokio_tungstenite::client_async(format!("ws://{addr}/api"), stream)
        .await
        .unwrap();

    let replay = next_json(&mut socket).await;
    assert_eq!(replay["topic"], "bridge/info");
    assert_eq!(replay["payload"]["version"], "2.14.3");

    socket
        .send(Message::Text(
            json!({"topic": "lamp/set", "payload": {"state": "ON"}})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();

    for _ in 0..100 {
        if !received.lock().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let received = received.lock().unwrap();
    assert_eq!(received[0]["topic"], "zigbee2mqtt/lamp/set");
    assert_eq!(received[0]["message"], r#"{"state":"ON"}"#);
}

#[tokio::test]
async fn broadcasts_published_messages_and_skips_entity_states() {
    let harness = harness(None);
    let frontend = Frontend::new(harness.context.clone());
    frontend.clone().start().await.unwrap();
    let addr = frontend.bound_addr().unwrap();
    let stream = TcpStream::connect(addr).await.unwrap();
    let (mut socket, _) = tokio_tungstenite::client_async(format!("ws://{addr}/api"), stream)
        .await
        .unwrap();

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_mqtt_message_published(json!({
            "topic": "zigbee2mqtt/bridge/state",
            "payload": "{\"state\":\"online\"}",
            "options": {"meta": {"isEntityState": false}},
        }));
    let frame = next_json(&mut socket).await;
    assert_eq!(frame["topic"], "bridge/state");
    assert_eq!(frame["payload"]["state"], "online");

    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_mqtt_message_published(json!({
            "topic": "zigbee2mqtt/lamp",
            "payload": "{\"state\":\"ON\"}",
            "options": {"meta": {"isEntityState": true}},
        }));
    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_publish_entity_state(json!({
            "entity": {"name": "lamp"},
            "message": {"state": "OFF"},
        }));
    let frame = next_json(&mut socket).await;
    assert_eq!(frame["topic"], "lamp");
    assert_eq!(frame["payload"], json!({"state": "OFF"}));
}

#[tokio::test]
async fn stop_sends_offline_state() {
    let harness = harness(None);
    let frontend = Frontend::new(harness.context.clone());
    frontend.clone().start().await.unwrap();
    let addr = frontend.bound_addr().unwrap();
    let stream = TcpStream::connect(addr).await.unwrap();
    let (mut socket, _) = tokio_tungstenite::client_async(format!("ws://{addr}/api"), stream)
        .await
        .unwrap();

    frontend.clone().stop().await.unwrap();
    let frame = next_json(&mut socket).await;
    assert_eq!(frame["topic"], "bridge/state");
    assert_eq!(frame["payload"]["state"], "offline");
}

#[cfg(unix)]
#[tokio::test]
async fn unix_socket_frontend_serves_websocket() {
    let socket_path = std::env::temp_dir().join(format!(
        "z2m-frontend-{}-{}.sock",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path = socket_path.to_string_lossy().to_string();
    let harness = harness_with_host(None, &path);
    let frontend = Frontend::new(harness.context.clone());
    frontend.clone().start().await.unwrap();

    let stream = tokio::net::UnixStream::connect(&socket_path).await.unwrap();
    let (mut socket, _) = tokio_tungstenite::client_async("ws://localhost/api", stream)
        .await
        .unwrap();
    socket
        .send(Message::Text(
            json!({"topic": "lamp/set", "payload": {"state": "ON"}})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();

    frontend.stop().await.unwrap();
    let _ = std::fs::remove_file(socket_path);
}
