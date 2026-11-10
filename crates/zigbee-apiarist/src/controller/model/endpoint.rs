// Original: .upstream/zigbee-herdsman/src/controller/model/endpoint.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex as StdMutex};

use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::adapter::events::ZclPayload;
use crate::controller::helpers::request::{BoxedRequestFuture, Request};
use crate::controller::helpers::request_queue::RequestQueue;
use crate::controller::helpers::zcl_frame_converter::{attribute_key_value, attribute_list};
use crate::controller::helpers::zcl_transaction_sequence_number;
use crate::controller::model::device::Device;
use crate::controller::model::entity::Entity;
use crate::controller::model::group::Group;
use crate::controller::tstype::{KeyValue, SendPolicy};
use crate::utils::logger::logger;
use crate::zspec::enums::BroadcastAddress;
use crate::zspec::zcl::definition::status::Status;
use crate::zspec::zcl::utils::{Cluster, Command};
use crate::zspec::zcl::utils::{CustomClusters, Utils};
use crate::zspec::zcl::zcl_frame::Frame;
use crate::zspec::zcl::zcl_header::Direction;
use crate::zspec::zcl::zcl_header::FrameType;
use crate::zspec::zdo::buffalo_zdo::BuffaloZdo;
use crate::zspec::zdo::definition::clusters::ClusterId;
use crate::zspec::zdo::definition::consts::{MULTICAST_BINDING, UNICAST_BINDING};

const NS: &str = "zh:controller:endpoint";

/// Upstream `sendRequest` is generic over the callback result. The endpoint
/// queue carries both regular ZCL responses and Green Power broadcasts (which
/// resolve to `undefined`); the enum keeps a single ordered queue.
#[derive(Clone)]
pub enum EndpointRequestResult {
    Payload(ZclPayload),
    Unit,
}

/// Upstream `BindInternal`.
#[derive(Debug, Clone, PartialEq)]
pub struct BindInternal {
    pub cluster: u16,
    pub r#type: String,
    pub device_ieee_address: Option<String>,
    pub endpoint_id: Option<u8>,
    pub group_id: Option<u16>,
}

/// Upstream `ConfiguredReportingInternal`.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfiguredReportingInternal {
    pub cluster: u16,
    pub attr_id: u16,
    pub min_rep_intval: u32,
    pub max_rep_intval: u32,
    pub rep_change: f64,
    pub manufacturer_code: Option<u16>,
}

/// Upstream `ConfiguredReporting`.
#[derive(Debug, Clone)]
pub struct ConfiguredReporting {
    pub cluster: Cluster,
    pub attribute: Value,
    pub minimum_report_interval: u32,
    pub maximum_report_interval: u32,
    pub reportable_change: f64,
}

/// Upstream `Options`.
#[derive(Debug, Clone, Default)]
pub struct Options {
    pub manufacturer_code: Option<u16>,
    pub disable_default_response: Option<bool>,
    pub disable_response: Option<bool>,
    pub timeout: Option<u64>,
    pub direction: Option<u8>,
    pub src_endpoint: Option<u8>,
    pub profile_id: Option<u16>,
    pub reserved_bits: Option<u8>,
    pub transaction_sequence_number: Option<u8>,
    pub disable_recovery: Option<bool>,
    pub write_undiv: Option<bool>,
    pub send_policy: Option<SendPolicy>,
}

/// Upstream `OptionsWithDefaults`.
#[derive(Debug, Clone)]
pub struct OptionsWithDefaults {
    pub manufacturer_code: Option<u16>,
    pub disable_default_response: bool,
    pub disable_response: bool,
    pub timeout: u64,
    pub direction: u8,
    pub src_endpoint: Option<u8>,
    pub profile_id: Option<u16>,
    pub reserved_bits: u8,
    pub transaction_sequence_number: Option<u8>,
    pub disable_recovery: bool,
    pub write_undiv: bool,
    pub send_policy: Option<SendPolicy>,
}

/// Upstream `Bind` target: Endpoint or Group.
#[derive(Clone)]
pub enum BindTarget {
    Endpoint(Arc<Endpoint>),
    Group(Arc<Group>),
}

impl PartialEq for BindTarget {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Endpoint(a), Self::Endpoint(b)) => Arc::ptr_eq(a, b),
            (Self::Group(a), Self::Group(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

/// Upstream `Bind`.
#[derive(Clone)]
pub struct Bind {
    pub cluster: Cluster,
    pub target: BindTarget,
}

/// Upstream `ConfigureReportingItem`.
#[derive(Debug, Clone)]
pub struct ConfigureReportingItem {
    /// Attribute name/ID or `{ID, type}`.
    pub attribute: Value,
    pub minimum_report_interval: u32,
    pub maximum_report_interval: u32,
    pub reportable_change: Option<f64>,
}

/// Upstream `ReadReportConfigItem`.
#[derive(Debug, Clone)]
pub struct ReadReportConfigItem {
    pub direction: Option<u8>,
    pub attribute: Value,
}

/// Upstream `Endpoint`.
pub struct Endpoint {
    pub device_id: StdMutex<Option<u16>>,
    pub profile_id: StdMutex<Option<u16>>,
    pub input_clusters: StdMutex<Vec<u16>>,
    pub output_clusters: StdMutex<Vec<u16>>,
    /// Upstream camelCase: `ID`.
    pub ID: u8,
    pub clusters: StdMutex<Value>,
    pub device_ieee_address: StdMutex<String>,
    pub device_network_address: StdMutex<u16>,
    binds: StdMutex<Vec<BindInternal>>,
    configured_reportings: StdMutex<Vec<ConfiguredReportingInternal>>,
    pub meta: StdMutex<KeyValue>,
    pending_requests: Mutex<RequestQueue<EndpointRequestResult>>,
    pending_request_count: Arc<AtomicUsize>,
    self_ref: std::sync::OnceLock<std::sync::Weak<Endpoint>>,
}

impl Endpoint {
    /// Upstream private constructor.
    #[allow(clippy::too_many_arguments)]
    fn new(
        id: u8,
        profile_id: Option<u16>,
        device_id: Option<u16>,
        input_clusters: Vec<u16>,
        output_clusters: Vec<u16>,
        device_network_address: u16,
        device_ieee_address: String,
        clusters: Value,
        binds: Vec<BindInternal>,
        configured_reportings: Vec<ConfiguredReportingInternal>,
        meta: KeyValue,
    ) -> Self {
        let pending_requests = RequestQueue::new(device_ieee_address.clone(), id);
        let pending_request_count = pending_requests.size_handle();
        Self {
            device_id: StdMutex::new(device_id),
            profile_id: StdMutex::new(profile_id),
            input_clusters: StdMutex::new(input_clusters),
            output_clusters: StdMutex::new(output_clusters),
            ID: id,
            clusters: StdMutex::new(clusters),
            device_ieee_address: StdMutex::new(device_ieee_address.clone()),
            device_network_address: StdMutex::new(device_network_address),
            binds: StdMutex::new(binds),
            configured_reportings: StdMutex::new(configured_reportings),
            meta: StdMutex::new(meta),
            pending_requests: Mutex::new(pending_requests),
            pending_request_count,
            self_ref: std::sync::OnceLock::new(),
        }
    }

    /// Upstream `get binds()`.
    pub fn binds(&self) -> Vec<Bind> {
        let mut result = Vec::new();
        for bind in self.binds.lock().unwrap().iter() {
            let target = if bind.r#type == "endpoint" {
                let Some(ieee) = bind.device_ieee_address.as_ref() else {
                    continue;
                };
                let Some(endpoint_id) = bind.endpoint_id else {
                    continue;
                };
                Device::by_ieee_addr(ieee, false)
                    .and_then(|device| device.get_endpoint(endpoint_id))
                    .map(BindTarget::Endpoint)
            } else {
                bind.group_id
                    .and_then(Group::by_group_id)
                    .map(BindTarget::Group)
            };
            if let Some(target) = target {
                result.push(Bind {
                    cluster: self.get_cluster(Value::from(bind.cluster), None, None),
                    target,
                });
            }
        }
        result
    }

    /// Upstream `get configuredReportings()`.
    pub fn configured_reportings(&self) -> Vec<ConfiguredReporting> {
        let device = self.get_device();
        let custom_clusters = device.custom_clusters();
        self.configured_reportings
            .lock()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let cluster = Utils::get_cluster(
                    entry.cluster as u32,
                    entry.manufacturer_code,
                    &custom_clusters,
                );
                let attribute = Utils::get_cluster_attribute(
                    &cluster,
                    entry.attr_id as u32,
                    entry.manufacturer_code,
                )
                .map(|attribute| {
                    json!({
                        "ID": attribute.ID,
                        "name": attribute.name,
                        "type": attribute.r#type as u16,
                        "manufacturerCode": attribute.manufacturerCode,
                    })
                })
                .unwrap_or_else(|| {
                    json!({
                        "ID": entry.attr_id,
                        "name": format!("attr{index}"),
                        "type": crate::zspec::zcl::definition::enums::DataType::UNKNOWN as u16,
                        "manufacturerCode": null,
                    })
                });
                ConfiguredReporting {
                    cluster,
                    attribute,
                    minimum_report_interval: entry.min_rep_intval,
                    maximum_report_interval: entry.max_rep_intval,
                    reportable_change: entry.rep_change,
                }
            })
            .collect()
    }

    /// Upstream `getDevice()`.
    pub fn get_device(&self) -> Arc<Device> {
        let ieee = self.device_ieee_address.lock().unwrap().clone();
        match Device::by_ieee_addr(&ieee, false) {
            Some(device) => device,
            None => {
                logger().error(
                    &format!(
                        "Tried to get unknown/deleted device {ieee} from endpoint {}.",
                        self.ID
                    ),
                    NS,
                );
                panic!("Unknown/deleted device {ieee}");
            }
        }
    }

    /// Upstream `supportsInputCluster(clusterKey)`.
    pub fn supports_input_cluster(&self, cluster_key: Value) -> bool {
        let cluster = self.get_cluster(cluster_key, None, None);
        self.input_clusters
            .lock()
            .unwrap()
            .contains(&(cluster.ID as u16))
    }

    /// Upstream `supportsOutputCluster(clusterKey)`.
    pub fn supports_output_cluster(&self, cluster_key: Value) -> bool {
        let cluster = self.get_cluster(cluster_key, None, None);
        self.output_clusters
            .lock()
            .unwrap()
            .contains(&(cluster.ID as u16))
    }

    /// Upstream `getInputClusters()`.
    pub fn get_input_clusters(&self) -> Vec<Cluster> {
        self.input_clusters
            .lock()
            .unwrap()
            .iter()
            .map(|id| self.get_cluster(Value::from(*id), None, None))
            .collect()
    }

    /// Upstream `getOutputClusters()`.
    pub fn get_output_clusters(&self) -> Vec<Cluster> {
        self.output_clusters
            .lock()
            .unwrap()
            .iter()
            .map(|id| self.get_cluster(Value::from(*id), None, None))
            .collect()
    }

    /// Upstream `fromDatabaseRecord(record, deviceNetworkAddress, deviceIeeeAddress)`.
    pub fn from_database_record(
        record: &KeyValue,
        device_network_address: u16,
        device_ieee_address: &str,
    ) -> Arc<Self> {
        let mut record = record.clone();
        if let Some(clusters) = record.get_mut("clusters").and_then(Value::as_object_mut) {
            for (_, entry) in clusters.iter_mut() {
                if let Some(attributes) = entry.get("attrs").cloned() {
                    entry["attributes"] = attributes;
                    if let Some(object) = entry.as_object_mut() {
                        object.remove("attrs");
                    }
                }
            }
            for (from, to) in [
                ("piRetailTunnel", "retailTunnel"),
                ("tunneling", "seTunneling"),
                ("haMeterIdentification", "seMeterIdentification"),
            ] {
                if let Some(value) = clusters.remove(from) {
                    clusters.insert(to.to_string(), value);
                }
            }
        }

        let value_u8 =
            |key: &str| -> u8 { record.get(key).and_then(Value::as_u64).unwrap_or(0) as u8 };
        let value_u16 = |key: &str| -> Option<u16> {
            record.get(key).and_then(Value::as_u64).map(|v| v as u16)
        };
        let value_vec = |key: &str| -> Vec<u16> {
            record
                .get(key)
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_u64)
                        .map(|v| v as u16)
                        .collect()
                })
                .unwrap_or_default()
        };
        let binds = record
            .get("binds")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(bind_from_value).collect::<Vec<_>>())
            .unwrap_or_default();
        let reportings = record
            .get("configuredReportings")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .map(|entry| ConfiguredReportingInternal {
                        cluster: entry.get("cluster").and_then(Value::as_u64).unwrap_or(0) as u16,
                        attr_id: entry.get("attrId").and_then(Value::as_u64).unwrap_or(0) as u16,
                        min_rep_intval: entry
                            .get("minRepIntval")
                            .and_then(Value::as_u64)
                            .unwrap_or(0) as u32,
                        max_rep_intval: entry
                            .get("maxRepIntval")
                            .and_then(Value::as_u64)
                            .unwrap_or(0) as u32,
                        rep_change: entry
                            .get("repChange")
                            .and_then(Value::as_f64)
                            .unwrap_or(0.0),
                        manufacturer_code: entry
                            .get("manufacturerCode")
                            .and_then(Value::as_u64)
                            .map(|v| v as u16),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Self::into_arc(Self::new(
            value_u8("epId"),
            value_u16("profId"),
            value_u16("devId"),
            value_vec("inClusterList"),
            value_vec("outClusterList"),
            device_network_address,
            device_ieee_address.to_string(),
            record.get("clusters").cloned().unwrap_or_else(|| json!({})),
            binds,
            reportings,
            record.get("meta").cloned().unwrap_or_else(|| json!({})),
        ))
    }

    /// Upstream `toDatabaseRecord()`.
    pub fn to_database_record(&self) -> KeyValue {
        let binds = self
            .binds
            .lock()
            .unwrap()
            .iter()
            .map(bind_to_value)
            .collect::<Vec<_>>();
        let reportings = self
            .configured_reportings
            .lock()
            .unwrap()
            .iter()
            .map(|entry| {
                json!({
                    "cluster": entry.cluster,
                    "attrId": entry.attr_id,
                    "minRepIntval": entry.min_rep_intval,
                    "maxRepIntval": entry.max_rep_intval,
                    "repChange": entry.rep_change,
                    "manufacturerCode": entry.manufacturer_code,
                })
            })
            .collect::<Vec<_>>();
        json!({
            "profId": *self.profile_id.lock().unwrap(),
            "epId": self.ID,
            "devId": *self.device_id.lock().unwrap(),
            "inClusterList": *self.input_clusters.lock().unwrap(),
            "outClusterList": *self.output_clusters.lock().unwrap(),
            "clusters": self.clusters.lock().unwrap().clone(),
            "binds": binds,
            "configuredReportings": reportings,
            "meta": self.meta.lock().unwrap().clone(),
        })
    }

    /// Upstream `create(id, profileID, deviceID, inputClusters, outputClusters, ...)`.
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        id: u8,
        profile_id: Option<u16>,
        device_id: Option<u16>,
        input_clusters: Vec<u16>,
        output_clusters: Vec<u16>,
        device_network_address: u16,
        device_ieee_address: &str,
    ) -> Arc<Self> {
        Self::into_arc(Self::new(
            id,
            profile_id,
            device_id,
            input_clusters,
            output_clusters,
            device_network_address,
            device_ieee_address.to_string(),
            json!({}),
            Vec::new(),
            Vec::new(),
            json!({}),
        ))
    }

    fn into_arc(endpoint: Self) -> Arc<Self> {
        Arc::new_cyclic(|weak| {
            let _ = endpoint.self_ref.set(weak.clone());
            endpoint
        })
    }

    fn arc_self(&self) -> Arc<Self> {
        self.self_ref
            .get()
            .and_then(std::sync::Weak::upgrade)
            .expect("endpoint self reference missing")
    }

    /// Upstream `saveClusterAttributeKeyValue(clusterKey, list)`.
    pub fn save_cluster_attribute_key_value(&self, cluster_key: Value, list: Value) {
        let cluster = self.get_cluster(cluster_key, None, None);
        let mut clusters = self.clusters.lock().unwrap();
        let object = clusters
            .as_object_mut()
            .expect("endpoint clusters must be an object");
        let entry = object
            .entry(cluster.name.clone())
            .or_insert_with(|| json!({"attributes": {}}));
        let attributes = entry
            .as_object_mut()
            .expect("cluster entry")
            .entry("attributes")
            .or_insert_with(|| json!({}));
        if let (Some(attributes), Some(list)) = (attributes.as_object_mut(), list.as_object()) {
            for (key, value) in list {
                attributes.insert(key.clone(), value.clone());
            }
        }
    }

    /// Upstream `getClusterAttributeValue(clusterKey, attributeKey)`.
    pub fn get_cluster_attribute_value(
        &self,
        cluster_key: Value,
        attribute_key: Value,
    ) -> Option<Value> {
        let cluster = self.get_cluster(cluster_key, None, None);
        let clusters = self.clusters.lock().unwrap();
        let entry = clusters.get(&cluster.name)?;
        let attributes = entry.get("attributes")?;
        let key = match attribute_key {
            Value::String(name) => Some(name),
            other => other.as_u64().and_then(|id| {
                Utils::get_cluster_attribute(&cluster, id as u32, None)
                    .map(|attribute| attribute.name)
            }),
        }?;
        attributes.get(&key).cloned()
    }

    /// Upstream `saveClusterAttributeReportConfig(...)`.
    pub fn save_cluster_attribute_report_config(
        &self,
        cluster_id: u16,
        manufacturer_code: Option<u16>,
        report_configs: &[Value],
    ) {
        {
            let mut reportings = self.configured_reportings.lock().unwrap();
            for entry in report_configs {
                if entry.get("direction").and_then(Value::as_u64)
                    == Some(Direction::SERVER_TO_CLIENT as u64)
                {
                    continue;
                }
                let attr_id = entry.get("attrId").and_then(Value::as_u64).unwrap_or(0) as u16;
                let existing = reportings.iter().position(|reporting| {
                    reporting.cluster == cluster_id
                        && reporting.attr_id == attr_id
                        && (manufacturer_code.is_none()
                            || manufacturer_code == reporting.manufacturer_code)
                });
                let status = entry.get("status").and_then(Value::as_u64).unwrap_or(0);
                if status == 0 {
                    let min = entry
                        .get("minRepIntval")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as u32;
                    let max = entry
                        .get("maxRepIntval")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as u32;
                    let rep = entry
                        .get("repChange")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);
                    if let Some(index) = existing {
                        reportings[index].min_rep_intval = min;
                        reportings[index].max_rep_intval = max;
                        reportings[index].rep_change = rep;
                    } else {
                        reportings.push(ConfiguredReportingInternal {
                            cluster: cluster_id,
                            attr_id,
                            min_rep_intval: min,
                            max_rep_intval: max,
                            rep_change: rep,
                            manufacturer_code,
                        });
                    }
                } else if let Some(index) = existing {
                    reportings.remove(index);
                }
            }
        }
        self.save();
    }

    /// Upstream `saveBindings(binds)`.
    pub fn save_bindings(&self, binds: Vec<BindInternal>) {
        *self.binds.lock().unwrap() = binds;
        self.save();
    }

    /// Upstream `clearBindings()`.
    pub fn clear_bindings(&self) {
        self.binds.lock().unwrap().clear();
        self.save();
    }

    /// Upstream `hasPendingRequests()`.
    pub fn has_pending_requests(&self) -> bool {
        self.pending_request_count.load(Ordering::SeqCst) > 0
    }

    /// Upstream `sendPendingRequests(fastPolling)`.
    pub async fn send_pending_requests(&self, fast_polling: bool) {
        self.pending_requests.lock().await.send(fast_polling).await;
    }

    /// Upstream `sendRequest(frame, options)`.
    async fn send_request(
        &self,
        frame: Frame,
        options: &OptionsWithDefaults,
    ) -> Result<ZclPayload, String> {
        let ieee = self.device_ieee_address.lock().unwrap().clone();
        let network = *self.device_network_address.lock().unwrap();
        let endpoint = self.ID;
        let frame_for_call = frame.clone();
        let timeout = options.timeout;
        let disable_response = options.disable_response;
        let disable_recovery = options.disable_recovery;
        let src_endpoint = options.src_endpoint;
        let profile_id = options.profile_id;
        let function = move || -> BoxedRequestFuture<EndpointRequestResult> {
            let ieee = ieee.clone();
            let frame = frame_for_call.clone();
            Box::pin(async move {
                let adapter = Entity::adapter();
                let response = adapter
                    .send_zcl_frame_to_endpoint(
                        &ieee,
                        network,
                        endpoint,
                        &frame,
                        timeout,
                        disable_response,
                        disable_recovery,
                        src_endpoint,
                        profile_id,
                    )
                    .await?;
                response
                    .map(EndpointRequestResult::Payload)
                    .ok_or_else(|| "No response received".to_string())
            })
        };
        match self.send_request_with(frame, options, function).await? {
            EndpointRequestResult::Payload(payload) => Ok(payload),
            EndpointRequestResult::Unit => Err("Expected ZCL payload".to_string()),
        }
    }

    /// Upstream `sendRequest(frame, options, func)`.
    async fn send_request_with<F>(
        &self,
        frame: Frame,
        options: &OptionsWithDefaults,
        function: F,
    ) -> Result<EndpointRequestResult, String>
    where
        F: Fn() -> BoxedRequestFuture<EndpointRequestResult> + Send + 'static,
    {
        let device = self.get_device();
        let timeout = device.pending_request_timeout();
        let mut request = Request::new_with_policy(function, frame, timeout, options.send_policy);
        let policy = request.send_policy;

        if policy != Some(SendPolicy::Bulk) {
            self.pending_requests.lock().await.filter(&mut request);
        }

        if policy == Some(SendPolicy::Immediate) || timeout == 0 {
            if timeout > 0 {
                logger().debug(
                    &format!(
                        "Request Queue ({} / {}): send {} request immediately (sendPolicy={:?})",
                        self.device_ieee_address.lock().unwrap(),
                        self.ID,
                        request.frame.command.name,
                        options.send_policy
                    ),
                    NS,
                );
            }
            return request.send().await;
        }

        if policy == Some(SendPolicy::Bulk) {
            return self.pending_requests.lock().await.queue(request).await;
        }

        match request.send().await {
            Ok(value) => Ok(value),
            Err(error) => {
                logger().debug(
                    &format!(
                        "Request Queue ({} / {}): queue request (transaction failed) ({error})",
                        self.device_ieee_address.lock().unwrap(),
                        self.ID
                    ),
                    NS,
                );
                self.pending_requests.lock().await.queue(request).await
            }
        }
    }

    fn check_status(&self, payload: &Value) -> Result<(), String> {
        let codes = if let Some(items) = payload.as_array() {
            items
                .iter()
                .filter_map(|item| item.get("status").and_then(Value::as_u64))
                .map(|code| code as u8)
                .collect::<Vec<_>>()
        } else {
            vec![payload
                .get("statusCode")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u8]
        };
        if let Some(invalid) = codes.iter().find(|code| **code != Status::SUCCESS as u8) {
            let name = Status::from_u8(*invalid)
                .map(Status::name)
                .unwrap_or("UNKNOWN");
            return Err(format!("Status '{name}'"));
        }
        Ok(())
    }

    /// Upstream `report(clusterKey, attributes, options?)`.
    pub async fn report(
        &self,
        cluster_key: Value,
        attributes: Value,
        options: Option<Options>,
    ) -> Result<(), String> {
        let cluster = self.get_cluster(
            cluster_key,
            None,
            options.as_ref().and_then(|o| o.manufacturer_code),
        );
        let mut payload = Vec::new();
        for (name_or_id, value) in attributes.as_object().cloned().unwrap_or_default() {
            let attribute = Utils::get_cluster_attribute(
                &cluster,
                name_or_id.as_str(),
                options.as_ref().and_then(|o| o.manufacturer_code),
            );
            if let Some(attribute) = attribute {
                payload.push(json!({
                    "attrId": attribute.ID,
                    "attrData": value,
                    "dataType": attribute.r#type as u16,
                }));
            } else if let Ok(id) = name_or_id.parse::<u16>() {
                payload.push(json!({
                    "attrId": id,
                    "attrData": value.get("value").cloned().unwrap_or(Value::Null),
                    "dataType": value.get("type").and_then(Value::as_u64).unwrap_or(0),
                }));
            } else {
                return Err(format!(
                    "Unknown attribute '{name_or_id}', specify either an existing attribute or a number"
                ));
            }
        }
        self.zcl_command(
            Value::Array(vec![json!(cluster.name)]),
            Value::from("report"),
            Value::Array(payload),
            options,
            Some(attributes),
            true,
            FrameType::GLOBAL as u8,
            Some(cluster),
        )
        .await?;
        Ok(())
    }

    /// Upstream `write(clusterKey, attributes, options?)`.
    pub async fn write(
        &self,
        cluster_key: Value,
        attributes: Value,
        options: Option<Options>,
    ) -> Result<(), String> {
        let cluster = self.get_cluster(
            cluster_key,
            None,
            options.as_ref().and_then(|o| o.manufacturer_code),
        );
        let mut options = self.get_options_with_defaults(
            options,
            true,
            Direction::CLIENT_TO_SERVER as u8,
            cluster.manufacturerCode,
        );
        options.manufacturer_code = self.ensure_manufacturer_code_is_unique_and_get(
            &cluster,
            attributes
                .as_object()
                .map(|object| object.keys().cloned().map(Value::String).collect())
                .unwrap_or_default(),
            options.manufacturer_code,
            "write",
        )?;
        let mut payload = Vec::new();
        for (name_or_id, value) in attributes.as_object().cloned().unwrap_or_default() {
            let attribute = Utils::get_cluster_attribute(
                &cluster,
                name_or_id.as_str(),
                options.manufacturer_code,
            );
            if let Some(attribute) = attribute {
                let attr_data = Utils::process_attribute_write(
                    &attribute,
                    crate::zspec::zcl::utils::ZclValue::from_json(&value),
                );
                payload.push(json!({
                    "attrId": attribute.ID,
                    "attrData": attr_data.to_json(),
                    "dataType": attribute.r#type as u16,
                }));
            } else if let Ok(id) = name_or_id.parse::<u16>() {
                payload.push(json!({
                    "attrId": id,
                    "attrData": value.get("value").cloned().unwrap_or(Value::Null),
                    "dataType": value.get("type").and_then(Value::as_u64).unwrap_or(0),
                }));
            } else {
                return Err(format!(
                    "Unknown attribute '{name_or_id}', specify either an existing attribute or a number"
                ));
            }
        }
        let command = if options.write_undiv {
            "writeUndiv"
        } else {
            "write"
        };
        self.zcl_command(
            Value::Array(vec![json!(cluster.name)]),
            Value::from(command),
            Value::Array(payload),
            Some(options.into()),
            Some(attributes),
            true,
            FrameType::GLOBAL as u8,
            Some(cluster),
        )
        .await?;
        Ok(())
    }

    /// Upstream `read(clusterKey, attributes, options?)`.
    pub async fn read(
        &self,
        cluster_key: Value,
        attributes: Value,
        options: Option<Options>,
    ) -> Result<Value, String> {
        let device = self.get_device();
        let cluster = self.get_cluster(
            cluster_key,
            Some(device.clone()),
            options.as_ref().and_then(|o| o.manufacturer_code),
        );
        let mut options = self.get_options_with_defaults(
            options,
            true,
            Direction::CLIENT_TO_SERVER as u8,
            cluster.manufacturerCode,
        );
        let attribute_keys = attributes
            .as_array()
            .map(|items| items.clone())
            .unwrap_or_default();
        options.manufacturer_code = self.ensure_manufacturer_code_is_unique_and_get(
            &cluster,
            attribute_keys.clone(),
            options.manufacturer_code,
            "read",
        )?;
        let mut payload = Vec::new();
        for attribute in attribute_keys {
            if let Some(id) = attribute.as_u64() {
                payload.push(json!({"attrId": id}));
            } else if let Some(name) = attribute.as_str() {
                if let Some(attribute) =
                    Utils::get_cluster_attribute(&cluster, name, options.manufacturer_code)
                {
                    Utils::process_attribute_pre_read(&attribute);
                    payload.push(json!({"attrId": attribute.ID}));
                } else {
                    logger().warning(
                        &format!(
                            "Ignoring unknown attribute {name} in cluster {}",
                            cluster.name
                        ),
                        NS,
                    );
                }
            }
        }
        let frame = self
            .zcl_command(
                Value::from(cluster.name.clone()),
                Value::from("read"),
                Value::Array(payload),
                Some(options.into()),
                Some(attributes),
                true,
                FrameType::GLOBAL as u8,
                Some(cluster),
            )
            .await?;
        Ok(match frame {
            Some(frame) => {
                attribute_key_value(&frame, device.manufacturer_id(), &device.custom_clusters())
            }
            None => json!({}),
        })
    }

    /// Upstream `readResponse(...)`.
    pub async fn read_response(
        &self,
        cluster_key: Value,
        transaction_sequence_number: u8,
        attributes: Value,
        options: Option<Options>,
    ) -> Result<(), String> {
        if options
            .as_ref()
            .and_then(|o| o.transaction_sequence_number)
            .is_some()
        {
            return Err("Use parameter".to_string());
        }
        let cluster = self.get_cluster(
            cluster_key,
            None,
            options.as_ref().and_then(|o| o.manufacturer_code),
        );
        let mut payload = Vec::new();
        for (name_or_id, value) in attributes.as_object().cloned().unwrap_or_default() {
            if let Some(attribute) = Utils::get_cluster_attribute(
                &cluster,
                name_or_id.as_str(),
                options.as_ref().and_then(|o| o.manufacturer_code),
            ) {
                if value.is_null() {
                    payload.push(json!({"attrId": attribute.ID, "status": Status::UNSUPPORTED_ATTRIBUTE as u8}));
                } else {
                    payload.push(json!({
                        "attrId": attribute.ID,
                        "attrData": value,
                        "dataType": attribute.r#type as u16,
                        "status": Status::SUCCESS as u8,
                    }));
                }
            } else if let Ok(id) = name_or_id.parse::<u16>() {
                let attr_data = value.get("value").cloned().unwrap_or(Value::Null);
                if attr_data.is_null() {
                    payload
                        .push(json!({"attrId": id, "status": Status::UNSUPPORTED_ATTRIBUTE as u8}));
                } else {
                    payload.push(json!({
                        "attrId": id,
                        "attrData": attr_data,
                        "dataType": value.get("type").and_then(Value::as_u64).unwrap_or(0),
                        "status": Status::SUCCESS as u8,
                    }));
                }
            } else {
                return Err(format!("Unknown attribute '{name_or_id}', specify either an existing attribute or a number"));
            }
        }
        let mut options = options.unwrap_or_default();
        options.direction = Some(Direction::SERVER_TO_CLIENT as u8);
        options.disable_default_response = Some(true);
        options.transaction_sequence_number = Some(transaction_sequence_number);
        self.zcl_command(
            Value::from(cluster.name.clone()),
            Value::from("readRsp"),
            Value::Array(payload),
            Some(options.into()),
            Some(attributes),
            false,
            FrameType::GLOBAL as u8,
            Some(cluster),
        )
        .await?;
        Ok(())
    }

    /// Upstream `writeResponse(...)`.
    pub async fn write_response(
        &self,
        cluster_key: Value,
        transaction_sequence_number: u8,
        attributes: Value,
        options: Option<Options>,
    ) -> Result<(), String> {
        if options
            .as_ref()
            .and_then(|o| o.transaction_sequence_number)
            .is_some()
        {
            return Err("Use parameter".to_string());
        }
        let cluster = self.get_cluster(
            cluster_key,
            None,
            options.as_ref().and_then(|o| o.manufacturer_code),
        );
        let mut payload = Vec::new();
        for (name_or_id, value) in attributes.as_object().cloned().unwrap_or_default() {
            let Some(status) = value.get("status").and_then(Value::as_u64) else {
                return Err("Missing attribute 'status'".to_string());
            };
            if let Some(attribute) = Utils::get_cluster_attribute(
                &cluster,
                name_or_id.as_str(),
                options.as_ref().and_then(|o| o.manufacturer_code),
            ) {
                payload.push(json!({"attrId": attribute.ID, "status": status}));
            } else if let Ok(id) = name_or_id.parse::<u16>() {
                payload.push(json!({"attrId": id, "status": status}));
            } else {
                return Err(format!(
                    "Unknown attribute '{name_or_id}', specify either an existing attribute or a number"
                ));
            }
        }
        let mut options = options.unwrap_or_default();
        options.direction = Some(Direction::SERVER_TO_CLIENT as u8);
        options.transaction_sequence_number = Some(transaction_sequence_number);
        self.zcl_command(
            Value::from(cluster.name.clone()),
            Value::from("writeRsp"),
            Value::Array(payload),
            Some(options),
            Some(attributes),
            false,
            FrameType::GLOBAL as u8,
            Some(cluster),
        )
        .await?;
        Ok(())
    }

    /// Upstream `updateSimpleDescriptor()`.
    pub async fn update_simple_descriptor(&self) -> Result<(), String> {
        let cluster_id = ClusterId::SIMPLE_DESCRIPTOR_REQUEST;
        let ieee = self.device_ieee_address.lock().unwrap().clone();
        let nwk = *self.device_network_address.lock().unwrap();
        let payload = BuffaloZdo::build_request(
            Entity::adapter().has_zdo_message_overhead(),
            cluster_id,
            &[json!(nwk), json!(self.ID)],
        );
        let response = Entity::adapter()
            .send_zdo(&ieee, nwk, cluster_id, &payload, false)
            .await?;
        if !BuffaloZdo::check_status(&response) {
            let code = response.get(0).and_then(Value::as_u64).unwrap_or(0) as u8;
            let name = crate::zspec::zdo::definition::status::Status::from_u8(code)
                .map(crate::zspec::zdo::definition::status::Status::name)
                .unwrap_or("UNKNOWN");
            return Err(format!("Status '{name}'"));
        }
        let descriptor = response.get(1).cloned().unwrap_or(Value::Null);
        *self.profile_id.lock().unwrap() = descriptor
            .get("profileId")
            .and_then(Value::as_u64)
            .map(|v| v as u16);
        *self.device_id.lock().unwrap() = descriptor
            .get("deviceId")
            .and_then(Value::as_u64)
            .map(|v| v as u16);
        *self.input_clusters.lock().unwrap() = descriptor
            .get("inClusterList")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_u64)
                    .map(|v| v as u16)
                    .collect()
            })
            .unwrap_or_default();
        *self.output_clusters.lock().unwrap() = descriptor
            .get("outClusterList")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_u64)
                    .map(|v| v as u16)
                    .collect()
            })
            .unwrap_or_default();
        Ok(())
    }

    /// Upstream `hasBind(clusterId, target)`.
    pub fn has_bind(&self, cluster_id: u16, target: &BindTarget) -> bool {
        self.get_bind_index(cluster_id, target) != -1
    }

    /// Upstream `getBindIndex(clusterId, target)`.
    pub fn get_bind_index(&self, cluster_id: u16, target: &BindTarget) -> i32 {
        let binds = self.binds();
        for (index, bind) in binds.iter().enumerate() {
            if bind.cluster.ID as u16 == cluster_id && &bind.target == target {
                return index as i32;
            }
        }
        -1
    }

    /// Upstream `addBinding(clusterKey, target)`.
    pub fn add_binding(&self, cluster_key: Value, target: BindTarget) -> Result<(), String> {
        let cluster = self.get_cluster(cluster_key, None, None);
        self.add_binding_internal(&cluster, &target)
    }

    fn add_binding_internal(&self, cluster: &Cluster, target: &BindTarget) -> Result<(), String> {
        if !self.has_bind(cluster.ID as u16, target) {
            let mut binds = self.binds.lock().unwrap();
            match target {
                BindTarget::Group(group) => binds.push(BindInternal {
                    cluster: cluster.ID as u16,
                    r#type: "group".to_string(),
                    device_ieee_address: None,
                    endpoint_id: None,
                    group_id: Some(group.group_id),
                }),
                BindTarget::Endpoint(endpoint) => binds.push(BindInternal {
                    cluster: cluster.ID as u16,
                    r#type: "endpoint".to_string(),
                    device_ieee_address: Some(endpoint.device_ieee_address.lock().unwrap().clone()),
                    endpoint_id: Some(endpoint.ID),
                    group_id: None,
                }),
            }
            drop(binds);
            self.save();
        }
        Ok(())
    }

    /// Upstream `bind(clusterKey, target)`.
    pub async fn bind(&self, cluster_key: Value, target: BindTarget) -> Result<(), String> {
        let cluster = self.get_cluster(cluster_key, None, None);
        let ieee = self.device_ieee_address.lock().unwrap().clone();
        let nwk = *self.device_network_address.lock().unwrap();
        let destination_address = match &target {
            BindTarget::Endpoint(endpoint) => endpoint.device_ieee_address.lock().unwrap().clone(),
            BindTarget::Group(group) => group.group_id.to_string(),
        };
        let log = format!(
            "Bind {ieee}/{} {} from '{}'",
            self.ID,
            cluster.name,
            match &target {
                BindTarget::Endpoint(endpoint) => format!("{destination_address}/{}", endpoint.ID),
                BindTarget::Group(group) => group.group_id.to_string(),
            }
        );
        logger().debug(&log, NS);
        let cluster_id = ClusterId::BIND_REQUEST;
        let (mode, dest_ieee, group_id, endpoint_id) = match &target {
            BindTarget::Endpoint(endpoint) => (
                UNICAST_BINDING,
                endpoint.device_ieee_address.lock().unwrap().clone(),
                0,
                endpoint.ID,
            ),
            BindTarget::Group(group) => (
                MULTICAST_BINDING,
                "0x0000000000000000".to_string(),
                group.group_id,
                0xff,
            ),
        };
        let payload = BuffaloZdo::build_request(
            Entity::adapter().has_zdo_message_overhead(),
            cluster_id,
            &[
                json!(ieee),
                json!(self.ID),
                json!(cluster.ID),
                json!(mode),
                json!(dest_ieee),
                json!(group_id),
                json!(endpoint_id),
            ],
        );
        let response = Entity::adapter()
            .send_zdo(&ieee, nwk, cluster_id, &payload, false)
            .await?;
        if !BuffaloZdo::check_status(&response) {
            let code = response.get(0).and_then(Value::as_u64).unwrap_or(0) as u8;
            let name = crate::zspec::zdo::definition::status::Status::from_u8(code)
                .map(crate::zspec::zdo::definition::status::Status::name)
                .unwrap_or("UNKNOWN");
            return Err(format!("{log} failed (Status '{name}')"));
        }
        self.add_binding_internal(&cluster, &target)
    }

    /// Upstream `save()`.
    pub fn save(&self) {
        self.get_device().save(true);
    }

    /// Upstream `unbind(clusterKey, target, force = false)`.
    pub async fn unbind(
        &self,
        cluster_key: Value,
        target: BindTarget,
        force: bool,
    ) -> Result<(), String> {
        let cluster = self.get_cluster(cluster_key, None, None);
        let ieee = self.device_ieee_address.lock().unwrap().clone();
        let nwk = *self.device_network_address.lock().unwrap();
        let action = format!("Unbind {ieee}/{} {}", self.ID, cluster.name);
        let index = self.get_bind_index(cluster.ID as u16, &target);
        if index == -1 && !force {
            logger().debug(&format!("{action} no bind present, skipping."), NS);
            return Ok(());
        }
        let (mode, dest_ieee, group_id, endpoint_id) = match &target {
            BindTarget::Endpoint(endpoint) => (
                UNICAST_BINDING,
                endpoint.device_ieee_address.lock().unwrap().clone(),
                0,
                endpoint.ID,
            ),
            BindTarget::Group(group) => (
                MULTICAST_BINDING,
                "0x0000000000000000".to_string(),
                group.group_id,
                0xff,
            ),
        };
        logger().debug(&action, NS);
        let cluster_id = ClusterId::UNBIND_REQUEST;
        let payload = BuffaloZdo::build_request(
            Entity::adapter().has_zdo_message_overhead(),
            cluster_id,
            &[
                json!(ieee),
                json!(self.ID),
                json!(cluster.ID),
                json!(mode),
                json!(dest_ieee),
                json!(group_id),
                json!(endpoint_id),
            ],
        );
        let response = Entity::adapter()
            .send_zdo(&ieee, nwk, cluster_id, &payload, false)
            .await?;
        if !BuffaloZdo::check_status(&response) {
            let code = response.get(0).and_then(Value::as_u64).unwrap_or(0) as u8;
            if code != crate::zspec::zdo::definition::status::Status::NO_ENTRY as u8 {
                let name = crate::zspec::zdo::definition::status::Status::from_u8(code)
                    .map(crate::zspec::zdo::definition::status::Status::name)
                    .unwrap_or("UNKNOWN");
                return Err(format!("{action} failed (Status '{name}')"));
            }
            logger().debug(
                &format!("{action} no entry on device, removing entry from database."),
                NS,
            );
        }
        if index != -1 {
            self.binds.lock().unwrap().remove(index as usize);
            self.save();
        }
        Ok(())
    }

    /// Upstream `defaultResponse(commandID, status, clusterID, transactionSequenceNumber, options?)`.
    pub async fn default_response(
        &self,
        command_id: u8,
        status: u8,
        cluster_id: u16,
        transaction_sequence_number: u8,
        options: Option<Options>,
    ) -> Result<(), String> {
        if options
            .as_ref()
            .and_then(|o| o.transaction_sequence_number)
            .is_some()
        {
            return Err("Use parameter".to_string());
        }
        let mut options = options.unwrap_or_default();
        options.direction = Some(Direction::SERVER_TO_CLIENT as u8);
        options.transaction_sequence_number = Some(transaction_sequence_number);
        self.zcl_command(
            Value::from(cluster_id),
            Value::from("defaultRsp"),
            json!({"cmdId": command_id, "statusCode": status}),
            Some(options),
            None,
            false,
            FrameType::GLOBAL as u8,
            None,
        )
        .await?;
        Ok(())
    }

    /// Upstream `configureReporting(clusterKey, items, options?)`.
    pub async fn configure_reporting(
        &self,
        cluster_key: Value,
        items: Vec<ConfigureReportingItem>,
        options: Option<Options>,
    ) -> Result<(), String> {
        let cluster = self.get_cluster(
            cluster_key,
            None,
            options.as_ref().and_then(|o| o.manufacturer_code),
        );
        let mut options = self.get_options_with_defaults(
            options,
            true,
            Direction::CLIENT_TO_SERVER as u8,
            cluster.manufacturerCode,
        );
        let keys = items
            .iter()
            .map(|item| item.attribute.clone())
            .collect::<Vec<_>>();
        options.manufacturer_code = self.ensure_manufacturer_code_is_unique_and_get(
            &cluster,
            keys.clone(),
            options.manufacturer_code,
            "configureReporting",
        )?;
        let mut payload = Vec::new();
        for item in &items {
            let (data_type, attr_id) = if let Some(object) = item.attribute.as_object() {
                (
                    object.get("type").and_then(Value::as_u64).unwrap_or(0) as u16,
                    object.get("ID").and_then(Value::as_u64).unwrap_or(0) as u16,
                )
            } else {
                let attribute = Utils::get_cluster_attribute(
                    &cluster,
                    item.attribute.as_str().unwrap_or_default(),
                    options.manufacturer_code,
                )
                .ok_or_else(|| {
                    format!(
                        "Invalid attribute '{}' for cluster '{}'",
                        item.attribute.as_str().unwrap_or_default(),
                        cluster.name
                    )
                })?;
                (attribute.r#type as u16, attribute.ID)
            };
            payload.push(json!({
                "direction": Direction::CLIENT_TO_SERVER as u8,
                "attrId": attr_id,
                "dataType": data_type,
                "minRepIntval": item.minimum_report_interval,
                "maxRepIntval": item.maximum_report_interval,
                "repChange": item.reportable_change,
            }));
        }
        self.zcl_command(
            Value::from(cluster.name.clone()),
            Value::from("configReport"),
            Value::Array(payload.clone()),
            Some(options.clone().into()),
            Some(json!(items
                .iter()
                .map(|item| item.attribute.clone())
                .collect::<Vec<_>>())),
            true,
            FrameType::GLOBAL as u8,
            Some(cluster.clone()),
        )
        .await?;
        {
            let mut reportings = self.configured_reportings.lock().unwrap();
            for entry in &payload {
                let attr_id = entry.get("attrId").and_then(Value::as_u64).unwrap_or(0) as u16;
                reportings.retain(|reporting| {
                    !(reporting.attr_id == attr_id
                        && reporting.cluster == cluster.ID as u16
                        && reporting.manufacturer_code == options.manufacturer_code)
                });
            }
            for entry in &payload {
                let max = entry
                    .get("maxRepIntval")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as u32;
                if max != 0xffff {
                    reportings.push(ConfiguredReportingInternal {
                        cluster: cluster.ID as u16,
                        attr_id: entry.get("attrId").and_then(Value::as_u64).unwrap_or(0) as u16,
                        min_rep_intval: entry
                            .get("minRepIntval")
                            .and_then(Value::as_u64)
                            .unwrap_or(0) as u32,
                        max_rep_intval: max,
                        rep_change: entry
                            .get("repChange")
                            .and_then(Value::as_f64)
                            .unwrap_or(0.0),
                        manufacturer_code: options.manufacturer_code,
                    });
                }
            }
        }
        self.save();
        Ok(())
    }

    /// Upstream `readReportingConfig(clusterKey, items, options?)`.
    pub async fn read_reporting_config(
        &self,
        cluster_key: Value,
        items: Vec<ReadReportConfigItem>,
        options: Option<Options>,
    ) -> Result<Value, String> {
        let cluster = self.get_cluster(
            cluster_key,
            None,
            options.as_ref().and_then(|o| o.manufacturer_code),
        );
        let mut options = self.get_options_with_defaults(
            options,
            true,
            Direction::CLIENT_TO_SERVER as u8,
            cluster.manufacturerCode,
        );
        let keys = items
            .iter()
            .map(|item| item.attribute.clone())
            .collect::<Vec<_>>();
        options.manufacturer_code = self.ensure_manufacturer_code_is_unique_and_get(
            &cluster,
            keys.clone(),
            options.manufacturer_code,
            "readReportingConfig",
        )?;
        let mut payload = Vec::new();
        for item in &items {
            if let Some(object) = item.attribute.as_object() {
                payload.push(json!({
                    "direction": item.direction.unwrap_or(Direction::CLIENT_TO_SERVER as u8),
                    "attrId": object.get("ID").and_then(Value::as_u64).unwrap_or(0),
                }));
            } else if let Some(attribute) = Utils::get_cluster_attribute(
                &cluster,
                item.attribute.as_str().unwrap_or_default(),
                options.manufacturer_code,
            ) {
                payload.push(json!({
                    "direction": item.direction.unwrap_or(Direction::CLIENT_TO_SERVER as u8),
                    "attrId": attribute.ID,
                }));
            } else {
                logger().warning(
                    &format!(
                        "Ignoring unknown attribute {} in cluster {}",
                        item.attribute.as_str().unwrap_or_default(),
                        cluster.name
                    ),
                    NS,
                );
            }
        }
        let frame = self
            .zcl_command(
                Value::from(cluster.name.clone()),
                Value::from("readReportConfig"),
                Value::Array(payload),
                Some(options.clone().into()),
                Some(json!(keys)),
                false,
                FrameType::GLOBAL as u8,
                Some(cluster.clone()),
            )
            .await?
            .ok_or_else(|| "No response received".to_string())?;
        self.save_cluster_attribute_report_config(
            frame.cluster.ID as u16,
            options.manufacturer_code,
            frame
                .payload
                .as_array()
                .map(|items| items.as_slice())
                .unwrap_or(&[]),
        );
        Ok(frame.payload)
    }

    /// Upstream `writeStructured(clusterKey, payload, options?)`.
    pub async fn write_structured(
        &self,
        cluster_key: Value,
        payload: Value,
        options: Option<Options>,
    ) -> Result<(), String> {
        self.zcl_command(
            cluster_key,
            Value::from("writeStructured"),
            payload,
            options,
            None,
            false,
            FrameType::GLOBAL as u8,
            None,
        )
        .await?;
        Ok(())
    }

    /// Upstream `command(clusterKey, commandKey, payload, options?)`.
    pub async fn command(
        &self,
        cluster_key: Value,
        command_key: Value,
        payload: Value,
        options: Option<Options>,
    ) -> Result<Option<Value>, String> {
        let frame = self
            .zcl_command(
                cluster_key,
                command_key,
                payload,
                options,
                None,
                false,
                FrameType::SPECIFIC as u8,
                None,
            )
            .await?;
        Ok(frame.map(|frame| frame.payload))
    }

    /// Upstream `commandResponse(clusterKey, commandKey, payload, options?, transactionSequenceNumber?)`.
    pub async fn command_response(
        &self,
        cluster_key: Value,
        command_key: Value,
        payload: Value,
        options: Option<Options>,
        transaction_sequence_number: Option<u8>,
    ) -> Result<(), String> {
        if options
            .as_ref()
            .and_then(|o| o.transaction_sequence_number)
            .is_some()
        {
            return Err("Use parameter".to_string());
        }
        let device = self.get_device();
        let custom_clusters = device.custom_clusters();
        let cluster = self.get_cluster(
            cluster_key,
            Some(device.clone()),
            options.as_ref().and_then(|o| o.manufacturer_code),
        );
        let command =
            Utils::get_cluster_command_response(&cluster, command_key.as_str().unwrap_or_default());
        let tsn = transaction_sequence_number.unwrap_or_else(zcl_transaction_sequence_number::next);
        let options = self.get_options_with_defaults(
            options,
            true,
            Direction::SERVER_TO_CLIENT as u8,
            cluster.manufacturerCode,
        );
        let frame = Frame::create_with_definitions(
            cluster.clone(),
            command.clone(),
            FrameType::SPECIFIC as u8,
            options.direction,
            options.disable_default_response,
            options.manufacturer_code,
            tsn,
            payload.clone(),
            options.reserved_bits,
        );
        logger().debug(
            &format!(
                "CommandResponse {}/{} {}.{}({payload}, {:?})",
                self.device_ieee_address.lock().unwrap(),
                self.ID,
                cluster.name,
                command.name,
                options
            ),
            NS,
        );
        if self.ID == 242 {
            self.send_request_with(frame, &options, {
                let cluster = cluster.clone();
                let command = command.clone();
                let payload = payload.clone();
                let reserved_bits = options.reserved_bits;
                let manufacturer_code = options.manufacturer_code;
                let tsn = tsn;
                move || {
                    let cluster = cluster.clone();
                    let command = command.clone();
                    let payload = payload.clone();
                    Box::pin(async move {
                        let frame = Frame::create_with_definitions(
                            cluster,
                            command,
                            FrameType::SPECIFIC as u8,
                            Direction::SERVER_TO_CLIENT as u8,
                            true,
                            manufacturer_code,
                            tsn,
                            payload,
                            reserved_bits,
                        );
                        Entity::adapter()
                            .send_zcl_frame_to_all(
                                242,
                                &frame,
                                242,
                                BroadcastAddress::RX_ON_WHEN_IDLE as u16,
                                None,
                            )
                            .await?;
                        Ok(EndpointRequestResult::Unit)
                    })
                }
            })
            .await?;
        } else {
            self.send_request(frame, &options).await?;
        }
        Ok(())
    }

    fn get_options_with_defaults(
        &self,
        options: Option<Options>,
        disable_default_response: bool,
        direction: u8,
        manufacturer_code: Option<u16>,
    ) -> OptionsWithDefaults {
        let options = options.unwrap_or_default();
        OptionsWithDefaults {
            timeout: options.timeout.unwrap_or(10000),
            disable_response: options.disable_response.unwrap_or(false),
            disable_recovery: options.disable_recovery.unwrap_or(false),
            disable_default_response: options
                .disable_default_response
                .unwrap_or(disable_default_response),
            direction: options.direction.unwrap_or(direction),
            src_endpoint: options.src_endpoint,
            reserved_bits: options.reserved_bits.unwrap_or(0),
            manufacturer_code: options.manufacturer_code.or(manufacturer_code),
            transaction_sequence_number: options.transaction_sequence_number,
            write_undiv: options.write_undiv.unwrap_or(false),
            send_policy: options.send_policy,
            profile_id: options.profile_id,
        }
    }

    fn ensure_manufacturer_code_is_unique_and_get(
        &self,
        cluster: &Cluster,
        attributes: Vec<Value>,
        fallback_manufacturer_code: Option<u16>,
        caller: &str,
    ) -> Result<Option<u16>, String> {
        let mut first_manufacturer_code = None;
        let mut code_set = false;
        for name_or_id in attributes {
            let attribute_id = if let Some(object) = name_or_id.as_object() {
                if let Some(attribute) = object.get("attribute") {
                    if attribute.as_object().is_some() {
                        if !code_set {
                            first_manufacturer_code = fallback_manufacturer_code;
                            code_set = true;
                        } else if first_manufacturer_code != fallback_manufacturer_code {
                            return Err(format!(
                                "Cannot have attributes with different manufacturerCode in single '{caller}' call"
                            ));
                        }
                        continue;
                    }
                    attribute.clone()
                } else {
                    name_or_id.clone()
                }
            } else {
                name_or_id
            };
            let attribute = match attribute_id {
                Value::String(name) => Utils::get_cluster_attribute(cluster, name.as_str(), None),
                Value::Number(number) => {
                    Utils::get_cluster_attribute(cluster, number.as_u64().unwrap_or(0) as u32, None)
                }
                _ => None,
            };
            let manufacturer_code = attribute
                .and_then(|attribute| attribute.manufacturerCode.or(fallback_manufacturer_code))
                .or(fallback_manufacturer_code);
            if !code_set {
                first_manufacturer_code = manufacturer_code;
                code_set = true;
            } else if first_manufacturer_code != manufacturer_code {
                return Err(format!(
                    "Cannot have attributes with different manufacturerCode in single '{caller}' call"
                ));
            }
        }
        Ok(first_manufacturer_code)
    }

    /// Upstream `addToGroup(group)`.
    pub async fn add_to_group(&self, group: &Group) -> Result<(), String> {
        let response = self
            .zcl_command(
                Value::from("genGroups"),
                Value::from("add"),
                json!({"groupid": group.group_id, "groupname": ""}),
                None,
                None,
                true,
                FrameType::SPECIFIC as u8,
                None,
            )
            .await?;
        if let Some(status) = response
            .as_ref()
            .and_then(|frame| frame.payload.get("status"))
            .and_then(Value::as_u64)
        {
            if status == Status::DUPLICATE_EXISTS as u64 {
                logger().info(
                    &format!(
                        "Group '{}' already exists on endpoint '{}/{}'",
                        group.group_id,
                        self.device_ieee_address.lock().unwrap(),
                        self.ID
                    ),
                    NS,
                );
            } else if status != Status::SUCCESS as u64 {
                let name = Status::from_u8(status as u8)
                    .map(Status::name)
                    .unwrap_or("UNKNOWN");
                return Err(format!("Status '{name}'"));
            }
        }
        group.add_member(&self.arc_self());
        Ok(())
    }

    /// Upstream `getCluster(...)`.
    fn get_cluster(
        &self,
        cluster_key: Value,
        device: Option<Arc<Device>>,
        manufacturer_code: Option<u16>,
    ) -> Cluster {
        let device = device.unwrap_or_else(|| self.get_device());
        Utils::get_cluster(
            key_from_value(&cluster_key),
            manufacturer_code.or(device.manufacturer_id()),
            &device.custom_clusters(),
        )
    }

    /// Upstream `removeFromGroup(group)`.
    pub async fn remove_from_group(&self, group: &Group) -> Result<(), String> {
        let response = self
            .zcl_command(
                Value::from("genGroups"),
                Value::from("remove"),
                json!({"groupid": group.group_id}),
                None,
                None,
                true,
                FrameType::SPECIFIC as u8,
                None,
            )
            .await?;
        let status = response
            .as_ref()
            .and_then(|frame| frame.payload.get("status"))
            .and_then(Value::as_u64);
        if status == Some(Status::NOT_FOUND as u64) {
            logger().info(
                &format!(
                    "Group '{}' was not found on endpoint '{}/{}'",
                    group.group_id,
                    self.device_ieee_address.lock().unwrap(),
                    self.ID
                ),
                NS,
            );
        } else if let Some(status) = status {
            if status != Status::SUCCESS as u64 {
                let name = Status::from_u8(status as u8)
                    .map(Status::name)
                    .unwrap_or("UNKNOWN");
                return Err(format!("Status '{name}'"));
            }
        }
        group.remove_member(&self.arc_self());
        Ok(())
    }

    /// Upstream `removeFromAllGroups()`.
    pub async fn remove_from_all_groups(&self) -> Result<(), String> {
        self.zcl_command(
            Value::from("genGroups"),
            Value::from("removeAll"),
            json!({}),
            Some(Options {
                disable_default_response: Some(true),
                ..Options::default()
            }),
            None,
            false,
            FrameType::SPECIFIC as u8,
            None,
        )
        .await?;
        self.remove_from_all_groups_database();
        Ok(())
    }

    /// Upstream `removeFromAllGroupsDatabase()`.
    pub fn remove_from_all_groups_database(&self) {
        for group in Group::all() {
            if group.has_member(&self.arc_self()) {
                group.remove_member(&self.arc_self());
            }
        }
    }

    /// Upstream `zclCommand(...)`.
    #[allow(clippy::too_many_arguments)]
    async fn zcl_command(
        &self,
        cluster_key: Value,
        command_key: Value,
        payload: Value,
        options: Option<Options>,
        log_payload: Option<Value>,
        check_status: bool,
        frame_type: u8,
        cluster_override: Option<Cluster>,
    ) -> Result<Option<Frame>, String> {
        let device = self.get_device();
        let cluster = cluster_override.unwrap_or_else(|| {
            self.get_cluster(
                cluster_key,
                Some(device.clone()),
                options.as_ref().and_then(|o| o.manufacturer_code),
            )
        });
        let command = if frame_type == FrameType::GLOBAL as u8 {
            Utils::get_global_command(key_from_command(&command_key))
        } else {
            Utils::get_cluster_command(&cluster, command_key.as_str().unwrap_or_default())
        };
        let has_response = if frame_type == FrameType::GLOBAL as u8 {
            true
        } else {
            command.response.is_some()
        };
        let options = self.get_options_with_defaults(
            options,
            has_response,
            Direction::CLIENT_TO_SERVER as u8,
            cluster.manufacturerCode,
        );
        let frame = Frame::create_with_definitions(
            cluster.clone(),
            command.clone(),
            frame_type,
            options.direction,
            options.disable_default_response,
            options.manufacturer_code,
            options
                .transaction_sequence_number
                .unwrap_or_else(zcl_transaction_sequence_number::next),
            payload.clone(),
            options.reserved_bits,
        );
        logger().debug(
            &format!(
                "ZCL command {}/{} {}.{}({}, {:?})",
                self.device_ieee_address.lock().unwrap(),
                self.ID,
                cluster.name,
                command.name,
                log_payload.unwrap_or_else(|| payload.clone()),
                options
            ),
            NS,
        );
        let result = self.send_request(frame, &options).await?;
        let frame = Frame::from_buffer(
            result.cluster_id,
            result.header,
            &result.data,
            &device.custom_clusters(),
        );
        if check_status && !options.disable_response {
            self.check_status(&frame.payload)?;
        }
        Ok(Some(frame))
    }

    /// Upstream `zclCommandBroadcast(...)`.
    #[allow(clippy::too_many_arguments)]
    pub async fn zcl_command_broadcast(
        &self,
        endpoint: u8,
        destination: BroadcastAddress,
        cluster_key: Value,
        command_key: Value,
        payload: Value,
        options: Option<Options>,
    ) -> Result<(), String> {
        let device = self.get_device();
        let cluster = self.get_cluster(
            cluster_key,
            Some(device.clone()),
            options.as_ref().and_then(|o| o.manufacturer_code),
        );
        let command =
            Utils::get_cluster_command(&cluster, command_key.as_str().unwrap_or_default());
        let options = self.get_options_with_defaults(
            options,
            true,
            Direction::CLIENT_TO_SERVER as u8,
            cluster.manufacturerCode,
        );
        let source_endpoint = options.src_endpoint.unwrap_or(self.ID);
        let frame = Frame::create_with_definitions(
            cluster.clone(),
            command.clone(),
            FrameType::SPECIFIC as u8,
            options.direction,
            true,
            options.manufacturer_code,
            options
                .transaction_sequence_number
                .unwrap_or_else(zcl_transaction_sequence_number::next),
            payload,
            options.reserved_bits,
        );
        logger().debug(
            &format!(
                "ZCL command broadcast {}/{} to {}/{} {}.{}",
                self.device_ieee_address.lock().unwrap(),
                source_endpoint,
                destination as u16,
                endpoint,
                cluster.name,
                command.name
            ),
            NS,
        );
        Entity::adapter()
            .send_zcl_frame_to_all(
                endpoint,
                &frame,
                source_endpoint,
                destination as u16,
                options.profile_id,
            )
            .await
    }
}

impl From<OptionsWithDefaults> for Options {
    fn from(options: OptionsWithDefaults) -> Self {
        Self {
            manufacturer_code: options.manufacturer_code,
            disable_default_response: Some(options.disable_default_response),
            disable_response: Some(options.disable_response),
            timeout: Some(options.timeout),
            direction: Some(options.direction),
            src_endpoint: options.src_endpoint,
            profile_id: options.profile_id,
            reserved_bits: Some(options.reserved_bits),
            transaction_sequence_number: options.transaction_sequence_number,
            disable_recovery: Some(options.disable_recovery),
            write_undiv: Some(options.write_undiv),
            send_policy: options.send_policy,
        }
    }
}

fn key_from_value(value: &Value) -> crate::zspec::zcl::utils::ClusterKey {
    match value {
        Value::String(name) => crate::zspec::zcl::utils::ClusterKey::Name(name.clone()),
        other => crate::zspec::zcl::utils::ClusterKey::Id(other.as_u64().unwrap_or(0) as u32),
    }
}

fn key_from_command(value: &Value) -> crate::zspec::zcl::utils::CommandKey {
    match value {
        Value::String(name) => crate::zspec::zcl::utils::CommandKey::Name(name.clone()),
        other => crate::zspec::zcl::utils::CommandKey::Id(other.as_u64().unwrap_or(0) as u32),
    }
}

fn bind_from_value(value: &Value) -> Option<BindInternal> {
    Some(BindInternal {
        cluster: value.get("cluster").and_then(Value::as_u64)? as u16,
        r#type: value.get("type").and_then(Value::as_str)?.to_string(),
        device_ieee_address: value
            .get("deviceIeeeAddress")
            .and_then(Value::as_str)
            .map(str::to_string),
        endpoint_id: value
            .get("endpointID")
            .and_then(Value::as_u64)
            .map(|v| v as u8),
        group_id: value
            .get("groupID")
            .and_then(Value::as_u64)
            .map(|v| v as u16),
    })
}

fn bind_to_value(bind: &BindInternal) -> Value {
    json!({
        "cluster": bind.cluster,
        "type": bind.r#type,
        "deviceIeeeAddress": bind.device_ieee_address,
        "endpointID": bind.endpoint_id,
        "groupID": bind.group_id,
    })
}
