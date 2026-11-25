// Offline lazy-bridge cold-start probe: loader + real quickjs prepareDefinition.
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use converter_artifact::{CacheConfig, FetchError, LazyConfig, ModuleFetcher, ModuleRecord};
use serde_json::json;
use z2m_rs::converter::{ConverterDispatcher, DispatcherConfig, OfflineMockTransport};
use zigbee_apiarist::controller::database::{Database, DeviceType};
use zigbee_apiarist::controller::model::device::{Device as ZhDevice, InterviewState};
use zigbee_apiarist::controller::model::entity::inject_database;

struct Source {
    dir: PathBuf,
    bytes: Arc<std::sync::atomic::AtomicU64>,
}
impl ModuleFetcher for Source {
    fn fetch(&self, module: &ModuleRecord) -> Result<Vec<u8>, FetchError> {
        let data = std::fs::read(self.dir.join(&module.file)).map_err(|_| FetchError::NotFound)?;
        self.bytes
            .fetch_add(data.len() as u64, std::sync::atomic::Ordering::Relaxed);
        Ok(data)
    }
}
fn arg(name: &str, fallback: &str) -> String {
    let mut a = std::env::args().skip(1);
    while let Some(x) = a.next() {
        if x == name {
            return a.next().expect("value");
        }
    }
    fallback.to_owned()
}
fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("z2m-lazy-bridge-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}
fn write_db(path: &Path, count: usize) {
    let mut text = String::new();
    for i in 0..count {
        let record = json!({"type":"EndDevice","ieeeAddr":format!("0x0017880104e4{:04x}",i),"nwkAddr":0x1000+i,"manufId":4417,"manufName":"_TZE200_locansqn","powerSource":"Battery","modelId":"TS0601","epList":[1],"endpoints":{"1":{"profId":260,"epId":1,"devId":81,"inClusterList":[0,4,5,61184],"outClusterList":[25,10]}}});
        text.push_str(&record.to_string());
        text.push('\n');
    }
    std::fs::write(path, text).unwrap();
}
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let artifact = PathBuf::from(arg("--artifact", "/tmp/engine-study/lazy-artifact"));
    let count: usize = arg("--devices", "20").parse().unwrap();
    let work = temp("work");
    let db = work.join("database.db");
    write_db(&db, count);
    let fetched = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let fetcher = Arc::new(Source {
        dir: artifact.clone(),
        bytes: Arc::clone(&fetched),
    });
    let lazy = LazyConfig {
        artifact_dir: artifact.clone(),
        cache: CacheConfig::new(work.join("cache")),
        host_abi: 1,
        engine: "quickjs-oxide".to_owned(),
        engine_version: "0.5.0".to_owned(),
    };
    let herdsman = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".upstream/zigbee2mqtt/node_modules/zigbee-herdsman");
    let started = Instant::now();
    let dispatcher = ConverterDispatcher::start_lazy(
        DispatcherConfig::new(&artifact, herdsman),
        Arc::new(OfflineMockTransport::new()),
        lazy,
        db.clone(),
        fetcher,
        work.join("runtime"),
        true,
    )
    .await
    .unwrap();
    let loader_ms = started.elapsed().as_secs_f64() * 1000.0;
    // Create the paired devices from the same profile and prepare each through
    // the real dispatcher/quickjs path.
    inject_database(Arc::new(std::sync::Mutex::new(Database::open(
        work.join("probe.db"),
    ))));
    ZhDevice::reset_cache();
    let mut devices = Vec::new();
    for i in 0..count {
        let zh = ZhDevice::create(
            DeviceType::EndDevice,
            &format!("0x0017880104e4{:04x}", i),
            0x1000 + i as u16,
            Some(4417),
            Some("_TZE200_locansqn".to_owned()),
            Some("Battery".to_owned()),
            Some("TS0601".to_owned()),
            InterviewState::Successful,
            None,
        )
        .unwrap();
        let ep = zh.create_endpoint(1).unwrap();
        *ep.input_clusters.lock().unwrap() = vec![0, 4, 5, 61184];
        *ep.output_clusters.lock().unwrap() = vec![25, 10];
        devices.push(zh);
    }
    let prepare_started = Instant::now();
    let mut ready = 0;
    let mut errors = 0;
    for zh in &devices {
        if dispatcher.pair_device(zh).await.is_ok() {
            ready += 1
        } else {
            errors += 1
        }
    }
    let prepare_ms = prepare_started.elapsed().as_secs_f64() * 1000.0;
    let total_ms = started.elapsed().as_secs_f64() * 1000.0;
    let report = dispatcher.startup_report().unwrap_or_default();
    println!(
        "{}",
        serde_json::json!({"devices":count,"resolved":report.resolved,"modulesLoaded":report.modules_loaded,"networkBytes":fetched.load(std::sync::atomic::Ordering::Relaxed),"cacheBytes":dispatcher.startup_report().map(|r|r.cache_bytes).unwrap_or(0),"loaderStartupMs":loader_ms,"prepareAllMs":prepare_ms,"fullColdStartMs":total_ms,"readyDevices":ready,"failedDevices":errors})
    );
    dispatcher.shutdown().await;
    tokio::time::sleep(Duration::from_millis(50)).await;
}
