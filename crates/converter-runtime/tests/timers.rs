// Timer/lifecycle tests that do not need the upstream device database.
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use converter_runtime::{
    CancelDeviceStatus, CancelStatus, ConverterRuntime, HostResult, JobRequest, JobState,
    RespondStatus, RuntimeConfig, SubmitError, WorkerHealth,
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
    let status = runtime.shutdown_with_timeout(Duration::from_millis(500));
    assert!(matches!(
        status,
        converter_runtime::ShutdownStatus::Stopped | converter_runtime::ShutdownStatus::TimedOut
    ));
    assert!(started.elapsed() < Duration::from_secs(2));
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
    let receiver = runtime.take_host_receiver().expect("host receiver");
    let request = receiver.recv_timeout(Duration::from_secs(2)).unwrap();
    let correlation = request.correlation_id;
    drop(receiver);
    assert_eq!(
        runtime.respond(correlation, HostResult::Ok(json!(true))),
        RespondStatus::Delivered
    );
    assert_eq!(
        runtime.respond(correlation, HostResult::Ok(json!(true))),
        RespondStatus::Delivered
    );
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
    let status = runtime.shutdown_with_timeout(Duration::from_secs(1));
    assert!(matches!(
        status,
        converter_runtime::ShutdownStatus::Stopped | converter_runtime::ShutdownStatus::TimedOut
    ));
    assert!(started.elapsed() < Duration::from_secs(2));
    let _ = id;
}

#[test]
fn response_delivery_backpressure_is_explicit() {
    let mut config = config();
    config.max_pending_responses = 1;
    config.job_timeout = Duration::from_secs(5);
    let mut runtime = ConverterRuntime::new(config).unwrap();
    let _loop = runtime
        .submit(JobRequest {
            device: "response-bp".to_owned(),
            scenario: json!({"id": "loop", "kind": "jsLoop"}),
        })
        .unwrap();
    thread::sleep(Duration::from_millis(30));
    assert_eq!(
        runtime.respond(999, HostResult::Ok(json!(true))),
        RespondStatus::Delivered
    );
    assert_eq!(
        runtime.respond(998, HostResult::Ok(json!(true))),
        RespondStatus::Backpressure
    );
    runtime.shutdown();
}

#[test]
fn file_access_is_sandboxed_against_escapes_and_symlinks() {
    let mut runtime = ConverterRuntime::new(config()).unwrap();
    let allowed = runtime
        .submit(JobRequest {
            device: "file".to_owned(),
            scenario: json!({
                "id": "allowed",
                "kind": "read-file",
                "path": PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixture-package/dist/index.js")
                    .to_string_lossy(),
            }),
        })
        .unwrap();
    let outcome = runtime.wait(allowed, Duration::from_secs(1)).unwrap();
    assert_eq!(outcome.state, JobState::Completed);
    assert!(
        outcome.result.unwrap()["content"]
            .as_str()
            .unwrap()
            .contains("prepareDefinition")
    );

    for path in ["/etc/passwd", "../../../../etc/passwd"] {
        let denied = runtime
            .submit(JobRequest {
                device: "file".to_owned(),
                scenario: json!({"id": "denied", "kind": "read-file", "path": path}),
            })
            .unwrap();
        let outcome = runtime.wait(denied, Duration::from_secs(1)).unwrap();
        assert_eq!(outcome.state, JobState::Failed, "{path}");
    }

    #[cfg(unix)]
    {
        let link = std::env::temp_dir().join("converter-runtime-escape-link");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink("/etc/passwd", &link).unwrap();
        let denied = runtime
            .submit(JobRequest {
                device: "file".to_owned(),
                scenario: json!({
                    "id": "symlink",
                    "kind": "read-file",
                    "path": link.to_string_lossy(),
                }),
            })
            .unwrap();
        let outcome = runtime.wait(denied, Duration::from_secs(1)).unwrap();
        assert_eq!(outcome.state, JobState::Failed);
        let _ = std::fs::remove_file(link);
    }
    runtime.shutdown();
}

#[test]
fn device_leave_cancels_jobs_and_timers() {
    let mut runtime = ConverterRuntime::new(config()).unwrap();
    let long = runtime
        .submit(JobRequest {
            device: "leave-me".to_owned(),
            scenario: json!({"id": "long", "kind": "timer", "delay": 10_000}),
        })
        .unwrap();
    thread::sleep(Duration::from_millis(20));
    assert_eq!(
        runtime.cancel_device("leave-me"),
        CancelDeviceStatus::Accepted
    );
    let outcome = runtime.wait(long, Duration::from_secs(1)).unwrap();
    assert_eq!(outcome.state, JobState::Cancelled);
    let replacement = runtime
        .submit(JobRequest {
            device: "leave-me".to_owned(),
            scenario: json!({"id": "short", "kind": "timer", "delay": 10}),
        })
        .unwrap();
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
fn timer_outliving_job_is_removed_on_device_leave() {
    let mut runtime = ConverterRuntime::new(config()).unwrap();
    let late = runtime
        .submit(JobRequest {
            device: "late-device".to_owned(),
            scenario: json!({"id": "late", "kind": "timer-late", "delay": 500}),
        })
        .unwrap();
    assert_eq!(
        runtime.wait(late, Duration::from_secs(1)).unwrap().state,
        JobState::Completed
    );
    assert_eq!(
        runtime.cancel_device("late-device"),
        CancelDeviceStatus::Accepted
    );
    thread::sleep(Duration::from_millis(700));
    let check = runtime
        .submit(JobRequest {
            device: "late-device".to_owned(),
            scenario: json!({"id": "check", "kind": "timer-check"}),
        })
        .unwrap();
    let outcome = runtime.wait(check, Duration::from_secs(1)).unwrap();
    assert_eq!(outcome.state, JobState::Completed);
    assert_eq!(outcome.result.unwrap()["lateTimerFired"], false);
    runtime.shutdown();
}

#[test]
fn timer_callback_error_does_not_kill_the_worker() {
    let mut runtime = ConverterRuntime::new(config()).unwrap();
    let error = runtime
        .submit(JobRequest {
            device: "timer-error".to_owned(),
            scenario: json!({"id": "error", "kind": "timer-error", "delay": 10}),
        })
        .unwrap();
    assert_eq!(
        runtime.wait(error, Duration::from_secs(1)).unwrap().state,
        JobState::Completed
    );
    thread::sleep(Duration::from_millis(50));
    let next = runtime.submit(timer_job(10, false, false)).unwrap();
    assert_eq!(
        runtime.wait(next, Duration::from_secs(1)).unwrap().state,
        JobState::Completed
    );
    runtime.shutdown();
}

fn on_event_config() -> RuntimeConfig {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixture-package");
    let mut config = RuntimeConfig::new(fixture.clone(), fixture.clone());
    config.external_path = fixture.join("external/on-event.js");
    config.max_concurrent_jobs = 1;
    config.job_timeout = Duration::from_secs(5);
    config.host_timeout = Duration::from_millis(100);
    config.max_rss_bytes = 512 << 20;
    config
}

fn on_event_job(device: &str, event_type: &str) -> JobRequest {
    JobRequest {
        device: device.to_owned(),
        scenario: json!({
            "id": "on-event",
            "kind": "onEvent",
            "eventType": event_type,
            "definition": {
                "source": "external",
                "model": "synthetic-on-event",
                "file": "",
            },
        }),
    }
}

#[test]
fn on_event_timer_is_removed_on_device_leave() {
    let mut runtime = ConverterRuntime::new(on_event_config()).unwrap();
    let id = runtime
        .submit(on_event_job("on-event-device", "start"))
        .unwrap();
    let outcome = runtime.wait(id, Duration::from_secs(2)).unwrap();
    assert_eq!(outcome.state, JobState::Completed, "{outcome:?}");
    assert_eq!(
        runtime.cancel_device("on-event-device"),
        CancelDeviceStatus::Accepted
    );
    thread::sleep(Duration::from_millis(250));
    let check = runtime
        .submit(JobRequest {
            device: "observer".to_owned(),
            scenario: json!({"id": "check", "kind": "on-event-timer-check"}),
        })
        .unwrap();
    let outcome = runtime.wait(check, Duration::from_secs(2)).unwrap();
    assert_eq!(outcome.state, JobState::Completed);
    assert_eq!(outcome.result.unwrap()["onEventTimerFired"], false);
    runtime.shutdown();
}

#[test]
fn cancelling_a_running_on_event_removes_its_timer_and_frees_the_device() {
    let mut runtime = ConverterRuntime::new(on_event_config()).unwrap();
    let running = runtime
        .submit(on_event_job("on-event-running", "deviceJoined"))
        .unwrap();
    thread::sleep(Duration::from_millis(30));
    assert_eq!(runtime.cancel_request(running), CancelStatus::Accepted);
    assert_eq!(
        runtime.wait(running, Duration::from_secs(2)).unwrap().state,
        JobState::Cancelled
    );
    thread::sleep(Duration::from_millis(250));
    let check = runtime
        .submit(JobRequest {
            device: "observer".to_owned(),
            scenario: json!({"id": "check", "kind": "on-event-timer-check"}),
        })
        .unwrap();
    let outcome = runtime.wait(check, Duration::from_secs(2)).unwrap();
    assert_eq!(outcome.state, JobState::Completed);
    assert_eq!(outcome.result.unwrap()["onEventWaitTimerFired"], false);
    let replacement = runtime
        .submit(JobRequest {
            device: "on-event-running".to_owned(),
            scenario: json!({"id": "short", "kind": "timer", "delay": 10}),
        })
        .unwrap();
    assert_eq!(
        runtime
            .wait(replacement, Duration::from_secs(2))
            .unwrap()
            .state,
        JobState::Completed
    );
    runtime.shutdown();
}

#[test]
fn fast_completions_do_not_leak_admission() {
    let mut config = config();
    config.max_total_queue = 1;
    config.max_per_device_queue = 1;
    let mut runtime = ConverterRuntime::new(config).unwrap();
    for _ in 0..100 {
        let mut submitted = None;
        for _ in 0..100 {
            match runtime.submit(timer_job(0, false, false)) {
                Ok(id) => {
                    submitted = Some(id);
                    break;
                }
                Err(SubmitError::Backpressure) => thread::sleep(Duration::from_millis(1)),
                Err(error) => panic!("unexpected submit error: {error:?}"),
            }
        }
        let id = submitted.expect("admission credit never returned");
        assert_eq!(
            runtime.wait(id, Duration::from_secs(2)).unwrap().state,
            JobState::Completed
        );
    }
    runtime.shutdown();
}

#[test]
fn job_events_are_delivered_without_waiting() {
    let mut runtime = ConverterRuntime::new(config()).unwrap();
    let events = runtime.take_events().expect("events");
    assert!(runtime.take_events().is_none(), "single event consumer");
    let id = runtime.submit(timer_job(10, false, false)).unwrap();
    let started = events.recv_timeout(Duration::from_secs(2)).unwrap();
    match started {
        converter_runtime::JobEvent::Started { id: started, .. } => assert_eq!(started, id),
        other => panic!("unexpected event: {other:?}"),
    }
    let finished = events.recv_timeout(Duration::from_secs(2)).unwrap();
    match finished {
        converter_runtime::JobEvent::Finished(outcome) => {
            assert_eq!(outcome.id, id);
            assert_eq!(outcome.state, JobState::Completed);
        }
        other => panic!("unexpected event: {other:?}"),
    }
    runtime.shutdown();
}

#[test]
fn events_survive_a_temporarily_full_event_queue() {
    let mut config = config();
    config.max_pending_events = 1;
    let mut runtime = ConverterRuntime::new(config).unwrap();
    // Submit while the receiver is not being drained; the worker queues the
    // overflow and flushes it once the bridge reads.
    let mut ids = Vec::new();
    for _ in 0..5 {
        ids.push(runtime.submit(timer_job(5, false, false)).unwrap());
    }
    for id in &ids {
        let _ = runtime.wait(*id, Duration::from_secs(2)).unwrap();
    }
    let events = runtime.take_events().expect("events");
    let mut finished = 0;
    for _ in 0..10 {
        match events.recv_timeout(Duration::from_secs(2)) {
            Ok(converter_runtime::JobEvent::Finished(outcome)) => {
                assert_eq!(outcome.state, JobState::Completed);
                finished += 1;
            }
            Ok(_) => {}
            Err(error) => panic!("event missing after {finished}: {error}"),
        }
        if finished == ids.len() {
            break;
        }
    }
    assert_eq!(finished, ids.len());
    runtime.shutdown();
}

#[test]
fn device_cancel_reports_closed_after_shutdown() {
    let mut runtime = ConverterRuntime::new(config()).unwrap();
    assert_eq!(
        runtime.cancel_device("closed-device"),
        CancelDeviceStatus::Accepted
    );
    runtime.shutdown();
    assert_eq!(
        runtime.cancel_device("closed-device"),
        CancelDeviceStatus::Closed
    );
}
