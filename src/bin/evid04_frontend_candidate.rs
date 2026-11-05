//! Frontend WebSocket differential driver (harness only).
//!
//! Starts the real frontend server on an ephemeral port with a fake broker and
//! records the same facts as the pinned upstream dump: the close code for a
//! missing/incorrect token and that an authorized upgrade succeeds.
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use zigbee2mqtt_rs::config::MqttConfig;
use zigbee2mqtt_rs::devices::DeviceRegistry;
use zigbee2mqtt_rs::frontend::{self, ws, ServeOptions};
use zigbee2mqtt_rs::mqtt::fake_broker::FakeBroker;
use zigbee2mqtt_rs::mqtt::MqttBridge;

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("evid04-frontend-candidate: {message}");
    std::process::exit(2);
}

async fn handshake(stream: &mut TcpStream, path: &str, token: Option<&str>) -> String {
    let target = match token {
        Some(token) => format!("{path}?token={token}"),
        None => path.to_string(),
    };
    let request = format!(
        "GET {target} HTTP/1.1\r\nHost: 127.0.0.1\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        stream.read_exact(&mut byte).await.unwrap();
        response.push(byte[0]);
        if response.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&response).into_owned()
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let broker = FakeBroker::start().await;
    let cfg = MqttConfig {
        server: "127.0.0.1".into(),
        port: broker.port,
        base_topic: "zigbee2mqtt".into(),
        client_id: format!("z2m-frontend-diff-{}", std::process::id()),
        ..MqttConfig::default()
    };
    let (events, receiver) = tokio::sync::broadcast::channel(64);
    let (mqtt, _commands) = MqttBridge::connect_with_events(&cfg, Some(events))
        .await
        .unwrap_or_else(|error| fail(format!("mqtt connect: {error}")));
    let static_dir = std::env::temp_dir().join(format!("z2m-frontend-diff-{}", std::process::id()));
    let options = ServeOptions {
        host: Some("127.0.0.1".into()),
        port: 0,
        base_url: String::new(),
        auth_token: Some("secret".into()),
        disable_ui_serving: true,
        static_dir,
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| fail(format!("bind: {error}")));
    let port = listener
        .local_addr()
        .unwrap_or_else(|error| fail(format!("local addr: {error}")))
        .port();
    let devices = Arc::new(DeviceRegistry::new());
    tokio::spawn(async move {
        let _ = frontend::serve_on(
            listener,
            options,
            "zigbee2mqtt".into(),
            devices,
            receiver,
            mqtt,
        )
        .await;
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut socket = TcpStream::connect(("127.0.0.1", port))
        .await
        .unwrap_or_else(|error| fail(format!("connect: {error}")));
    let response = handshake(&mut socket, "/api", None).await;
    if !response.starts_with("HTTP/1.1 101") {
        fail(format!("expected upgrade, got {response}"));
    }
    let close_code = match ws::read_frame(&mut socket).await {
        Ok(Some(ws::Frame::Close(Some(code), _))) => Some(code),
        _ => None,
    };

    let mut socket = TcpStream::connect(("127.0.0.1", port))
        .await
        .unwrap_or_else(|error| fail(format!("connect: {error}")));
    let response = handshake(&mut socket, "/api", Some("secret")).await;
    let authorized = response.starts_with("HTTP/1.1 101");

    println!(
        "{}",
        serde_json::to_string(&json!({
            "auth_close_code": close_code,
            "authorized_upgrade": authorized,
            "path": "/api",
        }))
        .unwrap_or_else(|_| "{}".into())
    );
}
