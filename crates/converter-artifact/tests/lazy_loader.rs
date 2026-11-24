use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use converter_artifact::{
    CacheConfig, FetchError, LazyConfig, LazyLoader, ModuleFetcher, ModuleRecord, ModuleState,
};
use serde_json::json;

struct SourceFetcher {
    dir: PathBuf,
}
impl ModuleFetcher for SourceFetcher {
    fn fetch(&self, module: &ModuleRecord) -> Result<Vec<u8>, FetchError> {
        let file = self.dir.join(&module.file);
        std::fs::read(&file).map_err(|_| FetchError::NotFound)
    }
}
fn temp(name: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "z2m-lazy-{name}-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
fn database(path: &Path) {
    let value = json!({"id":1,"type":"EndDevice","ieeeAddr":"0x0017880104e45519","nwkAddr":1234,"manufId":4417,"manufName":"_TZE200_locansqn","powerSource":"Battery","modelId":"TS0601","epList":[1],"endpoints":{"1":{"profId":260,"epId":1,"devId":81,"inClusterList":[0,4,5,61184],"outClusterList":[25,10]}},"appVersion":72,"stackVersion":0,"hwVersion":1,"zclVersion":3,"dateCode":"","swBuildId":"","interviewState":"SUCCESSFUL"});
    std::fs::write(path.join("database.db"), format!("{}\n", value)).unwrap();
}

#[test]
fn lazy_startup_pairs_only_selected_module_and_works_offline() {
    let Ok(artifact) = std::env::var("CONVERTER_ARTIFACT_DIR") else {
        return;
    };
    let artifact = PathBuf::from(artifact);
    let cache = temp("cache");
    let db = temp("db");
    database(&db);
    let config = LazyConfig {
        artifact_dir: artifact.clone(),
        cache: CacheConfig::new(&cache),
        host_abi: 1,
        engine: "quickjs-oxide".to_owned(),
        engine_version: "0.5.0".to_owned(),
    };
    let loader = Arc::new(LazyLoader::open(config.clone()).unwrap());
    let fetcher = SourceFetcher {
        dir: artifact.clone(),
    };
    let report = loader
        .startup(&db.join("database.db"), &fetcher, true)
        .unwrap();
    assert_eq!(report.devices, 1);
    assert_eq!(report.resolved, 1);
    assert!(report.modules_loaded.iter().any(|module| module == "nous"));
    assert!(
        !report
            .modules_loaded
            .iter()
            .any(|module| module == "tuya-0")
    );
    assert!(report.network_bytes > 0);
    // Offline startup uses the verified cache, no fetcher calls.
    let offline = LazyLoader::open(config).unwrap();
    let no_fetch = SourceFetcher {
        dir: PathBuf::from("/nonexistent"),
    };
    let report = offline
        .startup(&db.join("database.db"), &no_fetch, false)
        .unwrap();
    assert_eq!(report.resolved, 1);
    assert!(report.network_bytes == 0);
    assert!(report.modules_loaded.iter().any(|module| module == "nous"));
}

#[test]
fn concurrent_pairing_is_bounded_and_does_not_touch_existing_devices() {
    let Ok(artifact) = std::env::var("CONVERTER_ARTIFACT_DIR") else {
        return;
    };
    let artifact = PathBuf::from(artifact);
    let cache = temp("concurrent");
    let db = temp("db2");
    database(&db);
    let config = LazyConfig {
        artifact_dir: artifact.clone(),
        cache: CacheConfig::new(&cache),
        host_abi: 1,
        engine: "quickjs-oxide".to_owned(),
        engine_version: "0.5.0".to_owned(),
    };
    let loader = Arc::new(LazyLoader::open(config).unwrap());
    let fetcher: Arc<dyn ModuleFetcher> = Arc::new(SourceFetcher { dir: artifact });
    let existing = loader
        .startup(&db.join("database.db"), fetcher.as_ref(), true)
        .unwrap();
    assert_eq!(existing.resolved, 1);
    let profile = converter_artifact::DeviceProfile {
        r#type: converter_artifact::DeviceType::EndDevice,
        model_id: Some("TS0601".to_owned()),
        manufacturer_name: Some("_TZE200_locansqn".to_owned()),
        ..Default::default()
    };
    let first = loader.pair(profile.clone(), Arc::clone(&fetcher), true);
    let second = loader.pair(profile, Arc::clone(&fetcher), true);
    assert!(first.join().unwrap().is_ok());
    assert!(second.join().unwrap().is_ok());
    // A device leave does not invalidate the verified shared cache entry.
    let report = loader
        .startup(&db.join("database.db"), fetcher.as_ref(), false)
        .unwrap();
    assert_eq!(report.resolved, 1);
}

#[test]
fn device_leave_keeps_verified_cache_available() {
    let Ok(artifact) = std::env::var("CONVERTER_ARTIFACT_DIR") else {
        return;
    };
    let artifact = PathBuf::from(artifact);
    let cache = temp("leave");
    let db = temp("leave-db");
    database(&db);
    let config = LazyConfig {
        artifact_dir: artifact,
        cache: CacheConfig::new(&cache),
        host_abi: 1,
        engine: "quickjs-oxide".to_owned(),
        engine_version: "0.5.0".to_owned(),
    };
    let loader = Arc::new(LazyLoader::open(config.clone()).unwrap());
    let fetcher = SourceFetcher {
        dir: config.artifact_dir.clone(),
    };
    assert!(
        loader
            .startup(&db.join("database.db"), &fetcher, true)
            .is_ok()
    );
    std::fs::write(db.join("database.db"), "").unwrap();
    let offline = LazyLoader::open(config.clone()).unwrap();
    let report = offline
        .startup(&db.join("database.db"), &fetcher, false)
        .unwrap();
    assert_eq!(report.resolved, 0);
    assert!(report.failed.is_empty());
    // The module remains verified and can be loaded again if the device returns.
    assert_eq!(offline.cache().state("nous"), ModuleState::Ready);
}
