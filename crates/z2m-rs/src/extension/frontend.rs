// Original: .upstream/zigbee2mqtt/lib/extension/frontend.ts
// TODO: port the frontend extension.
//
// The Rust port records the server configuration/start-stop observables that
// the upstream tests assert through node:http/node-static mocks.

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListenTarget {
    Tcp { port: u16, host: Option<String> },
    Unix(String),
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
}

/// Upstream: `controller.enableDisableExtension(enable, "Frontend")` guard.
pub fn enable_disable_extension(settings_enabled: bool, enable: bool) -> Result<(), String> {
    todo!()
}
