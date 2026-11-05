//! Frontend HTTP/WebSocket server (`lib/extension/frontend.ts`).
//!
//! Upstream serves the Windfront assets over HTTP and exposes the bridge API on
//! a WebSocket at `<base_url>/api`:
//!
//! - with an `auth_token`, the query `?token=` must match or the socket is
//!   closed with code 4401;
//! - on connect the server replays retained MQTT messages (topic without the
//!   configured base topic) and the current state of every non-coordinator
//!   device;
//! - every publish under the base topic is forwarded as `{topic, payload}`;
//! - a client text frame `{topic, payload}` is published to
//!   `<base_topic>/<topic>`.
//!
//! The implementation is dependency-free: a small HTTP/1.1 server plus the
//! RFC 6455 subset in [`ws`].

pub mod ws;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex};
use tracing::{debug, info};

use crate::config::FrontendConfig;
use crate::devices::DeviceRegistry;
use crate::mqtt::{MqttBridge, PublishedEvent};

const MAX_HEADER_BYTES: usize = 16 * 1024;

/// Frontend server options resolved from config/environment.
#[derive(Debug, Clone)]
pub struct ServeOptions {
    pub host: Option<String>,
    pub port: u16,
    pub base_url: String,
    pub auth_token: Option<String>,
    pub disable_ui_serving: bool,
    pub static_dir: PathBuf,
}

impl ServeOptions {
    #[must_use]
    pub fn from_config(cfg: &FrontendConfig) -> Self {
        let static_dir = std::env::var("ZIGBEE2MQTT_FRONTEND_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/www/zigbee2mqtt"));
        Self {
            host: cfg.host.clone(),
            port: cfg.port,
            base_url: normalize_base_url(&cfg.base_url),
            auth_token: cfg.auth_token.clone(),
            disable_ui_serving: cfg.disable_ui_serving,
            static_dir,
        }
    }

    fn ws_path(&self) -> String {
        join_url(&self.base_url, "api")
    }
}

/// Normalize `base_url`: leading slash, no trailing slash (`/` becomes "").
#[must_use]
pub fn normalize_base_url(base: &str) -> String {
    let trimmed = base.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        String::new()
    } else if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

fn join_url(base: &str, path: &str) -> String {
    if base.is_empty() {
        format!("/{path}")
    } else {
        format!("{base}/{path}")
    }
}

/// Serve HTTP and WebSocket requests until the listener fails.
pub async fn serve(
    options: ServeOptions,
    base_topic: String,
    devices: Arc<DeviceRegistry>,
    events: broadcast::Receiver<PublishedEvent>,
    mqtt: MqttBridge,
) -> std::io::Result<()> {
    if options
        .host
        .as_deref()
        .is_some_and(|host| host.starts_with('/'))
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "frontend unix sockets are not supported",
        ));
    }
    let address = match &options.host {
        Some(host) => format!("{host}:{}", options.port),
        None => format!("0.0.0.0:{}", options.port),
    };
    let listener = TcpListener::bind(&address).await?;
    serve_on(listener, options, base_topic, devices, events, mqtt).await
}

/// Serve on an already-bound listener (also used by tests/harness to obtain an
/// ephemeral port).
pub async fn serve_on(
    listener: TcpListener,
    options: ServeOptions,
    base_topic: String,
    devices: Arc<DeviceRegistry>,
    mut events: broadcast::Receiver<PublishedEvent>,
    mqtt: MqttBridge,
) -> std::io::Result<()> {
    let address = listener.local_addr()?;
    info!(
        "Frontend listening on {address}{} (websocket {})",
        if options.disable_ui_serving {
            " (UI serving disabled)"
        } else {
            ""
        },
        options.ws_path()
    );

    // Retained messages, as replayed to every new client, and the live frame
    // bus every connected client follows.
    let retained: Arc<Mutex<BTreeMap<String, Value>>> = Arc::new(Mutex::new(BTreeMap::new()));
    let (frames, _) = broadcast::channel::<Arc<String>>(512);
    let _ingest = {
        let retained = Arc::clone(&retained);
        let frames = frames.clone();
        let base_topic = base_topic.clone();
        tokio::spawn(async move {
            loop {
                match events.recv().await {
                    Ok(event) => {
                        let Some(suffix) = event
                            .topic
                            .strip_prefix(&format!("{base_topic}/"))
                            .map(str::to_string)
                        else {
                            continue;
                        };
                        let payload = parse_payload(&event.payload);
                        if event.retain {
                            let mut retained = retained.lock().await;
                            if event.payload.is_empty() {
                                retained.remove(&suffix);
                            } else {
                                retained.insert(suffix.clone(), payload.clone());
                            }
                        }
                        let frame = Arc::new(
                            serde_json::json!({"topic": suffix, "payload": payload}).to_string(),
                        );
                        let _ = frames.send(frame);
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        debug!("frontend event bus lagged, skipped {skipped} messages");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        })
    };

    let options = Arc::new(options);
    loop {
        let (stream, peer) = listener.accept().await?;
        let options = Arc::clone(&options);
        let retained = Arc::clone(&retained);
        let frames = frames.subscribe();
        let devices = Arc::clone(&devices);
        let base_topic = base_topic.clone();
        let mqtt = mqtt.clone();
        tokio::spawn(async move {
            if let Err(error) =
                handle_connection(stream, options, base_topic, devices, retained, frames, mqtt)
                    .await
            {
                debug!("frontend connection from {peer} ended: {error}");
            }
        });
    }
}

fn parse_payload(payload: &[u8]) -> Value {
    serde_json::from_slice(payload)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(payload).into_owned()))
}

struct Request {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
}

async fn read_request(stream: &mut TcpStream) -> std::io::Result<Option<Request>> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            return Ok(None);
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if buffer.len() > MAX_HEADER_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "HTTP header too large",
            ));
        }
    }
    let text = String::from_utf8_lossy(&buffer);
    let mut lines = text.split("\r\n");
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();
    let mut headers = BTreeMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    Ok(Some(Request {
        method,
        path,
        headers,
    }))
}

async fn handle_connection(
    mut stream: TcpStream,
    options: Arc<ServeOptions>,
    base_topic: String,
    devices: Arc<DeviceRegistry>,
    retained: Arc<Mutex<BTreeMap<String, Value>>>,
    mut frames: broadcast::Receiver<Arc<String>>,
    mqtt: MqttBridge,
) -> std::io::Result<()> {
    let Some(request) = read_request(&mut stream).await? else {
        return Ok(());
    };
    let path_only = request.path.split('?').next().unwrap_or("/");
    let is_websocket = request
        .headers
        .get("upgrade")
        .is_some_and(|value| value.eq_ignore_ascii_case("websocket"));

    if is_websocket {
        if path_only != options.ws_path() {
            respond(&mut stream, 404, "Not Found", "text/plain", b"Not Found").await?;
            return Ok(());
        }
        let key = request
            .headers
            .get("sec-websocket-key")
            .cloned()
            .unwrap_or_default();
        let accept = ws::accept_key(&key);
        let response = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
        );
        stream.write_all(response.as_bytes()).await?;
        stream.flush().await?;

        let token = request.path.split_once('?').and_then(|(_, query)| {
            query
                .split('&')
                .find_map(|pair| pair.strip_prefix("token="))
                .map(str::to_string)
        });
        if let Some(expected) = &options.auth_token {
            if token.as_deref() != Some(expected.as_str()) {
                ws::write_frame(&mut stream, &ws::close_frame(4401, "Unauthorized")).await?;
                return Ok(());
            }
        }

        for (topic, payload) in retained.lock().await.iter() {
            let frame = serde_json::json!({"topic": topic, "payload": payload}).to_string();
            ws::write_frame(&mut stream, &ws::text_frame(&frame)).await?;
        }
        for device in devices.all_devices() {
            let frame = serde_json::json!({
                "topic": device.friendly_name,
                "payload": Value::Object(device.state.clone()),
            })
            .to_string();
            ws::write_frame(&mut stream, &ws::text_frame(&frame)).await?;
        }

        loop {
            tokio::select! {
                frame = ws::read_frame(&mut stream) => {
                    match frame? {
                        Some(ws::Frame::Text(text)) => {
                            let Ok(message) = serde_json::from_str::<Value>(&text) else {
                                continue;
                            };
                            let Some(topic) = message.get("topic").and_then(|topic| topic.as_str()) else {
                                continue;
                            };
                            let Some(payload) = message.get("payload") else {
                                continue;
                            };
                            let payload = serde_json::to_vec(payload).unwrap_or_default();
                            // Upstream calls `mqtt.onMessage` directly. In this
                            // architecture the bridge event loop already
                            // subscribes to the base topic, so publishing is
                            // the equivalent entry point.
                            let _ = mqtt
                                .publish_raw(&format!("{base_topic}/{topic}"), &payload)
                                .await;
                        }
                        Some(ws::Frame::Ping(payload)) => {
                            ws::write_frame(&mut stream, &ws::pong_frame(&payload)).await?;
                        }
                        Some(ws::Frame::Close(_, _)) | None => break,
                        Some(_) => {}
                    }
                }
                frame = frames.recv() => {
                    match frame {
                        Ok(frame) => ws::write_frame(&mut stream, &ws::text_frame(&frame)).await?,
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
        return Ok(());
    }

    // Static frontend (upstream `createStaticFileServer`).
    if options.disable_ui_serving {
        respond(&mut stream, 404, "Not Found", "text/plain", b"Not Found").await?;
        return Ok(());
    }
    if !options.base_url.is_empty() && !path_only.starts_with(&options.base_url) {
        respond(&mut stream, 404, "Not Found", "text/plain", b"Not Found").await?;
        return Ok(());
    }
    if path_only == options.base_url && !request.path.ends_with('/') && !options.base_url.is_empty()
    {
        let location = format!("{}/", options.base_url);
        let response = format!(
            "HTTP/1.1 301 Moved Permanently\r\nLocation: {location}\r\nContent-Length: 0\r\n\r\n"
        );
        stream.write_all(response.as_bytes()).await?;
        return Ok(());
    }
    let relative = path_only
        .strip_prefix(&options.base_url)
        .unwrap_or(path_only)
        .trim_start_matches('/');
    if relative.split('/').any(|part| part == "..") {
        respond(&mut stream, 404, "Not Found", "text/plain", b"Not Found").await?;
        return Ok(());
    }
    let mut file = options.static_dir.join(relative);
    if relative.is_empty() || file.is_dir() {
        file = file.join("index.html");
    }
    if file.is_file() && (request.method == "GET" || request.method == "HEAD") {
        // srvx serves the precompressed `.br`/`.gz` variant when the client
        // accepts it (`createStaticFileServer`).
        let accept = request
            .headers
            .get("accept-encoding")
            .map(String::as_str)
            .unwrap_or("");
        let compressed = if accept.split(',').any(|item| item.trim().starts_with("br")) {
            let candidate = PathBuf::from(format!("{}.br", file.display()));
            candidate.is_file().then_some((candidate, "br"))
        } else if accept
            .split(',')
            .any(|item| item.trim().starts_with("gzip"))
        {
            let candidate = PathBuf::from(format!("{}.gz", file.display()));
            candidate.is_file().then_some((candidate, "gzip"))
        } else {
            None
        };
        let content_type = content_type(&file);
        let cache = if content_type.starts_with("text/html") {
            "Cache-Control: no-store\r\n"
        } else {
            ""
        };
        let (file, encoding) = match compressed {
            Some((candidate, encoding)) => (candidate, Some(encoding)),
            None => (file, None),
        };
        let body = tokio::fs::read(&file).await?;
        let encoding_header = encoding
            .map(|encoding| format!("Content-Encoding: {encoding}\r\n"))
            .unwrap_or_default();
        if request.method == "HEAD" {
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\n{encoding_header}{cache}Vary: Accept-Encoding\r\nContent-Length: {}\r\n\r\n",
                body.len()
            );
            stream.write_all(head.as_bytes()).await?;
        } else {
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\n{encoding_header}{cache}Vary: Accept-Encoding\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(head.as_bytes()).await?;
            stream.write_all(&body).await?;
            stream.flush().await?;
        }
        return Ok(());
    }
    // `staticFileServer.sendNotFound`: security headers + HTML body.
    let message = format!("Cannot {} {}", request.method, request.path);
    let escaped = message
        .replace('&', "&#38;")
        .replace('<', "&#60;")
        .replace('>', "&#62;")
        .replace('"', "&#34;")
        .replace('\'', "&#39;");
    let body = format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>Error</title>\n</head>\n<body>\n<pre>{escaped}</pre>\n</body>\n</html>\n"
    );
    let head = format!(
        "HTTP/1.1 404 Not Found\r\nContent-Security-Policy: default-src 'none'\r\nX-Content-Type-Options: nosniff\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    if request.method != "HEAD" {
        stream.write_all(body.as_bytes()).await?;
    }
    stream.flush().await
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") | Some("map") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("ttf") => "font/ttf",
        _ => "application/octet-stream",
    }
}

async fn respond(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_normalization_matches_posix_join_inputs() {
        assert_eq!(normalize_base_url("/"), "");
        assert_eq!(normalize_base_url(""), "");
        assert_eq!(normalize_base_url("/z2m"), "/z2m");
        assert_eq!(normalize_base_url("z2m/"), "/z2m");
        assert_eq!(join_url("", "api"), "/api");
        assert_eq!(join_url("/z2m", "api"), "/z2m/api");
    }

    #[test]
    fn content_types_cover_windfront_assets() {
        assert_eq!(
            content_type(Path::new("a.html")),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            content_type(Path::new("a.js")),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(content_type(Path::new("a.svg")), "image/svg+xml");
        assert_eq!(content_type(Path::new("a.woff2")), "font/woff2");
        assert_eq!(content_type(Path::new("a.bin")), "application/octet-stream");
    }

    mod server {
        use super::*;
        use crate::config::MqttConfig;
        use crate::devices::Device;
        use crate::mqtt::fake_broker::FakeBroker;
        use crate::zigbee::IeeeAddr;
        use tokio::io::AsyncWriteExt;

        async fn handshake(stream: &mut TcpStream, path: &str) -> String {
            let request = format!(
                "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
            );
            stream.write_all(request.as_bytes()).await.unwrap();
            let mut response = Vec::new();
            // Read byte-wise: the server may append WebSocket frames to the
            // same TCP segment as the 101 response.
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

        fn masked_text(text: &str) -> Vec<u8> {
            let payload = text.as_bytes();
            let mut frame = vec![0x81, 0x80 | payload.len() as u8];
            let mask = [0x11u8, 0x22, 0x33, 0x44];
            frame.extend_from_slice(&mask);
            for (index, byte) in payload.iter().enumerate() {
                frame.push(byte ^ mask[index % 4]);
            }
            frame
        }

        async fn wait_for_publish(broker: &FakeBroker, topic: &str) -> Vec<u8> {
            for _ in 0..100 {
                if let Some(item) = broker
                    .publishes()
                    .into_iter()
                    .find(|item| item.topic == topic)
                {
                    return item.payload.clone();
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            panic!("no publish on {topic}; saw {:?}", broker.publishes());
        }

        async fn start(options: ServeOptions) -> (u16, Arc<DeviceRegistry>, FakeBroker) {
            let broker = FakeBroker::start().await;
            let cfg = MqttConfig {
                server: "127.0.0.1".into(),
                port: broker.port,
                base_topic: "zigbee2mqtt".into(),
                client_id: format!("z2m-frontend-test-{}", std::process::id()),
                ..MqttConfig::default()
            };
            let (events_tx, events_rx) = broadcast::channel(64);
            let (mqtt, _commands) = MqttBridge::connect_with_events(&cfg, Some(events_tx))
                .await
                .unwrap();
            let ieee = IeeeAddr::from_hex("0x00124b0024c1e2f3").unwrap();
            let mut device = Device::new(ieee, 0x1234);
            device.friendly_name = "bulb".into();
            device
                .state
                .insert("state".into(), Value::String("ON".into()));
            let devices = Arc::new(DeviceRegistry::new());
            devices.add(device);
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            let mqtt_for_server = mqtt.clone();
            let devices_for_server = Arc::clone(&devices);
            tokio::spawn(async move {
                serve_on(
                    listener,
                    options,
                    "zigbee2mqtt".into(),
                    devices_for_server,
                    events_rx,
                    mqtt_for_server,
                )
                .await
                .unwrap();
            });
            // Seed retained state after the ingest task is running.
            mqtt.publish_bridge_state(true).await.unwrap();
            mqtt.publish_device_state("bulb", &serde_json::json!({"state": "ON"}))
                .await
                .unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
            (port, devices, broker)
        }

        fn options(static_dir: PathBuf, auth_token: Option<&str>) -> ServeOptions {
            ServeOptions {
                host: Some("127.0.0.1".into()),
                port: 0,
                base_url: String::new(),
                auth_token: auth_token.map(str::to_string),
                disable_ui_serving: false,
                static_dir,
            }
        }

        #[tokio::test]
        async fn websocket_auth_replay_forwarding_and_static_files() {
            let dir = std::env::temp_dir().join(format!("z2m-frontend-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("index.html"), "<html>windfront</html>").unwrap();
            let (port, _devices, broker) = start(options(dir.clone(), Some("secret"))).await;

            // Wrong token: upgrade succeeds, then close 4401 (upstream `ws`).
            let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
            let response = handshake(&mut stream, "/api").await;
            assert!(response.starts_with("HTTP/1.1 101"), "{response}");
            assert_eq!(
                ws::read_frame(&mut stream).await.unwrap().unwrap(),
                ws::Frame::Close(Some(4401), "Unauthorized".into())
            );

            // Correct token: retained state + device state, then live forwarding.
            let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
            let response = handshake(&mut stream, "/api?token=secret").await;
            assert!(response.starts_with("HTTP/1.1 101"), "{response}");
            let first = ws::read_frame(&mut stream).await.unwrap().unwrap();
            let second = ws::read_frame(&mut stream).await.unwrap().unwrap();
            let frames: Vec<Value> = [first, second]
                .into_iter()
                .map(|frame| match frame {
                    ws::Frame::Text(text) => serde_json::from_str(&text).unwrap(),
                    other => panic!("unexpected frame {other:?}"),
                })
                .collect();
            assert_eq!(frames[0]["topic"], "bridge/state");
            assert_eq!(frames[0]["payload"]["state"], "online");
            assert_eq!(frames[1]["topic"], "bulb");
            assert_eq!(frames[1]["payload"]["state"], "ON");

            stream
                .write_all(&masked_text(
                    r#"{"topic":"bridge/request/test","payload":{"value":1}}"#,
                ))
                .await
                .unwrap();
            let published = wait_for_publish(&broker, "zigbee2mqtt/bridge/request/test").await;
            assert_eq!(
                serde_json::from_slice::<Value>(&published).unwrap(),
                serde_json::json!({"value": 1})
            );

            // Static file serving on the same listener.
            let mut http = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
            http.write_all(b"GET / HTTP/1.1\r\nHost: x\r\n\r\n")
                .await
                .unwrap();
            let mut body = Vec::new();
            let mut chunk = [0u8; 512];
            loop {
                let read = http.read(&mut chunk).await.unwrap();
                if read == 0 {
                    break;
                }
                body.extend_from_slice(&chunk[..read]);
            }
            let body = String::from_utf8_lossy(&body);
            assert!(body.starts_with("HTTP/1.1 200 OK"), "{body}");
            assert!(body.contains("windfront"));
            std::fs::remove_dir_all(&dir).ok();
        }

        #[tokio::test]
        async fn base_url_redirects_without_trailing_slash() {
            let dir =
                std::env::temp_dir().join(format!("z2m-frontend-base-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("index.html"), "ok").unwrap();
            let mut serve_options = options(dir.clone(), None);
            serve_options.base_url = "/z2m".into();
            let (port, _devices, _broker) = start(serve_options).await;
            let mut http = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
            http.write_all(b"GET /z2m HTTP/1.1\r\nHost: x\r\n\r\n")
                .await
                .unwrap();
            let mut response = Vec::new();
            let mut chunk = [0u8; 512];
            let read = http.read(&mut chunk).await.unwrap();
            response.extend_from_slice(&chunk[..read]);
            let response = String::from_utf8_lossy(&response);
            assert!(response.starts_with("HTTP/1.1 301"), "{response}");
            assert!(response.contains("Location: /z2m/"), "{response}");
            std::fs::remove_dir_all(&dir).ok();
        }
    }
}
