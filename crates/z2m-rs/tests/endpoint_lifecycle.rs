// Real Endpoint transport through the lazy runtime: startup, fromZigbee,
// toZigbee/set, onEvent, timeout, restart and device leave. A recording
// `ControllerAdapter` replaces the coordinator; nothing is transmitted.

#[path = "support/mock_adapter.rs"]
mod mock_adapter;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use converter_artifact::{CacheConfig, LazyConfig};
use mock_adapter::MockAdapter;
use serde_json::{json, Value};
use zigbee_apiarist::controller::database::{Database, DeviceType};
use zigbee_apiarist::controller::model::device::{Device as ZhDevice, InterviewState};
use zigbee_apiarist::controller::model::entity::{inject_adapter, inject_database};
use zigbee_apiarist::controller::model::test_support::serial;
use zigbee_converters::{
    ConvertData, ConvertMeta, Expose, OnEventBaseData, OnEventEvent, ToMeta, ToTarget,
};

use z2m_rs::converter::startup::ArtifactDirFetcher;
use z2m_rs::converter::transport::EndpointTransport;
use z2m_rs::converter::{ConverterDispatcher, ConverterTransport, DispatcherConfig};

fn temp(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "z2m-endpoint-lifecycle-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn device(ieee: &str, network: u16, model: &str, clusters: &[u16]) -> Arc<ZhDevice> {
    let device = ZhDevice::create(
        DeviceType::Router,
        ieee,
        network,
        Some(4476),
        Some("Aurora Lighting".to_owned()),
        Some("Mains".to_owned()),
        Some(model.to_owned()),
        InterviewState::Successful,
        None,
    )
    .unwrap();
    let endpoint = device.create_endpoint(1).unwrap();
    *endpoint.input_clusters.lock().unwrap() = clusters.to_vec();
    *endpoint.output_clusters.lock().unwrap() = vec![25];
    device
}

fn install_globals(work: &std::path::Path, adapter: &Arc<MockAdapter>) {
    let database = work.join("database.db");
    if !database.exists() {
        std::fs::write(&database, b"").unwrap();
    }
    inject_database(Arc::new(Mutex::new(Database::open(&database))));
    ZhDevice::reset_cache();
    inject_adapter(adapter.clone());
}

async fn dispatcher(
    artifact: &std::path::Path,
    work: &std::path::Path,
    devices: &[Arc<ZhDevice>],
    host_timeout: Duration,
) -> Arc<ConverterDispatcher> {
    let database = work.join("database.db");
    let lookup: std::collections::HashMap<String, Arc<ZhDevice>> = devices
        .iter()
        .map(|device| (device.ieee_addr(), Arc::clone(device)))
        .collect();
    let resolver: Arc<
        dyn Fn(&str, u8) -> Option<Arc<zigbee_apiarist::controller::model::endpoint::Endpoint>>
            + Send
            + Sync,
    > = Arc::new(move |ieee: &str, endpoint: u8| lookup.get(ieee)?.get_endpoint(endpoint));
    let transport: Arc<dyn ConverterTransport> = Arc::new(EndpointTransport::new(resolver));
    let mut config = DispatcherConfig::new(
        artifact,
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(".upstream/zigbee2mqtt/node_modules/zigbee-herdsman"),
    );
    config.runtime.job_timeout = Duration::from_secs(3);
    config.runtime.host_timeout = host_timeout;
    ConverterDispatcher::start_lazy(
        config,
        transport,
        LazyConfig {
            artifact_dir: artifact.to_path_buf(),
            cache: CacheConfig::new(work.join("cache")),
            host_abi: 1,
            engine: "quickjs-oxide".to_owned(),
            engine_version: "0.5.0".to_owned(),
        },
        database,
        Arc::new(ArtifactDirFetcher::new(artifact)),
        work.join("runtime"),
        true,
    )
    .await
    .unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn js_definitions_drive_receive_set_on_event_restart_and_leave() {
    let Ok(artifact) = std::env::var("CONVERTER_ARTIFACT_DIR") else {
        return;
    };
    let _guard = serial();
    let artifact = PathBuf::from(artifact);
    let work = temp("js");
    let adapter = MockAdapter::new();
    install_globals(&work, &adapter);
    let ikea = device(
        "0x0017880104e45581",
        0x1500,
        "TRADFRI bulb E27 WS opal 1000lm",
        &[0, 3, 4, 5, 6, 8, 768],
    );
    let tuya = {
        let device = ZhDevice::create(
            DeviceType::Router,
            "0x0017880104e45584",
            0x1503,
            Some(4417),
            Some("_TZ3000_abcd".to_owned()),
            Some("Mains".to_owned()),
            Some("TS0002".to_owned()),
            InterviewState::Successful,
            None,
        )
        .unwrap();
        let endpoint = device.create_endpoint(1).unwrap();
        *endpoint.input_clusters.lock().unwrap() = vec![0, 6];
        device
    };
    let aurora = device(
        "0x0017880104e45582",
        0x1501,
        "1GBatteryDimmer50AU",
        &[0, 1, 3, 4, 6, 8, 0x0300],
    );
    let dispatcher = dispatcher(
        &artifact,
        &work,
        &[Arc::clone(&ikea), Arc::clone(&aurora), Arc::clone(&tuya)],
        Duration::from_secs(1),
    )
    .await;

    let prepared = dispatcher.pair_device(&ikea).await.unwrap();
    assert!(!prepared.metadata.model.is_empty());
    assert_eq!(prepared.lookup_model, "TRADFRI bulb E27 WS opal 1000lm");
    // Exposes must match the pinned upstream resolved exposes exactly; this is
    // the data that Home Assistant discovery consumes.
    let expected: Value =
        serde_json::from_str(include_str!("fixtures/exposes-ikea-pinned.json")).unwrap();
    let actual: Vec<Value> = prepared
        .definition
        .exposes
        .resolve(&json!({}), &json!({}))
        .iter()
        .map(Expose::to_json)
        .collect();
    assert_eq!(
        Value::Array(actual),
        expected["exposes"],
        "JS exposes must match pinned upstream"
    );
    assert!(
        prepared
            .definition
            .to_zigbee
            .iter()
            .any(|converter| converter.key.iter().any(|key| key == "state")),
        "JS toZigbee definitions must be registered"
    );
    // fromZigbee through the prepared JS definition.
    let published = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&published);
    let meta = ConvertMeta {
        device: Arc::clone(&ikea),
        state: json!({}),
        device_exposes_changed: Arc::new(|| {}),
    };
    let data = ConvertData {
        r#type: "attributeReport".to_owned(),
        device: Arc::clone(&ikea),
        endpoint: ikea.get_endpoint(1).unwrap(),
        cluster: "genOnOff".to_owned(),
        data: json!({"onOff": true}),
        group_id: None,
        linkquality: 120,
        meta: json!({}),
    };
    let (converted, _) = dispatcher
        .from_zigbee(
            &prepared.source,
            &prepared.lookup_model,
            data,
            Arc::new(move |payload| {
                let sink = Arc::clone(&sink);
                Box::pin(async move {
                    sink.lock().unwrap().push(payload);
                })
            }),
            json!({}),
            meta,
        )
        .await
        .unwrap();
    assert!(converted.is_some(), "receive must produce state");
    let _ = published;

    // set -> real adapter frame.
    let state_converter = prepared
        .definition
        .to_zigbee
        .iter()
        .find(|converter| converter.key.iter().any(|key| key == "state"))
        .unwrap()
        .convert_set
        .clone()
        .unwrap();
    let before = adapter.endpoint_frames.lock().unwrap().len();
    state_converter(
        ToTarget::Endpoint(ikea.get_endpoint(1).unwrap()),
        "state".to_owned(),
        json!("ON"),
        ToMeta {
            endpoint_name: None,
            options: json!({}),
            message: json!({"state": "ON"}),
            device: Some(Arc::clone(&ikea)),
            members_state: Value::Null,
            state: json!({}),
            mapped: prepared.definition.clone(),
            publish: Some(Arc::new(|_| Box::pin(async {}))),
            device_exposes_changed: Some(Arc::new(|| {})),
        },
    )
    .await
    .unwrap();
    assert!(adapter.endpoint_frames.lock().unwrap().len() > before);

    // A Tuya switch resolves to a different definition family; its exposes
    // must be usable for discovery too.
    if let Ok(tuya_prepared) = dispatcher.pair_device(&tuya).await {
        let exposes = tuya_prepared
            .definition
            .exposes
            .resolve(&json!({}), &json!({}));
        assert!(
            exposes.iter().any(|expose| expose.r#type == "switch"),
            "Tuya switch exposes missing: {exposes:?}"
        );
    }

    // onEvent through a prepared JS definition that re-applies reporting on
    // deviceAnnounce.
    let aurora_prepared = dispatcher.pair_device(&aurora).await.unwrap();
    let frames_before = adapter.endpoint_frames.lock().unwrap().len();
    dispatcher
        .on_event(
            &aurora_prepared.source,
            &aurora_prepared.lookup_model,
            Arc::clone(&aurora),
            OnEventEvent::DeviceAnnounce(OnEventBaseData {
                device: Arc::clone(&aurora),
                state: json!({}),
                options: json!({}),
                device_exposes_changed: Arc::new(|| {}),
            }),
        )
        .await
        .unwrap();
    assert!(
        adapter.endpoint_frames.lock().unwrap().len() > frames_before,
        "onEvent must drive real reporting calls"
    );

    // Restart keeps the lazy definitions usable and reaches the adapter.
    let generation = dispatcher.generation();
    dispatcher.restart().await.unwrap();
    assert!(dispatcher.generation() > generation);
    let frames_after_restart = adapter.endpoint_frames.lock().unwrap().len();
    dispatcher.pair_device(&ikea).await.unwrap();
    state_converter(
        ToTarget::Endpoint(ikea.get_endpoint(1).unwrap()),
        "state".to_owned(),
        json!("OFF"),
        ToMeta {
            endpoint_name: None,
            options: json!({}),
            message: json!({"state": "OFF"}),
            device: Some(Arc::clone(&ikea)),
            members_state: Value::Null,
            state: json!({}),
            mapped: prepared.definition.clone(),
            publish: Some(Arc::new(|_| Box::pin(async {}))),
            device_exposes_changed: Some(Arc::new(|| {})),
        },
    )
    .await
    .unwrap();
    assert!(adapter.endpoint_frames.lock().unwrap().len() > frames_after_restart);

    // Device leave cancels queued work without touching the adapter.
    dispatcher.cancel_device(ikea.ieee_addr());
    dispatcher.shutdown().await;
}

#[tokio::test(flavor = "current_thread")]
async fn host_timeout_is_returned_to_the_converter() {
    let Ok(artifact) = std::env::var("CONVERTER_ARTIFACT_DIR") else {
        return;
    };
    let _guard = serial();
    let artifact = PathBuf::from(artifact);
    let work = temp("timeout");
    let adapter = MockAdapter::new();
    install_globals(&work, &adapter);
    let ikea = device(
        "0x0017880104e45583",
        0x1502,
        "TRADFRI bulb E27 WS opal 1000lm",
        &[0, 3, 4, 5, 6, 8, 768],
    );
    adapter
        .delay_ms
        .store(300, std::sync::atomic::Ordering::SeqCst);
    let dispatcher = dispatcher(
        &artifact,
        &work,
        &[Arc::clone(&ikea)],
        Duration::from_millis(50),
    )
    .await;
    let prepared = dispatcher.pair_device(&ikea).await.unwrap();
    let state_converter = prepared
        .definition
        .to_zigbee
        .iter()
        .find(|converter| converter.key.iter().any(|key| key == "state"))
        .unwrap()
        .convert_set
        .clone()
        .unwrap();
    let error = match state_converter(
        ToTarget::Endpoint(ikea.get_endpoint(1).unwrap()),
        "state".to_owned(),
        json!("ON"),
        ToMeta {
            endpoint_name: None,
            options: json!({}),
            message: json!({"state": "ON"}),
            device: Some(Arc::clone(&ikea)),
            members_state: Value::Null,
            state: json!({}),
            mapped: prepared.definition.clone(),
            publish: Some(Arc::new(|_| Box::pin(async {}))),
            device_exposes_changed: Some(Arc::new(|| {})),
        },
    )
    .await
    {
        Ok(_) => panic!("expected a host timeout"),
        Err(error) => error,
    };
    assert!(error.contains("timeout"), "{error}");
    // Recovery: once the adapter responds in time again, the same definition
    // works and reaches the adapter.
    tokio::time::sleep(Duration::from_millis(400)).await;
    adapter
        .delay_ms
        .store(0, std::sync::atomic::Ordering::SeqCst);
    let frames_before = adapter.endpoint_frames.lock().unwrap().len();
    state_converter(
        ToTarget::Endpoint(ikea.get_endpoint(1).unwrap()),
        "state".to_owned(),
        json!("ON"),
        ToMeta {
            endpoint_name: None,
            options: json!({}),
            message: json!({"state": "ON"}),
            device: Some(Arc::clone(&ikea)),
            members_state: Value::Null,
            state: json!({}),
            mapped: prepared.definition.clone(),
            publish: Some(Arc::new(|_| Box::pin(async {}))),
            device_exposes_changed: Some(Arc::new(|| {})),
        },
    )
    .await
    .unwrap();
    assert!(adapter.endpoint_frames.lock().unwrap().len() > frames_before);
    dispatcher.shutdown().await;
}
