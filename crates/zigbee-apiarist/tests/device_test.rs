// Original: .upstream/zigbee-herdsman/test/device.test.ts

use serde_json::{json, Value};
use zigbee_apiarist::controller::model::device::{Device, ZclPayload};
use zigbee_apiarist::zspec::zcl::definition::cluster;
use zigbee_apiarist::zspec::zcl::definition::enums::DataType;
use zigbee_apiarist::zspec::zcl::definition::status::Status;
use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
use zigbee_apiarist::zspec::zcl::zcl_header::{Direction, FrameType};

fn device() -> Device {
    let mut device = Device::create(
        "Router",
        "0xfe34ac2385ff8311",
        0x0001,
        0x0102,
        "Herdsman",
        "Mains (single phase)",
        "Herd-01",
        "Successful",
        None,
    );
    device.create_endpoint(1);
    device
}

fn payload(frame: &Frame) -> ZclPayload {
    ZclPayload {
        cluster_id: frame.cluster.ID,
        address: 0x1234,
        data: frame.to_buffer(),
        endpoint: 1,
        linkquality: 150,
        group_id: 0,
        was_broadcast: false,
        destination_endpoint: 1,
    }
}

fn frame(
    frame_type: FrameType,
    direction: Direction,
    command: &'static str,
    cluster_id: u16,
    payload: Value,
) -> Frame {
    Frame::create(
        frame_type as u8,
        direction as u8,
        false,
        None,
        1,
        command,
        cluster_id,
        payload,
        json!({}),
        0,
    )
}

#[tokio::test]
async fn replies_to_basic_read() {
    let mut device = device();
    let frame = frame(
        FrameType::GLOBAL,
        Direction::CLIENT_TO_SERVER,
        "read",
        cluster::genBasic::ID,
        json!([{"attrId": 0x0000}, {"attrId": 0xfffa}]),
    );

    device
        .on_zcl_data(&payload(&frame), &frame, 0, Value::Null)
        .await;

    assert_eq!(device.endpoints[0].read_response_calls.len(), 1);
    assert_eq!(device.endpoints[0].command_calls.len(), 0);
    assert_eq!(device.endpoints[0].read_calls.len(), 0);
    assert_eq!(device.endpoints[0].default_response_calls.len(), 0);
    let call = &device.endpoints[0].read_response_calls[0];
    assert_eq!(call[0], json!(cluster::genBasic::ID));
    assert_eq!(call[1], json!(1));
    assert_eq!(
        call[2]["65530"],
        json!({"value": null, "type": DataType::NO_DATA as u8})
    );
}

#[tokio::test]
async fn replies_to_time_read() {
    let mut device = device();
    let frame = frame(
        FrameType::GLOBAL,
        Direction::CLIENT_TO_SERVER,
        "read",
        cluster::genTime::ID,
        json!([{"attrId": 0x0000}, {"attrId": 0xfffa}]),
    );

    device
        .on_zcl_data(&payload(&frame), &frame, 0, Value::Null)
        .await;

    assert_eq!(device.endpoints[0].read_response_calls.len(), 1);
    assert_eq!(device.endpoints[0].command_calls.len(), 0);
    assert_eq!(device.endpoints[0].read_calls.len(), 0);
    assert_eq!(device.endpoints[0].default_response_calls.len(), 0);
    assert_eq!(
        device.endpoints[0].read_response_calls[0][0],
        json!(cluster::genTime::ID)
    );
}

#[tokio::test]
async fn replies_to_unsupported_read() {
    let mut device = device();
    let frame = frame(
        FrameType::GLOBAL,
        Direction::CLIENT_TO_SERVER,
        "read",
        cluster::genGroups::ID,
        json!([{"attrId": 0x0000}]),
    );

    device
        .on_zcl_data(&payload(&frame), &frame, 0, Value::Null)
        .await;

    assert_eq!(device.endpoints[0].read_response_calls.len(), 1);
    assert_eq!(device.endpoints[0].command_calls.len(), 0);
    assert_eq!(device.endpoints[0].read_calls.len(), 0);
    assert_eq!(device.endpoints[0].default_response_calls.len(), 0);
    assert_eq!(
        device.endpoints[0].read_response_calls[0][2]["0"],
        json!({"value": null, "type": DataType::NO_DATA as u8})
    );
}

#[tokio::test]
async fn replies_to_read_with_custom_response() {
    let mut device = device();
    device.custom_read_response = Some(false);
    let frame = frame(
        FrameType::GLOBAL,
        Direction::CLIENT_TO_SERVER,
        "read",
        cluster::genBasic::ID,
        json!([{"attrId": 0x0000}]),
    );

    device
        .on_zcl_data(&payload(&frame), &frame, 0, Value::Null)
        .await;

    assert_eq!(device.endpoints[0].read_response_calls.len(), 1);
    assert_eq!(device.endpoints[0].command_calls.len(), 0);
    assert_eq!(device.endpoints[0].read_calls.len(), 0);
    assert_eq!(device.endpoints[0].default_response_calls.len(), 0);
}

#[tokio::test]
async fn replies_to_read_with_override_custom_response() {
    let mut device = device();
    device.custom_read_response = Some(true);
    let frame = frame(
        FrameType::GLOBAL,
        Direction::CLIENT_TO_SERVER,
        "read",
        cluster::genBasic::ID,
        json!([{"attrId": 0x0000, "type": DataType::UINT8 as u8}]),
    );

    device
        .on_zcl_data(&payload(&frame), &frame, 0, Value::Null)
        .await;

    assert_eq!(device.endpoints[0].read_response_calls.len(), 0);
    assert_eq!(device.endpoints[0].command_calls.len(), 0);
    assert_eq!(device.endpoints[0].read_calls.len(), 0);
    assert_eq!(device.endpoints[0].default_response_calls.len(), 0);
}

#[tokio::test]
async fn does_not_send_default_response_for_default_response() {
    let mut device = device();
    let frame = frame(
        FrameType::GLOBAL,
        Direction::CLIENT_TO_SERVER,
        "defaultRsp",
        cluster::genBasic::ID,
        json!({"cmdId": 0x00, "statusCode": Status::SUCCESS as u8}),
    );

    device
        .on_zcl_data(&payload(&frame), &frame, 0, Value::Null)
        .await;

    assert_eq!(device.endpoints[0].read_response_calls.len(), 0);
    assert_eq!(device.endpoints[0].command_calls.len(), 0);
    assert_eq!(device.endpoints[0].read_calls.len(), 0);
    assert_eq!(device.endpoints[0].default_response_calls.len(), 0);
}

#[tokio::test]
async fn replies_to_ias_zone_enroll_request() {
    let mut device = device();
    let frame = frame(
        FrameType::SPECIFIC,
        Direction::SERVER_TO_CLIENT,
        "enrollReq",
        cluster::ssIasZone::ID,
        json!({"zonetype": 0x4567, "manucode": 0x1234}),
    );

    device
        .on_zcl_data(&payload(&frame), &frame, 0, Value::Null)
        .await;

    assert_eq!(device.endpoints[0].read_response_calls.len(), 0);
    assert_eq!(device.endpoints[0].command_calls.len(), 1);
    assert_eq!(
        device.endpoints[0].command_calls[0],
        json!([
            "ssIasZone",
            "enrollRsp",
            {"enrollrspcode": 0, "zoneid": 23},
            {"transactionSequenceNumber": 1, "disableDefaultResponse": true},
        ])
    );
    assert_eq!(device.endpoints[0].read_calls.len(), 0);
    assert_eq!(device.endpoints[0].default_response_calls.len(), 0);
}

#[tokio::test]
async fn fails_to_reply_to_ias_zone_enroll_request() {
    let mut device = device();
    device.endpoints[0].fail_next_command = true;
    let frame = frame(
        FrameType::SPECIFIC,
        Direction::SERVER_TO_CLIENT,
        "enrollReq",
        cluster::ssIasZone::ID,
        json!({"zonetype": 0x4567, "manucode": 0x1234}),
    );

    device
        .on_zcl_data(&payload(&frame), &frame, 0, Value::Null)
        .await;

    assert_eq!(device.endpoints[0].read_response_calls.len(), 0);
    assert_eq!(device.endpoints[0].command_calls.len(), 1);
    assert_eq!(device.endpoints[0].read_calls.len(), 0);
    assert_eq!(device.endpoints[0].default_response_calls.len(), 1);
    assert_eq!(
        device.endpoints[0].default_response_calls[0],
        json!([frame.command.ID, Status::FAILURE as u8, frame.cluster.ID, 1, {"direction": Direction::CLIENT_TO_SERVER as u8}])
    );
}

#[tokio::test]
async fn replies_to_poll_control_checkin() {
    let mut device = device();
    device.checkin_interval = None;
    device.endpoints[0].next_read_result = json!({"checkinInterval": 100});
    let frame = frame(
        FrameType::SPECIFIC,
        Direction::SERVER_TO_CLIENT,
        "checkin",
        cluster::genPollCtrl::ID,
        json!({}),
    );

    device
        .on_zcl_data(&payload(&frame), &frame, 0, Value::Null)
        .await;

    assert_eq!(device.endpoints[0].read_response_calls.len(), 0);
    assert_eq!(device.endpoints[0].command_calls.len(), 2);
    assert_eq!(
        device.endpoints[0].command_calls[0],
        json!([
            "genPollCtrl",
            "checkinRsp",
            {"startFastPolling": 1, "fastPollTimeout": 0},
            {"transactionSequenceNumber": 1, "disableDefaultResponse": true, "sendPolicy": "immediate"},
        ])
    );
    assert_eq!(
        device.endpoints[0].command_calls[1],
        json!(["genPollCtrl", "fastPollStop", {}, {"sendPolicy": "immediate"}])
    );
    assert_eq!(device.endpoints[0].read_calls.len(), 1);

    device.endpoints[0].read_response_calls.clear();
    device.endpoints[0].command_calls.clear();
    device.endpoints[0].read_calls.clear();
    device.endpoints[0].default_response_calls.clear();
    device
        .on_zcl_data(&payload(&frame), &frame, 0, Value::Null)
        .await;

    assert_eq!(device.endpoints[0].command_calls.len(), 1);
    assert_eq!(
        device.endpoints[0].command_calls[0],
        json!([
            "genPollCtrl",
            "checkinRsp",
            {"startFastPolling": 0, "fastPollTimeout": 0},
            {"transactionSequenceNumber": 1, "disableDefaultResponse": true, "sendPolicy": "immediate"},
        ])
    );
    assert_eq!(device.endpoints[0].read_calls.len(), 0);
    assert_eq!(device.endpoints[0].default_response_calls.len(), 0);
}

#[tokio::test]
async fn fails_to_reply_to_poll_control_checkin() {
    let mut device = device();
    device.checkin_interval = None;
    device.endpoints[0].fail_next_command = true;
    let frame = frame(
        FrameType::SPECIFIC,
        Direction::SERVER_TO_CLIENT,
        "checkin",
        cluster::genPollCtrl::ID,
        json!({}),
    );

    device
        .on_zcl_data(&payload(&frame), &frame, 0, Value::Null)
        .await;

    assert_eq!(device.endpoints[0].command_calls.len(), 1);
    assert_eq!(device.endpoints[0].read_calls.len(), 0);
    assert_eq!(device.endpoints[0].default_response_calls.len(), 1);
}

#[tokio::test]
async fn fails_to_stop_fast_poll() {
    let mut device = device();
    device.checkin_interval = None;
    device.endpoints[0].next_read_result = json!({"checkinInterval": 100});
    device.endpoints[0].fail_next_command = true;
    let frame = frame(
        FrameType::SPECIFIC,
        Direction::SERVER_TO_CLIENT,
        "checkin",
        cluster::genPollCtrl::ID,
        json!({}),
    );

    device
        .on_zcl_data(&payload(&frame), &frame, 0, Value::Null)
        .await;

    assert_eq!(device.endpoints[0].command_calls.len(), 2);
    assert_eq!(device.endpoints[0].read_calls.len(), 1);
    assert_eq!(device.endpoints[0].default_response_calls.len(), 0);
}

#[tokio::test]
async fn replies_to_global_with_success_default_response_by_default() {
    let mut device = device();
    let frame = frame(
        FrameType::GLOBAL,
        Direction::SERVER_TO_CLIENT,
        "readRsp",
        cluster::genOnOff::ID,
        json!([{"attrId": 0x0000, "status": Status::SUCCESS as u8, "dataType": DataType::BOOLEAN as u8, "attrData": 1}]),
    );

    device
        .on_zcl_data(&payload(&frame), &frame, 0, Value::Null)
        .await;

    assert_eq!(device.endpoints[0].read_response_calls.len(), 0);
    assert_eq!(device.endpoints[0].command_calls.len(), 0);
    assert_eq!(device.endpoints[0].read_calls.len(), 0);
    assert_eq!(device.endpoints[0].default_response_calls.len(), 1);
    assert_eq!(
        device.endpoints[0].default_response_calls[0],
        json!([frame.command.ID, Status::SUCCESS as u8, frame.cluster.ID, 1, {"direction": Direction::CLIENT_TO_SERVER as u8}])
    );
}

#[tokio::test]
async fn replies_to_specific_from_server_with_success_default_response_by_default() {
    let mut device = device();
    let frame = frame(
        FrameType::SPECIFIC,
        Direction::SERVER_TO_CLIENT,
        "removeRsp",
        cluster::genScenes::ID,
        json!({"status": 0, "groupid": 1, "sceneid": 2}),
    );

    device
        .on_zcl_data(&payload(&frame), &frame, 0, Value::Null)
        .await;

    assert_eq!(device.endpoints[0].default_response_calls.len(), 1);
}

#[tokio::test]
async fn replies_to_specific_from_client_with_success_default_response_by_default() {
    let mut device = device();
    let frame = frame(
        FrameType::SPECIFIC,
        Direction::CLIENT_TO_SERVER,
        "add",
        cluster::genScenes::ID,
        json!({"groupid": 1, "sceneid": 2}),
    );

    device
        .on_zcl_data(&payload(&frame), &frame, 0, Value::Null)
        .await;

    assert_eq!(device.endpoints[0].default_response_calls.len(), 1);
}

#[tokio::test]
async fn replies_with_default_response_from_caller() {
    let mut device = device();
    let frame = frame(
        FrameType::SPECIFIC,
        Direction::SERVER_TO_CLIENT,
        "addRsp",
        cluster::genScenes::ID,
        json!({"status": 0}),
    );

    device
        .on_zcl_data(
            &payload(&frame),
            &frame,
            0,
            json!({"defaultResponse": true}),
        )
        .await;

    assert_eq!(device.endpoints[0].default_response_calls.len(), 1);
}

#[tokio::test]
async fn replies_with_non_success_default_response_even_when_disable_default_response_is_on() {
    let mut device = device();
    let mut frame = frame(
        FrameType::SPECIFIC,
        Direction::SERVER_TO_CLIENT,
        "addRsp",
        cluster::genScenes::ID,
        json!({"status": 0}),
    );
    frame.header.frame_control.disable_default_response = true;

    device
        .on_zcl_data(
            &payload(&frame),
            &frame,
            0,
            json!({"defaultResponse": Status::FAILURE as u8}),
        )
        .await;

    assert_eq!(device.endpoints[0].default_response_calls.len(), 1);
    assert_eq!(
        device.endpoints[0].default_response_calls[0][1],
        json!(Status::FAILURE as u8)
    );
}

#[tokio::test]
async fn skips_replying_with_default_response_when_device_option_set() {
    let mut device = device();
    device.options = json!({"disableDefaultResponse": true});
    let frame = frame(
        FrameType::SPECIFIC,
        Direction::SERVER_TO_CLIENT,
        "addRsp",
        cluster::genScenes::ID,
        json!({"status": 0}),
    );

    device
        .on_zcl_data(&payload(&frame), &frame, 0, Value::Null)
        .await;

    assert_eq!(device.endpoints[0].default_response_calls.len(), 0);
}

#[tokio::test]
async fn de_duplicates_replying_with_default_response() {
    let mut device = device();
    let frame = frame(
        FrameType::SPECIFIC,
        Direction::SERVER_TO_CLIENT,
        "addRsp",
        cluster::genScenes::ID,
        json!({"status": 0}),
    );

    device
        .on_zcl_data(&payload(&frame), &frame, 0, Value::Null)
        .await;
    device
        .on_zcl_data(&payload(&frame), &frame, 0, Value::Null)
        .await;

    assert_eq!(device.endpoints[0].default_response_calls.len(), 1);
}

#[tokio::test]
async fn skips_replying_with_default_response_when_tuya_device_option_set() {
    let mut device = device();
    device.model_id = "TS0601".to_string();
    device.manufacturer_name = "_TZE200_dragons".to_string();
    let frame = frame(
        FrameType::SPECIFIC,
        Direction::SERVER_TO_CLIENT,
        "addRsp",
        cluster::genScenes::ID,
        json!({"status": 0}),
    );

    device
        .on_zcl_data(&payload(&frame), &frame, 0, Value::Null)
        .await;

    assert_eq!(device.endpoints[0].default_response_calls.len(), 0);
}
