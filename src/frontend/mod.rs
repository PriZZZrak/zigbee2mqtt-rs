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
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
#[cfg(unix)]
use tokio::net::UnixListener;
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
    if let Some(socket) = options
        .host
        .as_deref()
        .filter(|host| host.starts_with('/'))
        .map(str::to_string)
    {
        return serve_socket(socket, options, base_topic, devices, events, mqtt).await;
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
    events: broadcast::Receiver<PublishedEvent>,
    mqtt: MqttBridge,
) -> std::io::Result<()> {
    let address = listener.local_addr()?;
    let runtime = Runtime::new(options, devices, mqtt);
    info!(
        "Frontend listening on {address}{} (websocket {})",
        runtime.ui_suffix(),
        runtime.options.ws_path()
    );
    let _ingest = runtime.ingest(events, base_topic);
    loop {
        let (stream, peer) = listener.accept().await?;
        runtime.spawn(stream, peer.to_string());
    }
}

/// Unix-domain socket listener (`frontend.host` is an absolute path upstream).
#[cfg(unix)]
async fn serve_socket(
    socket_path: String,
    options: ServeOptions,
    base_topic: String,
    devices: Arc<DeviceRegistry>,
    events: broadcast::Receiver<PublishedEvent>,
    mqtt: MqttBridge,
) -> std::io::Result<()> {
    // A crashed process leaves the socket file behind; only a socket may be
    // replaced, never an arbitrary file.
    if let Ok(metadata) = std::fs::symlink_metadata(&socket_path) {
        use std::os::unix::fs::FileTypeExt;
        if metadata.file_type().is_socket() {
            std::fs::remove_file(&socket_path)?;
        }
    }
    let listener = UnixListener::bind(&socket_path)?;
    let runtime = Runtime::new(options, devices, mqtt);
    info!(
        "Frontend listening on unix:{socket_path}{} (websocket {})",
        runtime.ui_suffix(),
        runtime.options.ws_path()
    );
    let _ingest = runtime.ingest(events, base_topic);
    loop {
        let (stream, _peer) = listener.accept().await?;
        runtime.spawn(stream, socket_path.clone());
    }
}

#[cfg(not(unix))]
async fn serve_socket(
    _socket_path: String,
    _options: ServeOptions,
    _base_topic: String,
    _devices: Arc<DeviceRegistry>,
    _events: broadcast::Receiver<PublishedEvent>,
    _mqtt: MqttBridge,
) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "frontend unix sockets are not supported on this platform",
    ))
}

/// Per-listener state shared by TCP and unix socket accept loops.
struct Runtime {
    options: Arc<ServeOptions>,
    devices: Arc<DeviceRegistry>,
    retained: Arc<Mutex<BTreeMap<String, Value>>>,
    frames: broadcast::Sender<Arc<String>>,
    mqtt: MqttBridge,
}

impl Runtime {
    fn new(options: ServeOptions, devices: Arc<DeviceRegistry>, mqtt: MqttBridge) -> Self {
        let (frames, _) = broadcast::channel(512);
        Self {
            options: Arc::new(options),
            devices,
            retained: Arc::new(Mutex::new(BTreeMap::new())),
            frames,
            mqtt,
        }
    }

    fn ui_suffix(&self) -> &'static str {
        if self.options.disable_ui_serving {
            " (UI serving disabled)"
        } else {
            ""
        }
    }

    /// Consume the MQTT publish bus: update the retained map and fan frames out
    /// to every connected client.
    fn ingest(
        &self,
        mut events: broadcast::Receiver<PublishedEvent>,
        base_topic: String,
    ) -> tokio::task::JoinHandle<()> {
        let retained = Arc::clone(&self.retained);
        let frames = self.frames.clone();
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
    }

    fn spawn<S>(&self, stream: S, peer: String)
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let options = Arc::clone(&self.options);
        let retained = Arc::clone(&self.retained);
        let frames = self.frames.subscribe();
        let devices = Arc::clone(&self.devices);
        let mqtt = self.mqtt.clone();
        tokio::spawn(async move {
            if let Err(error) =
                handle_connection(stream, options, devices, retained, frames, mqtt).await
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

async fn read_request<S: AsyncRead + Unpin>(stream: &mut S) -> std::io::Result<Option<Request>> {
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

async fn handle_connection<S: AsyncRead + AsyncWrite + Unpin>(
    mut stream: S,
    options: Arc<ServeOptions>,
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
                            if !mqtt.dispatch_frontend_message(topic, &payload) {
                                debug!("frontend message on unrecognized topic '{topic}'");
                            }
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
    if relative
        .split('/')
        .any(|part| part == ".." || (part.starts_with('.') && part != ".well-known"))
    {
        respond(&mut stream, 404, "Not Found", "text/plain", b"Not Found").await?;
        return Ok(());
    }
    let trailing = relative.is_empty() || relative.ends_with('/');
    let trimmed = relative.trim_end_matches('/');
    let candidates: Vec<PathBuf> = if trailing {
        vec![options.static_dir.join(if trimmed.is_empty() {
            "index.html".to_string()
        } else {
            format!("{trimmed}/index.html")
        })]
    } else if Path::new(relative).extension().is_none() {
        vec![
            options.static_dir.join(relative),
            options.static_dir.join(format!("{relative}.html")),
            options.static_dir.join(format!("{relative}/index.html")),
        ]
    } else {
        vec![options.static_dir.join(relative)]
    };
    let Some(file) = candidates.into_iter().find(|candidate| candidate.is_file()) else {
        // `staticFileServer.sendNotFound`: security headers + HTML body.
        return send_not_found(&mut stream, &request).await;
    };
    let content_type = content_type(&file);
    let compressible = is_compressible(content_type);
    let range_header = if request.method == "GET" {
        request
            .headers
            .get("range")
            .filter(|header| header.starts_with("bytes="))
            .cloned()
    } else {
        None
    };
    let accept = request
        .headers
        .get("accept-encoding")
        .map(String::as_str)
        .unwrap_or("");
    let (br_ok, gzip_ok) = accepted_encodings(accept);
    let mut served_file = file.clone();
    let mut encoding: Option<&str> = None;
    if compressible && range_header.is_none() {
        let br = PathBuf::from(format!("{}.br", file.display()));
        let gzip = PathBuf::from(format!("{}.gz", file.display()));
        if br_ok && br.is_file() {
            served_file = br;
            encoding = Some("br");
        } else if gzip_ok && gzip.is_file() {
            served_file = gzip;
            encoding = Some("gzip");
        }
    }
    let metadata = tokio::fs::metadata(&served_file).await?;
    let size = metadata.len();
    let mtime_ms = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);
    let last_modified_ms = (mtime_ms / 1000 * 1000).min(now_ms / 1000 * 1000);
    let last_modified = http_date(last_modified_ms);
    let etag_value = weak_etag(size, mtime_ms, encoding);
    let mut headers = Vec::new();
    headers.push(format!("Content-Type: {content_type}"));
    headers.push(format!("Content-Length: {size}"));
    if let Some(encoding) = encoding {
        headers.push(format!("Content-Encoding: {encoding}"));
    } else {
        headers.push("Accept-Ranges: bytes".into());
    }
    if compressible {
        headers.push("Vary: Accept-Encoding".into());
    }
    headers.push(format!("ETag: {etag_value}"));
    headers.push(format!("Last-Modified: {last_modified}"));
    if content_type.starts_with("text/html") {
        headers.push("Cache-Control: no-store".into());
    }

    // Conditional requests (`If-None-Match` wins over `If-Modified-Since`).
    let conditional = if let Some(if_none_match) = request.headers.get("if-none-match") {
        matches_if_none_match(if_none_match, &etag_value)
    } else {
        request
            .headers
            .get("if-modified-since")
            .is_some_and(|since| matches_if_modified_since(since, last_modified_ms))
    };
    if conditional {
        let mut response = String::from("HTTP/1.1 304 Not Modified\r\n");
        for header in headers.iter().filter(|header| {
            header.starts_with("ETag:")
                || header.starts_with("Last-Modified:")
                || header.starts_with("Vary:")
                || header.starts_with("Cache-Control:")
        }) {
            response.push_str(header);
            response.push_str("\r\n");
        }
        response.push_str("Connection: close\r\n\r\n");
        stream.write_all(response.as_bytes()).await?;
        return stream.flush().await;
    }

    let body = tokio::fs::read(&served_file).await?;
    if let Some(range_header) = range_header {
        let if_range_ok = request
            .headers
            .get("if-range")
            .map(|value| matches_if_range(value, &last_modified))
            .unwrap_or(true);
        if if_range_ok {
            match parse_range(&range_header, size) {
                RangeResult::Unsatisfiable => {
                    let response = format!(
                        "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{size}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    );
                    stream.write_all(response.as_bytes()).await?;
                    return stream.flush().await;
                }
                RangeResult::Satisfiable(start, end) => {
                    let mut response = String::from("HTTP/1.1 206 Partial Content\r\n");
                    for header in &headers {
                        response.push_str(header);
                        response.push_str("\r\n");
                    }
                    response.push_str(&format!(
                        "Content-Range: bytes {start}-{end}/{size}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        end - start + 1
                    ));
                    stream.write_all(response.as_bytes()).await?;
                    stream
                        .write_all(&body[start as usize..=end as usize])
                        .await?;
                    return stream.flush().await;
                }
                RangeResult::Ignore => {}
            }
        }
    }

    let mut response = String::from("HTTP/1.1 200 OK\r\n");
    for header in &headers {
        response.push_str(header);
        response.push_str("\r\n");
    }
    response.push_str("Connection: close\r\n\r\n");
    stream.write_all(response.as_bytes()).await?;
    if request.method != "HEAD" {
        stream.write_all(&body).await?;
    }
    stream.flush().await
}

async fn send_not_found<S: AsyncWrite + Unpin>(
    stream: &mut S,
    request: &Request,
) -> std::io::Result<()> {
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

#[derive(Debug, PartialEq, Eq)]
enum RangeResult {
    Ignore,
    Unsatisfiable,
    Satisfiable(u64, u64),
}

fn parse_range(header: &str, size: u64) -> RangeResult {
    let Some(spec) = header.strip_prefix("bytes=") else {
        return RangeResult::Ignore;
    };
    let Some((start_text, end_text)) = spec.split_once('-') else {
        return RangeResult::Ignore;
    };
    if start_text.is_empty() {
        let Ok(length) = end_text.parse::<u64>() else {
            return RangeResult::Ignore;
        };
        if length == 0 || size == 0 {
            return RangeResult::Unsatisfiable;
        }
        let start = size.saturating_sub(length);
        return RangeResult::Satisfiable(start, size - 1);
    }
    let Ok(start) = start_text.parse::<u64>() else {
        return RangeResult::Ignore;
    };
    let end = if end_text.is_empty() {
        size.saturating_sub(1)
    } else {
        let Ok(end) = end_text.parse::<u64>() else {
            return RangeResult::Ignore;
        };
        if end < start {
            return RangeResult::Ignore;
        }
        end.min(size.saturating_sub(1))
    };
    if start >= size {
        RangeResult::Unsatisfiable
    } else {
        RangeResult::Satisfiable(start, end)
    }
}

fn matches_if_none_match(header: &str, etag: &str) -> bool {
    if header.trim() == "*" {
        return true;
    }
    let bare = etag.strip_prefix("W/").unwrap_or(etag);
    header.split(',').any(|candidate| {
        candidate
            .trim()
            .strip_prefix("W/")
            .unwrap_or(candidate.trim())
            == bare
    })
}

/// `srvx` `matchesIfModifiedSince`: parse the header and accept when the
/// resource is not newer than the client's copy.
fn matches_if_modified_since(header: &str, last_modified_ms: u64) -> bool {
    parse_http_date(header).is_some_and(|since| last_modified_ms <= since)
}

/// `srvx` `matchesIfRange`: a quoted/weak validator never matches a range.
fn matches_if_range(header: &str, last_modified: &str) -> bool {
    let value = header.trim();
    if value.starts_with('"') || value.starts_with("W/") {
        return false;
    }
    value == last_modified
}

/// Parse an IMF-fixdate (`Sun, 06 Nov 1994 08:49:37 GMT`) into milliseconds
/// since the epoch. Other HTTP date formats accepted by `Date.parse` are not
/// needed here: the server only ever emits IMF-fixdate and clients replay it.
fn parse_http_date(value: &str) -> Option<u64> {
    let (_, rest) = value.trim().split_once(", ")?;
    let mut parts = rest.split_whitespace();
    let day: i64 = parts.next()?.parse().ok()?;
    let month = match parts.next()? {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let year: i64 = parts.next()?.parse().ok()?;
    let time = parts.next()?;
    if parts.next()? != "GMT" || parts.next().is_some() {
        return None;
    }
    let mut clock = time.split(':');
    let hour: u64 = clock.next()?.parse().ok()?;
    let minute: u64 = clock.next()?.parse().ok()?;
    let second: u64 = clock.next()?.parse().ok()?;
    if !(1..=31).contains(&day) || hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    let days = days_from_civil(year, month, day);
    if days < 0 {
        return None;
    }
    Some(days as u64 * 86_400_000 + hour * 3_600_000 + minute * 60_000 + second * 1000)
}

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn accepted_encodings(header: &str) -> (bool, bool) {
    if header.is_empty() {
        return (false, false);
    }
    let mut br: Option<f32> = None;
    let mut gzip: Option<f32> = None;
    let mut wildcard: Option<f32> = None;
    for part in header.split(',') {
        let mut pieces = part.split(';');
        let name = pieces.next().unwrap_or("").trim().to_ascii_lowercase();
        if name.is_empty() {
            continue;
        }
        let mut quality = 1.0f32;
        for parameter in pieces {
            let parameter = parameter.trim();
            if let Some(value) = parameter.strip_prefix("q=") {
                quality = value.parse().unwrap_or(0.0);
            }
        }
        match name.as_str() {
            "br" => br = Some(quality),
            "gzip" => gzip = Some(quality),
            "*" => wildcard = Some(quality),
            _ => {}
        }
    }
    let effective = |value: Option<f32>| value.or(wildcard).unwrap_or(0.0) > 0.0;
    (effective(br), effective(gzip))
}

fn is_compressible(content_type: &str) -> bool {
    content_type.starts_with("text/")
        || content_type.ends_with("+json")
        || content_type.ends_with("+xml")
        || content_type == "application/json"
        || content_type == "application/xml"
        || content_type == "application/wasm"
}

fn weak_etag(size: u64, mtime_ms: u64, encoding: Option<&str>) -> String {
    let tag = format!("{size:x}-{mtime_ms:x}");
    match encoding {
        Some(encoding) => format!("W/\"{tag}-{encoding}\""),
        None => format!("W/\"{tag}\""),
    }
}

fn http_date(milliseconds: u64) -> String {
    let seconds = milliseconds / 1000;
    let days = seconds / 86_400;
    let rem = seconds % 86_400;
    let (hour, minute, second) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Howard Hinnant's civil_from_days, for days since 1970-01-01.
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    let weekday = (days + 4) % 7;
    let weekdays = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!(
        "{}, {:02} {} {:04} {:02}:{:02}:{:02} GMT",
        weekdays[weekday as usize],
        day,
        months[(month - 1) as usize],
        year,
        hour,
        minute,
        second
    )
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") | Some("map") => "application/json",
        Some("txt") => "text/plain; charset=utf-8",
        Some("xml") => "application/xml; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/vnd.microsoft.icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("ttf") => "font/ttf",
        _ => "application/octet-stream",
    }
}

async fn respond<S: AsyncWrite + Unpin>(
    stream: &mut S,
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
    fn weak_etag_matches_srvx_format() {
        // `computeETag`: size and mtime in lowercase hex, weak tag, suffix per
        // precompressed encoding.
        assert_eq!(
            weak_etag(0x13, 1_700_000_000_000, None),
            "W/\"13-18bcfe56800\""
        );
        assert_eq!(
            weak_etag(0xd, 1_700_000_000_000, Some("br")),
            "W/\"d-18bcfe56800-br\""
        );
        assert_eq!(
            weak_etag(0xa, 1_700_000_000_000, Some("gzip")),
            "W/\"a-18bcfe56800-gzip\""
        );
    }

    #[test]
    fn http_date_round_trips_imf_fixdate() {
        assert_eq!(
            http_date(1_700_000_000_000),
            "Tue, 14 Nov 2023 22:13:20 GMT"
        );
        assert_eq!(
            parse_http_date("Tue, 14 Nov 2023 22:13:20 GMT"),
            Some(1_700_000_000_000)
        );
        assert_eq!(parse_http_date("not a date"), None);
        // Later client copies are not modified.
        assert!(matches_if_modified_since(
            "Wed, 15 Nov 2023 00:00:00 GMT",
            1_700_000_000_000
        ));
        // Older copies are modified.
        assert!(!matches_if_modified_since(
            "Mon, 13 Nov 2023 00:00:00 GMT",
            1_700_000_000_000
        ));
    }

    #[test]
    fn parse_range_matches_srvx_rules() {
        assert_eq!(parse_range("bytes=0-4", 19), RangeResult::Satisfiable(0, 4));
        assert_eq!(
            parse_range("bytes=0-100", 19),
            RangeResult::Satisfiable(0, 18)
        );
        assert_eq!(
            parse_range("bytes=-5", 19),
            RangeResult::Satisfiable(14, 18)
        );
        assert_eq!(
            parse_range("bytes=-100", 19),
            RangeResult::Satisfiable(0, 18)
        );
        assert_eq!(parse_range("bytes=100-", 19), RangeResult::Unsatisfiable);
        assert_eq!(parse_range("bytes=-0", 19), RangeResult::Unsatisfiable);
        assert_eq!(parse_range("bytes=5-1", 19), RangeResult::Ignore);
        assert_eq!(parse_range("items=0-4", 19), RangeResult::Ignore);
        assert_eq!(parse_range("bytes=0-4,6-9", 19), RangeResult::Ignore);
    }

    #[test]
    fn accepted_encodings_follow_srvx_quality_rules() {
        assert_eq!(accepted_encodings(""), (false, false));
        assert_eq!(accepted_encodings("br"), (true, false));
        assert_eq!(accepted_encodings("gzip"), (false, true));
        // Precompressed variants are served in `br` then `gzip` order.
        assert_eq!(accepted_encodings("gzip, br"), (true, true));
        assert_eq!(accepted_encodings("br;q=0"), (false, false));
        assert_eq!(accepted_encodings("*"), (true, true));
        assert_eq!(accepted_encodings("br;q=0, *"), (false, true));
    }

    #[test]
    fn conditional_validators_match_srvx() {
        assert!(matches_if_none_match("W/\"a-1\"", "W/\"a-1\""));
        assert!(matches_if_none_match("\"a-1\", W/\"a-1\"", "W/\"a-1\""));
        assert!(matches_if_none_match("*", "W/\"a-1\""));
        assert!(!matches_if_none_match("W/\"b-1\"", "W/\"a-1\""));
        assert!(matches_if_range(
            "Tue, 14 Nov 2023 22:13:20 GMT",
            "Tue, 14 Nov 2023 22:13:20 GMT"
        ));
        assert!(!matches_if_range(
            "W/\"13-18bcfe56800\"",
            "Tue, 14 Nov 2023 22:13:20 GMT"
        ));
        assert!(!matches_if_range(
            "\"13-18bcfe56800\"",
            "Tue, 14 Nov 2023 22:13:20 GMT"
        ));
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
        use tokio::net::TcpStream;

        async fn handshake<S: AsyncRead + AsyncWrite + Unpin>(
            stream: &mut S,
            path: &str,
        ) -> String {
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

        async fn start(
            options: ServeOptions,
        ) -> (
            u16,
            Arc<DeviceRegistry>,
            FakeBroker,
            tokio::sync::mpsc::Receiver<crate::mqtt::MqttCommand>,
            MqttBridge,
        ) {
            let broker = FakeBroker::start().await;
            let cfg = MqttConfig {
                server: "127.0.0.1".into(),
                port: broker.port,
                base_topic: "zigbee2mqtt".into(),
                client_id: format!("z2m-frontend-test-{}", std::process::id()),
                ..MqttConfig::default()
            };
            let (events_tx, events_rx) = broadcast::channel(64);
            let (mqtt, commands) = MqttBridge::connect_with_events(&cfg, Some(events_tx))
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
            (port, devices, broker, commands, mqtt)
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
            let (port, _devices, broker, mut commands, mqtt) =
                start(options(dir.clone(), Some("secret"))).await;

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

            // Live publish after connect is fanned out to the client.
            mqtt.publish_device_state("bulb", &serde_json::json!({"state": "OFF"}))
                .await
                .unwrap();
            // The ingest task may still be draining the pre-connect publishes,
            // so skip anything else until the live update arrives.
            let mut live_seen = false;
            for _ in 0..10 {
                let live = tokio::time::timeout(
                    std::time::Duration::from_secs(2),
                    ws::read_frame(&mut stream),
                )
                .await
                .expect("live frame timed out")
                .unwrap()
                .unwrap();
                let live: Value = match live {
                    ws::Frame::Text(text) => serde_json::from_str(&text).unwrap(),
                    other => panic!("unexpected frame {other:?}"),
                };
                if live["topic"] == "bulb" && live["payload"]["state"] == "OFF" {
                    live_seen = true;
                    break;
                }
            }
            assert!(live_seen, "live device update was not forwarded");

            stream
                .write_all(&masked_text(
                    r#"{"topic":"bulb/set","payload":{"state":"OFF"}}"#,
                ))
                .await
                .unwrap();
            // `frontend.ts` calls `mqtt.onMessage`; no broker publish is made
            // for the command itself.
            let command = tokio::time::timeout(std::time::Duration::from_secs(2), commands.recv())
                .await
                .expect("command dispatch timed out")
                .expect("command channel closed");
            match command {
                crate::mqtt::MqttCommand::SetDevice {
                    friendly_name,
                    endpoint,
                    ..
                } => {
                    assert_eq!(friendly_name, "bulb");
                    assert_eq!(endpoint, None);
                }
                other => panic!("unexpected command {other:?}"),
            }
            assert!(
                !broker
                    .publishes()
                    .iter()
                    .any(|item| item.topic == "zigbee2mqtt/bulb/set"),
                "frontend commands must not be echoed through the broker"
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

        #[cfg(unix)]
        #[tokio::test]
        async fn unix_socket_serves_the_websocket() {
            use tokio::net::UnixStream;

            let dir =
                std::env::temp_dir().join(format!("z2m-frontend-unix-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let socket_path = dir.join("frontend.sock");

            let broker = FakeBroker::start().await;
            let cfg = MqttConfig {
                server: "127.0.0.1".into(),
                port: broker.port,
                base_topic: "zigbee2mqtt".into(),
                client_id: format!("z2m-frontend-unix-{}", std::process::id()),
                ..MqttConfig::default()
            };
            let (events_tx, events_rx) = broadcast::channel(64);
            let (mqtt, _commands) = MqttBridge::connect_with_events(&cfg, Some(events_tx))
                .await
                .unwrap();
            let mut unix_options = options(dir.clone(), None);
            unix_options.host = Some(socket_path.to_string_lossy().into_owned());
            let server_mqtt = mqtt.clone();
            let server_socket = socket_path.clone();
            tokio::spawn(async move {
                serve(
                    unix_options,
                    "zigbee2mqtt".into(),
                    Arc::new(DeviceRegistry::new()),
                    events_rx,
                    server_mqtt,
                )
                .await
                .unwrap();
            });
            for _ in 0..200 {
                if server_socket.exists() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            assert!(socket_path.exists(), "unix socket was not created");

            let mut stream = UnixStream::connect(&socket_path).await.unwrap();
            let response = handshake(&mut stream, "/api").await;
            assert!(response.starts_with("HTTP/1.1 101"), "{response}");

            mqtt.publish_bridge_state(true).await.unwrap();
            let frame = tokio::time::timeout(
                std::time::Duration::from_secs(2),
                ws::read_frame(&mut stream),
            )
            .await
            .expect("frame timed out")
            .unwrap()
            .unwrap();
            let frame: Value = match frame {
                ws::Frame::Text(text) => serde_json::from_str(&text).unwrap(),
                other => panic!("unexpected frame {other:?}"),
            };
            assert_eq!(frame["topic"], "bridge/state");
            assert_eq!(frame["payload"]["state"], "online");
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
            let (port, _devices, _broker, _commands, _mqtt) = start(serve_options).await;
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
