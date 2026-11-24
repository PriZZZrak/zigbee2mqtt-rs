//! Versioned local module cache with hash/ABI verification, staging,
//! atomic activation and last-known-good rollback.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::manifest::{Manifest, ModuleRecord};
use crate::{ArtifactError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleState {
    Missing,
    Pending,
    Downloading,
    Verified,
    Ready,
    Failed(String),
}

#[derive(Debug)]
pub enum FetchError {
    Network(String),
    NotFound,
    TooLarge,
}

pub trait ModuleFetcher: Send + Sync {
    /// Fetch exactly one module. The caller verifies size/hash before activation.
    fn fetch(&self, module: &ModuleRecord) -> std::result::Result<Vec<u8>, FetchError>;
}

pub trait SpaceProbe: Send + Sync {
    fn available_bytes(&self) -> Option<u64>;
}

#[derive(Clone)]
pub struct CacheConfig {
    pub root: PathBuf,
    pub max_download_bytes: u64,
    pub max_cache_bytes: u64,
    pub max_concurrent_downloads: usize,
    pub space_probe: Option<Arc<dyn SpaceProbe>>,
}

impl CacheConfig {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            max_download_bytes: 16 << 20,
            max_cache_bytes: 64 << 20,
            max_concurrent_downloads: 2,
            space_probe: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct EnsureReport {
    pub modules: Vec<String>,
    pub network_bytes: u64,
    pub cache_bytes: u64,
    pub fetched: usize,
}

pub struct ModuleCache {
    root: PathBuf,
    manifest: Manifest,
    states: Mutex<HashMap<String, ModuleState>>,
    sequence: AtomicU64,
}

impl ModuleCache {
    pub fn open(config: CacheConfig, manifest: Manifest) -> Result<Self> {
        if manifest.manifest_schema != 1 {
            return Err(ArtifactError("unsupported manifest schema".to_owned()));
        }
        let root = config.root.join(&manifest.artifact_id);
        for directory in ["modules", "staging", "lkg"] {
            fs::create_dir_all(root.join(directory))
                .map_err(|error| ArtifactError(format!("cache {}: {error}", root.display())))?;
        }
        let cache = Self {
            root,
            manifest,
            states: Mutex::new(HashMap::new()),
            sequence: AtomicU64::new(1),
        };
        cache.load_state()?;
        Ok(cache)
    }

    pub fn artifact_id(&self) -> &str {
        &self.manifest.artifact_id
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub fn state(&self, module: &str) -> ModuleState {
        self.states
            .lock()
            .unwrap()
            .get(module)
            .cloned()
            .unwrap_or(ModuleState::Missing)
    }
    pub fn cache_bytes(&self) -> u64 {
        directory_bytes(&self.root.join("modules"))
    }

    pub fn module<'a>(&'a self, id: &str) -> Result<&'a ModuleRecord> {
        self.manifest
            .modules
            .entries
            .iter()
            .find(|module| module.id == id || module.file == id)
            .ok_or_else(|| ArtifactError(format!("module {id} not in trusted manifest")))
    }

    fn active_path(&self, id: &str) -> PathBuf {
        self.root.join("modules").join(format!("{id}.js"))
    }
    fn lkg_path(&self, id: &str) -> PathBuf {
        self.root.join("lkg").join(format!("{id}.js"))
    }

    fn load_state(&self) -> Result<()> {
        let path = self.root.join("state.json");
        if !path.is_file() {
            return Ok(());
        }
        let value: BTreeMap<String, String> = serde_json::from_slice(
            &fs::read(&path).map_err(|error| ArtifactError(format!("state: {error}")))?,
        )
        .map_err(|error| ArtifactError(format!("state JSON: {error}")))?;
        let mut states = self.states.lock().unwrap();
        for (id, value) in value {
            if value == "ready" {
                if self.verify_active(&id).is_ok() {
                    states.insert(id, ModuleState::Ready);
                } else {
                    states.insert(id, ModuleState::Failed("cached hash mismatch".to_owned()));
                }
            }
        }
        Ok(())
    }

    fn save_state(&self) -> Result<()> {
        let mut value = BTreeMap::new();
        for (id, state) in self.states.lock().unwrap().iter() {
            if *state == ModuleState::Ready {
                value.insert(id.clone(), "ready".to_owned());
            }
        }
        let path = self.root.join("state.json");
        let tmp = self.root.join("state.json.tmp");
        fs::write(&tmp, serde_json::to_vec(&value).unwrap())
            .map_err(|error| ArtifactError(format!("state write: {error}")))?;
        fs::rename(&tmp, &path)
            .map_err(|error| ArtifactError(format!("state activate: {error}")))?;
        Ok(())
    }

    fn verify_active(&self, id: &str) -> Result<Vec<u8>> {
        let bytes = fs::read(self.active_path(id))
            .map_err(|error| ArtifactError(format!("module {id}: {error}")))?;
        let module = self.module(id)?;
        self.manifest.verify_module(&module.id, &bytes)?;
        Ok(bytes)
    }

    /// Load a ready module without network. Verification always happens before
    /// returning bytes; corrupted cache entries are reported, never executed.
    pub fn load_ready(&self, id: &str) -> Result<Vec<u8>> {
        let bytes = self.verify_active(id)?;
        self.states
            .lock()
            .unwrap()
            .insert(id.to_owned(), ModuleState::Ready);
        Ok(bytes)
    }

    pub fn missing_closure(&self, ids: &[String]) -> Result<Vec<String>> {
        let mut required = BTreeSet::new();
        let mut stack: Vec<String> = ids.to_vec();
        while let Some(id) = stack.pop() {
            if !required.insert(id.clone()) {
                continue;
            }
            let module = self.module(&id)?;
            for dependency in &module.requires {
                let dependency_id = dependency.trim_end_matches(".js");
                stack.push(dependency_id.to_owned());
            }
        }
        let mut missing = Vec::new();
        for id in required {
            if self.verify_active(&id).is_err() {
                missing.push(id);
            }
        }
        Ok(missing)
    }

    /// Download the selected modules and their module dependencies only.
    /// Existing active modules are never replaced until every staged module is
    /// verified; activation is per-file rename with LKG restore on failure.
    pub fn ensure(
        &self,
        ids: &[String],
        fetcher: &dyn ModuleFetcher,
        config: &CacheConfig,
        allow_network: bool,
    ) -> Result<EnsureReport> {
        let mut required = BTreeSet::new();
        let mut stack: Vec<String> = ids.to_vec();
        while let Some(id) = stack.pop() {
            if !required.insert(id.clone()) {
                continue;
            }
            for dependency in &self.module(&id)?.requires {
                stack.push(dependency.trim_end_matches(".js").to_owned());
            }
        }
        let mut missing = Vec::new();
        for id in &required {
            if self.verify_active(id).is_err() {
                missing.push(id.clone());
            }
        }
        if missing.is_empty() {
            return Ok(EnsureReport {
                modules: required.into_iter().collect(),
                cache_bytes: self.cache_bytes(),
                ..Default::default()
            });
        }
        if !allow_network {
            for id in &missing {
                self.states
                    .lock()
                    .unwrap()
                    .insert(id.clone(), ModuleState::Failed("offline".to_owned()));
            }
            return Err(ArtifactError(format!(
                "offline cache miss: {}",
                missing.join(",")
            )));
        }
        let mut total_download = 0u64;
        for id in &missing {
            let module = self.module(id)?;
            if module.bytes > config.max_download_bytes {
                self.states.lock().unwrap().insert(
                    id.clone(),
                    ModuleState::Failed("module too large".to_owned()),
                );
                return Err(ArtifactError(format!(
                    "module {id} exceeds download budget"
                )));
            }
            total_download = total_download.saturating_add(module.bytes);
        }
        let projected = self.cache_bytes().saturating_add(total_download);
        if projected > config.max_cache_bytes {
            return Err(ArtifactError(format!(
                "cache budget exceeded: {projected} > {}",
                config.max_cache_bytes
            )));
        }
        if let Some(space) = config
            .space_probe
            .as_ref()
            .and_then(|probe| probe.available_bytes())
            && total_download > space
        {
            return Err(ArtifactError(format!(
                "insufficient space: need {total_download}, have {space}"
            )));
        }
        for id in &missing {
            self.states
                .lock()
                .unwrap()
                .insert(id.clone(), ModuleState::Pending);
        }

        let txn = format!(
            "txn-{}-{}",
            std::process::id(),
            self.sequence.fetch_add(1, Ordering::Relaxed)
        );
        let staging = self.root.join("staging").join(txn);
        fs::create_dir_all(&staging).map_err(|error| ArtifactError(format!("staging: {error}")))?;
        let mut fetched: Vec<String> = Vec::new();
        let mut network_bytes = 0u64;
        let max_concurrent = config.max_concurrent_downloads.max(1);
        let mut cursor = 0usize;
        while cursor < missing.len() {
            let end = (cursor + max_concurrent).min(missing.len());
            let chunk = &missing[cursor..end];
            let mut results = std::thread::scope(|scope| {
                let mut handles = Vec::new();
                for id in chunk {
                    let id = id.clone();
                    let module = self.module(&id)?.clone();
                    self.states
                        .lock()
                        .unwrap()
                        .insert(id.clone(), ModuleState::Downloading);
                    handles.push((id, scope.spawn(move || fetcher.fetch(&module))));
                }
                let mut results = Vec::new();
                for (id, handle) in handles {
                    results.push((id, handle.join().expect("fetcher thread")));
                }
                Ok::<_, ArtifactError>(results)
            })?;
            for (id, result) in results.drain(..) {
                let bytes = match result {
                    Ok(bytes) => bytes,
                    Err(FetchError::Network(error)) => {
                        self.states
                            .lock()
                            .unwrap()
                            .insert(id.clone(), ModuleState::Failed(error.clone()));
                        cleanup(&staging);
                        return Err(ArtifactError(format!("network fetch {id}: {error}")));
                    }
                    Err(FetchError::NotFound) => {
                        cleanup(&staging);
                        return Err(ArtifactError(format!("module {id} not found")));
                    }
                    Err(FetchError::TooLarge) => {
                        cleanup(&staging);
                        return Err(ArtifactError(format!("module {id} too large")));
                    }
                };
                network_bytes = network_bytes.saturating_add(bytes.len() as u64);
                let module = self.module(&id)?;
                // Verify before writing/activating; partial downloads never
                // replace an active module.
                if let Err(error) = self.manifest.verify_module(&module.id, &bytes) {
                    self.states
                        .lock()
                        .unwrap()
                        .insert(id.clone(), ModuleState::Failed(error.to_string()));
                    cleanup(&staging);
                    return Err(error);
                }
                let target = staging.join(format!("{id}.js"));
                let mut file = fs::File::create(&target)
                    .map_err(|error| ArtifactError(format!("stage {id}: {error}")))?;
                file.write_all(&bytes)
                    .map_err(|error| ArtifactError(format!("stage {id}: {error}")))?;
                file.sync_all().ok();
                self.states
                    .lock()
                    .unwrap()
                    .insert(id.clone(), ModuleState::Verified);
                fetched.push(id);
            }
            cursor = end;
        }

        // Atomic per-module activation; previous files are kept in lkg/.
        let mut activated: Vec<String> = Vec::new();
        let mut backup: Vec<String> = Vec::new();
        for id in &fetched {
            let active = self.active_path(id);
            let lkg = self.lkg_path(id);
            if active.is_file() {
                fs::copy(&active, &lkg)
                    .map_err(|error| ArtifactError(format!("lkg {id}: {error}")))?;
                backup.push(id.clone());
            }
            if let Err(error) = fs::rename(staging.join(format!("{id}.js")), &active) {
                for id in &activated {
                    let _ = fs::remove_file(self.active_path(id));
                }
                for id in &backup {
                    let _ = fs::rename(self.lkg_path(id), self.active_path(id));
                }
                cleanup(&staging);
                return Err(ArtifactError(format!("activate {id}: {error}")));
            }
            activated.push(id.clone());
            self.states
                .lock()
                .unwrap()
                .insert(id.clone(), ModuleState::Ready);
        }
        cleanup(&staging);
        self.save_state()?;
        Ok(EnsureReport {
            modules: required.into_iter().collect(),
            network_bytes,
            cache_bytes: self.cache_bytes(),
            fetched: fetched.len(),
        })
    }

    pub fn rollback(&self, id: &str) -> Result<()> {
        let lkg = self.lkg_path(id);
        if !lkg.is_file() {
            return Err(ArtifactError(format!("no last-known-good for {id}")));
        }
        let bytes = fs::read(&lkg).map_err(|error| ArtifactError(format!("lkg read: {error}")))?;
        self.manifest.verify_module(id, &bytes)?;
        fs::copy(&lkg, self.active_path(id))
            .map_err(|error| ArtifactError(format!("rollback {id}: {error}")))?;
        self.states
            .lock()
            .unwrap()
            .insert(id.to_owned(), ModuleState::Ready);
        self.save_state()
    }

    pub fn rollback_all(&self) -> Result<usize> {
        let mut count = 0;
        for entry in
            fs::read_dir(self.root.join("lkg")).map_err(|error| ArtifactError(error.to_string()))?
        {
            let entry = entry.map_err(|error| ArtifactError(error.to_string()))?;
            if let Some(name) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_suffix(".js"))
                && self.rollback(name).is_ok()
            {
                count += 1;
            }
        }
        Ok(count)
    }
}

fn cleanup(path: &Path) {
    let _ = fs::remove_dir_all(path);
}
fn directory_bytes(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata()
                && meta.is_file()
            {
                total += meta.len();
            }
        }
    }
    total
}
