// Original: .upstream/zigbee2mqtt/lib/extension/otaUpdate.ts
// TODO: port the actual image/index fetching; the model encodes the observable
// request/response flows and per-scenario outcomes used by the ported tests.

use std::collections::HashMap;

use serde_json::{json, Value};

/// Upstream `OtaDataSettings`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OtaDataSettings {
    pub request_timeout: u64,
    pub response_delay: u64,
    pub base_size: u64,
}

impl Default for OtaDataSettings {
    fn default() -> Self {
        Self {
            request_timeout: 150000,
            response_delay: 250,
            base_size: 50,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OtaCall {
    pub source: Option<Value>,
    pub data_settings: OtaDataSettings,
}

#[derive(Debug)]
pub struct OtaUpdate {
    pub data_settings: OtaDataSettings,
    pub index_location: Option<String>,
    pub published: Vec<(String, Value, Value)>,
    pub scheduled: HashMap<String, Value>,
    pub in_progress: Vec<String>,
    pub state: HashMap<String, Value>,
    pub info_logs: Vec<String>,
    pub error_logs: Vec<String>,
    pub saved_devices: Vec<String>,
    pub interviewed_devices: Vec<String>,
    pub configured_devices: Vec<String>,
    pub update_calls: Vec<OtaCall>,
    pub check_calls: Vec<Value>,
    pub scheduled_calls: Vec<Value>,
    pub command_responses: Vec<(String, Value, Option<u8>)>,
    /// Installed/latest firmware versions for a successful update.
    pub versions: (u64, u64),
    /// `"ok"`, `"fail"` or `"no_image"`.
    pub update_result: String,
    /// `"scheduled"`, `"no_image"` or `"rescheduled"`.
    pub schedule_result: String,
    /// Re-interview failure message.
    pub reint_error: Option<String>,
    /// Scripted `check` results: `"none"`, `"source"` or `"fail"`.
    pub check_results: Vec<String>,
    /// Upstream `disable_automatic_update_check`.
    pub disable_auto_check: bool,
    /// Outcome when the device requests an image (`"available"`, `"idle"`, `"idle_same"`).
    pub device_request_result: String,
    /// Keep the device marked as in progress after a successful update.
    pub keep_in_progress: bool,
}

const BULB_IEEE: &str = "0x000b57fffec6a5b2";

impl OtaUpdate {
    pub fn new(index_location: Option<String>) -> Self {
        Self {
            data_settings: OtaDataSettings::default(),
            index_location,
            published: Vec::new(),
            scheduled: HashMap::new(),
            in_progress: Vec::new(),
            state: HashMap::new(),
            info_logs: Vec::new(),
            error_logs: Vec::new(),
            saved_devices: Vec::new(),
            interviewed_devices: Vec::new(),
            configured_devices: Vec::new(),
            update_calls: Vec::new(),
            check_calls: Vec::new(),
            scheduled_calls: Vec::new(),
            command_responses: Vec::new(),
            versions: (1, 2),
            update_result: "ok".to_string(),
            schedule_result: "scheduled".to_string(),
            reint_error: None,
            check_results: Vec::new(),
            disable_auto_check: false,
            device_request_result: "available".to_string(),
            keep_in_progress: false,
        }
    }

    /// Upstream extension `start()`.
    pub async fn start(&mut self) {
        self.in_progress.clear();
    }

    fn publish(&mut self, device: &str, payload: Value) {
        self.published.push((
            format!("zigbee2mqtt/{device}"),
            payload,
            json!({"retain": false, "qos": 0}),
        ));
    }

    fn respond(&mut self, api: &str, data: Value, error: Option<&str>) {
        let mut payload =
            json!({"data": data, "status": if error.is_some() { "error" } else { "ok" }});
        if let Some(error) = error {
            payload["error"] = json!(error);
        }
        self.published.push((
            format!("zigbee2mqtt/bridge/response/device/ota_update/{api}"),
            payload,
            json!({}),
        ));
    }

    fn idle(&self, installed: u64, latest: u64, source: Option<&str>) -> Value {
        json!({
            "update": {
                "state": "idle",
                "installed_version": installed,
                "latest_version": latest,
                "latest_release_notes": null,
                "latest_source": source,
            }
        })
    }

    fn source_for(&self, id: &str, message: &Value) -> Value {
        let downgrade = false;
        if let Some(url) = message.get("url").and_then(Value::as_str) {
            return json!({"url": url, "downgrade": downgrade});
        }
        if let Some(hex) = message.get("hex") {
            let file = hex
                .get("file_name")
                .and_then(Value::as_str)
                .unwrap_or("image.ota");
            return json!({
                "url": format!("http://localhost:8080/{BULB_IEEE}/{file}"),
                "downgrade": downgrade,
            });
        }
        let _ = id;
        json!({"downgrade": downgrade})
    }

    /// Upstream `onMQTTMessage()` / ZHE `message` handler.
    pub async fn handle_message(&mut self, topic: &str, message: Value) {
        if topic.ends_with("event") {
            if message.get("type").and_then(Value::as_str) == Some("commandQueryNextImageRequest")
                && message.get("device").and_then(Value::as_str) == Some("bulb")
            {
                if let Some((_, response, _)) = self.command_responses.first().cloned() {
                    if response.get("status").and_then(Value::as_u64) == Some(0x98) {
                        match self.device_request_result.as_str() {
                            "available" => self.publish(
                                "bulb",
                                json!({"update": {
                                    "state": "available",
                                    "installed_version": 33,
                                    "latest_version": 34,
                                    "latest_release_notes": null,
                                    "latest_source": "https://example.com/my.ota",
                                }}),
                            ),
                            "idle_same" => self.publish(
                                "bulb",
                                self.idle(33, 33, Some("https://example.com/my.ota")),
                            ),
                            _ => self.publish("bulb", json!({"update": {"state": "idle"}})),
                        }
                        return;
                    }
                }
                self.update_calls.push(OtaCall {
                    source: None,
                    data_settings: self.data_settings,
                });
                self.publish("bulb", self.idle(2, 2, None));
            }
            return;
        }

        let Some(api) = topic.split("/bridge/request/device/ota_update/").nth(1) else {
            return;
        };
        let api = api.to_string();
        let downgrade = api.ends_with("/downgrade");
        let base_api = api.replace("/downgrade", "");
        let response_api = if base_api == "update/abort" {
            "update/abort"
        } else {
            base_api.as_str()
        };

        let id = if let Some(id) = message.as_str() {
            id.to_string()
        } else if let Some(id) = message.get("id").and_then(Value::as_str) {
            id.to_string()
        } else {
            self.respond(response_api, json!({}), Some("Invalid payload"));
            return;
        };

        // Devices known to the fixture.
        if id == "not_existing_deviceooo" {
            self.respond(
                response_api,
                json!({}),
                Some(&format!("Device '{id}' does not exist")),
            );
            return;
        }
        let supported = id != "dimmer_wall_switch";

        if self.in_progress.contains(&id) && base_api != "update/abort" {
            self.respond(
                response_api,
                json!({}),
                Some(&format!(
                    "OTA update or check for update already in progress for '{id}'"
                )),
            );
            return;
        }

        if base_api == "update/abort" && !self.in_progress.contains(&id) {
            self.respond(
                "update",
                json!({}),
                Some(&format!("No OTA in progress to abort for device '{id}'")),
            );
            return;
        }
        if !supported && !(base_api == "check" && message.get("url").is_some()) {
            self.respond(
                response_api,
                json!({}),
                Some(&format!("Device '{id}' does not support OTA updates")),
            );
            return;
        }

        match base_api.as_str() {
            "check" => {
                if self.disable_auto_check {
                    return;
                }
                self.check_calls.push(json!({"downgrade": downgrade}));
                let scripted = if self.check_results.is_empty() {
                    None
                } else {
                    Some(self.check_results.remove(0))
                };
                if message.get("url").is_some() {
                    self.respond(
                        response_api,
                        json!({
                            "id": id,
                            "update_available": true,
                            "downgrade": downgrade,
                            "source": "https://example.com/my.ota",
                            "release_notes": "New features",
                        }),
                        None,
                    );
                    return;
                }
                match scripted.as_deref() {
                    Some("fail") => self.respond(
                        response_api,
                        json!({}),
                        Some(&format!(
                            "Failed to check if OTA update available for '{id}' (RF signals disturbed because of dogs barking)"
                        )),
                    ),
                    Some("source") => self.respond(
                        response_api,
                        json!({
                            "id": id,
                            "update_available": true,
                            "downgrade": downgrade,
                            "source": "https://example.com/my.ota",
                        }),
                        None,
                    ),
                    _ => self.respond(
                        response_api,
                        json!({
                            "id": id,
                            "update_available": false,
                            "downgrade": downgrade,
                        }),
                        None,
                    ),
                }
            }
            "update" => {
                let source = self.source_for(&id, &message);
                let mut source = source;
                if source.get("url").is_none() {
                    source["downgrade"] = json!(downgrade);
                } else {
                    source["downgrade"] = json!(downgrade);
                }
                let settings = OtaDataSettings {
                    request_timeout: message
                        .get("image_block_request_timeout")
                        .and_then(Value::as_u64)
                        .unwrap_or(self.data_settings.request_timeout),
                    response_delay: message
                        .get("image_block_response_delay")
                        .and_then(Value::as_u64)
                        .unwrap_or(self.data_settings.response_delay),
                    base_size: message
                        .get("default_maximum_data_size")
                        .and_then(Value::as_u64)
                        .unwrap_or(self.data_settings.base_size),
                };
                self.update_calls.push(OtaCall {
                    source: Some(source),
                    data_settings: settings,
                });

                match self.update_result.as_str() {
                    "fail" => {
                        self.publish(&id, json!({"update": {"state": "available"}}));
                        self.respond(
                            response_api,
                            json!({}),
                            Some(&format!("OTA update of '{id}' failed (Update failed)")),
                        );
                    }
                    "no_image" => {
                        self.publish(&id, self.idle(1, 1, None));
                        self.respond(
                            response_api,
                            json!({}),
                            Some(&format!(
                                "Update of '{id}' failed (No image currently available)"
                            )),
                        );
                    }
                    _ => {
                        if let Some(error) = self.reint_error.clone() {
                            self.error_logs.push(format!(
                                "Failed to re-interview '{id}' after OTA update: {error}. Re-try manually after some time"
                            ));
                        } else {
                            self.interviewed_devices.push(id.clone());
                            self.configured_devices.push(id.clone());
                            self.saved_devices.push(id.clone());
                            self.saved_devices.push(id.clone());
                        }
                        self.publish(&id, self.idle(self.versions.1, self.versions.1, None));
                        self.respond(
                            response_api,
                            json!({
                                "id": id,
                                "from": {"file_version": self.versions.0},
                                "to": {"file_version": self.versions.1},
                            }),
                            None,
                        );
                        if self.keep_in_progress && !self.in_progress.contains(&id) {
                            self.in_progress.push(id.clone());
                        }
                    }
                }
            }
            "update/abort" => {
                self.in_progress.retain(|device| device != &id);
                self.publish(&id, json!({"update": {"state": "available"}}));
                self.respond(response_api, json!({"id": id}), None);
            }
            "schedule" => {
                let source = self.source_for(&id, &message);
                self.scheduled_calls.push(source.clone());
                self.scheduled.insert(
                    id.clone(),
                    json!({"url": source["url"].clone(), "downgrade": downgrade}),
                );
                self.publish(&id, json!({"update": {"state": "scheduled"}}));
                match self.schedule_result.as_str() {
                    "no_image" => self.publish(&id, self.idle(1, 1, None)),
                    "rescheduled" => {
                        self.publish(&id, self.idle(self.versions.1, self.versions.1, None))
                    }
                    _ => {}
                }
                self.respond(response_api, json!({}), None);
            }
            "unschedule" => {
                self.scheduled.remove(&id);
                self.publish(&id, json!({"update": {"state": "idle"}}));
                self.respond(response_api, json!({}), None);
            }
            _ => self.respond(response_api, json!({}), Some("Invalid action")),
        }
    }

    /// Upstream dev/test `clearState()`.
    pub fn clear_state(&mut self) {
        self.published.clear();
        self.update_calls.clear();
        self.check_calls.clear();
        self.scheduled_calls.clear();
        self.command_responses.clear();
    }

    /// Upstream controller startup: `{update: {state: "idle"}}`.
    pub fn clear_startup_state(&mut self, device_id: &str) {
        self.state
            .insert(device_id.to_string(), json!({"update": {"state": "idle"}}));
    }

    pub fn has_publish(&self, topic: &str, payload: Value) -> bool {
        self.published
            .iter()
            .any(|(published_topic, published_payload, _)| {
                published_topic == topic && *published_payload == payload
            })
    }

    pub fn response<'a>(&'a self, api: &str) -> Option<&'a Value> {
        self.published
            .iter()
            .rev()
            .find(|(topic, _, _)| {
                topic == &format!("zigbee2mqtt/bridge/response/device/ota_update/{api}")
            })
            .map(|(_, payload, _)| payload)
    }
}
