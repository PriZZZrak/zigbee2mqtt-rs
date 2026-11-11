// Original: .upstream/zigbee-herdsman/test/zcl.test.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).
//
// Ported case-by-case. Buffers from upstream `Buffer.from([...])` are Rust
// `vec![...]`; object literals are `serde_json::json!`.

use std::panic::{catch_unwind, AssertUnwindSafe};

use serde_json::{json, Value};
use zigbee_apiarist::zspec::zcl::buffalo_zcl::BuffaloZcl;
use zigbee_apiarist::zspec::zcl::definition::enums::{
    BuffaloZclDataType, DataType, StructuredIndicatorType,
};
use zigbee_apiarist::zspec::zcl::utils::{
    AttributeKey, Cluster, CommandKey, CustomClusters, Utils, ZclValue,
};
use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
use zigbee_apiarist::zspec::zcl::zcl_header::{Direction, FrameControl, FrameType, Header};

fn fc(
    reserved: u8,
    frame_type: FrameType,
    direction: Direction,
    disable_default_response: bool,
    manufacturer_specific: bool,
) -> FrameControl {
    FrameControl {
        reserved_bits: reserved,
        frame_type,
        direction,
        disable_default_response,
        manufacturer_specific,
    }
}

#[allow(clippy::too_many_arguments)]
fn hdr(
    reserved: u8,
    frame_type: FrameType,
    direction: Direction,
    disable_default_response: bool,
    manufacturer_specific: bool,
    manufacturer_code: Option<u16>,
    transaction_sequence_number: u8,
    command_identifier: u8,
) -> Header {
    Header::new(
        fc(
            reserved,
            frame_type,
            direction,
            disable_default_response,
            manufacturer_specific,
        ),
        manufacturer_code,
        transaction_sequence_number,
        command_identifier,
    )
}

fn parse(cluster: u16, buffer: &[u8]) -> Frame {
    Frame::from_buffer(
        cluster,
        Header::from_buffer(buffer),
        buffer,
        &CustomClusters::empty(),
    )
}

fn parse_custom(cluster: u16, buffer: &[u8], custom: &CustomClusters) -> Frame {
    Frame::from_buffer(cluster, Header::from_buffer(buffer), buffer, custom)
}

fn assert_panics(function: impl FnOnce()) {
    assert!(catch_unwind(AssertUnwindSafe(function)).is_err());
}

fn manu_specific_assa_clusters() -> CustomClusters {
    let mut clusters = CustomClusters::empty();
    clusters.insert(Cluster::new("manuSpecificAssaDoorLock", 0xfc00));
    clusters
}

#[test]
fn get_cluster_by_name() {
    let cluster = Utils::get_cluster("genIdentify", None, &CustomClusters::empty());
    assert_eq!(cluster.ID, 3);
    assert_eq!(cluster.name, "genIdentify");
    assert_eq!(cluster.commands["identifyQuery"].ID, 1);
    assert_eq!(cluster.commands["identifyQuery"].name, "identifyQuery");
    assert_eq!(cluster.commandsResponse["identifyQueryRsp"].ID, 0);
    assert_eq!(
        cluster.commandsResponse["identifyQueryRsp"].name,
        "identifyQueryRsp"
    );
}

#[test]
fn get_cluster_by_id() {
    let cluster1 = Utils::get_cluster(0u16, None, &CustomClusters::empty());
    let cluster2 = Utils::get_cluster("genBasic", None, &CustomClusters::empty());
    assert_eq!(cluster1, cluster2);
}

#[test]
fn get_cluster_attribute_by_id() {
    let cluster = Utils::get_cluster(0u16, None, &CustomClusters::empty());
    let attribute =
        Utils::get_cluster_attribute(&cluster, AttributeKey::Id(1), None).expect("attribute");
    assert_eq!(attribute.ID, 1);
    assert_eq!(attribute.r#type, DataType::UINT8);
    assert_eq!(attribute.name, "appVersion");
    assert_eq!(attribute.default, Some(ZclValue::num(0)));
    assert_eq!(attribute.max, Some(255.0));
}

#[test]
fn cluster_has_attribute() {
    let cluster = Utils::get_cluster(0u16, None, &CustomClusters::empty());
    assert!(
        Utils::get_cluster_attribute(&cluster, AttributeKey::Name("zclVersion".into()), None)
            .is_some()
    );
    assert!(
        Utils::get_cluster_attribute(&cluster, AttributeKey::Name("NOTEXISTING".into()), None)
            .is_none()
    );
    assert!(Utils::get_cluster_attribute(&cluster, AttributeKey::Id(0), None).is_some());
    assert!(Utils::get_cluster_attribute(&cluster, AttributeKey::Id(910293), None).is_none());
}

#[test]
fn get_specific_command_by_name() {
    let cluster = Utils::get_cluster("genIdentify", None, &CustomClusters::empty());
    let command = Utils::get_cluster_command(&cluster, CommandKey::Name("identify".into()));
    assert_eq!(command.ID, 0);
    assert_eq!(command.name, "identify");
}

#[test]
fn get_global_command_by_name() {
    let command = Utils::get_global_command("readRsp");
    assert_eq!(command.ID, 1);
    assert_eq!(command.name, "readRsp");
}

#[test]
fn get_global_command_non_existing() {
    let error = catch_unwind(AssertUnwindSafe(|| {
        Utils::get_global_command("nonexisting")
    }))
    .expect_err("must panic");
    let message = error
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| error.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default();
    assert_eq!(message, "Status 'UNSUP_COMMAND' foundation:nonexisting");
}

#[test]
fn get_cluster_by_name_non_existing() {
    let error = catch_unwind(AssertUnwindSafe(|| {
        Utils::get_cluster("notExisting", None, &CustomClusters::empty());
    }))
    .expect_err("must panic");
    let message = error
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| error.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default();
    assert_eq!(message, "Status 'UNSUPPORTED_CLUSTER' notExisting");
}

#[test]
fn get_cluster_by_id_non_existing() {
    let cluster = Utils::get_cluster(0x190231u32, None, &CustomClusters::empty());
    assert_eq!(cluster.ID, 1638961);
    assert_eq!(cluster.name, "1638961");
    assert!(cluster.attributes.is_empty());
    assert!(cluster.commands.is_empty());
    assert!(cluster.commandsResponse.is_empty());
}

#[test]
fn get_specific_command_by_id() {
    let cluster = Utils::get_cluster("genIdentify", None, &CustomClusters::empty());
    let by_id = Utils::get_cluster_command(&cluster, CommandKey::Id(0));
    let by_name = Utils::get_cluster_command(&cluster, CommandKey::Name("identify".into()));
    assert_eq!(by_id, by_name);
}

#[test]
fn get_specific_command_by_name_server_to_client() {
    let cluster = Utils::get_cluster("genIdentify", None, &CustomClusters::empty());
    let command = Utils::get_cluster_command_response(&cluster, CommandKey::Id(0));
    assert_eq!(command.ID, 0);
    assert_eq!(command.name, "identifyQueryRsp");
}

#[test]
fn get_specific_command_by_name_non_existing() {
    let cluster = Utils::get_cluster("genIdentify", None, &CustomClusters::empty());
    let error = catch_unwind(AssertUnwindSafe(|| {
        Utils::get_cluster_command_response(&cluster, CommandKey::Name("nonexisting".into()));
    }))
    .expect_err("must panic");
    let message = error
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| error.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default();
    assert_eq!(
        message,
        "Status 'UNSUP_COMMAND' response genIdentify:nonexisting"
    );
}

#[test]
fn zcl_frame_from_buffer_parse_payload_with_unknown_frame_type() {
    // Upstream calls `Frame.parsePayload({frameType: 9})`; Rust's typed
    // FrameType makes that state unrepresentable, so the equivalent malformed
    // frame type is rejected when parsing the header.
    assert!(Header::from_buffer(&[0b0000_0010]).is_none());
}

#[test]
fn zcl_frame_from_buffer_report() {
    let buffer = vec![0x18, 0x4a, 0x0a, 0x55, 0x00, 0x39, 0x00, 0x00, 0x00, 0x00];
    let frame = parse(0x000c, &buffer);
    assert_eq!(
        frame.header,
        hdr(
            0,
            FrameType::GLOBAL,
            Direction::SERVER_TO_CLIENT,
            true,
            false,
            None,
            74,
            10
        )
    );
    assert_eq!(
        frame.payload,
        json!([{ "attrData": 0.0, "attrId": 85, "dataType": 57 }])
    );
    assert!(frame.header.is_global());
    assert!(!frame.header.is_specific());
    assert!(frame.is_cluster("genAnalogInput"));
    assert!(frame.is_command("report"));
}

#[test]
fn zcl_frame_from_buffer_gen_groups_get_membership() {
    let buffer = vec![0x11, 0x7c, 0x02, 2, 10, 0, 20, 0];
    let frame = parse(0x0004, &buffer);
    assert_eq!(
        frame.header,
        hdr(
            0,
            FrameType::SPECIFIC,
            Direction::CLIENT_TO_SERVER,
            true,
            false,
            None,
            124,
            2
        )
    );
    assert_eq!(
        frame.payload,
        json!({ "groupcount": 2, "grouplist": [10, 20] })
    );
}

#[test]
fn zcl_frame_from_buffer_gen_groups_get_membership_server_to_client() {
    let buffer = vec![0x19, 0x7c, 0x03, 0, 10, 0];
    let frame = parse(0x0004, &buffer);
    assert_eq!(
        frame.header,
        hdr(
            0,
            FrameType::SPECIFIC,
            Direction::SERVER_TO_CLIENT,
            true,
            false,
            None,
            124,
            3
        )
    );
    assert_eq!(frame.payload, json!({ "groupid": 10, "status": 0 }));
}

#[test]
fn zcl_frame_from_buffer_occupancy_report() {
    let buffer = vec![24, 169, 10, 0, 0, 24, 1];
    let frame = parse(0x0406, &buffer);
    assert_eq!(
        frame.header,
        hdr(
            0,
            FrameType::GLOBAL,
            Direction::SERVER_TO_CLIENT,
            true,
            false,
            None,
            169,
            10
        )
    );
    assert_eq!(
        frame.payload,
        json!([{ "attrId": 0, "dataType": 24, "attrData": 1 }])
    );
}

#[test]
fn zcl_frame_from_buffer_config_report_rsp_short() {
    let buffer = vec![0x08, 0x01, 0x07, 0x00];
    let frame = parse(0x0001, &buffer);
    assert_eq!(
        frame.header,
        hdr(
            0,
            FrameType::GLOBAL,
            Direction::SERVER_TO_CLIENT,
            false,
            false,
            None,
            1,
            7
        )
    );
    assert_eq!(frame.payload, json!([{ "status": 0 }]));
}

#[test]
fn zcl_frame_from_buffer_config_report_rsp_long() {
    let buffer = vec![
        0x08, 0x01, 0x07, 0x02, 0x01, 0x34, 0x12, 0x01, 0x01, 0x35, 0x12,
    ];
    let frame = parse(0x0001, &buffer);
    assert_eq!(
        frame.payload,
        json!([
            { "status": 2, "direction": 1, "attrId": 0x1234 },
            { "status": 1, "direction": 1, "attrId": 0x1235 }
        ])
    );
}

#[test]
fn zcl_frame_from_buffer_config_report_rsp_hvac_thermostat() {
    let buffer = vec![0x18, 0x03, 0x07, 0x00];
    let frame = parse(0x0201, &buffer);
    assert_eq!(frame.payload, json!([{ "status": 0 }]));
    assert_eq!(
        frame.header,
        hdr(
            0,
            FrameType::GLOBAL,
            Direction::SERVER_TO_CLIENT,
            true,
            false,
            None,
            3,
            7
        )
    );
}

#[test]
fn zcl_frame_from_buffer_get_weekly_schedule_rsp_hvac_thermostat() {
    let buffer_heat = vec![
        9, 7, 0, 6, 64, 1, 104, 1, 252, 8, 58, 2, 152, 8, 208, 2, 102, 8, 72, 3, 102, 8, 222, 3,
        252, 8, 100, 5, 52, 8,
    ];
    let frame_heat = parse(0x0201, &buffer_heat);
    assert_eq!(
        frame_heat.payload,
        json!({
            "numoftrans": 6,
            "dayofweek": 64,
            "mode": 1,
            "transitions": [
                {"transitionTime": 360, "heatSetpoint": 2300},
                {"transitionTime": 570, "heatSetpoint": 2200},
                {"transitionTime": 720, "heatSetpoint": 2150},
                {"transitionTime": 840, "heatSetpoint": 2150},
                {"transitionTime": 990, "heatSetpoint": 2300},
                {"transitionTime": 1380, "heatSetpoint": 2100}
            ]
        })
    );

    let buffer_cool = vec![
        9, 7, 0, 6, 64, 2, 104, 1, 252, 8, 58, 2, 152, 8, 208, 2, 102, 8, 72, 3, 102, 8, 222, 3,
        252, 8, 100, 5, 52, 8,
    ];
    let frame_cool = parse(0x0201, &buffer_cool);
    assert_eq!(
        frame_cool.payload,
        json!({
            "numoftrans": 6,
            "dayofweek": 64,
            "mode": 2,
            "transitions": [
                {"transitionTime": 360, "coolSetpoint": 2300},
                {"transitionTime": 570, "coolSetpoint": 2200},
                {"transitionTime": 720, "coolSetpoint": 2150},
                {"transitionTime": 840, "coolSetpoint": 2150},
                {"transitionTime": 990, "coolSetpoint": 2300},
                {"transitionTime": 1380, "coolSetpoint": 2100}
            ]
        })
    );

    let buffer_heat_and_cool = vec![9, 7, 0, 1, 64, 3, 104, 1, 252, 8, 58, 2];
    let frame_heat_and_cool = parse(0x0201, &buffer_heat_and_cool);
    assert_eq!(
        frame_heat_and_cool.payload,
        json!({
            "numoftrans": 1,
            "dayofweek": 64,
            "mode": 3,
            "transitions": [{"transitionTime": 360, "coolSetpoint": 570, "heatSetpoint": 2300}]
        })
    );
}

#[test]
fn zcl_frame_from_buffer_config_report_rsp_failed() {
    let buffer = vec![0x08, 0x01, 0x07, 0x02, 0x01, 0x01, 0x01];
    let frame = parse(0x0001, &buffer);
    assert_eq!(
        frame.header,
        hdr(
            0,
            FrameType::GLOBAL,
            Direction::SERVER_TO_CLIENT,
            false,
            false,
            None,
            1,
            7
        )
    );
    assert_eq!(
        frame.payload,
        json!([{ "status": 2, "direction": 1, "attrId": 257 }])
    );
}

#[test]
fn zcl_frame_from_buffer_default_rsp() {
    let buffer = vec![0x18, 0x04, 0x0b, 0x0c, 0x82];
    let frame = parse(0x0000, &buffer);
    assert_eq!(
        frame.header,
        hdr(
            0,
            FrameType::GLOBAL,
            Direction::SERVER_TO_CLIENT,
            true,
            false,
            None,
            4,
            11
        )
    );
    assert_eq!(frame.payload, json!({ "cmdId": 12, "statusCode": 130 }));
}

#[test]
fn zcl_frame_from_buffer_xiaomi_struct() {
    let cases = vec![
        (
            vec![
                28, 95, 17, 3, 10, 1, 255, 66, 68, 3, 40, 29, 5, 33, 190, 45, 8, 33, 47, 18, 9, 33,
                2, 21, 100, 16, 1, 101, 16, 0, 110, 32, 255, 111, 32, 255, 148, 32, 4, 149, 57,
                184, 30, 21, 62, 150, 57, 211, 249, 17, 69, 151, 57, 0, 48, 104, 59, 152, 57, 0, 0,
                0, 0, 155, 33, 1, 0, 156, 32, 1, 10, 33, 56, 38, 12, 40, 0, 0,
            ],
            json!([
                {
                    "attrId": 65281,
                    "dataType": 66,
                    "attrData": {
                        "3": 29, "5": 11710, "8": 4655, "9": 5378, "10": 9784, "12": 0,
                        "100": 1, "101": 0, "110": 255, "111": 255, "148": 4,
                        "149": 0.14562499523162842, "150": 2335.614013671875,
                        "151": 0.0035429000854492188, "152": 0.0, "155": 1, "156": 1
                    }
                }
            ]),
        ),
        (
            vec![
                28, 95, 17, 3, 10, 5, 0, 66, 21, 108, 117, 109, 105, 46, 115, 101, 110, 115, 111,
                114, 95, 119, 108, 101, 97, 107, 46, 97, 113, 49, 1, 255, 66, 34, 1, 33, 213, 12,
                3, 40, 33, 4, 33, 168, 19, 5, 33, 43, 0, 6, 36, 0, 0, 5, 0, 0, 8, 33, 4, 2, 10, 33,
                0, 0, 100, 16, 0,
            ],
            json!([
                { "attrId": 5, "dataType": 66, "attrData": "lumi.sensor_wleak.aq1" },
                { "attrId": 65281, "dataType": 66, "attrData": {
                    "1": 3285, "3": 33, "4": 5032, "5": 43, "6": 327680, "8": 516, "10": 0, "100": 0
                }}
            ]),
        ),
    ];
    for (buffer, payload) in cases {
        let frame = parse(0x0000, &buffer);
        assert_eq!(
            frame.header,
            hdr(
                0,
                FrameType::GLOBAL,
                Direction::SERVER_TO_CLIENT,
                true,
                true,
                Some(4447),
                3,
                10
            )
        );
        assert_eq!(frame.payload, payload);
    }
}

#[test]
fn zcl_frame_from_buffer_struct() {
    let buffer = vec![
        28, 52, 18, 194, 10, 2, 255, 76, 6, 0, 16, 1, 33, 206, 11, 33, 168, 67, 36, 1, 0, 0, 0, 0,
        33, 48, 2, 32, 86,
    ];
    let frame = parse(0x0000, &buffer);
    assert_eq!(
        frame.header,
        hdr(
            0,
            FrameType::GLOBAL,
            Direction::SERVER_TO_CLIENT,
            true,
            true,
            Some(4660),
            194,
            10
        )
    );
    assert_eq!(
        frame.payload,
        json!([{
            "attrId": 65282,
            "dataType": 76,
            "attrData": [
                {"elmType": 16, "elmVal": 1},
                {"elmType": 33, "elmVal": 3022},
                {"elmType": 33, "elmVal": 17320},
                {"elmType": 36, "elmVal": 1},
                {"elmType": 33, "elmVal": 560},
                {"elmType": 32, "elmVal": 86}
            ]
        }])
    );
}

#[test]
fn zcl_frame_from_buffer_discover_rsp() {
    let buffer = vec![
        24, 23, 13, 0, 32, 0, 32, 33, 0, 32, 49, 0, 48, 51, 0, 32, 53, 0, 24,
    ];
    let frame = parse(0x0001, &buffer);
    assert_eq!(
        frame.header,
        hdr(
            0,
            FrameType::GLOBAL,
            Direction::SERVER_TO_CLIENT,
            true,
            false,
            None,
            23,
            13
        )
    );
    assert_eq!(
        frame.payload,
        json!({
            "discComplete": 0,
            "attrInfos": [
                {"attrId": 32, "dataType": 32},
                {"attrId": 33, "dataType": 32},
                {"attrId": 49, "dataType": 48},
                {"attrId": 51, "dataType": 32},
                {"attrId": 53, "dataType": 24}
            ]
        })
    );
    assert_eq!(frame.cluster.name, "genPowerCfg");
    assert_eq!(frame.command.name, "discoverRsp");
}

#[test]
fn zcl_frame_from_buffer_error_on_malformed() {
    assert_panics(|| {
        let buffer = vec![0x08, 0x01];
        parse(0x0001, &buffer);
    });
}

#[test]
fn zcl_frame_from_buffer_read_rsp_failed() {
    let buffer = vec![8, 1, 1, 1, 0, 2];
    let frame = parse(0x0000, &buffer);
    assert_eq!(
        frame.header,
        hdr(
            0,
            FrameType::GLOBAL,
            Direction::SERVER_TO_CLIENT,
            false,
            false,
            None,
            1,
            1
        )
    );
    assert_eq!(frame.payload, json!([{ "status": 2, "attrId": 1 }]));
}

#[test]
fn zcl_frame_from_buffer_read_rsp_success() {
    let buffer = vec![8, 1, 1, 1, 0, 0, 32, 3];
    let frame = parse(0x0000, &buffer);
    assert_eq!(
        frame.payload,
        json!([{ "status": 0, "attrId": 1, "dataType": 32, "attrData": 3 }])
    );
}

#[test]
fn zcl_frame_from_buffer_gpd_commission() {
    let buffer = vec![
        0x11, 0x00, 0x04, 0x00, 0x00, 0xfe, 0xf4, 0x46, 0x00, 0xf9, 0x00, 0x00, 0x00, 0xe0, 0x1b,
        0x02, 0x81, 0xf2, 0xf1, 0xec, 0x92, 0xab, 0xff, 0x8f, 0x13, 0x63, 0xe1, 0x46, 0xbe, 0xb5,
        0x18, 0xc9, 0x0c, 0xab, 0xa4, 0x46, 0xd4, 0xd5, 0xf9, 0x01, 0x00, 0x00,
    ];
    let frame = parse(0x0021, &buffer);
    assert_eq!(
        frame.header,
        hdr(
            0,
            FrameType::SPECIFIC,
            Direction::CLIENT_TO_SERVER,
            true,
            false,
            None,
            0,
            4
        )
    );
    assert_eq!(
        frame.payload,
        json!({
            "options": 0,
            "srcID": 4650238,
            "frameCounter": 249,
            "commandID": 224,
            "payloadSize": 27,
            "commandFrame": {
                "deviceID": 2,
                "options": 129,
                "extendedOptions": 242,
                "gpdClientClusters": [],
                "gpdServerClusters": [],
                "manufacturerID": 0,
                "modelID": 0,
                "numClientClusters": 0,
                "numServerClusters": 0,
                "securityKey": [0xf1, 0xec, 0x92, 0xab, 0xff, 0x8f, 0x13, 0x63, 0xe1, 0x46, 0xbe, 0xb5, 0x18, 0xc9, 0x0c, 0xab],
                "keyMic": 3587458724u32,
                "outgoingCounter": 505,
                "applicationInfo": 0,
                "numGpdCommands": 0,
                "gpdCommandIdList": [],
                "genericSwitchConfig": 0,
                "currentContactStatus": 0
            }
        })
    );
}

#[test]
fn zcl_frame_from_buffer_gpd_scene_0() {
    let buffer = vec![
        0x11, 0x00, 0x00, 0xa0, 0x14, 0xfe, 0xf4, 0x46, 0x00, 0xe5, 0x04, 0x00, 0x00, 0x10, 0xff,
    ];
    let frame = parse(0x0021, &buffer);
    assert_eq!(
        frame.header,
        hdr(
            0,
            FrameType::SPECIFIC,
            Direction::CLIENT_TO_SERVER,
            true,
            false,
            None,
            0,
            0
        )
    );
    assert_eq!(
        frame.payload,
        json!({
            "srcID": 4650238,
            "commandFrame": {},
            "commandID": 16,
            "frameCounter": 1253,
            "options": 5280,
            "payloadSize": Value::Null
        })
    );
}

#[test]
fn zcl_frame_from_buffer_gpd_with_extra_data() {
    let buffer = vec![
        0x11, 0x00, 0x00, 0xa0, 0x14, 0xfe, 0xf4, 0x46, 0x00, 0xe5, 0x04, 0x00, 0x00, 0x10, 0x01,
        0x01,
    ];
    let frame = parse(0x0021, &buffer);
    assert_eq!(
        frame.payload,
        json!({
            "srcID": 4650238,
            "commandFrame": {"raw": [1]},
            "commandID": 16,
            "frameCounter": 1253,
            "options": 5280,
            "payloadSize": 1
        })
    );
}

#[test]
fn zcl_frame_from_buffer_gpd_pairing() {
    let buffer = vec![
        0x19, 0x17, 0x01, 0x68, 0xe5, 0x00, 0xf8, 0x71, 0x71, 0x01, 0x47, 0x65, 0xa1, 0x1c, 0x00,
        0x4b, 0x12, 0x00, 0x00, 0x00, 0x02, 0x1c, 0x12, 0x00, 0x00, 0x09, 0x3c, 0xed, 0x1d, 0xbf,
        0x25, 0x63, 0xf9, 0x29, 0x5c, 0x0d, 0x3d, 0x9f, 0xc5, 0x76, 0xe1, 0, 0, 0, 0, 0, 0,
    ];
    let frame = parse(0x0021, &buffer);
    assert_eq!(
        frame.header,
        hdr(
            0,
            FrameType::SPECIFIC,
            Direction::SERVER_TO_CLIENT,
            true,
            false,
            None,
            23,
            1
        )
    );
    assert_eq!(
        frame.payload,
        json!({
            "options": 0x00e568,
            "srcID": 0x017171f8,
            "sinkIEEEAddr": "0x00124b001ca16547",
            "sinkNwkAddr": 0,
            "deviceID": 2,
            "frameCounter": 4636,
            "gpdKey": [0x09, 0x3c, 0xed, 0x1d, 0xbf, 0x25, 0x63, 0xf9, 0x29, 0x5c, 0x0d, 0x3d, 0x9f, 0xc5, 0x76, 0xe1]
        })
    );
}

#[test]
fn zcl_frame_from_buffer_read_rsp_alias_type() {
    let buffer = vec![8, 1, 1, 1, 0, 0, 8, 3];
    let frame = parse(0x0000, &buffer);
    assert_eq!(
        frame.payload,
        json!([{ "status": 0, "attrId": 1, "dataType": 8, "attrData": 3 }])
    );
}

#[test]
fn zcl_frame_from_buffer_config_report_rsp_server_to_client() {
    let buffer = vec![8, 1, 6, 1, 1, 0, 10, 10];
    let frame = parse(0x0000, &buffer);
    assert_eq!(
        frame.payload,
        json!([{ "attrId": 1, "direction": 1, "timeout": 2570 }])
    );
}

#[test]
fn zcl_frame_from_buffer_config_report_rsp_client_to_server_analog() {
    let buffer = vec![8, 1, 6, 0, 0, 1, 32, 1, 0, 10, 0, 20];
    let frame = parse(0x0000, &buffer);
    assert_eq!(
        frame.payload,
        json!([{
            "attrId": 256, "dataType": 32, "direction": 0,
            "maxRepIntval": 10, "minRepIntval": 1, "repChange": 20
        }])
    );

    let buffer = vec![8, 1, 6, 0, 0, 1, 8, 1, 0, 10, 0];
    let frame = parse(0x0000, &buffer);
    assert_eq!(
        frame.payload,
        json!([{
            "attrId": 256, "dataType": 8, "direction": 0,
            "maxRepIntval": 10, "minRepIntval": 1
        }])
    );
}

#[test]
fn zcl_frame_from_buffer_read_rsp_long() {
    let buffer = vec![
        24, 7, 1, 5, 0, 0, 66, 30, 84, 82, 65, 68, 70, 82, 73, 32, 98, 117, 108, 98, 32, 69, 50,
        55, 32, 87, 83, 32, 111, 112, 97, 108, 32, 57, 56, 48, 108, 109, 6, 0, 0, 66, 8, 50, 48,
        49, 55, 48, 51, 51, 49, 7, 0, 0, 48, 1, 10, 0, 0, 65, 15, 76, 69, 68, 49, 53, 52, 53, 71,
        49, 50, 69, 50, 55, 69, 85,
    ];
    let frame = parse(0x0000, &buffer);
    assert_eq!(
        frame.payload,
        json!([
            {"attrId": 5, "status": 0, "dataType": 66, "attrData": "TRADFRI bulb E27 WS opal 980lm"},
            {"attrId": 6, "status": 0, "dataType": 66, "attrData": "20170331"},
            {"attrId": 7, "status": 0, "dataType": 48, "attrData": 1},
            {"attrId": 10, "status": 0, "dataType": 65, "attrData": [76, 69, 68, 49, 53, 52, 53, 71, 49, 50, 69, 50, 55, 69, 85]}
        ])
    );
}

#[test]
fn zcl_frame_with_assa_manufacturer_specific_cluster_create() {
    let payload = json!([{"attrId": 0x0012, "status": 0, "attrData": 1, "dataType": 32}]);
    let clusters = manu_specific_assa_clusters();
    let frame = Frame::create_with_custom_clusters(
        FrameType::GLOBAL as u8,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        Some(0x101d),
        8,
        "readRsp",
        0xfc00,
        payload,
        &clusters,
        0,
    );
    assert_eq!(frame.cluster.name, "manuSpecificAssaDoorLock");
}

#[test]
fn zcl_frame_with_assa_manufacturer_specific_cluster_create_with_non_assamanufcode() {
    let payload = json!([{"attrId": 0x0012, "status": 0, "attrData": 1, "dataType": 32}]);
    let clusters = manu_specific_assa_clusters();
    let frame = Frame::create_with_custom_clusters(
        FrameType::GLOBAL as u8,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        Some(0x10f3),
        8,
        "readRsp",
        0xfc00,
        payload,
        &clusters,
        0,
    );
    assert_eq!(frame.cluster.name, "manuSpecificAssaDoorLock");
}

#[test]
fn zcl_frame_with_assa_manufacturer_specific_cluster_from_buffer() {
    let buffer = vec![0x04, 0xf2, 0x10, 0x08, 0x01, 0x00, 0x00, 0x00, 0x20, 0x01];
    let clusters = manu_specific_assa_clusters();
    let frame = parse_custom(0xfc00, &buffer, &clusters);
    assert_eq!(frame.cluster.name, "manuSpecificAssaDoorLock");
}

#[test]
fn zcl_frame_to_buffer_with_reserved_bits() {
    let expected = vec![224, 8, 12, 0, 0, 240];
    let payload = json!({"startAttrId": 0, "maxAttrIds": 240});
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        None,
        8,
        "discover",
        0,
        payload,
        json!({}),
        7,
    );
    assert_eq!(frame.to_buffer(), expected);
}

#[test]
fn zcl_frame_from_buffer_with_reserved_bits() {
    let buffer = vec![224, 8, 12, 0, 0, 240];
    let frame = parse(0, &buffer);
    assert_eq!(
        frame.header,
        hdr(
            7,
            FrameType::GLOBAL,
            Direction::CLIENT_TO_SERVER,
            false,
            false,
            None,
            8,
            12
        )
    );
    assert_eq!(frame.payload, json!({"startAttrId": 0, "maxAttrIds": 240}));
}

#[test]
fn zcl_frame_to_buffer_discover() {
    let expected = vec![0, 8, 12, 0, 0, 240];
    let payload = json!({"startAttrId": 0, "maxAttrIds": 240});
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        None,
        8,
        "discover",
        0,
        payload,
        json!({}),
        0,
    );
    assert_eq!(frame.to_buffer(), expected);
}

#[test]
fn zcl_frame_to_buffer_gen_scenes_add() {
    let payload = json!({
        "groupid": 0,
        "sceneid": 3,
        "scenename": "",
        "transtime": 1,
        "extensionfieldsets": [
            {"clstId": 6, "len": 1, "extField": [1]},
            {"clstId": 768, "len": 4, "extField": [31260, 27110]},
            {"clstId": 8, "len": 1, "extField": [254]}
        ]
    });
    let frame = Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        None,
        1,
        "add",
        "genScenes",
        payload,
        json!({}),
        0,
    );
    assert_eq!(
        frame.to_buffer(),
        vec![1, 1, 0, 0, 0, 3, 1, 0, 0, 6, 0, 1, 1, 0, 3, 4, 28, 122, 230, 105, 8, 0, 1, 254]
    );
}

#[test]
fn zcl_frame_to_buffer_query_next_image_response_with_non_zero_status() {
    let expected = vec![9, 8, 2, 1];
    let payload = json!({"status": 1});
    let frame = Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::SERVER_TO_CLIENT as u8,
        false,
        None,
        8,
        "queryNextImageResponse",
        25,
        payload,
        json!({}),
        0,
    );
    assert_eq!(frame.to_buffer(), expected);
}

#[test]
fn zcl_frame_to_buffer_query_next_image_response_with_zero_status() {
    let expected = vec![9, 8, 2, 0, 1, 0, 3, 0, 5, 0, 0, 0, 6, 0, 0, 0];
    let payload = json!({"status": 0, "manufacturerCode": 1, "imageType": 3, "fileVersion": 5, "imageSize": 6});
    let frame = Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::SERVER_TO_CLIENT as u8,
        false,
        None,
        8,
        "queryNextImageResponse",
        25,
        payload,
        json!({}),
        0,
    );
    assert_eq!(frame.to_buffer(), expected);
}

#[test]
fn zcl_frame_from_buffer_gen_ota_query_next_image_request_keeps_ffff_manufacturer_code() {
    let payload =
        json!({"fieldControl": 0, "manufacturerCode": 0xffff, "imageType": 0, "fileVersion": 260});
    let frame = Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        None,
        8,
        "queryNextImageRequest",
        25,
        payload.clone(),
        json!({}),
        0,
    );
    let parsed = parse(25, &frame.to_buffer());
    assert_eq!(parsed.payload, payload);
}

#[test]
fn zcl_frame_from_buffer_gen_ota_upgrade_end_request_keeps_ffff_manufacturer_code() {
    let payload =
        json!({"status": 0, "manufacturerCode": 0xffff, "imageType": 0, "fileVersion": 260});
    let frame = Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        None,
        8,
        "upgradeEndRequest",
        25,
        payload.clone(),
        json!({}),
        0,
    );
    let parsed = parse(25, &frame.to_buffer());
    assert_eq!(parsed.payload, payload);
}

#[test]
fn zcl_frame_to_buffer_query_next_image_response_with_zero_status_and_missing_parameters() {
    let payload = json!({"status": 0});
    let frame = Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::SERVER_TO_CLIENT as u8,
        false,
        None,
        8,
        "queryNextImageResponse",
        25,
        payload,
        json!({}),
        0,
    );
    let error = catch_unwind(AssertUnwindSafe(|| frame.to_buffer())).expect_err("must panic");
    let message = error
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| error.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default();
    assert_eq!(message, "Parameter 'manufacturerCode' is missing");
}

#[test]
fn zcl_frame_to_buffer_read_rsp_utc() {
    let expected = vec![24, 74, 1, 0, 0, 0, 226, 234, 83, 218, 36];
    let payload = json!([{"attrId": 0, "status": 0, "attrData": 618288106, "dataType": 226}]);
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        true,
        None,
        74,
        "readRsp",
        0,
        payload,
        json!({}),
        0,
    );
    assert_eq!(frame.to_buffer(), expected);
}

#[test]
fn zcl_frame_to_buffer_write_livolo_malformed() {
    let expected_on = vec![
        0x1c, 0xd2, 0x1a, 0xe9, 0x02, 0x01, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];
    let payload_on = json!([{"attrId": 1, "attrData": [1, 0, 0, 0, 0, 0, 0, 0], "dataType": 1}]);
    let frame_on = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        true,
        Some(0x1ad2),
        233,
        "write",
        0,
        payload_on,
        json!({}),
        0,
    );
    assert_eq!(frame_on.to_buffer(), expected_on);

    let expected_off = vec![
        0x1c, 0xd2, 0x1a, 0xe9, 0x02, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00,
    ];
    let payload_off = json!([{"attrId": 1, "attrData": [0, 0, 0, 0, 0, 0, 0, 0], "dataType": 1}]);
    let frame_off = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        true,
        Some(0x1ad2),
        233,
        "write",
        0,
        payload_off,
        json!({}),
        0,
    );
    assert_eq!(frame_off.to_buffer(), expected_off);
}

#[test]
fn zcl_frame_write_request_with_string_as_bytes_array() {
    let payload = json!([{"attrId": 0x0401, "attrData": [0x07, 0x00, 0x02, 0x01, 0x00, 0x00, 0x00, 0x14], "dataType": 0x42}]);
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::CLIENT_TO_SERVER as u8,
        true,
        Some(0x115f),
        15,
        "write",
        0,
        payload,
        json!({}),
        0,
    );
    assert_eq!(
        frame.to_buffer(),
        vec![
            0x14, 0x5f, 0x11, 0x0f, 0x02, 0x01, 0x04, 0x42, 0x07, 0x00, 0x02, 0x01, 0x00, 0x00,
            0x00, 0x14
        ]
    );
}

#[test]
fn zcl_frame_write_rsp() {
    let payload = json!([{"status": 0x11, "attrId": 0x22}]);
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::CLIENT_TO_SERVER as u8,
        true,
        Some(0x115f),
        15,
        "writeRsp",
        0,
        payload,
        json!({}),
        0,
    );
    assert_eq!(
        frame.to_buffer(),
        vec![0x14, 0x5f, 0x11, 0x0f, 0x04, 0x11, 0x22, 0x00]
    );
}

#[test]
fn zcl_frame_to_buffer_read_rsp_success() {
    let payload = json!([{"status": 0, "attrId": 1, "dataType": 32, "attrData": 3}]);
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        false,
        None,
        1,
        1u8,
        0,
        payload,
        json!({}),
        0,
    );
    assert_eq!(frame.to_buffer(), vec![8, 1, 1, 1, 0, 0, 32, 3]);
}

#[test]
fn zcl_frame_to_buffer_default_rsp_success() {
    let payload = json!({"cmdId": 12, "statusCode": 130});
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        true,
        None,
        4,
        11u8,
        0,
        payload,
        json!({}),
        0,
    );
    assert_eq!(frame.to_buffer(), vec![0x18, 0x04, 0x0b, 0x0c, 0x82]);
}

#[test]
fn zcl_frame_to_buffer_read_structured_single_element() {
    let payload = json!([{"attrId": 0x0001, "selector": {"indexes": [2]}}]);
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        true,
        None,
        2,
        0x0eu8,
        "genBasic",
        payload,
        json!({}),
        0,
    );
    assert_eq!(
        frame.to_buffer(),
        vec![0x18, 0x02, 0x0e, 0x01, 0x00, 0x01, 0x02, 0x00]
    );
}

#[test]
fn zcl_frame_to_buffer_read_structured_multiple_elements() {
    let payload = json!([
        {"attrId": 0x0002, "selector": {"indexes": [3, 4]}},
        {"attrId": 0x0005, "selector": {"indexes": [6, 7, 8]}}
    ]);
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        true,
        None,
        2,
        0x0eu8,
        "genBasic",
        payload,
        json!({}),
        0,
    );
    assert_eq!(
        frame.to_buffer(),
        vec![
            0x18, 0x02, 0x0e, 0x02, 0x00, 0x02, 0x03, 0x00, 0x04, 0x00, 0x05, 0x00, 0x03, 0x06,
            0x00, 0x07, 0x00, 0x08, 0x00
        ]
    );
}

#[test]
fn zcl_frame_to_buffer_read_structured_whole_attribute() {
    let payload = json!([{"attrId": 0x0009, "selector": {}}]);
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        true,
        None,
        2,
        0x0eu8,
        "genBasic",
        payload,
        json!({}),
        0,
    );
    assert_eq!(frame.to_buffer(), vec![0x18, 0x02, 0x0e, 0x09, 0x00, 0x00]);
}

#[test]
fn zcl_frame_from_buffer_read_structured_single_elements() {
    let buffer = vec![0x18, 0x02, 0x0e, 0x01, 0x00, 0x01, 0x02, 0x00];
    let frame = parse(0x0000, &buffer);
    assert_eq!(
        frame.header,
        hdr(
            0,
            FrameType::GLOBAL,
            Direction::SERVER_TO_CLIENT,
            true,
            false,
            None,
            2,
            0x0e
        )
    );
    assert_eq!(
        frame.payload,
        json!([{"attrId": 0x0001, "selector": {"indexes": [2]}}])
    );
}

#[test]
fn zcl_frame_from_buffer_read_structured_multiple_elements() {
    let buffer = vec![
        0x18, 0x02, 0x0e, 0x02, 0x00, 0x02, 0x03, 0x00, 0x04, 0x00, 0x05, 0x00, 0x03, 0x06, 0x00,
        0x07, 0x00, 0x08, 0x00,
    ];
    let frame = parse(0x0000, &buffer);
    assert_eq!(
        frame.payload,
        json!([
            {"attrId": 0x0002, "selector": {"indexes": [3, 4]}},
            {"attrId": 0x0005, "selector": {"indexes": [6, 7, 8]}}
        ])
    );
}

#[test]
fn zcl_frame_from_buffer_read_structured_whole_attribute() {
    let buffer = vec![0x18, 0x02, 0x0e, 0x09, 0x00, 0x00];
    let frame = parse(0x0000, &buffer);
    assert_eq!(
        frame.payload,
        json!([{"attrId": 0x0009, "selector": {"indicatorType": 0}}])
    );
}

#[test]
fn zcl_frame_to_buffer_write_structured_single_element() {
    let payload =
        json!([{"attrId": 0x0001, "selector": {"indexes": [2]}, "dataType": 32, "elementData": 3}]);
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::CLIENT_TO_SERVER as u8,
        true,
        None,
        2,
        0x0fu8,
        "genBasic",
        payload,
        json!({}),
        0,
    );
    assert_eq!(
        frame.to_buffer(),
        vec![0x10, 0x02, 0x0f, 0x01, 0x00, 0x01, 0x02, 0x00, 0x20, 0x03]
    );
}

#[test]
fn zcl_frame_to_buffer_write_structured_multiple_elements() {
    let payload = json!([
        {"attrId": 0x0002, "selector": {"indexes": [3, 4]}, "dataType": 66, "elementData": "foo"},
        {"attrId": 0x0005, "selector": {"indexes": [6, 7, 8]}, "dataType": 66, "elementData": "bar"}
    ]);
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::CLIENT_TO_SERVER as u8,
        true,
        None,
        2,
        0x0fu8,
        "genBasic",
        payload,
        json!({}),
        0,
    );
    assert_eq!(
        frame.to_buffer(),
        vec![
            0x10, 0x02, 0x0f, 0x02, 0x00, 0x02, 0x03, 0x00, 0x04, 0x00, 0x42, 0x03, 0x66, 0x6f,
            0x6f, 0x05, 0x00, 0x03, 0x06, 0x00, 0x07, 0x00, 0x08, 0x00, 0x42, 0x03, 0x62, 0x61,
            0x72
        ]
    );
}

#[test]
fn zcl_frame_to_buffer_write_structured_whole_attribute() {
    let payload = json!([{
        "attrId": 0x0009,
        "selector": {},
        "dataType": 72,
        "elementData": {"elementType": 32, "elements": [10, 11, 12]}
    }]);
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::CLIENT_TO_SERVER as u8,
        true,
        None,
        2,
        0x0fu8,
        "genBasic",
        payload,
        json!({}),
        0,
    );
    assert_eq!(
        frame.to_buffer(),
        vec![0x10, 0x02, 0x0f, 0x09, 0x00, 0x00, 0x48, 0x20, 0x03, 0x00, 0x0a, 0x0b, 0x0c]
    );
}

#[test]
fn zcl_frame_to_buffer_write_structured_add_element_into_set_bag() {
    let payload = json!([{
        "attrId": 0x000d,
        "selector": {"indicatorType": 16},
        "dataType": 66,
        "elementData": "foo"
    }]);
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::CLIENT_TO_SERVER as u8,
        true,
        None,
        2,
        0x0fu8,
        "genBasic",
        payload,
        json!({}),
        0,
    );
    assert_eq!(
        frame.to_buffer(),
        vec![0x10, 0x02, 0x0f, 0x0d, 0x00, 0x10, 0x42, 0x03, 0x66, 0x6f, 0x6f]
    );
}

#[test]
fn zcl_frame_to_buffer_write_structured_remove_element_from_set_bag() {
    let payload = json!([{
        "attrId": 0x000e,
        "selector": {"indicatorType": 32},
        "dataType": 66,
        "elementData": "bar"
    }]);
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::CLIENT_TO_SERVER as u8,
        true,
        None,
        2,
        0x0fu8,
        "genBasic",
        payload,
        json!({}),
        0,
    );
    assert_eq!(
        frame.to_buffer(),
        vec![0x10, 0x02, 0x0f, 0x0e, 0x00, 0x20, 0x42, 0x03, 0x62, 0x61, 0x72]
    );
}

#[test]
fn zcl_frame_to_buffer_write_structured_livolo_malformed() {
    let payload = json!([{
        "attrId": 0x0000,
        "selector": Value::Null,
        "elementData": [0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    }]);
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        true,
        Some(0x1ad2),
        0xe9,
        0x0fu8,
        "genBasic",
        payload,
        json!({}),
        3,
    );
    assert_eq!(
        frame.to_buffer(),
        vec![
            0x7c, 0xd2, 0x1a, 0xe9, 0x0f, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00
        ]
    );
}

#[test]
fn zcl_frame_to_buffer_write_structured_rsp_success() {
    let payload = json!([{"status": 0}]);
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        false,
        None,
        1,
        0x10u8,
        "genBasic",
        payload,
        json!({}),
        0,
    );
    assert_eq!(frame.to_buffer(), vec![8, 1, 0x10, 0]);
}

#[test]
fn zcl_frame_to_buffer_write_structured_rsp_failed_selector_unknown_or_none_for_type() {
    let payload = json!([{"status": 147, "attrId": 1, "selector": {"indicatorType": 0}}]);
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        false,
        None,
        1,
        0x10u8,
        "genBasic",
        payload,
        json!({}),
        0,
    );
    assert_eq!(frame.to_buffer(), vec![8, 1, 0x10, 147, 1, 0, 0]);
}

#[test]
fn zcl_frame_to_buffer_write_structured_rsp_failed_with_selector() {
    let payload = json!([{"status": 147, "attrId": 1, "selector": {"indexes": [254]}}]);
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        false,
        None,
        1,
        0x10u8,
        "genBasic",
        payload,
        json!({}),
        0,
    );
    assert_eq!(frame.to_buffer(), vec![8, 1, 0x10, 147, 1, 0, 1, 254, 0]);
}

#[test]
fn zcl_frame_to_buffer_write_structured_rsp_failed_mixed_selectors() {
    let payload = json!([
        {"status": 147, "attrId": 16, "selector": {"indicatorType": 0}},
        {"status": 149, "attrId": 14, "selector": {"indexes": [254, 32]}}
    ]);
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        false,
        None,
        1,
        0x10u8,
        "genBasic",
        payload,
        json!({}),
        0,
    );
    assert_eq!(
        frame.to_buffer(),
        vec![8, 1, 0x10, 147, 16, 0, 0, 149, 14, 0, 2, 254, 0, 32, 0]
    );
}

#[test]
fn zcl_frame_from_buffer_write_structured_rsp_success() {
    let buffer = vec![8, 1, 0x10, 0];
    let frame = parse(0x0000, &buffer);
    assert_eq!(
        frame.header,
        hdr(
            0,
            FrameType::GLOBAL,
            Direction::SERVER_TO_CLIENT,
            false,
            false,
            None,
            1,
            0x10
        )
    );
    assert_eq!(frame.payload, json!([{"status": 0}]));
}

#[test]
fn zcl_frame_from_buffer_write_structured_rsp_failed_selector_unknown_or_none_for_type() {
    let buffer = vec![8, 1, 0x10, 147, 1, 0, 0];
    let frame = parse(0x0000, &buffer);
    assert_eq!(
        frame.payload,
        json!([{"status": 147, "attrId": 1, "selector": {"indicatorType": 0}}])
    );
}

#[test]
fn zcl_frame_from_buffer_write_structured_rsp_failed_with_selector() {
    let buffer = vec![8, 1, 0x10, 147, 1, 0, 1, 254, 0];
    let frame = parse(0x0000, &buffer);
    assert_eq!(
        frame.payload,
        json!([{"status": 147, "attrId": 1, "selector": {"indexes": [254]}}])
    );
}

#[test]
fn zcl_frame_from_buffer_write_structured_rsp_failed_mixed_selectors() {
    let buffer = vec![8, 1, 0x10, 147, 16, 0, 0, 149, 14, 0, 2, 254, 0, 32, 0];
    let frame = parse(0x0000, &buffer);
    assert_eq!(
        frame.payload,
        json!([
            {"status": 147, "attrId": 16, "selector": {"indicatorType": 0}},
            {"status": 149, "attrId": 14, "selector": {"indexes": [254, 32]}}
        ])
    );
}

#[test]
fn zcl_frame_from_buffer_ss_ias_ace_arm_command() {
    let buffer = vec![1, 87, 0, 0, 6, 49, 50, 51, 52, 53, 54, 0];
    let frame = parse(0x0501, &buffer);
    assert_eq!(
        frame.header,
        hdr(
            0,
            FrameType::SPECIFIC,
            Direction::CLIENT_TO_SERVER,
            false,
            false,
            None,
            87,
            0
        )
    );
    assert_eq!(
        frame.payload,
        json!({"armmode": 0, "code": "123456", "zoneid": 0})
    );
    assert!(!frame.header.is_global());
    assert!(frame.header.is_specific());
    assert!(frame.is_cluster("ssIasAce"));
    assert!(frame.is_command("arm"));
}

#[test]
fn zcl_frame_to_buffer_discover_rsp() {
    let payload = json!({
        "discComplete": 0,
        "attrInfos": [
            {"attrId": 32, "dataType": 32},
            {"attrId": 33, "dataType": 32},
            {"attrId": 49, "dataType": 48},
            {"attrId": 51, "dataType": 32},
            {"attrId": 53, "dataType": 24}
        ]
    });
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        true,
        None,
        23,
        13u8,
        0,
        payload,
        json!({}),
        0,
    );
    assert_eq!(
        frame.to_buffer(),
        vec![24, 23, 13, 0, 32, 0, 32, 33, 0, 32, 49, 0, 48, 51, 0, 32, 53, 0, 24]
    );
}

#[test]
fn zcl_frame_to_buffer_read_rsp_failed() {
    let payload = json!([{"status": 2, "attrId": 1}]);
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        false,
        None,
        1,
        1u8,
        0,
        payload,
        json!({}),
        0,
    );
    assert_eq!(frame.to_buffer(), vec![8, 1, 1, 1, 0, 2]);
}

#[test]
fn zcl_frame_to_buffer_off_with_effect() {
    let payload = json!({"effectid": 1, "effectvariant": 0});
    let frame = Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        None,
        1,
        64u8,
        6,
        payload,
        json!({}),
        0,
    );
    assert_eq!(frame.to_buffer(), vec![0x01, 1, 64, 1, 0]);

    let frame = Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::SERVER_TO_CLIENT as u8,
        false,
        None,
        9,
        "restartDeviceRsp",
        21,
        json!({"status": 1}),
        json!({}),
        0,
    );
    assert_eq!(frame.to_buffer(), vec![9, 9, 0, 1]);
}

#[test]
fn zcl_frame_to_buffer_invalid_frametype() {
    let error = catch_unwind(AssertUnwindSafe(|| {
        Frame::create(
            3,
            Direction::CLIENT_TO_SERVER as u8,
            false,
            None,
            1,
            64u8,
            6,
            json!({}),
            json!({}),
            0,
        )
        .to_buffer();
    }))
    .expect_err("must panic");
    let message = error
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| error.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default();
    assert_eq!(message, "Frametype '3' not valid");
}

#[test]
fn buffalo_zcl_read_array() {
    let buffer = vec![32, 3, 0, 1, 2, 3];
    let mut buffalo = BuffaloZcl::new(buffer);
    let value = buffalo.read(DataType::ARRAY as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 6);
    assert_eq!(value, json!([1, 2, 3]));
}

#[test]
fn buffalo_zcl_read_struct() {
    let buffer = vec![0, 2, 0, 32, 8, 33, 4, 0];
    let mut buffalo = BuffaloZcl::new(buffer);
    buffalo.set_position(1);
    let value = buffalo.read(DataType::STRUCT as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 8);
    assert_eq!(
        value,
        json!([{"elmType": 32, "elmVal": 8}, {"elmType": 33, "elmVal": 4}])
    );
}

#[test]
fn buffalo_zcl_read_long_char_str() {
    let buffer = vec![5, 0, 0x68, 0x65, 0x6c, 0x6c, 0x6f];
    let mut buffalo = BuffaloZcl::new(buffer);
    let value = buffalo.read(DataType::LONG_CHAR_STR as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 7);
    assert_eq!(value, json!("hello"));
}

#[test]
fn buffalo_zcl_read_long_octet_str() {
    let buffer = vec![5, 0, 0x68, 0x65, 0x6c, 0x6c, 0x6f];
    let mut buffalo = BuffaloZcl::new(buffer);
    let value = buffalo.read(DataType::LONG_OCTET_STR as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 7);
    assert_eq!(value, json!([0x68, 0x65, 0x6c, 0x6c, 0x6f]));
}

#[test]
fn buffalo_zcl_read_extension_field_sets() {
    let buffer = vec![0, 3, 8, 1, 0, 2, 0, 3, 0, 4, 5];
    let mut buffalo = BuffaloZcl::new(buffer);
    let value = buffalo.read(BuffaloZclDataType::EXTENSION_FIELD_SETS as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 11);
    assert_eq!(
        value,
        json!([{"clstId": 768, "len": 8, "extField": [1, 2, 3, 4, 5]}])
    );
}

#[test]
fn buffalo_zcl_read_list_zoneinfo() {
    let buffer = vec![1, 5, 0, 2, 6, 0];
    let mut buffalo = BuffaloZcl::new(buffer);
    let value = buffalo.read(
        BuffaloZclDataType::LIST_ZONEINFO as u32,
        &json!({"length": 2}),
    );
    assert_eq!(buffalo.get_position(), 6);
    assert_eq!(
        value,
        json!([{"zoneID": 1, "zoneStatus": 5}, {"zoneID": 2, "zoneStatus": 6}])
    );
}

#[test]
fn buffalo_zcl_read_uint40_data40_bitmap40() {
    for data_type in [DataType::UINT40, DataType::DATA40, DataType::BITMAP40] {
        let mut buffalo = BuffaloZcl::new(vec![1, 5, 4, 5, 6, 7]);
        let value = buffalo.read(data_type as u32, &json!({}));
        assert_eq!(buffalo.get_position(), 5);
        assert_eq!(value, json!(25853953281u64));
    }
}

#[test]
fn buffalo_zcl_read_uint48_data48_bitmap48() {
    for data_type in [DataType::UINT48, DataType::DATA48, DataType::BITMAP48] {
        let mut buffalo = BuffaloZcl::new(vec![1, 5, 4, 5, 6, 7]);
        let value = buffalo.read(data_type as u32, &json!({}));
        assert_eq!(buffalo.get_position(), 6);
        assert_eq!(value, json!(7722435347713u64));
    }
}

#[test]
fn buffalo_zcl_read_uint56_data56_bitmap56() {
    for data_type in [DataType::UINT56, DataType::DATA56, DataType::BITMAP56] {
        let mut buffalo = BuffaloZcl::new(vec![1, 5, 4, 5, 6, 7, 7]);
        let value = buffalo.read(data_type as u32, &json!({}));
        assert_eq!(buffalo.get_position(), 7);
        assert_eq!(value, json!(1978047272322305u64));
    }
}

#[test]
fn buffalo_zcl_read_uint64_data64_bitmap64() {
    for data_type in [DataType::UINT64, DataType::DATA64, DataType::BITMAP64] {
        let mut buffalo = BuffaloZcl::new(vec![1, 5, 4, 5, 6, 7, 7, 9]);
        let value = buffalo.read(data_type as u32, &json!({}));
        assert_eq!(buffalo.get_position(), 8);
        assert_eq!(value, json!(0x0907070605040501u64));
    }
}

#[test]
fn buffalo_zcl_read_int40_int56_int64() {
    let mut buffalo = BuffaloZcl::new(vec![254, 255, 255, 255, 127]);
    assert_eq!(
        buffalo.read(DataType::INT40 as u32, &json!({})),
        json!(549755813886u64)
    );
    assert_eq!(buffalo.get_position(), 5);

    let mut buffalo = BuffaloZcl::new(vec![1, 5, 4, 5, 6, 7, 7]);
    assert_eq!(
        buffalo.read(DataType::INT56 as u32, &json!({})),
        json!(1978047272322305u64)
    );
    assert_eq!(buffalo.get_position(), 7);

    let mut buffalo = BuffaloZcl::new(vec![1, 5, 4, 5, 6, 7, 7, 9]);
    assert_eq!(
        buffalo.read(DataType::INT64 as u32, &json!({})),
        json!(650496393613673729u64)
    );
    assert_eq!(buffalo.get_position(), 8);
}

#[test]
fn buffalo_zcl_read_list_tuya_data_point_values() {
    let cases = vec![
        (vec![], json!([])),
        (
            vec![1, 4, 0, 1, 1],
            json!([{"dp": 1, "datatype": 4, "data": [1]}]),
        ),
        (
            vec![1, 4, 0, 1, 1, 4, 2, 0, 4, 0, 0, 0, 90],
            json!([
                {"dp": 1, "datatype": 4, "data": [1]},
                {"dp": 4, "datatype": 2, "data": [0, 0, 0, 90]}
            ]),
        ),
        (
            vec![1, 4, 0, 1, 1, 4],
            json!([{"dp": 1, "datatype": 4, "data": [1]}]),
        ),
        (
            vec![1, 4, 0, 1, 1, 4, 2, 0, 4, 0, 0, 0],
            json!([
                {"dp": 1, "datatype": 4, "data": [1]},
                {"dp": 4, "datatype": 2, "data": [0, 0, 0]}
            ]),
        ),
    ];
    for (buffer, expected) in cases {
        let mut buffalo = BuffaloZcl::new(buffer);
        let value = buffalo.read(
            BuffaloZclDataType::LIST_TUYA_DATAPOINT_VALUES as u32,
            &json!({}),
        );
        assert_eq!(value, expected);
        assert!(
            buffalo.get_position() >= buffalo.buffer.len(),
            "position {} len {}",
            buffalo.get_position(),
            buffalo.buffer.len()
        );
    }
}

#[test]
fn buffalo_zcl_write_char_str() {
    let mut buffalo = BuffaloZcl::new(vec![0; 7]);
    buffalo.write(DataType::CHAR_STR as u32, &json!("hello"), &json!({}));
    assert_eq!(buffalo.get_position(), 6);
    assert_eq!(buffalo.get_written(), vec![5, 0x68, 0x65, 0x6c, 0x6c, 0x6f]);
}

#[test]
fn buffalo_zcl_write_long_char_str() {
    let mut buffalo = BuffaloZcl::new(vec![0; 7]);
    buffalo.write(DataType::LONG_CHAR_STR as u32, &json!("hello"), &json!({}));
    assert_eq!(buffalo.get_position(), 7);
    assert_eq!(
        buffalo.get_written(),
        vec![5, 0, 0x68, 0x65, 0x6c, 0x6c, 0x6f]
    );
}

#[test]
fn buffalo_zcl_write_octet_str() {
    let mut buffalo = BuffaloZcl::new(vec![0; 5]);
    buffalo.write(DataType::OCTET_STR as u32, &json!([1, 2, 4]), &json!({}));
    assert_eq!(buffalo.get_position(), 4);
    assert_eq!(buffalo.get_written(), vec![3, 1, 2, 4]);
}

#[test]
fn buffalo_zcl_write_long_octet_str() {
    let mut buffalo = BuffaloZcl::new(vec![0; 7]);
    buffalo.write(
        DataType::LONG_OCTET_STR as u32,
        &json!([1, 2, 3, 4, 5]),
        &json!({}),
    );
    assert_eq!(buffalo.get_position(), 7);
    assert_eq!(buffalo.get_written(), vec![5, 0, 1, 2, 3, 4, 5]);
}

#[test]
fn buffalo_zcl_write_extension_field_sets() {
    let mut buffalo = BuffaloZcl::new(vec![0; 12]);
    buffalo.write(
        BuffaloZclDataType::EXTENSION_FIELD_SETS as u32,
        &json!([{"clstId": 768, "len": 8, "extField": [1, 2, 3, 4, 5]}]),
        &json!({}),
    );
    assert_eq!(buffalo.get_position(), 11);
    assert_eq!(buffalo.get_written(), vec![0, 3, 8, 1, 0, 2, 0, 3, 0, 4, 5]);
}

#[test]
fn buffalo_zcl_write_zoneinfo() {
    let mut buffalo = BuffaloZcl::new(vec![0; 6]);
    buffalo.write(
        BuffaloZclDataType::LIST_ZONEINFO as u32,
        &json!([{"zoneID": 1, "zoneStatus": 5}, {"zoneID": 2, "zoneStatus": 6}]),
        &json!({}),
    );
    assert_eq!(buffalo.get_position(), 6);
    assert_eq!(buffalo.get_written(), vec![1, 5, 0, 2, 6, 0]);
}

#[test]
fn buffalo_zcl_write_uint40_data40_bitmap40() {
    for data_type in [DataType::UINT40, DataType::DATA40, DataType::BITMAP40] {
        let mut buffalo = BuffaloZcl::new(vec![0; 5]);
        buffalo.write(data_type as u32, &json!(1099511627773u64), &json!({}));
        assert_eq!(buffalo.get_position(), 5);
        assert_eq!(buffalo.get_written(), vec![253, 255, 255, 255, 255]);
    }
}

#[test]
fn buffalo_zcl_write_uint48_data48_bitmap48() {
    for data_type in [DataType::UINT48, DataType::DATA48, DataType::BITMAP48] {
        let mut buffalo = BuffaloZcl::new(vec![0; 6]);
        buffalo.write(data_type as u32, &json!(281474976710653u64), &json!({}));
        assert_eq!(buffalo.get_position(), 6);
        assert_eq!(buffalo.get_written(), vec![253, 255, 255, 255, 255, 255]);
    }
}

#[test]
fn buffalo_zcl_write_uint56_data56_bitmap56() {
    for data_type in [DataType::UINT56, DataType::DATA56, DataType::BITMAP56] {
        let mut buffalo = BuffaloZcl::new(vec![0; 7]);
        buffalo.write(data_type as u32, &json!(1978047272322305u64), &json!({}));
        assert_eq!(buffalo.get_position(), 7);
        assert_eq!(buffalo.get_written(), vec![1, 5, 4, 5, 6, 7, 7]);
    }
}

#[test]
fn buffalo_zcl_write_uint64_data64_bitmap64() {
    for data_type in [DataType::UINT64, DataType::DATA64, DataType::BITMAP64] {
        let mut buffalo = BuffaloZcl::new(vec![0; 8]);
        buffalo.write(data_type as u32, &json!(0x0907070605040501u64), &json!({}));
        assert_eq!(buffalo.get_position(), 8);
        assert_eq!(buffalo.get_written(), vec![1, 5, 4, 5, 6, 7, 7, 9]);
    }
}

#[test]
fn buffalo_zcl_write_int40_int56_int64() {
    let mut buffalo = BuffaloZcl::new(vec![0; 5]);
    buffalo.write(DataType::INT40 as u32, &json!(549755813886u64), &json!({}));
    assert_eq!(buffalo.get_position(), 5);
    assert_eq!(buffalo.get_written(), vec![254, 255, 255, 255, 127]);

    let mut buffalo = BuffaloZcl::new(vec![0; 7]);
    buffalo.write(
        DataType::INT56 as u32,
        &json!(1978047272322305u64),
        &json!({}),
    );
    assert_eq!(buffalo.get_position(), 7);
    assert_eq!(buffalo.get_written(), vec![1, 5, 4, 5, 6, 7, 7]);

    let mut buffalo = BuffaloZcl::new(vec![0; 8]);
    buffalo.write(
        DataType::INT64 as u32,
        &json!(650496393613673729u64),
        &json!({}),
    );
    assert_eq!(buffalo.get_position(), 8);
    assert_eq!(buffalo.get_written(), vec![1, 5, 4, 5, 6, 7, 7, 9]);
}

#[test]
fn buffalo_zcl_write_array_element_type_string() {
    let payload = json!({
        "elementType": 0x41,
        "elements": [
            [0, 13, 1, 6, 0, 2],
            [1, 13, 2, 6, 0, 2],
            [2, 13, 3, 6, 0, 2],
            [3, 13, 4, 6, 0, 2]
        ]
    });
    let expected = vec![
        0x41, 0x04, 0x00, 6, 0, 13, 1, 6, 0, 2, 6, 1, 13, 2, 6, 0, 2, 6, 2, 13, 3, 6, 0, 2, 6, 3,
        13, 4, 6, 0, 2,
    ];
    let mut buffalo = BuffaloZcl::new(vec![0; expected.len()]);
    buffalo.write(DataType::ARRAY as u32, &payload, &json!({}));
    assert_eq!(buffalo.get_position(), expected.len());
    assert_eq!(buffalo.get_written(), expected);
}

#[test]
fn buffalo_zcl_write_array_element_type_numeric() {
    let payload = json!({"elementType": 8, "elements": [0, 0, 0, 0]});
    let expected = vec![0x08, 0x04, 0x00, 0, 0, 0, 0];
    let mut buffalo = BuffaloZcl::new(vec![0; expected.len()]);
    buffalo.write(DataType::ARRAY as u32, &payload, &json!({}));
    assert_eq!(buffalo.get_position(), expected.len());
    assert_eq!(buffalo.get_written(), expected);

    let payload = json!({"elementType": "DATA8", "elements": [0, 0, 0, 0]});
    let mut buffalo = BuffaloZcl::new(vec![0; expected.len()]);
    buffalo.write(DataType::ARRAY as u32, &payload, &json!({}));
    assert_eq!(buffalo.get_position(), expected.len());
    assert_eq!(buffalo.get_written(), expected);
}

#[test]
fn buffalo_zcl_write_struct() {
    let payload = json!([
        {"elmType": 32, "elmVal": 3},
        {"elmType": 66, "elmVal": "a"}
    ]);
    let expected = vec![2, 0, 32, 3, 66, 1, 0x61];
    let mut buffalo = BuffaloZcl::new(vec![0; expected.len()]);
    buffalo.write(DataType::STRUCT as u32, &payload, &json!({}));
    assert_eq!(buffalo.get_position(), expected.len());
    assert_eq!(buffalo.get_written(), expected);
}

#[test]
fn buffalo_zcl_write_security_key() {
    let payload = json!([
        0x09, 0x07, 0x07, 0x06, 0x05, 0x04, 0x05, 0x01, 0x09, 0x07, 0x07, 0x06, 0x05, 0x04, 0x05,
        0x01
    ]);
    let expected = vec![
        0x09, 0x07, 0x07, 0x06, 0x05, 0x04, 0x05, 0x01, 0x09, 0x07, 0x07, 0x06, 0x05, 0x04, 0x05,
        0x01,
    ];
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    buffalo.write(DataType::SEC_KEY as u32, &payload, &json!({}));
    assert_eq!(buffalo.get_position(), 16);
    assert_eq!(buffalo.get_written(), expected);
}

#[test]
fn buffalo_zcl_write_list_tuya_data_point_values() {
    let cases = vec![
        (json!([]), vec![]),
        (
            json!([{"dp": 1, "datatype": 4, "data": [1]}]),
            vec![1, 4, 0, 1, 1],
        ),
        (
            json!([
                {"dp": 1, "datatype": 4, "data": [1]},
                {"dp": 4, "datatype": 2, "data": [0, 0, 0, 90]}
            ]),
            vec![1, 4, 0, 1, 1, 4, 2, 0, 4, 0, 0, 0, 90],
        ),
    ];
    for (payload, expected) in cases {
        let mut buffalo = BuffaloZcl::new(vec![0; expected.len()]);
        buffalo.write(
            BuffaloZclDataType::LIST_TUYA_DATAPOINT_VALUES as u32,
            &payload,
            &json!({}),
        );
        assert_eq!(buffalo.get_position(), expected.len());
        assert_eq!(buffalo.get_written(), expected);
    }
}

#[test]
fn zcl_utils_get_cluster_without_manufacturer_code() {
    let clusters = manu_specific_assa_clusters();
    let cluster = Utils::get_cluster(0xfc00u16, None, &clusters);
    assert_eq!(cluster.ID, 0xfc00);
    assert_eq!(cluster.name, "manuSpecificAssaDoorLock");
}

#[test]
fn zcl_utils_get_cluster_with_manufacturer_code() {
    let clusters = manu_specific_assa_clusters();
    let cluster = Utils::get_cluster(0xfc57u16, Some(0x1217), &clusters);
    assert_eq!(cluster.ID, 0xfc57);
    assert_eq!(cluster.name, "manuSpecificAmazonWWAH");
}

#[test]
fn zcl_utils_get_cluster_manufacturer_code_wrong() {
    let clusters = manu_specific_assa_clusters();
    let cluster = Utils::get_cluster(0xfc00u16, Some(123), &clusters);
    assert_eq!(cluster.ID, 0xfc00);
    assert_eq!(cluster.name, "manuSpecificAssaDoorLock");
}

#[test]
fn zcl_utils_get_cluster_attributes_manufacturer_code() {
    let mut cluster = Cluster::new("closuresWindowCovering", 0x0102);
    let mut attribute = zigbee_apiarist::zspec::zcl::utils::Attribute::new(
        "stepPositionTilt",
        0xf004,
        DataType::ENUM8,
    );
    attribute.manufacturerCode = Some(0x1021);
    attribute.write = true;
    attribute.max = Some(255.0);
    cluster
        .attributes
        .insert("stepPositionTilt".to_string(), attribute);
    let mut clusters = CustomClusters::empty();
    clusters.insert(cluster);

    let cluster = Utils::get_cluster("closuresWindowCovering", Some(0x1021), &clusters);
    let attribute = Utils::get_cluster_attribute(&cluster, AttributeKey::Id(0xf004), Some(0x1021))
        .expect("attribute");
    assert_eq!(attribute.ID, 0xf004);
    assert_eq!(attribute.manufacturerCode, Some(0x1021));
    assert_eq!(attribute.name, "stepPositionTilt");
    assert_eq!(attribute.r#type, DataType::ENUM8);
}

#[test]
fn zcl_utils_get_cluster_attributes_manufacturer_code_wrong() {
    let cluster = Utils::get_cluster(
        "closuresWindowCovering",
        Some(123),
        &CustomClusters::empty(),
    );
    assert!(Utils::get_cluster_attribute(&cluster, AttributeKey::Id(0x1000), Some(123)).is_none());
}

#[test]
fn zcl_utils_get_command() {
    let cluster = Utils::get_cluster("genOnOff", None, &CustomClusters::empty());
    let command = Utils::get_cluster_command(&cluster, CommandKey::Id(0));
    assert_eq!(command.name, "off");
    assert_eq!(
        Utils::get_cluster_command(&cluster, CommandKey::Name("off".into())),
        command
    );
}

#[test]
fn zcl_utils_get_attribute() {
    let cluster = Utils::get_cluster("genOnOff", None, &CustomClusters::empty());
    let attribute =
        Utils::get_cluster_attribute(&cluster, AttributeKey::Id(16385), None).expect("attribute");
    assert_eq!(attribute.name, "onTime");
    assert_eq!(
        Utils::get_cluster_attribute(&cluster, AttributeKey::Name("onTime".into()), None),
        Some(attribute)
    );
}

#[test]
fn zcl_utils_get_attribute_non_existing() {
    let cluster = Utils::get_cluster("genOnOff", None, &CustomClusters::empty());
    assert!(
        Utils::get_cluster_attribute(&cluster, AttributeKey::Name("notExisting".into()), None)
            .is_none()
    );
}

#[test]
fn zcl_utils_get_command_non_existing() {
    let cluster = Utils::get_cluster("genOnOff", None, &CustomClusters::empty());
    let error = catch_unwind(AssertUnwindSafe(|| {
        Utils::get_cluster_command(&cluster, CommandKey::Name("notExisting".into()));
    }))
    .expect_err("must panic");
    let message = error
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| error.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default();
    assert_eq!(message, "Status 'UNSUP_COMMAND' genOnOff:notExisting");
}

#[test]
fn buffalo_zcl_read_big_endian_uint24() {
    let mut buffalo = BuffaloZcl::new(vec![0x01, 0x01, 0x86, 0xa0, 0x02]);
    buffalo.set_position(1);
    let value = buffalo.read(BuffaloZclDataType::BIG_ENDIAN_UINT24 as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 4);
    assert_eq!(value, json!(100000));
}

#[test]
fn buffalo_zcl_write_big_endian_uint24() {
    let mut buffalo = BuffaloZcl::new(vec![0; 4]);
    buffalo.set_position(1);
    buffalo.write(
        BuffaloZclDataType::BIG_ENDIAN_UINT24 as u32,
        &json!(16777200),
        &json!({}),
    );
    assert_eq!(buffalo.get_position(), 4);
    assert_eq!(buffalo.buffer, vec![0x00, 0xff, 0xff, 0xf0]);
}

#[test]
fn buffalo_zcl_read_buffer_whole_and_with_length() {
    let mut buffalo = BuffaloZcl::new(vec![1, 2, 3, 4, 5]);
    assert_eq!(
        buffalo.read(BuffaloZclDataType::BUFFER as u32, &json!({})),
        json!([1, 2, 3, 4, 5])
    );

    let mut buffalo = BuffaloZcl::new(vec![1, 2, 3, 4, 5]);
    assert_eq!(
        buffalo.read(BuffaloZclDataType::BUFFER as u32, &json!({"length": 3})),
        json!([1, 2, 3])
    );
}

#[test]
fn buffalo_zcl_write_buffer_whole_and_length_ignored() {
    let mut buffalo = BuffaloZcl::new(vec![0; 5]);
    buffalo.write(
        BuffaloZclDataType::BUFFER as u32,
        &json!([1, 2, 3, 4, 5]),
        &json!({}),
    );
    assert_eq!(buffalo.buffer, vec![1, 2, 3, 4, 5]);

    let mut buffalo = BuffaloZcl::new(vec![0; 5]);
    buffalo.write(
        BuffaloZclDataType::BUFFER as u32,
        &json!([1, 2, 3, 4, 5]),
        &json!({"length": 3}),
    );
    assert_eq!(buffalo.buffer, vec![1, 2, 3, 4, 5]);
}

fn panic_message(function: impl FnOnce()) -> String {
    let error = catch_unwind(AssertUnwindSafe(function)).expect_err("must panic");
    error
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| error.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default()
}

#[test]
fn buffalo_zcl_write_and_read_non_existing_type() {
    assert_eq!(
        panic_message(|| {
            let mut buffalo = BuffaloZcl::new(Vec::new());
            buffalo.write(9999, &json!(1), &json!({}));
        }),
        "Write for '9999' not available"
    );
    assert_eq!(
        panic_message(|| {
            let mut buffalo = BuffaloZcl::new(Vec::new());
            buffalo.read(9999, &json!({}));
        }),
        "Read for '9999' not available"
    );
}

#[test]
fn buffalo_zcl_write_and_read_tod_date() {
    let mut buffalo = BuffaloZcl::new(vec![0; 4]);
    buffalo.write(
        DataType::TOD as u32,
        &json!({"hours": 0, "minutes": 59, "seconds": 34, "hundredths": 88}),
        &json!({}),
    );
    assert_eq!(buffalo.buffer, vec![0, 59, 34, 88]);
    let mut buffalo = BuffaloZcl::new(vec![0, 59, 34, 88]);
    assert_eq!(
        buffalo.read(DataType::TOD as u32, &json!({})),
        json!({"hours": 0, "minutes": 59, "seconds": 34, "hundredths": 88})
    );

    let mut buffalo = BuffaloZcl::new(vec![0; 4]);
    buffalo.write(
        DataType::DATE as u32,
        &json!({"year": 2000, "month": 8, "dayOfMonth": 31, "dayOfWeek": 3}),
        &json!({}),
    );
    assert_eq!(buffalo.buffer, vec![100, 8, 31, 3]);
    let mut buffalo = BuffaloZcl::new(vec![100, 8, 31, 3]);
    assert_eq!(
        buffalo.read(DataType::DATE as u32, &json!({})),
        json!({"year": 2000, "month": 8, "dayOfMonth": 31, "dayOfWeek": 3})
    );
}

#[test]
fn buffalo_zcl_write_tod_date_non_value() {
    let mut buffalo = BuffaloZcl::new(vec![0; 4]);
    buffalo.write(
        DataType::TOD as u32,
        &json!({"hours": Value::Null, "minutes": Value::Null, "seconds": Value::Null, "hundredths": Value::Null}),
        &json!({}),
    );
    assert_eq!(buffalo.buffer, vec![0xff, 0xff, 0xff, 0xff]);

    let mut buffalo = BuffaloZcl::new(vec![0; 4]);
    buffalo.write(
        DataType::DATE as u32,
        &json!({"year": Value::Null, "month": Value::Null, "dayOfMonth": Value::Null, "dayOfWeek": Value::Null}),
        &json!({}),
    );
    assert_eq!(buffalo.buffer, vec![0xff, 0xff, 0xff, 0xff]);
}

#[test]
fn buffalo_zcl_read_non_value_sentinels() {
    assert_eq!(
        BuffaloZcl::new(vec![0xff]).read(DataType::OCTET_STR as u32, &json!({})),
        json!([])
    );
    assert_eq!(
        BuffaloZcl::new(vec![0xff]).read(DataType::CHAR_STR as u32, &json!({})),
        json!("")
    );
    assert_eq!(
        BuffaloZcl::new(vec![0xff]).read(BuffaloZclDataType::MI_STRUCT as u32, &json!({})),
        json!({})
    );
    assert_eq!(
        BuffaloZcl::new(vec![0xff, 0xff]).read(DataType::LONG_OCTET_STR as u32, &json!({})),
        json!([])
    );
    assert_eq!(
        BuffaloZcl::new(vec![0xff, 0xff]).read(DataType::LONG_CHAR_STR as u32, &json!({})),
        json!("")
    );
    assert_eq!(
        BuffaloZcl::new(vec![0x20, 0xff, 0xff]).read(DataType::ARRAY as u32, &json!({})),
        json!([])
    );
    assert_eq!(
        BuffaloZcl::new(vec![0xff, 0xff]).read(DataType::STRUCT as u32, &json!({})),
        json!([])
    );
}

#[test]
fn buffalo_zcl_write_and_read_no_data_unknown() {
    for data_type in [DataType::NO_DATA, DataType::UNKNOWN] {
        let mut buffalo = BuffaloZcl::new(vec![0, 0]);
        buffalo.write(data_type as u32, &Value::Null, &json!({}));
        assert_eq!(buffalo.get_position(), 0);
        assert_eq!(buffalo.buffer, vec![0, 0]);

        let mut buffalo = BuffaloZcl::new(vec![0, 0]);
        buffalo.set_position(1);
        let value = buffalo.read(data_type as u32, &json!({}));
        assert_eq!(buffalo.get_position(), 1);
        assert!(value.is_null());
    }
}

#[test]
fn zcl_header_return_undefined_when_too_short() {
    assert!(Header::from_buffer(&[0, 8]).is_none());
    assert!(Header::from_buffer(&[4, 8, 3]).is_none());
}

#[test]
fn zcl_green_power_read_gpd_commissioning() {
    let buffer = vec![0xff, 0x00];
    let mut buffalo = BuffaloZcl::new(buffer.clone());
    let value = buffalo.read(
        BuffaloZclDataType::GPD_FRAME as u32,
        &json!({"payload": {"payloadSize": buffer.len(), "commandID": 0xe0}}),
    );
    assert_eq!(
        value,
        json!({
            "deviceID": 0xff,
            "options": 0x00,
            "extendedOptions": 0x00,
            "securityKey": vec![0u8; 16],
            "keyMic": 0,
            "outgoingCounter": 0,
            "manufacturerID": 0,
            "modelID": 0,
            "numGpdCommands": 0,
            "gpdCommandIdList": [],
            "numServerClusters": 0,
            "numClientClusters": 0,
            "gpdServerClusters": [],
            "gpdClientClusters": [],
            "applicationInfo": 0x00,
            "genericSwitchConfig": 0,
            "currentContactStatus": 0
        })
    );
}

#[test]
fn zcl_green_power_read_gpd_commissioning_all_options() {
    let buffer = vec![
        0xff, 0x84, 0xe0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0x1f, 0, 0, 0, 0, 0, 0, 2, 0, 1,
    ];
    let mut buffalo = BuffaloZcl::new(buffer.clone());
    let value = buffalo.read(
        BuffaloZclDataType::GPD_FRAME as u32,
        &json!({"payload": {"payloadSize": buffer.len(), "commandID": 0xe0}}),
    );
    assert_eq!(
        value,
        json!({
            "deviceID": 0xff,
            "options": 0x84,
            "extendedOptions": 0xe0,
            "securityKey": vec![0u8; 16],
            "keyMic": 0,
            "outgoingCounter": 0,
            "manufacturerID": 0,
            "modelID": 0,
            "numGpdCommands": 0,
            "gpdCommandIdList": [],
            "numServerClusters": 0,
            "numClientClusters": 0,
            "gpdServerClusters": [],
            "gpdClientClusters": [],
            "applicationInfo": 0x1f,
            "genericSwitchConfig": 0,
            "currentContactStatus": 1
        })
    );
}

#[test]
fn zcl_green_power_read_gpd_channel_request() {
    let mut buffalo = BuffaloZcl::new(vec![0xfa]);
    let value = buffalo.read(
        BuffaloZclDataType::GPD_FRAME as u32,
        &json!({"payload": {"payloadSize": 1, "commandID": 0xe3}}),
    );
    assert_eq!(value, json!({"nextChannel": 0xa, "nextNextChannel": 0xf}));
}

#[test]
fn zcl_green_power_read_gpd_attribute_report() {
    let buffer = vec![
        0x12,
        0x34,
        0xff,
        0xff,
        0x00,
        0x00,
        DataType::UINT32 as u8,
        0x00,
        0x01,
        0x02,
        0x03,
        0x01,
        0x00,
        DataType::CHAR_STR as u8,
        0x06,
        0x5a,
        0x49,
        0x47,
        0x42,
        0x45,
        0x45,
        0x02,
        0x00,
        DataType::BOOLEAN as u8,
        0x01,
    ];
    let mut buffalo = BuffaloZcl::new(buffer.clone());
    let value = buffalo.read(
        BuffaloZclDataType::GPD_FRAME as u32,
        &json!({"payload": {"commandID": 0xa1, "payloadSize": buffer.len()}}),
    );
    assert_eq!(
        value,
        json!({
            "manufacturerCode": 13330,
            "clusterID": 65535,
            "clusterName": "65535",
            "attributes": {"0": 50462976, "1": "ZIGBEE", "2": 1}
        })
    );
}

#[test]
fn zcl_green_power_write_gpd_commissioning() {
    let mut buffalo = BuffaloZcl::new(vec![0; 2]);
    buffalo.write(
        BuffaloZclDataType::GPD_FRAME as u32,
        &json!({"commandID": 0xf0, "options": 0, "panID": 0, "securityKey": vec![0u8; 16], "keyMic": 0, "frameCounter": 0}),
        &json!({}),
    );
    assert_eq!(buffalo.get_written(), vec![1, 0]);
}

#[test]
fn zcl_green_power_write_gpd_commissioning_all_options() {
    let mut expected = vec![27, 0b11111, 0xff, 0xff];
    expected.extend_from_slice(&[0u8; 24]);
    let mut buffalo = BuffaloZcl::new(vec![0; 28]);
    buffalo.write(
        BuffaloZclDataType::GPD_FRAME as u32,
        &json!({"commandID": 0xf0, "options": 0b11111, "panID": 0xffff, "securityKey": vec![0u8; 16], "keyMic": 0, "frameCounter": 0}),
        &json!({}),
    );
    assert_eq!(buffalo.get_written(), expected);
}

#[test]
fn zcl_green_power_write_gpd_custom_reply() {
    let mut buffalo = BuffaloZcl::new(vec![0; 7]);
    buffalo.write(
        BuffaloZclDataType::GPD_FRAME as u32,
        &json!({"commandID": 0xf4, "buffer": b"ZIGBEE".to_vec()}),
        &json!({}),
    );
    assert_eq!(buffalo.get_written(), vec![6, 90, 73, 71, 66, 69, 69]);
}

#[test]
fn zcl_green_power_write_gpd_unhandled_command() {
    let mut buffalo = BuffaloZcl::new(vec![0; 7]);
    buffalo.write(
        BuffaloZclDataType::GPD_FRAME as u32,
        &json!({"commandID": 0x1ff}),
        &json!({}),
    );
    assert!(buffalo.get_written().is_empty());
}

#[test]
fn zcl_green_power_write_gpd_channel_configuration() {
    let mut buffalo = BuffaloZcl::new(vec![0; 2]);
    buffalo.write(
        BuffaloZclDataType::GPD_FRAME as u32,
        &json!({"commandID": 0xf3, "operationalChannel": 0xf, "basic": false}),
        &json!({}),
    );
    assert_eq!(buffalo.get_written(), vec![1, 0xf]);

    let mut buffalo = BuffaloZcl::new(vec![0; 2]);
    buffalo.write(
        BuffaloZclDataType::GPD_FRAME as u32,
        &json!({"commandID": 0xf3, "operationalChannel": 0xf, "basic": true}),
        &json!({}),
    );
    assert_eq!(buffalo.get_written(), vec![1, 0x1f]);
}

fn miboxer_clusters() -> CustomClusters {
    let mut cluster = Cluster::new("genGroups", 0x0004);
    let mut command = zigbee_apiarist::zspec::zcl::utils::Command::new("miboxerSetZones", 0xf0);
    command.parameters =
        vec![
            zigbee_apiarist::zspec::zcl::utils::Parameter::new("zones", DataType::UNKNOWN)
                .with_buffalo_type(BuffaloZclDataType::LIST_MIBOXER_ZONES),
        ];
    command.order = 0;
    cluster
        .commands
        .insert("miboxerSetZones".to_string(), command);
    let mut clusters = CustomClusters::empty();
    clusters.insert(cluster);
    clusters
}

#[test]
fn zcl_frame_parse_mi_boxer_zone_configuration_command() {
    let buffer = vec![
        0x11, 0x01, 0xf0, 0x08, 0x84, 0x2b, 0x01, 0x98, 0x2b, 0x02, 0xac, 0x2b, 0x03, 0xc0, 0x2b,
        0x04, 0xd4, 0x2b, 0x05, 0xe8, 0x2b, 0x06, 0xfc, 0x2b, 0x07, 0x10, 0x2c, 0x08,
    ];
    let clusters = miboxer_clusters();
    let frame = parse_custom(0x0004, &buffer, &clusters);
    assert_eq!(
        frame.payload["zones"],
        json!([
            {"zoneNum": 1, "groupId": 0x2b84},
            {"zoneNum": 2, "groupId": 0x2b98},
            {"zoneNum": 3, "groupId": 0x2bac},
            {"zoneNum": 4, "groupId": 0x2bc0},
            {"zoneNum": 5, "groupId": 0x2bd4},
            {"zoneNum": 6, "groupId": 0x2be8},
            {"zoneNum": 7, "groupId": 0x2bfc},
            {"zoneNum": 8, "groupId": 0x2c10}
        ])
    );
}

#[test]
fn zcl_frame_serialize_mi_boxer_zone_configuration_command() {
    let zones = json!([
        {"zoneNum": 1, "groupId": 0x2b84},
        {"zoneNum": 2, "groupId": 0x2b98},
        {"zoneNum": 3, "groupId": 0x2bac},
        {"zoneNum": 4, "groupId": 0x2bc0},
        {"zoneNum": 5, "groupId": 0x2bd4},
        {"zoneNum": 6, "groupId": 0x2be8},
        {"zoneNum": 7, "groupId": 0x2bfc},
        {"zoneNum": 8, "groupId": 0x2c10}
    ]);
    let clusters = miboxer_clusters();
    let frame = Frame::create_with_custom_clusters(
        FrameType::SPECIFIC as u8,
        Direction::CLIENT_TO_SERVER as u8,
        true,
        None,
        1,
        "miboxerSetZones",
        0x0004,
        json!({"zones": zones}),
        &clusters,
        0,
    );
    assert_eq!(
        frame.to_buffer(),
        vec![
            0x11, 0x01, 0xf0, 0x08, 0x84, 0x2b, 0x01, 0x98, 0x2b, 0x02, 0xac, 0x2b, 0x03, 0xc0,
            0x2b, 0x04, 0xd4, 0x2b, 0x05, 0xe8, 0x2b, 0x06, 0xfc, 0x2b, 0x07, 0x10, 0x2c, 0x08
        ]
    );
}

#[test]
fn buffalo_zcl_write_passthrough_to_buffalo() {
    let cases: &[(&str, &[u32], usize)] = &[
        (
            "uint8",
            &[
                DataType::DATA8 as u32,
                DataType::BOOLEAN as u32,
                DataType::BITMAP8 as u32,
                DataType::UINT8 as u32,
                DataType::ENUM8 as u32,
            ],
            1,
        ),
        (
            "uint16",
            &[
                DataType::DATA16 as u32,
                DataType::BITMAP16 as u32,
                DataType::UINT16 as u32,
                DataType::ENUM16 as u32,
                DataType::CLUSTER_ID as u32,
                DataType::ATTR_ID as u32,
            ],
            2,
        ),
        (
            "uint24",
            &[
                DataType::DATA24 as u32,
                DataType::BITMAP24 as u32,
                DataType::UINT24 as u32,
            ],
            3,
        ),
        (
            "uint32",
            &[
                DataType::DATA32 as u32,
                DataType::BITMAP32 as u32,
                DataType::UINT32 as u32,
                DataType::UTC as u32,
                DataType::BAC_OID as u32,
            ],
            4,
        ),
        ("int8", &[DataType::INT8 as u32], 1),
        ("int16", &[DataType::INT16 as u32], 2),
        ("int24", &[DataType::INT24 as u32], 3),
        ("int32", &[DataType::INT32 as u32], 4),
        ("int48", &[DataType::INT48 as u32], 6),
        ("single", &[DataType::SINGLE_PREC as u32], 4),
        ("double", &[DataType::DOUBLE_PREC as u32], 8),
        ("ieee", &[DataType::IEEE_ADDR as u32], 8),
    ];
    for (_name, types, width) in cases {
        let mut buffalo = BuffaloZcl::new(vec![0; 255]);
        let mut expected = 0;
        for data_type in *types {
            buffalo.write(*data_type, &json!(1), &json!({}));
            expected += width;
            assert_eq!(buffalo.get_position(), expected);
        }
    }

    for (data_type, width) in [
        (BuffaloZclDataType::LIST_UINT8, 1),
        (BuffaloZclDataType::LIST_UINT16, 2),
        (BuffaloZclDataType::LIST_UINT24, 3),
        (BuffaloZclDataType::LIST_UINT32, 4),
    ] {
        let mut buffalo = BuffaloZcl::new(vec![0; 255]);
        buffalo.write(data_type as u32, &json!([1]), &json!({}));
        assert_eq!(buffalo.get_position(), width);
    }
}

#[test]
fn buffalo_zcl_read_passthrough_to_buffalo() {
    let cases: &[(&[u32], usize)] = &[
        (
            &[
                DataType::DATA8 as u32,
                DataType::BOOLEAN as u32,
                DataType::BITMAP8 as u32,
                DataType::UINT8 as u32,
                DataType::ENUM8 as u32,
            ],
            1,
        ),
        (
            &[
                DataType::DATA16 as u32,
                DataType::BITMAP16 as u32,
                DataType::UINT16 as u32,
                DataType::ENUM16 as u32,
                DataType::CLUSTER_ID as u32,
                DataType::ATTR_ID as u32,
            ],
            2,
        ),
        (
            &[
                DataType::DATA24 as u32,
                DataType::BITMAP24 as u32,
                DataType::UINT24 as u32,
            ],
            3,
        ),
        (
            &[
                DataType::DATA32 as u32,
                DataType::BITMAP32 as u32,
                DataType::UINT32 as u32,
                DataType::UTC as u32,
                DataType::BAC_OID as u32,
            ],
            4,
        ),
        (&[DataType::INT8 as u32], 1),
        (&[DataType::INT16 as u32], 2),
        (&[DataType::INT24 as u32], 3),
        (&[DataType::INT32 as u32], 4),
        (&[DataType::INT48 as u32], 6),
        (&[DataType::SINGLE_PREC as u32], 4),
        (&[DataType::DOUBLE_PREC as u32], 8),
        (&[DataType::IEEE_ADDR as u32], 8),
    ];
    for (types, width) in cases {
        let mut buffalo = BuffaloZcl::new(vec![0; 255]);
        let mut expected = 0;
        for data_type in *types {
            buffalo.read(*data_type, &json!({}));
            expected += width;
            assert_eq!(buffalo.get_position(), expected);
        }
    }

    for (data_type, width) in [
        (BuffaloZclDataType::LIST_UINT8, 1),
        (BuffaloZclDataType::LIST_UINT16, 2),
        (BuffaloZclDataType::LIST_UINT24, 3),
        (BuffaloZclDataType::LIST_UINT32, 4),
    ] {
        let mut buffalo = BuffaloZcl::new(vec![0; 255]);
        buffalo.read(data_type as u32, &json!({"length": 1}));
        assert_eq!(buffalo.get_position(), width);
    }
}

#[test]
fn buffalo_zcl_throws_when_read_missing_required_length_option() {
    for (data_type, name) in [
        (BuffaloZclDataType::LIST_UINT8, "LIST_UINT8"),
        (BuffaloZclDataType::LIST_UINT16, "LIST_UINT16"),
        (BuffaloZclDataType::LIST_UINT24, "LIST_UINT24"),
        (BuffaloZclDataType::LIST_UINT32, "LIST_UINT32"),
        (BuffaloZclDataType::LIST_ZONEINFO, "LIST_ZONEINFO"),
    ] {
        assert_eq!(
            panic_message(|| {
                let mut buffalo = BuffaloZcl::new(vec![0; 1]);
                buffalo.read(data_type as u32, &json!({}));
            }),
            format!("Cannot read {name} without length option specified")
        );
    }
}

#[test]
fn buffalo_zcl_read_and_write_use_data_type_buffer_fallback() {
    let mut buffalo = BuffaloZcl::new(vec![12, 34]);
    assert_eq!(
        buffalo.read(BuffaloZclDataType::USE_DATA_TYPE as u32, &json!({})),
        json!([12, 34])
    );
    buffalo.set_position(1);
    assert_eq!(
        buffalo.read(
            BuffaloZclDataType::USE_DATA_TYPE as u32,
            &json!({"length": 1})
        ),
        json!([34])
    );

    let mut buffalo = BuffaloZcl::new(vec![0; 2]);
    buffalo.write(
        BuffaloZclDataType::USE_DATA_TYPE as u32,
        &json!([12, 34]),
        &json!({}),
    );
    assert_eq!(buffalo.buffer, vec![12, 34]);
}

#[test]
fn buffalo_zcl_throws_when_write_use_data_type_missing_and_not_buffer() {
    assert_eq!(
        panic_message(|| {
            let mut buffalo = BuffaloZcl::new(vec![0; 2]);
            buffalo.write(
                BuffaloZclDataType::USE_DATA_TYPE as u32,
                &json!("abcd"),
                &json!({}),
            );
        }),
        "Cannot write USE_DATA_TYPE without dataType option specified"
    );
}

#[test]
fn buffalo_zcl_throws_when_gpd_frame_missing_payload_size() {
    assert_eq!(
        panic_message(|| {
            let mut buffalo = BuffaloZcl::new(vec![0; 1]);
            buffalo.read(
                BuffaloZclDataType::GPD_FRAME as u32,
                &json!({"payload": {"commandID": 0xa1}}),
            );
        }),
        "Cannot read GPD_FRAME without required payload options specified"
    );
}

#[test]
fn buffalo_zcl_throws_when_list_thermo_transitions_missing_payload_options() {
    for options in [
        json!({}),
        json!({"payload": {}}),
        json!({"payload": {"mode": 1}}),
        json!({"payload": {"numoftrans": 1}}),
    ] {
        assert_eq!(
            panic_message(|| {
                let mut buffalo = BuffaloZcl::new(vec![0; 1]);
                buffalo.read(BuffaloZclDataType::LIST_THERMO_TRANSITIONS as u32, &options);
            }),
            "Cannot read LIST_THERMO_TRANSITIONS without required payload options specified"
        );
    }
}

#[test]
fn zcl_frame_to_buffer_set_weekly_schedule_rsp_hvac_thermostat() {
    let payload_heat = json!({
        "numoftrans": 6, "dayofweek": 64, "mode": 1,
        "transitions": [
            {"transitionTime": 360, "heatSetpoint": 23},
            {"transitionTime": 570, "heatSetpoint": 2200},
            {"transitionTime": 720, "heatSetpoint": 2150},
            {"transitionTime": 840, "heatSetpoint": 2150},
            {"transitionTime": 990, "heatSetpoint": 2300},
            {"transitionTime": 1380, "heatSetpoint": 2100}
        ]
    });
    let frame = Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        None,
        8,
        "setWeeklySchedule",
        513,
        payload_heat,
        json!({}),
        0,
    );
    assert_eq!(
        frame.to_buffer(),
        vec![
            1, 8, 1, 6, 64, 1, 104, 1, 23, 0, 58, 2, 152, 8, 208, 2, 102, 8, 72, 3, 102, 8, 222, 3,
            252, 8, 100, 5, 52, 8
        ]
    );

    let payload_cool = json!({
        "numoftrans": 6, "dayofweek": 64, "mode": 2,
        "transitions": [
            {"transitionTime": 360, "coolSetpoint": 2300},
            {"transitionTime": 570, "coolSetpoint": 2200},
            {"transitionTime": 720, "coolSetpoint": 2150},
            {"transitionTime": 840, "coolSetpoint": 2150},
            {"transitionTime": 990, "coolSetpoint": 2300},
            {"transitionTime": 1380, "coolSetpoint": 2100}
        ]
    });
    let frame = Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        None,
        8,
        "setWeeklySchedule",
        513,
        payload_cool,
        json!({}),
        0,
    );
    assert_eq!(
        frame.to_buffer(),
        vec![
            1, 8, 1, 6, 64, 2, 104, 1, 252, 8, 58, 2, 152, 8, 208, 2, 102, 8, 72, 3, 102, 8, 222,
            3, 252, 8, 100, 5, 52, 8
        ]
    );

    let payload_both = json!({
        "numoftrans": 6, "dayofweek": 64, "mode": 2,
        "transitions": [{"transitionTime": 360, "coolSetpoint": 570, "heatSetpoint": 2300}]
    });
    let frame = Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        None,
        8,
        "setWeeklySchedule",
        513,
        payload_both,
        json!({}),
        0,
    );
    assert_eq!(
        frame.to_buffer(),
        vec![1, 8, 1, 6, 64, 2, 104, 1, 252, 8, 58, 2]
    );
}

#[test]
fn zcl_utils_get_cluster_manufacturer_code() {
    let clusters = manu_specific_assa_clusters();
    let cluster = Utils::get_cluster(0xfc00u16, Some(0x10f2), &clusters);
    assert_eq!(cluster.ID, 0xfc00);
    assert_eq!(cluster.name, "manuSpecificAssaDoorLock");
}
