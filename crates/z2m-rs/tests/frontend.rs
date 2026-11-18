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
    harness_with_frontend(
        json!({"enabled": true, "port": 0, "base_url": "/", "host": host}),
        auth_token,
    )
}

fn harness_with_frontend(mut frontend: Value, auth_token: Option<&str>) -> Harness {
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
    let _guard = zigbee_apiarist::controller::model::test_support::serial();
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
    let _guard = zigbee_apiarist::controller::model::test_support::serial();
    let harness = harness(None);
    harness
        .mqtt
        .lock()
        .await
        .retained_messages
        .lock()
        .unwrap()
        .insert(
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
    let _guard = zigbee_apiarist::controller::model::test_support::serial();
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
    let _guard = zigbee_apiarist::controller::model::test_support::serial();
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
    let _guard = zigbee_apiarist::controller::model::test_support::serial();
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

#[derive(Debug)]
struct NoVerify;

impl rustls::client::danger::ServerCertVerifier for NoVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls_rustcrypto::provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[tokio::test]
async fn tls_frontend_serves_websocket() {
    let _guard = zigbee_apiarist::controller::model::test_support::serial();
    let harness = harness_with_host(None, "127.0.0.1");
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    {
        let mut settings = harness.context.settings.lock().unwrap();
        settings.set(
            &["frontend", "ssl_key"],
            json!(fixtures.join("tls.key").display().to_string()),
        );
        settings.set(
            &["frontend", "ssl_cert"],
            json!(fixtures.join("tls.crt").display().to_string()),
        );
    }
    let frontend = Frontend::new(harness.context.clone());
    frontend.clone().start().await.unwrap();
    let addr = frontend.bound_addr().unwrap();

    let provider = Arc::new(rustls_rustcrypto::provider());
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoVerify))
        .with_no_client_auth();
    let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
    let stream = TcpStream::connect(addr).await.unwrap();
    let server_name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
    let tls = connector.connect(server_name, stream).await.unwrap();
    let (mut socket, _) = tokio_tungstenite::client_async("wss://localhost/api", tls)
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
}

async fn raw_request(addr: std::net::SocketAddr, request: &str) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response).await;
    String::from_utf8_lossy(&response).to_string()
}

async fn http_get(addr: std::net::SocketAddr, path: &str, accept: &str) -> (u16, String) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream
        .write_all(
            format!(
                "GET {path} HTTP/1.1\r\nHost: localhost\r\nAccept: {accept}\r\nConnection: close\r\n\r\n"
            )
            .as_bytes(),
        )
        .await
        .unwrap();
    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response).await;
    let response = String::from_utf8_lossy(&response).to_string();
    let status = response
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or(0);
    (status, response)
}

fn assets_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "z2m-windfront-assets-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    std::fs::write(dir.join("index.html"), "<html>windfront</html>").unwrap();
    std::fs::write(dir.join("assets/app.js"), "console.log(1)").unwrap();
    std::fs::write(dir.join("package.json"), r#"{"version":"2.15.1"}"#).unwrap();
    dir
}

#[tokio::test]
async fn serves_windfront_assets_with_cache_and_spa_fallback() {
    let _guard = zigbee_apiarist::controller::model::test_support::serial();
    let dir = assets_dir();
    std::env::set_var("ZIGBEE2MQTT_FRONTEND_ASSETS", &dir);
    let harness = harness(None);
    let frontend = Frontend::new(harness.context.clone());
    frontend.clone().start().await.unwrap();
    let addr = frontend.bound_addr().unwrap();

    let (status, index) = http_get(addr, "/", "text/html").await;
    assert_eq!(status, 200, "{index}");
    assert!(index.contains("<html>windfront</html>"));
    assert!(index.contains("no-store"));

    let (status, asset) = http_get(addr, "/assets/app.js", "*/*").await;
    assert_eq!(status, 200, "{asset}");
    assert!(
        asset.contains("public, max-age=31536000, immutable"),
        "{asset}"
    );

    // SPA fallback for client-side routes.
    let (status, fallback) = http_get(addr, "/devices/0x0017880104e45519", "text/html").await;
    assert_eq!(status, 200, "{fallback}");
    assert!(fallback.contains("<html>windfront</html>"));

    // The WebSocket endpoint is not served as a static file.
    let (status, _) = http_get(addr, "/api", "*/*").await;
    assert_eq!(status, 404);

    frontend.stop().await.unwrap();
    std::env::remove_var("ZIGBEE2MQTT_FRONTEND_ASSETS");
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn serves_assets_under_a_custom_base_url() {
    let _guard = zigbee_apiarist::controller::model::test_support::serial();
    let dir = assets_dir();
    std::env::set_var("ZIGBEE2MQTT_FRONTEND_ASSETS", &dir);
    let harness = harness(None);
    harness
        .context
        .settings
        .lock()
        .unwrap()
        .set(&["frontend", "base_url"], json!("/z2m"));
    let frontend = Frontend::new(harness.context.clone());
    frontend.clone().start().await.unwrap();
    let addr = frontend.bound_addr().unwrap();

    let (status, index) = http_get(addr, "/z2m/", "text/html").await;
    assert_eq!(status, 200, "{index}");
    assert!(index.contains("<html>windfront</html>"));

    let (status, _) = http_get(addr, "/", "text/html").await;
    assert_eq!(status, 404);

    frontend.stop().await.unwrap();
    std::env::remove_var("ZIGBEE2MQTT_FRONTEND_ASSETS");
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn accepts_windfront_package_json_from_the_package_root() {
    let _guard = zigbee_apiarist::controller::model::test_support::serial();
    let root = std::env::temp_dir().join(format!(
        "z2m-windfront-pkg-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let dist = root.join("dist");
    std::fs::create_dir_all(&dist).unwrap();
    std::fs::write(dist.join("index.html"), "<html>pkg</html>").unwrap();
    // Development layout: package.json lives in the package root, assets in dist/.
    std::fs::write(root.join("package.json"), r#"{"version":"2.15.1"}"#).unwrap();

    std::env::set_var("ZIGBEE2MQTT_FRONTEND_ASSETS", &dist);
    let harness = harness(None);
    let frontend = Frontend::new(harness.context.clone());
    frontend.clone().start().await.unwrap();
    let addr = frontend.bound_addr().unwrap();

    let (status, body) = http_get(addr, "/", "text/html").await;
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("<html>pkg</html>"));

    frontend.stop().await.unwrap();
    std::env::remove_var("ZIGBEE2MQTT_FRONTEND_ASSETS");
    let _ = std::fs::remove_dir_all(root);
}

/// The frontend must emit a single `Content-Length` (the static server already
/// sets one) and no body/`Content-Length` for 304 — Node's llhttp rejects
/// duplicate framing headers, which made a Node HTTP client see a socket error
/// while curl tolerated the response.
#[tokio::test]
async fn serves_single_content_length_and_empty_304() {
    let _guard = zigbee_apiarist::controller::model::test_support::serial();
    let dir = assets_dir();
    std::env::set_var("ZIGBEE2MQTT_FRONTEND_ASSETS", &dir);
    let harness = harness(None);
    let frontend = Frontend::new(harness.context.clone());
    frontend.clone().start().await.unwrap();
    let addr = frontend.bound_addr().unwrap();

    let (status, response) = http_get(addr, "/assets/app.js", "*/*").await;
    assert_eq!(status, 200, "{response}");
    let content_lengths = response
        .lines()
        .filter(|line| line.to_ascii_lowercase().starts_with("content-length:"))
        .count();
    assert_eq!(content_lengths, 1, "{response}");

    // Conditional request: ETag matches -> 304 without content-length/body.
    let etag = response
        .lines()
        .find_map(|line| line.strip_prefix("etag: ").map(str::to_string))
        .expect("etag header");
    let conditional = raw_request(addr, &format!(
        "GET /assets/app.js HTTP/1.1\r\nHost: localhost\r\nIf-None-Match: {etag}\r\nConnection: close\r\n\r\n"
    ))
    .await;
    assert!(conditional.starts_with("HTTP/1.1 304"), "{conditional}");
    assert!(
        !conditional.to_ascii_lowercase().contains("content-length"),
        "{conditional}"
    );

    frontend.stop().await.unwrap();
    std::env::remove_var("ZIGBEE2MQTT_FRONTEND_ASSETS");
    let _ = std::fs::remove_dir_all(dir);
}

/// Cases 5-6: an invalid/missing TLS configuration falls back to plain HTTP.
#[tokio::test]
async fn invalid_tls_configuration_falls_back_to_http() {
    let _guard = zigbee_apiarist::controller::model::test_support::serial();
    for frontend_settings in [
        // Missing key configuration.
        json!({"enabled": true, "port": 0, "base_url": "/", "host": "127.0.0.1", "ssl_cert": "/nonexistent/cert.pem"}),
        // Missing files.
        json!({"enabled": true, "port": 0, "base_url": "/", "host": "127.0.0.1", "ssl_cert": "/nonexistent/cert.pem", "ssl_key": "/nonexistent/key.pem"}),
    ] {
        let harness = harness_with_frontend(frontend_settings, None);
        let frontend = Frontend::new(harness.context.clone());
        frontend.clone().start().await.expect("frontend starts");
        let addr = frontend.bound_addr().expect("bound address");
        let stream = TcpStream::connect(addr).await.unwrap();
        let (socket, _) = tokio_tungstenite::client_async(format!("ws://{addr}/api"), stream)
            .await
            .expect("plain websocket fallback");
        drop(socket);
        frontend.clone().stop().await.unwrap();
    }
}

/// A Windfront package with a different version is rejected at startup.
#[tokio::test]
async fn rejects_a_mismatched_windfront_version() {
    let _guard = zigbee_apiarist::controller::model::test_support::serial();
    let dir = assets_dir();
    std::fs::write(dir.join("package.json"), r#"{"version":"9.9.9"}"#).unwrap();
    std::env::set_var("ZIGBEE2MQTT_FRONTEND_ASSETS", &dir);
    let harness = harness(None);
    let frontend = Frontend::new(harness.context.clone());
    let error = frontend.start().await.expect_err("version mismatch");
    assert!(
        error.contains("does not match the tested version") && error.contains("9.9.9"),
        "{error}"
    );
    std::env::remove_var("ZIGBEE2MQTT_FRONTEND_ASSETS");
    let _ = std::fs::remove_dir_all(dir);
}
