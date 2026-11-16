//! Port of upstream `test/device.test.ts`.
//!
//! Anchor: `.upstream/zigbee-herdsman/test/device.test.ts` (v11.2.0, c1a54e3d).
//!
//! Upstream spies on `endpoint.readResponse/command/read/defaultResponse`; the
//! Rust model has no per-endpoint spies, so the same expectations are asserted
//! on the ZCL frames the device sends through the injected adapter
//! (`MockAdapter::sent_endpoint`), which is strictly more observable.

#[path = "controller_suite/support.rs"]
mod support;

use std::sync::{Arc, Mutex};

use serde_json::json;
use support::{temp_path, MockAdapter};

use zigbee_apiarist::adapter::events::ZclPayload;
use zigbee_apiarist::controller::database::{Database, DeviceType};
use zigbee_apiarist::controller::model::device::{Device, InterviewState};
use zigbee_apiarist::controller::model::endpoint::Endpoint;
use zigbee_apiarist::controller::model::entity::Entity;
use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
use zigbee_apiarist::zspec::zcl::zcl_header::{Direction, FrameType};

struct Fixture {
    device: Arc<Device>,
    endpoint: Arc<Endpoint>,
    adapter: Arc<MockAdapter>,
}

fn fixture() -> Fixture {
    let adapter = MockAdapter::new();
    let database = Arc::new(Mutex::new(Database::open(temp_path("device_suite"))));
    Entity::inject_database(database);
    Entity::inject_adapter(adapter.clone());
    Device::reset_cache();

    let device = Device::create(
        DeviceType::Router,
        "0xfe34ac2385ff8311",
        0x0001,
        Some(0x0102),
        Some("Herdsman".to_string()),
        Some("Mains (single phase)".to_string()),
        Some("Herd-01".to_string()),
        InterviewState::Successful,
        None,
    )
    .expect("device created");
    let endpoint = device.create_endpoint(1).expect("endpoint 1");

    Fixture {
        device,
        endpoint,
        adapter,
    }
}

fn incoming(frame: &Frame) -> ZclPayload {
    ZclPayload {
        cluster_id: frame.cluster.ID as u16,
        data: frame.to_buffer(),
        header: Some(frame.header.clone()),
        address: json!(0x1234),
        endpoint: 1,
        linkquality: 150,
        group_id: 0,
        was_broadcast: false,
        destination_endpoint: 1,
    }
}

fn command_frame(
    frame_type: FrameType,
    direction: Direction,
    disable_default_response: bool,
    transaction_sequence_number: u8,
    command: &'static str,
    cluster: u16,
    payload: serde_json::Value,
) -> Frame {
    Frame::create(
        frame_type as u8,
        direction as u8,
        disable_default_response,
        None,
        transaction_sequence_number,
        command,
        cluster,
        payload,
        json!({}),
        0,
    )
}

fn sent(adapter: &MockAdapter) -> Vec<Frame> {
    adapter.sent_endpoint.lock().unwrap().clone()
}

fn names(frames: &[Frame]) -> Vec<String> {
    frames.iter().map(|frame| frame.command.name.clone()).collect()
}

fn count(frames: &[Frame], name: &str) -> usize {
    frames
        .iter()
        .filter(|frame| frame.command.name == name)
        .count()
}

/// A `readRsp` for `cluster` as the mocked device answer (upstream `readSpy.mockResolvedValue`).
fn read_response_payload(cluster: u16, payload: serde_json::Value) -> ZclPayload {
    let frame = command_frame(
        FrameType::GLOBAL,
        Direction::SERVER_TO_CLIENT,
        false,
        1,
        "readRsp",
        cluster,
        payload,
    );
    incoming(&frame)
}

fn gen_basic() -> u16 {
    0x0000
}
fn gen_time() -> u16 {
    0x000a
}
fn gen_groups() -> u16 {
    0x0004
}
fn gen_on_off() -> u16 {
    0x0006
}
fn gen_identify() -> u16 {
    0x0003
}
fn gen_scenes() -> u16 {
    0x0005
}
fn gen_poll_ctrl() -> u16 {
    0x0020
}
fn ss_ias_zone() -> u16 {
    0x0500
}

#[tokio::test]
async fn replies_to_basic_read() {
    let _guard = support::test_lock().await;
    let fixture = fixture();
    let frame = command_frame(
        FrameType::GLOBAL,
        Direction::CLIENT_TO_SERVER,
        false,
        1,
        "read",
        gen_basic(),
        json!([{"attrId": 0x0000}, {"attrId": 0xfffa}]),
    );

    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, None)
        .await
        .expect("on zcl data");

    let frames = sent(&fixture.adapter);
    assert_eq!(names(&frames), vec!["readRsp"], "{frames:?}");
    let reply = &frames[0];
    assert_eq!(reply.cluster.ID as u16, gen_basic());
    assert_eq!(reply.header.transaction_sequence_number, 1);
    let attributes = reply.payload.as_array().expect("attribute list");
    assert_eq!(attributes.len(), 2);
    let zcl_version = attributes
        .iter()
        .find(|attribute| attribute["attrId"] == 0x0000)
        .expect("zclVersion entry");
    assert_eq!(zcl_version["status"], 0);
    // Unsupported attribute -> NO_DATA (0x86).
    let unsupported = attributes
        .iter()
        .find(|attribute| attribute["attrId"] == 0xfffa)
        .expect("unsupported entry");
    assert_eq!(unsupported["status"], 0x86);
}

#[tokio::test]
async fn replies_to_time_read() {
    let _guard = support::test_lock().await;
    let fixture = fixture();
    let frame = command_frame(
        FrameType::GLOBAL,
        Direction::CLIENT_TO_SERVER,
        false,
        1,
        "read",
        gen_time(),
        json!([{"attrId": 0x0000}, {"attrId": 0xfffa}]),
    );

    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, None)
        .await
        .expect("on zcl data");

    let frames = sent(&fixture.adapter);
    assert_eq!(names(&frames), vec!["readRsp"], "{frames:?}");
    let attributes = frames[0].payload.as_array().expect("attribute list");
    let time = attributes
        .iter()
        .find(|attribute| attribute["attrId"] == 0x0000)
        .expect("time entry");
    assert_eq!(time["status"], 0);
    assert!(
        time["attrData"].as_u64().unwrap_or(0) > 0,
        "time is generated: {attributes:?}"
    );
    assert!(attributes
        .iter()
        .any(|attribute| attribute["attrId"] == 0xfffa));
}

#[tokio::test]
async fn replies_to_unsupported_read() {
    let _guard = support::test_lock().await;
    let fixture = fixture();
    let frame = command_frame(
        FrameType::GLOBAL,
        Direction::CLIENT_TO_SERVER,
        false,
        1,
        "read",
        gen_groups(),
        json!([{"attrId": 0x0000}]),
    );

    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, None)
        .await
        .expect("on zcl data");

    let frames = sent(&fixture.adapter);
    assert_eq!(names(&frames), vec!["readRsp"], "{frames:?}");
    assert_eq!(frames[0].cluster.ID as u16, gen_groups());
    let attributes = frames[0].payload.as_array().expect("attribute list");
    assert_eq!(attributes.len(), 1);
    assert_eq!(attributes[0]["attrId"], 0x0000);
    assert_eq!(attributes[0]["status"], 0x86);
}

#[tokio::test]
async fn replies_to_read_with_custom_response() {
    let _guard = support::test_lock().await;
    let fixture = fixture();
    fixture
        .device
        .set_custom_read_response(Some(Box::new(|_frame: &Frame, _endpoint: &Endpoint| false)));
    let frame = command_frame(
        FrameType::GLOBAL,
        Direction::CLIENT_TO_SERVER,
        false,
        1,
        "read",
        gen_basic(),
        json!([{"attrId": 0x0000}]),
    );

    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, None)
        .await
        .expect("on zcl data");

    // Returning `false` does not take over: the default read response is sent.
    let frames = sent(&fixture.adapter);
    assert_eq!(names(&frames), vec!["readRsp"], "{frames:?}");
}

#[tokio::test]
async fn replies_to_read_with_override_custom_response() {
    let _guard = support::test_lock().await;
    let fixture = fixture();
    fixture
        .device
        .set_custom_read_response(Some(Box::new(|_frame: &Frame, _endpoint: &Endpoint| true)));
    let frame = command_frame(
        FrameType::GLOBAL,
        Direction::CLIENT_TO_SERVER,
        false,
        1,
        "read",
        gen_basic(),
        json!([{"attrId": 0x0000, "type": 0x20}]),
    );

    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, None)
        .await
        .expect("on zcl data");

    assert_eq!(sent(&fixture.adapter).len(), 0, "no response is sent");
}

#[tokio::test]
async fn does_not_send_default_response_for_default_response() {
    let _guard = support::test_lock().await;
    let fixture = fixture();
    let frame = command_frame(
        FrameType::GLOBAL,
        Direction::CLIENT_TO_SERVER,
        false,
        1,
        "defaultRsp",
        gen_basic(),
        json!({"cmdId": 0x00, "statusCode": 0}),
    );

    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, None)
        .await
        .expect("on zcl data");

    assert_eq!(sent(&fixture.adapter).len(), 0, "no default response reply");
}

#[tokio::test]
async fn replies_to_ias_zone_enroll_request() {
    let _guard = support::test_lock().await;
    let fixture = fixture();
    let frame = command_frame(
        FrameType::SPECIFIC,
        Direction::SERVER_TO_CLIENT,
        false,
        1,
        "enrollReq",
        ss_ias_zone(),
        json!({"zonetype": 0x4567, "manucode": 0x1234}),
    );

    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, None)
        .await
        .expect("on zcl data");

    let frames = sent(&fixture.adapter);
    assert_eq!(names(&frames), vec!["enrollRsp"], "{frames:?}");
    let reply = &frames[0];
    assert_eq!(reply.cluster.ID as u16, ss_ias_zone());
    assert_eq!(reply.header.transaction_sequence_number, 1);
    assert!(reply.header.frame_control.disable_default_response);
    assert_eq!(reply.payload["enrollrspcode"], 0);
    assert_eq!(reply.payload["zoneid"], 23);
}

#[tokio::test]
async fn fails_to_reply_to_ias_zone_enroll_request() {
    let _guard = support::test_lock().await;
    let fixture = fixture();
    *fixture.adapter.fail_zcl_command.lock().unwrap() = Some("enrollRsp".to_string());
    let frame = command_frame(
        FrameType::SPECIFIC,
        Direction::SERVER_TO_CLIENT,
        false,
        1,
        "enrollReq",
        ss_ias_zone(),
        json!({"zonetype": 0x4567, "manucode": 0x1234}),
    );

    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, None)
        .await
        .expect("on zcl data");

    let frames = sent(&fixture.adapter);
    assert_eq!(names(&frames), vec!["enrollRsp", "defaultRsp"], "{frames:?}");
    let reply = &frames[1];
    assert_eq!(reply.cluster.ID as u16, ss_ias_zone());
    assert_eq!(reply.payload["cmdId"], frame.command.ID as u64);
    assert_eq!(reply.payload["statusCode"], 1, "FAILURE");
    assert_eq!(
        reply.header.frame_control.direction,
        Direction::CLIENT_TO_SERVER
    );
}

#[tokio::test]
async fn replies_to_poll_control_check_in() {
    let _guard = support::test_lock().await;
    let fixture = fixture();
    fixture.adapter.set_zcl(|_ieee, _nwk, _endpoint, frame| {
        if frame.command.name == "read" {
            return Ok(Some(read_response_payload(
                gen_poll_ctrl(),
                json!([{"attrId": 0x0000, "status": 0, "dataType": 0x23, "attrData": 100}]),
            )));
        }
        Ok(None)
    });
    let frame = command_frame(
        FrameType::SPECIFIC,
        Direction::SERVER_TO_CLIENT,
        false,
        1,
        "checkin",
        gen_poll_ctrl(),
        json!({}),
    );

    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, None)
        .await
        .expect("on zcl data");

    let frames = sent(&fixture.adapter);
    assert_eq!(
        names(&frames),
        vec!["checkinRsp", "read", "fastPollStop"],
        "{frames:?}"
    );
    assert_eq!(frames[0].payload["startFastPolling"], 1);
    assert_eq!(frames[0].payload["fastPollTimeout"], 0);
    assert!(frames[0].header.frame_control.disable_default_response);
    assert_eq!(frames[1].cluster.ID as u16, gen_poll_ctrl());

    // Second check-in with a known interval only acknowledges.
    fixture.adapter.sent_endpoint.lock().unwrap().clear();
    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, None)
        .await
        .expect("on zcl data");
    let frames = sent(&fixture.adapter);
    assert_eq!(names(&frames), vec!["checkinRsp"], "{frames:?}");
    assert_eq!(frames[0].payload["startFastPolling"], 0);
}

#[tokio::test]
async fn fails_to_reply_to_poll_control_check_in() {
    let _guard = support::test_lock().await;
    let fixture = fixture();
    *fixture.adapter.fail_zcl_command.lock().unwrap() = Some("checkinRsp".to_string());
    let frame = command_frame(
        FrameType::SPECIFIC,
        Direction::SERVER_TO_CLIENT,
        false,
        1,
        "checkin",
        gen_poll_ctrl(),
        json!({}),
    );

    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, None)
        .await
        .expect("on zcl data");

    let frames = sent(&fixture.adapter);
    assert_eq!(names(&frames), vec!["checkinRsp", "defaultRsp"], "{frames:?}");
    assert_eq!(frames[1].payload["cmdId"], frame.command.ID as u64);
    assert_eq!(frames[1].payload["statusCode"], 1);
}

#[tokio::test]
async fn fails_to_stop_fast_poll() {
    let _guard = support::test_lock().await;
    let fixture = fixture();
    fixture.adapter.set_zcl(|_ieee, _nwk, _endpoint, frame| {
        match frame.command.name.as_str() {
            "read" => Ok(Some(read_response_payload(
                gen_poll_ctrl(),
                json!([{"attrId": 0x0000, "status": 0, "dataType": 0x23, "attrData": 100}]),
            ))),
            "fastPollStop" => Err("failure".to_string()),
            _ => Ok(None),
        }
    });
    let frame = command_frame(
        FrameType::SPECIFIC,
        Direction::SERVER_TO_CLIENT,
        false,
        1,
        "checkin",
        gen_poll_ctrl(),
        json!({}),
    );

    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, None)
        .await
        .expect("on zcl data");

    let frames = sent(&fixture.adapter);
    assert_eq!(
        names(&frames),
        vec!["checkinRsp", "read", "fastPollStop"],
        "{frames:?}"
    );
    // The failure to stop fast polling is logged, not answered with a default response.
    assert_eq!(count(&frames, "defaultRsp"), 0);
}

#[tokio::test]
async fn replies_to_global_with_success_default_response_by_default() {
    let _guard = support::test_lock().await;
    let fixture = fixture();
    let frame = command_frame(
        FrameType::GLOBAL,
        Direction::SERVER_TO_CLIENT,
        false,
        1,
        "readRsp",
        gen_on_off(),
        json!([{"attrId": 0x0000, "status": 0, "dataType": 0x10, "attrData": 1}]),
    );

    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, None)
        .await
        .expect("on zcl data");

    let frames = sent(&fixture.adapter);
    assert_eq!(names(&frames), vec!["defaultRsp"], "{frames:?}");
    assert_eq!(frames[0].cluster.ID as u16, gen_on_off());
    assert_eq!(frames[0].payload["cmdId"], frame.command.ID as u64);
    assert_eq!(frames[0].payload["statusCode"], 0);
    assert_eq!(
        frames[0].header.frame_control.direction,
        Direction::CLIENT_TO_SERVER
    );
}

#[tokio::test]
async fn replies_to_specific_from_server_with_success_default_response_by_default() {
    let _guard = support::test_lock().await;
    let fixture = fixture();
    let frame = command_frame(
        FrameType::SPECIFIC,
        Direction::SERVER_TO_CLIENT,
        false,
        1,
        "removeRsp",
        gen_scenes(),
        json!({"status": 0, "groupid": 1, "sceneid": 2}),
    );

    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, None)
        .await
        .expect("on zcl data");

    let frames = sent(&fixture.adapter);
    assert_eq!(names(&frames), vec!["defaultRsp"], "{frames:?}");
    assert_eq!(frames[0].cluster.ID as u16, gen_scenes());
    assert_eq!(frames[0].payload["statusCode"], 0);
    assert_eq!(
        frames[0].header.frame_control.direction,
        Direction::CLIENT_TO_SERVER
    );
}

#[tokio::test]
async fn replies_to_specific_from_client_with_success_default_response_by_default() {
    let _guard = support::test_lock().await;
    let fixture = fixture();
    let frame = command_frame(
        FrameType::SPECIFIC,
        Direction::CLIENT_TO_SERVER,
        false,
        1,
        "identify",
        gen_identify(),
        json!({"identifytime": 1}),
    );

    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, None)
        .await
        .expect("on zcl data");

    let frames = sent(&fixture.adapter);
    assert_eq!(names(&frames), vec!["defaultRsp"], "{frames:?}");
    assert_eq!(frames[0].cluster.ID as u16, gen_identify());
    assert_eq!(
        frames[0].header.frame_control.direction,
        Direction::SERVER_TO_CLIENT
    );
}

#[tokio::test]
async fn replies_with_default_response_from_caller() {
    let _guard = support::test_lock().await;
    let fixture = fixture();
    let frame = command_frame(
        FrameType::GLOBAL,
        Direction::SERVER_TO_CLIENT,
        false,
        1,
        "readRsp",
        gen_on_off(),
        json!([{"attrId": 0x0000, "status": 0, "dataType": 0x10, "attrData": 1}]),
    );

    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, Some(0x87))
        .await
        .expect("on zcl data");

    let frames = sent(&fixture.adapter);
    assert_eq!(names(&frames), vec!["defaultRsp"], "{frames:?}");
    assert_eq!(frames[0].payload["statusCode"], 0x87, "INVALID_VALUE");
    assert_eq!(
        frames[0].header.frame_control.direction,
        Direction::CLIENT_TO_SERVER
    );
}

#[tokio::test]
async fn replies_with_non_success_default_response_even_when_disable_default_response_is_on() {
    let _guard = support::test_lock().await;
    let fixture = fixture();
    let frame = command_frame(
        FrameType::GLOBAL,
        Direction::SERVER_TO_CLIENT,
        true,
        1,
        "report",
        gen_on_off(),
        json!([{"attrId": 0x0000, "dataType": 0x10, "attrData": 0}]),
    );

    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, Some(0x87))
        .await
        .expect("on zcl data");

    let frames = sent(&fixture.adapter);
    assert_eq!(names(&frames), vec!["defaultRsp"], "{frames:?}");
    assert_eq!(frames[0].payload["statusCode"], 0x87);
}

#[tokio::test]
async fn skips_replying_with_default_response_when_device_option_set() {
    let _guard = support::test_lock().await;
    let fixture = fixture();
    fixture.device.set_skip_default_response(true);
    let frame = command_frame(
        FrameType::GLOBAL,
        Direction::SERVER_TO_CLIENT,
        false,
        1,
        "readRsp",
        gen_on_off(),
        json!([{"attrId": 0x0000, "status": 0, "dataType": 0x10, "attrData": 1}]),
    );

    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, None)
        .await
        .expect("on zcl data");

    assert_eq!(sent(&fixture.adapter).len(), 0);
}

#[tokio::test]
async fn de_duplicates_replying_with_default_response() {
    let _guard = support::test_lock().await;
    let fixture = fixture();
    let frame = command_frame(
        FrameType::GLOBAL,
        Direction::SERVER_TO_CLIENT,
        false,
        0,
        "readRsp",
        gen_on_off(),
        json!([{"attrId": 0x0000, "status": 0, "dataType": 0x10, "attrData": 1}]),
    );

    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, None)
        .await
        .expect("on zcl data");
    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, None)
        .await
        .expect("on zcl data");

    // The duplicate (same transaction sequence number) is answered once.
    assert_eq!(count(&sent(&fixture.adapter), "defaultRsp"), 1);

    // A different transaction sequence number is answered again.
    let next = command_frame(
        FrameType::GLOBAL,
        Direction::SERVER_TO_CLIENT,
        false,
        1,
        "readRsp",
        gen_on_off(),
        json!([{"attrId": 0x0000, "status": 0, "dataType": 0x10, "attrData": 1}]),
    );
    fixture
        .device
        .on_zcl_data(&incoming(&next), Some(&next), &fixture.endpoint, None)
        .await
        .expect("on zcl data");
    assert_eq!(count(&sent(&fixture.adapter), "defaultRsp"), 2);
}

#[tokio::test]
async fn skips_replying_with_default_response_when_tuya_device_option_set() {
    let _guard = support::test_lock().await;
    std::env::set_var("DISABLE_TUYA_DEFAULT_RESPONSE", "1");
    let fixture = fixture();
    fixture
        .device
        .set_manufacturer_name(Some("_TZ1234_5678".to_string()));
    let frame = command_frame(
        FrameType::GLOBAL,
        Direction::SERVER_TO_CLIENT,
        false,
        1,
        "readRsp",
        gen_on_off(),
        json!([{"attrId": 0x0000, "status": 0, "dataType": 0x10, "attrData": 1}]),
    );

    fixture
        .device
        .on_zcl_data(&incoming(&frame), Some(&frame), &fixture.endpoint, None)
        .await
        .expect("on zcl data");

    assert_eq!(sent(&fixture.adapter).len(), 0);
    std::env::remove_var("DISABLE_TUYA_DEFAULT_RESPONSE");
}

