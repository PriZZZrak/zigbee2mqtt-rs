// Original: .upstream/zigbee2mqtt/lib/extension/externalExtensions.ts
// TODO: port loading, saving and extension lifecycle.

use serde_json::{json, Value};

/// One extension file (`name` + JS/MJS `code`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionFile {
    pub name: String,
    pub code: String,
}

impl ExtensionFile {
    pub fn new(name: &str, code: &str) -> Self {
        Self {
            name: name.to_string(),
            code: code.to_string(),
        }
    }
}

/// Model of `lib/extension/externalExtensions.ts` used by the ported tests.
#[derive(Debug)]
pub struct ExternalExtensions {
    /// Upstream `advanced.enable_external_js`.
    pub enabled: bool,
    /// Extension files found in `external_extensions/`.
    pub folder: Vec<ExtensionFile>,
    /// Files successfully loaded (upstream `this.extensions`).
    pub extensions: Vec<ExtensionFile>,
    /// `mockMQTTPublishAsync` calls: `(topic, payload, options)`.
    pub published: Vec<(String, Value, Value)>,
    /// `fs.writeFileSync(path, code, "utf8")` calls.
    pub written_files: Vec<(String, String)>,
    /// `fs.rmSync(path, {force:true})` calls.
    pub removed_files: Vec<String>,
    /// Parse errors logged while loading files.
    pub parse_errors: Vec<String>,
    /// `(from, to)` files relocated on parse failure.
    pub relocated_files: Vec<(String, String)>,
    /// Names of started external extensions (upstream `start()` called once).
    pub started: Vec<String>,
}

impl Default for ExternalExtensions {
    fn default() -> Self {
        Self {
            enabled: true,
            folder: Vec::new(),
            extensions: Vec::new(),
            published: Vec::new(),
            written_files: Vec::new(),
            removed_files: Vec::new(),
            parse_errors: Vec::new(),
            relocated_files: Vec::new(),
            started: Vec::new(),
        }
    }
}

impl ExternalExtensions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Upstream extension `start()`: load files from `external_extensions/`.
    pub async fn start(&mut self) {
        todo!()
    }

    /// Upstream `onMQTTMessage()` for `bridge/request/extension/{save,remove}`.
    pub async fn on_mqtt_message(&mut self, _topic: &str, _message: Value) {
        todo!()
    }

    /// Upstream controller only adds this extension when external JS is enabled.
    pub fn extension_enabled(&self) -> bool {
        self.enabled
    }

    /// Payloads published on `zigbee2mqtt/bridge/extensions`.
    pub fn published_extensions(&self) -> Vec<Value> {
        self.published
            .iter()
            .filter(|(topic, _, _)| topic == "zigbee2mqtt/bridge/extensions")
            .map(|(_, payload, _)| payload.clone())
            .collect()
    }

    pub fn has_publish(&self, topic: &str, payload: Value) -> bool {
        self.published
            .iter()
            .any(|(published_topic, published_payload, _)| {
                published_topic == topic && *published_payload == payload
            })
    }

    pub fn extension_payload(&self) -> Value {
        json!(self
            .extensions
            .iter()
            .map(|extension| json!({"name": extension.name, "code": extension.code}))
            .collect::<Vec<_>>())
    }
}
