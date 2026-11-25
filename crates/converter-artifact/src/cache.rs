//! Versioned, crash-consistent module cache with immutable generations and an
//! atomically replaced active pointer.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::json;

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
    fn fetch(&self, module: &ModuleRecord) -> std::result::Result<Vec<u8>, FetchError>;
}

pub trait SpaceProbe: Send + Sync {
    fn available_bytes(&self) -> Option<u64>;
}

#[derive(Clone)]
pub struct CacheConfig {
    pub root: PathBuf,
    /// Per-module download cap.
    pub max_download_bytes: u64,
    /// Total on-disk cap for generations, staging and last-known-good.
    pub max_cache_bytes: u64,
    /// Peak in-memory cap for the concurrently fetched chunk.
    pub max_memory_bytes: u64,
    pub max_concurrent_downloads: usize,
    pub space_probe: Option<Arc<dyn SpaceProbe>>,
}

impl CacheConfig {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            max_download_bytes: 16 << 20,
            max_cache_bytes: 64 << 20,
            max_memory_bytes: 16 << 20,
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

#[derive(Debug, Clone)]
struct Generation {
    name: String,
    modules: BTreeMap<String, String>, // module id -> sha256
}

impl Generation {
    fn empty() -> Self {
        Self {
            name: "gen-0".to_owned(),
            modules: BTreeMap::new(),
        }
    }
}

pub struct ModuleCache {
    root: PathBuf,
    manifest: Manifest,
    states: Mutex<HashMap<String, ModuleState>>,
    active: Mutex<Generation>,
    sequence: AtomicU64,
    ensure_lock: Mutex<()>,
}

impl ModuleCache {
    pub fn open(config: CacheConfig, manifest: Manifest) -> Result<Self> {
        if manifest.manifest_schema != 1 {
            return Err(ArtifactError("unsupported manifest schema".to_owned()));
        }
        let root = config.root.join(&manifest.artifact_id);
        for directory in ["generations", "staging"] {
            fs::create_dir_all(root.join(directory))
                .map_err(|error| ArtifactError(format!("cache {}: {error}", root.display())))?;
        }
        let cache = Self {
            root,
            manifest,
            states: Mutex::new(HashMap::new()),
            active: Mutex::new(Generation::empty()),
            sequence: AtomicU64::new(1),
            ensure_lock: Mutex::new(()),
        };
        // Recover only from complete, verified generations. A staging directory
        // is never active; stale staging is removed.
        let active = cache
            .read_pointer("active.json")
            .and_then(|generation| cache.verify_generation(generation));
        let active = match active {
            Some(generation) => generation,
            None => {
                if let Some(lkg) = cache
                    .read_pointer("lkg.json")
                    .and_then(|generation| cache.verify_generation(generation))
                {
                    cache.write_pointer("active.json", &lkg)?;
                    lkg
                } else {
                    Generation::empty()
                }
            }
        };
        if let Ok(entries) = fs::read_dir(cache.root.join("staging")) {
            for entry in entries.flatten() {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
        {
            let mut states = cache.states.lock().unwrap();
            for id in active.modules.keys() {
                states.insert(id.clone(), ModuleState::Ready);
            }
        }
        *cache.active.lock().unwrap() = active;
        cache.prune_generations()?;
        Ok(cache)
    }

    /// Cache locks are data-structure guards only; a panicking fetcher must not
    /// poison the cache for every later device.
    fn lock_states(&self) -> std::sync::MutexGuard<'_, HashMap<String, ModuleState>> {
        self.states
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn lock_active(&self) -> std::sync::MutexGuard<'_, Generation> {
        self.active
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
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
        self.lock_states()
            .get(module)
            .cloned()
            .unwrap_or(ModuleState::Missing)
    }
    pub fn cache_bytes(&self) -> u64 {
        ["generations", "staging"]
            .iter()
            .map(|directory| directory_bytes(&self.root.join(directory)))
            .sum()
    }
    pub fn module<'a>(&'a self, id: &str) -> Result<&'a ModuleRecord> {
        self.manifest
            .modules
            .entries
            .iter()
            .find(|module| module.id == id || module.file == id)
            .ok_or_else(|| ArtifactError(format!("module {id} not in trusted manifest")))
    }
    pub fn active_module_path(&self, id: &str) -> Option<PathBuf> {
        let active = self.lock_active();
        active
            .modules
            .get(id)
            .map(|_| self.generation_path(&active.name, id))
    }
    fn generation_path(&self, generation: &str, id: &str) -> PathBuf {
        self.root
            .join("generations")
            .join(generation)
            .join(format!("{id}.js"))
    }
    fn pointer_path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn read_pointer(&self, name: &str) -> Option<Generation> {
        let bytes = fs::read(self.pointer_path(name)).ok()?;
        let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
        let name = value.get("generation")?.as_str()?.to_owned();
        let modules = value.get("modules")?.as_object()?;
        let mut map = BTreeMap::new();
        for (id, sha) in modules {
            map.insert(id.clone(), sha.as_str()?.to_owned());
        }
        Some(Generation { name, modules: map })
    }

    fn verify_generation(&self, generation: Generation) -> Option<Generation> {
        for (id, expected) in &generation.modules {
            let bytes = fs::read(self.generation_path(&generation.name, id)).ok()?;
            let module = self.module(id).ok()?;
            if module.sha256 != *expected
                || module.bytes != bytes.len() as u64
                || self.manifest.verify_module(id, &bytes).is_err()
            {
                return None;
            }
        }
        Some(generation)
    }

    fn write_pointer(&self, name: &str, generation: &Generation) -> Result<()> {
        let path = self.pointer_path(name);
        let tmp = self.pointer_path(&format!("{name}.tmp"));
        let modules: BTreeMap<&String, &String> = generation.modules.iter().collect();
        let value = json!({"generation": generation.name, "modules": modules});
        let mut file = fs::File::create(&tmp)
            .map_err(|error| ArtifactError(format!("pointer {name}: {error}")))?;
        file.write_all(&serde_json::to_vec(&value).unwrap())
            .map_err(|error| ArtifactError(error.to_string()))?;
        file.sync_all().ok();
        fs::rename(&tmp, &path)
            .map_err(|error| ArtifactError(format!("activate pointer {name}: {error}")))?;
        // The rename is the commit point; sync the directory so a power loss
        // cannot resurrect the previous pointer.
        if let Ok(directory) = fs::File::open(&self.root) {
            let _ = directory.sync_all();
        }
        Ok(())
    }

    fn verify_active(&self, id: &str) -> Result<Vec<u8>> {
        let active = self.lock_active().clone();
        let Some(expected) = active.modules.get(id) else {
            return Err(ArtifactError(format!(
                "module {id}: not in active generation"
            )));
        };
        let bytes = fs::read(self.generation_path(&active.name, id))
            .map_err(|error| ArtifactError(format!("module {id}: {error}")))?;
        if expected != &self.module(id)?.sha256 || bytes.len() as u64 != self.module(id)?.bytes {
            return Err(ArtifactError(format!(
                "module {id}: cache metadata mismatch"
            )));
        }
        self.manifest.verify_module(id, &bytes)?;
        Ok(bytes)
    }

    pub fn load_ready(&self, id: &str) -> Result<Vec<u8>> {
        // A concurrent commit may have pruned the generation between reading
        // the active pointer and reading the file; retry once against the new
        // pointer before reporting a cache miss.
        let bytes = match self.verify_active(id) {
            Ok(bytes) => bytes,
            Err(_) => self.verify_active(id)?,
        };
        self.lock_states().insert(id.to_owned(), ModuleState::Ready);
        Ok(bytes)
    }

    fn required_closure(&self, ids: &[String]) -> Result<BTreeSet<String>> {
        let mut required = BTreeSet::new();
        let mut stack = ids.to_vec();
        while let Some(id) = stack.pop() {
            if !required.insert(id.clone()) {
                continue;
            }
            for dependency in &self.module(&id)?.requires {
                stack.push(dependency.trim_end_matches(".js").to_owned());
            }
        }
        Ok(required)
    }

    pub fn missing_closure(&self, ids: &[String]) -> Result<Vec<String>> {
        let required = self.required_closure(ids)?;
        Ok(required
            .into_iter()
            .filter(|id| self.verify_active(id).is_err())
            .collect())
    }

    /// Single-flight, crash-consistent ensure. The fast path avoids the lock
    /// for fully cached devices; the lock serializes budget and generation
    /// activation across concurrent ensure calls.
    pub fn ensure(
        &self,
        ids: &[String],
        fetcher: &dyn ModuleFetcher,
        config: &CacheConfig,
        allow_network: bool,
    ) -> Result<EnsureReport> {
        let required = self.required_closure(ids)?;
        if required.iter().all(|id| self.verify_active(id).is_ok()) {
            return Ok(EnsureReport {
                modules: required.into_iter().collect(),
                cache_bytes: self.cache_bytes(),
                ..Default::default()
            });
        }
        if !allow_network {
            for id in required {
                self.lock_states()
                    .insert(id, ModuleState::Failed("offline".to_owned()));
            }
            return Err(ArtifactError("offline cache miss".to_owned()));
        }
        let _guard = self
            .ensure_lock
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        // Another ensure may have completed while we waited for the lock.
        let required = self.required_closure(ids)?;
        let missing: Vec<String> = required
            .iter()
            .filter(|id| self.verify_active(id).is_err())
            .cloned()
            .collect();
        if missing.is_empty() {
            return Ok(EnsureReport {
                modules: required.into_iter().collect(),
                cache_bytes: self.cache_bytes(),
                ..Default::default()
            });
        }
        let mut download = 0u64;
        for id in &missing {
            let module = self.module(id)?;
            if module.bytes > config.max_download_bytes {
                self.lock_states().insert(
                    id.clone(),
                    ModuleState::Failed("module too large".to_owned()),
                );
                return Err(ArtifactError(format!(
                    "module {id} exceeds download budget"
                )));
            }
            download += module.bytes;
        }
        // The transaction copies the active generation into staging before it
        // adds the missing modules, so the peak footprint is
        // `current + active_bytes + download`; only after the pointer swap are
        // obsolete generations pruned. Refuse before writing anything.
        let current = self.cache_bytes();
        let active = self.lock_active().clone();
        let active_bytes: u64 = active
            .modules
            .keys()
            .filter_map(|id| self.module(id).ok())
            .map(|module| module.bytes)
            .sum();
        let peak = current
            .saturating_add(active_bytes)
            .saturating_add(download);
        if peak > config.max_cache_bytes {
            for id in &missing {
                self.lock_states().insert(
                    id.clone(),
                    ModuleState::Failed("cache budget exceeded".to_owned()),
                );
            }
            return Err(ArtifactError(format!(
                "cache budget exceeded: peak {peak} > {}",
                config.max_cache_bytes
            )));
        }
        if let Some(space) = config
            .space_probe
            .as_ref()
            .and_then(|probe| probe.available_bytes())
            && peak > space
        {
            for id in &missing {
                self.lock_states().insert(
                    id.clone(),
                    ModuleState::Failed("insufficient space".to_owned()),
                );
            }
            return Err(ArtifactError(format!(
                "insufficient space: need {peak}, have {space}"
            )));
        }
        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);
        let generation_name = format!("gen-{}-{sequence}", std::process::id());
        let staging = self.root.join("staging").join(&generation_name);
        fs::create_dir_all(&staging).map_err(|error| ArtifactError(format!("staging: {error}")))?;
        // Copy the current active generation so the new generation is immutable
        // and never mutates files referenced by the active pointer.
        for id in active.modules.keys() {
            fs::copy(
                self.generation_path(&active.name, id),
                staging.join(format!("{id}.js")),
            )
            .map_err(|error| ArtifactError(format!("copy active {id}: {error}")))?;
        }
        for id in &missing {
            self.lock_states().insert(id.clone(), ModuleState::Pending);
        }
        let mut fetched = Vec::new();
        let mut network_bytes = 0u64;
        let max_concurrent = config.max_concurrent_downloads.max(1);
        let mut cursor = 0usize;
        while cursor < missing.len() {
            let end = (cursor + max_concurrent).min(missing.len());
            let chunk = &missing[cursor..end];
            let chunk_bytes: u64 = chunk
                .iter()
                .filter_map(|id| self.module(id).ok())
                .map(|module| module.bytes)
                .sum();
            if chunk_bytes > config.max_memory_bytes {
                let id = chunk.first().cloned().unwrap_or_default();
                self.fail(&staging, &missing, &id, "memory budget exceeded".to_owned());
                return Err(ArtifactError(format!(
                    "memory budget exceeded: chunk {chunk_bytes} > {}",
                    config.max_memory_bytes
                )));
            }
            let mut results = std::thread::scope(|scope| {
                let mut handles = Vec::new();
                for id in chunk {
                    let id = id.clone();
                    let module = self.module(&id)?.clone();
                    self.lock_states()
                        .insert(id.clone(), ModuleState::Downloading);
                    handles.push((
                        id,
                        scope.spawn(move || {
                            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                fetcher.fetch(&module)
                            }))
                            .unwrap_or_else(|_| {
                                Err(FetchError::Network("fetcher panicked".to_owned()))
                            })
                        }),
                    ));
                }
                Ok::<_, ArtifactError>(
                    handles
                        .into_iter()
                        .map(|(id, handle)| (id, handle.join().expect("fetcher thread")))
                        .collect::<Vec<_>>(),
                )
            })?;
            for (id, result) in results.drain(..) {
                let bytes = match result {
                    Ok(bytes) => bytes,
                    Err(FetchError::Network(error)) => {
                        self.fail(&staging, &missing, &id, error.clone());
                        return Err(ArtifactError(format!("network fetch {id}: {error}")));
                    }
                    Err(FetchError::NotFound) => {
                        self.fail(&staging, &missing, &id, "not found".to_owned());
                        return Err(ArtifactError(format!("module {id} not found")));
                    }
                    Err(FetchError::TooLarge) => {
                        self.fail(&staging, &missing, &id, "too large".to_owned());
                        return Err(ArtifactError(format!("module {id} too large")));
                    }
                };
                network_bytes += bytes.len() as u64;
                if let Err(error) = self.manifest.verify_module(&id, &bytes) {
                    self.fail(&staging, &missing, &id, error.to_string());
                    return Err(error);
                }
                fs::write(staging.join(format!("{id}.js")), &bytes).map_err(|error| {
                    self.fail(&staging, &missing, &id, error.to_string());
                    ArtifactError(format!("stage {id}: {error}"))
                })?;
                self.lock_states().insert(id.clone(), ModuleState::Verified);
                fetched.push(id);
            }
            cursor = end;
        }
        let mut modules = active.modules.clone();
        for id in &missing {
            modules.insert(id.clone(), self.module(id)?.sha256.clone());
        }
        let generation = Generation {
            name: generation_name.clone(),
            modules,
        };
        // Move the complete immutable generation into place; the active pointer
        // is the only commit point. A crash before the pointer leaves an orphan
        // generation that open() ignores.
        let final_dir = self.root.join("generations").join(&generation_name);
        fs::rename(&staging, &final_dir)
            .map_err(|error| ArtifactError(format!("publish generation: {error}")))?;
        if self.verify_generation(generation.clone()).is_none() {
            let _ = fs::remove_dir_all(&final_dir);
            return Err(ArtifactError(
                "staged generation verification failed".to_owned(),
            ));
        }
        if !active.modules.is_empty() {
            self.write_pointer("lkg.json", &active)?;
        }
        if let Err(error) = self.write_pointer("active.json", &generation) {
            let _ = fs::remove_dir_all(&final_dir);
            return Err(error);
        }
        *self.lock_active() = generation;
        for id in &missing {
            self.lock_states().insert(id.clone(), ModuleState::Ready);
        }
        // Retain only the active and last-known-good generations.
        self.prune_generations()?;
        Ok(EnsureReport {
            modules: required.into_iter().collect(),
            network_bytes,
            cache_bytes: self.cache_bytes(),
            fetched: fetched.len(),
        })
    }

    /// Abort a transaction without a commit point: the staging generation is
    /// discarded, every module in the closure is reported as failed, and the
    /// previously active generation keeps serving.
    fn fail(&self, staging: &Path, missing: &[String], id: &str, reason: String) {
        let mut states = self.lock_states();
        for other in missing {
            if other != id {
                states.insert(
                    other.clone(),
                    ModuleState::Failed("transaction aborted".to_owned()),
                );
            }
        }
        states.insert(id.to_owned(), ModuleState::Failed(reason));
        drop(states);
        let _ = fs::remove_dir_all(staging);
    }

    fn prune_generations(&self) -> Result<()> {
        let active = self.lock_active().name.clone();
        let lkg = self
            .read_pointer("lkg.json")
            .map(|generation| generation.name);
        for entry in fs::read_dir(self.root.join("generations"))
            .map_err(|error| ArtifactError(error.to_string()))?
        {
            let entry = entry.map_err(|error| ArtifactError(error.to_string()))?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name != active && Some(&name) != lkg.as_ref() {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
        Ok(())
    }

    pub fn rollback(&self, id: &str) -> Result<()> {
        let Some(lkg) = self
            .read_pointer("lkg.json")
            .and_then(|generation| self.verify_generation(generation))
        else {
            return Err(ArtifactError(format!(
                "no last-known-good generation for {id}"
            )));
        };
        let current = self.lock_active().clone();
        self.write_pointer("active.json", &lkg)?;
        if !current.modules.is_empty() {
            self.write_pointer("lkg.json", &current)?;
        }
        *self.lock_active() = lkg;
        self.lock_states().insert(id.to_owned(), ModuleState::Ready);
        Ok(())
    }
}

fn directory_bytes(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    total += meta.len();
                } else if meta.is_dir() {
                    total += directory_bytes(&entry.path());
                }
            }
        }
    }
    total
}
