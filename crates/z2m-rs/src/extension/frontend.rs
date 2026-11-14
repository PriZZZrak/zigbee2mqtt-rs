// Original: .upstream/zigbee2mqtt/lib/extension/frontend.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// Protocol/API scope (AGENTS §5 Q5): WebSocket protocol + auth + retained/state
// replay. Static assets and TLS are deferred: the Windfront package is not
// shipped (OpenWrt packaging), and TLS would require a C crypto provider,
// which AGENTS §6 forbids. Non-API HTTP requests receive 404 from the
// WebSocket handshake.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{accept_hdr_async, WebSocketStream};

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
            mqtt.retained_messages
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
        stream: S,
        clients: Clients,
        api_path: String,
        auth_token: Option<String>,
    ) -> Result<(), String>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let callback =
            move |request: &Request, response: Response| -> Result<Response, ErrorResponse> {
                if request.uri().path() != api_path {
                    let mut error = ErrorResponse::new(Some("Not Found".to_string()));
                    *error.status_mut() = StatusCode::NOT_FOUND;
                    return Err(error);
                }
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

        let socket = accept_hdr_async(stream, callback)
            .await
            .map_err(|error| error.to_string())?;
        let id = this.next_id.fetch_add(1, Ordering::SeqCst);
        let (sender, receiver) = mpsc::unbounded_channel::<Message>();
        clients.lock().unwrap().insert(id, sender.clone());
        this.replay(&sender).await;
        Self::handle_socket(&this, socket, id, receiver).await;
        Ok(())
    }
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
            if settings["frontend"]["ssl_key"].as_str().is_some()
                || settings["frontend"]["ssl_cert"].as_str().is_some()
            {
                return Err(
                    "Frontend TLS is not available: a pure-Rust build cannot use a C crypto provider (AGENTS §6)"
                        .to_string(),
                );
            }
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
                                    tokio::spawn(async move {
                                        let _ = Self::serve(this, stream, clients, api_path, auth_token).await;
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
                                    tokio::spawn(async move {
                                        let _ = Self::serve(this, stream, clients, api_path, auth_token).await;
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
                                    tokio::spawn(async move {
                                        let _ = Self::serve(this, stream, clients, api_path, auth_token).await;
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
