// Original: .upstream/zigbee-herdsman/test/zspec/zcl/foundation.test.ts

use std::panic::{catch_unwind, AssertUnwindSafe};

use serde_json::json;
use zigbee_apiarist::zspec::zcl::definition::cluster;
use zigbee_apiarist::zspec::zcl::definition::enums::{DataType, StructuredIndicatorType};
use zigbee_apiarist::zspec::zcl::definition::status::Status;
use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
use zigbee_apiarist::zspec::zcl::zcl_header::{Direction, FrameType};

/// Upstream `expect(frame).toMatchObject({header, payload, cluster: {name}})`.
fn assert_frame_roundtrip(frame: &Frame, cluster_name: &str) {
    let raw_frame = frame.to_buffer();
    let parsed = Frame::from_buffer(frame.cluster.ID, Some(frame.header), &raw_frame, json!({}));

    assert_eq!(parsed.header, frame.header);
    assert_eq!(parsed.payload, frame.payload);
    assert_eq!(parsed.cluster.name, cluster_name);
}

fn create_frame(
    direction: Direction,
    command: &'static str,
    cluster_name: &'static str,
    payload: serde_json::Value,
) -> Frame {
    Frame::create(
        FrameType::GLOBAL as u8,
        direction as u8,
        true,
        None,
        10,
        command,
        cluster_name,
        payload,
        json!({}),
        0,
    )
}

#[test]
#[ignore = "not implemented yet"]
fn throws_on_invalid_payload_type_when_writing() {
    let cases: [(&'static str, serde_json::Value); 23] = [
        ("read", json!({"a": 1})),
        ("readRsp", json!({"a": 1})),
        ("write", json!({"a": 1})),
        ("writeUndiv", json!({"a": 1})),
        ("writeRsp", json!({"a": 1})),
        ("writeNoRsp", json!({"a": 1})),
        ("configReport", json!({"a": 1})),
        ("configReportRsp", json!({"a": 1})),
        ("readReportConfig", json!({"a": 1})),
        ("readReportConfigRsp", json!({"a": 1})),
        ("report", json!({"a": 1})),
        ("defaultRsp", json!([{"a": 1}])),
        ("discover", json!([{"a": 1}])),
        ("discoverRsp", json!([{"a": 1}])),
        ("readStructured", json!({"a": 1})),
        ("writeStructured", json!({"a": 1})),
        ("writeStructuredRsp", json!({"a": 1})),
        ("discoverCommands", json!([{"a": 1}])),
        ("discoverCommandsRsp", json!([{"a": 1}])),
        ("discoverCommandsGen", json!([{"a": 1}])),
        ("discoverCommandsGenRsp", json!([{"a": 1}])),
        ("discoverExt", json!([{"a": 1}])),
        ("discoverExtRsp", json!([{"a": 1}])),
    ];

    for (cmd, payload) in cases {
        let frame = create_frame(Direction::CLIENT_TO_SERVER, cmd, "genBasic", payload);

        let result = catch_unwind(AssertUnwindSafe(|| frame.to_buffer()));
        assert!(result.is_err(), "expected {cmd} payload to be rejected");
    }
}

// ---------------------------------- read ----------------------------------

#[test]
#[ignore = "not implemented yet"]
fn read_parses_writes() {
    let frame = create_frame(
        Direction::CLIENT_TO_SERVER,
        "read",
        "genBasic",
        json!([
            {"attrId": cluster::genBasic::attributes::zclVersion.ID},
            {"attrId": cluster::genBasic::attributes::modelId.ID},
        ]),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(raw_frame, vec![0x10, 0x0a, 0x00, 0x00, 0x00, 0x05, 0x00]);
    assert_frame_roundtrip(&frame, "genBasic");
}

// --------------------------------- readRsp ---------------------------------

#[test]
#[ignore = "not implemented yet"]
fn read_rsp_parses_writes() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "readRsp",
        "genBasic",
        json!([
            {
                "attrId": cluster::genBasic::attributes::zclVersion.ID,
                "status": Status::SUCCESS as u8,
                "dataType": cluster::genBasic::attributes::zclVersion.r#type as u8,
                "attrData": 8,
            },
            {
                "attrId": cluster::genBasic::attributes::manufacturerName.ID,
                "status": Status::SUCCESS as u8,
                "dataType": cluster::genBasic::attributes::manufacturerName.r#type as u8,
                "attrData": "efgh",
            },
        ]),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(
        raw_frame,
        vec![
            0x18, 0x0a, 0x01, 0x00, 0x00, 0x00, 0x20, 0x08, 0x04, 0x00, 0x00, 0x42, 0x04, 0x65,
            0x66, 0x67, 0x68,
        ]
    );
    assert_frame_roundtrip(&frame, "genBasic");
}

#[test]
#[ignore = "not implemented yet"]
fn read_rsp_workaround_parses_char_str_as_mi_struct_for_attribute_0xff01() {
    let raw_frame = vec![
        0x18, 0x0a, 0x01, 0x01, 0xff, 0x00, 0x42, 34, 1, 33, 213, 12, 3, 40, 33, 4, 33, 168, 19, 5,
        33, 43, 0, 6, 36, 0, 0, 5, 0, 0, 8, 33, 4, 2, 10, 33, 0, 0, 100, 16, 0,
    ];

    let frame = Frame::from_buffer(
        0x0000,
        zigbee_apiarist::zspec::zcl::zcl_header::Header::from_buffer(&raw_frame),
        &raw_frame,
        json!({}),
    );

    assert_eq!(
        frame.payload,
        json!([{
            "attrId": 0xff01,
            "status": Status::SUCCESS as u8,
            "dataType": DataType::CHAR_STR as u8,
            "attrData": {
                "1": 3285,
                "3": 33,
                "4": 5032,
                "5": 43,
                "6": 327680,
                "8": 516,
                "10": 0,
                "100": 0,
            },
        }])
    );
    assert_eq!(frame.cluster.name, "genBasic");
}

// ---------------------------------- write ----------------------------------

#[test]
#[ignore = "not implemented yet"]
fn write_parses_writes() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "write",
        "genOnOff",
        json!([
            {
                "attrId": cluster::genOnOff::attributes::onTime.ID,
                "dataType": cluster::genOnOff::attributes::onTime.r#type as u8,
                "attrData": 8,
            },
            {
                "attrId": cluster::genOnOff::attributes::offWaitTime.ID,
                "dataType": cluster::genOnOff::attributes::offWaitTime.r#type as u8,
                "attrData": 4,
            },
        ]),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(
        raw_frame,
        vec![0x18, 0x0a, 0x02, 0x01, 0x40, 0x21, 0x08, 0x00, 0x02, 0x40, 0x21, 0x04, 0x00]
    );
    assert_frame_roundtrip(&frame, "genOnOff");
}

#[test]
#[ignore = "not implemented yet"]
fn write_undiv_parses_writes() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "writeUndiv",
        "genOnOff",
        json!([
            {
                "attrId": cluster::genOnOff::attributes::onTime.ID,
                "dataType": cluster::genOnOff::attributes::onTime.r#type as u8,
                "attrData": 8,
            },
            {
                "attrId": cluster::genOnOff::attributes::offWaitTime.ID,
                "dataType": cluster::genOnOff::attributes::offWaitTime.r#type as u8,
                "attrData": 4,
            },
        ]),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(
        raw_frame,
        vec![0x18, 0x0a, 0x03, 0x01, 0x40, 0x21, 0x08, 0x00, 0x02, 0x40, 0x21, 0x04, 0x00]
    );
    assert_frame_roundtrip(&frame, "genOnOff");
}

#[test]
#[ignore = "not implemented yet"]
fn write_rsp_parses_writes() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "writeRsp",
        "genOnOff",
        json!([
            {
                "status": Status::UNSUPPORTED_ATTRIBUTE as u8,
                "attrId": cluster::genOnOff::attributes::onTime.ID,
            },
            {
                "status": Status::NOT_AUTHORIZED as u8,
                "attrId": cluster::genOnOff::attributes::offWaitTime.ID,
            },
        ]),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(
        raw_frame,
        vec![0x18, 0x0a, 0x04, 0x86, 0x01, 0x40, 0x7e, 0x02, 0x40]
    );
    assert_frame_roundtrip(&frame, "genOnOff");
}

#[test]
#[ignore = "not implemented yet"]
fn write_rsp_handles_success_payload() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "writeRsp",
        "genOnOff",
        json!([{"status": Status::SUCCESS as u8}]),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(raw_frame, vec![0x18, 0x0a, 0x04, 0x00]);
    assert_frame_roundtrip(&frame, "genOnOff");
}

// -------------------------------- writeNoRsp --------------------------------

#[test]
#[ignore = "not implemented yet"]
fn write_no_rsp_parses_writes() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "writeNoRsp",
        "genOnOff",
        json!([
            {
                "attrId": cluster::genOnOff::attributes::onTime.ID,
                "dataType": cluster::genOnOff::attributes::onTime.r#type as u8,
                "attrData": 8,
            },
            {
                "attrId": cluster::genOnOff::attributes::offWaitTime.ID,
                "dataType": cluster::genOnOff::attributes::offWaitTime.r#type as u8,
                "attrData": 4,
            },
        ]),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(
        raw_frame,
        vec![0x18, 0x0a, 0x05, 0x01, 0x40, 0x21, 0x08, 0x00, 0x02, 0x40, 0x21, 0x04, 0x00]
    );
    assert_frame_roundtrip(&frame, "genOnOff");
}

// ------------------------------- configReport -------------------------------

#[test]
#[ignore = "not implemented yet"]
fn config_report_parses_writes() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "configReport",
        "genOnOff",
        json!([
            {
                "direction": Direction::CLIENT_TO_SERVER as u8,
                "attrId": cluster::genOnOff::attributes::onTime.ID,
                "dataType": cluster::genOnOff::attributes::onTime.r#type as u8,
                "minRepIntval": 300,
                "maxRepIntval": 4000,
                "repChange": 3,
            },
            {
                "direction": Direction::CLIENT_TO_SERVER as u8,
                "attrId": cluster::genOnOff::attributes::onOff.ID,
                "dataType": cluster::genOnOff::attributes::onOff.r#type as u8,
                "minRepIntval": 250,
                "maxRepIntval": 5290,
            },
            {
                "direction": Direction::SERVER_TO_CLIENT as u8,
                "attrId": cluster::genOnOff::attributes::startUpOnOff.ID,
                "timeout": 3000,
            },
        ]),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(
        raw_frame,
        vec![
            0x18, 0x0a, 0x06, 0x00, 0x01, 0x40, 0x21, 0x2c, 0x01, 0xa0, 0x0f, 0x03, 0x00, 0x00,
            0x00, 0x00, 0x10, 0xfa, 0x00, 0xaa, 0x14, 0x01, 0x03, 0x40, 0xb8, 0x0b,
        ]
    );
    assert_frame_roundtrip(&frame, "genOnOff");
}

// ----------------------------- configReportRsp ------------------------------

#[test]
#[ignore = "not implemented yet"]
fn config_report_rsp_parses_writes() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "configReportRsp",
        "genOnOff",
        json!([
            {
                "status": Status::UNSUPPORTED_ATTRIBUTE as u8,
                "direction": Direction::CLIENT_TO_SERVER as u8,
                "attrId": cluster::genOnOff::attributes::onTime.ID,
            },
            {
                "status": Status::NOT_AUTHORIZED as u8,
                "direction": Direction::SERVER_TO_CLIENT as u8,
                "attrId": cluster::genOnOff::attributes::offWaitTime.ID,
            },
        ]),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(
        raw_frame,
        vec![0x18, 0x0a, 0x07, 0x86, 0x00, 0x01, 0x40, 0x7e, 0x01, 0x02, 0x40]
    );
    assert_frame_roundtrip(&frame, "genOnOff");
}

#[test]
#[ignore = "not implemented yet"]
fn config_report_rsp_handles_success_payload() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "configReportRsp",
        "genOnOff",
        json!([{"status": Status::SUCCESS as u8}]),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(raw_frame, vec![0x18, 0x0a, 0x07, 0x00]);
    assert_frame_roundtrip(&frame, "genOnOff");
}

// ----------------------------- readReportConfig -----------------------------

#[test]
#[ignore = "not implemented yet"]
fn read_report_config_parses_writes() {
    let frame = create_frame(
        Direction::CLIENT_TO_SERVER,
        "readReportConfig",
        "genBasic",
        json!([
            {
                "direction": Direction::CLIENT_TO_SERVER as u8,
                "attrId": cluster::genBasic::attributes::zclVersion.ID,
            },
            {
                "direction": Direction::SERVER_TO_CLIENT as u8,
                "attrId": cluster::genBasic::attributes::modelId.ID,
            },
        ]),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(
        raw_frame,
        vec![0x10, 0x0a, 0x08, 0x00, 0x00, 0x00, 0x01, 0x05, 0x00]
    );
    assert_frame_roundtrip(&frame, "genBasic");
}

// --------------------------- readReportConfigRsp ----------------------------

#[test]
#[ignore = "not implemented yet"]
fn read_report_config_rsp_parses_writes() {
    let frame = create_frame(
        Direction::CLIENT_TO_SERVER,
        "readReportConfigRsp",
        "genBasic",
        json!([
            {
                "status": Status::SUCCESS as u8,
                "direction": Direction::CLIENT_TO_SERVER as u8,
                "attrId": cluster::genBasic::attributes::zclVersion.ID,
                "dataType": cluster::genBasic::attributes::zclVersion.r#type as u8,
                "minRepIntval": 10,
                "maxRepIntval": 100,
                "repChange": 60,
            },
            {
                "status": Status::NOT_AUTHORIZED as u8,
                "direction": Direction::CLIENT_TO_SERVER as u8,
                "attrId": cluster::genBasic::attributes::modelId.ID,
            },
            {
                "status": Status::SUCCESS as u8,
                "direction": Direction::CLIENT_TO_SERVER as u8,
                "attrId": cluster::genBasic::attributes::powerSource.ID,
                "dataType": cluster::genBasic::attributes::powerSource.r#type as u8,
                "minRepIntval": 3500,
                "maxRepIntval": 45000,
            },
            {
                "status": Status::SUCCESS as u8,
                "direction": Direction::SERVER_TO_CLIENT as u8,
                "attrId": cluster::genBasic::attributes::alarmMask.ID,
                "timeout": 4500,
            },
            {
                "status": Status::UNSUPPORTED_ATTRIBUTE as u8,
                "direction": Direction::SERVER_TO_CLIENT as u8,
                "attrId": cluster::genBasic::attributes::physicalEnv.ID,
            },
        ]),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(
        raw_frame,
        vec![
            0x10, 0x0a, 0x09, 0x00, 0x00, 0x00, 0x00, 0x20, 0x0a, 0x00, 0x64, 0x00, 0x3c, 0x7e,
            0x00, 0x05, 0x00, 0x00, 0x00, 0x07, 0x00, 0x30, 0xac, 0x0d, 0xc8, 0xaf, 0x00, 0x01,
            0x13, 0x00, 0x94, 0x11, 0x86, 0x01, 0x11, 0x00,
        ]
    );
    assert_frame_roundtrip(&frame, "genBasic");
}

// --------------------------------- report ----------------------------------

#[test]
#[ignore = "not implemented yet"]
fn report_parses_writes() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "report",
        "genBasic",
        json!([
            {
                "attrId": cluster::genBasic::attributes::zclVersion.ID,
                "dataType": cluster::genBasic::attributes::zclVersion.r#type as u8,
                "attrData": 8,
            },
            {
                "attrId": cluster::genBasic::attributes::manufacturerName.ID,
                "dataType": cluster::genBasic::attributes::manufacturerName.r#type as u8,
                "attrData": "efgh",
            },
        ]),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(
        raw_frame,
        vec![
            0x18, 0x0a, 0x0a, 0x00, 0x00, 0x20, 0x08, 0x04, 0x00, 0x42, 0x04, 0x65, 0x66, 0x67,
            0x68
        ]
    );
    assert_frame_roundtrip(&frame, "genBasic");
}

#[test]
#[ignore = "not implemented yet"]
fn report_workaround_parses_char_str_as_mi_struct_for_attribute_0xff01() {
    let raw_frame = vec![
        0x18, 0x0a, 0x01, 0x01, 0xff, 0x00, 0x42, 34, 1, 33, 213, 12, 3, 40, 33, 4, 33, 168, 19, 5,
        33, 43, 0, 6, 36, 0, 0, 5, 0, 0, 8, 33, 4, 2, 10, 33, 0, 0, 100, 16, 0,
    ];

    let frame = Frame::from_buffer(
        0x0000,
        zigbee_apiarist::zspec::zcl::zcl_header::Header::from_buffer(&raw_frame),
        &raw_frame,
        json!({}),
    );

    assert_eq!(
        frame.payload,
        json!([{
            "attrId": 0xff01,
            "dataType": DataType::CHAR_STR as u8,
            "attrData": {
                "1": 3285,
                "3": 33,
                "4": 5032,
                "5": 43,
                "6": 327680,
                "8": 516,
                "10": 0,
                "100": 0,
            },
        }])
    );
    assert_eq!(frame.cluster.name, "genBasic");
}

// -------------------------------- defaultRsp --------------------------------

#[test]
#[ignore = "not implemented yet"]
fn default_rsp_parses_writes() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "defaultRsp",
        "genBasic",
        json!({
            "cmdId": cluster::genBasic::commands::resetFactDefault.ID,
            "statusCode": Status::SUCCESS as u8,
        }),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(raw_frame, vec![0x18, 0x0a, 0x0b, 0x00, 0x00]);
    assert_frame_roundtrip(&frame, "genBasic");
}

// --------------------------------- discover ---------------------------------

#[test]
#[ignore = "not implemented yet"]
fn discover_parses_writes() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "discover",
        "genBasic",
        json!({"startAttrId": 0x0012, "maxAttrIds": 8}),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(raw_frame, vec![0x18, 0x0a, 0x0c, 0x12, 0x00, 0x08]);
    assert_frame_roundtrip(&frame, "genBasic");
}

#[test]
#[ignore = "not implemented yet"]
fn discover_rsp_parses_writes() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "discoverRsp",
        "genOnOff",
        json!({
            "discComplete": 1,
            "attrInfos": [
                {
                    "attrId": cluster::genOnOff::attributes::onOff.ID,
                    "dataType": cluster::genOnOff::attributes::onOff.r#type as u8,
                },
                {
                    "attrId": cluster::genOnOff::attributes::globalSceneCtrl.ID,
                    "dataType": cluster::genOnOff::attributes::globalSceneCtrl.r#type as u8,
                },
            ],
        }),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(
        raw_frame,
        vec![0x18, 0x0a, 0x0d, 0x01, 0x00, 0x00, 0x10, 0x00, 0x40, 0x10]
    );
    assert_frame_roundtrip(&frame, "genOnOff");
}

// ------------------------------ readStructured ------------------------------

#[test]
#[ignore = "not implemented yet"]
fn read_structured_parses_writes() {
    let frame = create_frame(
        Direction::CLIENT_TO_SERVER,
        "readStructured",
        "genBasic",
        json!([
            {
                "attrId": cluster::genBasic::attributes::zclVersion.ID,
                "selector": {"indicatorType": StructuredIndicatorType::Whole as u8},
            },
            {
                "attrId": cluster::genBasic::attributes::modelId.ID,
                "selector": {"indicatorType": StructuredIndicatorType::Whole as u8},
            },
        ]),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(
        raw_frame,
        vec![0x10, 0x0a, 0x0e, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00]
    );
    assert_frame_roundtrip(&frame, "genBasic");
}

// ----------------------------- writeStructured ------------------------------

#[test]
#[ignore = "not implemented yet"]
fn write_structured_parses_writes() {
    let frame = create_frame(
        Direction::CLIENT_TO_SERVER,
        "writeStructured",
        "genBasic",
        json!([
            {
                "attrId": cluster::genBasic::attributes::zclVersion.ID,
                "selector": {"indicatorType": StructuredIndicatorType::Whole as u8},
                "dataType": cluster::genBasic::attributes::zclVersion.r#type as u8,
                "elementData": 0x08,
            },
            {
                "attrId": cluster::genBasic::attributes::modelId.ID,
                "selector": {"indicatorType": StructuredIndicatorType::Whole as u8},
                "dataType": cluster::genBasic::attributes::modelId.r#type as u8,
                "elementData": "abcd",
            },
        ]),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(
        raw_frame,
        vec![
            0x10, 0x0a, 0x0f, 0x00, 0x00, 0x00, 0x20, 0x08, 0x05, 0x00, 0x00, 0x42, 0x04, 0x61,
            0x62, 0x63, 0x64,
        ]
    );
    assert_frame_roundtrip(&frame, "genBasic");
}

// --------------------------- writeStructuredRsp -----------------------------

#[test]
#[ignore = "not implemented yet"]
fn write_structured_rsp_parses_writes() {
    let frame = create_frame(
        Direction::CLIENT_TO_SERVER,
        "writeStructuredRsp",
        "genBasic",
        json!([
            {
                "status": Status::UNSUPPORTED_ATTRIBUTE as u8,
                "attrId": cluster::genBasic::attributes::zclVersion.ID,
                "selector": {"indicatorType": StructuredIndicatorType::Whole as u8},
            },
            {
                "status": Status::NOT_AUTHORIZED as u8,
                "attrId": cluster::genBasic::attributes::modelId.ID,
                "selector": {"indicatorType": StructuredIndicatorType::Whole as u8},
            },
        ]),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(
        raw_frame,
        vec![0x10, 0x0a, 0x10, 0x86, 0x00, 0x00, 0x00, 0x7e, 0x05, 0x00, 0x00]
    );
    assert_frame_roundtrip(&frame, "genBasic");
}

#[test]
#[ignore = "not implemented yet"]
fn write_structured_rsp_handles_success_payload() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "writeStructuredRsp",
        "genOnOff",
        json!([{"status": Status::SUCCESS as u8}]),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(raw_frame, vec![0x18, 0x0a, 0x10, 0x00]);
    assert_frame_roundtrip(&frame, "genOnOff");
}

// ----------------------------- discoverCommands -----------------------------

#[test]
#[ignore = "not implemented yet"]
fn discover_commands_parses_writes() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "discoverCommands",
        "genOnOff",
        json!({"startCmdId": 0x0010, "maxCmdIds": 3}),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(raw_frame, vec![0x18, 0x0a, 0x11, 0x10, 0x03]);
    assert_frame_roundtrip(&frame, "genOnOff");
}

#[test]
#[ignore = "not implemented yet"]
fn discover_commands_rsp_parses_writes() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "discoverCommandsRsp",
        "genOnOff",
        json!({
            "discComplete": 1,
            "commandIds": [
                cluster::genOnOff::commands::off.ID,
                cluster::genOnOff::commands::on.ID,
            ],
        }),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(raw_frame, vec![0x18, 0x0a, 0x12, 0x01, 0x00, 0x01]);
    assert_frame_roundtrip(&frame, "genOnOff");
}

#[test]
#[ignore = "not implemented yet"]
fn discover_commands_gen_parses_writes() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "discoverCommandsGen",
        "genOnOff",
        json!({"startCmdId": 0x0010, "maxCmdIds": 3}),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(raw_frame, vec![0x18, 0x0a, 0x13, 0x10, 0x03]);
    assert_frame_roundtrip(&frame, "genOnOff");
}

#[test]
#[ignore = "not implemented yet"]
fn discover_commands_gen_rsp_parses_writes() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "discoverCommandsGenRsp",
        "genOnOff",
        json!({
            "discComplete": 1,
            "commandIds": [
                cluster::genOnOff::commands::off.ID,
                cluster::genOnOff::commands::on.ID,
            ],
        }),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(raw_frame, vec![0x18, 0x0a, 0x14, 0x01, 0x00, 0x01]);
    assert_frame_roundtrip(&frame, "genOnOff");
}

// -------------------------------- discoverExt -------------------------------

#[test]
#[ignore = "not implemented yet"]
fn discover_ext_parses_writes() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "discoverExt",
        "genBasic",
        json!({"startAttrId": 0x0012, "maxAttrIds": 8}),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(raw_frame, vec![0x18, 0x0a, 0x15, 0x12, 0x00, 0x08]);
    assert_frame_roundtrip(&frame, "genBasic");
}

#[test]
#[ignore = "not implemented yet"]
fn discover_ext_rsp_parses_writes() {
    let frame = create_frame(
        Direction::SERVER_TO_CLIENT,
        "discoverExtRsp",
        "genOnOff",
        json!({
            "discComplete": 1,
            "attrInfos": [
                {
                    "attrId": cluster::genOnOff::attributes::onOff.ID,
                    "dataType": cluster::genOnOff::attributes::onOff.r#type as u8,
                    "access": 1,
                },
                {
                    "attrId": cluster::genOnOff::attributes::globalSceneCtrl.ID,
                    "dataType": cluster::genOnOff::attributes::globalSceneCtrl.r#type as u8,
                    "access": 2,
                },
            ],
        }),
    );

    let raw_frame = frame.to_buffer();

    assert_eq!(
        raw_frame,
        vec![0x18, 0x0a, 0x16, 0x01, 0x00, 0x00, 0x10, 0x01, 0x00, 0x40, 0x10, 0x02]
    );
    assert_frame_roundtrip(&frame, "genOnOff");
}
