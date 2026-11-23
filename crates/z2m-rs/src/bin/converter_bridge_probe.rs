// Offline bridge dispatch probe.
//
// Measures the bridge-side dispatcher overhead separately from JS execution:
// - outer: submit -> event -> oneshot wake-up;
// - workerTotal: runtime JobMetrics.total_ms;
// - js: runtime JobMetrics.js_exec_ms;
// - dispatchOverhead = outer - workerTotal (event loop / channel / caller);
// - workerOverhead = workerTotal - js - hostWait - promiseSched - eval.
// No coordinator, MQTT broker or Home Assistant is contacted.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use zigbee_apiarist::controller::database::{Database, DeviceType};
use zigbee_apiarist::controller::model::device::{Device as ZhDevice, InterviewState};
use zigbee_apiarist::controller::model::entity::inject_database;
use zigbee_converters::{ConvertData, ConvertMeta, ToMeta, ToTarget};

use z2m_rs::converter::{ConverterDispatcher, DispatcherConfig, OfflineMockTransport};

fn arg(name: &str, fallback: &str) -> String {
    let mut iter = std::env::args().skip(1);
    while let Some(flag) = iter.next() {
        if flag == name {
            return iter.next().expect("missing value");
        }
    }
    fallback.to_owned()
}

fn percentile(values: &mut [f64], percentile: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let index = ((values.len() - 1) as f64 * percentile).round() as usize;
    values[index]
}

fn create_device(ieee: &str, network: u16, model: &str) -> Arc<ZhDevice> {
    let zh = ZhDevice::create(
        DeviceType::Router,
        ieee,
        network,
        None,
        Some("IKEA of Sweden".to_owned()),
        None,
        Some(model.to_owned()),
        InterviewState::Successful,
        None,
    )
    .unwrap();
    let endpoint = zh.create_endpoint(1).unwrap();
    *endpoint.input_clusters.lock().unwrap() =
        vec![0x0000, 0x0003, 0x0004, 0x0005, 0x0006, 0x0008, 0x0300];
    *endpoint.output_clusters.lock().unwrap() = vec![0x0019];
    zh
}

fn convert_data(zh: &Arc<ZhDevice>) -> ConvertData {
    ConvertData {
        r#type: "attributeReport".to_owned(),
        device: zh.clone(),
        endpoint: zh.get_endpoint(1).unwrap(),
        cluster: "genOnOff".to_owned(),
        data: json!({"onOff": true}),
        group_id: None,
        linkquality: 120,
        meta: json!({}),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let package = PathBuf::from(arg(
        "--package",
        ".upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters",
    ));
    let herdsman = PathBuf::from(arg(
        "--herdsman",
        ".upstream/zigbee2mqtt/node_modules/zigbee-herdsman",
    ));
    let model = arg("--model", "TRADFRI bulb E27 WS opal 1000lm");
    let mode = arg("--mode", "report");
    let iterations: usize = arg("--iterations", "200").parse().unwrap();
    let concurrent: usize = arg("--concurrent", "1").parse().unwrap();

    let database =
        std::env::temp_dir().join(format!("converter-bridge-probe-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&database);
    inject_database(Arc::new(std::sync::Mutex::new(Database::open(&database))));
    ZhDevice::reset_cache();
    let mut devices = Vec::new();
    for index in 0..concurrent {
        devices.push(create_device(
            &format!("0x0017880104e455{:02x}", 0x10 + index),
            0x1000 + index as u16,
            &model,
        ));
    }

    let mut config = DispatcherConfig::new(package, herdsman);
    config.runtime.max_concurrent_jobs = concurrent.max(1);
    config.runtime.job_timeout = Duration::from_secs(30);
    config.runtime.host_timeout = Duration::from_secs(5);
    let transport = Arc::new(OfflineMockTransport::new());
    let dispatcher = ConverterDispatcher::start(config, transport).await.unwrap();
    let prepared = dispatcher.prepare(&devices[0]).await.unwrap();
    let source = prepared.source.clone();
    let definition_model = prepared.lookup_model.clone();

    let mut tasks = Vec::new();
    for (index, device) in devices.iter().cloned().enumerate() {
        let dispatcher_for_task = Arc::clone(&dispatcher);
        let source_for_task = source.clone();
        let definition_for_task = definition_model.clone();
        let mapped_for_task = prepared.definition.clone();
        let mode = mode.clone();
        tasks.push(tokio::spawn(async move {
            let mut samples = Vec::new();
            for _ in 0..iterations {
                let started = Instant::now();
                match mode.as_str() {
                    "report" => {
                        let meta = ConvertMeta {
                            device: device.clone(),
                            state: json!({}),
                            device_exposes_changed: Arc::new(|| {}),
                        };
                        let (_, metrics) = dispatcher_for_task
                            .from_zigbee(
                                &source_for_task,
                                &definition_for_task,
                                convert_data(&device),
                                Arc::new(|_| Box::pin(async {})),
                                json!({}),
                                meta,
                            )
                            .await
                            .unwrap();
                        samples.push((started.elapsed().as_secs_f64() * 1000.0, metrics));
                    }
                    "set" => {
                        let endpoint = device.get_endpoint(1).unwrap();
                        let meta = ToMeta {
                            endpoint_name: None,
                            options: json!({}),
                            message: json!({"state": "ON"}),
                            device: Some(device.clone()),
                            members_state: Value::Null,
                            state: json!({}),
                            mapped: mapped_for_task.clone(),
                            publish: Some(Arc::new(|_| Box::pin(async {}))),
                            device_exposes_changed: Some(Arc::new(|| {})),
                        };
                        let (_, metrics) = dispatcher_for_task
                            .to_zigbee(
                                &source_for_task,
                                &definition_for_task,
                                "set",
                                ToTarget::Endpoint(endpoint),
                                "state".to_owned(),
                                json!("ON"),
                                meta,
                            )
                            .await
                            .unwrap();
                        samples.push((started.elapsed().as_secs_f64() * 1000.0, metrics));
                    }
                    "configure" => {
                        let metrics = dispatcher_for_task
                            .configure(&source_for_task, &definition_for_task, device.clone())
                            .await
                            .unwrap();
                        samples.push((started.elapsed().as_secs_f64() * 1000.0, metrics));
                    }
                    other => panic!("unknown mode {other}"),
                }
            }
            let _ = index;
            samples
        }));
    }
    let mut all = Vec::new();
    for task in tasks {
        all.extend(task.await.unwrap());
    }

    let mut outer: Vec<f64> = all.iter().map(|(outer, _)| *outer).collect();
    let mut total: Vec<f64> = all.iter().map(|(_, metrics)| metrics.total_ms).collect();
    let mut js: Vec<f64> = all.iter().map(|(_, metrics)| metrics.js_exec_ms).collect();
    let mut host: Vec<f64> = all
        .iter()
        .map(|(_, metrics)| metrics.host_wait_ms)
        .collect();
    let mut dispatch: Vec<f64> = all
        .iter()
        .map(|(outer, metrics)| (outer - metrics.total_ms).max(0.0))
        .collect();
    let mut worker_overhead: Vec<f64> = all
        .iter()
        .map(|(_, metrics)| {
            (metrics.total_ms
                - metrics.js_exec_ms
                - metrics.host_wait_ms
                - metrics.promise_sched_ms
                - metrics.eval_ms
                - metrics.queue_wait_ms)
                .max(0.0)
        })
        .collect();

    let metrics = dispatcher.metrics();
    let result = json!({
        "mode": mode,
        "model": definition_model,
        "source": source,
        "iterations": all.len(),
        "concurrentDevices": concurrent,
        "outerMs": {"p50": percentile(&mut outer, 0.5), "p95": percentile(&mut outer, 0.95)},
        "workerTotalMs": {"p50": percentile(&mut total, 0.5), "p95": percentile(&mut total, 0.95)},
        "jsExecMs": {"p50": percentile(&mut js, 0.5), "p95": percentile(&mut js, 0.95)},
        "hostWaitMs": {"p50": percentile(&mut host, 0.5), "p95": percentile(&mut host, 0.95)},
        "dispatchOverheadMs": {"p50": percentile(&mut dispatch, 0.5), "p95": percentile(&mut dispatch, 0.95)},
        "workerOverheadMs": {"p50": percentile(&mut worker_overhead, 0.5), "p95": percentile(&mut worker_overhead, 0.95)},
        "counters": {
            "submitted": metrics.submitted,
            "completed": metrics.completed,
            "failed": metrics.failed,
            "cancelled": metrics.cancelled,
            "hostTimeouts": metrics.host_timeouts,
            "lateResponses": metrics.late_responses,
            "responseBackpressure": metrics.response_backpressure,
            "heapLiveBytes": metrics.heap_live_bytes,
        },
        "transportCalls": dispatcher.transport().calls().len(),
    });
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
    dispatcher.shutdown().await;
    let _ = std::fs::remove_file(database);
}
