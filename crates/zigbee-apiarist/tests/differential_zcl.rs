// Differential fixtures generated from zigbee-herdsman v11.2.0:
// tools/oracle/generate.mjs -> fixtures/zcl.json.

use serde::Deserialize;
use serde_json::Value;
use zigbee_apiarist::zspec::zcl::utils::{ClusterKey, CommandKey, CustomClusters, Utils};
use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    op: String,
    key: Option<Value>,
    cluster: Option<String>,
    expected: Value,
    #[serde(rename = "frameType")]
    frame_type: Option<u8>,
    direction: Option<u8>,
    #[serde(rename = "disableDefaultResponse")]
    disable_default_response: Option<bool>,
    #[serde(rename = "manufacturerCode")]
    manufacturer_code: Option<u16>,
    #[serde(rename = "transactionSequenceNumber")]
    transaction_sequence_number: Option<u8>,
    command: Option<String>,
    payload: Option<Value>,
    #[serde(rename = "reservedBits")]
    reserved_bits: Option<u8>,
}

fn cluster_key(value: &Value) -> ClusterKey {
    match value {
        Value::String(name) => ClusterKey::Name(name.clone()),
        Value::Number(id) => ClusterKey::Id(id.as_u64().expect("cluster id") as u32),
        other => panic!("invalid cluster key {other:?}"),
    }
}

fn command_key(value: &Value) -> CommandKey {
    match value {
        Value::String(name) => CommandKey::Name(name.clone()),
        Value::Number(id) => CommandKey::Id(id.as_u64().expect("command id") as u32),
        other => panic!("invalid command key {other:?}"),
    }
}

fn assert_id_name(expected: &Value, id: u64, name: &str, case: &str) {
    assert_eq!(expected["ID"].as_u64(), Some(id), "{case}: ID");
    assert_eq!(expected["name"].as_str(), Some(name), "{case}: name");
}

#[test]
fn zcl_matches_upstream_fixtures() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("fixtures/zcl.json")).expect("zcl fixtures");
    let custom = CustomClusters::empty();

    for case in cases {
        match case.op.as_str() {
            "get_cluster" => {
                let cluster =
                    Utils::get_cluster(cluster_key(case.key.as_ref().expect("key")), None, &custom);
                assert_id_name(
                    &case.expected,
                    u64::from(cluster.ID),
                    &cluster.name,
                    &case.id,
                );
            }
            "get_global_command" => {
                let command =
                    Utils::get_global_command(command_key(case.key.as_ref().expect("key")));
                assert_id_name(
                    &case.expected,
                    u64::from(command.ID),
                    &command.name,
                    &case.id,
                );
            }
            "get_foundation_command" => {
                let command = Utils::get_foundation_command(
                    case.key
                        .as_ref()
                        .and_then(Value::as_u64)
                        .expect("foundation command id") as u16,
                );
                assert_id_name(
                    &case.expected,
                    u64::from(command.ID),
                    &command.name,
                    &case.id,
                );
            }
            "get_cluster_command" => {
                let cluster = Utils::get_cluster(
                    ClusterKey::Name(case.cluster.as_ref().expect("cluster").clone()),
                    None,
                    &custom,
                );
                let command = Utils::get_cluster_command(
                    &cluster,
                    command_key(case.key.as_ref().expect("key")),
                );
                assert_id_name(
                    &case.expected,
                    u64::from(command.ID),
                    &command.name,
                    &case.id,
                );
            }
            "frame_to_buffer" => {
                // `Frame::create` mirrors upstream's string keys as `&'static str`;
                // fixtures own their strings, so leak them for the test process.
                let command: &'static str =
                    Box::leak(case.command.clone().expect("command").into_boxed_str());
                let cluster: &'static str =
                    Box::leak(case.cluster.clone().expect("cluster").into_boxed_str());
                let frame = Frame::create(
                    case.frame_type.expect("frameType"),
                    case.direction.expect("direction"),
                    case.disable_default_response
                        .expect("disableDefaultResponse"),
                    case.manufacturer_code,
                    case.transaction_sequence_number
                        .expect("transactionSequenceNumber"),
                    command,
                    cluster,
                    case.payload.clone().unwrap_or(Value::Null),
                    Value::Object(Default::default()),
                    case.reserved_bits.unwrap_or(0),
                );
                let expected = case.expected.as_str().expect("expected hex");
                let actual = frame
                    .to_buffer()
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>();
                assert_eq!(actual, expected, "{}", case.id);
            }
            other => panic!("{}: unknown op '{other}'", case.id),
        }
    }
}
