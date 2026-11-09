// HIL probe for a real Z-Stack coordinator (Dongle-P / CC2652P).
//
// Run only with a passed-through serial device:
//   Z2M_SERIAL_PORT=/dev/ttyACM0 cargo test -p zigbee-apiarist \
//       --test hil_znp -- --ignored --nocapture
//
// Without `Z2M_SERIAL_PORT` the test is skipped by `#[ignore]`.

use serde_json::json;
use std::sync::Mutex;

/// HIL tests share one physical coordinator; serialize them.
static HIL_LOCK: Mutex<()> = Mutex::new(());
use zigbee_apiarist::adapter::transport::{AdapterTransport, TransportOptions};
use zigbee_apiarist::adapter::z_stack::unpi::constants::{Subsystem, Type};
use zigbee_apiarist::adapter::z_stack::znp::znp::Znp;

#[tokio::test]
#[ignore = "requires Z2M_SERIAL_PORT and a real Z-Stack coordinator"]
async fn hil_version_request() {
    let _guard = HIL_LOCK.lock().unwrap();
    let path = std::env::var("Z2M_SERIAL_PORT")
        .expect("set Z2M_SERIAL_PORT to the coordinator serial device");
    let baud_rate = std::env::var("Z2M_SERIAL_BAUD")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(115200);

    let transport = AdapterTransport::new(TransportOptions {
        path,
        baud_rate: Some(baud_rate),
        rtscts: Some(false),
    });
    let mut znp = Znp::new(transport);

    znp.open().await.expect("open coordinator");
    let reply = znp
        .request_with_reply(Subsystem::SYS, "version", json!({}))
        .await
        .expect("SYS version request");
    eprintln!("version payload: {}", reply.payload);
    znp.close().await.expect("close coordinator");

    assert!(
        reply.payload.get("product").is_some(),
        "unexpected version payload: {}",
        reply.payload
    );
}

#[tokio::test]
#[ignore = "requires Z2M_SERIAL_PORT and a real Z-Stack coordinator"]
async fn hil_nv_memory_init() {
    let _guard = HIL_LOCK.lock().unwrap();
    use zigbee_apiarist::adapter::z_stack::adapter::adapter_nv_memory::AdapterNvMemory;

    let path = std::env::var("Z2M_SERIAL_PORT")
        .expect("set Z2M_SERIAL_PORT to the coordinator serial device");
    let baud_rate = std::env::var("Z2M_SERIAL_BAUD")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(115200);

    let transport = AdapterTransport::new(TransportOptions {
        path,
        baud_rate: Some(baud_rate),
        rtscts: Some(false),
    });
    let mut znp = Znp::new(transport);
    znp.open().await.expect("open coordinator");

    let mut nv = AdapterNvMemory::new(znp);
    nv.init().await.expect("NV init");
    eprintln!("NV memory alignment: {:?}", nv.memory_alignment);
    assert!(nv.memory_alignment.is_some());

    nv.znp
        .lock()
        .await
        .close()
        .await
        .expect("close coordinator");
}

#[tokio::test]
#[ignore = "requires Z2M_SERIAL_PORT and a real Z-Stack coordinator"]
async fn hil_transport_get() {
    let _guard = HIL_LOCK.lock().unwrap();
    let path = std::env::var("Z2M_SERIAL_PORT")
        .expect("set Z2M_SERIAL_PORT to the coordinator serial device");
    let baud_rate = std::env::var("Z2M_SERIAL_BAUD")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(115200);

    let transport = AdapterTransport::new(TransportOptions {
        path,
        baud_rate: Some(baud_rate),
        rtscts: Some(false),
    });
    let mut znp = Znp::new(transport);
    znp.open().await.expect("open coordinator");
    let status = znp.transport.get().await.expect("get modem status");
    eprintln!("modem status: {status:?}");
    znp.close().await.expect("close coordinator");
}

#[tokio::test]
#[ignore = "requires Z2M_SERIAL_PORT, Z2M_HIL_DIR and a real Z-Stack coordinator"]
async fn hil_create_backup_matches_router() {
    use zigbee_apiarist::adapter::z_stack::adapter::adapter_backup::AdapterBackup;
    use zigbee_apiarist::controller::database::Database;
    use zigbee_apiarist::utils::backup::from_unified_backup;

    let _guard = HIL_LOCK.lock().unwrap();
    let path = std::env::var("Z2M_SERIAL_PORT").expect("set Z2M_SERIAL_PORT");
    let hil_dir = std::env::var("Z2M_HIL_DIR").expect("set Z2M_HIL_DIR");
    let baud_rate = std::env::var("Z2M_SERIAL_BAUD")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(115200);

    let transport = AdapterTransport::new(TransportOptions {
        path,
        baud_rate: Some(baud_rate),
        rtscts: Some(false),
    });
    let mut znp = Znp::new(transport);
    znp.open().await.expect("open coordinator");
    let mut nv =
        zigbee_apiarist::adapter::z_stack::adapter::adapter_nv_memory::AdapterNvMemory::new(znp);
    nv.init().await.expect("NV init");

    let database = Database::open(format!("{hil_dir}/router-node/database.db"));
    let ieee_addresses = database
        .all_entries()
        .filter_map(|entry| {
            entry
                .get("ieeeAddr")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .collect::<Vec<_>>();

    let mut adapter_backup = AdapterBackup::new(nv, "/tmp/opencode/hil-backup-not-present.json");
    let created = adapter_backup
        .create_backup(&ieee_addresses)
        .await
        .expect("create backup");

    let router_backup_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(format!("{hil_dir}/router-node/coordinator_backup.json"))
            .expect("router backup"),
    )
    .expect("router backup json");
    let router_backup = from_unified_backup(&router_backup_json).expect("parse router backup");

    assert_eq!(
        created.network_options.pan_id,
        router_backup.network_options.pan_id
    );
    assert_eq!(created.logical_channel, router_backup.logical_channel);
    assert_eq!(
        created.network_options.network_key,
        router_backup.network_options.network_key
    );
    assert_eq!(
        created.network_options.extended_pan_id,
        router_backup.network_options.extended_pan_id
    );
    assert_eq!(
        created.coordinator_ieee_address,
        router_backup.coordinator_ieee_address
    );
    assert_eq!(created.security_level, router_backup.security_level);
    assert_eq!(
        created.network_key_info.sequence_number,
        router_backup.network_key_info.sequence_number
    );
    assert_eq!(created.devices.len(), router_backup.devices.len());
    eprintln!(
        "backup: pan={:04x} channel={} devices={} key_len={} seed={}",
        created.network_options.pan_id,
        created.logical_channel,
        created.devices.len(),
        created.network_options.network_key.len(),
        created.trust_center_link_key_seed.is_some()
    );

    adapter_backup
        .nv
        .znp
        .lock()
        .await
        .close()
        .await
        .expect("close coordinator");
}

#[tokio::test]
#[ignore = "requires Z2M_SERIAL_PORT, Z2M_HIL_DIR and a real Z-Stack coordinator"]
async fn hil_manager_start_resumes_network() {
    use zigbee_apiarist::adapter::z_stack::adapter::adapter_nv_memory::AdapterNvMemory;
    use zigbee_apiarist::adapter::z_stack::adapter::manager::{StartResult, ZnpAdapterManager};
    use zigbee_apiarist::adapter::z_stack::adapter::tstype::ZnpVersion;
    use zigbee_apiarist::models::startup_options::{
        AdapterOptions, ConfigNetworkOptions, StartupOptions,
    };

    let _guard = HIL_LOCK.lock().unwrap();
    let path = std::env::var("Z2M_SERIAL_PORT").expect("set Z2M_SERIAL_PORT");
    let hil_dir = std::env::var("Z2M_HIL_DIR").expect("set Z2M_HIL_DIR");
    let baud_rate = std::env::var("Z2M_SERIAL_BAUD")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(115200);

    let transport = AdapterTransport::new(TransportOptions {
        path,
        baud_rate: Some(baud_rate),
        rtscts: Some(false),
    });
    let mut znp = Znp::new(transport);
    znp.open().await.expect("open coordinator");
    let nv = AdapterNvMemory::new(znp);

    let options = StartupOptions {
        version: ZnpVersion::ZStack3x0,
        network_options: ConfigNetworkOptions {
            pan_id: 0x1a62,
            extended_pan_id: Some(vec![0xdd; 8]),
            network_key: Some(vec![1, 3, 5, 7, 9, 11, 13, 15, 0, 2, 4, 6, 8, 10, 12, 13]),
            channel_list: vec![11],
            network_key_distribute: Some(false),
        },
        green_power_group: 242,
        backup_path: format!("{hil_dir}/router-node/coordinator_backup.json"),
        adapter_options: AdapterOptions::default(),
    };
    let mut manager = ZnpAdapterManager::new(
        zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::ZStackZdo::default(),
        nv,
        options,
    );
    let result = manager.start().await.expect("manager start");
    eprintln!("manager start result: {result:?}");
    assert_eq!(result, StartResult::Resumed);
    manager
        .backup
        .nv
        .znp
        .lock()
        .await
        .close()
        .await
        .expect("close coordinator");
}

#[tokio::test]
#[ignore = "requires Z2M_SERIAL_PORT, Z2M_HIL_DIR and a real Z-Stack coordinator"]
async fn hil_zstack_adapter_start_stop() {
    use zigbee_apiarist::adapter::z_stack::adapter::manager::StartResult;
    use zigbee_apiarist::adapter::z_stack::adapter::tstype::ZnpVersion;
    use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::ZStackAdapter;
    use zigbee_apiarist::models::startup_options::{AdapterOptions, ConfigNetworkOptions};

    let _guard = HIL_LOCK.lock().unwrap();
    let path = std::env::var("Z2M_SERIAL_PORT").expect("set Z2M_SERIAL_PORT");
    let hil_dir = std::env::var("Z2M_HIL_DIR").expect("set Z2M_HIL_DIR");
    let baud_rate = std::env::var("Z2M_SERIAL_BAUD")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(115200);

    let transport = AdapterTransport::new(TransportOptions {
        path,
        baud_rate: Some(baud_rate),
        rtscts: Some(false),
    });
    let mut adapter = ZStackAdapter::new(
        transport,
        ConfigNetworkOptions {
            pan_id: 0x1a62,
            extended_pan_id: Some(vec![0xdd; 8]),
            network_key: Some(vec![1, 3, 5, 7, 9, 11, 13, 15, 0, 2, 4, 6, 8, 10, 12, 13]),
            channel_list: vec![11],
            network_key_distribute: Some(false),
        },
        format!("{hil_dir}/router-node/coordinator_backup.json"),
        AdapterOptions::default(),
    );

    let result = adapter.start().await.expect("adapter start");
    eprintln!("zstack adapter start: {result:?}");
    assert_eq!(result, StartResult::Resumed);

    let ieee = adapter
        .get_coordinator_ieee()
        .await
        .expect("coordinator IEEE");
    eprintln!("coordinator IEEE: {ieee}");
    assert_eq!(ieee, "0x00124b0024c2d400");

    let version = adapter.get_coordinator_version().expect("version");
    eprintln!("coordinator version: {version:?}");
    assert_eq!(version.type_name, "ZStack3x0");
    assert_eq!(
        version
            .meta
            .get("product")
            .and_then(serde_json::Value::as_u64),
        Some(ZnpVersion::ZStack3x0 as u64)
    );

    adapter.stop().await.expect("adapter stop");
}
