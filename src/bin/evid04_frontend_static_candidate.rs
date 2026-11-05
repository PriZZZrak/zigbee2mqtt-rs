//! Static frontend differential driver (harness only).
//!
//! Serves the same fixture tree as the upstream `srvx` reference dump and
//! records status/headers/body for index, asset, precompressed asset and 404.
use std::time::Duration;

use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use zigbee2mqtt_rs::config::MqttConfig;
use zigbee2mqtt_rs::devices::DeviceRegistry;
use zigbee2mqtt_rs::frontend::{self, ServeOptions};
use zigbee2mqtt_rs::mqtt::fake_broker::FakeBroker;
use zigbee2mqtt_rs::mqtt::MqttBridge;

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("evid04-frontend-static-candidate: {message}");
    std::process::exit(2);
}

async fn request(port: u16, path: &str, headers: &[(&str, &str)]) -> serde_json::Value {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .unwrap_or_else(|error| fail(format!("connect: {error}")));
    let mut request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n");
    for (key, value) in headers {
        request.push_str(&format!("{key}: {value}\r\n"));
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.unwrap();
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .unwrap_or_else(|| fail("malformed HTTP response"));
    let head = String::from_utf8_lossy(&response[..split]).into_owned();
    let body = &response[split + 4..];
    let mut lines = head.split("\r\n");
    let status: u16 = lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    let mut response_headers = serde_json::Map::new();
    for line in lines {
        if let Some((key, value)) = line.split_once(':') {
            response_headers.insert(key.trim().to_ascii_lowercase(), json!(value.trim()));
        }
    }
    json!({
        "status": status,
        "headers": response_headers,
        "body": zigbee2mqtt_rs::frontend::ws::base64_encode(body),
    })
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let broker = FakeBroker::start().await;
    let cfg = MqttConfig {
        server: "127.0.0.1".into(),
        port: broker.port,
        base_topic: "zigbee2mqtt".into(),
        client_id: format!("z2m-frontend-static-{}", std::process::id()),
        ..MqttConfig::default()
    };
    let (events, receiver) = tokio::sync::broadcast::channel(16);
    let (mqtt, _commands) = MqttBridge::connect_with_events(&cfg, Some(events))
        .await
        .unwrap_or_else(|error| fail(format!("mqtt connect: {error}")));

    let dir = std::env::temp_dir().join(format!("z2m-frontend-static-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    std::fs::write(dir.join("index.html"), "<html>index</html>").unwrap();
    std::fs::write(dir.join("assets").join("app.js"), "console.log('app');").unwrap();
    std::fs::write(dir.join("assets").join("app.js.br"), "compressed-js").unwrap();

    let options = ServeOptions {
        host: Some("127.0.0.1".into()),
        port: 0,
        base_url: String::new(),
        auth_token: None,
        disable_ui_serving: false,
        static_dir: dir.clone(),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap_or_else(|error| fail(format!("bind: {error}")));
    let port = listener
        .local_addr()
        .unwrap_or_else(|error| fail(format!("local addr: {error}")))
        .port();
    let devices = std::sync::Arc::new(DeviceRegistry::new());
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

    let responses = json!({
        "index": request(port, "/", &[]).await,
        "asset": request(port, "/assets/app.js", &[]).await,
        "asset-br": request(port, "/assets/app.js", &[("Accept-Encoding", "br")]).await,
        "missing": request(port, "/nope.txt", &[]).await,
    });
    println!(
        "{}",
        serde_json::to_string(&json!({ "responses": responses })).unwrap()
    );
    std::fs::remove_dir_all(&dir).ok();
}
