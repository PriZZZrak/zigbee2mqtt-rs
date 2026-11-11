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

/// The generated catalog must expose every cluster/attribute/command from the
/// pinned upstream `definition/cluster.ts` dump.
#[test]
fn zcl_catalog_covers_all_upstream_clusters() {
    use zigbee_apiarist::zspec::zcl::definition::enums::{BuffaloZclDataType, DataType};

    let root: Value =
        serde_json::from_str(include_str!("../src/zspec/zcl/definition/clusters.json"))
            .expect("cluster catalog");
    let clusters = root.as_object().expect("clusters object");
    assert_eq!(clusters.len(), 129);

    let custom = CustomClusters::empty();
    for (name, raw) in clusters {
        let cluster = Utils::get_cluster(ClusterKey::Name(name.clone()), None, &custom);
        assert_eq!(
            cluster.ID as u64,
            raw["ID"].as_u64().expect("cluster ID"),
            "{name}: ID"
        );
        assert_eq!(cluster.name, *name, "{name}: name");
        assert_eq!(
            cluster.manufacturerCode,
            raw.get("manufacturerCode")
                .and_then(Value::as_u64)
                .map(|code| code as u16),
            "{name}: manufacturerCode"
        );

        let by_id = Utils::get_cluster(
            ClusterKey::Id(raw["ID"].as_u64().expect("cluster ID") as u32),
            None,
            &custom,
        );
        assert_eq!(by_id.name, *name, "{name}: lookup by ID");

        if let Some(attributes) = raw.get("attributes").and_then(Value::as_object) {
            let mut id_counts = std::collections::HashMap::new();
            for attribute in attributes.values() {
                *id_counts
                    .entry(attribute["ID"].as_u64().unwrap_or(0))
                    .or_insert(0usize) += 1;
            }
            for (attribute_name, attribute) in attributes {
                let expected_id = attribute["ID"].as_u64().expect("attribute ID") as u16;
                let by_name = Utils::get_cluster_attribute(&cluster, attribute_name.as_str(), None)
                    .unwrap_or_else(|| panic!("{name}: missing attribute {attribute_name}"));
                assert_eq!(by_name.ID, expected_id, "{name}.{attribute_name}: ID");
                assert_eq!(
                    by_name.r#type,
                    DataType::from_u8(attribute["type"].as_u64().expect("type") as u8)
                        .expect("data type"),
                    "{name}.{attribute_name}: type"
                );
                let by_id = Utils::get_cluster_attribute(&cluster, expected_id, None)
                    .unwrap_or_else(|| panic!("{name}: missing attribute ID {expected_id}"));
                if id_counts.get(&u64::from(expected_id)) == Some(&1) {
                    assert_eq!(
                        by_id.name, *attribute_name,
                        "{name}.{attribute_name}: ID lookup"
                    );
                } else {
                    assert_eq!(
                        by_id.ID, expected_id,
                        "{name}.{attribute_name}: duplicate ID"
                    );
                }
            }
        }

        for (response, map) in [
            ("commands", &cluster.commands),
            ("commandsResponse", &cluster.commandsResponse),
        ] {
            if let Some(commands) = raw.get(response).and_then(Value::as_object) {
                let mut id_counts = std::collections::HashMap::new();
                for command in commands.values() {
                    *id_counts
                        .entry(command["ID"].as_u64().unwrap_or(0))
                        .or_insert(0usize) += 1;
                }
                for (command_name, command) in commands {
                    let expected_id = command["ID"].as_u64().expect("command ID") as u8;
                    let by_name = if response == "commands" {
                        Utils::get_cluster_command(&cluster, command_name.as_str())
                    } else {
                        Utils::get_cluster_command_response(&cluster, command_name.as_str())
                    };
                    assert_eq!(by_name.ID, expected_id, "{name}.{command_name}: ID");
                    assert_eq!(map.get(command_name).map(|c| c.ID), Some(expected_id));
                    let by_id = if response == "commands" {
                        Utils::get_cluster_command(&cluster, expected_id)
                    } else {
                        Utils::get_cluster_command_response(&cluster, expected_id)
                    };
                    if id_counts.get(&u64::from(expected_id)) == Some(&1) {
                        assert_eq!(
                            by_id.name, *command_name,
                            "{name}.{command_name}: ID lookup"
                        );
                    } else {
                        assert_eq!(by_id.ID, expected_id, "{name}.{command_name}: duplicate ID");
                    }
                    let expected_params = command
                        .get("parameters")
                        .and_then(Value::as_array)
                        .map(Vec::len)
                        .unwrap_or(0);
                    assert_eq!(
                        by_name.parameters.len(),
                        expected_params,
                        "{name}.{command_name}: parameters"
                    );
                    for (parameter, expected) in by_name.parameters.iter().zip(
                        command
                            .get("parameters")
                            .and_then(Value::as_array)
                            .into_iter()
                            .flatten(),
                    ) {
                        let type_code = expected["type"].as_u64().expect("parameter type") as u32;
                        if type_code >= 1000 {
                            let expected_buffalo = match type_code {
                                1000 => BuffaloZclDataType::USE_DATA_TYPE,
                                1001 => BuffaloZclDataType::LIST_UINT8,
                                1002 => BuffaloZclDataType::LIST_UINT16,
                                1003 => BuffaloZclDataType::LIST_UINT24,
                                1004 => BuffaloZclDataType::LIST_UINT32,
                                1005 => BuffaloZclDataType::LIST_ZONEINFO,
                                1006 => BuffaloZclDataType::EXTENSION_FIELD_SETS,
                                1007 => BuffaloZclDataType::LIST_THERMO_TRANSITIONS,
                                1008 => BuffaloZclDataType::BUFFER,
                                1009 => BuffaloZclDataType::GPD_FRAME,
                                1010 => BuffaloZclDataType::STRUCTURED_SELECTOR,
                                1011 => BuffaloZclDataType::LIST_TUYA_DATAPOINT_VALUES,
                                1012 => BuffaloZclDataType::LIST_MIBOXER_ZONES,
                                1013 => BuffaloZclDataType::BIG_ENDIAN_UINT24,
                                1014 => BuffaloZclDataType::MI_STRUCT,
                                other => panic!("unknown buffalo type {other}"),
                            };
                            assert_eq!(
                                parameter.buffalo_type,
                                Some(expected_buffalo),
                                "{name}.{command_name}.{}: buffalo type",
                                parameter.name
                            );
                        } else {
                            assert_eq!(
                                parameter.r#type,
                                DataType::from_u8(type_code as u8).expect("data type"),
                                "{name}.{command_name}.{}: type",
                                parameter.name
                            );
                        }
                        assert_eq!(
                            parameter.name,
                            expected["name"].as_str().unwrap_or_default()
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn unknown_cluster_reports_unsupported_status() {
    use zigbee_apiarist::zspec::zcl::zcl_frame::{try_from_buffer, FrameParseError};
    use zigbee_apiarist::zspec::zcl::zcl_header::{Direction, FrameControl, FrameType, Header};

    let header = Header::new(
        FrameControl {
            reserved_bits: 0,
            frame_type: FrameType::SPECIFIC,
            direction: Direction::CLIENT_TO_SERVER,
            disable_default_response: false,
            manufacturer_specific: false,
        },
        None,
        1,
        1,
    );
    let error = try_from_buffer(
        0xffff,
        Some(header),
        &[0x01, 1, 1],
        &CustomClusters::empty(),
    )
    .expect_err("unknown cluster must fail");
    assert_eq!(error, FrameParseError::Status(0xc3));
}
