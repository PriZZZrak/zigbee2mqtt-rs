use std::path::{Path, PathBuf};
use std::sync::Arc;

use converter_artifact::{CacheConfig, FetchError, LazyConfig, ModuleFetcher, ModuleRecord};
use serde_json::json;
use zigbee_apiarist::controller::database::{Database, DeviceType};
use zigbee_apiarist::controller::model::device::{Device as ZhDevice, InterviewState};
use zigbee_apiarist::controller::model::entity::inject_database;

use z2m_rs::converter::{ConverterDispatcher, DispatcherConfig};

struct Source {
    dir: PathBuf,
}
impl ModuleFetcher for Source {
    fn fetch(&self, module: &ModuleRecord) -> Result<Vec<u8>, FetchError> {
        std::fs::read(self.dir.join(&module.file)).map_err(|_| FetchError::NotFound)
    }
}
fn temp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("z2m-lazy-dispatch-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn database(path: &Path) {
    let record = json!({"type":"EndDevice","ieeeAddr":"0x0017880104e45519","nwkAddr":1234,"manufId":4417,"manufName":"_TZE200_locansqn","powerSource":"Battery","modelId":"TS0601","epList":[1],"endpoints":{"1":{"profId":260,"epId":1,"devId":81,"inClusterList":[0,4,5,61184],"outClusterList":[25,10]}}});
    std::fs::write(path.join("database.db"), format!("{}\n", record)).unwrap();
}
#[tokio::test]
async fn lazy_dispatcher_materializes_only_paired_module_and_prepares() {
    let Ok(artifact) = std::env::var("CONVERTER_ARTIFACT_DIR") else {
        return;
    };
    let artifact = PathBuf::from(artifact);
    let work = temp("work");
    let db = work.join("database.db");
    database(&work);
    let cache = work.join("cache");
    let runtime_root = work.join("runtime");
    let transport: Arc<dyn z2m_rs::converter::ConverterTransport> =
        Arc::new(z2m_rs::converter::OfflineMockTransport::new());
    let herdsman = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".upstream/zigbee2mqtt/node_modules/zigbee-herdsman");
    let config = DispatcherConfig::new(&artifact, herdsman);
    let lazy = LazyConfig {
        artifact_dir: artifact.clone(),
        cache: CacheConfig::new(&cache),
        host_abi: 1,
        engine: "quickjs-oxide".to_owned(),
        engine_version: "0.5.0".to_owned(),
    };
    let dispatcher = ConverterDispatcher::start_lazy(
        config,
        transport,
        lazy,
        db.clone(),
        Arc::new(Source {
            dir: artifact.clone(),
        }),
        runtime_root.clone(),
        true,
    )
    .await
    .unwrap();
    let report = dispatcher.startup_report().unwrap();
    assert_eq!(report.resolved, 1);
    assert!(report.modules_loaded.iter().any(|module| module == "nous"));
    let active_root = dispatcher.runtime_root().expect("active runtime root");
    assert!(active_root.starts_with(&runtime_root));
    assert!(active_root.join("dist/devices/nous.js").is_file());
    assert!(!active_root.join("dist/devices/tuya-0.js").exists());
    assert!(runtime_root.join("active.json").is_file());
    // The same dispatcher prepares the cached module through quickjs.
    inject_database(Arc::new(std::sync::Mutex::new(Database::open(
        work.join("probe.db"),
    ))));
    ZhDevice::reset_cache();
    let zh = ZhDevice::create(
        DeviceType::EndDevice,
        "0x0017880104e45519",
        1234,
        Some(4417),
        Some("_TZE200_locansqn".to_owned()),
        Some("Battery".to_owned()),
        Some("TS0601".to_owned()),
        InterviewState::Successful,
        None,
    )
    .unwrap();
    zh.create_endpoint(1).unwrap();
    let prepared = dispatcher.pair_device(&zh).await;
    assert!(prepared.is_ok(), "{prepared:?}");
    dispatcher.shutdown().await;
}

#[tokio::test]
async fn external_lytko_still_prepares_when_lazy_artifact_has_no_definition() {
    let Ok(artifact) = std::env::var("CONVERTER_ARTIFACT_DIR") else {
        return;
    };
    let artifact = PathBuf::from(artifact);
    let external = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/bench/fixtures/external/L101Ze_z2m.js");
    if !external.is_file() {
        return;
    }
    let work = temp("lytko");
    let db = work.join("database.db");
    std::fs::write(&db, format!("{}\n", json!({"type":"Router","ieeeAddr":"0x0017880104e45541","nwkAddr":1234,"manufId":1,"manufName":"LYTKO","powerSource":"Mains","modelId":"L101Ze.v2","epList":[1,2,3],"endpoints":{"1":{"profId":260,"epId":1,"devId":81,"inClusterList":[0,6],"outClusterList":[25]}}})).to_string()).unwrap();
    let cache = work.join("cache");
    let runtime_root = work.join("runtime");
    let mut config = DispatcherConfig::new(
        &artifact,
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../.upstream/zigbee2mqtt/node_modules/zigbee-herdsman"),
    );
    config.runtime.external_path = external;
    let transport: Arc<dyn z2m_rs::converter::ConverterTransport> =
        Arc::new(z2m_rs::converter::OfflineMockTransport::new());
    let lazy = LazyConfig {
        artifact_dir: artifact.clone(),
        cache: CacheConfig::new(&cache),
        host_abi: 1,
        engine: "quickjs-oxide".to_owned(),
        engine_version: "0.5.0".to_owned(),
    };
    let dispatcher = ConverterDispatcher::start_lazy(
        config,
        transport,
        lazy,
        db,
        Arc::new(Source { dir: artifact }),
        runtime_root,
        false,
    )
    .await
    .unwrap();
    inject_database(Arc::new(std::sync::Mutex::new(Database::open(
        work.join("probe.db"),
    ))));
    ZhDevice::reset_cache();
    let zh = ZhDevice::create(
        DeviceType::Router,
        "0x0017880104e45541",
        1234,
        Some(1),
        Some("LYTKO".to_owned()),
        Some("Mains".to_owned()),
        Some("L101Ze.v2".to_owned()),
        InterviewState::Successful,
        None,
    )
    .unwrap();
    zh.create_endpoint(1).unwrap();
    let prepared = dispatcher.prepare(&zh).await;
    assert!(prepared.is_ok(), "{prepared:?}");
    assert_eq!(prepared.unwrap().metadata.model, "L101Ze.v2");
    dispatcher.shutdown().await;
}
