// Original: .upstream/zigbee-herdsman/test/zspec/zcl/frame.test.ts

use serde_json::{json, Value};
use zigbee_apiarist::zspec::zcl::definition::cluster;
use zigbee_apiarist::zspec::zcl::definition::cluster::{Parameter, ParameterCondition};
use zigbee_apiarist::zspec::zcl::definition::enums::DataType;
use zigbee_apiarist::zspec::zcl::definition::foundation::Foundation;
use zigbee_apiarist::zspec::zcl::definition::manufacturer_code::ManufacturerCode;
use zigbee_apiarist::zspec::zcl::definition::status::Status;
use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
use zigbee_apiarist::zspec::zcl::zcl_header::{Direction, FrameControl, FrameType, Header};

fn uint16_to_8_array(value: u16) -> [u8; 2] {
    value.to_le_bytes()
}

fn uint32_to_8_array(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

fn uint56_to_8_array(value: u64) -> [u8; 7] {
    let bytes = value.to_le_bytes();
    [
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
    ]
}

fn frame_control(
    frame_type: FrameType,
    direction: Direction,
    disable_default_response: bool,
    manufacturer_specific: bool,
    reserved_bits: u8,
) -> FrameControl {
    FrameControl {
        reserved_bits,
        frame_type,
        direction,
        disable_default_response,
        manufacturer_specific,
    }
}

// ------------------------------- headers -------------------------------

fn global_header() -> Header {
    Header::new(
        frame_control(
            FrameType::GLOBAL,
            Direction::CLIENT_TO_SERVER,
            false,
            false,
            0,
        ),
        None,
        123,
        Foundation::read.ID,
    )
}

fn global_rsp_header() -> Header {
    Header::new(
        frame_control(
            FrameType::GLOBAL,
            Direction::SERVER_TO_CLIENT,
            false,
            false,
            0,
        ),
        None,
        78,
        Foundation::readRsp.ID,
    )
}

fn global_header_report() -> Header {
    Header::new(
        frame_control(
            FrameType::GLOBAL,
            Direction::CLIENT_TO_SERVER,
            false,
            false,
            0,
        ),
        None,
        123,
        Foundation::report.ID,
    )
}

fn global_condition_header() -> Header {
    Header::new(
        frame_control(
            FrameType::GLOBAL,
            Direction::SERVER_TO_CLIENT,
            false,
            false,
            0,
        ),
        None,
        78,
        Foundation::configReport.ID,
    )
}

fn specific_header() -> Header {
    Header::new(
        frame_control(
            FrameType::SPECIFIC,
            Direction::CLIENT_TO_SERVER,
            false,
            false,
            0,
        ),
        None,
        44,
        cluster::genAlarms::commands::getAlarm.ID,
    )
}

fn specific_condition_header() -> Header {
    Header::new(
        frame_control(
            FrameType::SPECIFIC,
            Direction::SERVER_TO_CLIENT,
            false,
            false,
            0,
        ),
        None,
        45,
        cluster::genOta::commandsResponse::queryNextImageResponse.ID,
    )
}

fn specific_rsp_header() -> Header {
    Header::new(
        frame_control(
            FrameType::SPECIFIC,
            Direction::SERVER_TO_CLIENT,
            false,
            false,
            0,
        ),
        None,
        53,
        cluster::genAlarms::commandsResponse::alarm.ID,
    )
}

fn manuf_spe_header() -> Header {
    Header::new(
        frame_control(
            FrameType::GLOBAL,
            Direction::CLIENT_TO_SERVER,
            false,
            true,
            0,
        ),
        Some(ManufacturerCode::AAC_TECHNOLOGIES_HOLDING),
        234,
        Foundation::read.ID,
    )
}

fn global_header_buffer() -> Vec<u8> {
    vec![0, 123, Foundation::read.ID]
}

fn global_rsp_header_buffer() -> Vec<u8> {
    vec![8, 78, Foundation::readRsp.ID]
}

fn global_header_report_buffer() -> Vec<u8> {
    vec![0, 123, Foundation::report.ID]
}

fn global_condition_header_buffer() -> Vec<u8> {
    vec![8, 78, Foundation::configReport.ID]
}

fn specific_header_buffer() -> Vec<u8> {
    vec![1, 44, cluster::genAlarms::commands::getAlarm.ID]
}

fn specific_condition_header_buffer() -> Vec<u8> {
    vec![
        9,
        45,
        cluster::genOta::commandsResponse::queryNextImageResponse.ID,
    ]
}

fn specific_rsp_header_buffer() -> Vec<u8> {
    vec![9, 53, cluster::genAlarms::commandsResponse::alarm.ID]
}

fn manuf_spe_header_buffer() -> Vec<u8> {
    let mut buffer = vec![4];
    buffer.extend_from_slice(&uint16_to_8_array(
        ManufacturerCode::AAC_TECHNOLOGIES_HOLDING,
    ));
    buffer.push(234);
    buffer.push(Foundation::read.ID);
    buffer
}

// ------------------------------- frames --------------------------------

fn global_frame() -> Frame {
    Frame::create(
        FrameType::GLOBAL as u8,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        None,
        123,
        Foundation::read.ID,
        cluster::genBasic::ID,
        json!([{"attrId": 256}]),
        json!({}),
        0,
    )
}

fn global_frame_buffer() -> Vec<u8> {
    let mut buffer = global_header_buffer();
    buffer.extend_from_slice(&uint16_to_8_array(256));
    buffer
}

const GLOBAL_FRAME_STRING: &str = r#"{"header":{"frameControl":{"reservedBits":0,"frameType":0,"direction":0,"disableDefaultResponse":false,"manufacturerSpecific":false},"transactionSequenceNumber":123,"commandIdentifier":0},"payload":[{"attrId":256}],"command":{"name":"read","ID":0,"response":1}}"#;

fn global_frame_big_int() -> Frame {
    Frame::create(
        FrameType::GLOBAL as u8,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        None,
        123,
        Foundation::report.ID,
        cluster::haApplianceIdentification::ID,
        json!([{
            "attrId": 0,
            "dataType": DataType::UINT56 as u8,
            "attrData": 200,
        }]),
        json!({}),
        0,
    )
}

fn global_frame_big_int_buffer() -> Vec<u8> {
    let mut buffer = global_header_report_buffer();
    buffer.extend_from_slice(&uint16_to_8_array(0));
    buffer.push(DataType::UINT56 as u8);
    buffer.extend_from_slice(&uint56_to_8_array(200));
    buffer
}

const GLOBAL_FRAME_BIG_INT_STRING: &str = r#"{"header":{"frameControl":{"reservedBits":0,"frameType":0,"direction":0,"disableDefaultResponse":false,"manufacturerSpecific":false},"transactionSequenceNumber":123,"commandIdentifier":10},"payload":[{"attrId":0,"dataType":38,"attrData":"200"}],"command":{"name":"report","ID":10}}"#;

fn global_rsp_frame() -> Frame {
    Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        false,
        None,
        78,
        Foundation::readRsp.ID,
        cluster::genPowerCfg::ID,
        json!([{
            "attrId": 256,
            "status": Status::SUCCESS as u8,
            "dataType": DataType::ENUM8 as u8,
            "attrData": 127,
        }]),
        json!({}),
        0,
    )
}

fn global_rsp_frame_buffer() -> Vec<u8> {
    let mut buffer = global_rsp_header_buffer();
    buffer.extend_from_slice(&uint16_to_8_array(256));
    buffer.push(Status::SUCCESS as u8);
    buffer.push(DataType::ENUM8 as u8);
    buffer.push(127);
    buffer
}

const GLOBAL_RSP_FRAME_STRING: &str = r#"{"header":{"frameControl":{"reservedBits":0,"frameType":0,"direction":1,"disableDefaultResponse":false,"manufacturerSpecific":false},"transactionSequenceNumber":78,"commandIdentifier":1},"payload":[{"attrId":256,"status":0,"dataType":48,"attrData":127}],"command":{"name":"readRsp","ID":1}}"#;

fn global_condition_frame() -> Frame {
    Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        false,
        None,
        78,
        Foundation::configReport.ID,
        cluster::genOnOff::ID,
        json!([{
            "direction": Direction::SERVER_TO_CLIENT as u8,
            "attrId": 256,
            "timeout": 10000,
        }]),
        json!({}),
        0,
    )
}

fn global_condition_frame_buffer() -> Vec<u8> {
    let mut buffer = global_condition_header_buffer();
    buffer.push(Direction::SERVER_TO_CLIENT as u8);
    buffer.extend_from_slice(&uint16_to_8_array(256));
    buffer.extend_from_slice(&uint16_to_8_array(10000));
    buffer
}

const GLOBAL_CONDITION_FRAME_STRING: &str = r#"{"header":{"frameControl":{"reservedBits":0,"frameType":0,"direction":1,"disableDefaultResponse":false,"manufacturerSpecific":false},"transactionSequenceNumber":78,"commandIdentifier":6},"payload":[{"direction":1,"attrId":256,"timeout":10000}],"command":{"name":"configReport","ID":6,"response":7}}"#;

fn specific_frame() -> Frame {
    Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        None,
        44,
        cluster::genAlarms::commands::getAlarm.ID,
        cluster::genAlarms::ID,
        json!({}),
        json!({}),
        0,
    )
}

const SPECIFIC_FRAME_STRING: &str = r#"{"header":{"frameControl":{"reservedBits":0,"frameType":1,"direction":0,"disableDefaultResponse":false,"manufacturerSpecific":false},"transactionSequenceNumber":44,"commandIdentifier":2},"payload":{},"command":{"name":"getAlarm","ID":2,"parameters":[]}}"#;

fn specific_rsp_frame() -> Frame {
    Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::SERVER_TO_CLIENT as u8,
        false,
        None,
        53,
        cluster::genAlarms::commandsResponse::alarm.ID,
        cluster::genAlarms::ID,
        json!({"alarmcode": 246, "clusterid": 3456}),
        json!({}),
        0,
    )
}

fn specific_rsp_frame_buffer() -> Vec<u8> {
    let mut buffer = specific_rsp_header_buffer();
    buffer.push(246);
    buffer.extend_from_slice(&uint16_to_8_array(3456));
    buffer
}

const SPECIFIC_RSP_FRAME_STRING: &str = r#"{"header":{"frameControl":{"reservedBits":0,"frameType":1,"direction":1,"disableDefaultResponse":false,"manufacturerSpecific":false},"transactionSequenceNumber":53,"commandIdentifier":0},"payload":{"alarmcode":246,"clusterid":3456},"command":{"name":"alarm","ID":0,"parameters":[{"name":"alarmcode","type":48},{"name":"clusterid","type":232}],"required":true}}"#;

fn specific_condition_frame() -> Frame {
    Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::SERVER_TO_CLIENT as u8,
        false,
        None,
        45,
        cluster::genOta::commandsResponse::queryNextImageResponse.ID,
        cluster::genOta::ID,
        json!({"status": Status::ABORT as u8}),
        json!({}),
        0,
    )
}

fn specific_condition_frame_buffer() -> Vec<u8> {
    let mut buffer = specific_condition_header_buffer();
    buffer.push(Status::ABORT as u8);
    buffer
}

const SPECIFIC_CONDITION_FRAME_STRING: &str = r#"{"header":{"frameControl":{"reservedBits":0,"frameType":1,"direction":1,"disableDefaultResponse":false,"manufacturerSpecific":false},"transactionSequenceNumber":45,"commandIdentifier":2},"payload":{"status":149},"command":{"name":"queryNextImageResponse","ID":2,"parameters":[{"name":"status","type":48},{"name":"manufacturerCode","type":33,"conditions":[{"type":"fieldEquals","field":"status","value":0}]},{"name":"imageType","type":33,"conditions":[{"type":"fieldEquals","field":"status","value":0}],"max":65471},{"name":"fileVersion","type":35,"conditions":[{"type":"fieldEquals","field":"status","value":0}]},{"name":"imageSize","type":35,"conditions":[{"type":"fieldEquals","field":"status","value":0}]}],"required":true}}"#;

fn manuf_spe_frame() -> Frame {
    Frame::create(
        FrameType::GLOBAL as u8,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        Some(ManufacturerCode::AAC_TECHNOLOGIES_HOLDING),
        234,
        Foundation::read.ID,
        0,
        json!([{"attrId": 256}]),
        json!({}),
        0,
    )
}

fn manuf_spe_frame_buffer() -> Vec<u8> {
    let mut buffer = manuf_spe_header_buffer();
    buffer.extend_from_slice(&uint16_to_8_array(256));
    buffer
}

const MANUF_SPE_FRAME_STRING: &str = r#"{"header":{"frameControl":{"reservedBits":0,"frameType":0,"direction":0,"disableDefaultResponse":false,"manufacturerSpecific":true},"manufacturerCode":4344,"transactionSequenceNumber":234,"commandIdentifier":0},"payload":[{"attrId":256}],"command":{"name":"read","ID":0,"response":1}}"#;

fn list_header() -> Header {
    Header::new(
        frame_control(
            FrameType::SPECIFIC,
            Direction::CLIENT_TO_SERVER,
            false,
            false,
            0,
        ),
        None,
        43,
        cluster::genGroups::commands::getMembership.ID,
    )
}

fn list_buffer() -> Vec<u8> {
    vec![
        1,
        43,
        cluster::genGroups::commands::getMembership.ID,
        2,
        10,
        0,
        20,
        0,
    ]
}

fn list_frame() -> Frame {
    Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        None,
        43,
        cluster::genGroups::commands::getMembership.ID,
        cluster::genGroups::ID,
        json!({"groupcount": 2, "grouplist": [10, 20]}),
        json!({}),
        0,
    )
}

const LIST_STRING: &str = r#"{"header":{"frameControl":{"reservedBits":0,"frameType":1,"direction":0,"disableDefaultResponse":false,"manufacturerSpecific":false},"transactionSequenceNumber":43,"commandIdentifier":2},"payload":{"groupcount":2,"grouplist":[10,20]},"command":{"name":"getMembership","ID":2,"response":2,"parameters":[{"name":"groupcount","type":32},{"name":"grouplist","type":1002}],"required":true}}"#;

#[test]
#[ignore = "not implemented yet"]
fn minimum_remaining_buffer_bytes() {
    const CONDITIONS: &[ParameterCondition] =
        &[ParameterCondition::MINIMUM_REMAINING_BUFFER_BYTES(3)];
    let parameter = Parameter {
        name: "direction",
        r#type: DataType::DATA8,
        conditions: CONDITIONS,
        max: None,
    };

    assert!(Frame::conditions_valid(
        &parameter,
        &json!({"status": 1}),
        Some(3)
    ));
    assert!(!Frame::conditions_valid(
        &parameter,
        &json!({"status": 1}),
        Some(2)
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn bitmask_set() {
    let parameter = cluster::greenPower::commands::notification.parameters[8];

    assert!(Frame::conditions_valid(
        &parameter,
        &json!({"options": 0x4000}),
        None
    ));
    assert!(Frame::conditions_valid(
        &parameter,
        &json!({"options": 0x4150}),
        None
    ));
    assert!(!Frame::conditions_valid(
        &parameter,
        &json!({"options": 0x0400}),
        None
    ));
    assert!(!Frame::conditions_valid(
        &parameter,
        &json!({"options": 0x1400}),
        None
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn bitfield_enum() {
    // {param:'options', offset: 0, size: 3, value: 0b000}
    let parameter = cluster::greenPower::commands::notification.parameters[1];

    assert!(Frame::conditions_valid(
        &parameter,
        &json!({"options": 0b000}),
        None
    ));
    assert!(Frame::conditions_valid(
        &parameter,
        &json!({"options": 0b1000}),
        None
    ));
    assert!(!Frame::conditions_valid(
        &parameter,
        &json!({"options": 0b001}),
        None
    ));
    assert!(!Frame::conditions_valid(
        &parameter,
        &json!({"options": 0b011}),
        None
    ));
    assert!(!Frame::conditions_valid(
        &parameter,
        &json!({"options": 0b100}),
        None
    ));
    assert!(!Frame::conditions_valid(
        &parameter,
        &json!({"options": 0b1010}),
        None
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn field_equal() {
    const CONDITIONS: &[ParameterCondition] = &[ParameterCondition::FIELD_EQUAL {
        field: "status",
        value: Status::SUCCESS as u32,
        reversed: false,
    }];
    const REVERSED_CONDITIONS: &[ParameterCondition] = &[ParameterCondition::FIELD_EQUAL {
        field: "status",
        value: Status::SUCCESS as u32,
        reversed: true,
    }];
    let parameter = Parameter {
        name: "dataType",
        r#type: DataType::DATA8,
        conditions: CONDITIONS,
        max: None,
    };
    let reversed_parameter = Parameter {
        name: "dataType",
        r#type: DataType::DATA8,
        conditions: REVERSED_CONDITIONS,
        max: None,
    };

    assert!(Frame::conditions_valid(
        &parameter,
        &json!({"status": 0}),
        None
    ));
    assert!(!Frame::conditions_valid(
        &parameter,
        &json!({"status": 1}),
        None
    ));
    assert!(!Frame::conditions_valid(
        &reversed_parameter,
        &json!({"status": 0}),
        None
    ));
    assert!(Frame::conditions_valid(
        &reversed_parameter,
        &json!({"status": 1}),
        None
    ));

    assert!(Frame::conditions_valid(
        &cluster::touchlink::commandsResponse::scanResponse.parameters[13],
        &json!({"numberOfSubDevices": 1}),
        None
    ));
    assert!(!Frame::conditions_valid(
        &cluster::touchlink::commandsResponse::scanResponse.parameters[13],
        &json!({"numberOfSubDevices": 0}),
        None
    ));
    assert!(!Frame::conditions_valid(
        &cluster::touchlink::commandsResponse::scanResponse.parameters[13],
        &json!({"numberOfSubDevices": 3}),
        None
    ));

    assert!(!Frame::conditions_valid(
        &cluster::genOta::commandsResponse::imageNotify.parameters[2],
        &json!({"payloadType": 0}),
        None
    ));
    assert!(Frame::conditions_valid(
        &cluster::genOta::commandsResponse::imageNotify.parameters[2],
        &json!({"payloadType": 1}),
        None
    ));
    assert!(!Frame::conditions_valid(
        &cluster::genOta::commandsResponse::imageNotify.parameters[3],
        &json!({"payloadType": 1}),
        None
    ));
    assert!(Frame::conditions_valid(
        &cluster::genOta::commandsResponse::imageNotify.parameters[3],
        &json!({"payloadType": 2}),
        None
    ));
    assert!(!Frame::conditions_valid(
        &cluster::genOta::commandsResponse::imageNotify.parameters[4],
        &json!({"payloadType": 2}),
        None
    ));
    assert!(Frame::conditions_valid(
        &cluster::genOta::commandsResponse::imageNotify.parameters[4],
        &json!({"payloadType": 3}),
        None
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn header() {
    struct HeaderCase {
        name: &'static str,
        frame_control: FrameControl,
        manufacturer_code: Option<u16>,
        transaction_sequence_number: u8,
        command_id: u8,
        written: Vec<u8>,
    }

    let cases = vec![
        HeaderCase {
            name: "global",
            frame_control: global_header().frame_control,
            manufacturer_code: global_header().manufacturer_code,
            transaction_sequence_number: global_header().transaction_sequence_number,
            command_id: global_header().command_identifier,
            written: global_header_buffer(),
        },
        HeaderCase {
            name: "global response",
            frame_control: global_rsp_header().frame_control,
            manufacturer_code: global_rsp_header().manufacturer_code,
            transaction_sequence_number: global_rsp_header().transaction_sequence_number,
            command_id: global_rsp_header().command_identifier,
            written: global_rsp_header_buffer(),
        },
        HeaderCase {
            name: "specific",
            frame_control: specific_header().frame_control,
            manufacturer_code: specific_header().manufacturer_code,
            transaction_sequence_number: specific_header().transaction_sequence_number,
            command_id: specific_header().command_identifier,
            written: specific_header_buffer(),
        },
        HeaderCase {
            name: "manufacturer-specific",
            frame_control: manuf_spe_header().frame_control,
            manufacturer_code: manuf_spe_header().manufacturer_code,
            transaction_sequence_number: manuf_spe_header().transaction_sequence_number,
            command_id: manuf_spe_header().command_identifier,
            written: manuf_spe_header_buffer(),
        },
        HeaderCase {
            name: "disable default response",
            frame_control: frame_control(
                FrameType::GLOBAL,
                Direction::CLIENT_TO_SERVER,
                true,
                false,
                0,
            ),
            manufacturer_code: None,
            transaction_sequence_number: 234,
            command_id: 1,
            written: vec![16, 234, 1],
        },
        HeaderCase {
            name: "reserved bits - non-compliant use",
            frame_control: frame_control(
                FrameType::GLOBAL,
                Direction::CLIENT_TO_SERVER,
                false,
                false,
                7,
            ),
            manufacturer_code: None,
            transaction_sequence_number: 234,
            command_id: 1,
            written: vec![224, 234, 1],
        },
        HeaderCase {
            name: "specific manufacturer-specific",
            frame_control: frame_control(
                FrameType::SPECIFIC,
                Direction::CLIENT_TO_SERVER,
                false,
                true,
                0,
            ),
            manufacturer_code: Some(ManufacturerCode::S3C),
            transaction_sequence_number: 234,
            command_id: 1,
            written: {
                let mut buffer = vec![5];
                buffer.extend_from_slice(&uint16_to_8_array(ManufacturerCode::S3C));
                buffer.push(234);
                buffer.push(1);
                buffer
            },
        },
        HeaderCase {
            name: "all non-zero - non-compliant use of reservedBits",
            frame_control: frame_control(
                FrameType::SPECIFIC,
                Direction::SERVER_TO_CLIENT,
                true,
                true,
                3,
            ),
            manufacturer_code: Some(ManufacturerCode::BOSCH_SECURITY_SYSTEMS_INC),
            transaction_sequence_number: 234,
            command_id: 1,
            written: {
                let mut buffer = vec![125];
                buffer.extend_from_slice(&uint16_to_8_array(
                    ManufacturerCode::BOSCH_SECURITY_SYSTEMS_INC,
                ));
                buffer.push(234);
                buffer.push(1);
                buffer
            },
        },
    ];

    for case in cases {
        // write
        {
            let header = Header::new(
                case.frame_control,
                case.manufacturer_code,
                case.transaction_sequence_number,
                case.command_id,
            );
            let buffer = header.to_buffer();

            assert_eq!(buffer, case.written, "case: {}", case.name);
            assert_eq!(header.length, case.written.len(), "case: {}", case.name);
            assert_eq!(
                header.is_global(),
                case.frame_control.frame_type == FrameType::GLOBAL,
                "case: {}",
                case.name
            );
            assert_eq!(
                header.is_specific(),
                case.frame_control.frame_type == FrameType::SPECIFIC,
                "case: {}",
                case.name
            );
        }
        // read
        {
            let header = Header::from_buffer(&case.written).expect("header");
            assert_eq!(header.length, case.written.len(), "case: {}", case.name);
            assert_eq!(
                header.is_global(),
                case.frame_control.frame_type == FrameType::GLOBAL,
                "case: {}",
                case.name
            );
            assert_eq!(
                header.is_specific(),
                case.frame_control.frame_type == FrameType::SPECIFIC,
                "case: {}",
                case.name
            );
            assert_eq!(
                header.frame_control, case.frame_control,
                "case: {}",
                case.name
            );
            assert_eq!(
                header.manufacturer_code, case.manufacturer_code,
                "case: {}",
                case.name
            );
            assert_eq!(
                header.transaction_sequence_number, case.transaction_sequence_number,
                "case: {}",
                case.name
            );
            assert_eq!(
                header.command_identifier, case.command_id,
                "case: {}",
                case.name
            );
        }
    }
}

#[test]
#[ignore = "not implemented yet"]
fn reads_non_compliant_header_as_undefined() {
    // basic
    assert_eq!(Header::from_buffer(&[0, 234]), None);
    // manufacturer specific
    let mut manufacturer_specific = vec![4];
    manufacturer_specific.extend_from_slice(&uint16_to_8_array(1234));
    manufacturer_specific.push(234);
    assert_eq!(Header::from_buffer(&manufacturer_specific), None);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame() {
    struct FrameCase {
        name: &'static str,
        frame: Frame,
        string: &'static str,
        header: Header,
        written: Vec<u8>,
        cluster_id: u16,
    }

    let cases = vec![
        FrameCase {
            name: "global",
            frame: global_frame(),
            string: GLOBAL_FRAME_STRING,
            header: global_header(),
            written: global_frame_buffer(),
            cluster_id: cluster::genBasic::ID,
        },
        FrameCase {
            name: "global BigInt",
            frame: global_frame_big_int(),
            string: GLOBAL_FRAME_BIG_INT_STRING,
            header: global_header_report(),
            written: global_frame_big_int_buffer(),
            cluster_id: cluster::haApplianceIdentification::ID,
        },
        FrameCase {
            name: "global response",
            frame: global_rsp_frame(),
            string: GLOBAL_RSP_FRAME_STRING,
            header: global_rsp_header(),
            written: global_rsp_frame_buffer(),
            cluster_id: cluster::genPowerCfg::ID,
        },
        FrameCase {
            name: "global with condition-based parameters",
            frame: global_condition_frame(),
            string: GLOBAL_CONDITION_FRAME_STRING,
            header: global_condition_header(),
            written: global_condition_frame_buffer(),
            cluster_id: cluster::genOnOff::ID,
        },
        FrameCase {
            name: "specific",
            frame: specific_frame(),
            string: SPECIFIC_FRAME_STRING,
            header: specific_header(),
            written: specific_header_buffer(),
            cluster_id: cluster::genAlarms::ID,
        },
        FrameCase {
            name: "specific response",
            frame: specific_rsp_frame(),
            string: SPECIFIC_RSP_FRAME_STRING,
            header: specific_rsp_header(),
            written: specific_rsp_frame_buffer(),
            cluster_id: cluster::genAlarms::ID,
        },
        FrameCase {
            name: "specific with condition-based parameters",
            frame: specific_condition_frame(),
            string: SPECIFIC_CONDITION_FRAME_STRING,
            header: specific_condition_header(),
            written: specific_condition_frame_buffer(),
            cluster_id: cluster::genOta::ID,
        },
        FrameCase {
            name: "manufacturer-specific",
            frame: manuf_spe_frame(),
            string: MANUF_SPE_FRAME_STRING,
            header: manuf_spe_header(),
            written: manuf_spe_frame_buffer(),
            cluster_id: 0,
        },
        FrameCase {
            name: "list",
            frame: list_frame(),
            string: LIST_STRING,
            header: list_header(),
            written: list_buffer(),
            cluster_id: cluster::genGroups::ID,
        },
    ];

    for case in cases {
        assert_eq!(case.frame.to_string(), case.string, "case: {}", case.name);
        assert_eq!(case.frame.header, case.header, "case: {}", case.name);
        assert_eq!(case.frame.to_buffer(), case.written, "case: {}", case.name);
        assert_eq!(
            Frame::from_buffer(case.cluster_id, Some(case.header), &case.written, json!({}))
                .to_string(),
            case.string,
            "case: {}",
            case.name
        );
    }
}

#[test]
#[ignore = "not implemented yet"]
fn writes_reads_repetitive_strategy() {
    let expected = json!([{"attrId": 256}, {"attrId": 127}]);
    let header = Header::new(
        frame_control(
            FrameType::GLOBAL,
            Direction::CLIENT_TO_SERVER,
            false,
            false,
            0,
        ),
        None,
        123,
        Foundation::read.ID,
    );
    let frame = Frame::create(
        header.frame_control.frame_type as u8,
        header.frame_control.direction as u8,
        header.frame_control.disable_default_response,
        header.manufacturer_code,
        header.transaction_sequence_number,
        header.command_identifier,
        0,
        expected.clone(),
        json!({}),
        header.frame_control.reserved_bits,
    );

    assert_eq!(frame.payload, expected);

    let buffer = frame.to_buffer();
    let read_header = Header::from_buffer(&buffer).expect("header");
    let read_frame = Frame::from_buffer(0, Some(read_header), &buffer, json!({}));

    assert_eq!(read_frame.payload, expected);
}

#[test]
#[ignore = "not implemented yet"]
fn writes_reads_flat_strategy() {
    let expected = json!({"cmdId": 253, "statusCode": 127});
    let header = Header::new(
        frame_control(
            FrameType::GLOBAL,
            Direction::CLIENT_TO_SERVER,
            false,
            false,
            0,
        ),
        None,
        123,
        Foundation::defaultRsp.ID,
    );
    let frame = Frame::create(
        header.frame_control.frame_type as u8,
        header.frame_control.direction as u8,
        header.frame_control.disable_default_response,
        header.manufacturer_code,
        header.transaction_sequence_number,
        header.command_identifier,
        0,
        expected.clone(),
        json!({}),
        header.frame_control.reserved_bits,
    );

    assert_eq!(frame.payload, expected);

    let buffer = frame.to_buffer();
    let read_header = Header::from_buffer(&buffer).expect("header");
    let read_frame = Frame::from_buffer(0, Some(read_header), &buffer, json!({}));

    assert_eq!(read_frame.payload, expected);
}

#[test]
#[ignore = "not implemented yet"]
fn writes_reads_oneof_strategy() {
    let expected = json!({
        "discComplete": 123,
        "attrInfos": [
            {"attrId": 32, "dataType": DataType::UINT16 as u8},
            {"attrId": 67, "dataType": DataType::ARRAY as u8},
        ],
    });
    let header = Header::new(
        frame_control(
            FrameType::GLOBAL,
            Direction::CLIENT_TO_SERVER,
            false,
            false,
            0,
        ),
        None,
        123,
        Foundation::discoverRsp.ID,
    );
    let frame = Frame::create(
        header.frame_control.frame_type as u8,
        header.frame_control.direction as u8,
        header.frame_control.disable_default_response,
        header.manufacturer_code,
        header.transaction_sequence_number,
        header.command_identifier,
        0,
        expected.clone(),
        json!({}),
        header.frame_control.reserved_bits,
    );

    assert_eq!(frame.payload, expected);

    let buffer = frame.to_buffer();
    let read_header = Header::from_buffer(&buffer).expect("header");
    let read_frame = Frame::from_buffer(0, Some(read_header), &buffer, json!({}));

    assert_eq!(read_frame.payload, expected);
}

#[test]
#[ignore = "not implemented yet"]
#[should_panic(expected = "Frametype '3' not valid")]
fn throws_when_writting_invalid_frame_type() {
    let frame = Frame::create(
        3,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        None,
        123,
        cluster::genAlarms::commands::reset.ID,
        cluster::genAlarms::ID,
        json!({}),
        json!({}),
        0,
    );

    frame.to_buffer();
}

#[test]
#[ignore = "not implemented yet"]
#[should_panic(expected = "Unsupported frameType '2'")]
fn throws_when_reading_invalid_frame_type() {
    let header = Header::from_buffer(&[2, 123, 0]);
    Frame::from_buffer(cluster::genBasic::ID, header, &[], json!({}));
}

#[test]
#[ignore = "not implemented yet"]
#[should_panic(expected = "Invalid ZclHeader.")]
fn throws_when_reading_frame_without_header() {
    Frame::from_buffer(cluster::genBasic::ID, None, &[], json!({}));
}

#[test]
#[ignore = "not implemented yet"]
#[should_panic(expected = "Parameter 'clusterid' is missing")]
fn throws_when_writting_missing_payload() {
    let frame = Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::SERVER_TO_CLIENT as u8,
        false,
        None,
        53,
        cluster::genAlarms::commandsResponse::alarm.ID,
        cluster::genAlarms::ID,
        json!({"alarmcode": 246}),
        json!({}),
        0,
    );

    frame.to_buffer();
}

#[test]
#[ignore = "not implemented yet"]
fn checks_cluster_match_by_name() {
    let global_frame = global_frame();
    assert!(global_frame.is_cluster("genBasic"));
    assert!(!global_frame.is_cluster("discoverCommands"));
    assert!(!global_frame.is_cluster("notacluster"));

    let specific_frame = specific_frame();
    assert!(specific_frame.is_cluster("genAlarms"));
    assert!(!specific_frame.is_cluster("genAnalogInput"));
    assert!(!specific_frame.is_cluster("notacluster"));
}

#[test]
#[ignore = "not implemented yet"]
fn checks_command_match_by_name() {
    let global_frame = global_frame();
    assert!(global_frame.is_command("read"));
    assert!(!global_frame.is_command("discoverCommands"));
    assert!(!global_frame.is_command("notacommand"));

    let specific_frame = specific_frame();
    assert!(specific_frame.is_command("getAlarm"));
    assert!(!specific_frame.is_command("enrollReq"));
    assert!(!specific_frame.is_command("notacommand"));
}

#[test]
#[ignore = "not implemented yet"]
fn workaround_reads_foundation_char_str_as_mi_struct_for_xiaomi_attrid_id_65281() {
    let expected = json!([
        {"attrId": 5, "dataType": DataType::CHAR_STR as u8, "attrData": "lumi.sensor_wleak.aq1"},
        {
            "attrId": 65281,
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
        },
    ]);
    let mut buffer = vec![28];
    buffer.extend_from_slice(&uint16_to_8_array(
        ManufacturerCode::LUMI_UNITED_TECHOLOGY_LTD_SHENZHEN,
    ));
    buffer.extend_from_slice(&[
        3,
        10, /*header*/
        // regular attr parsing
        5,
        0,
        DataType::CHAR_STR as u8,
        21,
        108,
        117,
        109,
        105,
        46,
        115,
        101,
        110,
        115,
        111,
        114,
        95,
        119,
        108,
        101,
        97,
        107,
        46,
        97,
        113,
        49,
        // workaround parsing
        1,
        255,
        DataType::CHAR_STR as u8,
        34,
        1,
        DataType::UINT16 as u8,
    ]);
    buffer.extend_from_slice(&uint16_to_8_array(3285));
    buffer.extend_from_slice(&[3, DataType::INT8 as u8, 33, 4, DataType::UINT16 as u8]);
    buffer.extend_from_slice(&uint16_to_8_array(5032));
    buffer.extend_from_slice(&[5, DataType::UINT16 as u8]);
    buffer.extend_from_slice(&uint16_to_8_array(43));
    buffer.extend_from_slice(&[6, DataType::UINT40 as u8]);
    buffer.extend_from_slice(&uint32_to_8_array(327680));
    buffer.extend_from_slice(&[0, 8, DataType::UINT16 as u8]);
    buffer.extend_from_slice(&uint16_to_8_array(516));
    buffer.extend_from_slice(&[10, DataType::UINT16 as u8]);
    buffer.extend_from_slice(&uint16_to_8_array(0));
    buffer.extend_from_slice(&[100, DataType::BOOLEAN as u8, 0]);

    let header = Header::from_buffer(&buffer).expect("header");
    let frame = Frame::from_buffer(0, Some(header), &buffer, json!({}));

    assert_eq!(frame.payload, expected);
}

#[test]
#[ignore = "not implemented yet"]
fn workaround_reads_foundation_struct_with_extra_payload_to_match_zcl_packet() {
    let expected = json!([{
        "attrId": 65282,
        "dataType": DataType::STRUCT as u8,
        "attrData": [
            {"elmType": DataType::BOOLEAN as u8, "elmVal": 1},
            {"elmType": DataType::UINT16 as u8, "elmVal": 3022},
            {"elmType": DataType::UINT16 as u8, "elmVal": 17320},
            {"elmType": DataType::UINT40 as u8, "elmVal": 1},
            {"elmType": DataType::UINT16 as u8, "elmVal": 560},
            {"elmType": DataType::UINT8 as u8, "elmVal": 86},
        ],
    }]);
    let mut buffer = vec![28];
    buffer.extend_from_slice(&uint16_to_8_array(ManufacturerCode::DSR_CORPORATION));
    buffer.extend_from_slice(&[194, 10 /*header*/]);
    buffer.extend_from_slice(&uint16_to_8_array(65282));
    buffer.push(DataType::STRUCT as u8);
    buffer.push(6);
    buffer.extend_from_slice(&[0, DataType::BOOLEAN as u8, 1, DataType::UINT16 as u8]);
    buffer.extend_from_slice(&uint16_to_8_array(3022));
    buffer.push(DataType::UINT16 as u8);
    buffer.extend_from_slice(&uint16_to_8_array(17320));
    buffer.push(DataType::UINT40 as u8);
    buffer.extend_from_slice(&uint32_to_8_array(1));
    buffer.extend_from_slice(&[0, DataType::UINT16 as u8]);
    buffer.extend_from_slice(&uint16_to_8_array(560));
    buffer.extend_from_slice(&[DataType::UINT8 as u8, 86]);

    let header = Header::from_buffer(&buffer).expect("header");
    let frame = Frame::from_buffer(0, Some(header), &buffer, json!({}));

    assert_eq!(frame.payload, expected);
}

#[test]
#[ignore = "not implemented yet"]
fn reads_writes_touchlink_scan_response_with_different_size_payloads() {
    let full = vec![
        0x19, 0x0, 0x1, 0xe0, 0xde, 0x5e, 0x2f, 0xa, 0x5, 0x0, 0x12, 0x0, 0x3d, 0x30, 0x1d, 0x4a,
        0x8f, 0xb7, 0xdc, 0x1c, 0x0, 0x4b, 0x12, 0x0, 0x0, 0xf, 0x62, 0x1a, 0xb3, 0xaa, 0x1, 0x0,
        0xb, 0x5e, 0xc0, 0x10, 0x2, 0x2, 0x0,
    ];
    let short = vec![
        0x19, 0x0, 0x1, 0xe0, 0xde, 0x5e, 0x2f, 0xa, 0x5, 0x0, 0x12, 0x0, 0x3d, 0x30, 0x1d, 0x4a,
        0x8f, 0xb7, 0xdc, 0x1c, 0x0, 0x4b, 0x12, 0x0, 0x0, 0xf, 0x62, 0x1a, 0xb3, 0xaa, 0x0, 0x0,
    ];
    let short2 = vec![
        0x19, 0x0, 0x1, 0xe0, 0xde, 0x5e, 0x2f, 0xa, 0x5, 0x0, 0x12, 0x0, 0x3d, 0x30, 0x1d, 0x4a,
        0x8f, 0xb7, 0xdc, 0x1c, 0x0, 0x4b, 0x12, 0x0, 0x0, 0xf, 0x62, 0x1a, 0xb3, 0xaa, 0x3, 0x0,
    ];

    let full_zcl = Frame::from_buffer(0x1000, Header::from_buffer(&full), &full, json!({}));
    assert_eq!(full_zcl.payload["numberOfSubDevices"], json!(1));
    assert_eq!(full_zcl.payload["endpointID"], json!(11));
    assert_eq!(full_zcl.payload["profileID"], json!(0xc05e));
    assert_eq!(full_zcl.payload["deviceID"], json!(0x0210));
    assert_eq!(full_zcl.payload["version"], json!(0x02));
    assert_eq!(full_zcl.payload["groupIDCount"], json!(0));

    let short_zcl = Frame::from_buffer(0x1000, Header::from_buffer(&short), &short, json!({}));
    assert_eq!(short_zcl.payload["numberOfSubDevices"], json!(0));
    assert_eq!(short_zcl.payload.get("endpointID"), None);
    assert_eq!(short_zcl.payload.get("profileID"), None);
    assert_eq!(short_zcl.payload.get("deviceID"), None);
    assert_eq!(short_zcl.payload.get("version"), None);
    assert_eq!(short_zcl.payload.get("groupIDCount"), None);

    let short2_zcl = Frame::from_buffer(0x1000, Header::from_buffer(&short2), &short2, json!({}));
    assert_eq!(short2_zcl.payload["numberOfSubDevices"], json!(3));
    assert_eq!(short2_zcl.payload.get("endpointID"), None);
    assert_eq!(short2_zcl.payload.get("profileID"), None);
    assert_eq!(short2_zcl.payload.get("deviceID"), None);
    assert_eq!(short2_zcl.payload.get("version"), None);
    assert_eq!(short2_zcl.payload.get("groupIDCount"), None);

    assert_eq!(full_zcl.to_buffer(), full);
    assert_eq!(short_zcl.to_buffer(), short);
    assert_eq!(short2_zcl.to_buffer(), short2);
}

#[test]
#[ignore = "not implemented yet"]
#[should_panic(expected = "INVALID_FIELD")]
fn throws_when_payload_too_short_to_continue_reading() {
    let buffer = vec![1, 3, 3, 0, 33, 16, 18, 0, 255, 69, 81, 0, 62, 0, 0, 0];

    Frame::from_buffer(
        cluster::genOta::ID,
        Header::from_buffer(&buffer),
        &buffer,
        json!({}),
    );
}

#[test]
#[ignore = "not implemented yet"]
fn workaround_reads_gen_ota_image_block_request_even_if_minimum_block_period_is_missing() {
    let buffer = vec![1, 3, 3, 2, 33, 16, 18, 0, 255, 69, 81, 0, 62, 0, 0, 0, 8];
    let frame = Frame::from_buffer(
        cluster::genOta::ID,
        Header::from_buffer(&buffer),
        &buffer,
        json!({}),
    );

    assert_eq!(
        frame.payload,
        json!({
            "fieldControl": 2,
            "manufacturerCode": 4129,
            "imageType": 18,
            "fileVersion": 5326335,
            "fileOffset": 62,
            "maximumDataSize": 8,
        })
    );
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_minimum_remaining_buffer_bytes_parameters_when_present() {
    // moveToLevelWithOnOff with optionsMask/optionsOverride
    let payload = json!({"level": 150, "transtime": 0, "optionsMask": 0, "optionsOverride": 1});
    let frame = Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        None,
        1,
        "moveToLevelWithOnOff",
        cluster::genLevelCtrl::ID,
        payload,
        json!({}),
        0,
    );

    let buffer = frame.to_buffer();
    assert_eq!(buffer, vec![0x01, 0x01, 0x04, 0x96, 0x00, 0x00, 0x00, 0x01]);
}

#[test]
#[ignore = "not implemented yet"]
fn allows_omitting_minimum_remaining_buffer_bytes_parameters() {
    // moveToLevelWithOnOff without optionsMask/optionsOverride
    let payload = json!({"level": 150, "transtime": 0});
    let frame = Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::CLIENT_TO_SERVER as u8,
        false,
        None,
        1,
        "moveToLevelWithOnOff",
        cluster::genLevelCtrl::ID,
        payload,
        json!({}),
        0,
    );

    let buffer = frame.to_buffer();
    assert_eq!(buffer, vec![0x01, 0x01, 0x04, 0x96, 0x00, 0x00]);
}

#[test]
#[ignore = "not implemented yet"]
fn allows_omitting_parameters_with_false_conditions() {
    // Touchlink scanResponse without sub-device fields when numberOfSubDevices != 1
    let payload = json!({
        "transactionID": 1,
        "rssiCorrection": 10,
        "zigbeeInformation": 5,
        "touchlinkInformation": 3,
        "keyBitmask": 15,
        "responseID": 100,
        "extendedPanID": "0x0011223344556677",
        "networkUpdateID": 1,
        "logicalChannel": 11,
        "panID": 0x1234,
        "networkAddress": 0x5678,
        "numberOfSubDevices": 0,
        "totalGroupIdentifiers": 0,
    });

    let frame = Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::SERVER_TO_CLIENT as u8,
        false,
        None,
        2,
        "scanResponse",
        cluster::touchlink::ID,
        payload,
        json!({}),
        0,
    );

    let buffer = frame.to_buffer();
    assert_eq!(
        buffer,
        vec![
            0x09, 0x02, 0x01, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x05, 0x03, 0x0f, 0x00, 0x64, 0x00,
            0x00, 0x00, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, 0x00, 0x01, 0x0b, 0x34, 0x12,
            0x78, 0x56, 0x00, 0x00,
        ]
    );
}

#[test]
#[ignore = "not implemented yet"]
#[should_panic(expected = "Parameter 'manufacturerCode' is missing")]
fn requires_parameters_when_conditions_are_true() {
    // queryNextImageResponse with status=SUCCESS requires manufacturerCode
    let payload = json!({"status": 0}); // SUCCESS but missing required fields

    Frame::create(
        FrameType::SPECIFIC as u8,
        Direction::SERVER_TO_CLIENT as u8,
        false,
        None,
        3,
        "queryNextImageResponse",
        cluster::genOta::ID,
        payload,
        json!({}),
        0,
    )
    .to_buffer();
}
