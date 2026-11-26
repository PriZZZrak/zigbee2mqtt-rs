// Real Endpoint transport: host operations reach the controller adapter and
// failures are returned unchanged (no mock fallback). A recording
// `ControllerAdapter` replaces the coordinator; nothing is transmitted.

#[path = "support/mock_adapter.rs"]
mod mock_adapter;

use std::sync::Arc;

use mock_adapter::MockAdapter;
use serde_json::{json, Value};
use zigbee_apiarist::adapter::events::ZclPayload;
use zigbee_apiarist::controller::database::{Database, DeviceType};
use zigbee_apiarist::controller::model::device::{Device as ZhDevice, InterviewState};
use zigbee_apiarist::controller::model::entity::{inject_adapter, inject_database};
use zigbee_apiarist::controller::model::test_support::serial;
use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
use zigbee_apiarist::zspec::zcl::zcl_header::{Direction, Header};
use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;

use converter_runtime::HostRequest;
use z2m_rs::converter::transport::EndpointTransport;
use z2m_rs::converter::ConverterTransport;

fn temp(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("z2m-endpoint-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn create_device(ieee: &str, network: u16, endpoint: u8) -> Arc<ZhDevice> {
    let device = ZhDevice::create(
        DeviceType::Router,
        ieee,
        network,
        None,
        Some("IKEA of Sweden".to_owned()),
        None,
        Some("TRADFRI bulb E27 WS opal 1000lm".to_owned()),
        InterviewState::Successful,
        None,
    )
    .unwrap();
    let endpoint = device.create_endpoint(endpoint).unwrap();
    *endpoint.input_clusters.lock().unwrap() = vec![0, 6, 8];
    device
}

fn setup(name: &str, devices: &[(&str, u16, u8)]) -> (Vec<Arc<ZhDevice>>, Arc<MockAdapter>) {
    let work = temp(name);
    inject_database(Arc::new(std::sync::Mutex::new(Database::open(
        work.join("database.db"),
    ))));
    ZhDevice::reset_cache();
    let adapter = MockAdapter::new();
    inject_adapter(adapter.clone());
    let devices = devices
        .iter()
        .map(|(ieee, network, endpoint)| create_device(ieee, *network, *endpoint))
        .collect();
    (devices, adapter)
}

fn transport(devices: &[Arc<ZhDevice>]) -> EndpointTransport {
    let lookup: std::collections::HashMap<String, Arc<ZhDevice>> = devices
        .iter()
        .map(|device| (device.ieee_addr(), Arc::clone(device)))
        .collect();
    EndpointTransport::new(Arc::new(move |address: &str, id: u8| {
        lookup.get(address)?.get_endpoint(id)
    }))
}

async fn execute(
    transport: &EndpointTransport,
    device: &str,
    operation: Value,
) -> Result<Value, String> {
    let request = HostRequest {
        correlation_id: 1,
        job_id: 1,
        device: device.to_owned(),
        operation,
    };
    match transport.execute(&request).await {
        converter_runtime::HostResult::Ok(value) => Ok(value),
        converter_runtime::HostResult::Err(error) => Err(error),
    }
}

fn read_response(cluster: &str, payload: Value) -> ZclPayload {
    let cluster =
        zigbee_apiarist::zspec::zcl::utils::Utils::get_cluster(cluster, None, &Default::default());
    let frame = Frame::create_with_definitions(
        cluster,
        zigbee_apiarist::zspec::zcl::utils::Utils::get_global_command("readRsp"),
        0,
        Direction::SERVER_TO_CLIENT as u8,
        true,
        None,
        1,
        payload,
        0,
    );
    let data = frame.to_buffer();
    let header = Header::from_buffer(&data).unwrap();
    ZclPayload {
        cluster_id: 6,
        data,
        header: Some(header),
        address: Value::Null,
        endpoint: 1,
        linkquality: 0,
        group_id: 0,
        was_broadcast: false,
        destination_endpoint: 1,
    }
}

#[tokio::test]
async fn real_transport_sends_through_the_adapter_and_returns_device_data() {
    let _guard = serial();
    let ieee = "0x0017880104e45571";
    let (devices, adapter) = setup("send", &[(ieee, 0x1400, 1)]);
    let transport = transport(&devices);

    let result = execute(
        &transport,
        ieee,
        json!({"op":"command","endpoint":1,"cluster":"genOnOff","command":"on","payload":{}}),
    )
    .await
    .unwrap();
    assert_eq!(result, json!({}));
    {
        let frames = adapter.endpoint_frames.lock().unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].command.name, "on");
        assert_eq!(frames[0].cluster.name, "genOnOff");
    }
    execute(
        &transport,
        ieee,
        json!({"op":"write","endpoint":1,"cluster":"genOnOff","attributes":{"onTime":10},"options":{"manufacturerCode":4660,"disableDefaultResponse":true}}),
    )
    .await
    .unwrap();
    execute(
        &transport,
        ieee,
        json!({"op":"configureReporting","endpoint":1,"cluster":"genOnOff","items":[{"attribute":"onOff","minimumReportInterval":1,"maximumReportInterval":60,"reportableChange":0}]}),
    )
    .await
    .unwrap();
    {
        let frames = adapter.endpoint_frames.lock().unwrap();
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[1].command.name, "write");
        assert_eq!(frames[1].header.manufacturer_code, Some(4660));
        assert_eq!(frames[2].command.name, "configReport");
    }
    // A read returns the device payload, not a mock default.
    *adapter.zcl_response.lock().unwrap() = Some(read_response(
        "genOnOff",
        json!([{"attrId":0,"status":0,"dataType":0x10,"attrData":1}]),
    ));
    let read = execute(
        &transport,
        ieee,
        json!({"op":"read","endpoint":1,"cluster":"genOnOff","attributes":["onOff"]}),
    )
    .await
    .unwrap();
    assert_eq!(read, json!({"onOff": 1}));
}

#[tokio::test]
async fn real_transport_propagates_adapter_errors_without_fallback() {
    let _guard = serial();
    let ieee = "0x0017880104e45572";
    let (devices, adapter) = setup("fail", &[(ieee, 0x1400, 1)]);
    let transport = transport(&devices);
    adapter
        .fail_zcl
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let error = execute(
        &transport,
        ieee,
        json!({"op":"command","endpoint":1,"cluster":"genOnOff","command":"on","payload":{}}),
    )
    .await
    .unwrap_err();
    assert!(error.contains("ZCL failed"), "{error}");
    let error = execute(
        &transport,
        ieee,
        json!({"op":"read","endpoint":1,"cluster":"genOnOff","attributes":["onOff"]}),
    )
    .await
    .unwrap_err();
    assert!(error.contains("ZCL failed"), "{error}");
    // Failures are never recorded as successful operations.
    assert!(transport.calls().is_empty());
    // Unknown devices/endpoints are explicit errors, not mock responses.
    let error = execute(
        &transport,
        "0x0017880104e455ff",
        json!({"op":"read","endpoint":1,"cluster":"genOnOff","attributes":["onOff"]}),
    )
    .await
    .unwrap_err();
    assert!(error.contains("not found"), "{error}");
}

#[tokio::test]
async fn real_bind_and_unbind_go_through_the_zdo_adapter() {
    let _guard = serial();
    let source_ieee = "0x0017880104e45573";
    let target_ieee = "0x0017880104e45574";
    let (devices, adapter) = setup(
        "bind",
        &[(source_ieee, 0x1401, 1), (target_ieee, 0x1402, 2)],
    );
    let transport = transport(&devices);
    *adapter.zdo_response.lock().unwrap() = json!([0]);
    execute(
        &transport,
        source_ieee,
        json!({"op":"bind","endpoint":1,"cluster":"genOnOff","target":2,"targetType":"endpoint","targetIeee":target_ieee}),
    )
    .await
    .unwrap();
    {
        let calls = adapter.zdo_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].2, ClusterId::BIND_REQUEST);
        assert_eq!(calls[0].0, source_ieee);
    }
    execute(
        &transport,
        source_ieee,
        json!({"op":"unbind","endpoint":1,"cluster":"genOnOff","target":2,"targetType":"endpoint","targetIeee":target_ieee,"force":true}),
    )
    .await
    .unwrap();
    {
        let calls = adapter.zdo_calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[1].2, ClusterId::UNBIND_REQUEST);
    }
    // A failing ZDO status is surfaced, not swallowed.
    adapter.zdo_response.lock().unwrap().clone_from(&json!([1]));
    let error = execute(
        &transport,
        source_ieee,
        json!({"op":"bind","endpoint":1,"cluster":"genOnOff","target":2,"targetType":"endpoint","targetIeee":target_ieee}),
    )
    .await
    .unwrap_err();
    assert!(error.contains("Status"), "{error}");
}
