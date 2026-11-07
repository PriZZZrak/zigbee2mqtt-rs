// Original: .upstream/zigbee2mqtt/lib/extension/frontend.ts
// TODO: port the frontend extension.
//
// The Rust port records the node:http/node-static/ws observables that the
// upstream tests assert through mocks.

use std::collections::BTreeMap;

use serde_json::Value;

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

    /// Upstream: `controller.start()` starting the Frontend extension.
    pub async fn start(&mut self) {
        todo!()
    }

    /// Upstream: `controller.stop()`.
    pub async fn stop(&mut self) {
        todo!()
    }

    /// Upstream `http.on("upgrade", ...)`.
    pub fn on_upgrade(&mut self, url: &str, socket_id: u64) {
        todo!()
    }

    /// Upstream `http.on("request", ...)` routing to node-static / 404.
    pub fn on_request(&mut self, url: &str) {
        todo!()
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
        todo!()
    }

    /// Upstream client `message` event.
    pub fn on_message(&mut self, client_id: usize, message: Option<&str>, _is_binary: bool) {
        todo!()
    }

    /// Upstream client `error` event.
    pub fn on_error(&mut self, message: &str) {
        todo!()
    }

    /// Upstream MQTT message handler updating device states.
    pub fn on_mqtt_message(&mut self, topic: &str, payload: &Value) {
        todo!()
    }

    /// Upstream `deviceJoined` event published on `bridge/event`.
    pub fn on_device_joined(&mut self, ieee_address: &str, friendly_name: &str) {
        todo!()
    }
}

/// Upstream: `controller.enableDisableExtension(enable, "Frontend")` guard.
pub fn enable_disable_extension(settings_enabled: bool, enable: bool) -> Result<(), String> {
    todo!()
}
