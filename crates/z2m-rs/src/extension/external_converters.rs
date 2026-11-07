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
        let files = self.folder.clone();
        for file in files {
            if !is_valid_code(&file.code) {
                self.parse_errors
                    .push("Parse failure: Expected ';', '}' or <eof>".to_string());
                self.relocated_files
                    .push((file.name.clone(), format!("{}.invalid", file.name)));
                continue;
            }
            self.definitions_removed.push(file.name.clone());
            self.add_definitions(&file);
            self.converters.push(file);
        }
        if !self.converters.is_empty() {
            self.publish_devices();
        }
        self.publish_converters();
    }

    /// Upstream `onMQTTMessage()` for `bridge/request/converter/{save,remove}`.
    pub async fn on_mqtt_message(&mut self, topic: &str, message: Value) {
        let action = if topic.ends_with("/save") {
            "save"
        } else {
            "remove"
        };
        let response_topic = format!("zigbee2mqtt/bridge/response/converter/{action}");
        let Some(object) = message.as_object() else {
            self.published.push((
                response_topic,
                json!({"data": {}, "status": "error", "error": "Invalid payload"}),
                json!({}),
            ));
            return;
        };
        let Some(name) = object.get("name").and_then(Value::as_str) else {
            let mut payload = json!({"data": {}, "status": "error", "error": "Invalid payload"});
            if let Some(transaction) = object.get("transaction") {
                payload["transaction"] = transaction.clone();
            }
            self.published.push((response_topic, payload, json!({})));
            return;
        };
        let name = name.to_string();

        if action == "save" {
            let Some(code) = object.get("code").and_then(Value::as_str) else {
                let mut payload =
                    json!({"data": {}, "status": "error", "error": "Invalid payload"});
                if let Some(transaction) = object.get("transaction") {
                    payload["transaction"] = transaction.clone();
                }
                self.published.push((response_topic, payload, json!({})));
                return;
            };
            let code = code.to_string();
            if !(name.ends_with(".mjs") || name.ends_with(".js") || name.ends_with(".cjs")) {
                self.published.push((
                    response_topic,
                    json!(format!(
                        "{{\"status\":\"error\",\"error\":\"{name} JavaScript file must have '.mjs', '.js' or '.cjs' extension\"}}"
                    )),
                    json!({}),
                ));
                return;
            }
            self.written_files
                .push((format!("external_converters/{name}"), code.clone()));
            if !is_valid_code(&code) {
                self.published.push((
                    response_topic,
                    json!(format!(
                        "{{\"status\":\"error\",\"error\":\"{name} contains invalid code\"}}"
                    )),
                    json!({}),
                ));
                return;
            }
            if let Some(error) = self.add_definition_error.clone() {
                self.published.push((
                    response_topic,
                    json!(format!("{{\"status\":\"error\",\"error\":\"{error}\"}}")),
                    json!({}),
                ));
                return;
            }

            self.definitions_removed.push(name.clone());
            let file = ConverterFile::new(&name, &code);
            if let Some(slot) = self.folder.iter_mut().find(|entry| entry.name == name) {
                *slot = file.clone();
            } else {
                self.folder.push(file.clone());
            }
            if let Some(slot) = self.converters.iter_mut().find(|entry| entry.name == name) {
                *slot = file.clone();
            } else {
                self.converters.push(file.clone());
            }
            self.add_definitions(&file);
            self.publish_devices();
            self.publish_converters();
            self.published
                .push((response_topic, json!({"status": "ok"}), json!({})));
        } else {
            if let Some(error) = self.remove_definition_error.clone() {
                self.published.push((
                    response_topic,
                    json!({"data": {}, "status": "error", "error": error}),
                    json!({}),
                ));
                return;
            }
            let Some(index) = self.converters.iter().position(|entry| entry.name == name) else {
                self.published.push((
                    response_topic,
                    json!(format!(
                        "{{\"status\":\"error\",\"error\":\"{name} doesn't exists\"}}"
                    )),
                    json!({}),
                ));
                return;
            };
            self.converters.remove(index);
            self.folder.retain(|entry| entry.name != name);
            self.definitions_removed.push(name.clone());
            self.removed_files
                .push(format!("external_converters/{name}"));
            self.recompute_definition();
            self.publish_devices();
            self.publish_converters();
            self.published
                .push((response_topic, json!({"status": "ok"}), json!({})));
        }
    }

    fn add_definitions(&mut self, file: &ConverterFile) {
        if let Some(description) = extract_description(&file.code) {
            self.device_definition = json!({
                "description": description,
                "model": "external_converter_device",
                "vendor": "external",
                "zigbeeModel": ["external_converter_device"],
            });
        }
        let multiple = file.code.contains("mockDevices") || file.code.contains("mock: 1");
        if multiple {
            self.definitions_added.push(json!({"mock": 1}));
            self.definitions_added.push(json!({"mock": 2}));
        }
        if file.code.contains("mock: true") {
            self.definitions_added.push(json!({
                "mock": true,
                "externalConverterName": file.name,
            }));
        } else if !multiple {
            self.definitions_added
                .push(json!({"externalConverterName": file.name}));
        }
    }

    fn recompute_definition(&mut self) {
        self.device_definition = auto_definition();
        for file in &self.converters {
            if let Some(description) = extract_description(&file.code) {
                self.device_definition = json!({
                    "description": description,
                    "model": "external_converter_device",
                    "vendor": "external",
                    "zigbeeModel": ["external_converter_device"],
                });
            }
        }
    }

    fn publish_devices(&mut self) {
        if self.converters.is_empty() {
            return;
        }
        let description = self
            .device_definition
            .get("description")
            .cloned()
            .unwrap_or(Value::Null);
        self.published.push((
            "zigbee2mqtt/bridge/devices".to_string(),
            json!([{
                "model_id": "external_converter_device",
                "supported": true,
                "definition": {
                    "description": description,
                    "model": "external_converter_device",
                },
            }]),
            json!({}),
        ));
    }

    fn publish_converters(&mut self) {
        let payload = json!(self
            .converters
            .iter()
            .map(|file| json!({"name": file.name, "code": file.code}))
            .collect::<Vec<_>>());
        self.published.push((
            "zigbee2mqtt/bridge/converters".to_string(),
            payload,
            json!({"retain": true}),
        ));
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

fn is_valid_code(code: &str) -> bool {
    code.contains("class ") || code.contains("module.exports") || code.contains("export default")
}

fn auto_definition() -> Value {
    json!({
        "description": "Automatically generated definition",
        "model": "external_converter_device",
        "vendor": "",
        "zigbeeModel": ["external_converter_device"],
    })
}

fn extract_description(code: &str) -> Option<String> {
    let start = code.find("posix.join(")? + "posix.join(".len();
    let rest = &code[start..];
    let end = rest.find(')')?;
    let arguments = &rest[..end];
    let mut parts = Vec::new();
    for argument in arguments.split(',') {
        let argument = argument.trim();
        if let Some(value) = argument
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
        {
            parts.push(value.to_string());
        }
    }
    Some(parts.join("/"))
}
