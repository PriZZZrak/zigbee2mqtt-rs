// Lazy converter lifecycle: existing devices are prepared at startup, new
// devices are paired in a bounded background queue, and reports submitted while
// a pairing download/prepare runs on the shared JS worker are not lost.
//
// Requires the generated artifact (CONVERTER_ARTIFACT_DIR); no coordinator, MQTT
// broker, Home Assistant or Zigbee transmission is involved.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use converter_artifact::{CacheConfig, FetchError, LazyConfig, ModuleFetcher, ModuleRecord};
use serde_json::{json, Value};
use zigbee_apiarist::controller::controller::{Controller, ControllerOptions};
use zigbee_apiarist::controller::database::{Database, DeviceType};
use zigbee_apiarist::controller::model::device::{Device as ZhDevice, InterviewState};
use zigbee_apiarist::controller::model::entity::inject_database;
use zigbee_apiarist::models::network_options::NetworkOptions;
use zigbee_converters::{ConvertData, ConvertMeta};

use z2m_rs::converter::startup::ArtifactDirFetcher;
use z2m_rs::converter::{ConverterDispatcher, DispatcherConfig, OfflineMockTransport};
use z2m_rs::event_bus::EventBus;
use z2m_rs::util::logger::Logger;
use z2m_rs::util::settings::Settings;
use z2m_rs::zigbee::Zigbee;

const IKEA: &str = "0x0017880104e45531";

fn temp(name: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "z2m-pairing-{name}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

struct SlowFetcher {
    inner: ArtifactDirFetcher,
    delay: Duration,
}
impl ModuleFetcher for SlowFetcher {
    fn fetch(&self, module: &ModuleRecord) -> Result<Vec<u8>, FetchError> {
        std::thread::sleep(self.delay);
        self.inner.fetch(module)
    }
}

fn create_device(
    ieee: &str,
    network: u16,
    manufacturer: &str,
    model: &str,
    clusters: &[u16],
    out: &[u16],
) -> Arc<ZhDevice> {
    let device = ZhDevice::create(
        DeviceType::Router,
        ieee,
        network,
        Some(4476),
        Some(manufacturer.to_owned()),
        Some("Mains".to_owned()),
        Some(model.to_owned()),
        InterviewState::Successful,
        None,
    )
    .unwrap();
    let endpoint = device.create_endpoint(1).unwrap();
    *endpoint.input_clusters.lock().unwrap() = clusters.to_vec();
    *endpoint.output_clusters.lock().unwrap() = out.to_vec();
    device.save(true);
    device
}

async fn wait_definition(device: &Arc<z2m_rs::model::device::Device>) -> bool {
    for _ in 0..1500 {
        device.resolve_definition(false);
        if device.definition().is_some() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    false
}

#[tokio::test(flavor = "current_thread")]
async fn new_device_pairs_in_background_and_reports_survive_pairing() {
    let Ok(artifact) = std::env::var("CONVERTER_ARTIFACT_DIR") else {
        return;
    };
    let artifact = PathBuf::from(artifact);
    let work = temp("work");
    let database_path = work.join("database.db");
    inject_database(Arc::new(Mutex::new(Database::open(&database_path))));
    ZhDevice::reset_cache();
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
    let _ikea_zh = create_device(
        IKEA,
        4660,
        "IKEA of Sweden",
        "TRADFRI bulb E27 WS opal 1000lm",
        &[0, 3, 4, 5, 6, 8, 768],
        &[25],
    );
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
        "devices": {IKEA: {"friendly_name": "ikea"}},
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
    assert!(zigbee.resolve_device(IKEA).is_some());

    let lazy = LazyConfig {
        artifact_dir: artifact.clone(),
        cache: CacheConfig::new(work.join("cache")),
        host_abi: 1,
        engine: "quickjs-oxide".to_owned(),
        engine_version: "0.5.0".to_owned(),
    };
    let fetcher = Arc::new(SlowFetcher {
        inner: ArtifactDirFetcher::new(&artifact),
        delay: Duration::from_millis(40),
    });
    let dispatcher = ConverterDispatcher::start_lazy(
        DispatcherConfig::new(
            &artifact,
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(".upstream/zigbee2mqtt/node_modules/zigbee-herdsman"),
        ),
        Arc::new(OfflineMockTransport::new()),
        lazy,
        database_path.clone(),
        fetcher.clone(),
        work.join("runtime"),
        true,
    )
    .await
    .unwrap();
    zigbee.set_converter_dispatcher(dispatcher.clone());
    // Existing devices are prepared first and are not queued for pairing.
    let prepared = zigbee.prepare_converter_definitions().await;
    assert_eq!(prepared, 1);
    let ikea = zigbee.resolve_device(IKEA).unwrap();
    assert!(ikea.definition().is_some());
    assert_eq!(zigbee.pairing_queue_len(), 0);

    // A new device (not present in database.db at startup) is queued.
    create_device(
        "0x0017880104e45541",
        0x1240,
        "_TZ3000_abcd",
        "TS0002",
        &[0, 6],
        &[25],
    );
    let tuya = zigbee.resolve_device("0x0017880104e45541").unwrap();
    zigbee.schedule_converter_pairing(tuya.clone());
    // Duplicate announcements must not grow the queue.
    zigbee.schedule_converter_pairing(tuya.clone());
    assert!(zigbee.pairing_queue_len() <= 1);
    assert!(wait_definition(&tuya).await, "new device was not paired");
    assert_eq!(zigbee.pairing_queue_len(), 0);

    // Reports submitted while another new device is paired must all complete.
    let prepared_ikea = dispatcher.prepare(&ikea.zh).await.unwrap();
    let source = prepared_ikea.source.clone();
    let model = prepared_ikea.lookup_model.clone();
    let third = create_device(
        "0x0017880104e45542",
        0x1241,
        "_TZ3000_abcd",
        "TS0002",
        &[0, 6],
        &[25],
    );
    let pairing = tokio::spawn({
        let dispatcher = dispatcher.clone();
        let third = third.clone();
        async move { dispatcher.pair_device(&third).await }
    });
    let mut reports_ok = 0;
    let iterations = 120;
    for _ in 0..iterations {
        let meta = ConvertMeta {
            device: ikea.zh.clone(),
            state: json!({}),
            device_exposes_changed: Arc::new(|| {}),
        };
        let data = ConvertData {
            r#type: "attributeReport".to_owned(),
            device: ikea.zh.clone(),
            endpoint: ikea.zh.get_endpoint(1).unwrap(),
            cluster: "genOnOff".to_owned(),
            data: json!({"onOff": true}),
            group_id: None,
            linkquality: 120,
            meta: json!({}),
        };
        dispatcher
            .from_zigbee(
                &source,
                &model,
                data,
                Arc::new(|_| Box::pin(async {})),
                json!({}),
                meta,
            )
            .await
            .unwrap_or_else(|error| panic!("report {reports_ok} was lost: {error}"));
        reports_ok += 1;
    }
    assert_eq!(reports_ok, iterations);
    assert!(pairing.await.unwrap().is_ok(), "background pairing failed");
    dispatcher.shutdown().await;
}
