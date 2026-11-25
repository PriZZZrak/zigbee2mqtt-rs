// Offline lazy-bridge probe.
//
// Modes:
// - startup      loader + per-device real quickjs prepareDefinition (cold start);
// - impact       report-path latency while a new Tuya fragment is downloaded and
//                prepared on the shared JS worker;
// - single-flight concurrent vs sequential closure loads with a latent fetcher.
//
// No coordinator, MQTT broker, Home Assistant or network is contacted.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use converter_artifact::{
    read_database, CacheConfig, DeviceProfile, FetchError, LazyConfig, LazyLoader, ModuleFetcher,
    ModuleRecord,
};
use serde_json::{json, Value};
use z2m_rs::converter::{ConverterDispatcher, DispatcherConfig, OfflineMockTransport};
use zigbee_apiarist::controller::database::{Database, DeviceType};
use zigbee_apiarist::controller::model::device::{Device as ZhDevice, InterviewState};
use zigbee_apiarist::controller::model::entity::inject_database;
use zigbee_converters::{ConvertData, ConvertMeta};

struct Source {
    dir: PathBuf,
    bytes: Arc<AtomicU64>,
    delay: Duration,
    calls: Mutex<BTreeMap<String, usize>>,
}
impl ModuleFetcher for Source {
    fn fetch(&self, module: &ModuleRecord) -> Result<Vec<u8>, FetchError> {
        *self
            .calls
            .lock()
            .unwrap()
            .entry(module.id.clone())
            .or_insert(0) += 1;
        if !self.delay.is_zero() {
            std::thread::sleep(self.delay);
        }
        let data = std::fs::read(self.dir.join(&module.file)).map_err(|_| FetchError::NotFound)?;
        self.bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
        Ok(data)
    }
}
impl Source {
    fn new(dir: PathBuf, delay: Duration) -> Self {
        Self {
            dir,
            bytes: Arc::new(AtomicU64::new(0)),
            delay,
            calls: Mutex::new(BTreeMap::new()),
        }
    }
    fn calls(&self) -> BTreeMap<String, usize> {
        self.calls.lock().unwrap().clone()
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
fn percentile(values: &mut [f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    values[((values.len() - 1) as f64 * percentile).round() as usize]
}
fn write_db(path: &Path, records: &[Value]) {
    let mut text = String::new();
    for record in records {
        text.push_str(&record.to_string());
        text.push('\n');
    }
    std::fs::write(path, text).unwrap();
}
fn nous_record(index: usize) -> Value {
    json!({"type":"EndDevice","ieeeAddr":format!("0x0017880104e4{:04x}",index),"nwkAddr":0x1000+index,"manufId":4417,"manufName":"_TZE200_locansqn","powerSource":"Battery","modelId":"TS0601","epList":[1],"endpoints":{"1":{"profId":260,"epId":1,"devId":81,"inClusterList":[0,4,5,61184],"outClusterList":[25,10]}}})
}
fn ikea_record() -> Value {
    json!({"type":"Router","ieeeAddr":"0x0017880104e45531","nwkAddr":4660,"manufId":4476,"manufName":"IKEA of Sweden","powerSource":"Mains","modelId":"TRADFRI bulb E27 WS opal 1000lm","epList":[1],"endpoints":{"1":{"profId":260,"epId":1,"devId":81,"inClusterList":[0,3,4,5,6,8,768],"outClusterList":[25]}}})
}
fn create_device(
    ieee: &str,
    network: u16,
    manufacturer: &str,
    model: &str,
    device_type: DeviceType,
    clusters: &[u16],
    out: &[u16],
) -> Arc<ZhDevice> {
    let zh = ZhDevice::create(
        device_type,
        ieee,
        network,
        None,
        Some(manufacturer.to_owned()),
        None,
        Some(model.to_owned()),
        InterviewState::Successful,
        None,
    )
    .unwrap();
    let endpoint = zh.create_endpoint(1).unwrap();
    *endpoint.input_clusters.lock().unwrap() = clusters.to_vec();
    *endpoint.output_clusters.lock().unwrap() = out.to_vec();
    zh
}
fn herbsman_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".upstream/zigbee2mqtt/node_modules/zigbee-herdsman")
}
fn lazy_config(artifact: &Path, work: &Path) -> LazyConfig {
    LazyConfig {
        artifact_dir: artifact.to_path_buf(),
        cache: CacheConfig::new(work.join("cache")),
        host_abi: 1,
        engine: "quickjs-oxide".to_owned(),
        engine_version: "0.5.0".to_owned(),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let artifact = PathBuf::from(arg("--artifact", "/tmp/engine-study/lazy-artifact"));
    let mode = arg("--mode", "startup");
    match mode.as_str() {
        "startup" => startup(&artifact).await,
        "impact" => impact(&artifact).await,
        "single-flight" => single_flight(&artifact),
        other => panic!("unknown mode {other}"),
    }
}

async fn startup(artifact: &Path) {
    let count: usize = arg("--devices", "20").parse().unwrap();
    let work = temp("work");
    let db = work.join("database.db");
    let records: Vec<Value> = (0..count).map(nous_record).collect();
    write_db(&db, &records);
    let fetcher = Arc::new(Source::new(artifact.to_path_buf(), Duration::ZERO));
    let started = Instant::now();
    let dispatcher = ConverterDispatcher::start_lazy(
        DispatcherConfig::new(artifact, herbsman_root()),
        Arc::new(OfflineMockTransport::new()),
        lazy_config(artifact, &work),
        db,
        fetcher.clone(),
        work.join("runtime"),
        true,
    )
    .await
    .unwrap();
    let loader_ms = started.elapsed().as_secs_f64() * 1000.0;
    inject_database(Arc::new(Mutex::new(Database::open(work.join("probe.db")))));
    ZhDevice::reset_cache();
    let mut devices = Vec::new();
    for index in 0..count {
        devices.push(create_device(
            &format!("0x0017880104e4{index:04x}"),
            0x1000 + index as u16,
            "_TZE200_locansqn",
            "TS0601",
            DeviceType::EndDevice,
            &[0, 4, 5, 61184],
            &[25, 10],
        ));
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
        json!({"mode":"startup","devices":count,"resolved":report.resolved,"modulesLoaded":report.modules_loaded,"networkBytes":fetcher.bytes.load(Ordering::Relaxed),"cacheBytes":report.cache_bytes,"loaderStartupMs":loader_ms,"prepareAllMs":prepare_ms,"fullColdStartMs":total_ms,"readyDevices":ready,"failedDevices":errors,"runtimeRoot":dispatcher.runtime_root()})
    );
    dispatcher.shutdown().await;
    tokio::time::sleep(Duration::from_millis(50)).await;
}

async fn impact(artifact: &Path) {
    let iterations: usize = arg("--iterations", "150").parse().unwrap();
    let delay_ms: u64 = arg("--pair-delay-ms", "120").parse().unwrap();
    let work = temp("impact");
    let db = work.join("database.db");
    write_db(&db, &[ikea_record()]);
    let fetcher = Arc::new(Source::new(
        artifact.to_path_buf(),
        Duration::from_millis(delay_ms),
    ));
    let dispatcher = ConverterDispatcher::start_lazy(
        DispatcherConfig::new(artifact, herbsman_root()),
        Arc::new(OfflineMockTransport::new()),
        lazy_config(artifact, &work),
        db,
        fetcher.clone(),
        work.join("runtime"),
        true,
    )
    .await
    .unwrap();
    inject_database(Arc::new(Mutex::new(Database::open(work.join("probe.db")))));
    ZhDevice::reset_cache();
    let ikea = create_device(
        "0x0017880104e45531",
        4660,
        "IKEA of Sweden",
        "TRADFRI bulb E27 WS opal 1000lm",
        DeviceType::Router,
        &[0, 3, 4, 5, 6, 8, 768],
        &[25],
    );
    let prepared = dispatcher.prepare(&ikea).await.unwrap();
    let source = prepared.source.clone();
    let model = prepared.lookup_model.clone();
    let measure = |dispatcher: Arc<ConverterDispatcher>,
                   ikea: Arc<ZhDevice>,
                   source: String,
                   model: String,
                   done: Arc<AtomicBool>| async move {
        let mut samples = Vec::new();
        let mut before_done = 0usize;
        for _ in 0..iterations {
            let started = Instant::now();
            let meta = ConvertMeta {
                device: ikea.clone(),
                state: json!({}),
                device_exposes_changed: Arc::new(|| {}),
            };
            let data = ConvertData {
                r#type: "attributeReport".to_owned(),
                device: ikea.clone(),
                endpoint: ikea.get_endpoint(1).unwrap(),
                cluster: "genOnOff".to_owned(),
                data: json!({"onOff": true}),
                group_id: None,
                linkquality: 120,
                meta: json!({}),
            };
            dispatcher
                .from_zigbee(
                    &source,
                    &model,
                    data,
                    Arc::new(|_| Box::pin(async {})),
                    json!({}),
                    meta,
                )
                .await
                .unwrap();
            samples.push(started.elapsed().as_secs_f64() * 1000.0);
            if !done.load(Ordering::Relaxed) {
                before_done += 1;
            }
        }
        (samples, before_done)
    };
    let finished = Arc::new(AtomicBool::new(false));
    let (mut baseline, _) = measure(
        Arc::clone(&dispatcher),
        ikea.clone(),
        source.clone(),
        model.clone(),
        finished.clone(),
    )
    .await;
    let tuya = create_device(
        "0x0017880104e45541",
        0x1240,
        "_TZ3000_abcd",
        "TS0002",
        DeviceType::Router,
        &[0, 6],
        &[25],
    );
    let pairing_started = Instant::now();
    let pairing = tokio::spawn({
        let dispatcher = Arc::clone(&dispatcher);
        let tuya = tuya.clone();
        let finished = Arc::clone(&finished);
        async move {
            let result = dispatcher.pair_device(&tuya).await;
            finished.store(true, Ordering::Relaxed);
            result
        }
    });
    let (mut during, before_done) = measure(
        Arc::clone(&dispatcher),
        ikea.clone(),
        source,
        model,
        finished.clone(),
    )
    .await;
    let pairing_result = pairing.await.unwrap();
    let pairing_ms = pairing_started.elapsed().as_secs_f64() * 1000.0;
    println!(
        "{}",
        json!({
            "mode":"impact",
            "iterations":iterations,
            "pairDelayMs":delay_ms,
            "pairOk":pairing_result.is_ok(),
            "pairError":pairing_result.err(),
            "pairingMs":pairing_ms,
            "samplesOverlappingPairing":before_done,
            "baselineP50Ms":percentile(&mut baseline, 0.50),
            "baselineP95Ms":percentile(&mut baseline, 0.95),
            "duringP50Ms":percentile(&mut during, 0.50),
            "duringP95Ms":percentile(&mut during, 0.95),
            "duringMaxMs":during.iter().copied().fold(0.0_f64, f64::max),
            "fetchCalls":fetcher.calls(),
        })
    );
    dispatcher.shutdown().await;
    tokio::time::sleep(Duration::from_millis(50)).await;
}

fn single_flight(artifact: &Path) {
    let delay_ms: u64 = arg("--fetch-delay-ms", "30").parse().unwrap();
    let work = temp("single-flight");
    let db = work.join("database.db");
    write_db(&db, &[ikea_record(), nous_record(1)]);
    let mut records = read_database(&db).unwrap();
    records.push(converter_artifact::DatabaseRecord {
        ieee_addr: "0x0017880104e45542".to_owned(),
        r#type: Some("Router".to_owned()),
        model_id: Some("TS0002".to_owned()),
        manufacturer_name: Some("_TZ3000_abcd".to_owned()),
        manufacturer_id: Some(4417),
        power_source: Some("Mains".to_owned()),
        application_version: None,
        stack_version: None,
        hardware_version: None,
        zcl_version: None,
        date_code: None,
        software_build_id: None,
        endpoints: Vec::new(),
    });
    let profiles: Vec<DeviceProfile> = records
        .iter()
        .filter_map(|record| record.to_profile())
        .collect();
    let delay = Duration::from_millis(delay_ms);
    let sequential_cache = work.join("sequential");
    let sequential_fetcher = Source::new(artifact.to_path_buf(), delay);
    let sequential_started = Instant::now();
    {
        let loader = LazyLoader::open(lazy_config(artifact, &sequential_cache)).unwrap();
        for profile in &profiles {
            loader
                .load_device(profile, &sequential_fetcher, true)
                .unwrap();
        }
    }
    let sequential_ms = sequential_started.elapsed().as_secs_f64() * 1000.0;
    let concurrent_cache = work.join("concurrent");
    let concurrent_fetcher = Arc::new(Source::new(artifact.to_path_buf(), delay));
    let concurrent_started = Instant::now();
    {
        let loader = Arc::new(LazyLoader::open(lazy_config(artifact, &concurrent_cache)).unwrap());
        std::thread::scope(|scope| {
            for profile in &profiles {
                let loader = Arc::clone(&loader);
                let fetcher = Arc::clone(&concurrent_fetcher);
                scope.spawn(move || {
                    loader.load_device(profile, fetcher.as_ref(), true).unwrap();
                });
            }
        });
    }
    let concurrent_ms = concurrent_started.elapsed().as_secs_f64() * 1000.0;
    println!(
        "{}",
        json!({
            "mode":"single-flight",
            "devices":profiles.len(),
            "fetchDelayMs":delay_ms,
            "sequentialMs":sequential_ms,
            "concurrentMs":concurrent_ms,
            "overheadPct": if sequential_ms > 0.0 { (concurrent_ms / sequential_ms - 1.0) * 100.0 } else { 0.0 },
            "sequentialFetchCalls":sequential_fetcher.calls(),
            "concurrentFetchCalls":concurrent_fetcher.calls(),
        })
    );
}
