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
    let active = cache.root().join("modules/b.js");
    let lkg = cache.root().join("lkg/b.js");
    std::fs::create_dir_all(lkg.parent().unwrap()).unwrap();
    std::fs::copy(&active, &lkg).unwrap();
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
