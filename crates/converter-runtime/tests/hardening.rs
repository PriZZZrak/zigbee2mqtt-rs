//! Hardening regressions. These start the real runtime and require the pinned
//! `.upstream` package; run with:
//! `cargo test -p converter-runtime --test hardening -- --ignored --test-threads=1`

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use converter_runtime::{ConverterRuntime, HostResult, JobRequest, RuntimeConfig};
use serde_json::{Value as Json, json};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn config() -> RuntimeConfig {
    let root = root();
    let mut config = RuntimeConfig::new(
        root.join(".upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters"),
        root.join(".upstream/zigbee2mqtt/node_modules/zigbee-herdsman"),
    );
    config.external_path = root.join("tools/bench/fixtures/external/L101Ze_z2m.js");
    config.job_timeout = Duration::from_millis(800);
    config.host_timeout = Duration::from_millis(100);
    config.max_concurrent_jobs = 1;
    config
}

fn scenarios() -> Vec<Json> {
    let path = root().join("tools/engine-study/fixtures/scenarios.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn responder(
    runtime: Arc<ConverterRuntime>,
    drop_host: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let receiver = runtime.take_host_receiver().expect("host receiver");
        while let Ok(request) = receiver.recv_timeout(Duration::from_millis(50)) {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            if drop_host.load(Ordering::Relaxed) {
                continue;
            }
            let _ = runtime.respond(request.correlation_id, HostResult::Ok(json!(true)));
        }
    })
}

#[test]
#[ignore = "requires the pinned .upstream converter package"]
fn timeout_then_same_device_job_completes() {
    let config = config();
    if !config.package_root.exists() {
        return;
    }
    let scenarios = scenarios();
    let configure = scenarios
        .iter()
        .find(|s| s["id"] == "configure-danfoss")
        .unwrap()
        .clone();
    let report = scenarios
        .iter()
        .find(|s| s["id"] == "fz-ikea-onoff")
        .unwrap()
        .clone();

    let runtime = Arc::new(ConverterRuntime::new(config).unwrap());
    let drop_host = Arc::new(AtomicBool::new(true));
    let stop = Arc::new(AtomicBool::new(false));
    let handle = responder(
        Arc::clone(&runtime),
        Arc::clone(&drop_host),
        Arc::clone(&stop),
    );

    let first = runtime
        .submit(JobRequest {
            device: "regression-device".to_owned(),
            scenario: configure,
        })
        .unwrap();
    let first_outcome = runtime.wait(first, Duration::from_secs(3)).unwrap();
    assert!(matches!(
        first_outcome.state,
        converter_runtime::JobState::Failed | converter_runtime::JobState::Cancelled
    ));

    drop_host.store(false, Ordering::Relaxed);
    let second = runtime
        .submit(JobRequest {
            device: "regression-device".to_owned(),
            scenario: report,
        })
        .unwrap();
    let second_outcome = runtime.wait(second, Duration::from_secs(5)).unwrap();
    assert_eq!(
        second_outcome.state,
        converter_runtime::JobState::Completed,
        "{:?}",
        second_outcome.error
    );

    stop.store(true, Ordering::Relaxed);
    handle.join().ok();
    let mut runtime = Arc::try_unwrap(runtime).ok().unwrap();
    runtime.shutdown();
}

#[test]
#[ignore = "requires the pinned .upstream converter package"]
fn cancel_then_same_device_job_completes() {
    let config = config();
    if !config.package_root.exists() {
        return;
    }
    let scenarios = scenarios();
    let configure = scenarios
        .iter()
        .find(|s| s["id"] == "configure-danfoss")
        .unwrap()
        .clone();
    let report = scenarios
        .iter()
        .find(|s| s["id"] == "fz-ikea-onoff")
        .unwrap()
        .clone();

    let runtime = Arc::new(ConverterRuntime::new(config).unwrap());
    let drop_host = Arc::new(AtomicBool::new(false));
    let stop = Arc::new(AtomicBool::new(false));
    let handle = responder(
        Arc::clone(&runtime),
        Arc::clone(&drop_host),
        Arc::clone(&stop),
    );

    let first = runtime
        .submit(JobRequest {
            device: "cancel-device".to_owned(),
            scenario: configure,
        })
        .unwrap();
    thread::sleep(Duration::from_millis(20));
    assert!(runtime.cancel(first));
    let first_outcome = runtime.wait(first, Duration::from_secs(3)).unwrap();
    assert_eq!(first_outcome.state, converter_runtime::JobState::Cancelled);

    let second = runtime
        .submit(JobRequest {
            device: "cancel-device".to_owned(),
            scenario: report,
        })
        .unwrap();
    let second_outcome = runtime.wait(second, Duration::from_secs(5)).unwrap();
    assert_eq!(
        second_outcome.state,
        converter_runtime::JobState::Completed,
        "{:?}",
        second_outcome.error
    );

    stop.store(true, Ordering::Relaxed);
    handle.join().ok();
    let mut runtime = Arc::try_unwrap(runtime).ok().unwrap();
    runtime.shutdown();
}
