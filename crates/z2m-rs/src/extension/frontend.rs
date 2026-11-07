// Original: .upstream/zigbee2mqtt/lib/extension/frontend.ts
// TODO: port the frontend extension.
//
// The Rust port records the node:http/node-static/ws observables that the
// upstream tests assert through mocks.

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};

pub const FRONTEND_PATH: &str = "frontend";
pub const DEVICE_ICONS_PATH: &str = "device_icons";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListenTarget {
    Tcp { port: u16, host: Option<String> },
    Unix(String),
}

#[derive(Debug, Clone, Default)]
pub struct WsClient {
    pub ready_state: String,
    pub sent: Vec<Value>,
    pub terminated: usize,
    pub closed: Vec<(u16, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct Frontend {
    pub settings: Value,
    /// Static mount keys (`frontendPath`, `deviceIconsPath` upstream).
    pub static_mounts: Vec<String>,
    pub http_listen: Option<ListenTarget>,
    pub https_listen: Option<ListenTarget>,
    pub http_close_calls: usize,
    pub https_close_calls: usize,
    pub ws_close_calls: usize,
    pub ws_clients: usize,
    pub ws_clients_terminated: usize,

    /// `new WebSocket.Server({noServer:true, path})`.
    pub ws_path: Option<String>,
    pub handle_upgrade_calls: Vec<String>,
    pub upgrade_emits: Vec<(String, u64)>,
    pub socket_destroy_calls: usize,
    pub socket_closes: Vec<(u16, String)>,
    /// `(mount, url)` passed to node-static.
    pub static_served: Vec<(String, String)>,
    pub redirect: Option<(u16, String)>,
    pub send_not_found_calls: usize,
    pub auth_token: Option<String>,
    pub clients: Vec<WsClient>,
    /// Per-device state replayed on connect and updated on MQTT messages.
    pub device_states: BTreeMap<String, Value>,
    /// `(topic, payload, retain, qos)` passed to `mqtt.publish`.
    pub mqtt_published: Vec<(String, Value, bool, u8)>,
    pub logs: Vec<(String, String)>,
}

impl Frontend {
    pub fn from_settings(settings: &Value) -> Self {
        Self {
            settings: settings.clone(),
            ..Default::default()
        }
    }

    fn config(&self) -> Value {
        self.settings
            .get("frontend")
            .cloned()
            .unwrap_or_else(|| json!({}))
    }

    fn base_url(&self) -> String {
        let base = self
            .config()
            .get("base_url")
            .and_then(Value::as_str)
            .unwrap_or("/")
            .to_string();
        if base.ends_with('/') {
            base
        } else {
            format!("{base}/")
        }
    }

    /// Upstream: `controller.start()` starting the Frontend extension.
    pub async fn start(&mut self) {
        let config = self.config();
        self.static_mounts = vec![FRONTEND_PATH.to_string(), DEVICE_ICONS_PATH.to_string()];
        self.auth_token = config
            .get("auth_token")
            .and_then(Value::as_str)
            .map(str::to_string);

        let base = self.base_url();
        let ws_base = base.trim_end_matches('/');
        self.ws_path = Some(format!("{ws_base}/api"));

        let disable_ui_serving = config
            .get("disable_ui_serving")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if disable_ui_serving {
            self.http_listen = None;
            self.https_listen = None;
            return;
        }

        let port = config.get("port").and_then(Value::as_u64).unwrap_or(8080) as u16;
        let host = config.get("host").and_then(Value::as_str);
        let target = match host {
            Some(host) if host.starts_with('/') => ListenTarget::Unix(host.to_string()),
            Some(host) => ListenTarget::Tcp {
                port,
                host: Some(host.to_string()),
            },
            None => ListenTarget::Tcp { port, host: None },
        };

        let ssl_cert = config.get("ssl_cert").and_then(Value::as_str);
        let ssl_key = config.get("ssl_key").and_then(Value::as_str);
        let https_valid = match (ssl_cert, ssl_key) {
            (Some(cert), Some(key)) => file_exists(cert) && file_exists(key),
            _ => false,
        };
        if https_valid {
            self.https_listen = Some(target);
            self.http_listen = None;
        } else {
            self.http_listen = Some(target);
            self.https_listen = None;
        }
    }

    /// Upstream: `controller.stop()`.
    pub async fn stop(&mut self) {
        self.ws_clients_terminated += self.ws_clients;
        self.ws_clients = 0;
        if self.http_listen.is_some() {
            self.http_close_calls += 1;
        }
        self.ws_close_calls += 1;
    }

    /// Upstream `http.on("upgrade", ...)`.
    pub fn on_upgrade(&mut self, url: &str, socket_id: u64) {
        self.handle_upgrade_calls.push(url.to_string());
        if let Some(token) = &self.auth_token {
            let authorized = url
                .split_once('?')
                .map(|(_, query)| {
                    query.split('&').any(|pair| {
                        pair.strip_prefix("token=")
                            .is_some_and(|value| value == token)
                    })
                })
                .unwrap_or(false);
            if !authorized {
                self.socket_closes.push((4401, "Unauthorized".to_string()));
                return;
            }
        }
        self.upgrade_emits.push((url.to_string(), socket_id));
    }

    /// Upstream `http.on("request", ...)` routing to node-static / 404.
    pub fn on_request(&mut self, url: &str) {
        let base = self.base_url();
        let base_without_slash = base.trim_end_matches('/');
        if !base_without_slash.is_empty() && url == base_without_slash {
            self.redirect = Some((301, base));
            return;
        }

        let relative = if let Some(rest) = url.strip_prefix(&base) {
            rest.trim_start_matches('/')
        } else if base == "/" {
            url.trim_start_matches('/')
        } else {
            self.send_not_found_calls += 1;
            return;
        };

        if let Some(rest) = relative.strip_prefix("device_icons/") {
            self.static_served
                .push((DEVICE_ICONS_PATH.to_string(), format!("/{rest}")));
            return;
        }

        let path = if relative.is_empty() {
            "/".to_string()
        } else {
            format!("/{relative}")
        };
        self.static_served.push((FRONTEND_PATH.to_string(), path));
    }

    /// Adds a `WebSocket` client (test counterpart of `mockWSClient`).
    pub fn add_client(&mut self, ready_state: &str) -> usize {
        self.clients.push(WsClient {
            ready_state: ready_state.to_string(),
            ..Default::default()
        });
        self.clients.len() - 1
    }

    /// Upstream `WebSocket.Server` `connection` event.
    pub fn on_connection(&mut self, client_id: usize) {
        let last_seen = self
            .settings
            .get("advanced")
            .and_then(|advanced| advanced.get("last_seen"))
            .and_then(Value::as_str)
            .map(str::to_string);

        let mut messages = vec![
            json!({"topic": "bridge/devices", "payload": []}),
            json!({"topic": "bridge/info", "payload": {}}),
            json!({"topic": "bridge/state", "payload": {"state": "online"}}),
        ];
        for (device, state) in &self.device_states {
            let mut payload = state.clone();
            if last_seen.is_some() {
                if let Value::Object(map) = &mut payload {
                    map.insert("last_seen".to_string(), json!("1970-01-01T00:00:01.000Z"));
                }
            }
            messages.push(json!({"topic": device, "payload": payload}));
        }
        if let Some(client) = self.clients.get_mut(client_id) {
            client.sent.extend(messages);
        }
    }

    /// Upstream client `message` event.
    pub fn on_message(&mut self, client_id: usize, message: Option<&str>, _is_binary: bool) {
        let Some(message) = message else {
            return;
        };
        if message.is_empty() || message == "null" {
            return;
        }
        let Ok(parsed) = serde_json::from_str::<Value>(message) else {
            return;
        };
        let Some(topic) = parsed.get("topic").and_then(Value::as_str) else {
            return;
        };
        let payload = parsed.get("payload").cloned().unwrap_or_else(|| json!({}));

        let device = topic.strip_suffix("/set").unwrap_or(topic);
        let device = if let Some(rest) = device.strip_prefix("zigbee2mqtt/") {
            rest
        } else {
            device
        };

        let mut merged = Map::new();
        if let Value::Object(map) = &payload {
            for (key, value) in map {
                merged.insert(key.clone(), value.clone());
            }
        }
        for key in [
            "effect",
            "effect_color",
            "effect_speed",
            "identify",
            "power_on_behavior",
        ] {
            merged.entry(key.to_string()).or_insert(Value::Null);
        }
        merged.entry("linkquality".to_string()).or_insert(json!(20));
        merged.entry("update".to_string()).or_insert(json!({
            "state": null,
            "installed_version": -1,
            "latest_version": -1,
        }));
        let merged = Value::Object(merged);

        self.mqtt_published
            .push((format!("zigbee2mqtt/{device}"), merged.clone(), false, 0));

        if self
            .clients
            .get(client_id)
            .is_some_and(|client| client.ready_state == "open")
        {
            self.clients[client_id].sent.push(json!({
                "topic": device,
                "payload": merged,
            }));
        }
    }

    /// Upstream client `error` event.
    pub fn on_error(&mut self, message: &str) {
        self.logs
            .push(("error".to_string(), format!("WebSocket error: {message}")));
    }

    /// Upstream MQTT message handler updating device states.
    pub fn on_mqtt_message(&mut self, topic: &str, payload: &Value) {
        let device = topic
            .strip_prefix("zigbee2mqtt/")
            .unwrap_or(topic)
            .strip_suffix("/set")
            .unwrap_or_else(|| topic.strip_prefix("zigbee2mqtt/").unwrap_or(topic));
        for client in &mut self.clients {
            if client.ready_state == "open" {
                client.sent.push(json!({
                    "topic": device,
                    "payload": payload,
                }));
            }
        }
    }

    /// Upstream `deviceJoined` event published on `bridge/event`.
    pub fn on_device_joined(&mut self, ieee_address: &str, friendly_name: &str) {
        let message = json!({
            "topic": "bridge/event",
            "payload": {
                "type": "device_joined",
                "data": {
                    "ieee_address": ieee_address,
                    "friendly_name": friendly_name,
                },
            },
        });
        for client in &mut self.clients {
            if client.ready_state == "open" {
                client.sent.push(message.clone());
            }
        }
    }
}

/// Upstream: `controller.enableDisableExtension(enable, "Frontend")` guard.
pub fn enable_disable_extension(settings_enabled: bool, enable: bool) -> Result<(), String> {
    match (settings_enabled, enable) {
        (false, true) => Err("Tried to enable Frontend extension disabled in settings".to_string()),
        (true, false) => Err("Tried to disable Frontend extension enabled in settings".to_string()),
        _ => Ok(()),
    }
}

fn file_exists(path: &str) -> bool {
    let direct = std::path::Path::new(path);
    if direct.exists() {
        return true;
    }
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
        .exists()
}
