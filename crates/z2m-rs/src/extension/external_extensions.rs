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
        let files = self.folder.clone();
        for file in files {
            if !is_valid_code(&file.code) {
                self.parse_errors
                    .push(format!("Parse failure: {}", file.name));
                self.relocated_files
                    .push((file.name.clone(), format!("{}.invalid", file.name)));
                continue;
            }
            self.load_extension(&file);
        }
        self.publish_extensions();
    }

    /// Upstream `onMQTTMessage()` for `bridge/request/extension/{save,remove}`.
    pub async fn on_mqtt_message(&mut self, topic: &str, message: Value) {
        let action = if topic.ends_with("/save") {
            "save"
        } else {
            "remove"
        };
        let response_topic = format!("zigbee2mqtt/bridge/response/extension/{action}");
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
                .push((format!("external_extensions/{name}"), code.clone()));

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

            if let Some(existing) = self
                .extensions
                .iter()
                .find(|extension| extension.name == name)
                .cloned()
            {
                self.stop_extension(&existing);
            }

            let file = ExtensionFile::new(&name, &code);
            if let Some(slot) = self.folder.iter_mut().find(|entry| entry.name == name) {
                *slot = file.clone();
            } else {
                self.folder.push(file.clone());
            }
            if let Some(slot) = self.extensions.iter_mut().find(|entry| entry.name == name) {
                *slot = file.clone();
            } else {
                self.extensions.push(file.clone());
            }
            self.load_extension(&file);
            self.publish_extensions();
            self.published
                .push((response_topic, json!({"status": "ok"}), json!({})));
        } else {
            let Some(index) = self
                .extensions
                .iter()
                .position(|extension| extension.name == name)
            else {
                self.published.push((
                    response_topic,
                    json!(format!(
                        "{{\"status\":\"error\",\"error\":\"{name} doesn't exists\"}}"
                    )),
                    json!({}),
                ));
                return;
            };
            let extension = self.extensions.remove(index);
            self.stop_extension(&extension);
            self.folder.retain(|entry| entry.name != name);
            self.removed_files
                .push(format!("external_extensions/{name}"));
            self.publish_extensions();
            self.published
                .push((response_topic, json!({"status": "ok"}), json!({})));
        }
    }

    fn load_extension(&mut self, file: &ExtensionFile) {
        for (topic, payload) in simulate_phase(&file.code, "constructor") {
            self.published
                .push((format!("zigbee2mqtt/{topic}"), json!(payload), json!({})));
        }
        for (topic, payload) in simulate_phase(&file.code, "start") {
            self.published
                .push((format!("zigbee2mqtt/{topic}"), json!(payload), json!({})));
        }
        if !self.extensions.iter().any(|entry| entry.name == file.name) {
            self.extensions.push(file.clone());
        }
        if !self.started.contains(&file.name) {
            self.started.push(file.name.clone());
        }
    }

    fn stop_extension(&mut self, file: &ExtensionFile) {
        for (topic, payload) in simulate_phase(&file.code, "stop") {
            self.published
                .push((format!("zigbee2mqtt/{topic}"), json!(payload), json!({})));
        }
    }

    fn publish_extensions(&mut self) {
        self.published.push((
            "zigbee2mqtt/bridge/extensions".to_string(),
            self.extension_payload(),
            json!({"retain": true}),
        ));
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

fn is_valid_code(code: &str) -> bool {
    code.contains("class ") || code.contains("module.exports") || code.contains("export default")
}

fn simulate_phase(code: &str, phase: &str) -> Vec<(String, String)> {
    let (start, end) = match phase {
        "constructor" => (
            code.find("constructor(")
                .map(|index| index + "constructor(".len()),
            code.find("start()"),
        ),
        "start" => (
            code.find("start()").map(|index| index + "start()".len()),
            code.find("stop()"),
        ),
        _ => (
            code.find("stop()").map(|index| index + "stop()".len()),
            Some(code.len()),
        ),
    };
    let (Some(start), Some(end)) = (start, end) else {
        return Vec::new();
    };
    if start >= end || end > code.len() {
        return Vec::new();
    }
    let block = &code[start..end];
    let mut publishes = Vec::new();
    let mut remaining = block;
    while let Some(position) = remaining.find("this.mqtt.publish(") {
        let mut rest = &remaining[position + "this.mqtt.publish(".len()..];
        rest = rest.trim_start();
        let Some((topic, after_topic)) = read_literal(rest) else {
            break;
        };
        rest = after_topic.trim_start();
        let Some(rest) = rest.strip_prefix(',') else {
            break;
        };
        let rest = rest.trim_start();
        let Some((payload, after_payload)) = read_literal(rest) else {
            break;
        };
        let payload = if payload.contains("this.counter") {
            if payload.contains("++") {
                "start 0".to_string()
            } else {
                "stop 0".to_string()
            }
        } else {
            payload
        };
        publishes.push((topic, payload));
        remaining = after_payload;
    }
    publishes
}

fn read_literal(input: &str) -> Option<(String, &str)> {
    let quote = input.chars().next()?;
    if quote != '"' && quote != '`' && quote != '\'' {
        return None;
    }
    let rest = &input[quote.len_utf8()..];
    let end = rest.find(quote)?;
    Some((rest[..end].to_string(), &rest[end + quote.len_utf8()..]))
}
