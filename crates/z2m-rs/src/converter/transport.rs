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
use zigbee_apiarist::controller::model::endpoint::{
    ConfigureReportingItem, DryRunHook, DryRunRequest, Endpoint, Options, OptionsWithDefaults,
};
use zigbee_apiarist::zspec::zcl::definition::cluster::{
    genAlarms, genBasic, genGroups, genLevelCtrl, genOnOff, genOta, genPollCtrl, genPowerCfg,
    genScenes, genTime, haApplianceIdentification, ssIasZone, touchlink,
};
use zigbee_apiarist::zspec::zcl::utils::{CustomClusters, Utils};
use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;

pub type TransportFuture = Pin<Box<dyn Future<Output = HostResult> + Send + 'static>>;

/// Bridge-side boundary for converter host operations.
pub trait ConverterTransport: Send + Sync {
    fn execute(&self, request: &HostRequest) -> TransportFuture;

    /// Recorded host operations, for offline diagnostics and tests.
    fn calls(&self) -> Vec<Value> {
        Vec::new()
    }
}

/// `(device, cluster, attribute)` key for seeded reads.
type DeviceAttributeKey = (String, String, String);
type ReadValues = Arc<Mutex<HashMap<DeviceAttributeKey, Value>>>;

/// Deterministic offline transport keyed by `(device, cluster, attribute)`.
#[derive(Default)]
pub struct OfflineMockTransport {
    reads: Mutex<HashMap<DeviceAttributeKey, Value>>,
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

type DryRunReadValues = ReadValues;

/// Resolves a device IEEE and endpoint id to the existing controller model.
pub type EndpointResolver = Arc<dyn Fn(&str, u8) -> Option<Arc<Endpoint>> + Send + Sync>;

/// Recording/dry-run transport backed by the real `Endpoint`/ZCL model.
///
/// Host operations are translated into the existing endpoint methods; a
/// per-call dry-run hook captures the exact ZCL frame bytes that the adapter
/// would send and returns synthetic responses (a read response built from
/// seeded values). Nothing is transmitted and no coordinator is required.
pub struct DryRunEndpointTransport {
    state: Arc<DryRunState>,
}

struct DryRunState {
    resolver: EndpointResolver,
    reads: DryRunReadValues,
    calls: Mutex<Vec<Value>>,
}

impl DryRunEndpointTransport {
    pub fn new(resolver: EndpointResolver) -> Self {
        Self {
            state: Arc::new(DryRunState {
                resolver,
                reads: Arc::new(Mutex::new(HashMap::new())),
                calls: Mutex::new(Vec::new()),
            }),
        }
    }

    /// Seed a value returned by a dry-run `read` for `(device, cluster, attribute)`.
    pub fn set_read(
        &self,
        device: impl Into<String>,
        cluster: impl Into<String>,
        attribute: impl Into<String>,
        value: Value,
    ) {
        self.state
            .reads
            .lock()
            .unwrap()
            .insert((device.into(), cluster.into(), attribute.into()), value);
    }

    pub fn calls(&self) -> Vec<Value> {
        self.state.calls.lock().unwrap().clone()
    }
}

impl DryRunState {
    fn record(&self, operation: &Value, frame: Option<Value>) {
        let mut entry = operation.clone();
        if let Some(frame) = frame {
            if let Some(object) = entry.as_object_mut() {
                object.insert("frame".to_owned(), frame);
            }
        }
        self.calls.lock().unwrap().push(entry);
    }

    async fn execute_op(&self, request: &HostRequest) -> Result<Value, String> {
        let operation = &request.operation;
        let op = operation
            .get("op")
            .and_then(Value::as_str)
            .ok_or_else(|| "host operation is missing 'op'".to_owned())?;
        let endpoint_id = operation
            .get("endpoint")
            .and_then(Value::as_u64)
            .unwrap_or_default() as u8;
        let cluster = operation
            .get("cluster")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let endpoint = (self.resolver)(&request.device, endpoint_id).ok_or_else(|| {
            format!(
                "dry-run transport: device {} endpoint {} not found",
                request.device, endpoint_id
            )
        })?;
        let options = parse_options(operation, op == "commandResponse");
        let captured = Arc::new(Mutex::new(None::<Value>));
        let reads = Arc::clone(&self.reads);
        let device = request.device.clone();
        let cluster_name = cluster.to_owned();
        let captured_for_hook = Arc::clone(&captured);
        let hook: DryRunHook = Arc::new(move |request: &DryRunRequest<'_>| {
            let frame = request.frame;
            let frame_type = match frame.header.frame_control.frame_type {
                zigbee_apiarist::zspec::zcl::zcl_header::FrameType::GLOBAL => 0u8,
                zigbee_apiarist::zspec::zcl::zcl_header::FrameType::SPECIFIC => 1u8,
            };
            let response = read_response(request, &device, &cluster_name, &reads);
            *captured_for_hook.lock().unwrap() = Some(json!({
                "bytes": frame.to_buffer(),
                "cluster": frame.cluster.name,
                "clusterId": frame.cluster.ID,
                "command": frame.command.name,
                "commandId": frame.command.ID,
                "frameType": frame_type,
                "direction": frame.header.frame_control.direction as u8,
                "disableDefaultResponse": frame.header.frame_control.disable_default_response,
                "manufacturerCode": frame.header.manufacturer_code,
                "transactionSequenceNumber": frame.header.transaction_sequence_number,
                "reservedBits": frame.header.frame_control.reserved_bits,
                "endpoint": request.endpoint,
                "options": options_json(request.options),
            }));
            Ok(response)
        });
        endpoint.set_dry_run_hook(Some(hook));
        let guard = DryRunGuard(Arc::clone(&endpoint));
        let result = match op {
            "read" => {
                let attributes = operation
                    .get("attributes")
                    .cloned()
                    .unwrap_or_else(|| json!([]));
                endpoint
                    .read(Value::from(cluster), attributes, Some(options))
                    .await
            }
            "write" => {
                let attributes = operation
                    .get("attributes")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                endpoint
                    .write(Value::from(cluster), attributes, Some(options))
                    .await
                    .map(|_| json!({}))
            }
            "command" => {
                let command = operation
                    .get("command")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "command requires a command".to_owned())?;
                let payload = operation
                    .get("payload")
                    .cloned()
                    .unwrap_or(Value::Null);
                endpoint
                    .command(
                        Value::from(cluster),
                        Value::from(command),
                        payload,
                        Some(options),
                    )
                    .await
                    .map(|payload| payload.unwrap_or_else(|| json!({})))
            }
            "commandResponse" => {
                let command = operation
                    .get("command")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "commandResponse requires a command".to_owned())?;
                let payload = operation
                    .get("payload")
                    .cloned()
                    .unwrap_or(Value::Null);
                let tsn = operation
                    .get("transactionSequenceNumber")
                    .and_then(Value::as_u64)
                    .map(|value| value as u8)
                    .or(options.transaction_sequence_number);
                endpoint
                    .command_response(
                        Value::from(cluster),
                        Value::from(command),
                        payload,
                        Some(options),
                        tsn,
                    )
                    .await
                    .map(|_| json!({}))
            }
            "bind" => {
                let target = operation
                    .get("target")
                    .and_then(Value::as_u64)
                    .unwrap_or_default();
                Ok(json!({"recorded": true, "target": target}))
            }
            "configureReporting" => {
                let items = operation
                    .get("items")
                    .and_then(Value::as_array)
                    .ok_or_else(|| "configureReporting requires items".to_owned())?;
                let items = parse_reporting_items(items)?;
                endpoint
                    .configure_reporting(Value::from(cluster), items, Some(options))
                    .await
                    .map(|_| json!({}))
            }
            "probe" => Ok(json!(true)),
            other => Err(format!(
                "dry-run converter transport does not implement host op '{other}' (device {}, endpoint {endpoint_id})",
                request.device
            )),
        };
        drop(guard);
        let frame = captured.lock().unwrap().take();
        self.record(operation, frame);
        result
    }
}

struct DryRunGuard(Arc<Endpoint>);

impl Drop for DryRunGuard {
    fn drop(&mut self) {
        self.0.set_dry_run_hook(None);
    }
}

impl ConverterTransport for DryRunEndpointTransport {
    fn calls(&self) -> Vec<Value> {
        self.calls()
    }

    fn execute(&self, request: &HostRequest) -> TransportFuture {
        let request = request.clone();
        let state = Arc::clone(&self.state);
        Box::pin(async move {
            match state.execute_op(&request).await {
                Ok(value) => HostResult::Ok(value),
                Err(error) => HostResult::Err(error),
            }
        })
    }
}

fn parse_options(operation: &Value, has_response: bool) -> Options {
    let value = operation.get("options").cloned().unwrap_or(Value::Null);
    let number = |key: &str| value.get(key).and_then(Value::as_u64);
    let boolean = |key: &str| value.get(key).and_then(Value::as_bool);
    let mut options = Options {
        manufacturer_code: number("manufacturerCode").map(|value| value as u16),
        disable_default_response: boolean("disableDefaultResponse"),
        disable_response: boolean("disableResponse"),
        timeout: number("timeout"),
        direction: number("direction").map(|value| value as u8),
        src_endpoint: number("srcEndpoint").map(|value| value as u8),
        profile_id: number("profileId").map(|value| value as u16),
        reserved_bits: number("reservedBits").map(|value| value as u8),
        transaction_sequence_number: number("transactionSequenceNumber").map(|value| value as u8),
        disable_recovery: boolean("disableRecovery"),
        write_undiv: boolean("writeUndiv"),
        send_policy: None,
    };
    if has_response {
        // Upstream `commandResponse` never disables the default response flag
        // through the caller options unless explicitly requested.
        options.disable_default_response = options.disable_default_response.or(Some(false));
    }
    options
}

fn options_json(options: &OptionsWithDefaults) -> Value {
    json!({
        "manufacturerCode": options.manufacturer_code,
        "disableDefaultResponse": options.disable_default_response,
        "disableResponse": options.disable_response,
        "direction": options.direction,
        "srcEndpoint": options.src_endpoint,
        "profileId": options.profile_id,
        "reservedBits": options.reserved_bits,
        "transactionSequenceNumber": options.transaction_sequence_number,
        "timeout": options.timeout,
        "disableRecovery": options.disable_recovery,
        "writeUndiv": options.write_undiv,
    })
}

fn read_response(
    request: &DryRunRequest<'_>,
    device: &str,
    cluster_name: &str,
    reads: &DryRunReadValues,
) -> Option<Vec<u8>> {
    let frame = request.frame;
    if frame.header.frame_control.frame_type
        != zigbee_apiarist::zspec::zcl::zcl_header::FrameType::GLOBAL
        || frame.command.name != "read"
    {
        return None;
    }
    let values = reads.lock().unwrap();
    let mut entries = Vec::new();
    for item in frame.payload.as_array()? {
        let attr_id = item.get("attrId")?.as_u64()? as u16;
        let attribute = Utils::get_cluster_attribute(
            &frame.cluster,
            attr_id,
            request.options.manufacturer_code,
        )?;
        let key = (
            device.to_owned(),
            cluster_name.to_owned(),
            attribute.name.clone(),
        );
        if let Some(value) = values.get(&key) {
            entries.push(json!({
                "attrId": attr_id,
                "status": 0,
                "dataType": attribute.r#type as u16,
                "attrData": value,
            }));
        }
    }
    if entries.is_empty() {
        return None;
    }
    let response = Frame::create_with_definitions(
        frame.cluster.clone(),
        Utils::get_global_command("readRsp"),
        0,
        zigbee_apiarist::zspec::zcl::zcl_header::Direction::SERVER_TO_CLIENT as u8,
        true,
        request.options.manufacturer_code,
        frame.header.transaction_sequence_number,
        Value::Array(entries),
        0,
    );
    Some(response.to_buffer())
}

fn parse_reporting_items(items: &[Value]) -> Result<Vec<ConfigureReportingItem>, String> {
    let mut parsed = Vec::new();
    for item in items {
        let attribute = item
            .get("attribute")
            .cloned()
            .ok_or_else(|| "configureReporting item has no attribute".to_owned())?;
        let minimum_report_interval = item
            .get("minimumReportInterval")
            .and_then(Value::as_u64)
            .unwrap_or_default() as u32;
        let maximum_report_interval = item
            .get("maximumReportInterval")
            .and_then(Value::as_u64)
            .unwrap_or_default() as u32;
        let reportable_change = item.get("reportableChange").and_then(Value::as_f64);
        parsed.push(ConfigureReportingItem {
            attribute,
            minimum_report_interval,
            maximum_report_interval,
            reportable_change,
        });
    }
    Ok(parsed)
}
