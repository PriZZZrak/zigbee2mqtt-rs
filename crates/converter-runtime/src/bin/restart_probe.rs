// Restart memory probe: sequential shutdown-before-create strategy.
use std::path::PathBuf;
use std::time::{Duration, Instant};

use converter_runtime::{ConverterRuntime, JobRequest, RuntimeConfig};
use serde_json::json;

fn arg(name: &str, fallback: &str) -> String {
    let mut iter = std::env::args().skip(1);
    while let Some(flag) = iter.next() {
        if flag == name {
            return iter.next().expect("missing value");
        }
    }
    fallback.to_owned()
}

fn status_kb(field: &str) -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix(field) {
            return rest
                .trim()
                .trim_end_matches("kB")
                .trim()
                .parse()
                .unwrap_or(0);
        }
    }
    0
}

fn main() {
    let package = PathBuf::from(arg(
        "--package",
        ".upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters",
    ));
    let herdsman = PathBuf::from(arg(
        "--herdsman",
        ".upstream/zigbee2mqtt/node_modules/zigbee-herdsman",
    ));
    let external = PathBuf::from(arg(
        "--external",
        "tools/bench/fixtures/external/L101Ze_z2m.js",
    ));
    let mut config = RuntimeConfig::new(package, herdsman);
    config.external_path = external;
    config.heap_limit_bytes = 32 << 20;
    config.job_timeout = Duration::from_secs(5);
    let mut runtime = ConverterRuntime::new(config).expect("runtime");

    let submit_timer = |runtime: &ConverterRuntime| {
        runtime
            .submit(JobRequest {
                device: "restart-device".to_owned(),
                scenario: json!({"id": "timer", "kind": "timer", "delay": 5}),
            })
            .expect("timer job")
    };
    let id = submit_timer(&runtime);
    let _ = runtime.wait(id, Duration::from_secs(2));
    let rss_before = status_kb("VmRSS:");
    let peak_before = status_kb("VmHWM:");
    let started = Instant::now();
    let status = runtime.restart().expect("restart");
    let restart_ms = started.elapsed().as_secs_f64() * 1000.0;
    let rss_after = status_kb("VmRSS:");
    let peak_after = status_kb("VmHWM:");
    let id = submit_timer(&runtime);
    let outcome = runtime.wait(id, Duration::from_secs(2)).expect("wait");
    let second_ok = outcome.state == converter_runtime::JobState::Completed;
    runtime.shutdown();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "status": format!("{status:?}"),
            "restartMs": restart_ms,
            "rssBeforeKb": rss_before,
            "rssAfterKb": rss_after,
            "peakBeforeKb": peak_before,
            "peakAfterKb": peak_after,
            "secondJobCompleted": second_ok,
        }))
        .unwrap()
    );
}
