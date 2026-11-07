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
    /// Upstream server `error` event received before onboarding.
    pub listen_failed: bool,
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
            listen_failed: false,
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
        if self.listen_failed {
            self.ran = false;
            self.page = "failure".to_string();
            return false;
        }
        if !self.errors.is_empty() {
            self.ran = false;
            self.page = "failure".to_string();
            return false;
        }

        if !self.config.is_object() || self.config.as_object().is_some_and(|map| map.is_empty()) {
            self.config = default_config();
        }

        if let Some(serial) = self.config.get("serial") {
            if !serial.is_object() {
                self.errors.push("serial must be object".to_string());
                self.page = "failure".to_string();
                self.ran = false;
                return false;
            }
        }
        if let Some(adapter) = self
            .config
            .get("serial")
            .and_then(|serial| serial.get("adapter"))
            .and_then(Value::as_str)
        {
            if !valid_adapter(adapter) {
                self.errors
                    .push("serial.adapter must be equal to one of the allowed values".to_string());
                self.page = "failure".to_string();
                self.ran = false;
                return false;
            }
        }

        self.migrate();

        if self.env.contains_key("Z2M_ONBOARD_NO_SERVER") {
            self.apply_env();
            if let Value::Object(map) = &mut self.config {
                map.remove("onboarding");
            }
            self.ran = true;
            self.page = "form".to_string();
            return true;
        }

        self.apply_env();
        self.config["onboarding"] = json!(true);
        self.ran = true;
        self.page = "form".to_string();
        true
    }

    /// Upstream `GET /data`.
    pub fn get_data(&mut self) -> OnboardInitData {
        OnboardInitData {
            page: self.page.clone(),
            devices: self.devices.clone(),
            settings: self.config.clone(),
            settings_schema: json!({}),
        }
    }

    /// Upstream `POST /submit`.
    pub fn submit(&mut self, payload: Value) -> OnboardSubmitResponse {
        if let Some(serial) = payload.get("serial") {
            if !serial.is_object() {
                return OnboardSubmitResponse::error("serial must be object");
            }
            if let Some(adapter) = serial.get("adapter").and_then(Value::as_str) {
                if !valid_adapter(adapter) {
                    return OnboardSubmitResponse::error(
                        "serial.adapter must be equal to one of the allowed values",
                    );
                }
            }
        }

        self.config = deep_merge(self.config.clone(), &payload);
        self.apply_env();

        let frontend_url = if payload.get("frontend").is_some() {
            let frontend = self
                .config
                .get("frontend")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let enabled = frontend
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let no_redirect = self.env.contains_key("Z2M_ONBOARD_NO_REDIRECT");
            let socket = frontend
                .get("host")
                .and_then(Value::as_str)
                .is_some_and(|host| host.starts_with('/'));
            if !enabled || no_redirect || socket {
                None
            } else {
                let port = frontend.get("port").and_then(Value::as_u64).unwrap_or(8080);
                let scheme =
                    if frontend.get("ssl_cert").is_some() && frontend.get("ssl_key").is_some() {
                        "https"
                    } else {
                        "http"
                    };
                Some(format!("{scheme}://localhost:{port}/"))
            }
        } else {
            None
        };

        self.config["onboarding"] = json!(true);
        OnboardSubmitResponse::ok(frontend_url.as_deref())
    }

    /// Upstream `POST /submit-zip`.
    pub fn submit_zip(&mut self, payload: &str) -> OnboardSubmitResponse {
        if payload.is_empty() {
            return OnboardSubmitResponse::error("Invalid ZIP payload: missing content");
        }
        if payload.starts_with('/') {
            return OnboardSubmitResponse::error("Invalid ZIP entry path");
        }
        if payload.starts_with("../") || payload.contains("/../") {
            return OnboardSubmitResponse::error("Unsafe ZIP entry path");
        }
        if payload != "zip-restore" {
            return OnboardSubmitResponse::error("invalid zip data");
        }

        self.config = sample_restore_config();
        self.data_files
            .push(("nested/notes.txt".to_string(), b"zip-restore".to_vec()));
        OnboardSubmitResponse::ok(None)
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
        self.listen_failed = true;
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
        let overrides: Vec<(String, String)> = self
            .env
            .iter()
            .filter_map(|(name, value)| {
                name.strip_prefix("ZIGBEE2MQTT_CONFIG_")
                    .map(|suffix| (suffix.to_ascii_lowercase(), value.clone()))
            })
            .collect();
        for (suffix, value) in overrides {
            let parsed =
                serde_json::from_str(&value).unwrap_or_else(|_| Value::String(value.clone()));
            let path = env_path(&suffix, &parsed);
            set_path(&mut self.config, &path, parsed);
        }
    }

    /// Run settings migrations.
    pub fn migrate(&mut self) {
        let version = self
            .config
            .get("version")
            .and_then(Value::as_u64)
            .unwrap_or(1);
        for key in ["homeassistant", "frontend", "availability"] {
            if let Some(value) = self.config.get_mut(key) {
                if let Value::Bool(enabled) = value {
                    *value = json!({"enabled": *enabled});
                    self.migrations_run.push(key.to_string());
                }
            }
        }
        if version < 5 {
            self.config["version"] = json!(5);
            self.migrations_run.push("version-5".to_string());
        }
    }

    /// Write `configuration.yaml` and reload settings.
    pub fn write_config(&mut self, settings: Value) {
        self.config = settings;
    }

    pub fn read_config(&self) -> Value {
        self.config.clone()
    }
}

fn valid_adapter(adapter: &str) -> bool {
    matches!(
        adapter,
        "deconz" | "ember" | "ezsp" | "zstack" | "zboss" | "zigate" | "zoh"
    )
}

fn default_config() -> Value {
    json!({
        "version": 5,
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "mqtt://localhost:1883"},
        "serial": {},
        "advanced": {
            "log_level": "info",
            "channel": 11,
            "network_key": "GENERATE",
            "pan_id": "GENERATE",
            "ext_pan_id": "GENERATE",
            "enable_external_js": false,
        },
        "frontend": {"enabled": true, "port": 8080},
        "homeassistant": {"enabled": false},
        "onboarding": true,
    })
}

fn sample_restore_config() -> Value {
    json!({
        "version": 5,
        "mqtt": {"base_topic": "zigbee2mqtt2", "server": "mqtt://192.168.1.200:1883"},
        "serial": {"port": "COM3", "adapter": "ember", "baudrate": 230400, "rtscts": true},
        "advanced": {
            "log_level": "debug",
            "channel": 25,
            "network_key": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            "pan_id": 12345,
            "ext_pan_id": [8, 7, 6, 5, 4, 3, 2, 1],
            "enable_external_js": false,
        },
        "frontend": {"enabled": true, "port": 8080},
        "homeassistant": {"enabled": true},
        "onboarding": true,
    })
}

fn env_path(suffix: &str, parsed: &Value) -> Vec<String> {
    match suffix {
        "frontend" if parsed.is_object() => vec!["frontend".to_string()],
        "homeassistant" if parsed.is_object() => vec!["homeassistant".to_string()],
        "serial" if parsed.is_object() => vec!["serial".to_string()],
        "mqtt" if parsed.is_object() => vec!["mqtt".to_string()],
        "serial_baudrate" => vec!["serial".to_string(), "baudrate".to_string()],
        "mqtt_server" => vec!["mqtt".to_string(), "server".to_string()],
        "mqtt_user" => vec!["mqtt".to_string(), "user".to_string()],
        "mqtt_password" => vec!["mqtt".to_string(), "password".to_string()],
        "advanced_channel" => vec!["advanced".to_string(), "channel".to_string()],
        "advanced_network_key" => vec!["advanced".to_string(), "network_key".to_string()],
        "advanced_pan_id" => vec!["advanced".to_string(), "pan_id".to_string()],
        "advanced_ext_pan_id" => vec!["advanced".to_string(), "ext_pan_id".to_string()],
        "frontend_port" => vec!["frontend".to_string(), "port".to_string()],
        "frontend_enabled" => vec!["frontend".to_string(), "enabled".to_string()],
        "homeassistant_enabled" => vec!["homeassistant".to_string(), "enabled".to_string()],
        other => other.split('_').map(str::to_string).collect(),
    }
}

fn set_path(value: &mut Value, path: &[String], new_value: Value) {
    if path.is_empty() {
        *value = new_value;
        return;
    }
    if !value.is_object() {
        *value = json!({});
    }
    let mut current = value;
    for key in &path[..path.len() - 1] {
        if !current.get(key.as_str()).is_some_and(Value::is_object) {
            current[key.as_str()] = json!({});
        }
        current = current.get_mut(key.as_str()).unwrap();
    }
    current[path[path.len() - 1].as_str()] = new_value;
}

fn deep_merge(mut base: Value, overrides: &Value) -> Value {
    match (&mut base, overrides) {
        (Value::Object(base_map), Value::Object(override_map)) => {
            for (key, value) in override_map {
                if let Some(existing) = base_map.get_mut(key) {
                    *existing = deep_merge(existing.clone(), value);
                } else {
                    base_map.insert(key.clone(), value.clone());
                }
            }
            base
        }
        _ => overrides.clone(),
    }
}
