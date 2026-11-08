// Original: .upstream/zigbee-herdsman/src/controller/model/endpoint.ts
//
// Standalone Endpoint model: cluster attributes, bindings, groups and
// configured reportings. The device-level test model keeps its own lightweight
// Endpoint in `model::device`; this type mirrors the upstream public API.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

/// Upstream `BindInternal`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BindInternal {
    pub cluster: u16,
    /// `"endpoint"` or `"group"`.
    pub kind: String,
    pub device_ieee_address: Option<String>,
    pub endpoint_id: Option<u8>,
    pub group_id: Option<u64>,
}

/// Upstream `ConfiguredReportingInternal`.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ConfiguredReportingInternal {
    pub cluster: u16,
    pub attr_id: u16,
    pub min_rep_intval: u16,
    pub max_rep_intval: u16,
    pub rep_change: Value,
    pub manufacturer_code: Option<u16>,
}

/// Upstream `Endpoint`.
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub id: u8,
    pub device_ieee_address: String,
    pub device_network_address: u16,
    pub profile_id: Option<u16>,
    pub device_id: Option<u16>,
    pub input_clusters: Vec<u16>,
    pub output_clusters: Vec<u16>,
    /// cluster id -> attribute id -> value
    pub clusters: BTreeMap<u16, BTreeMap<u16, Value>>,
    pub binds: Vec<BindInternal>,
    pub configured_reportings: Vec<ConfiguredReportingInternal>,
    pub meta: Value,
    pub groups: BTreeSet<u64>,
    pending_requests: Vec<Value>,
}

impl Endpoint {
    pub fn new(
        id: u8,
        device_ieee_address: impl Into<String>,
        device_network_address: u16,
    ) -> Self {
        Self {
            id,
            device_ieee_address: device_ieee_address.into(),
            device_network_address,
            profile_id: Some(0x0104),
            device_id: None,
            input_clusters: Vec::new(),
            output_clusters: Vec::new(),
            clusters: BTreeMap::new(),
            binds: Vec::new(),
            configured_reportings: Vec::new(),
            meta: json!({}),
            groups: BTreeSet::new(),
            pending_requests: Vec::new(),
        }
    }

    pub fn get_input_clusters(&self) -> &[u16] {
        &self.input_clusters
    }

    pub fn get_output_clusters(&self) -> &[u16] {
        &self.output_clusters
    }

    pub fn supports_input_cluster(&self, cluster_id: u16) -> bool {
        self.input_clusters.contains(&cluster_id)
    }

    pub fn supports_output_cluster(&self, cluster_id: u16) -> bool {
        self.output_clusters.contains(&cluster_id)
    }

    pub fn get_cluster_attribute_value(
        &self,
        cluster_id: u16,
        attribute_id: u16,
    ) -> Option<&Value> {
        self.clusters.get(&cluster_id)?.get(&attribute_id)
    }

    pub fn set_cluster_attribute_value(
        &mut self,
        cluster_id: u16,
        attribute_id: u16,
        value: Value,
    ) {
        self.clusters
            .entry(cluster_id)
            .or_default()
            .insert(attribute_id, value);
    }

    /// Upstream `getDevice()` (identity of the owning device).
    pub fn get_device(&self) -> (&str, u16) {
        (&self.device_ieee_address, self.device_network_address)
    }

    /// Upstream `updateSimpleDescriptor(descriptor)`.
    pub fn update_simple_descriptor(&mut self, descriptor: &Value) {
        if let Some(profile) = descriptor.get("profileId").and_then(Value::as_u64) {
            self.profile_id = Some(profile as u16);
        }
        if let Some(device_id) = descriptor.get("deviceId").and_then(Value::as_u64) {
            self.device_id = Some(device_id as u16);
        }
        if let Some(clusters) = descriptor.get("inputClusters").and_then(Value::as_array) {
            self.input_clusters = clusters
                .iter()
                .filter_map(Value::as_u64)
                .map(|id| id as u16)
                .collect();
        }
        if let Some(clusters) = descriptor.get("outputClusters").and_then(Value::as_array) {
            self.output_clusters = clusters
                .iter()
                .filter_map(Value::as_u64)
                .map(|id| id as u16)
                .collect();
        }
    }

    pub fn add_binding(&mut self, binding: BindInternal) {
        if !self.binds.contains(&binding) {
            self.binds.push(binding);
        }
    }

    pub fn get_bind_index(&self, binding: &BindInternal) -> Option<usize> {
        self.binds.iter().position(|candidate| candidate == binding)
    }

    pub fn has_bind(&self, binding: &BindInternal) -> bool {
        self.get_bind_index(binding).is_some()
    }

    /// Upstream `bind(cluster, target)` / `unbind`.
    pub fn bind(&mut self, binding: BindInternal) {
        self.add_binding(binding);
    }

    pub fn unbind(&mut self, binding: &BindInternal) {
        self.binds.retain(|candidate| candidate != binding);
    }

    pub fn clear_bindings(&mut self) {
        self.binds.clear();
    }

    pub fn add_to_group(&mut self, group_id: u64) {
        self.groups.insert(group_id);
    }

    pub fn remove_from_group(&mut self, group_id: u64) {
        self.groups.remove(&group_id);
    }

    pub fn remove_from_all_groups(&mut self) {
        self.groups.clear();
    }

    /// Upstream `removeFromAllGroupsDatabase()` returns the removed ids.
    pub fn remove_from_all_groups_database(&mut self) -> Vec<u64> {
        let groups = self.groups.iter().copied().collect();
        self.groups.clear();
        groups
    }

    pub fn has_pending_requests(&self) -> bool {
        !self.pending_requests.is_empty()
    }

    pub fn push_pending_request(&mut self, request: Value) {
        self.pending_requests.push(request);
    }

    /// Upstream `sendPendingRequests()`: drain pending requests.
    pub fn send_pending_requests(&mut self) -> Vec<Value> {
        std::mem::take(&mut self.pending_requests)
    }

    /// Upstream `toDatabaseRecord()`.
    pub fn to_database_record(&self) -> Value {
        json!({
            "id": self.id,
            "type": "endpoint",
            "deviceIeeeAddress": self.device_ieee_address,
            "deviceNetworkAddress": self.device_network_address,
            "profileID": self.profile_id,
            "deviceID": self.device_id,
            "inputClusters": self.input_clusters,
            "outputClusters": self.output_clusters,
            "clusters": self.clusters,
            "binds": self.binds,
            "configuredReportings": self.configured_reportings,
            "meta": self.meta,
        })
    }

    pub fn save(&self) -> Value {
        self.to_database_record()
    }

    pub fn save_bindings(&self) -> Value {
        json!({"id": self.id, "type": "endpoint", "binds": self.binds})
    }

    pub fn save_cluster_attribute_key_value(&self) -> Value {
        json!({"id": self.id, "type": "endpoint", "clusters": self.clusters})
    }

    pub fn save_cluster_attribute_report_config(&self) -> Value {
        json!({"id": self.id, "type": "endpoint", "configuredReportings": self.configured_reportings})
    }

    /// Upstream `defaultResponse(cmdId, status, ...)` result payload.
    pub fn default_response(&self, cmd_id: u8, status: u8, cluster_id: u16, tsn: u8) -> Value {
        json!({
            "cmdId": cmd_id,
            "statusCode": status,
            "clusterId": cluster_id,
            "transactionSequenceNumber": tsn,
        })
    }
}
