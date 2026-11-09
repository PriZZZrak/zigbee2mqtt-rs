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
    let _guard = HIL_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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
    let _guard = HIL_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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
    let _guard = HIL_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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

    let _guard = HIL_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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

    let _guard = HIL_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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

    let _guard = HIL_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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

#[tokio::test]
#[ignore = "requires Z2M_SERIAL_PORT, Z2M_HIL_DIR and a real Z-Stack coordinator"]
async fn hil_zstack_adapter_send_zcl_broadcast() {
    use serde_json::json;
    use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::ZStackAdapter;
    use zigbee_apiarist::models::startup_options::{AdapterOptions, ConfigNetworkOptions};
    use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;

    let _guard = HIL_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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
    adapter.start().await.expect("adapter start");

    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    adapter
        .send_zcl_frame_to_all(1, &frame, 1, 0xfffc, Some(0x0104))
        .await
        .expect("broadcast ZCL");
    eprintln!("broadcast ZCL read sent");
    adapter.stop().await.expect("adapter stop");
}

#[tokio::test]
#[ignore = "requires Z2M_SERIAL_PORT, Z2M_HIL_DIR and a real Z-Stack coordinator"]
async fn hil_zstack_adapter_zcl_read_roundtrip() {
    use serde_json::json;
    use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::ZStackAdapter;
    use zigbee_apiarist::models::startup_options::{AdapterOptions, ConfigNetworkOptions};
    use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;

    let _guard = HIL_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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
    adapter.start().await.expect("adapter start");

    let ieee = "0x404ccafffe5c4ce0"; // kitchen thermostat (router)
    let network_address = adapter
        .request_network_address(ieee)
        .await
        .expect("network address");
    eprintln!("thermostat network address: {network_address}");

    let frame = Frame::create(
        0,
        0,
        true,
        None,
        7,
        "read",
        0x0000,
        json!([{"attribute": 5}]),
        json!({}),
        0,
    );
    let response = adapter
        .send_zcl_frame_to_endpoint(
            ieee,
            network_address,
            1,
            &frame,
            10000,
            false,
            false,
            Some(1),
            Some(0x0104),
        )
        .await
        .expect("ZCL read");
    assert!(response.is_some(), "expected a ZCL read response");
    let payload = response.unwrap();
    eprintln!(
        "ZCL read response: cluster={} endpoint={} cmd={} data={}",
        payload.cluster_id,
        payload.endpoint,
        payload.header.command_identifier,
        payload
            .data
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    assert_eq!(payload.cluster_id, 0x0000);
    assert_eq!(payload.endpoint, 1);

    adapter.stop().await.expect("adapter stop");
}

#[tokio::test]
#[ignore = "requires Z2M_SERIAL_PORT, Z2M_HIL_DIR and a real Z-Stack coordinator"]
async fn hil_zstack_network_parameters_and_backup() {
    use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::ZStackAdapter;
    use zigbee_apiarist::controller::database::Database;
    use zigbee_apiarist::models::startup_options::{AdapterOptions, ConfigNetworkOptions};

    let _guard = HIL_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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
    adapter.start().await.expect("adapter start");

    let parameters = adapter
        .get_network_parameters()
        .await
        .expect("network parameters");
    eprintln!("network parameters: {parameters:?}");
    assert_eq!(parameters.pan_id, 0x1a62);
    assert_eq!(parameters.channel, 11);
    assert_eq!(parameters.extended_pan_id, "0x00124b0024c2d400");
    assert_eq!(parameters.nwk_update_id, 0);

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
    let backup = adapter.backup(&ieee_addresses).await.expect("backup");
    eprintln!(
        "adapter backup: pan={:04x} channel={} devices={}",
        backup.network_options.pan_id,
        backup.logical_channel,
        backup.devices.len()
    );
    assert_eq!(backup.network_options.pan_id, 0x1a62);
    assert_eq!(backup.devices.len(), 9);

    adapter.stop().await.expect("adapter stop");
}

#[tokio::test]
#[ignore = "requires Z2M_SERIAL_PORT, Z2M_HIL_DIR and a real Z-Stack coordinator"]
async fn hil_zstack_interpan_set_restore() {
    use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::ZStackAdapter;
    use zigbee_apiarist::models::startup_options::{AdapterOptions, ConfigNetworkOptions};

    let _guard = HIL_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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
    adapter.start().await.expect("adapter start");
    adapter
        .set_channel_inter_pan(11)
        .await
        .expect("set interPAN");
    eprintln!("interPAN channel set");
    adapter
        .restore_channel_inter_pan()
        .await
        .expect("restore interPAN");
    eprintln!("interPAN restored");
    adapter.stop().await.expect("adapter stop");
}

#[tokio::test]
#[ignore = "requires Z2M_SERIAL_PORT, Z2M_HIL_DIR and a real Z-Stack coordinator"]
async fn hil_zstack_active_endpoints_match_upstream() {
    use serde_json::json;
    use zigbee_apiarist::adapter::z_stack::adapter::endpoints::endpoints;
    use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::ZStackAdapter;
    use zigbee_apiarist::models::startup_options::{AdapterOptions, ConfigNetworkOptions};
    use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
    use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;

    let _guard = HIL_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
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
    adapter.start().await.expect("adapter start");

    let cluster_id = ClusterId::ACTIVE_ENDPOINTS_REQUEST;
    let payload = BuffaloZdo::build_request(false, cluster_id, &[json!(0)]);
    let response = adapter
        .send_zdo("0x0000000000000000", 0, cluster_id, &payload, false)
        .await
        .expect("active endpoints");
    assert_eq!(response.get(0).and_then(serde_json::Value::as_u64), Some(0));

    let active: Vec<u64> = response
        .get(1)
        .and_then(|payload| payload.get("endpointList"))
        .and_then(serde_json::Value::as_array)
        .map(|items| items.iter().filter_map(serde_json::Value::as_u64).collect())
        .unwrap_or_default();
    eprintln!("coordinator active endpoints: {active:?}");

    for endpoint in endpoints() {
        assert!(
            active.contains(&(endpoint.endpoint as u64)),
            "upstream endpoint {} is not registered on the coordinator",
            endpoint.endpoint
        );
    }

    adapter.stop().await.expect("adapter stop");
}

#[tokio::test]
#[ignore = "requires Z2M_SERIAL_PORT, Z2M_HIL_DIR and a real Z-Stack coordinator"]
async fn hil_manager_restore_backup_recovers_network() {
    use serde_json::json;
    use zigbee_apiarist::adapter::z_stack::adapter::adapter_backup::AdapterBackup;
    use zigbee_apiarist::adapter::z_stack::adapter::adapter_nv_memory::AdapterNvMemory;
    use zigbee_apiarist::adapter::z_stack::adapter::manager::{StartResult, ZnpAdapterManager};
    use zigbee_apiarist::adapter::z_stack::adapter::tstype::ZnpVersion;
    use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::ZStackZdo;
    use zigbee_apiarist::adapter::z_stack::constants::common::NvItemsIds;
    use zigbee_apiarist::models::startup_options::{
        AdapterOptions, ConfigNetworkOptions, StartupOptions,
    };
    use zigbee_apiarist::utils::backup::from_unified_backup;

    let _guard = HIL_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if std::env::var("Z2M_HIL_DESTRUCTIVE").ok().as_deref() != Some("1") {
        eprintln!("skipping destructive HIL test (set Z2M_HIL_DESTRUCTIVE=1)");
        return;
    }
    let path = std::env::var("Z2M_SERIAL_PORT").expect("set Z2M_SERIAL_PORT");
    let hil_dir = std::env::var("Z2M_HIL_DIR").expect("set Z2M_HIL_DIR");
    let baud_rate = std::env::var("Z2M_SERIAL_BAUD")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(115200);
    let backup_path = format!("{hil_dir}/router-node/coordinator_backup.json");

    let expected = {
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&backup_path).expect("router backup"))
                .expect("router backup json");
        from_unified_backup(&value).expect("parse router backup")
    };
    eprintln!(
        "expected: pan={:04x} channel={} devices={}",
        expected.network_options.pan_id,
        expected.logical_channel,
        expected.devices.len()
    );

    let transport = AdapterTransport::new(TransportOptions {
        path,
        baud_rate: Some(baud_rate),
        rtscts: Some(false),
    });
    let mut znp = Znp::new(transport);
    znp.open().await.expect("open coordinator");
    let mut nv = AdapterNvMemory::new(znp);
    nv.init().await.expect("NV init");

    // Simulate a fresh adapter: the has-configured flag is not 0x55.
    nv.write_item(NvItemsIds::ZNP_HAS_CONFIGURED_ZSTACK3, &[0x00], 0, true)
        .await
        .expect("clear has-configured flag");

    let options = StartupOptions {
        version: ZnpVersion::ZStack3x0,
        network_options: ConfigNetworkOptions {
            pan_id: expected.network_options.pan_id,
            extended_pan_id: Some(expected.network_options.extended_pan_id.clone()),
            network_key: Some(expected.network_options.network_key.clone()),
            channel_list: expected.network_options.channel_list.clone(),
            network_key_distribute: Some(expected.network_options.network_key_distribute),
        },
        green_power_group: 242,
        backup_path: backup_path.clone(),
        adapter_options: AdapterOptions::default(),
    };
    let mut manager = ZnpAdapterManager::new(ZStackZdo::default(), nv, options);
    let result = manager.start().await.expect("restore startup");
    eprintln!("restore startup result: {result:?}");
    assert_eq!(result, StartResult::Restored);

    let params = manager
        .backup
        .nv
        .znp
        .lock()
        .await
        .request_with_reply(Subsystem::ZDO, "extNwkInfo", json!({}))
        .await
        .expect("network parameters");
    let pan_id = params
        .payload
        .get("panid")
        .and_then(serde_json::Value::as_u64);
    let channel = params
        .payload
        .get("channel")
        .and_then(serde_json::Value::as_u64);
    eprintln!("restored network: panId={pan_id:?} channel={channel:?}");
    assert_eq!(pan_id, Some(expected.network_options.pan_id as u64));
    assert_eq!(
        channel,
        expected
            .network_options
            .channel_list
            .first()
            .copied()
            .map(u64::from)
    );

    // The restored backup must match the stored router backup.
    let restored = {
        let mut backup_nv = AdapterNvMemory::from_shared(manager.backup.nv.znp.clone());
        backup_nv.init().await.expect("restored NV init");
        let mut backup = AdapterBackup::new(backup_nv, backup_path.clone());
        backup.create_backup(&[]).await.expect("restored backup")
    };
    assert_eq!(
        restored.network_options.pan_id,
        expected.network_options.pan_id
    );
    assert_eq!(
        restored.network_options.extended_pan_id,
        expected.network_options.extended_pan_id
    );
    assert_eq!(
        restored.network_options.network_key,
        expected.network_options.network_key
    );
    assert_eq!(restored.logical_channel, expected.logical_channel);
    assert_eq!(
        restored.coordinator_ieee_address,
        expected.coordinator_ieee_address
    );
    let expected_devices = expected
        .devices
        .iter()
        .map(|device| device.ieee_address.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let restored_devices = restored
        .devices
        .iter()
        .map(|device| device.ieee_address.clone())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        expected_devices.is_subset(&restored_devices),
        "restored backup lost devices"
    );

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
async fn hil_manager_commissioning_then_restore() {
    use serde_json::json;
    use zigbee_apiarist::adapter::z_stack::adapter::adapter_nv_memory::AdapterNvMemory;
    use zigbee_apiarist::adapter::z_stack::adapter::manager::{StartResult, ZnpAdapterManager};
    use zigbee_apiarist::adapter::z_stack::adapter::tstype::ZnpVersion;
    use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::ZStackZdo;
    use zigbee_apiarist::models::startup_options::{
        AdapterOptions, ConfigNetworkOptions, StartupOptions,
    };
    use zigbee_apiarist::utils::backup::from_unified_backup;

    let _guard = HIL_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if std::env::var("Z2M_HIL_DESTRUCTIVE").ok().as_deref() != Some("1") {
        eprintln!("skipping destructive HIL test (set Z2M_HIL_DESTRUCTIVE=1)");
        return;
    }

    let path = std::env::var("Z2M_SERIAL_PORT").expect("set Z2M_SERIAL_PORT");
    let hil_dir = std::env::var("Z2M_HIL_DIR").expect("set Z2M_HIL_DIR");
    let baud_rate = std::env::var("Z2M_SERIAL_BAUD")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(115200);
    let backup_path = format!("{hil_dir}/router-node/coordinator_backup.json");

    let expected = {
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&backup_path).expect("router backup"))
                .expect("router backup json");
        from_unified_backup(&value).expect("parse router backup")
    };

    let transport = AdapterTransport::new(TransportOptions {
        path,
        baud_rate: Some(baud_rate),
        rtscts: Some(false),
    });
    let mut znp = Znp::new(transport);
    znp.open().await.expect("open coordinator");
    let mut nv = AdapterNvMemory::new(znp);
    nv.init().await.expect("NV init");

    // Commissioning against a fresh, non-matching network and no backup.
    let commissioned = StartupOptions {
        version: ZnpVersion::ZStack3x0,
        network_options: ConfigNetworkOptions {
            pan_id: 0x1b63,
            extended_pan_id: Some(vec![0xaa; 8]),
            network_key: Some(vec![
                0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
                0xff, 0x00,
            ]),
            channel_list: vec![15],
            network_key_distribute: Some(false),
        },
        green_power_group: 242,
        backup_path: "/tmp/opencode/hil-commissioning-no-backup.json".to_string(),
        adapter_options: AdapterOptions::default(),
    };
    let mut manager = ZnpAdapterManager::new(ZStackZdo::default(), nv, commissioned);
    let result = manager.start().await.expect("commissioning startup");
    eprintln!("commissioning startup result: {result:?}");
    assert_eq!(result, StartResult::Reset);

    let params = manager
        .backup
        .nv
        .znp
        .lock()
        .await
        .request_with_reply(Subsystem::ZDO, "extNwkInfo", json!({}))
        .await
        .expect("commissioned network parameters");
    eprintln!(
        "commissioned network: panId={:?} channel={:?}",
        params.payload.get("panid"),
        params.payload.get("channel")
    );
    assert_eq!(
        params
            .payload
            .get("panid")
            .and_then(serde_json::Value::as_u64),
        Some(0x1b63)
    );
    assert_eq!(
        params
            .payload
            .get("channel")
            .and_then(serde_json::Value::as_u64),
        Some(15)
    );

    // Recover the router network from the stored backup.
    let restore = StartupOptions {
        version: ZnpVersion::ZStack3x0,
        network_options: ConfigNetworkOptions {
            pan_id: expected.network_options.pan_id,
            extended_pan_id: Some(expected.network_options.extended_pan_id.clone()),
            network_key: Some(expected.network_options.network_key.clone()),
            channel_list: expected.network_options.channel_list.clone(),
            network_key_distribute: Some(expected.network_options.network_key_distribute),
        },
        green_power_group: 242,
        backup_path: backup_path.clone(),
        adapter_options: AdapterOptions::default(),
    };
    let mut manager = ZnpAdapterManager::new(ZStackZdo::default(), manager.backup.nv, restore);
    let result = manager.start().await.expect("recovery startup");
    eprintln!("recovery startup result: {result:?}");
    assert_eq!(result, StartResult::Restored);

    let params = manager
        .backup
        .nv
        .znp
        .lock()
        .await
        .request_with_reply(Subsystem::ZDO, "extNwkInfo", json!({}))
        .await
        .expect("recovered network parameters");
    eprintln!(
        "recovered network: panId={:?} channel={:?}",
        params.payload.get("panid"),
        params.payload.get("channel")
    );
    assert_eq!(
        params
            .payload
            .get("panid")
            .and_then(serde_json::Value::as_u64),
        Some(expected.network_options.pan_id as u64)
    );
    assert_eq!(
        params
            .payload
            .get("channel")
            .and_then(serde_json::Value::as_u64),
        expected
            .network_options
            .channel_list
            .first()
            .copied()
            .map(u64::from)
    );

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
