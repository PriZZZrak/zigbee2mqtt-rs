// Original: .upstream/zigbee2mqtt/lib/util/onboarding.ts
// TODO: port HTTP server, validation, ZIP extraction and migrations.

use std::collections::HashMap;

use serde_json::{json, Value};

/// Upstream `OnboardInitData`.
#[derive(Debug, Clone, PartialEq)]
pub struct OnboardInitData {
    pub page: String,
    pub devices: Vec<Value>,
    pub settings: Value,
    /// Upstream camelCase: `settingsSchema`.
    pub settings_schema: Value,
}

/// Upstream `OnboardSubmitResponse`.
#[derive(Debug, Clone, PartialEq)]
pub struct OnboardSubmitResponse {
    pub success: bool,
    /// Upstream camelCase: `frontendUrl`.
    pub frontend_url: Option<String>,
    pub error: Option<String>,
}

impl OnboardSubmitResponse {
    pub fn ok(frontend_url: Option<&str>) -> Self {
        Self {
            success: true,
            frontend_url: frontend_url.map(str::to_string),
            error: None,
        }
    }

    pub fn error(error: &str) -> Self {
        Self {
            success: false,
            frontend_url: None,
            error: Some(error.to_string()),
        }
    }
}

/// Upstream `OnboardFailureData`.
#[derive(Debug, Clone, PartialEq)]
pub struct OnboardFailureData {
    pub errors: Vec<String>,
}

/// Model of `lib/util/onboarding.ts` used by the ported tests.
#[derive(Debug)]
pub struct Onboarding {
    /// Current configuration (upstream `settings`).
    pub config: Value,
    /// Environment variables (`Z2M_ONBOARD_*` / `ZIGBEE2MQTT_CONFIG_*`).
    pub env: HashMap<String, String>,
    /// Result of `findAllDevices()`.
    pub devices: Vec<Value>,
    /// `form` or `failure`.
    pub page: String,
    pub errors: Vec<String>,
    /// Redirect URL returned by `/submit` (null when disabled/socket/SSL rules).
    pub frontend_url: Option<String>,
    /// Extracted files: `(path, bytes)`.
    pub data_files: Vec<(String, Vec<u8>)>,
    /// Requests handed to the static file server.
    pub static_requests: Vec<String>,
    /// `/submit` and `/submit-zip` responses in order.
    pub submit_responses: Vec<OnboardSubmitResponse>,
    /// `onboard()` result.
    pub ran: bool,
    /// Migrations applied (upstream settings version bumps).
    pub migrations_run: Vec<String>,
}

impl Default for Onboarding {
    fn default() -> Self {
        Self {
            config: json!({}),
            env: HashMap::new(),
            devices: Vec::new(),
            page: "form".to_string(),
            errors: Vec::new(),
            frontend_url: None,
            data_files: Vec::new(),
            static_requests: Vec::new(),
            submit_responses: Vec::new(),
            ran: false,
            migrations_run: Vec::new(),
        }
    }
}

impl Onboarding {
    pub fn new(config: Value) -> Self {
        Self {
            config,
            ..Self::default()
        }
    }

    /// Upstream `onboard()`: true when onboarding finished successfully.
    pub async fn onboard(&mut self) -> bool {
        todo!()
    }

    /// Upstream `GET /data`.
    pub fn get_data(&mut self) -> OnboardInitData {
        todo!()
    }

    /// Upstream `POST /submit`.
    pub fn submit(&mut self, _payload: Value) -> OnboardSubmitResponse {
        todo!()
    }

    /// Upstream `POST /submit-zip`.
    pub fn submit_zip(&mut self, _payload: &str) -> OnboardSubmitResponse {
        todo!()
    }

    /// Upstream `/submit` request-stream error handler.
    pub fn submit_stream_error(&mut self, error: &str) -> OnboardSubmitResponse {
        OnboardSubmitResponse::error(error)
    }

    /// Upstream `/submit-zip` request-stream error handler.
    pub fn submit_zip_stream_error(&mut self, error: &str) -> OnboardSubmitResponse {
        OnboardSubmitResponse::error(error)
    }

    /// Upstream server `error` event: onboarding fails.
    pub fn listen_error(&mut self, _error: &str) {
        self.ran = false;
    }

    /// Upstream unknown-route handling (`staticFileServer`).
    pub fn request_unknown_route(&mut self, url: &str) {
        self.static_requests.push(url.to_string());
    }

    /// Upstream failure page data.
    pub fn run_failure(&mut self) -> OnboardFailureData {
        OnboardFailureData {
            errors: self.errors.clone(),
        }
    }

    /// Apply `ZIGBEE2MQTT_CONFIG_*` environment overrides.
    pub fn apply_env(&mut self) {
        todo!()
    }

    /// Run settings migrations.
    pub fn migrate(&mut self) {
        todo!()
    }

    /// Write `configuration.yaml` and reload settings.
    pub fn write_config(&mut self, _settings: Value) {
        todo!()
    }

    pub fn read_config(&self) -> Value {
        self.config.clone()
    }
}
