use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRecord {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegexSpec {
    pub source: String,
    pub flags: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EndpointFingerprint {
    #[serde(rename = "ID", skip_serializing_if = "Option::is_none")]
    pub id: Option<u8>,
    #[serde(rename = "profileID", skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<u16>,
    #[serde(rename = "deviceID", skip_serializing_if = "Option::is_none")]
    pub device_id: Option<u16>,
    #[serde(rename = "inputClusters", skip_serializing_if = "Option::is_none")]
    pub input_clusters: Option<Vec<u16>>,
    #[serde(rename = "outputClusters", skip_serializing_if = "Option::is_none")]
    pub output_clusters: Option<Vec<u16>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Fingerprint {
    #[serde(rename = "applicationVersion", skip_serializing_if = "Option::is_none")]
    pub application_version: Option<u32>,
    #[serde(rename = "manufacturerID", skip_serializing_if = "Option::is_none")]
    pub manufacturer_id: Option<u32>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(rename = "dateCode", skip_serializing_if = "Option::is_none")]
    pub date_code: Option<String>,
    #[serde(rename = "hardwareVersion", skip_serializing_if = "Option::is_none")]
    pub hardware_version: Option<u32>,
    #[serde(rename = "manufacturerName", skip_serializing_if = "Option::is_none")]
    pub manufacturer_name: Option<String>,
    #[serde(rename = "modelID", skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(rename = "powerSource", skip_serializing_if = "Option::is_none")]
    pub power_source: Option<String>,
    #[serde(rename = "softwareBuildID", skip_serializing_if = "Option::is_none")]
    pub software_build_id: Option<String>,
    #[serde(rename = "stackVersion", skip_serializing_if = "Option::is_none")]
    pub stack_version: Option<u32>,
    #[serde(rename = "zclVersion", skip_serializing_if = "Option::is_none")]
    pub zcl_version: Option<u32>,
    #[serde(rename = "ieeeAddr", skip_serializing_if = "Option::is_none")]
    pub ieee_addr: Option<RegexSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<Vec<EndpointFingerprint>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhiteLabel {
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub fingerprints: Vec<Fingerprint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Definition {
    pub file: u32,
    pub index: u32,
    pub model: String,
    pub vendor: String,
    pub description: String,
    pub version: String,
    #[serde(rename = "zigbeeModel", default)]
    pub zigbee_model: Vec<String>,
    #[serde(default)]
    pub fingerprints: Vec<Fingerprint>,
    #[serde(rename = "whiteLabel", default)]
    pub white_label: Vec<WhiteLabel>,
}

#[derive(Debug, Clone)]
pub struct ArtifactIndex {
    pub files: Vec<FileRecord>,
    pub definitions: Vec<Definition>,
    pub keys: BTreeMap<String, Vec<u32>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeviceType {
    Coordinator,
    Router,
    EndDevice,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceEndpoint {
    pub id: Option<u8>,
    pub profile_id: Option<u16>,
    pub device_id: Option<u16>,
    pub input_clusters: Vec<u16>,
    pub output_clusters: Vec<u16>,
}

#[derive(Debug, Clone, Default)]
pub struct DeviceProfile {
    pub ieee_addr: Option<String>,
    pub r#type: DeviceType,
    pub model_id: Option<String>,
    pub manufacturer_name: Option<String>,
    pub manufacturer_id: Option<u32>,
    pub power_source: Option<String>,
    pub application_version: Option<u32>,
    pub stack_version: Option<u32>,
    pub hardware_version: Option<u32>,
    pub zcl_version: Option<u32>,
    pub date_code: Option<String>,
    pub software_build_id: Option<String>,
    pub endpoints: Vec<DeviceEndpoint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub definition: Arc<Definition>,
    pub file: FileRecord,
    pub file_index: u32,
    pub model: String,
    pub vendor: String,
    pub description: String,
    pub version: String,
    pub white_label: bool,
}

impl ArtifactIndex {
    pub fn resolve(&self, device: &DeviceProfile) -> Option<Resolution> {
        crate::resolver::resolve(self, device)
    }
}
