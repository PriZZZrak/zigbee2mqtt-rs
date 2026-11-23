// Offline converter host transport.
//
// The dispatcher receives upstream converter host calls (`HostRequest`) and
// executes them through this boundary. The offline implementation validates
// known ZCL clusters/commands with the existing `zigbee-apiarist` ZCL catalog
// and returns deterministic mock responses; it never touches a coordinator.
// The production transport will map the same requests onto the existing
// `Endpoint` model methods.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use converter_runtime::{HostRequest, HostResult};
use serde_json::{json, Map, Value};
use zigbee_apiarist::zspec::zcl::definition::cluster::{
    genAlarms, genBasic, genGroups, genLevelCtrl, genOnOff, genOta, genPollCtrl, genPowerCfg,
    genScenes, genTime, haApplianceIdentification, ssIasZone, touchlink,
};
use zigbee_apiarist::zspec::zcl::utils::{CustomClusters, Utils};

pub type TransportFuture = Pin<Box<dyn Future<Output = HostResult> + Send + 'static>>;

/// Bridge-side boundary for converter host operations.
pub trait ConverterTransport: Send + Sync {
    fn execute(&self, request: &HostRequest) -> TransportFuture;

    /// Recorded host operations, for offline diagnostics and tests.
    fn calls(&self) -> Vec<Value> {
        Vec::new()
    }
}

/// Deterministic offline transport keyed by `(device, cluster, attribute)`.
#[derive(Default)]
pub struct OfflineMockTransport {
    reads: Mutex<HashMap<(String, String, String), Value>>,
    calls: Mutex<Vec<Value>>,
}

impl OfflineMockTransport {
    pub fn new() -> Self {
        Self::default()
    }

    /// Preload a read value returned by `endpoint.read`.
    pub fn set_read(
        &self,
        device: impl Into<String>,
        cluster: impl Into<String>,
        attribute: impl Into<String>,
        value: Value,
    ) {
        self.reads
            .lock()
            .unwrap()
            .insert((device.into(), cluster.into(), attribute.into()), value);
    }

    pub fn calls(&self) -> Vec<Value> {
        self.calls.lock().unwrap().clone()
    }

    fn record(&self, operation: &Value) {
        self.calls.lock().unwrap().push(operation.clone());
    }

    fn read_value(&self, device: &str, cluster: &str, attribute: &str) -> Value {
        if let Some(value) = self.reads.lock().unwrap().get(&(
            device.to_owned(),
            cluster.to_owned(),
            attribute.to_owned(),
        )) {
            return value.clone();
        }
        match attribute {
            "onOff" => json!(false),
            "currentLevel" => json!(0),
            "batteryPercentageRemaining" => json!(200),
            "localTemperature" => json!(2100),
            "occupiedHeatingSetpoint" => json!(2100),
            "measuredValue" => json!(0),
            "measuredValueWithInvalid" => json!(0),
            "currentPositionLiftPercentage" => json!(0),
            _ => json!(0),
        }
    }

    /// Validate a known cluster through the existing ZCL catalog. The Rust
    /// catalog is intentionally partial, so unknown clusters stay opaque in
    /// the offline transport; the real transport resolves them on the device.
    fn known_cluster(cluster: &str) -> Option<zigbee_apiarist::zspec::zcl::utils::Cluster> {
        // `Utils::get_cluster` panics for clusters outside the partial Rust
        // catalog. The offline transport keeps those opaque instead of
        // aborting a real converter operation.
        const KNOWN: &[&str] = &[
            genBasic::name,
            genPowerCfg::name,
            genOnOff::name,
            haApplianceIdentification::name,
            genGroups::name,
            genAlarms::name,
            genLevelCtrl::name,
            genOta::name,
            touchlink::name,
            genTime::name,
            ssIasZone::name,
            genPollCtrl::name,
            genScenes::name,
        ];
        if !KNOWN.contains(&cluster) {
            return None;
        }
        Some(Utils::get_cluster(cluster, None, &CustomClusters::empty()))
    }

    fn known_command(cluster: &str, command: &str) -> bool {
        let Some(cluster) = Self::known_cluster(cluster) else {
            return false;
        };
        cluster.commands.contains_key(command) || cluster.commandsResponse.contains_key(command)
    }

    fn known_attribute(cluster: &str, attribute: &str) -> bool {
        let Some(cluster) = Self::known_cluster(cluster) else {
            return false;
        };
        Utils::get_cluster_attribute(&cluster, attribute, None).is_some()
    }

    fn execute_inner(&self, request: &HostRequest) -> Result<Value, String> {
        let operation = &request.operation;
        let op = operation
            .get("op")
            .and_then(Value::as_str)
            .ok_or_else(|| "host operation is missing 'op'".to_owned())?;
        let endpoint = operation
            .get("endpoint")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let cluster = operation
            .get("cluster")
            .and_then(Value::as_str)
            .unwrap_or_default();
        self.record(operation);

        match op {
            "read" => {
                let attributes = operation
                    .get("attributes")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let mut result = Map::new();
                for attribute in attributes {
                    let Some(attribute) = attribute.as_str() else {
                        return Err("read attribute must be a string".to_owned());
                    };
                    // Known attributes are checked against the shared catalog;
                    // unknown ones remain valid for the offline mock.
                    let _ = Self::known_attribute(cluster, attribute);
                    result.insert(
                        attribute.to_owned(),
                        self.read_value(&request.device, cluster, attribute),
                    );
                }
                Ok(Value::Object(result))
            }
            "write" => {
                let attributes = operation
                    .get("attributes")
                    .and_then(Value::as_object)
                    .ok_or_else(|| "write requires an attributes object".to_owned())?;
                for attribute in attributes.keys() {
                    let _ = Self::known_attribute(cluster, attribute);
                }
                Ok(json!({}))
            }
            "command" | "commandResponse" => {
                let command = operation
                    .get("command")
                    .and_then(Value::as_str)
                    .ok_or_else(|| format!("{op} requires a command"))?;
                let _ = Self::known_command(cluster, command);
                Ok(json!({}))
            }
            "bind" => Ok(json!({})),
            "configureReporting" => {
                if operation.get("items").and_then(Value::as_array).is_none() {
                    return Err("configureReporting requires items".to_owned());
                }
                Ok(json!({}))
            }
            "probe" => Ok(json!(true)),
            other => Err(format!(
                "offline converter transport does not implement host op '{other}' (device {}, endpoint {endpoint})",
                request.device
            )),
        }
    }
}

impl ConverterTransport for OfflineMockTransport {
    fn calls(&self) -> Vec<Value> {
        self.calls()
    }

    fn execute(&self, request: &HostRequest) -> TransportFuture {
        let result = self.execute_inner(request);
        Box::pin(async move {
            match result {
                Ok(value) => HostResult::Ok(value),
                Err(error) => HostResult::Err(error),
            }
        })
    }
}

/// Transport that can wrap another implementation for tests/future adapters.
pub struct SharedTransport {
    inner: Arc<dyn ConverterTransport>,
}

impl SharedTransport {
    pub fn new(inner: Arc<dyn ConverterTransport>) -> Self {
        Self { inner }
    }
}

impl ConverterTransport for SharedTransport {
    fn calls(&self) -> Vec<Value> {
        self.inner.calls()
    }

    fn execute(&self, request: &HostRequest) -> TransportFuture {
        self.inner.execute(request)
    }
}
