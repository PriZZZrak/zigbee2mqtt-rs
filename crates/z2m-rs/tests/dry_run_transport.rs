// Dry-run Endpoint transport: real Endpoint/ZCL construction, recorded frames,
// no adapter calls. Frame bytes are compared with the pinned upstream oracle.

use std::sync::Arc;

use converter_runtime::HostRequest;
use serde_json::{json, Value};
use z2m_rs::converter::transport::DryRunEndpointTransport;
use z2m_rs::converter::ConverterTransport;
use zigbee_apiarist::controller::database::{Database, DeviceType};
use zigbee_apiarist::controller::model::device::{Device as ZhDevice, InterviewState};
use zigbee_apiarist::controller::model::entity::inject_database;

fn temp(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("z2m-dry-run-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn golden() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../zigbee-apiarist/tests/fixtures/frame-options-pinned.json"
    ))
    .unwrap()
}

fn golden_hex(name: &str) -> String {
    golden()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == name)
        .unwrap()["expected"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn request(ieee: &str, operation: Value) -> HostRequest {
    HostRequest {
        correlation_id: 1,
        job_id: 1,
        device: ieee.to_owned(),
        operation,
    }
}

async fn execute(
    transport: &DryRunEndpointTransport,
    ieee: &str,
    operation: Value,
) -> Result<Value, String> {
    match transport.execute(&request(ieee, operation)).await {
        converter_runtime::HostResult::Ok(value) => Ok(value),
        converter_runtime::HostResult::Err(error) => Err(error),
    }
}

fn bytes(call: &Value) -> String {
    let bytes = call["frame"]["bytes"].as_array().expect("frame bytes");
    bytes
        .iter()
        .map(|byte| format!("{:02x}", byte.as_u64().unwrap_or(0)))
        .collect()
}

#[tokio::test]
async fn dry_run_transport_records_real_frames_and_serves_cached_reads() {
    let work = temp("transport");
    inject_database(Arc::new(std::sync::Mutex::new(Database::open(
        work.join("database.db"),
    ))));
    ZhDevice::reset_cache();
    let ieee = "0x0017880104e45551";
    let device = ZhDevice::create(
        DeviceType::Router,
        ieee,
        0x1234,
        Some(4476),
        Some("IKEA of Sweden".to_owned()),
        Some("Mains".to_owned()),
        Some("TRADFRI bulb E27 WS opal 1000lm".to_owned()),
        InterviewState::Successful,
        None,
    )
    .unwrap();
    let endpoint = device.create_endpoint(1).unwrap();
    *endpoint.input_clusters.lock().unwrap() = vec![0, 3, 4, 5, 6, 8, 768];
    *endpoint.output_clusters.lock().unwrap() = vec![25];

    let resolver_device = Arc::clone(&device);
    let transport = DryRunEndpointTransport::new(Arc::new(move |address: &str, id: u8| {
        if address != resolver_device.ieee_addr() || id != 1 {
            return None;
        }
        resolver_device.get_endpoint(1)
    }));
    transport.set_read(ieee, "genOnOff", "onOff", json!(true));

    // Standard command with an explicit default-response option.
    execute(
        &transport,
        ieee,
        json!({"op":"command","endpoint":1,"cluster":"genOnOff","command":"on","payload":{},"options":{"disableDefaultResponse":false,"transactionSequenceNumber":42}}),
    )
    .await
    .unwrap();
    // Manufacturer-specific write.
    execute(
        &transport,
        ieee,
        json!({"op":"write","endpoint":1,"cluster":"genBasic","attributes":{"16384":{"type":66,"value":"abc"}},"options":{"manufacturerCode":4660,"transactionSequenceNumber":5}}),
    )
    .await
    .unwrap();
    // Read served from the seeded value through a synthetic readRsp.
    let read = execute(
        &transport,
        ieee,
        json!({"op":"read","endpoint":1,"cluster":"genOnOff","attributes":["onOff"],"options":{"transactionSequenceNumber":11}}),
    )
    .await
    .unwrap();
    // ZCL BOOLEAN is decoded as 0/1, matching upstream `BuffaloZcl`.
    assert_eq!(read, json!({"onOff": 1}));
    // Configure reporting keeps the real configReport frame.
    execute(
        &transport,
        ieee,
        json!({"op":"configureReporting","endpoint":1,"cluster":"genOnOff","items":[{"attribute":"onOff","minimumReportInterval":1,"maximumReportInterval":60,"reportableChange":0}],"options":{"transactionSequenceNumber":12}}),
    )
    .await
    .unwrap();
    // Bind is recorded as an intent (ZDO, not a ZCL frame).
    execute(
        &transport,
        ieee,
        json!({"op":"bind","endpoint":1,"cluster":"genOnOff","target":1,"targetType":"endpoint"}),
    )
    .await
    .unwrap();

    let calls = transport.calls();
    let call = |op: &str, cluster: &str| {
        calls
            .iter()
            .find(|call| call["op"] == op && call["cluster"] == cluster)
            .unwrap_or_else(|| panic!("missing {op} {cluster}: {calls:?}"))
    };
    let command = call("command", "genOnOff");
    assert_eq!(bytes(command), golden_hex("onOff-on-default-response"));
    assert_eq!(command["frame"]["manufacturerCode"], Value::Null);
    assert_eq!(command["frame"]["direction"], 0);
    assert_eq!(command["frame"]["options"]["disableDefaultResponse"], false);
    let write = call("write", "genBasic");
    assert_eq!(bytes(write), golden_hex("manufacturer-write-attribute"));
    assert_eq!(write["frame"]["manufacturerCode"], 4660);
    assert_eq!(write["frame"]["direction"], 0);
    let read_call = call("read", "genOnOff");
    assert_eq!(bytes(read_call), golden_hex("read-attribute"));
    let reporting = call("configureReporting", "genOnOff");
    assert_eq!(bytes(reporting), golden_hex("configure-reporting"));
    let bind = call("bind", "genOnOff");
    assert_eq!(bind["target"], 1);
    assert!(bind.get("frame").is_none(), "bind is a ZDO intent");
}

#[tokio::test]
async fn dry_run_transport_resolves_every_manufacturer_code_and_endpoint() {
    let work = temp("manufacturer");
    inject_database(Arc::new(std::sync::Mutex::new(Database::open(
        work.join("database.db"),
    ))));
    ZhDevice::reset_cache();
    let ieee = "0x0017880104e45552";
    let device = ZhDevice::create(
        DeviceType::Router,
        ieee,
        0x1235,
        None,
        Some("Amazon".to_owned()),
        None,
        Some("Amazon device".to_owned()),
        InterviewState::Successful,
        None,
    )
    .unwrap();
    device.create_endpoint(1).unwrap();
    let resolver_device = Arc::clone(&device);
    let transport = DryRunEndpointTransport::new(Arc::new(move |address: &str, id: u8| {
        if address != resolver_device.ieee_addr() {
            return None;
        }
        resolver_device.get_endpoint(id)
    }));
    // Unknown endpoint must be an explicit error, never a silent success.
    assert!(execute(
        &transport,
        ieee,
        json!({"op":"command","endpoint":9,"cluster":"genOnOff","command":"on","payload":{}})
    )
    .await
    .is_err());
    execute(
        &transport,
        ieee,
        json!({"op":"command","endpoint":1,"cluster":"manuSpecificAmazonWWAH","command":"clearBindingTable","payload":{},"options":{"manufacturerCode":4631,"disableDefaultResponse":true,"transactionSequenceNumber":18}}),
    )
    .await
    .unwrap();
    let calls = transport.calls();
    assert_eq!(
        bytes(&calls[calls.len() - 1]),
        golden_hex("manufacturer-cluster-command")
    );
    assert_eq!(calls[calls.len() - 1]["frame"]["manufacturerCode"], 4631);
}

#[tokio::test]
async fn dry_run_transport_encodes_custom_cluster_commands() {
    use zigbee_apiarist::zspec::zcl::definition::enums::DataType;
    use zigbee_apiarist::zspec::zcl::utils::{Cluster, Command, Parameter};

    let work = temp("custom");
    inject_database(Arc::new(std::sync::Mutex::new(Database::open(
        work.join("database.db"),
    ))));
    ZhDevice::reset_cache();
    let ieee = "0x0017880104e45553";
    let device = ZhDevice::create(
        DeviceType::Router,
        ieee,
        0x1236,
        None,
        Some("Custom".to_owned()),
        None,
        Some("Custom device".to_owned()),
        InterviewState::Successful,
        None,
    )
    .unwrap();
    device.create_endpoint(1).unwrap();
    let mut cluster = Cluster::new("customCluster", 0xfc00);
    cluster.manufacturerCode = Some(0xabcd);
    let mut command = Command::new("setValue", 0x01);
    command.parameters = vec![Parameter::new("value", DataType::UINT8)];
    command.order = 0;
    cluster.commands.insert(command.name.clone(), command);
    device.add_custom_cluster("customCluster", cluster).unwrap();
    let resolver_device = Arc::clone(&device);
    let transport = DryRunEndpointTransport::new(Arc::new(move |address: &str, id: u8| {
        if address != resolver_device.ieee_addr() {
            return None;
        }
        resolver_device.get_endpoint(id)
    }));
    execute(
        &transport,
        ieee,
        json!({"op":"command","endpoint":1,"cluster":"customCluster","command":"setValue","payload":{"value":42},"options":{"manufacturerCode":43981,"disableDefaultResponse":true,"transactionSequenceNumber":17}}),
    )
    .await
    .unwrap();
    let calls = transport.calls();
    assert_eq!(
        bytes(&calls[calls.len() - 1]),
        golden_hex("custom-cluster-command")
    );
    assert_eq!(calls[calls.len() - 1]["frame"]["manufacturerCode"], 43981);
    assert_eq!(calls[calls.len() - 1]["frame"]["endpoint"], 1);
}
