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
    BindTarget, ConfigureReportingItem, DryRunRequest, Endpoint, Options, OptionsWithDefaults,
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

/// Resolves a device IEEE and endpoint id to the existing controller model.
pub type EndpointResolver = Arc<dyn Fn(&str, u8) -> Option<Arc<Endpoint>> + Send + Sync>;

/// Real converter transport: every host operation is executed by the existing
/// `Endpoint` methods and therefore by the controller/adapter. There is no
/// mock response path and no dry-run hook; every failure is returned to the
/// converter (which surfaces it through the bridge).
pub struct EndpointTransport {
    core: Arc<TransportCore>,
}

/// Recording/dry-run transport. Frames are built by the same endpoint methods,
/// captured before transmission and answered from seeded read values. The hook
/// is installed once per endpoint and is not removed by concurrent calls.
pub struct DryRunEndpointTransport {
    core: Arc<TransportCore>,
    dry: Arc<DryRunSink>,
}

struct TransportCore {
    resolver: EndpointResolver,
    calls: Arc<Mutex<Vec<Value>>>,
}

impl EndpointTransport {
    pub fn new(resolver: EndpointResolver) -> Self {
        Self {
            core: Arc::new(TransportCore {
                resolver,
                calls: Arc::new(Mutex::new(Vec::new())),
            }),
        }
    }

    pub fn calls(&self) -> Vec<Value> {
        self.core.calls.lock().unwrap().clone()
    }
}

impl DryRunEndpointTransport {
    pub fn new(resolver: EndpointResolver) -> Self {
        let dry = Arc::new(DryRunSink::default());
        Self {
            core: Arc::new(TransportCore {
                resolver,
                calls: Arc::new(Mutex::new(Vec::new())),
            }),
            dry,
        }
    }

    pub fn set_read(
        &self,
        device: impl Into<String>,
        cluster: impl Into<String>,
        attribute: impl Into<String>,
        value: Value,
    ) {
        self.dry
            .reads
            .lock()
            .unwrap()
            .insert((device.into(), cluster.into(), attribute.into()), value);
    }

    pub fn calls(&self) -> Vec<Value> {
        self.core.calls.lock().unwrap().clone()
    }

    /// Remove the capture hooks from the endpoints this transport touched.
    pub fn clear(&self) {
        self.dry.clear();
    }
}

impl Drop for DryRunEndpointTransport {
    fn drop(&mut self) {
        self.dry.clear();
    }
}

#[derive(Default)]
struct DryRunSink {
    reads: ReadValues,
    installed: Mutex<HashMap<(String, u8), Arc<Endpoint>>>,
}

impl DryRunSink {
    fn install(
        &self,
        calls: &Arc<Mutex<Vec<Value>>>,
        device: &str,
        endpoint_id: u8,
        endpoint: &Arc<Endpoint>,
    ) {
        let mut installed = self.installed.lock().unwrap();
        if installed.contains_key(&(device.to_owned(), endpoint_id)) {
            return;
        }
        let reads = Arc::clone(&self.reads);
        let calls = Arc::clone(calls);
        let hook_device = device.to_owned();
        endpoint.set_dry_run_hook(Some(Arc::new(move |request: &DryRunRequest<'_>| {
            calls
                .lock()
                .unwrap()
                .push(frame_record(request, &hook_device));
            Ok(read_response(request, &hook_device, &reads))
        })));
        installed.insert((device.to_owned(), endpoint_id), Arc::clone(endpoint));
    }

    fn clear(&self) {
        for endpoint in self.installed.lock().unwrap().values() {
            endpoint.set_dry_run_hook(None);
        }
        self.installed.lock().unwrap().clear();
    }
}

impl TransportCore {
    async fn execute(
        &self,
        request: &HostRequest,
        dry: Option<&DryRunSink>,
    ) -> Result<Value, String> {
        let operation = &request.operation;
        let op = operation
            .get("op")
            .and_then(Value::as_str)
            .ok_or_else(|| "host operation is missing 'op'".to_owned())?;
        let endpoint_id = operation_u8(operation, "endpoint")?.unwrap_or_default();
        let cluster = operation
            .get("cluster")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let endpoint = (self.resolver)(&request.device, endpoint_id).ok_or_else(|| {
            format!(
                "converter transport: device {} endpoint {} not found",
                request.device, endpoint_id
            )
        })?;
        let options = parse_options(operation)?;
        if let Some(dry) = dry {
            dry.install(&self.calls, &request.device, endpoint_id, &endpoint);
        }
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
                let payload = operation.get("payload").cloned().unwrap_or(Value::Null);
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
                let payload = operation.get("payload").cloned().unwrap_or(Value::Null);
                let tsn = operation_u8(operation, "transactionSequenceNumber")?
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
            "bind" | "unbind" => {
                let target = bind_target(operation, &self.resolver, dry.is_some())?;
                if let Some(target) = target {
                    if op == "bind" {
                        endpoint.bind(Value::from(cluster), target).await?;
                    } else {
                        let force = operation
                            .get("force")
                            .and_then(Value::as_bool)
                            .unwrap_or(false);
                        endpoint.unbind(Value::from(cluster), target, force).await?;
                    }
                }
                Ok(json!({"recorded": dry.is_some()}))
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
                "converter transport does not implement host op '{other}' (device {}, endpoint {endpoint_id})",
                request.device
            )),
        }?;
        if matches!(op, "bind" | "unbind") {
            self.record(
                operation,
                json!({
                    "target": operation.get("target").cloned().unwrap_or(Value::Null),
                    "targetType": operation.get("targetType").cloned().unwrap_or(Value::Null),
                    "recorded": dry.is_some(),
                }),
            );
        } else if dry.is_none() {
            self.record(operation, json!({"result": result.clone()}));
        }
        Ok(result)
    }

    fn record(&self, operation: &Value, extra: Value) {
        let mut entry = operation.clone();
        if let Some(object) = entry.as_object_mut() {
            if let Some(extra) = extra.as_object() {
                for (key, value) in extra {
                    object.insert(key.clone(), value.clone());
                }
            }
        }
        self.calls.lock().unwrap().push(entry);
    }
}

fn bind_target(
    operation: &Value,
    resolver: &EndpointResolver,
    dry_run: bool,
) -> Result<Option<BindTarget>, String> {
    let target_type = operation
        .get("targetType")
        .and_then(Value::as_str)
        .unwrap_or("endpoint");
    let target = operation_u16(operation, "target")?
        .ok_or_else(|| "bind/unbind requires a target".to_owned())?;
    match target_type {
        "group" => Ok((!dry_run).then_some(BindTarget::GroupId(target))),
        "endpoint" => {
            let endpoint_id = u8::try_from(target)
                .map_err(|_| "bind target endpoint is out of range".to_owned())?;
            if dry_run {
                // The dry-run records the intent; resolving the target
                // endpoint is only needed for a real ZDO bind/unbind.
                return Ok(None);
            }
            let ieee = operation
                .get("targetIeee")
                .and_then(Value::as_str)
                .ok_or_else(|| "endpoint bind target requires targetIeee".to_owned())?;
            let endpoint = resolver(ieee, endpoint_id)
                .ok_or_else(|| format!("bind target {ieee}/{endpoint_id} not found"))?;
            Ok(Some(BindTarget::Endpoint(endpoint)))
        }
        other => Err(format!("unknown bind target type '{other}'")),
    }
}

fn frame_record(request: &DryRunRequest<'_>, device: &str) -> Value {
    let frame = request.frame;
    let (frame_type, op) = match frame.header.frame_control.frame_type {
        zigbee_apiarist::zspec::zcl::zcl_header::FrameType::GLOBAL => (
            0u8,
            match frame.command.name.as_str() {
                "read" => "read",
                "write" | "writeUndiv" => "write",
                "configReport" => "configureReporting",
                other => other,
            },
        ),
        zigbee_apiarist::zspec::zcl::zcl_header::FrameType::SPECIFIC => (
            1u8,
            if frame.header.frame_control.direction
                == zigbee_apiarist::zspec::zcl::zcl_header::Direction::SERVER_TO_CLIENT
            {
                "commandResponse"
            } else {
                "command"
            },
        ),
    };
    let bytes = frame.to_buffer();
    json!({
        "op": op,
        "device": device,
        "endpoint": request.endpoint,
        "cluster": frame.cluster.name,
        "clusterId": frame.cluster.ID,
        "frame": {
            "bytes": bytes,
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
        },
    })
}

impl ConverterTransport for EndpointTransport {
    fn calls(&self) -> Vec<Value> {
        self.calls()
    }

    fn execute(&self, request: &HostRequest) -> TransportFuture {
        let request = request.clone();
        let core = Arc::clone(&self.core);
        Box::pin(async move {
            match core.execute(&request, None).await {
                Ok(value) => HostResult::Ok(value),
                Err(error) => HostResult::Err(error),
            }
        })
    }
}

impl ConverterTransport for DryRunEndpointTransport {
    fn calls(&self) -> Vec<Value> {
        self.calls()
    }

    fn execute(&self, request: &HostRequest) -> TransportFuture {
        let request = request.clone();
        let core = Arc::clone(&self.core);
        let dry = Arc::clone(&self.dry);
        Box::pin(async move {
            match core.execute(&request, Some(&dry)).await {
                Ok(value) => HostResult::Ok(value),
                Err(error) => HostResult::Err(error),
            }
        })
    }
}

fn operation_u64(operation: &Value, key: &str) -> Result<Option<u64>, String> {
    match operation.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .map(Some)
            .ok_or_else(|| format!("{key} must be a non-negative integer")),
    }
}

fn operation_u8(operation: &Value, key: &str) -> Result<Option<u8>, String> {
    operation_u64(operation, key)?
        .map(|value| u8::try_from(value).map_err(|_| format!("{key} must fit in u8 (0..=255)")))
        .transpose()
}

fn operation_u16(operation: &Value, key: &str) -> Result<Option<u16>, String> {
    operation_u64(operation, key)?
        .map(|value| u16::try_from(value).map_err(|_| format!("{key} must fit in u16 (0..=65535)")))
        .transpose()
}

fn option_u64(value: &Value, key: &str) -> Result<Option<u64>, String> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .map(Some)
            .ok_or_else(|| format!("option {key} must be a non-negative integer")),
    }
}

fn option_u8(value: &Value, key: &str) -> Result<Option<u8>, String> {
    option_u64(value, key)?
        .map(|value| {
            u8::try_from(value).map_err(|_| format!("option {key} must fit in u8 (0..=255)"))
        })
        .transpose()
}

fn option_u16(value: &Value, key: &str) -> Result<Option<u16>, String> {
    option_u64(value, key)?
        .map(|value| {
            u16::try_from(value).map_err(|_| format!("option {key} must fit in u16 (0..=65535)"))
        })
        .transpose()
}

fn option_bool(value: &Value, key: &str) -> Result<Option<bool>, String> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_bool()
            .map(Some)
            .ok_or_else(|| format!("option {key} must be a boolean")),
    }
}

fn parse_options(operation: &Value) -> Result<Options, String> {
    let value = operation.get("options").cloned().unwrap_or(Value::Null);
    Ok(Options {
        manufacturer_code: option_u16(&value, "manufacturerCode")?,
        disable_default_response: option_bool(&value, "disableDefaultResponse")?,
        disable_response: option_bool(&value, "disableResponse")?,
        timeout: option_u64(&value, "timeout")?,
        direction: option_u8(&value, "direction")?,
        src_endpoint: option_u8(&value, "srcEndpoint")?,
        profile_id: option_u16(&value, "profileId")?,
        reserved_bits: option_u8(&value, "reservedBits")?,
        transaction_sequence_number: option_u8(&value, "transactionSequenceNumber")?,
        disable_recovery: option_bool(&value, "disableRecovery")?,
        write_undiv: option_bool(&value, "writeUndiv")?,
        send_policy: None,
    })
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

fn read_response(request: &DryRunRequest<'_>, device: &str, reads: &ReadValues) -> Option<Vec<u8>> {
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
            frame.cluster.name.clone(),
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
        let minimum_report_interval = u32::try_from(
            item.get("minimumReportInterval")
                .and_then(Value::as_u64)
                .unwrap_or_default(),
        )
        .map_err(|_| "minimumReportInterval is out of range".to_owned())?;
        let maximum_report_interval = u32::try_from(
            item.get("maximumReportInterval")
                .and_then(Value::as_u64)
                .unwrap_or_default(),
        )
        .map_err(|_| "maximumReportInterval is out of range".to_owned())?;
        let reportable_change = match item.get("reportableChange") {
            None | Some(Value::Null) => None,
            Some(value) => Some(
                value
                    .as_f64()
                    .ok_or_else(|| "reportableChange must be a number".to_owned())?,
            ),
        };
        parsed.push(ConfigureReportingItem {
            attribute,
            minimum_report_interval,
            maximum_report_interval,
            reportable_change,
        });
    }
    Ok(parsed)
}
