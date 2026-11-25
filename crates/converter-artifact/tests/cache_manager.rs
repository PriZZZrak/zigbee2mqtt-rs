use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use converter_artifact::{
    CacheConfig, FetchError, Manifest, ModuleCache, ModuleFetcher, ModuleState, SpaceProbe,
};
use serde_json::json;
use sha2::{Digest, Sha256};

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn temp(name: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "z2m-cache-{name}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
struct Fixture {
    manifest: Manifest,
    #[allow(dead_code)]
    value: serde_json::Value,
    bytes: HashMap<String, Vec<u8>>,
}
fn fixture() -> Fixture {
    let dir = temp("artifact");
    let modules = [
        ("a", b"module-a".to_vec(), vec!["b"]),
        ("b", b"module-b".to_vec(), vec![]),
        ("c", b"module-c".to_vec(), vec![]),
    ];
    let mut bytes = HashMap::new();
    let mut entries = Vec::new();
    for (id, data, requires) in modules {
        std::fs::write(dir.join(format!("{id}.js")), &data).unwrap();
        entries.push(json!({"id": id, "file": format!("dist/devices/{id}.js"), "bytes": data.len(), "sha256": hash(&data), "definitions": 1, "requires": requires}));
        bytes.insert(id.to_owned(), data);
    }
    let manifest_value = json!({
        "manifestSchema": 1, "artifactId": "test-artifact",
        "upstream": {}, "host": {"abi": 1, "engine": "quickjs-oxide", "engineVersion": "0.5.0", "zclCatalogSha256": "00", "requiredFeatures": []},
        "index": {"file": "index.z2idx1", "format": "z2idx1", "version": 1, "bytes": 0, "sha256": hash(b""), "jsonBytes": 0, "jsonSha256": hash(b""), "lookupKeys": 0, "definitions": 0, "candidates": 0},
        "graph": {"file": "graph.json", "bytes": 0, "sha256": hash(b"")},
        "shared": {"root": "dist", "files": [], "bytes": 0, "sha256": hash(b"")},
        "modules": {"format": "cjs", "packageIncluded": false, "entries": entries},
        "compatibility": {"minHostAbi": 1, "maxHostAbi": 1}
    });
    let manifest =
        Manifest::parse(serde_json::to_vec(&manifest_value).unwrap().as_slice()).unwrap();
    Fixture {
        manifest,
        value: manifest_value,
        bytes,
    }
}
struct FetchState {
    bytes: HashMap<String, Vec<u8>>,
    calls: Mutex<Vec<String>>,
    fail: bool,
    corrupt: bool,
    truncate: bool,
}
struct Fetcher {
    state: Arc<FetchState>,
}
impl ModuleFetcher for Fetcher {
    fn fetch(&self, module: &converter_artifact::ModuleRecord) -> Result<Vec<u8>, FetchError> {
        self.state.calls.lock().unwrap().push(module.id.clone());
        if self.state.fail {
            return Err(FetchError::Network("offline".to_owned()));
        }
        let mut data = self
            .state
            .bytes
            .get(&module.id)
            .cloned()
            .ok_or(FetchError::NotFound)?;
        if self.state.truncate {
            data.truncate(data.len() / 2);
        }
        if self.state.corrupt {
            data[0] ^= 0xff;
        }
        Ok(data)
    }
}
fn fetcher(fixture: &Fixture) -> (Fetcher, Arc<FetchState>) {
    let state = Arc::new(FetchState {
        bytes: fixture.bytes.clone(),
        calls: Mutex::new(Vec::new()),
        fail: false,
        corrupt: false,
        truncate: false,
    });
    (
        Fetcher {
            state: Arc::clone(&state),
        },
        state,
    )
}
fn config(dir: &PathBuf) -> CacheConfig {
    CacheConfig::new(dir)
}

#[test]
fn downloads_only_selected_closure_and_works_offline() {
    let fixture = fixture();
    let cache_root = temp("cache");
    let cache = ModuleCache::open(config(&cache_root), fixture.manifest.clone()).unwrap();
    let (fetcher_impl, state) = fetcher(&fixture);
    let report = cache
        .ensure(&["a".to_owned()], &fetcher_impl, &config(&cache_root), true)
        .unwrap();
    let mut calls = state.calls.lock().unwrap().clone();
    calls.sort();
    assert_eq!(calls, ["a", "b"]);
    assert_eq!(report.fetched, 2);
    assert!(report.network_bytes > 0);
    drop(cache);
    let reopened = ModuleCache::open(config(&cache_root), fixture.manifest.clone()).unwrap();
    assert_eq!(reopened.state("a"), ModuleState::Ready);
    assert_eq!(reopened.state("b"), ModuleState::Ready);
    assert_eq!(reopened.state("c"), ModuleState::Missing);
    assert_eq!(reopened.load_ready("a").unwrap(), b"module-a");
    let (offline, offline_state) = fetcher(&fixture);
    reopened
        .ensure(&["a".to_owned()], &offline, &config(&cache_root), false)
        .unwrap();
    assert!(offline_state.calls.lock().unwrap().is_empty());
}

#[test]
fn hash_mismatch_and_partial_download_do_not_activate() {
    for (corrupt, truncate) in [(true, false), (false, true)] {
        let fixture = fixture();
        let cache_root = temp("bad");
        let cache = ModuleCache::open(config(&cache_root), fixture.manifest.clone()).unwrap();
        let (_fetcher, state) = fetcher(&fixture);
        *state.calls.lock().unwrap() = vec![];
        let state_handle = Arc::clone(&state);
        if corrupt { /* mutate fetcher after construction via interior flags */ }
        // Build a dedicated failing fetcher through a proxy state.
        let failing = Arc::new(FetchState {
            bytes: fixture.bytes.clone(),
            calls: Mutex::new(Vec::new()),
            fail: false,
            corrupt,
            truncate,
        });
        let proxy = Fetcher {
            state: Arc::clone(&failing),
        };
        assert!(
            cache
                .ensure(&["a".to_owned()], &proxy, &config(&cache_root), true)
                .is_err()
        );
        assert!(cache.cache_bytes() == 0);
        assert!(matches!(
            cache.state("a"),
            ModuleState::Failed(_) | ModuleState::Missing
        ));
        let _ = state_handle;
    }
}

#[test]
fn network_failure_and_insufficient_space_are_explicit() {
    let fixture = fixture();
    let cache_root = temp("errors");
    let cache = ModuleCache::open(config(&cache_root), fixture.manifest.clone()).unwrap();
    let state = Arc::new(FetchState {
        bytes: fixture.bytes.clone(),
        calls: Mutex::new(Vec::new()),
        fail: true,
        corrupt: false,
        truncate: false,
    });
    let error = cache
        .ensure(
            &["a".to_owned()],
            &Fetcher { state },
            &config(&cache_root),
            true,
        )
        .unwrap_err();
    assert!(error.to_string().contains("network"));
    let (fetcher_impl, _) = fetcher(&fixture);
    let mut limited = config(&cache_root);
    limited.max_cache_bytes = 1;
    assert!(
        cache
            .ensure(&["a".to_owned()], &fetcher_impl, &limited, true)
            .is_err()
    );
    struct NoSpace;
    impl SpaceProbe for NoSpace {
        fn available_bytes(&self) -> Option<u64> {
            Some(1)
        }
    }
    let mut space = config(&cache_root);
    space.space_probe = Some(Arc::new(NoSpace));
    assert!(
        cache
            .ensure(&["a".to_owned()], &fetcher_impl, &space, true)
            .is_err()
    );
}

#[test]
fn lkg_rollback_restores_previous_verified_module() {
    let fixture = fixture();
    let cache_root = temp("rollback");
    let cache = ModuleCache::open(config(&cache_root), fixture.manifest.clone()).unwrap();
    let (fetcher_impl, _) = fetcher(&fixture);
    cache
        .ensure(&["b".to_owned()], &fetcher_impl, &config(&cache_root), true)
        .unwrap();
    let active = cache.active_module_path("b").expect("active module");
    let old = cache.root().join("generations/old");
    std::fs::create_dir_all(&old).unwrap();
    std::fs::copy(&active, old.join("b.js")).unwrap();
    let lkg = json!({"generation":"old","modules":{"b":hash(b"module-b")}});
    std::fs::write(
        cache.root().join("lkg.json"),
        serde_json::to_vec(&lkg).unwrap(),
    )
    .unwrap();
    std::fs::write(&active, b"corrupt-active").unwrap();
    cache.rollback("b").unwrap();
    assert_eq!(cache.load_ready("b").unwrap(), b"module-b");
}

#[test]
fn abi_and_artifact_compatibility_are_enforced() {
    let fixture = fixture();
    assert!(
        fixture
            .manifest
            .verify_compatibility(1, "quickjs-oxide", "0.5.0")
            .is_ok()
    );
    assert!(
        fixture
            .manifest
            .verify_compatibility(2, "quickjs-oxide", "0.5.0")
            .is_err()
    );
    assert!(
        fixture
            .manifest
            .verify_compatibility(1, "other", "0.5.0")
            .is_err()
    );
}

#[test]
fn concurrent_ensure_is_single_flight_per_module() {
    let fixture = fixture();
    let cache_root = temp("single-flight");
    let cache = Arc::new(ModuleCache::open(config(&cache_root), fixture.manifest.clone()).unwrap());
    let (fetcher_impl, state) = fetcher(&fixture);
    let fetcher_impl = Arc::new(fetcher_impl);
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let cache = Arc::clone(&cache);
            let fetcher = Arc::clone(&fetcher_impl);
            let root = cache_root.clone();
            scope.spawn(move || {
                cache
                    .ensure(&["a".to_owned()], fetcher.as_ref(), &config(&root), true)
                    .unwrap();
            });
        }
    });
    let calls = state.calls.lock().unwrap().clone();
    assert_eq!(calls.iter().filter(|id| id.as_str() == "a").count(), 1);
    assert_eq!(calls.iter().filter(|id| id.as_str() == "b").count(), 1);
    assert_eq!(cache.load_ready("a").unwrap(), b"module-a");
}

#[test]
fn open_recovers_from_orphan_staging_and_invalid_active_pointer() {
    let fixture = fixture();
    let cache_root = temp("crash");
    let cache = ModuleCache::open(config(&cache_root), fixture.manifest.clone()).unwrap();
    let (fetcher_impl, _) = fetcher(&fixture);
    cache
        .ensure(&["b".to_owned()], &fetcher_impl, &config(&cache_root), true)
        .unwrap();
    let active = cache.active_module_path("b").unwrap();
    let old = cache.root().join("generations/old");
    std::fs::create_dir_all(&old).unwrap();
    std::fs::copy(&active, old.join("b.js")).unwrap();
    std::fs::write(
        cache.root().join("lkg.json"),
        serde_json::to_vec(&json!({"generation":"old","modules":{"b":hash(b"module-b")}})).unwrap(),
    )
    .unwrap();
    // Simulate a crash after a partial staging transaction and a corrupt
    // active pointer. open() must fall back to LKG.
    std::fs::create_dir_all(cache.root().join("staging/orphan")).unwrap();
    std::fs::write(cache.root().join("staging/orphan/b.js"), b"partial").unwrap();
    std::fs::write(cache.root().join("active.json"), b"not-json").unwrap();
    drop(cache);
    let reopened = ModuleCache::open(config(&cache_root), fixture.manifest).unwrap();
    assert_eq!(reopened.state("b"), ModuleState::Ready);
    assert_eq!(reopened.load_ready("b").unwrap(), b"module-b");
    assert!(!reopened.root().join("staging/orphan").exists());
}

struct FailOn {
    inner: Fetcher,
    id: &'static str,
}
impl ModuleFetcher for FailOn {
    fn fetch(&self, module: &converter_artifact::ModuleRecord) -> Result<Vec<u8>, FetchError> {
        if module.id == self.id {
            Err(FetchError::Network("boom".to_owned()))
        } else {
            self.inner.fetch(module)
        }
    }
}

#[test]
fn aborted_transaction_never_activates_partially_staged_modules() {
    let fixture = fixture();
    let cache_root = temp("partial");
    let cache = ModuleCache::open(config(&cache_root), fixture.manifest.clone()).unwrap();
    let (fetcher_impl, _) = fetcher(&fixture);
    let mut single = config(&cache_root);
    single.max_concurrent_downloads = 1;
    // "a" is staged, then its dependency "b" fails: nothing may become active.
    let failing = FailOn {
        inner: fetcher_impl,
        id: "b",
    };
    let error = cache
        .ensure(&["a".to_owned()], &failing, &single, true)
        .unwrap_err();
    assert!(error.to_string().contains("network"));
    assert_eq!(cache.cache_bytes(), 0);
    assert!(cache.load_ready("a").is_err());
    assert!(matches!(cache.state("b"), ModuleState::Failed(_)));
    assert!(matches!(cache.state("a"), ModuleState::Failed(_)));
    // The next transaction starts from a clean cache.
    let (fresh, _) = fetcher(&fixture);
    cache
        .ensure(&["a".to_owned()], &fresh, &single, true)
        .unwrap();
    assert_eq!(cache.load_ready("a").unwrap(), b"module-a");
}

#[test]
fn budget_counts_staging_lkg_and_peak_transaction() {
    let fixture = fixture();
    let cache_root = temp("budget");
    let cache = ModuleCache::open(config(&cache_root), fixture.manifest.clone()).unwrap();
    let (fetcher_impl, _) = fetcher(&fixture);
    cache
        .ensure(&["b".to_owned()], &fetcher_impl, &config(&cache_root), true)
        .unwrap();
    // "b" (8 B) is active. Adding "c" (8 B) copies the active generation into
    // staging, so the peak is 8 + 8 + 8 = 24 B even though the final cache is
    // 16 B. A 16 B cap must be rejected before any write.
    let mut tight = config(&cache_root);
    tight.max_cache_bytes = 16;
    let before = cache.cache_bytes();
    let error = cache
        .ensure(&["c".to_owned()], &fetcher_impl, &tight, true)
        .unwrap_err();
    assert!(error.to_string().contains("peak"), "{error}");
    assert_eq!(cache.cache_bytes(), before);
    assert!(cache.load_ready("c").is_err());
    let mut enough = config(&cache_root);
    enough.max_cache_bytes = 24;
    cache
        .ensure(&["c".to_owned()], &fetcher_impl, &enough, true)
        .unwrap();
    assert_eq!(cache.load_ready("c").unwrap(), b"module-c");
}

#[test]
fn memory_budget_rejects_oversized_fetch_chunk() {
    let fixture = fixture();
    let cache_root = temp("memory");
    let cache = ModuleCache::open(config(&cache_root), fixture.manifest.clone()).unwrap();
    let (fetcher_impl, _) = fetcher(&fixture);
    let mut limited = config(&cache_root);
    limited.max_memory_bytes = 1;
    let error = cache
        .ensure(&["a".to_owned()], &fetcher_impl, &limited, true)
        .unwrap_err();
    assert!(error.to_string().contains("memory budget"), "{error}");
    assert_eq!(cache.cache_bytes(), 0);
}

struct PanicOn {
    inner: Fetcher,
    id: &'static str,
}
impl ModuleFetcher for PanicOn {
    fn fetch(&self, module: &converter_artifact::ModuleRecord) -> Result<Vec<u8>, FetchError> {
        if module.id == self.id {
            panic!("simulated cancellation");
        }
        self.inner.fetch(module)
    }
}

#[test]
fn cancelled_fetch_aborts_cleanly_and_does_not_poison_the_cache() {
    let fixture = fixture();
    let cache_root = temp("cancel");
    let cache = ModuleCache::open(config(&cache_root), fixture.manifest.clone()).unwrap();
    let (fetcher_impl, _) = fetcher(&fixture);
    let panicking = PanicOn {
        inner: fetcher_impl,
        id: "b",
    };
    let mut single = config(&cache_root);
    single.max_concurrent_downloads = 1;
    let error = cache
        .ensure(&["a".to_owned()], &panicking, &single, true)
        .unwrap_err();
    assert!(error.to_string().contains("panicked"), "{error}");
    assert_eq!(cache.cache_bytes(), 0);
    // A panicking worker must not poison the activation lock: the next device
    // still loads normally.
    let (fresh, _) = fetcher(&fixture);
    cache
        .ensure(&["a".to_owned()], &fresh, &single, true)
        .unwrap();
    assert_eq!(cache.load_ready("a").unwrap(), b"module-a");
}
