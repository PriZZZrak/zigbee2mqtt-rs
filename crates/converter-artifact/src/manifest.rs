use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{ArtifactError, Result};

#[derive(Debug, Clone, Deserialize)]
pub struct SharedFile {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModuleRecord {
    pub id: String,
    pub file: String,
    pub bytes: u64,
    pub sha256: String,
    #[serde(default)]
    pub definitions: u64,
    #[serde(default)]
    pub requires: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    #[serde(rename = "manifestSchema")]
    pub manifest_schema: u32,
    #[serde(rename = "artifactId")]
    pub artifact_id: String,
    pub upstream: serde_json::Value,
    pub host: Host,
    pub index: IndexRecord,
    #[serde(default)]
    pub graph: Option<GraphRecord>,
    pub shared: Shared,
    pub modules: Modules,
    #[serde(default)]
    pub split: BTreeMap<String, serde_json::Value>,
    pub compatibility: Compatibility,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Host {
    pub abi: u32,
    pub engine: String,
    #[serde(rename = "engineVersion")]
    pub engine_version: String,
    #[serde(rename = "zclCatalogSha256")]
    pub zcl_catalog_sha256: String,
    #[serde(default)]
    pub required_features: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IndexRecord {
    pub file: String,
    pub format: String,
    pub version: u32,
    pub bytes: u64,
    pub sha256: String,
    #[serde(rename = "jsonBytes")]
    pub json_bytes: u64,
    #[serde(rename = "jsonSha256")]
    pub json_sha256: String,
    #[serde(rename = "lookupKeys")]
    pub lookup_keys: u64,
    pub definitions: u64,
    pub candidates: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphRecord {
    pub file: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Shared {
    pub root: String,
    pub files: Vec<SharedFile>,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Modules {
    pub format: String,
    #[serde(rename = "packageIncluded", default)]
    pub package_included: bool,
    pub entries: Vec<ModuleRecord>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Compatibility {
    #[serde(rename = "minHostAbi")]
    pub min_host_abi: u32,
    #[serde(rename = "maxHostAbi")]
    pub max_host_abi: u32,
}

#[derive(Debug)]
pub struct ManifestError(pub String);
impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ManifestError {}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn check_file(root: &Path, relative: &str, expected: &str, expected_bytes: u64) -> Result<()> {
    let path = root.join(relative);
    let bytes = std::fs::read(&path)
        .map_err(|error| ArtifactError(format!("{}: {error}", path.display())))?;
    if bytes.len() as u64 != expected_bytes {
        return Err(ArtifactError(format!(
            "{}: size {} != manifest {expected_bytes}",
            path.display(),
            bytes.len()
        )));
    }
    if sha256(&bytes) != expected {
        return Err(ArtifactError(format!(
            "{}: sha256 mismatch",
            path.display()
        )));
    }
    Ok(())
}

impl Manifest {
    pub fn load(path: &Path) -> std::result::Result<Self, ManifestError> {
        let bytes = std::fs::read(path)
            .map_err(|error| ManifestError(format!("{}: {error}", path.display())))?;
        serde_json::from_slice(&bytes)
            .map_err(|error| ManifestError(format!("manifest JSON: {error}")))
    }

    pub fn parse(bytes: &[u8]) -> std::result::Result<Self, ManifestError> {
        serde_json::from_slice(bytes)
            .map_err(|error| ManifestError(format!("manifest JSON: {error}")))
    }

    pub fn verify_compatibility(
        &self,
        host_abi: u32,
        engine: &str,
        engine_version: &str,
    ) -> Result<()> {
        if self.manifest_schema != 1 {
            return Err(ArtifactError(format!(
                "unsupported manifest schema {}",
                self.manifest_schema
            )));
        }
        if host_abi < self.compatibility.min_host_abi || host_abi > self.compatibility.max_host_abi
        {
            return Err(ArtifactError(format!(
                "host ABI {host_abi} outside manifest range {}..={}",
                self.compatibility.min_host_abi, self.compatibility.max_host_abi
            )));
        }
        if self.host.engine != engine || self.host.engine_version != engine_version {
            return Err(ArtifactError(format!(
                "manifest host {}/{} does not match {engine}/{engine_version}",
                self.host.engine, self.host.engine_version
            )));
        }
        Ok(())
    }

    pub fn verify_package(&self, root: &Path) -> Result<()> {
        check_file(root, &self.index.file, &self.index.sha256, self.index.bytes)?;
        if let Some(json) = self.graph.as_ref() {
            check_file(root, &json.file, &json.sha256, json.bytes)?;
        }
        for file in &self.shared.files {
            check_file(
                root,
                &format!("{}/{}", self.shared.root, file.path),
                &file.sha256,
                file.bytes,
            )?;
        }
        if tree_hash(&self.shared.files) != self.shared.sha256 {
            return Err(ArtifactError("shared tree hash mismatch".to_owned()));
        }
        Ok(())
    }

    pub fn verify_module(&self, module_id: &str, bytes: &[u8]) -> Result<&ModuleRecord> {
        let module = self
            .modules
            .entries
            .iter()
            .find(|module| module.id == module_id || module.file == module_id)
            .ok_or_else(|| ArtifactError(format!("module {module_id} not in manifest")))?;
        if bytes.len() as u64 != module.bytes {
            return Err(ArtifactError(format!("module {module_id}: size mismatch")));
        }
        if sha256(bytes) != module.sha256 {
            return Err(ArtifactError(format!(
                "module {module_id}: sha256 mismatch"
            )));
        }
        Ok(module)
    }
}

pub fn tree_hash(files: &[SharedFile]) -> String {
    let mut list = files.to_vec();
    list.sort_by(|a, b| a.path.cmp(&b.path));
    let mut data = Vec::new();
    for file in list {
        data.extend_from_slice(format!("{}\0{}\n", file.path, file.sha256).as_bytes());
    }
    sha256(&data)
}
