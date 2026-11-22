// Timer/lifecycle tests that do not need the upstream device database.
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use converter_runtime::{
    CancelStatus, ConverterRuntime, HostResult, JobRequest, JobState, RuntimeConfig, SubmitError,
    WorkerHealth,
};
use serde_json::json;

fn config() -> RuntimeConfig {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixture-package");
    let mut config = RuntimeConfig::new(fixture.clone(), fixture.clone());
    config.external_path = fixture.join("no-external.js");
    config.max_concurrent_jobs = 1;
    config.job_timeout = Duration::from_millis(300);
    config.host_timeout = Duration::from_millis(100);
    config.max_rss_bytes = 512 << 20;
    config
}

fn timer_job(delay: u64, clear: bool, unref: bool) -> JobRequest {
    JobRequest {
        device: "timer-device".to_owned(),
        scenario: json!({
            "id": "timer",
            "kind": "timer",
            "delay": delay,
            "clear": clear,
            "unref": unref,
        }),
    }
}

#[test]
fn timer_fires_and_clear_cancels() {
    let mut runtime = ConverterRuntime::new(config()).unwrap();
    let fired = runtime.submit(timer_job(20, false, false)).unwrap();
    let outcome = runtime.wait(fired, Duration::from_secs(2)).unwrap();
    assert_eq!(outcome.state, JobState::Completed);
    assert_eq!(outcome.result.unwrap()["fired"], true);

    let cleared = runtime.submit(timer_job(20, true, false)).unwrap();
    let outcome = runtime.wait(cleared, Duration::from_secs(2)).unwrap();
    assert_eq!(outcome.state, JobState::Completed);
    assert_eq!(outcome.result.unwrap()["fired"], false);
    runtime.shutdown();
}

#[test]
fn unref_timer_still_runs_while_runtime_is_alive() {
    let mut runtime = ConverterRuntime::new(config()).unwrap();
    let id = runtime.submit(timer_job(20, false, true)).unwrap();
    let outcome = runtime.wait(id, Duration::from_secs(2)).unwrap();
    assert_eq!(outcome.state, JobState::Completed);
    assert_eq!(outcome.result.unwrap()["fired"], true);
    runtime.shutdown();
}

#[test]
fn timer_can_outlive_the_converter_call() {
    let mut runtime = ConverterRuntime::new(config()).unwrap();
    let scheduled = runtime
        .submit(JobRequest {
            device: "timer-late".to_owned(),
            scenario: json!({
                "id": "timer-late",
                "kind": "timer-late",
                "delay": 20,
            }),
        })
        .unwrap();
    assert_eq!(
        runtime
            .wait(scheduled, Duration::from_secs(1))
            .unwrap()
            .state,
        JobState::Completed
    );
    thread::sleep(Duration::from_millis(80));
    let check = runtime
        .submit(JobRequest {
            device: "timer-late".to_owned(),
            scenario: json!({
                "id": "timer-check",
                "kind": "timer-check",
            }),
        })
        .unwrap();
    let outcome = runtime.wait(check, Duration::from_secs(1)).unwrap();
    assert_eq!(outcome.state, JobState::Completed);
    assert_eq!(outcome.result.unwrap()["lateTimerFired"], true);
    runtime.shutdown();
}

#[test]
fn js_timeout_cancels_and_same_device_recovers() {
    let mut config = config();
    config.job_timeout = Duration::from_millis(100);
    let mut runtime = ConverterRuntime::new(config).unwrap();
    let loop_id = runtime
        .submit(JobRequest {
            device: "js-device".to_owned(),
            scenario: json!({"id": "loop", "kind": "jsLoop"}),
        })
        .unwrap();
    let loop_outcome = runtime.wait(loop_id, Duration::from_secs(2)).unwrap();
    assert_eq!(loop_outcome.state, JobState::Cancelled);
    assert!(loop_outcome.error.unwrap().contains("JS execution"));

    let timer = runtime.submit(timer_job(10, false, false)).unwrap();
    let outcome = runtime.wait(timer, Duration::from_secs(2)).unwrap();
    assert_eq!(outcome.state, JobState::Completed);
    runtime.shutdown();
}

#[test]
fn cancellation_acceptance_is_separate_from_terminal_state() {
    let mut runtime = ConverterRuntime::new(config()).unwrap();
    let long = runtime
        .submit(JobRequest {
            device: "cancel-device".to_owned(),
            scenario: json!({
                "id": "loop",
                "kind": "jsLoop",
            }),
        })
        .unwrap();
    thread::sleep(Duration::from_millis(30));
    assert_eq!(runtime.cancel_request(long), CancelStatus::Accepted);
    let outcome = runtime.wait(long, Duration::from_secs(2)).unwrap();
    assert_eq!(outcome.state, JobState::Cancelled);
    assert_eq!(runtime.cancel_request(long), CancelStatus::AlreadyTerminal);
    runtime.shutdown();
}

#[test]
fn admission_and_shutdown_are_bounded() {
    let mut config = config();
    config.max_total_queue = 2;
    config.max_per_device_queue = 2;
    let mut runtime = ConverterRuntime::new(config).unwrap();
    let mut ids = Vec::new();
    for _ in 0..2 {
        ids.push(runtime.submit(timer_job(20, false, false)).unwrap());
    }
    assert_eq!(
        runtime.submit(timer_job(20, false, false)),
        Err(SubmitError::Backpressure)
    );
    for id in ids {
        let _ = runtime.wait(id, Duration::from_secs(2));
    }

    // A pending 10 s timer must not block shutdown.
    let pending = runtime.submit(timer_job(10_000, false, false)).unwrap();
    thread::sleep(Duration::from_millis(20));
    let started = Instant::now();
    runtime.shutdown();
    assert!(started.elapsed() < Duration::from_secs(3));
    let _ = pending;
}

#[test]
fn duplicate_host_response_is_counted_as_late() {
    let mut runtime = ConverterRuntime::new(config()).unwrap();
    let id = runtime
        .submit(JobRequest {
            device: "duplicate".to_owned(),
            scenario: json!({"id": "dup", "kind": "host-op", "payload": "ok"}),
        })
        .unwrap();
    let receiver = runtime.host_receiver();
    let request = receiver.recv_timeout(Duration::from_secs(2)).unwrap();
    let correlation = request.correlation_id;
    drop(receiver);
    assert!(runtime.respond(correlation, HostResult::Ok(json!(true))));
    assert!(runtime.respond(correlation, HostResult::Ok(json!(true))));
    let outcome = runtime.wait(id, Duration::from_secs(2)).unwrap();
    assert_eq!(outcome.state, JobState::Completed);
    thread::sleep(Duration::from_millis(20));
    assert!(runtime.metrics().late_responses >= 1);
    runtime.shutdown();
}

#[test]
fn supervisor_reports_rss_and_health() {
    let mut config = config();
    config.rss_poll_interval = Duration::from_millis(50);
    let mut runtime = ConverterRuntime::new(config).unwrap();
    thread::sleep(Duration::from_millis(200));
    assert!(runtime.rss_bytes() > 0);
    assert_eq!(runtime.health(), WorkerHealth::Running);
    runtime.shutdown();
}

#[test]
fn host_payload_and_pending_limits_are_enforced() {
    let mut config = config();
    config.max_host_payload_bytes = 64;
    config.max_pending_host_ops = 1;
    config.max_concurrent_jobs = 2;
    let mut runtime = ConverterRuntime::new(config).unwrap();

    let large = runtime
        .submit(JobRequest {
            device: "host-large".to_owned(),
            scenario: json!({
                "id": "host-large",
                "kind": "host-op",
                "payload": "x".repeat(200),
            }),
        })
        .unwrap();
    let outcome = runtime.wait(large, Duration::from_secs(1)).unwrap();
    assert_eq!(outcome.state, JobState::Failed);

    let first = runtime
        .submit(JobRequest {
            device: "host-1".to_owned(),
            scenario: json!({"id": "host-1", "kind": "host-op", "payload": "ok"}),
        })
        .unwrap();
    let second = runtime
        .submit(JobRequest {
            device: "host-2".to_owned(),
            scenario: json!({"id": "host-2", "kind": "host-op", "payload": "ok"}),
        })
        .unwrap();
    let second_outcome = runtime.wait(second, Duration::from_secs(1)).unwrap();
    assert_eq!(second_outcome.state, JobState::Failed);
    let _ = runtime.wait(first, Duration::from_secs(2));
    let metrics = runtime.metrics();
    assert!(metrics.payload_rejected >= 1);
    assert!(metrics.host_request_backpressure >= 1);
    runtime.shutdown();
}

#[test]
fn restart_recreates_a_working_worker() {
    let mut runtime = ConverterRuntime::new(config()).unwrap();
    let first = runtime.submit(timer_job(10, false, false)).unwrap();
    assert_eq!(
        runtime.wait(first, Duration::from_secs(1)).unwrap().state,
        JobState::Completed
    );
    runtime.restart().unwrap();
    let second = runtime.submit(timer_job(10, false, false)).unwrap();
    assert_eq!(
        runtime.wait(second, Duration::from_secs(1)).unwrap().state,
        JobState::Completed
    );
    runtime.shutdown();
}

#[test]
fn cancelling_a_job_cancels_its_pending_timer_and_frees_the_device() {
    let mut runtime = ConverterRuntime::new(config()).unwrap();
    let long = runtime.submit(timer_job(10_000, false, false)).unwrap();
    thread::sleep(Duration::from_millis(20));
    assert_eq!(runtime.cancel_request(long), CancelStatus::Accepted);
    let outcome = runtime.wait(long, Duration::from_secs(1)).unwrap();
    assert_eq!(outcome.state, JobState::Cancelled);
    let replacement = runtime.submit(timer_job(10, false, false)).unwrap();
    assert_eq!(
        runtime
            .wait(replacement, Duration::from_secs(1))
            .unwrap()
            .state,
        JobState::Completed
    );
    runtime.shutdown();
}

#[test]
fn shutdown_interrupts_long_js_without_blocking() {
    let mut config = config();
    config.job_timeout = Duration::from_secs(30);
    let mut runtime = ConverterRuntime::new(config).unwrap();
    let id = runtime
        .submit(JobRequest {
            device: "shutdown-js".to_owned(),
            scenario: json!({"id": "loop", "kind": "jsLoop"}),
        })
        .unwrap();
    thread::sleep(Duration::from_millis(30));
    let started = Instant::now();
    runtime.shutdown();
    assert!(started.elapsed() < Duration::from_secs(3));
    let _ = id;
}
