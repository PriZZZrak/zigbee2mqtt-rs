//! Lazy converter definition loader: resolve only paired devices through the
//! fingerprint index and load only the selected module dependency closure.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::JoinHandle;

use serde_json::Value;

use crate::cache::{CacheConfig, ModuleCache, ModuleFetcher, ModuleState};
use crate::manifest::Manifest;
use crate::{ArtifactIndex, DeviceEndpoint, DeviceProfile, DeviceType, Resolution, from_binary};

#[derive(Clone)]
pub struct LazyConfig {
    pub artifact_dir: PathBuf,
    pub cache: CacheConfig,
    pub host_abi: u32,
    pub engine: String,
    pub engine_version: String,
}

#[derive(Debug, Clone)]
pub struct DeviceModule {
    pub file: String,
    pub index: u32,
    pub modules: Vec<(String, Vec<u8>)>,
}

#[derive(Debug, Clone, Default)]
pub struct StartupReport {
    pub devices: usize,
    pub resolved: usize,
    pub modules_loaded: Vec<String>,
    pub network_bytes: u64,
    pub cache_bytes: u64,
    pub failed: Vec<(String, String)>,
    pub states: BTreeMap<String, ModuleState>,
}

pub struct LazyLoader {
    index: ArtifactIndex,
    cache: ModuleCache,
    cache_config: CacheConfig,
}

impl LazyLoader {
    pub fn open(config: LazyConfig) -> crate::Result<Self> {
        let manifest = Manifest::load(&config.artifact_dir.join("manifest.json"))
            .map_err(|error| crate::ArtifactError(error.to_string()))?;
        manifest.verify_compatibility(config.host_abi, &config.engine, &config.engine_version)?;
        manifest.verify_package(&config.artifact_dir)?;
        let index = from_binary(
            &std::fs::read(config.artifact_dir.join(&manifest.index.file))
                .map_err(|error| crate::ArtifactError(format!("index: {error}")))?,
        )?;
        let cache = ModuleCache::open(config.cache.clone(), manifest)?;
        Ok(Self {
            index,
            cache,
            cache_config: config.cache,
        })
    }

    pub fn index(&self) -> &ArtifactIndex {
        &self.index
    }
    pub fn cache(&self) -> &ModuleCache {
        &self.cache
    }

    pub fn resolve(&self, device: &DeviceProfile) -> Option<Resolution> {
        self.index.resolve(device)
    }

    pub fn module_id(&self, resolution: &Resolution) -> crate::Result<String> {
        let wanted = resolution.file.path.trim_start_matches("dist/");
        self.cache
            .manifest()
            .modules
            .entries
            .iter()
            .find(|module| module.file.trim_start_matches("dist/") == wanted)
            .map(|module| module.id.clone())
            .ok_or_else(|| {
                crate::ArtifactError(format!(
                    "module for {} not in manifest",
                    resolution.file.path
                ))
            })
    }

    fn closure(&self, id: &str) -> crate::Result<Vec<String>> {
        let mut ordered = Vec::new();
        let mut visiting = BTreeSet::new();
        fn visit(
            cache: &ModuleCache,
            id: &str,
            visiting: &mut BTreeSet<String>,
            ordered: &mut Vec<String>,
        ) -> crate::Result<()> {
            if ordered.iter().any(|existing| existing == id) {
                return Ok(());
            }
            if !visiting.insert(id.to_owned()) {
                return Err(crate::ArtifactError(format!(
                    "module dependency cycle at {id}"
                )));
            }
            let module = cache.module(id)?;
            for dependency in &module.requires {
                visit(cache, dependency.trim_end_matches(".js"), visiting, ordered)?;
            }
            visiting.remove(id);
            ordered.push(id.to_owned());
            Ok(())
        }
        visit(&self.cache, id, &mut visiting, &mut ordered)?;
        Ok(ordered)
    }

    pub fn load_device(
        &self,
        device: &DeviceProfile,
        fetcher: &dyn ModuleFetcher,
        allow_network: bool,
    ) -> crate::Result<DeviceModule> {
        let resolution = self.resolve(device).ok_or_else(|| {
            crate::ArtifactError(format!("no definition for {:?}", device.model_id))
        })?;
        let module_id = self.module_id(&resolution)?;
        let closure = self.closure(&module_id)?;
        self.cache
            .ensure(&closure, fetcher, &self.cache_config, allow_network)?;
        let mut modules = Vec::new();
        for id in closure {
            modules.push((id.clone(), self.cache.load_ready(&id)?));
        }
        Ok(DeviceModule {
            file: resolution.file.path.clone(),
            index: resolution.file_index,
            modules,
        })
    }

    /// Startup uses the existing database list. Each device is prepared
    /// independently: a missing or corrupt module for one device does not make
    /// devices whose modules are already verified unavailable.
    pub fn startup(
        &self,
        database: &Path,
        fetcher: &dyn ModuleFetcher,
        allow_network: bool,
    ) -> crate::Result<StartupReport> {
        let records = read_database(database)?;
        let mut report = StartupReport {
            devices: records.len(),
            ..Default::default()
        };
        let mut loaded = BTreeSet::new();
        for record in &records {
            let Some(profile) = record.to_profile() else {
                report.failed.push((
                    record.ieee_addr.clone(),
                    "invalid device profile".to_owned(),
                ));
                continue;
            };
            let Some(resolution) = self.resolve(&profile) else {
                report
                    .failed
                    .push((record.ieee_addr.clone(), "definition not found".to_owned()));
                continue;
            };
            let module = match self.module_id(&resolution) {
                Ok(module) => module,
                Err(error) => {
                    report
                        .failed
                        .push((record.ieee_addr.clone(), error.to_string()));
                    continue;
                }
            };
            let closure = match self.closure(&module) {
                Ok(closure) => closure,
                Err(error) => {
                    report
                        .failed
                        .push((record.ieee_addr.clone(), error.to_string()));
                    continue;
                }
            };
            match self
                .cache
                .ensure(&closure, fetcher, &self.cache_config, allow_network)
            {
                Ok(result) => {
                    report.resolved += 1;
                    report.network_bytes =
                        report.network_bytes.saturating_add(result.network_bytes);
                    loaded.extend(result.modules);
                    loaded.insert(module.clone());
                }
                Err(error) => {
                    report
                        .failed
                        .push((record.ieee_addr.clone(), format!("{module}: {error}")));
                }
            }
        }
        report.cache_bytes = self.cache.cache_bytes();
        report.modules_loaded = loaded.into_iter().collect();
        for module in &report.modules_loaded {
            report
                .states
                .insert(module.clone(), self.cache.state(module));
        }
        Ok(report)
    }

    /// Materialize only the modules reported by startup into a runtime package
    /// root. The runtime sees the same dist layout as the artifact, while
    /// unselected device modules never appear on disk.
    pub fn materialize_runtime_package(
        &self,
        artifact_dir: &Path,
        report: &StartupReport,
        dest: &Path,
    ) -> crate::Result<()> {
        if dest.exists() {
            std::fs::remove_dir_all(dest).map_err(|error| {
                crate::ArtifactError(format!("runtime root {}: {error}", dest.display()))
            })?;
        }
        std::fs::create_dir_all(dest.join("dist/devices"))
            .map_err(|error| crate::ArtifactError(format!("runtime root: {error}")))?;
        let dist = artifact_dir.join("dist");
        std::fs::copy(dist.join("index.js"), dest.join("dist/index.js"))
            .map_err(|error| crate::ArtifactError(format!("runtime index.js: {error}")))?;
        if dist.join("models-index.json").is_file() {
            std::fs::copy(
                dist.join("models-index.json"),
                dest.join("dist/models-index.json"),
            )
            .map_err(|error| crate::ArtifactError(format!("runtime models-index: {error}")))?;
        }
        for directory in ["converters", "lib"] {
            copy_tree(&dist.join(directory), &dest.join("dist").join(directory))?;
        }
        let loaded: Vec<(String, Vec<u8>)> = report
            .modules_loaded
            .iter()
            .map(|id| Ok((id.clone(), self.cache.load_ready(id)?)))
            .collect::<crate::Result<_>>()?;
        self.materialize_modules(&loaded, dest)
    }

    /// Write verified module bytes into a runtime package root. Paths come from
    /// the trusted manifest only; the target must stay under `dest/dist`.
    /// Writes go through a temporary file plus rename so a concurrently running
    /// JS worker never observes a partially written module.
    pub fn materialize_modules(
        &self,
        modules: &[(String, Vec<u8>)],
        dest: &Path,
    ) -> crate::Result<()> {
        static WRITE_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        for (id, bytes) in modules {
            let module = self.cache.module(id)?;
            let relative = module.file.trim_start_matches("dist/");
            if relative.is_empty()
                || relative.starts_with('/')
                || relative.contains('\\')
                || relative.split('/').any(|part| part == ".." || part == ".")
            {
                return Err(crate::ArtifactError(format!(
                    "unsafe module path {relative}"
                )));
            }
            let target = dest.join("dist").join(relative);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|error| {
                    crate::ArtifactError(format!("runtime module dir: {error}"))
                })?;
            }
            let sequence = WRITE_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let temporary = target.with_extension(format!("{sequence}.tmp"));
            std::fs::write(&temporary, bytes)
                .map_err(|error| crate::ArtifactError(format!("runtime module {id}: {error}")))?;
            std::fs::rename(&temporary, &target).map_err(|error| {
                let _ = std::fs::remove_file(&temporary);
                crate::ArtifactError(format!("runtime module {id}: {error}"))
            })?;
        }
        Ok(())
    }

    pub fn pair(
        self: &Arc<Self>,
        device: DeviceProfile,
        fetcher: Arc<dyn ModuleFetcher>,
        allow_network: bool,
    ) -> JoinHandle<crate::Result<DeviceModule>> {
        let loader = Arc::clone(self);
        std::thread::spawn(move || loader.load_device(&device, fetcher.as_ref(), allow_network))
    }
}

#[derive(Debug, Clone)]
pub struct DatabaseRecord {
    pub ieee_addr: String,
    pub r#type: Option<String>,
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

impl DatabaseRecord {
    pub fn to_profile(&self) -> Option<DeviceProfile> {
        let r#type = match self.r#type.as_deref() {
            Some("Coordinator") => return None,
            Some("Router") => DeviceType::Router,
            Some("EndDevice") | None => DeviceType::EndDevice,
            Some(_) => DeviceType::Unknown,
        };
        Some(DeviceProfile {
            ieee_addr: Some(self.ieee_addr.clone()),
            r#type,
            model_id: self.model_id.clone(),
            manufacturer_name: self.manufacturer_name.clone(),
            manufacturer_id: self.manufacturer_id,
            power_source: self.power_source.clone(),
            application_version: self.application_version,
            stack_version: self.stack_version,
            hardware_version: self.hardware_version,
            zcl_version: self.zcl_version,
            date_code: self.date_code.clone(),
            software_build_id: self.software_build_id.clone(),
            endpoints: self.endpoints.clone(),
        })
    }
}

pub fn read_database(path: &Path) -> crate::Result<Vec<DatabaseRecord>> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| crate::ArtifactError(format!("database {}: {error}", path.display())))?;
    let mut records = Vec::new();
    for (line, raw) in text.lines().enumerate() {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(raw).map_err(|error| {
            crate::ArtifactError(format!("database line {}: {error}", line + 1))
        })?;
        if value.get("type").and_then(Value::as_str) == Some("Coordinator") {
            continue;
        }
        let Some(ieee_addr) = value
            .get("ieeeAddr")
            .and_then(Value::as_str)
            .map(str::to_owned)
        else {
            continue;
        };
        let mut endpoints = Vec::new();
        if let Some(map) = value.get("endpoints").and_then(Value::as_object) {
            for endpoint in map.values() {
                endpoints.push(DeviceEndpoint {
                    id: endpoint
                        .get("epId")
                        .and_then(Value::as_u64)
                        .map(|v| v as u8),
                    profile_id: endpoint
                        .get("profId")
                        .and_then(Value::as_u64)
                        .map(|v| v as u16),
                    device_id: endpoint
                        .get("devId")
                        .and_then(Value::as_u64)
                        .map(|v| v as u16),
                    input_clusters: endpoint
                        .get("inClusterList")
                        .and_then(Value::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(Value::as_u64)
                                .map(|v| v as u16)
                                .collect()
                        })
                        .unwrap_or_default(),
                    output_clusters: endpoint
                        .get("outClusterList")
                        .and_then(Value::as_array)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(Value::as_u64)
                                .map(|v| v as u16)
                                .collect()
                        })
                        .unwrap_or_default(),
                });
            }
        }
        records.push(DatabaseRecord {
            ieee_addr,
            r#type: value.get("type").and_then(Value::as_str).map(str::to_owned),
            model_id: value
                .get("modelId")
                .and_then(Value::as_str)
                .map(str::to_owned),
            manufacturer_name: value
                .get("manufName")
                .or_else(|| value.get("manufacturerName"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            manufacturer_id: value
                .get("manufId")
                .and_then(Value::as_u64)
                .map(|v| v as u32),
            power_source: value
                .get("powerSource")
                .and_then(Value::as_str)
                .map(str::to_owned),
            application_version: value
                .get("appVersion")
                .and_then(Value::as_u64)
                .map(|v| v as u32),
            stack_version: value
                .get("stackVersion")
                .and_then(Value::as_u64)
                .map(|v| v as u32),
            hardware_version: value
                .get("hwVersion")
                .and_then(Value::as_u64)
                .map(|v| v as u32),
            zcl_version: value
                .get("zclVersion")
                .and_then(Value::as_u64)
                .map(|v| v as u32),
            date_code: value
                .get("dateCode")
                .and_then(Value::as_str)
                .map(str::to_owned),
            software_build_id: value
                .get("swBuildId")
                .and_then(Value::as_str)
                .map(str::to_owned),
            endpoints,
        });
    }
    Ok(records)
}

fn copy_tree(source: &Path, target: &Path) -> crate::Result<()> {
    std::fs::create_dir_all(target)
        .map_err(|error| crate::ArtifactError(format!("copy {}: {error}", target.display())))?;
    for entry in std::fs::read_dir(source)
        .map_err(|error| crate::ArtifactError(format!("read {}: {error}", source.display())))?
    {
        let entry = entry.map_err(|error| crate::ArtifactError(error.to_string()))?;
        let destination = target.join(entry.file_name());
        if entry
            .file_type()
            .map_err(|error| crate::ArtifactError(error.to_string()))?
            .is_dir()
        {
            copy_tree(&entry.path(), &destination)?;
        } else if entry
            .path()
            .extension()
            .is_some_and(|ext| ext == "js" || ext == "json")
        {
            std::fs::copy(entry.path(), &destination).map_err(|error| {
                crate::ArtifactError(format!("copy {}: {error}", entry.path().display()))
            })?;
        }
    }
    Ok(())
}
