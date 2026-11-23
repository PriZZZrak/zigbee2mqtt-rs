// Offline bridge integration harness. This simulates the Rust bridge side of
// the contract in docs/converter-runtime-bridge-contract.md without ZNP, MQTT
// or Home Assistant: an event-loop thread submits jobs, a transport thread
// owns the host receiver and answers each request through `respond`, retaining
// the result across `Backpressure`.
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use converter_runtime::{
    CancelDeviceStatus, ConverterRuntime, HostResult, JobRequest, JobState, RespondStatus,
    RuntimeConfig, SubmitError,
};
use serde_json::json;

fn config() -> RuntimeConfig {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixture-package");
    let mut config = RuntimeConfig::new(fixture.clone(), fixture.clone());
    config.external_path = fixture.join("no-external.js");
    config.max_concurrent_jobs = 4;
    config.max_pending_responses = 2;
    config.job_timeout = Duration::from_secs(2);
    config.host_timeout = Duration::from_secs(1);
    config.shutdown_timeout = Duration::from_secs(1);
    config
}

fn host_job(device: &str, payload: &str) -> JobRequest {
    JobRequest {
        device: device.to_owned(),
        scenario: json!({"id": "bridge-host-op", "kind": "host-op", "payload": payload}),
    }
}

#[test]
fn offline_bridge_transport_contract_holds_without_zigbee_or_mqtt() {
    let runtime = Arc::new(ConverterRuntime::new(config()).unwrap());
    let stop = Arc::new(AtomicBool::new(false));
    let served = Arc::new(AtomicU64::new(0));

    let responder = {
        let runtime = Arc::clone(&runtime);
        let stop = Arc::clone(&stop);
        let served = Arc::clone(&served);
        thread::spawn(move || {
            // The bridge transport is the single consumer of host requests.
            let receiver = runtime.take_host_receiver().expect("host receiver");
            while !stop.load(Ordering::Relaxed) {
                let request = match receiver.recv_timeout(Duration::from_millis(20)) {
                    Ok(request) => request,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                };
                let operation = &request.operation;
                assert_eq!(operation["op"], "probe");
                let result = HostResult::Ok(json!({"echo": operation["payload"]}));
                // The completed physical result is retained and retried with
                // the same correlation ID; it is never dropped or re-executed.
                let mut delivered = false;
                for _ in 0..1000 {
                    match runtime.respond(request.correlation_id, result.clone()) {
                        RespondStatus::Delivered => {
                            delivered = true;
                            break;
                        }
                        RespondStatus::Backpressure => {
                            thread::sleep(Duration::from_millis(1));
                        }
                        RespondStatus::Closed => break,
                    }
                }
                assert!(delivered, "response could not be delivered");
                served.fetch_add(1, Ordering::Relaxed);
            }
        })
    };

    // Burst submit from the simulated bridge event loop. Every call must return
    // immediately even while host responses are in flight.
    let mut ids = Vec::new();
    let mut slowest = Duration::ZERO;
    for index in 0..8 {
        let started = Instant::now();
        let id = loop {
            match runtime.submit(host_job(
                &format!("bridge-{}", index % 3),
                &format!("p{index}"),
            )) {
                Ok(id) => break id,
                Err(SubmitError::Backpressure) => thread::sleep(Duration::from_millis(1)),
                Err(error) => panic!("unexpected submit error: {error:?}"),
            }
        };
        slowest = slowest.max(started.elapsed());
        ids.push(id);
    }
    assert!(
        slowest < Duration::from_millis(100),
        "submit blocked for {slowest:?}"
    );

    for (index, id) in ids.iter().enumerate() {
        let outcome = runtime.wait(*id, Duration::from_secs(3)).unwrap();
        assert_eq!(outcome.state, JobState::Completed, "{outcome:?}");
        assert_eq!(
            outcome.result.unwrap()["value"]["echo"],
            format!("p{index}")
        );
    }
    assert_eq!(served.load(Ordering::Relaxed), 8);

    // Device removal cancels queued/running work and frees the admission slot.
    let leaving = runtime
        .submit(JobRequest {
            device: "bridge-leave".to_owned(),
            scenario: json!({"id": "leave", "kind": "timer", "delay": 10_000}),
        })
        .unwrap();
    thread::sleep(Duration::from_millis(20));
    assert_eq!(
        runtime.cancel_device("bridge-leave"),
        CancelDeviceStatus::Accepted
    );
    assert_eq!(
        runtime.wait(leaving, Duration::from_secs(2)).unwrap().state,
        JobState::Cancelled
    );

    stop.store(true, Ordering::Relaxed);
    responder.join().unwrap();

    let mut runtime = Arc::try_unwrap(runtime).ok().unwrap();
    assert_eq!(
        runtime.shutdown_with_timeout(Duration::from_secs(1)),
        converter_runtime::ShutdownStatus::Stopped
    );
    assert_eq!(runtime.health(), converter_runtime::WorkerHealth::Stopped);
}
