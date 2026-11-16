// Original: .upstream/zigbee2mqtt/lib/extension/frontend.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// WebSocket protocol + auth + retained/state replay, static Windfront serving
// (env/package assets path, base_url, SPA fallback, pre-compressed assets) and
// TLS via the pure-Rust rustls-rustcrypto provider.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::Message;
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::{accept_hdr_async, WebSocketStream};
use crate::util::static_file_server::{send_not_found, HttpRequest, StaticFileServer};

use crate::extension::extension::{Extension, ExtensionContext, ExtensionFuture};
use crate::model::entity::Entity;
use crate::util::stringify::stringify;
use crate::util::utils;

type Clients = Arc<Mutex<HashMap<u64, mpsc::UnboundedSender<Message>>>>;

pub struct Frontend {
    context: ExtensionContext,
    base_url: String,
    mqtt_base_topic: String,
    clients: Clients,
    next_id: AtomicU64,
    shutdown: Mutex<Option<oneshot::Sender<()>>>,
    task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    bound: Mutex<Option<std::net::SocketAddr>>,
}

impl Frontend {
    pub fn new(context: ExtensionContext) -> Arc<Self> {
        let settings = context.settings_value();
        Arc::new(Self {
            context,
            base_url: settings["frontend"]["base_url"]
                .as_str()
                .unwrap_or("/")
                .to_string(),
            mqtt_base_topic: settings["mqtt"]["base_topic"]
                .as_str()
                .unwrap_or("zigbee2mqtt")
                .to_string(),
            clients: Arc::new(Mutex::new(HashMap::new())),
            next_id: AtomicU64::new(1),
            shutdown: Mutex::new(None),
            task: Mutex::new(None),
            bound: Mutex::new(None),
        })
    }

    fn api_path(&self) -> String {
        format!(
            "{}api",
            self.base_url.trim_end_matches('/').to_string() + "/"
        )
    }

    /// Test/diagnostics hook: actual bound address (`port: 0` support).
    pub fn bound_addr(&self) -> Option<std::net::SocketAddr> {
        *self.bound.lock().unwrap()
    }

    fn broadcast(&self, value: &Value) {
        let text = stringify(value);
        self.clients
            .lock()
            .unwrap()
            .retain(|_, sender| sender.send(Message::Text(text.clone().into())).is_ok());
    }

    /// Upstream `onMQTTPublishMessageOrEntityState` (`frontend.ts:204-233`).
    fn on_mqtt_published(&self, data: Value) {
        let topic = data.get("topic").and_then(Value::as_str).unwrap_or("");
        let is_entity_state = data
            .get("options")
            .and_then(|options| options.get("meta"))
            .and_then(|meta| meta.get("isEntityState"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let prefix = format!("{}/", self.mqtt_base_topic);
        if is_entity_state || !topic.starts_with(&prefix) {
            return;
        }
        let payload = utils::parse_json(
            data.get("payload").and_then(Value::as_str).unwrap_or(""),
            "",
        );
        if let Some(topic) = topic.strip_prefix(&prefix) {
            self.broadcast(&json!({"topic": topic, "payload": payload}));
        }
    }

    fn on_publish_entity_state(&self, data: Value) {
        let name = data
            .get("entity")
            .and_then(|entity| entity.get("name"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        self.broadcast(&json!({
            "topic": name,
            "payload": data.get("message").cloned().unwrap_or(Value::Null),
        }));
    }

    /// Upstream replay on connection (`frontend.ts:176-201`).
    async fn replay(&self, sender: &mpsc::UnboundedSender<Message>) {
        let retained: Vec<(String, String)> = {
            let mqtt = self.context.mqtt.lock().await;
            let retained_messages = mqtt.retained_messages.lock().unwrap();
            retained_messages
                .values()
                .map(|message| (message.topic.clone(), message.payload.clone()))
                .collect()
        };
        let prefix = format!("{}/", self.mqtt_base_topic);
        for (topic, payload) in retained {
            if let Some(topic) = topic.strip_prefix(&prefix) {
                let _ = sender.send(Message::Text(
                    stringify(&json!({
                        "topic": topic,
                        "payload": utils::parse_json(&payload, &payload),
                    }))
                    .into(),
                ));
            }
        }

        let last_seen = self.context.settings_value()["advanced"]["last_seen"]
            .as_str()
            .unwrap_or("disable")
            .to_string();
        for device in self
            .context
            .zigbee
            .devices_iterator(utils::device_not_coordinator)
        {
            let mut payload = self.context.state.lock().unwrap().get(&device.entity_id());
            if last_seen != "disable" {
                payload["last_seen"] = json!(utils::format_date(
                    device.zh.last_seen().unwrap_or(0) as i64,
                    &last_seen
                ));
            }
            if let Some(linkquality) = device.zh.linkquality() {
                payload["linkquality"] = json!(linkquality);
            }
            let _ = sender.send(Message::Text(
                stringify(&json!({"topic": device.name(), "payload": payload})).into(),
            ));
        }
    }

    async fn handle_socket<S>(
        self: &Arc<Self>,
        socket: WebSocketStream<S>,
        id: u64,
        mut receiver: mpsc::UnboundedReceiver<Message>,
    ) where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static
    {
        let (mut sink, mut stream) = socket.split();
        let writer = tokio::spawn(async move {
            while let Some(message) = receiver.recv().await {
                if sink.send(message).await.is_err() {
                    break;
                }
            }
        });

        while let Some(incoming) = stream.next().await {
            match incoming {
                Ok(Message::Text(text)) => match serde_json::from_str::<Value>(&text) {
                    Ok(value) => {
                        let topic = value.get("topic").and_then(Value::as_str).unwrap_or("");
                        let payload = value.get("payload").cloned().unwrap_or(Value::Null);
                        self.context
                            .mqtt
                            .lock()
                            .await
                            .on_message(
                                &format!("{}/{topic}", self.mqtt_base_topic),
                                Value::String(stringify(&payload)),
                            )
                            .await;
                    }
                    Err(_) => self.context.logger.lock().unwrap().error(
                        &format!("Received invalid message from WebSocket: {text}"),
                        "z2m",
                    ),
                },
                Ok(Message::Close(_)) | Err(_) => break,
                Ok(_) => {}
            }
        }

        writer.abort();
        self.clients.lock().unwrap().remove(&id);
    }

    async fn serve<S>(
        this: Arc<Self>,
        mut stream: S,
        clients: Clients,
        api_path: String,
        auth_token: Option<String>,
        server: Option<Arc<StaticFileServer>>,
    ) -> Result<(), String>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let (head, request) = read_head(&mut stream)
            .await
            .map_err(|error| error.to_string())?;
        let path = request
            .target
            .split(['?', '#'])
            .next()
            .unwrap_or("/")
            .to_string();
        let is_websocket = request
            .headers
            .get("upgrade")
            .is_some_and(|upgrade| upgrade.eq_ignore_ascii_case("websocket"));

        if is_websocket && path == api_path {
            let callback = move |request: &Request,
                                 response: Response|
                  -> Result<Response, ErrorResponse> {
                if let Some(expected) = &auth_token {
                    let provided = request.uri().query().and_then(|query| {
                        query.split('&').find_map(|pair| {
                            let (key, value) = pair.split_once('=')?;
                            (key == "token").then(|| value.to_string())
                        })
                    });
                    if provided.as_deref() != Some(expected.as_str()) {
                        let mut error = ErrorResponse::new(Some("Unauthorized".to_string()));
                        *error.status_mut() = StatusCode::UNAUTHORIZED;
                        return Err(error);
                    }
                }
                Ok(response)
            };
            let socket = accept_hdr_async(Rewind::new(head, stream), callback)
                .await
                .map_err(|error| error.to_string())?;
            let id = this.next_id.fetch_add(1, Ordering::SeqCst);
            let (sender, receiver) = mpsc::unbounded_channel::<Message>();
            clients.lock().unwrap().insert(id, sender.clone());
            this.replay(&sender).await;
            Self::handle_socket(&this, socket, id, receiver).await;
            return Ok(());
        }

        let response = match server {
            Some(server) => server.handle(&request),
            None => send_not_found(&request),
        };
        write_response(&mut stream, &response)
            .await
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

/// Resolves the Windfront assets directory: env override, OpenWrt package
/// path, then the Node package fallback used in development.
pub fn windfront_assets_dir() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(path) = std::env::var("ZIGBEE2MQTT_FRONTEND_ASSETS") {
        candidates.push(PathBuf::from(path));
    }
    candidates.push(PathBuf::from("/usr/share/zigbee2mqtt/windfront"));
    candidates.push(PathBuf::from(
        "node_modules/zigbee2mqtt-windfront/dist",
    ));
    candidates
        .into_iter()
        .find(|path| path.join("index.html").exists())
}

fn resolve_frontend_assets(settings: &Value) -> Option<PathBuf> {
    if settings["frontend"]["disable_ui_serving"]
        .as_bool()
        .unwrap_or(false)
    {
        return None;
    }
    windfront_assets_dir()
}

/// The package version this binary was built and tested against.
const EXPECTED_WINDFRONT_VERSION_RAW: &str =
    include_str!("../../../../packaging/windfront.version");

fn expected_windfront_version() -> &'static str {
    EXPECTED_WINDFRONT_VERSION_RAW.trim()
}

fn check_windfront_version(dir: &Path, logger: &Arc<Mutex<crate::util::logger::Logger>>) -> Result<(), String> {
    // The opkg package puts `package.json` next to the assets; a development
    // checkout (`node_modules/zigbee2mqtt-windfront/dist`) keeps it one level
    // up in the package root.
    let manifest_path = [dir.join("package.json"), dir.join("../package.json")]
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| {
            format!(
                "Windfront assets at '{}' are incomplete: no package.json",
                dir.display()
            )
        })?;
    let manifest = std::fs::read_to_string(&manifest_path)
        .map_err(|error| format!("Windfront assets at '{}' are incomplete: {error}", dir.display()))?;
    let parsed: Value = serde_json::from_str(&manifest)
        .map_err(|error| format!("Invalid Windfront package.json: {error}"))?;
    let version = parsed["version"].as_str().unwrap_or_default();
    if version == expected_windfront_version()
        || std::env::var("ZIGBEE2MQTT_ALLOW_WINDFRONT_MISMATCH").is_ok()
    {
        return Ok(());
    }
    let message = format!(
        "Windfront assets version '{version}' does not match the tested version '{}' (set ZIGBEE2MQTT_ALLOW_WINDFRONT_MISMATCH=1 to override)",
        expected_windfront_version()
    );
    logger.lock().unwrap().warning(&message, "z2m");
    Err(message)
}

/// Replays the bytes already read while detecting the HTTP request head so the
/// WebSocket handshake sees the original stream.
struct Rewind<S> {
    prefix: Vec<u8>,
    position: usize,
    inner: S,
}

impl<S> Rewind<S> {
    fn new(prefix: Vec<u8>, inner: S) -> Self {
        Self {
            prefix,
            position: 0,
            inner,
        }
    }
}

impl<S: tokio::io::AsyncRead + Unpin> tokio::io::AsyncRead for Rewind<S> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
        buffer: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        if self.position < self.prefix.len() {
            let remaining = &self.prefix[self.position..];
            let take = remaining.len().min(buffer.remaining());
            buffer.put_slice(&remaining[..take]);
            self.position += take;
            return std::task::Poll::Ready(Ok(()));
        }
        std::pin::Pin::new(&mut self.inner).poll_read(context, buffer)
    }
}

impl<S: tokio::io::AsyncWrite + Unpin> tokio::io::AsyncWrite for Rewind<S> {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
        buffer: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        std::pin::Pin::new(&mut self.inner).poll_write(context, buffer)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_flush(context)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_shutdown(context)
    }
}

/// Reads the HTTP request head (up to CRLFCRLF) and parses method/target/headers.
async fn read_head<S: tokio::io::AsyncRead + Unpin>(
    stream: &mut S,
) -> std::io::Result<(Vec<u8>, HttpRequest)> {
    use tokio::io::AsyncReadExt as _;

    let mut head = Vec::new();
    let mut chunk = [0u8; 2048];
    let end = loop {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            return Ok((head, HttpRequest::default()));
        }
        head.extend_from_slice(&chunk[..read]);
        if let Some(position) = head.windows(4).position(|window| window == b"\r\n\r\n") {
            break position;
        }
        if head.len() > 64 * 1024 {
            return Ok((head, HttpRequest::default()));
        }
    };
    let text = String::from_utf8_lossy(&head[..end]).to_string();
    let mut lines = text.split("\r\n");
    let mut request = HttpRequest::default();
    if let Some(first) = lines.next() {
        let mut parts = first.split_whitespace();
        request.method = parts.next().unwrap_or("").to_string();
        request.target = parts.next().unwrap_or("/").to_string();
    }
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            request
                .headers
                .insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    Ok((head, request))
}

async fn write_response<S: tokio::io::AsyncWrite + Unpin>(
    stream: &mut S,
    response: &crate::util::static_file_server::HttpResponse,
) -> std::io::Result<()> {
    use tokio::io::AsyncWriteExt as _;

    let reason = match response.status {
        200 => "OK",
        304 => "Not Modified",
        401 => "Unauthorized",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    // Upstream's `http` server writes the headers once: a `Content-Length`
    // supplied by the static file server must not be duplicated here (Node's
    // llhttp rejects duplicate framing headers, curl only tolerates them), and
    // 304 responses carry no body and therefore no `Content-Length`.
    let has_content_length = response
        .headers
        .keys()
        .any(|name| name.eq_ignore_ascii_case("content-length"));
    let mut text = format!("HTTP/1.1 {} {}\r\n", response.status, reason);
    if !has_content_length && response.status != 304 {
        text.push_str(&format!("Content-Length: {}\r\n", response.body.len()));
    }
    text.push_str("Connection: close\r\n");
    for (name, value) in &response.headers {
        text.push_str(&format!("{name}: {value}\r\n"));
    }
    text.push_str("\r\n");
    stream.write_all(text.as_bytes()).await?;
    if response.status != 304 && !response.body.is_empty() {
        stream.write_all(&response.body).await?;
    }
    stream.flush().await
}

impl Extension for Frontend {
    fn name(&self) -> &'static str {
        "Frontend"
    }

    fn context(&self) -> &ExtensionContext {
        &self.context
    }

    fn start(self: Arc<Self>) -> ExtensionFuture {
        Box::pin(async move {
            let settings = self.context.settings_value();
            if !settings["frontend"]["enabled"].as_bool().unwrap_or(false) {
                return Err("Frontend extension created with setting 'enabled: false'".to_string());
            }
            let tls_acceptor = match build_tls_acceptor(&settings, &self.context.logger) {
                Ok(acceptor) => acceptor,
                Err(error) => return Err(error),
            };
            if settings["frontend"]["disable_ui_serving"]
                .as_bool()
                .unwrap_or(false)
            {
                self.context.logger.lock().unwrap().info(
                    &format!(
                        "Frontend UI serving is disabled. WebSocket at: {}:{}",
                        settings["frontend"]["host"].as_str().unwrap_or("0.0.0.0"),
                        settings["frontend"]["port"]
                    ),
                    "z2m",
                );
            }

            let assets = resolve_frontend_assets(&settings);
            let server = match &assets {
                Some(dir) => {
                    check_windfront_version(dir, &self.context.logger)?;
                    self.context.logger.lock().unwrap().info(
                        &format!("Serving Windfront from '{}'", dir.display()),
                        "z2m",
                    );
                    let mut static_server = crate::util::static_file_server::create_static_file_server(
                        dir.clone(),
                        Arc::new(|message: &str| {
                            zigbee_apiarist::utils::logger::logger().error(message, "z2m");
                        }),
                    );
                    static_server.base_url = self.base_url.clone();
                    Some(Arc::new(static_server))
                }
                None => {
                    if !settings["frontend"]["disable_ui_serving"]
                        .as_bool()
                        .unwrap_or(false)
                    {
                        self.context.logger.lock().unwrap().warning(
                            "Windfront assets were not found, the UI will not be served (set ZIGBEE2MQTT_FRONTEND_ASSETS or install zigbee2mqtt-windfront)",
                            "z2m",
                        );
                    }
                    None
                }
            };
            let host = settings["frontend"]["host"]
                .as_str()
                .unwrap_or("")
                .to_string();
            let port = settings["frontend"]["port"].as_u64().unwrap_or(8080) as u16;
            let auth_token = settings["frontend"]["auth_token"]
                .as_str()
                .map(str::to_string);
            let api_path = self.api_path();
            let clients = self.clients.clone();
            let this = self.clone();
            let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
            *self.shutdown.lock().unwrap() = Some(shutdown_tx);

            let task = {
                #[cfg(unix)]
                if host.starts_with('/') {
                    let _ = std::fs::remove_file(&host);
                    let listener = tokio::net::UnixListener::bind(&host)
                        .map_err(|error| error.to_string())?;
                    self.context.logger.lock().unwrap().info(
                        &format!("Started frontend on socket {host}"),
                        "z2m",
                    );
                    tokio::spawn(async move {
                        loop {
                            tokio::select! {
                                _ = &mut shutdown_rx => break,
                                accepted = listener.accept() => {
                                    let Ok((stream, _)) = accepted else { continue };
                                    let clients = clients.clone();
                                    let this = this.clone();
                                    let api_path = api_path.clone();
                                    let auth_token = auth_token.clone();
                                    let tls = tls_acceptor.clone();
                                    let server = server.clone();
                                    let logger = this.context.logger.clone();
                                    tokio::spawn(async move {
                                        match tls {
                                            Some(acceptor) => match acceptor.accept(stream).await {
                                                Ok(tls_stream) => {
                                                    let _ = Self::serve(this, tls_stream, clients, api_path, auth_token, server).await;
                                                }
                                                Err(error) => logger.lock().unwrap().error(
                                                    &format!("TLS handshake failed: {error}"),
                                                    "z2m",
                                                ),
                                            },
                                            None => {
                                                let _ = Self::serve(this, stream, clients, api_path, auth_token, server).await;
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    })
                } else {
                    let listener = TcpListener::bind(if host.is_empty() {
                        format!("0.0.0.0:{port}")
                    } else {
                        format!("{host}:{port}")
                    })
                    .await
                    .map_err(|error| error.to_string())?;
                    *self.bound.lock().unwrap() = listener.local_addr().ok();
                    self.context.logger.lock().unwrap().info(
                        &format!(
                            "Started frontend on port {}",
                            if host.is_empty() {
                                port.to_string()
                            } else {
                                format!("{host}:{port}")
                            }
                        ),
                        "z2m",
                    );
                    tokio::spawn(async move {
                        loop {
                            tokio::select! {
                                _ = &mut shutdown_rx => break,
                                accepted = listener.accept() => {
                                    let Ok((stream, _)) = accepted else { continue };
                                    let clients = clients.clone();
                                    let this = this.clone();
                                    let api_path = api_path.clone();
                                    let auth_token = auth_token.clone();
                                    let server = server.clone();
                                    let tls = tls_acceptor.clone();
                                    let logger = this.context.logger.clone();
                                    tokio::spawn(async move {
                                        match tls {
                                            Some(acceptor) => match acceptor.accept(stream).await {
                                                Ok(tls_stream) => {
                                                    let _ = Self::serve(this, tls_stream, clients, api_path, auth_token, server).await;
                                                }
                                                Err(error) => logger.lock().unwrap().error(
                                                    &format!("TLS handshake failed: {error}"),
                                                    "z2m",
                                                ),
                                            },
                                            None => {
                                                let _ = Self::serve(this, stream, clients, api_path, auth_token, server).await;
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    })
                }
                #[cfg(not(unix))]
                {
                    let listener = TcpListener::bind(if host.is_empty() {
                        format!("0.0.0.0:{port}")
                    } else {
                        format!("{host}:{port}")
                    })
                    .await
                    .map_err(|error| error.to_string())?;
                    *self.bound.lock().unwrap() = listener.local_addr().ok();
                    self.context.logger.lock().unwrap().info(
                        &format!(
                            "Started frontend on port {}",
                            if host.is_empty() {
                                port.to_string()
                            } else {
                                format!("{host}:{port}")
                            }
                        ),
                        "z2m",
                    );
                    tokio::spawn(async move {
                        loop {
                            tokio::select! {
                                _ = &mut shutdown_rx => break,
                                accepted = listener.accept() => {
                                    let Ok((stream, _)) = accepted else { continue };
                                    let clients = clients.clone();
                                    let this = this.clone();
                                    let api_path = api_path.clone();
                                    let auth_token = auth_token.clone();
                                    let server = server.clone();
                                    let tls = tls_acceptor.clone();
                                    let logger = this.context.logger.clone();
                                    tokio::spawn(async move {
                                        match tls {
                                            Some(acceptor) => match acceptor.accept(stream).await {
                                                Ok(tls_stream) => {
                                                    let _ = Self::serve(this, tls_stream, clients, api_path, auth_token, server).await;
                                                }
                                                Err(error) => logger.lock().unwrap().error(
                                                    &format!("TLS handshake failed: {error}"),
                                                    "z2m",
                                                ),
                                            },
                                            None => {
                                                let _ = Self::serve(this, stream, clients, api_path, auth_token, server).await;
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    })
                }
            };

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_mqtt_message_published(
                self.context.key,
                Arc::new(move |data| this.on_mqtt_published(data)),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_publish_entity_state(
                self.context.key,
                Arc::new(move |data| this.on_publish_entity_state(data)),
            );

            Ok(())
        })
    }

    fn stop(self: Arc<Self>) -> ExtensionFuture {
        Box::pin(async move {
            self.broadcast(&json!({"topic": "bridge/state", "payload": {"state": "offline"}}));
            self.clients.lock().unwrap().clear();
            self.context
                .event_bus
                .lock()
                .unwrap()
                .remove_listeners(self.context.key);
            if let Some(shutdown) = self.shutdown.lock().unwrap().take() {
                let _ = shutdown.send(());
            }
            let task = self.task.lock().unwrap().take();
            if let Some(task) = task {
                let _ = task.await;
            }
            Ok(())
        })
    }
}

/// Upstream `hasSSL` + `createSecureServer` (`frontend.ts:56-108`).
fn build_tls_acceptor(
    settings: &Value,
    logger: &Arc<Mutex<crate::util::logger::Logger>>,
) -> Result<Option<TlsAcceptor>, String> {
    use std::io::BufReader;

    let key = settings["frontend"]["ssl_key"].as_str().filter(|value| !value.is_empty());
    let cert = settings["frontend"]["ssl_cert"].as_str().filter(|value| !value.is_empty());

    let has = |value: Option<&str>, name: &str| -> bool {
        let Some(path) = value else {
            return false;
        };
        if std::path::Path::new(path).exists() {
            return true;
        }
        logger.lock().unwrap().error(
            &format!("Defined {name} '{path}' file path does not exists, server won't be secured."),
            "z2m",
        );
        false
    };

    let (key, cert) = (has(key, "ssl_key"), has(cert, "ssl_cert"));
    if !key || !cert {
        return Ok(None);
    }
    let key = settings["frontend"]["ssl_key"].as_str().unwrap();
    let cert = settings["frontend"]["ssl_cert"].as_str().unwrap();

    let certs = rustls_pemfile::certs(&mut BufReader::new(
        std::fs::File::open(cert).map_err(|error| error.to_string())?,
    ))
    .collect::<Result<Vec<_>, _>>()
    .map_err(|error| error.to_string())?;
    let private_key = rustls_pemfile::private_key(&mut BufReader::new(
        std::fs::File::open(key).map_err(|error| error.to_string())?,
    ))
    .map_err(|error| error.to_string())?
    .ok_or_else(|| "No private key found in ssl_key".to_string())?;

    let provider = Arc::new(rustls_rustcrypto::provider());
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|error| error.to_string())?
        .with_no_client_auth()
        .with_single_cert(certs, private_key)
        .map_err(|error| error.to_string())?;

    Ok(Some(TlsAcceptor::from(Arc::new(config))))
}
