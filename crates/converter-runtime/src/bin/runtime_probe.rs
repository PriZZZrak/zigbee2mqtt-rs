// Minimal runtime probe: loads the real upstream package through the runtime,
// submits golden scenarios or a scheduler mix and answers host operations from
// a test responder. Not a bridge integration.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use converter_runtime::{ConverterRuntime, HostResult, JobRequest, RuntimeConfig, SubmitError};
use serde_json::{Value as Json, json};

fn rss_kb() -> u64 {
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
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

fn arg(name: &str, fallback: &str) -> String {
    let mut iter = std::env::args().skip(1);
    while let Some(flag) = iter.next() {
        if flag == name {
            return iter.next().expect("missing value");
        }
    }
    fallback.to_owned()
}

fn main() {
    let package = arg(
        "--package",
        ".upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters",
    );
    let herdsman = arg(
        "--herdsman",
        ".upstream/zigbee2mqtt/node_modules/zigbee-herdsman",
    );
    let external = arg("--external", "tools/bench/fixtures/external/L101Ze_z2m.js");
    let scenarios_path = arg("--scenarios", "tools/engine-study/fixtures/scenarios.json");
    let mode = arg("--mode", "all");
    let jobs: usize = arg("--jobs", "8").parse().expect("jobs");
    let max_concurrent: usize = arg("--max-concurrent", "1")
        .parse()
        .expect("max-concurrent");
    let host_delay_ms: u64 = arg("--host-delay-ms", "0").parse().expect("delay");
    let timeout_ms: u64 = arg("--timeout-ms", "15000").parse().expect("timeout");
    let warmup = arg("--warmup", "1") == "1";
    let host_mode = arg("--host-mode", "ok");
    let host_timeout_ms: u64 = arg("--host-timeout-ms", "5000")
        .parse()
        .expect("host timeout");
    let job_timeout_ms: u64 = arg("--job-timeout-ms", "10000")
        .parse()
        .expect("job timeout");
    let cancel_after_ms: u64 = arg("--cancel-after-ms", "0").parse().expect("cancel delay");
    let queue_cap: usize = arg("--queue-cap", "16").parse().expect("queue cap");
    let duration_ms: u64 = arg("--duration-ms", "20000").parse().expect("duration");
    let heap_mb: usize = arg("--heap-mb", "128").parse().expect("heap mb");
    let total_cap: usize = arg("--total-cap", "128").parse().expect("total cap");

    let scenarios: Json = serde_json::from_str(
        &std::fs::read_to_string(Path::new(&scenarios_path)).expect("read scenarios"),
    )
    .expect("parse scenarios");
    let scenarios = scenarios.as_array().expect("scenario array").clone();

    let mut config = RuntimeConfig::new(package, herdsman);
    config.external_path = external.into();
    config.max_concurrent_jobs = max_concurrent;
    config.max_per_device_queue = queue_cap;
    config.max_total_queue = total_cap;
    config.host_timeout = Duration::from_millis(host_timeout_ms);
    config.heap_limit_bytes = heap_mb << 20;
    config.job_timeout = Duration::from_millis(job_timeout_ms);
    let runtime = Arc::new(ConverterRuntime::new(config).expect("runtime"));

    let served = Arc::new(AtomicU64::new(0));
    let stop_responder = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_responder_host = Arc::clone(&stop_responder);
    let served_host = Arc::clone(&served);
    let runtime_host = Arc::clone(&runtime);
    let responder = thread::spawn(move || {
        let receiver = runtime_host.host_receiver();
        let mut mixed = 0u64;
        loop {
            let request = match receiver.recv_timeout(Duration::from_millis(200)) {
                Ok(request) => request,
                Err(error) => {
                    if stop_responder_host.load(Ordering::Relaxed) {
                        break;
                    }
                    let _ = error;
                    continue;
                }
            };
            served_host.fetch_add(1, Ordering::Relaxed);
            if host_delay_ms > 0 {
                thread::sleep(Duration::from_millis(host_delay_ms));
            }
            mixed += 1;
            let effective_mode = if host_mode == "mix" {
                if mixed % 7 == 0 {
                    "drop"
                } else if mixed % 11 == 0 {
                    "error"
                } else if mixed % 13 == 0 {
                    "late"
                } else {
                    "ok"
                }
            } else {
                host_mode.as_str()
            };
            match effective_mode {
                "drop" => {}
                "error" => {
                    let _ = runtime_host.respond(
                        request.correlation_id,
                        HostResult::Err("host EIO".to_owned()),
                    );
                }
                "late" => {
                    thread::sleep(Duration::from_millis(host_delay_ms));
                    let _ =
                        runtime_host.respond(request.correlation_id, HostResult::Ok(json!(true)));
                }
                _ => {
                    let _ =
                        runtime_host.respond(request.correlation_id, HostResult::Ok(json!(true)));
                }
            }
        }
    });

    let started = Instant::now();
    let mut ids = Vec::new();
    let mut labels = std::collections::HashMap::new();
    let mut submit_errors = 0usize;
    let configure = scenarios
        .iter()
        .find(|scenario| scenario["id"] == "configure-danfoss")
        .cloned();
    let set_job = scenarios
        .iter()
        .find(|scenario| scenario["id"] == "set-ikea-brightness")
        .cloned();
    let report_job = scenarios
        .iter()
        .find(|scenario| scenario["id"] == "fz-ikea-onoff")
        .cloned();
    let mut warmup_ms = 0.0;
    if warmup {
        let warm_started = Instant::now();
        let mut warm_ids = Vec::new();
        if let Some(configure) = &configure {
            if let Ok(id) = runtime.submit(JobRequest {
                device: "warmup-configure".to_owned(),
                scenario: configure.clone(),
            }) {
                warm_ids.push(id);
            }
        }
        if let Some(job) = &set_job {
            if let Ok(id) = runtime.submit(JobRequest {
                device: "warmup-set".to_owned(),
                scenario: job.clone(),
            }) {
                warm_ids.push(id);
            }
        }
        for id in warm_ids {
            let _ = runtime.wait(id, Duration::from_millis(timeout_ms));
        }
        warmup_ms = warm_started.elapsed().as_secs_f64() * 1000.0;
    }

    if mode == "all" {
        for scenario in &scenarios {
            let device = scenario["id"].as_str().unwrap_or("bench").to_owned();
            match runtime.submit(JobRequest {
                device,
                scenario: scenario.clone(),
            }) {
                Ok(id) => {
                    labels.insert(id, scenario["id"].as_str().unwrap_or("?").to_owned());
                    ids.push(id);
                }
                Err(_) => submit_errors += 1,
            }
        }
    } else if mode == "same-device" {
        if let Some(job) = set_job {
            for index in 0..jobs {
                let mut scenario = job.clone();
                scenario["device"]["ieeeAddr"] = json!("0x00124b0000000aaa");
                scenario["device"]["networkAddress"] = json!(0xaaa);
                let _ = index;
                match runtime.submit(JobRequest {
                    device: "same-device".to_owned(),
                    scenario,
                }) {
                    Ok(id) => {
                        labels.insert(id, "set-ikea-brightness".to_owned());
                        ids.push(id);
                    }
                    Err(SubmitError::Backpressure) => submit_errors += 1,
                    Err(SubmitError::Closed) => break,
                }
            }
        }
    } else if mode == "soak" {
        let deadline = Instant::now() + Duration::from_millis(duration_ms);
        let reports = scenarios
            .iter()
            .filter(|s| s["kind"] == "fromZigbee")
            .cloned()
            .collect::<Vec<_>>();
        let sets = scenarios
            .iter()
            .filter(|s| s["kind"] == "set")
            .cloned()
            .collect::<Vec<_>>();
        let mut round = 0usize;
        while Instant::now() < deadline {
            let mut batch = Vec::new();
            for index in 0..16 {
                let pool = if index % 2 == 0 { &reports } else { &sets };
                if pool.is_empty() {
                    continue;
                }
                let mut scenario = pool[(round + index) % pool.len()].clone();
                scenario["device"]["ieeeAddr"] = json!(format!("0x00124b{:010x}", 0x4000 + index));
                scenario["device"]["networkAddress"] = json!(0x4000 + index);
                if let Ok(id) = runtime.submit(JobRequest {
                    device: format!("soak-{index}"),
                    scenario,
                }) {
                    if index % 5 == 0 {
                        let _ = runtime.cancel(id);
                    }
                    batch.push(id);
                } else {
                    submit_errors += 1;
                }
            }
            for id in batch {
                let _ = runtime.wait(id, Duration::from_millis(500));
            }
            round += 1;
            thread::sleep(Duration::from_millis(5));
        }
    } else if mode == "js-timeout" {
        let scenario = json!({
            "id": "js-loop",
            "kind": "jsLoop",
            "definition": {"source": "package", "file": "ikea.js", "model": "LED1545G12"},
            "device": {"ieeeAddr": "0x00124b0000000bbb", "networkAddress": 0xbbb}
        });
        let first = runtime
            .submit(JobRequest {
                device: "js-timeout-device".to_owned(),
                scenario,
            })
            .expect("js loop job");
        labels.insert(first, "js-loop".to_owned());
        ids.push(first);
        let _ = runtime.wait(first, Duration::from_millis(timeout_ms));
        let Some(report) = report_job.clone() else {
            panic!("report missing")
        };
        let second = runtime
            .submit(JobRequest {
                device: "js-timeout-device".to_owned(),
                scenario: report,
            })
            .expect("reuse after js timeout");
        labels.insert(second, "fz-ikea-onoff".to_owned());
        ids.push(second);
    } else if mode == "timeout-reuse" || mode == "cancel-reuse" {
        let Some(configure) = configure.clone() else {
            panic!("configure scenario missing");
        };
        let Some(report) = report_job.clone() else {
            panic!("report scenario missing");
        };
        let first = runtime
            .submit(JobRequest {
                device: "reuse-device".to_owned(),
                scenario: configure,
            })
            .expect("first job");
        labels.insert(first, "configure-danfoss".to_owned());
        ids.push(first);
        if mode == "cancel-reuse" {
            thread::sleep(Duration::from_millis(20));
            runtime.cancel(first);
        }
        let _ = runtime.wait(first, Duration::from_millis(timeout_ms));
        let second = runtime
            .submit(JobRequest {
                device: "reuse-device".to_owned(),
                scenario: report,
            })
            .expect("reuse job");
        labels.insert(second, "fz-ikea-onoff".to_owned());
        ids.push(second);
    } else if mode == "mix" {
        if let Some(configure) = configure {
            let device = configure["device"]["ieeeAddr"]
                .as_str()
                .unwrap_or("cfg")
                .to_owned();
            match runtime.submit(JobRequest {
                device,
                scenario: configure,
            }) {
                Ok(id) => {
                    labels.insert(id, "configure-danfoss".to_owned());
                    ids.push(id);
                }
                Err(_) => submit_errors += 1,
            }
        }
        if let Some(job) = set_job {
            for index in 0..jobs {
                let mut scenario = job.clone();
                scenario["device"]["ieeeAddr"] = json!(format!("0x00124b{:010x}", 0x3000 + index));
                scenario["device"]["networkAddress"] = json!(0x3000 + index);
                match runtime.submit(JobRequest {
                    device: scenario["device"]["ieeeAddr"].as_str().unwrap().to_owned(),
                    scenario,
                }) {
                    Ok(id) => {
                        labels.insert(id, "set-ikea-brightness".to_owned());
                        ids.push(id);
                    }
                    Err(SubmitError::Backpressure) => submit_errors += 1,
                    Err(SubmitError::Closed) => break,
                }
            }
        }
    } else {
        panic!("--mode must be all or mix");
    }

    if cancel_after_ms > 0 {
        thread::sleep(Duration::from_millis(cancel_after_ms));
        if let Some(id) = ids.first() {
            runtime.cancel(*id);
        }
    }
    let mut outcomes = Vec::new();
    for id in &ids {
        match runtime.wait(*id, Duration::from_millis(timeout_ms)) {
            Ok(outcome) => outcomes.push(outcome),
            Err(error) => outcomes.push(converter_runtime::JobOutcome {
                id: *id,
                device: String::new(),
                state: converter_runtime::JobState::Failed,
                result: None,
                error: Some(format!("wait: {error:?}")),
                metrics: Default::default(),
            }),
        }
    }
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    let metrics = runtime.metrics();
    stop_responder.store(true, Ordering::Relaxed);
    responder.join().ok();
    let mut runtime = Arc::try_unwrap(runtime).ok().expect("runtime still shared");
    runtime.shutdown();

    let cases: Vec<Json> = outcomes
        .iter()
        .map(|outcome| {
            json!({
                "id": outcome.id,
                "scenarioId": labels.get(&outcome.id).cloned().unwrap_or_default(),
                "device": outcome.device,
                "state": format!("{:?}", outcome.state),
                "error": outcome.error,
                "result": outcome.result,
                "metrics": {
                    "queueWaitMs": outcome.metrics.queue_wait_ms,
                    "evalMs": outcome.metrics.eval_ms,
                    "jsExecMs": outcome.metrics.js_exec_ms,
                    "promiseSchedMs": outcome.metrics.promise_sched_ms,
                    "hostWaitMs": outcome.metrics.host_wait_ms,
                    "totalMs": outcome.metrics.total_ms,
                    "hostCalls": outcome.metrics.host_calls,
                    "gcCollections": outcome.metrics.gc_collections,
                    "gcMaxPauseMs": outcome.metrics.gc_max_pause_ms,
                    "heapLiveBytes": outcome.metrics.heap_live_bytes,
                },
            })
        })
        .collect();

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "engine": "quickjs-oxide",
            "mode": mode,
            "maxConcurrent": max_concurrent,
            "hostDelayMs": host_delay_ms,
            "submitted": ids.len(),
            "submitErrors": submit_errors,
            "servedHostOps": served.load(Ordering::Relaxed),
            "elapsedMs": elapsed_ms,
            "startupMs": metrics.startup_ms,
            "warmupMs": warmup_ms,
            "completed": metrics.completed,
            "failed": metrics.failed,
            "cancelled": metrics.cancelled,
            "lateResponses": metrics.late_responses,
            "hostTimeouts": metrics.host_timeouts,
            "gcCollections": metrics.gc_collections,
            "gcMaxPauseMs": metrics.gc_max_pause_ms,
            "heapLiveBytes": metrics.heap_live_bytes,
            "rssKb": rss_kb(),
            "cases": cases,
        }))
        .unwrap()
    );
}
