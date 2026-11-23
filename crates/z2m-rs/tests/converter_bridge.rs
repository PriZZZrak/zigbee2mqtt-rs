#![allow(clippy::await_holding_lock)]
// Offline bridge integration tests for the converter runtime.
//
// These tests use the real controller model, the real converter definitions
// registry and the real Receive/Publish/Configure/OnEvent extensions. The only
// mocked piece is the converter host transport: it validates ZCL names and
// returns deterministic responses without touching a coordinator.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use zigbee_apiarist::controller::controller::{Controller, ControllerOptions};
use zigbee_apiarist::controller::database::{Database, DeviceType};
use zigbee_apiarist::controller::model::device::{Device as ZhDevice, InterviewState};
use zigbee_apiarist::controller::model::entity::inject_database;
use zigbee_apiarist::controller::model::group::Group as ZhGroup;
use zigbee_apiarist::controller::model::test_support::serial;
use zigbee_apiarist::models::network_options::NetworkOptions;

use z2m_rs::converter::{
    ConverterDispatcher, ConverterTransport, DispatcherConfig, OfflineMockTransport,
    TransportFuture,
};
use z2m_rs::event_bus::EventBus;
use z2m_rs::extension::configure::Configure;
use z2m_rs::extension::extension::{Extension, ExtensionContext};
use z2m_rs::extension::publish::Publish;
use z2m_rs::extension::receive::Receive;
use z2m_rs::model::entity::{Entity, PublishEntityState};
use z2m_rs::mqtt::Mqtt;
use z2m_rs::state::State;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::zigbee::Zigbee;
use zigbee_converters::{ConvertData, ConvertMeta, DefinitionRef, ToMeta, ToTarget};

const LAMP: &str = "0x0017880104e45519";
const SECOND: &str = "0x0017880104e4551a";
const THIRD: &str = "0x0017880104e4551b";
const FOURTH: &str = "0x0017880104e4551c";

fn fixture_package() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/converter-package")
}

fn temp(name: &str) -> PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    std::env::temp_dir().join(format!("z2m-converter-{name}-{}-{id}", std::process::id()))
}

struct Harness {
    context: ExtensionContext,
    zigbee: Arc<Zigbee>,
    event_bus: Arc<Mutex<EventBus>>,
    transport: Arc<dyn z2m_rs::converter::ConverterTransport>,
    dispatcher: Arc<ConverterDispatcher>,
    publishes: Arc<Mutex<Vec<(String, Value)>>>,
    database_path: PathBuf,
    state_path: PathBuf,
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.database_path);
        let _ = std::fs::remove_file(&self.state_path);
    }
}

async fn harness(devices: &[(&str, &str)]) -> Harness {
    let transport = Arc::new(OfflineMockTransport::new());
    harness_with_transport(devices, |_| {}, transport).await
}

async fn harness_with(
    devices: &[(&str, &str)],
    configure: impl FnOnce(&mut converter_runtime::RuntimeConfig),
) -> Harness {
    let transport = Arc::new(OfflineMockTransport::new());
    harness_with_transport(devices, configure, transport).await
}

async fn harness_with_transport(
    devices: &[(&str, &str)],
    configure: impl FnOnce(&mut converter_runtime::RuntimeConfig),
    transport: Arc<dyn z2m_rs::converter::ConverterTransport>,
) -> Harness {
    let database_path = temp("db");
    inject_database(Arc::new(Mutex::new(Database::open(&database_path))));
    ZhDevice::reset_cache();
    ZhGroup::reset_cache();
    ZhDevice::create(
        DeviceType::Coordinator,
        "0x00124b0024c16b95",
        0,
        None,
        None,
        None,
        None,
        InterviewState::Successful,
        None,
    )
    .unwrap();
    let mut settings_devices = serde_json::Map::new();
    for (ieee, name) in devices {
        ZhDevice::create(
            DeviceType::Router,
            ieee,
            1234 + devices.len() as u16,
            None,
            Some("Fixture".to_string()),
            None,
            Some("FixtureLamp".to_string()),
            InterviewState::Successful,
            None,
        )
        .unwrap();
        settings_devices.insert(ieee.to_string(), json!({"friendly_name": name}));
    }

    let controller = Arc::new(
        Controller::new(ControllerOptions {
            network: NetworkOptions {
                pan_id: 0x1a62,
                extended_pan_id: vec![0xdd; 8],
                channel_list: vec![11],
                network_key: vec![1, 3, 5, 7, 9, 11, 13, 15, 0, 2, 4, 6, 8, 10, 12, 13],
                network_key_distribute: true,
                has_default_extended_pan_id: false,
            },
            transport: json!({}),
            database_path: database_path.display().to_string(),
            database_backup_path: None,
            backup_path: None,
            adapter: json!({}),
            accept_joining_device_handler: None,
        })
        .unwrap(),
    );

    let settings = Arc::new(Mutex::new(Settings::from_config(json!({
        "mqtt": {"server": "localhost"},
        "devices": settings_devices,
        "advanced": {"last_seen": "disable"},
    }))));
    let event_bus = Arc::new(Mutex::new(EventBus::new()));
    let logger = Arc::new(Mutex::new(Logger::new()));
    let zigbee = Zigbee::with_controller(
        controller,
        event_bus.clone(),
        settings.clone(),
        logger.clone(),
    );
    for (ieee, _) in devices {
        let device = zigbee.resolve_device(ieee).unwrap();
        let endpoint = device.zh.create_endpoint(1).unwrap();
        *endpoint.input_clusters.lock().unwrap() = vec![0x0000, 0x0006, 0x0008];
        *endpoint.output_clusters.lock().unwrap() = vec![0x0019];
    }

    let state_path = temp("state.json");
    let state = Arc::new(Mutex::new(State::with_file(
        settings.clone(),
        event_bus.clone(),
        logger.clone(),
        state_path.clone(),
    )));
    let mqtt = Arc::new(tokio::sync::Mutex::new(Mqtt::new(event_bus.clone())));
    let publishes = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&publishes);
    let publish: PublishEntityState = Arc::new(move |entity, payload, _reason| {
        let captured = Arc::clone(&captured);
        Box::pin(async move {
            captured
                .lock()
                .unwrap()
                .push((entity.entity_id().key(), payload));
        })
    });

    let mut runtime_config =
        converter_runtime::RuntimeConfig::new(fixture_package(), fixture_package());
    runtime_config.max_concurrent_jobs = 4;
    runtime_config.job_timeout = Duration::from_secs(3);
    runtime_config.host_timeout = Duration::from_secs(1);
    runtime_config.shutdown_timeout = Duration::from_secs(1);
    configure(&mut runtime_config);
    let dispatcher = ConverterDispatcher::start(
        DispatcherConfig {
            runtime: runtime_config,
            prefer_external: false,
        },
        transport.clone(),
    )
    .await
    .unwrap();

    let context = ExtensionContext {
        zigbee: zigbee.clone(),
        mqtt: mqtt.clone(),
        state: state.clone(),
        publish_entity_state: publish,
        event_bus: event_bus.clone(),
        logger: logger.clone(),
        settings,
        enable_disable_extension: Arc::new(|_, _| Box::pin(async { Ok(()) })),
        restart: Arc::new(|| Box::pin(async {})),
        add_extension: Arc::new(|_| Box::pin(async {})),
        key: 77,
    };

    Harness {
        context,
        zigbee,
        event_bus,
        transport,
        dispatcher,
        publishes,
        database_path,
        state_path,
    }
}

async fn prepare_all(harness: &Harness) -> Vec<DefinitionRef> {
    let mut definitions = Vec::new();
    for device in harness
        .zigbee
        .devices_iterator(|device| device.r#type() != DeviceType::Coordinator)
    {
        let prepared = harness.dispatcher.prepare(&device.zh).await.unwrap();
        assert!(prepared
            .metadata
            .zigbee_model
            .contains(&prepared.metadata.model));
        harness
            .zigbee
            .converters()
            .lock()
            .unwrap()
            .register_shared(prepared.definition.clone());
        device.resolve_definition(true);
        definitions.push(device.definition().unwrap());
    }
    definitions
}

fn convert_data(zh: &Arc<ZhDevice>, data: Value) -> ConvertData {
    let endpoint = zh.get_endpoint(1).unwrap();
    ConvertData {
        r#type: "attributeReport".to_string(),
        device: zh.clone(),
        endpoint,
        cluster: "genOnOff".to_string(),
        data,
        group_id: None,
        linkquality: 90,
        meta: json!({}),
    }
}

fn convert_meta(zh: &Arc<ZhDevice>) -> ConvertMeta {
    ConvertMeta {
        device: zh.clone(),
        state: json!({}),
        device_exposes_changed: Arc::new(|| {}),
    }
}

async fn wait_publish(publishes: &Arc<Mutex<Vec<(String, Value)>>>, count: usize) -> Vec<Value> {
    for _ in 0..200 {
        let current = publishes.lock().unwrap().clone();
        if current.len() >= count {
            return current.into_iter().map(|(_, payload)| payload).collect();
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("timed out waiting for {count} publishes");
}

#[tokio::test]
async fn mixed_reports_and_sets_use_receive_and_publish_extensions() {
    let _guard = serial();
    let harness = harness(&[(LAMP, "lamp")]).await;
    prepare_all(&harness).await;
    let receive = Receive::new(harness.context.clone());
    receive.clone().start().await.unwrap();
    let publish = Publish::new(harness.context.clone());
    publish.clone().start().await.unwrap();

    let device = harness.zigbee.resolve_device(LAMP).unwrap();
    harness
        .event_bus
        .lock()
        .unwrap()
        .emit_device_message(json!({
            "type": "attributeReport",
            "device": device.event_value(),
            "endpoint": {"ID": 1},
            "linkquality": 90,
            "cluster": "genOnOff",
            "data": {"onOff": true},
            "meta": {},
        }));
    let payloads = wait_publish(&harness.publishes, 1).await;
    assert_eq!(payloads[0]["state"], "ON");

    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": "zigbee2mqtt/lamp/set",
        "message": r#"{"state":"OFF"}"#,
    }));
    let payloads = wait_publish(&harness.publishes, 2).await;
    assert_eq!(payloads[1]["state"], "OFF");
    let calls = harness.transport.calls();
    assert!(
        calls.iter().any(|call| call["op"] == "command"
            && call["cluster"] == "genOnOff"
            && call["command"] == "off"),
        "{calls:?}"
    );
}

#[tokio::test]
async fn configure_runs_sequentially_per_device() {
    let _guard = serial();
    let harness = harness(&[(LAMP, "lamp")]).await;
    let definitions = prepare_all(&harness).await;
    let definition = definitions[0].clone();
    let device = harness.zigbee.resolve_device(LAMP).unwrap().zh.clone();
    let coordinator = harness.zigbee.first_coordinator_endpoint();

    let first = {
        let configure = definition.configure.clone().unwrap();
        let device = device.clone();
        let coordinator = coordinator.clone();
        let mapped = definition.clone();
        tokio::spawn(async move { configure(device, coordinator, mapped).await })
    };
    let second = {
        let configure = definition.configure.clone().unwrap();
        let device = device.clone();
        let coordinator = coordinator.clone();
        let mapped = definition.clone();
        tokio::spawn(async move { configure(device, coordinator, mapped).await })
    };
    let started = Instant::now();
    first.await.unwrap().unwrap();
    second.await.unwrap().unwrap();
    assert!(
        started.elapsed() >= Duration::from_millis(90),
        "configure calls overlapped: {:?}",
        started.elapsed()
    );
    let calls = harness.transport.calls();
    assert_eq!(
        calls.iter().filter(|call| call["op"] == "read").count(),
        2,
        "{calls:?}"
    );
    assert_eq!(
        calls.iter().filter(|call| call["op"] == "write").count(),
        2,
        "{calls:?}"
    );
    harness.dispatcher.shutdown().await;
}

#[tokio::test]
async fn reports_are_ordered_per_device_and_run_concurrently_across_devices() {
    let _guard = serial();
    let harness = harness(&[
        (LAMP, "lamp"),
        (SECOND, "second"),
        (THIRD, "third"),
        (FOURTH, "fourth"),
    ])
    .await;
    let definitions = prepare_all(&harness).await;
    let mut tasks = Vec::new();
    for (index, ieee) in [LAMP, SECOND, THIRD, FOURTH].iter().enumerate() {
        let definition = definitions[index].clone();
        let device = harness.zigbee.resolve_device(ieee).unwrap().zh.clone();
        tasks.push(tokio::spawn(async move {
            let mut order = Vec::new();
            for sequence in 0..5 {
                let meta = convert_meta(&device);
                let convert = definition.from_zigbee[0].convert.clone();
                let converted = convert(
                    definition.clone(),
                    convert_data(&device, json!({"onOff": sequence % 2 == 0})),
                    Arc::new(|_| Box::pin(async {})),
                    json!({}),
                    meta,
                )
                .await
                .unwrap()
                .unwrap();
                order.push(converted["state"].as_str().unwrap().to_string());
            }
            order
        }));
    }
    for task in tasks {
        let order = task.await.unwrap();
        assert_eq!(order, ["ON", "OFF", "ON", "OFF", "ON"]);
    }
    harness.dispatcher.shutdown().await;
}

#[tokio::test]
async fn restart_fails_pending_jobs_and_recreates_the_realm() {
    let _guard = serial();
    let harness = harness(&[(LAMP, "lamp")]).await;
    let definitions = prepare_all(&harness).await;
    let definition = definitions[0].clone();
    let device = harness.zigbee.resolve_device(LAMP).unwrap().zh.clone();
    let generation = harness.dispatcher.generation();

    let pending = {
        let definition = definition.clone();
        let device = device.clone();
        tokio::spawn(async move {
            definition.on_event[0](zigbee_converters::OnEventEvent::DeviceJoined(
                zigbee_converters::OnEventBaseData {
                    device,
                    state: json!({}),
                    options: json!({}),
                    device_exposes_changed: Arc::new(|| {}),
                },
            ))
            .await
        })
    };
    tokio::time::sleep(Duration::from_millis(50)).await;
    let status = harness.dispatcher.restart().await.unwrap();
    assert_eq!(status, converter_runtime::ShutdownStatus::Stopped);
    assert_eq!(harness.dispatcher.generation(), generation + 1);
    let result = pending.await.unwrap();
    assert!(result.is_err(), "pending job should fail explicitly");

    let convert = definition.from_zigbee[0].convert.clone();
    let converted = convert(
        definition.clone(),
        convert_data(&device, json!({"onOff": true})),
        Arc::new(|_| Box::pin(async {})),
        json!({}),
        convert_meta(&device),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(converted["state"], "ON");
    harness.dispatcher.shutdown().await;
}

#[tokio::test]
async fn bridge_stop_and_device_leave_are_bounded_and_non_blocking() {
    let _guard = serial();
    let harness = harness(&[(LAMP, "lamp")]).await;
    let definitions = prepare_all(&harness).await;
    let definition = definitions[0].clone();
    let device = harness.zigbee.resolve_device(LAMP).unwrap().zh.clone();

    let leave = {
        let definition = definition.clone();
        let device = device.clone();
        tokio::spawn(async move {
            definition.on_event[0](zigbee_converters::OnEventEvent::DeviceJoined(
                zigbee_converters::OnEventBaseData {
                    device,
                    state: json!({}),
                    options: json!({}),
                    device_exposes_changed: Arc::new(|| {}),
                },
            ))
            .await
        })
    };
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        harness.dispatcher.cancel_device(LAMP),
        converter_runtime::CancelDeviceStatus::Accepted
    );
    let result = leave.await.unwrap();
    assert!(result.is_err(), "device leave should cancel onEvent");

    let pending = {
        let definition = definition.clone();
        let device = device.clone();
        tokio::spawn(async move {
            definition.on_event[0](zigbee_converters::OnEventEvent::DeviceJoined(
                zigbee_converters::OnEventBaseData {
                    device,
                    state: json!({}),
                    options: json!({}),
                    device_exposes_changed: Arc::new(|| {}),
                },
            ))
            .await
        })
    };
    tokio::time::sleep(Duration::from_millis(50)).await;
    let started = Instant::now();
    harness.dispatcher.shutdown().await;
    assert!(started.elapsed() < Duration::from_secs(3));
    assert!(pending.await.unwrap().is_err());
}

#[tokio::test]
async fn backpressure_is_reported_by_the_dispatcher() {
    let _guard = serial();
    let harness = harness_with(&[(LAMP, "lamp"), (SECOND, "second")], |config| {
        config.max_total_queue = 1;
        config.max_per_device_queue = 1;
        config.max_concurrent_jobs = 1;
    })
    .await;
    let definitions = prepare_all(&harness).await;
    let first = definitions[0].clone();
    let second = definitions[1].clone();
    let lamp = harness.zigbee.resolve_device(LAMP).unwrap().zh.clone();
    let second_device = harness.zigbee.resolve_device(SECOND).unwrap().zh.clone();

    let long = {
        let definition = first.clone();
        let device = lamp.clone();
        tokio::spawn(async move {
            definition.on_event[0](zigbee_converters::OnEventEvent::DeviceJoined(
                zigbee_converters::OnEventBaseData {
                    device,
                    state: json!({}),
                    options: json!({}),
                    device_exposes_changed: Arc::new(|| {}),
                },
            ))
            .await
        })
    };
    tokio::time::sleep(Duration::from_millis(30)).await;
    // Fill the bounded runtime queue; the dispatcher must surface backpressure,
    // never block the Tokio worker.
    let mut saw_backpressure = false;
    for _ in 0..40 {
        let convert = second.from_zigbee[0].convert.clone();
        match convert(
            second.clone(),
            convert_data(&second_device, json!({"onOff": true})),
            Arc::new(|_| Box::pin(async {})),
            json!({}),
            convert_meta(&second_device),
        )
        .await
        {
            Err(error) if error.contains("backpressure") => {
                saw_backpressure = true;
                break;
            }
            other => assert!(other.is_ok(), "{other:?}"),
        }
    }
    assert!(saw_backpressure, "admission never applied backpressure");
    harness.dispatcher.cancel_device(LAMP);
    let _ = long.await.unwrap();
    harness.dispatcher.shutdown().await;
}

struct DelayedReadTransport {
    inner: Arc<OfflineMockTransport>,
    delay: Duration,
    executions: Arc<std::sync::atomic::AtomicU64>,
}

impl ConverterTransport for DelayedReadTransport {
    fn execute(&self, request: &converter_runtime::HostRequest) -> TransportFuture {
        self.executions
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let inner = Arc::clone(&self.inner);
        let request = request.clone();
        let delay = self.delay;
        Box::pin(async move {
            if request.operation["op"] == "read" {
                tokio::time::sleep(delay).await;
            }
            inner.execute(&request).await
        })
    }
}

#[tokio::test]
async fn host_timeout_and_late_response_do_not_reexecute_the_operation() {
    let _guard = serial();
    let inner = Arc::new(OfflineMockTransport::new());
    let executions = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let transport = Arc::new(DelayedReadTransport {
        inner,
        delay: Duration::from_millis(250),
        executions: Arc::clone(&executions),
    });
    let harness = harness_with_transport(
        &[(LAMP, "lamp")],
        |config| {
            config.host_timeout = Duration::from_millis(50);
            config.job_timeout = Duration::from_secs(2);
        },
        transport,
    )
    .await;
    let definitions = prepare_all(&harness).await;
    let definition = definitions[0].clone();
    let device = harness.zigbee.resolve_device(LAMP).unwrap().zh.clone();
    let endpoint = device.get_endpoint(1).unwrap();
    let meta = ToMeta {
        endpoint_name: None,
        options: json!({}),
        message: json!({}),
        device: Some(device.clone()),
        members_state: Value::Null,
        state: json!({}),
        mapped: definition.clone(),
        publish: Some(Arc::new(|_| Box::pin(async {}))),
        device_exposes_changed: Some(Arc::new(|| {})),
    };
    let convert_get = definition.to_zigbee[0].convert_get.clone().unwrap();
    let result = convert_get(
        ToTarget::Endpoint(endpoint),
        "state".to_owned(),
        Value::Null,
        meta,
    )
    .await;
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("expected a host timeout error"),
    };
    assert!(
        error.contains("timeout"),
        "expected a host timeout error: {error}"
    );
    tokio::time::sleep(Duration::from_millis(400)).await;
    let metrics = harness.dispatcher.metrics();
    assert!(metrics.host_timeouts >= 1, "{metrics:?}");
    assert!(metrics.late_responses >= 1, "{metrics:?}");
    assert_eq!(
        executions.load(std::sync::atomic::Ordering::Relaxed),
        1,
        "the physical operation must not be re-executed"
    );
    harness.dispatcher.shutdown().await;
}

#[tokio::test]
async fn zigbee_production_hook_prepares_definitions() {
    let _guard = serial();
    let harness = harness(&[(LAMP, "lamp")]).await;
    harness
        .zigbee
        .set_converter_dispatcher(harness.dispatcher.clone());
    let prepared = harness.zigbee.prepare_converter_definitions().await;
    assert_eq!(prepared, 1);
    let device = harness.zigbee.resolve_device(LAMP).unwrap();
    assert_eq!(device.definition().unwrap().model, "FixtureLamp");
    harness.dispatcher.shutdown().await;
}

#[tokio::test]
async fn configure_extension_dispatches_the_js_definition() {
    let _guard = serial();
    let harness = harness(&[(LAMP, "lamp")]).await;
    prepare_all(&harness).await;
    let configure = Configure::new(harness.context.clone());
    configure.clone().start().await.unwrap();
    harness.event_bus.lock().unwrap().emit_mqtt_message(json!({
        "topic": "zigbee2mqtt/bridge/request/device/configure",
        "message": format!(r#"{{"id":"{LAMP}"}}"#),
    }));
    let mut response = None;
    for _ in 0..200 {
        let attempted = harness.context.mqtt.lock().await.attempted.clone();
        if let Some((topic, payload, _)) = attempted
            .iter()
            .find(|(topic, _, _)| topic.ends_with("bridge/response/device/configure"))
        {
            let _ = topic;
            response = Some(serde_json::from_str::<Value>(payload).unwrap());
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let response = response.expect("configure response");
    assert_eq!(response["status"], "ok", "{response}");
    let calls = harness.transport.calls();
    assert!(calls.iter().any(|call| call["op"] == "read"), "{calls:?}");
    assert!(calls.iter().any(|call| call["op"] == "write"), "{calls:?}");
    harness.dispatcher.shutdown().await;
}
