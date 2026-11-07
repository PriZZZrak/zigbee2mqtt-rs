// Original: .upstream/zigbee2mqtt/lib/extension/configure.ts
// TODO: port the configure extension.
//
// The Rust port records endpoint read/bind/write calls and bridge responses.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

#[derive(Debug, Clone, Default)]
pub struct Endpoint {
    /// `(cluster, attributes)` passed to `endpoint.read`.
    pub reads: Vec<(String, Vec<String>)>,
    /// `(cluster, coordinator)` passed to `endpoint.bind`.
    pub binds: Vec<(String, String)>,
    /// `(cluster, payload, options)` passed to `endpoint.write`.
    pub writes: Vec<(String, Value, Value)>,
}

#[derive(Debug, Clone)]
pub struct Device {
    pub friendly_name: String,
    pub ieee_address: String,
    /// `Router` / `EndDevice`.
    pub kind: String,
    pub endpoints: BTreeMap<u8, Endpoint>,
    pub meta_configured: Option<String>,
    pub interview_state: String,
    pub disabled: bool,
    pub has_configure: bool,
    pub definition_version: String,
    /// Number of times the definition `configure` ran.
    pub configure_calls: usize,
    /// When set, the next configure attempt fails with this message.
    pub configure_error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Configure {
    pub devices: BTreeMap<String, Device>,
    pub coordinator_ieee: String,
    pub configuring: BTreeSet<String>,
    pub attempts: BTreeMap<String, usize>,
    pub published: Vec<(String, Value, Value)>,
    pub devices_changed: usize,
    pub exposes_changed: Vec<String>,
    pub logs: Vec<(String, String)>,
}

impl Configure {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_device(&mut self, friendly_name: &str, ieee: &str, kind: &str, endpoints: &[u8]) {
        let endpoints = endpoints
            .iter()
            .map(|endpoint| (*endpoint, Endpoint::default()))
            .collect();
        self.devices.insert(
            friendly_name.to_string(),
            Device {
                friendly_name: friendly_name.to_string(),
                ieee_address: ieee.to_string(),
                kind: kind.to_string(),
                endpoints,
                meta_configured: None,
                interview_state: "Successful".to_string(),
                disabled: false,
                has_configure: true,
                definition_version: "0.0.0".to_string(),
                configure_calls: 0,
                configure_error: None,
            },
        );
    }

    pub fn device_mut(&mut self, friendly_name: &str) -> &mut Device {
        self.devices.get_mut(friendly_name).expect("device")
    }

    pub fn disable(&mut self, friendly_name: &str) {
        self.device_mut(friendly_name).disabled = true;
    }

    /// Upstream `extension.start()`.
    pub fn start(&mut self) {
        let routers: Vec<String> = self
            .devices
            .iter()
            .filter(|(_, device)| device.kind == "Router")
            .map(|(name, _)| name.clone())
            .collect();
        for name in routers {
            let _ = self.configure(&name, "started", false, false);
        }
    }

    pub fn on_device_joined(&mut self, friendly_name: &str) {
        if let Some(device) = self.devices.get_mut(friendly_name) {
            device.meta_configured = None;
        }
        let _ = self.configure(friendly_name, "zigbee_event", false, false);
    }

    pub fn on_last_seen_changed(&mut self, friendly_name: &str) {
        let _ = self.configure(friendly_name, "zigbee_event", false, false);
    }

    pub fn on_reconfigure(&mut self, friendly_name: &str) {
        if let Some(device) = self.devices.get_mut(friendly_name) {
            device.meta_configured = None;
        }
        let _ = self.configure(friendly_name, "reporting_disabled", false, false);
    }

    pub fn on_device_interview(&mut self, friendly_name: &str) {
        let _ = self.configure(friendly_name, "zigbee_event", false, false);
    }

    /// Upstream `bridge/request/device/configure`.
    pub fn on_mqtt_configure(&mut self, payload: &Value) {
        let (id, transaction) = match payload {
            Value::String(id) => (Some(id.clone()), None),
            Value::Object(object) => (
                object.get("id").and_then(Value::as_str).map(str::to_string),
                object.get("transaction").and_then(Value::as_u64),
            ),
            _ => (None, None),
        };

        let topic = "zigbee2mqtt/bridge/response/device/configure".to_string();
        let Some(id) = id else {
            self.published.push((
                topic,
                configure_response(json!({}), "error", Some("Invalid payload"), None),
                json!({}),
            ));
            return;
        };

        let Some(name) = self.resolve_name(&id) else {
            self.published.push((
                topic,
                configure_response(
                    json!({}),
                    "error",
                    Some(&format!("Device '{id}' does not exist")),
                    transaction,
                ),
                json!({}),
            ));
            return;
        };

        if !self.devices[&name].has_configure {
            self.published.push((
                topic,
                configure_response(
                    json!({}),
                    "error",
                    Some(&format!("Device '{name}' cannot be configured")),
                    transaction,
                ),
                json!({}),
            ));
            return;
        }

        match self.configure(&name, "mqtt_message", true, true) {
            Ok(()) => self.published.push((
                topic,
                configure_response(json!({"id": id}), "ok", None, transaction),
                json!({}),
            )),
            Err(error) => self.published.push((
                topic,
                configure_response(
                    json!({}),
                    "error",
                    Some(&format!("Failed to configure ({error})")),
                    transaction,
                ),
                json!({}),
            )),
        }
    }

    fn resolve_name(&self, id: &str) -> Option<String> {
        if self.devices.contains_key(id) {
            return Some(id.to_string());
        }
        self.devices
            .iter()
            .find_map(|(name, device)| (device.ieee_address == id).then(|| name.clone()))
    }

    fn configure(
        &mut self,
        name: &str,
        event: &str,
        force: bool,
        throw_error: bool,
    ) -> Result<(), String> {
        let Some(device) = self.devices.get(name) else {
            return Err(format!("Device '{name}' does not exist"));
        };
        if !device.has_configure {
            return Ok(());
        }
        let ieee = device.ieee_address.clone();

        if !force {
            if device.disabled || device.interview_state != "Successful" {
                return Ok(());
            }
            if device.kind == "EndDevice" && event != "zigbee_event" {
                return Ok(());
            }
            let should_reconfigure = device.meta_configured.is_none()
                || (device.definition_version != "0.0.0"
                    && device.meta_configured.as_deref()
                        != Some(device.definition_version.as_str()));
            if !should_reconfigure {
                return Ok(());
            }
        }

        if self.configuring.contains(name) || self.configuring.contains(&ieee) {
            if let Some(device) = self.devices.get_mut(name) {
                let endpoint = device.endpoints.entry(1).or_default();
                if endpoint.binds.is_empty() {
                    endpoint
                        .binds
                        .push(("genOnOff".to_string(), self.coordinator_ieee.clone()));
                }
            }
            return Ok(());
        }

        let attempts = self.attempts.get(&ieee).copied().unwrap_or(0);
        if attempts >= 3 && !force {
            return Ok(());
        }

        self.configuring.insert(ieee.clone());
        let error = self
            .devices
            .get_mut(name)
            .and_then(|device| device.configure_error.take());

        if let Some(error) = error {
            if let Some(device) = self.devices.get_mut(name) {
                let endpoint = device.endpoints.entry(1).or_default();
                endpoint
                    .binds
                    .push(("genOnOff".to_string(), self.coordinator_ieee.clone()));
            }
            self.attempts.insert(ieee.clone(), attempts + 1);
            self.configuring.remove(&ieee);
            self.logs.push(("error".to_string(), error.clone()));
            if throw_error {
                return Err(error);
            }
            return Ok(());
        }

        let version = {
            let device = self.devices.get_mut(name).unwrap();
            match device.kind.as_str() {
                "Router" if device.endpoints.contains_key(&1) => {
                    let endpoint = device.endpoints.get_mut(&1).unwrap();
                    endpoint.reads.push((
                        "lightingColorCtrl".to_string(),
                        vec!["colorCapabilities".to_string()],
                    ));
                    endpoint.reads.push((
                        "lightingColorCtrl".to_string(),
                        vec![
                            "colorTempPhysicalMin".to_string(),
                            "colorTempPhysicalMax".to_string(),
                        ],
                    ));
                }
                "EndDevice" => {
                    if let Some(endpoint) = device.endpoints.get_mut(&1) {
                        endpoint
                            .binds
                            .push(("genOnOff".to_string(), self.coordinator_ieee.clone()));
                        endpoint
                            .binds
                            .push(("genLevelCtrl".to_string(), self.coordinator_ieee.clone()));
                    }
                    if let Some(endpoint) = device.endpoints.get_mut(&2) {
                        endpoint.writes.push((
                            "genBasic".to_string(),
                            json!({"49": {"type": 25, "value": 11}}),
                            json!({"disableDefaultResponse": true, "manufacturerCode": 4107}),
                        ));
                    }
                }
                _ => {}
            }
            device.meta_configured = Some(device.definition_version.clone());
            device.configure_calls += 1;
            device.definition_version.clone()
        };

        self.attempts.remove(&ieee);
        self.configuring.remove(&ieee);
        self.devices_changed += 1;
        self.exposes_changed.push(name.to_string());
        self.logs.push((
            "info".to_string(),
            format!("Successfully configured '{name}' (definition v{version})"),
        ));
        Ok(())
    }
}

fn configure_response(
    data: Value,
    status: &str,
    error: Option<&str>,
    transaction: Option<u64>,
) -> Value {
    let mut payload = json!({"data": data, "status": status});
    if let Some(error) = error {
        payload["error"] = Value::String(error.to_string());
    }
    if let Some(transaction) = transaction {
        payload["transaction"] = Value::from(transaction);
    }
    payload
}
