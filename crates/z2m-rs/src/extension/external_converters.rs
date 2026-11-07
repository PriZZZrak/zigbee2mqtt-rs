// Original: .upstream/zigbee2mqtt/lib/extension/externalConverters.ts
// TODO: port loading, saving and definition management.

use serde_json::{json, Value};

/// One converter file (`name` + JS/MJS `code`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConverterFile {
    pub name: String,
    pub code: String,
}

impl ConverterFile {
    pub fn new(name: &str, code: &str) -> Self {
        Self {
            name: name.to_string(),
            code: code.to_string(),
        }
    }
}

/// Model of `lib/extension/externalConverters.ts` used by the ported tests.
#[derive(Debug)]
pub struct ExternalConverters {
    /// Upstream `advanced.enable_external_js`.
    pub enabled: bool,
    /// Converter files found in `external_converters/`.
    pub folder: Vec<ConverterFile>,
    /// Files successfully loaded (upstream `this.converters`).
    pub converters: Vec<ConverterFile>,
    /// Definition of the external converter device.
    pub device_definition: Value,
    /// `mockMQTTPublishAsync` calls: `(topic, payload, options)`.
    pub published: Vec<(String, Value, Value)>,
    /// `zhc.addExternalDefinition` calls.
    pub definitions_added: Vec<Value>,
    /// `zhc.removeExternalDefinitions` calls.
    pub definitions_removed: Vec<String>,
    /// `fs.writeFileSync(path, code, "utf8")` calls.
    pub written_files: Vec<(String, String)>,
    /// `fs.rmSync(path, {force:true})` calls.
    pub removed_files: Vec<String>,
    /// Parse errors logged while loading files.
    pub parse_errors: Vec<String>,
    /// `(from, to)` files relocated on parse failure.
    pub relocated_files: Vec<(String, String)>,
    /// Injected `addExternalDefinition` failure.
    pub add_definition_error: Option<String>,
    /// Injected `removeExternalDefinitions` failure.
    pub remove_definition_error: Option<String>,
}

impl Default for ExternalConverters {
    fn default() -> Self {
        Self {
            enabled: true,
            folder: Vec::new(),
            converters: Vec::new(),
            device_definition: json!({
                "description": "Automatically generated definition",
                "model": "external_converter_device",
                "vendor": "",
                "zigbeeModel": ["external_converter_device"],
            }),
            published: Vec::new(),
            definitions_added: Vec::new(),
            definitions_removed: Vec::new(),
            written_files: Vec::new(),
            removed_files: Vec::new(),
            parse_errors: Vec::new(),
            relocated_files: Vec::new(),
            add_definition_error: None,
            remove_definition_error: None,
        }
    }
}

impl ExternalConverters {
    pub fn new() -> Self {
        Self::default()
    }

    /// Upstream extension `start()`: load files from `external_converters/`.
    pub async fn start(&mut self) {
        todo!()
    }

    /// Upstream `onMQTTMessage()` for `bridge/request/converter/{save,remove}`.
    pub async fn on_mqtt_message(&mut self, _topic: &str, _message: Value) {
        todo!()
    }

    /// Upstream controller only adds this extension when external JS is enabled.
    pub fn extension_enabled(&self) -> bool {
        self.enabled
    }

    pub fn device_definition(&self) -> Value {
        self.device_definition.clone()
    }

    /// Payloads published on `zigbee2mqtt/bridge/converters`.
    pub fn published_converters(&self) -> Vec<Value> {
        self.published
            .iter()
            .filter(|(topic, _, _)| topic == "zigbee2mqtt/bridge/converters")
            .map(|(_, payload, _)| payload.clone())
            .collect()
    }
}
