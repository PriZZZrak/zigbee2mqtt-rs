use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use converter_artifact::{
    CacheConfig, FetchError, LazyConfig, LazyLoader, ModuleFetcher, ModuleRecord,
};
use serde_json::json;

struct Source {
    dir: PathBuf,
    delay: Duration,
    bytes: AtomicU64,
    calls: AtomicU64,
}
impl ModuleFetcher for Source {
    fn fetch(&self, module: &ModuleRecord) -> Result<Vec<u8>, FetchError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if !self.delay.is_zero() {
            std::thread::sleep(self.delay);
        }
        let data = std::fs::read(self.dir.join(&module.file)).map_err(|_| FetchError::NotFound)?;
        self.bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
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
    let d = std::env::temp_dir().join(format!("z2m-lazy-probe-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}
fn synthetic_db(path: &Path, model: &str, manufacturer: &str, count: usize) {
    let mut text = String::new();
    for i in 0..count {
        let record = json!({
            "type": "EndDevice",
            "ieeeAddr": format!("0x0017880104e4{:04x}", i),
            "nwkAddr": 0x1000 + i,
            "manufId": 4417,
            "manufName": manufacturer,
            "powerSource": "Battery",
            "modelId": model,
            "epList": [1],
            "endpoints": {"1": {"profId": 260, "epId": 1, "devId": 81, "inClusterList": [0,4,5,61184], "outClusterList": [25,10]}}
        });
        text.push_str(&record.to_string());
        text.push('\n');
    }
    std::fs::write(path, text).unwrap();
}

fn percentile(values: &mut [f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    values[((values.len() - 1) as f64 * p).round() as usize]
}
fn main() {
    let artifact = PathBuf::from(arg("--artifact", "/tmp/engine-study/artifact"));
    let model = arg("--model", "TS0601");
    let manufacturer = arg("--manufacturer", "_TZE200_locansqn");
    let devices: usize = arg("--devices", "20").parse().unwrap();
    let iterations: usize = arg("--iterations", "50").parse().unwrap();
    let dir = temp("work");
    let db = dir.join("database.db");
    synthetic_db(&db, &model, &manufacturer, devices);
    let cache_root = dir.join("cache");
    let config = LazyConfig {
        artifact_dir: artifact.clone(),
        cache: CacheConfig::new(&cache_root),
        host_abi: 1,
        engine: "quickjs-oxide".to_owned(),
        engine_version: "0.5.0".to_owned(),
    };
    let loader = Arc::new(LazyLoader::open(config).unwrap());
    let source = Arc::new(Source {
        dir: artifact.clone(),
        delay: Duration::from_millis(0),
        bytes: AtomicU64::new(0),
        calls: AtomicU64::new(0),
    });
    let started = Instant::now();
    let report = loader.startup(&db, source.as_ref(), true).unwrap();
    let cold_ms = started.elapsed().as_secs_f64() * 1000.0;
    // Baseline existing-device startup with a warm cache.
    let mut baseline = Vec::new();
    for _ in 0..iterations {
        let t = Instant::now();
        let _ = loader.startup(&db, source.as_ref(), true);
        baseline.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    // Existing-device startup under a concurrent pairing download.
    let pair_profile = converter_artifact::DeviceProfile {
        r#type: converter_artifact::DeviceType::EndDevice,
        model_id: Some(arg("--pair-model", "TS0601")),
        manufacturer_name: Some(arg("--pair-manufacturer", "_TZE284_iwyqtclw")),
        ..Default::default()
    };
    let pair_fetcher: Arc<dyn ModuleFetcher> = Arc::new(Source {
        dir: artifact,
        delay: Duration::from_millis(20),
        bytes: AtomicU64::new(0),
        calls: AtomicU64::new(0),
    });
    let pair = loader.pair(pair_profile, pair_fetcher, true);
    std::thread::sleep(Duration::from_millis(5));
    let mut samples = Vec::new();
    for _ in 0..iterations {
        let t = Instant::now();
        let _ = loader.startup(&db, source.as_ref(), true);
        samples.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    let paired = pair.join().unwrap();
    let cache_after = loader.cache().cache_bytes();
    println!(
        "{}",
        serde_json::json!({"devices":devices,"resolved":report.resolved,"modulesLoaded":report.modules_loaded,"networkBytes":report.network_bytes,"cacheBytes":report.cache_bytes,"cacheBytesAfterPairing":cache_after,"coldStartMs":cold_ms,"pairingOk":paired.is_ok(),"baselineP50Ms":percentile(&mut baseline,0.5),"baselineP95Ms":percentile(&mut baseline,0.95),"startupUnderPairingP50Ms":percentile(&mut samples,0.5),"startupUnderPairingP95Ms":percentile(&mut samples,0.95),"fetches":source.calls.load(Ordering::Relaxed)})
    );
}
