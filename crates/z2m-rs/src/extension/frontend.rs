// Original: .upstream/zigbee2mqtt/lib/extension/frontend.ts
// TODO: port the frontend extension.
//
// The Rust port records the node:http/node-static/ws observables that the
// upstream tests assert through mocks.

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

    /// Upstream `WebSocket.Server` `connection` event.
    pub fn on_connection(&mut self, client_id: usize) {
        todo!()
    }

    /// Upstream client `message` event.
    pub fn on_message(&mut self, client_id: usize, message: &str, _is_binary: bool) {
        todo!()
    }

    /// Upstream client `error` event.
    pub fn on_error(&mut self, _message: &str) {
        todo!()
    }
}

/// Upstream: `controller.enableDisableExtension(enable, "Frontend")` guard.
pub fn enable_disable_extension(settings_enabled: bool, enable: bool) -> Result<(), String> {
    todo!()
}
