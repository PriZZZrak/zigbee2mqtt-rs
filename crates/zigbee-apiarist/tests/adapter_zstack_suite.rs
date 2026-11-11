//! Port of upstream `test/adapter/z-stack/adapter.test.ts`.
//!
//! Anchor: .upstream/zigbee-herdsman/test/adapter/z-stack/adapter.test.ts

#[path = "adapter_zstack_suite/fixtures.rs"]
mod fixtures;
#[path = "adapter_zstack_suite/support.rs"]
mod support;

use zigbee_apiarist::adapter::z_stack::adapter::manager::StartResult;
use zigbee_apiarist::models::startup_options::{AdapterOptions, ConfigNetworkOptions};

use support::Harness;

const PAN_ID: u16 = 123;
const EXTENDED_PAN_ID: [u8; 8] = [0x00, 0x12, 0x4b, 0x00, 0x09, 0xd6, 0x9f, 0x77];
const DEFAULT_EXTENDED_PAN_ID: [u8; 8] = [0xdd; 8];

fn network_options(pan_id: u16, extended_pan_id: [u8; 8], channel: u8) -> ConfigNetworkOptions {
    ConfigNetworkOptions {
        pan_id,
        extended_pan_id: Some(extended_pan_id.to_vec()),
        network_key: Some(fixtures::NETWORK_KEY.to_vec()),
        channel_list: vec![channel],
        network_key_distribute: Some(false),
    }
}

fn default_network_options() -> ConfigNetworkOptions {
    network_options(PAN_ID, EXTENDED_PAN_ID, 21)
}

fn harness(options: ConfigNetworkOptions, backup: &str) -> Harness {
    Harness::new(
        options,
        backup,
        AdapterOptions {
            concurrent: Some(3),
            ..AdapterOptions::default()
        },
    )
}

#[tokio::test]
async fn should_start_network_with_3_0_x_adapter() {
    // Anchor: adapter.test.ts "should start network with 3.0.x adapter".
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_3_aligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Resumed);
}

#[tokio::test]
async fn should_commission_network_with_3_0_x_adapter() {
    // Anchor: adapter.test.ts "should commission network with 3.0.x adapter".
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::empty_3_aligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Reset);
}

#[tokio::test]
async fn should_commission_network_with_3_0_x_adapter_default_extended_pan_id() {
    // Anchor: "... - default extended pan id".
    let mut harness = harness(
        network_options(PAN_ID, DEFAULT_EXTENDED_PAN_ID, 21),
        "backup.json",
    );
    fixtures::empty_3_aligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Reset);
}

#[tokio::test]
async fn should_commission_network_with_3_0_x_adapter_unaligned_8_bit() {
    // Anchor: "... - unaligned 8-bit".
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::empty_3_unaligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Reset);
}

#[tokio::test]
async fn should_commission_network_with_3_0_x_adapter_auto_concurrency() {
    let mut harness = Harness::new(
        default_network_options(),
        "backup.json",
        AdapterOptions::default(),
    );
    fixtures::empty_3_aligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Reset);
}

#[tokio::test]
async fn should_commission_network_with_3_x_0_adapter_auto_concurrency() {
    let mut harness = Harness::new(
        default_network_options(),
        "backup.json",
        AdapterOptions::default(),
    );
    fixtures::empty_3x0_aligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Reset);
}

#[tokio::test]
async fn should_fail_to_commission_network_with_3_0_x_adapter_with_invalid_pan_id_65535() {
    let mut harness = harness(network_options(65535, EXTENDED_PAN_ID, 21), "backup.json");
    fixtures::empty_3_aligned(&harness.mock);
    let error = harness.start().await.expect_err("invalid pan id");
    assert_eq!(
        error,
        "network commissioning failed - cannot use pan id 65535"
    );
}

#[tokio::test]
async fn should_fail_to_commission_network_with_3_0_x_adapter_when_nib_reports_different_pan_id() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::empty_3_aligned(&harness.mock);
    harness.mock.handle(
        zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::APP_CNF,
        "bdbStartCommissioning",
        |_, state| {
            state.nv.insert(
                zigbee_apiarist::adapter::z_stack::constants::common::NvItemsIds::NIB,
                Some(fixtures::hex(fixtures::NIB_COMMISSIONED_3_PAN_MISMATCH)),
            );
            support::Reply::empty().with_extra(
                zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::ZDO,
                192,
                vec![9],
            )
        },
    );
    let error = harness.start().await.expect_err("pan id collision");
    assert_eq!(
        error,
        "network commissioning failed - panId collision detected (expected=123, actual=124)"
    );
}

#[tokio::test]
async fn should_start_network_with_3_0_x_adapter_resume_in_coordinator_mode() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_3_aligned(&harness.mock);
    harness.mock.handle(
        zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::UTIL,
        "getDeviceInfo",
        |_, _| support::Reply::payload(serde_json::json!({"devicestate": 9})),
    );
    harness.mock.handle(
        zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::ZDO,
        "activeEpReq",
        |_, _| {
            support::Reply::empty().with_zdo(
                zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::ACTIVE_ENDPOINTS_RESPONSE,
                Some(0),
                &[serde_json::json!(0), serde_json::json!({"nwkAddress": 0, "endpointList": [1, 2, 3]})],
            )
        },
    );
    assert_eq!(harness.start().await.expect("start"), StartResult::Resumed);
}

#[tokio::test]
async fn should_start_network_with_3_0_x_adapter_resume_in_coordinator_mode_ext_group_find_failed()
{
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_3_aligned(&harness.mock);
    harness.mock.handle(
        zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::UTIL,
        "getDeviceInfo",
        |_, _| support::Reply::payload(serde_json::json!({"devicestate": 9})),
    );
    harness.mock.handle(
        zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::ZDO,
        "extFindGroup",
        |_, _| support::Reply::payload(serde_json::json!({"status": 1})),
    );
    assert_eq!(harness.start().await.expect("start"), StartResult::Resumed);
}

#[tokio::test]
async fn should_commission_network_with_1_2_adapter() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::empty_12_unaligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Restored);
}

#[tokio::test]
async fn should_commission_network_with_1_2_adapter_default_extended_pan_id() {
    let mut harness = harness(
        network_options(PAN_ID, DEFAULT_EXTENDED_PAN_ID, 21),
        "backup.json",
    );
    fixtures::empty_12_unaligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Restored);
}

#[tokio::test]
async fn should_reset_network_with_1_2_adapter_config_mismatch() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_12_unaligned_mismatch(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Reset);
}

#[tokio::test]
async fn add_install_code_with_crc() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_3x0_aligned(&harness.mock);
    harness.start().await.expect("start");
    harness.mock.clear_requests();
    let code = vec![
        0xae, 0x3b, 0x28, 0x72, 0x81, 0xcf, 0x16, 0xf5, 0x50, 0x73, 0x3a, 0x0c, 0xec, 0x38, 0xaa,
        0x31, 0xe8, 0x02,
    ];
    harness
        .adapter
        .add_install_code("0x9035EAFFFE424783", &code, false)
        .await
        .expect("install code");
    let requests = harness.mock.requests_named("bdbAddInstallCode");
    assert_eq!(requests.len(), 1);
    let frame = &requests[0].frame;
    let mut expected = vec![0x1];
    expected.extend_from_slice(&fixtures::hex("834742feffea3590"));
    expected.extend_from_slice(&code);
    assert_eq!(&frame[4..frame.len() - 1], expected.as_slice());
}

#[tokio::test]
async fn add_install_code_key_derived_from_install_code() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_3x0_aligned(&harness.mock);
    harness.start().await.expect("start");
    harness.mock.clear_requests();
    let code = vec![
        0xae, 0x3b, 0x28, 0x72, 0x81, 0xcf, 0x16, 0xf5, 0x50, 0x73, 0x3a, 0x0c, 0xec, 0x38, 0xaa,
        0x31,
    ];
    harness
        .adapter
        .add_install_code("0x9035EAFFFE424783", &code, true)
        .await
        .expect("install code");
    let requests = harness.mock.requests_named("bdbAddInstallCode");
    assert_eq!(requests.len(), 1);
    let frame = &requests[0].frame;
    let mut expected = vec![0x2];
    expected.extend_from_slice(&fixtures::hex("834742feffea3590"));
    expected.extend_from_slice(&code);
    assert_eq!(&frame[4..frame.len() - 1], expected.as_slice());
}

#[tokio::test]
async fn close_adapter() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_3x0_aligned(&harness.mock);
    harness.start().await.expect("start");
    harness.adapter.stop().await.expect("stop");
}

#[tokio::test]
async fn get_coordinator_ieee() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_3x0_aligned(&harness.mock);
    harness.start().await.expect("start");
    assert_eq!(
        harness.adapter.get_coordinator_ieee().await.expect("ieee"),
        "0x00124b0009d80ba7"
    );
}

#[tokio::test]
async fn get_coordinator_version() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_3x0_aligned(&harness.mock);
    harness.start().await.expect("start");
    let version = harness
        .adapter
        .get_coordinator_version()
        .expect("coordinator version");
    assert_eq!(version.type_name, "ZStack3x0");
    assert_eq!(version.meta["revision"], 20210430);
    assert_eq!(version.meta["product"], 1);
}

#[tokio::test]
async fn permit_join_specific_network_address() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_3x0_aligned(&harness.mock);
    harness.start().await.expect("start");
    harness.mock.clear_requests();
    harness
        .adapter
        .permit_join(102, Some(42102))
        .await
        .expect("permit join");
    let zdo = harness.mock.requests_named("mgmtPermitJoinReq");
    assert_eq!(zdo.len(), 1);
    let zdo_payload = zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo::build_request(
        false,
        zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::PERMIT_JOINING_REQUEST,
        &[
            serde_json::json!(102),
            serde_json::json!(1),
            serde_json::json!([]),
        ],
    );
    let mut expected = vec![2, (42102 & 0xff) as u8, (42102 >> 8) as u8];
    expected.extend_from_slice(&zdo_payload);
    assert_eq!(
        &zdo[0].frame[4..zdo[0].frame.len() - 1],
        expected.as_slice()
    );
    let led = harness.mock.requests_named("ledControl");
    assert_eq!(led.len(), 1);
    assert_eq!(led[0].payload()["ledid"], 3);
    assert_eq!(led[0].payload()["mode"], 1);
}

#[tokio::test]
async fn soft_and_hard_reset_use_expected_reset_types() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_3x0_aligned(&harness.mock);
    harness.start().await.expect("start");

    harness.mock.clear_requests();
    harness.adapter.reset("soft").await.expect("soft reset");
    let requests = harness.mock.requests_named("resetReq");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].payload()["type"], 1);

    harness.mock.clear_requests();
    harness.adapter.reset("hard").await.expect("hard reset");
    let requests = harness.mock.requests_named("resetReq");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].payload()["type"], 0);
}

fn write_no_rsp_frame() -> zigbee_apiarist::zspec::zcl::zcl_frame::Frame {
    use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
    use zigbee_apiarist::zspec::zcl::zcl_header::{Direction, FrameType};
    Frame::create(
        FrameType::GLOBAL as u8,
        Direction::CLIENT_TO_SERVER as u8,
        true,
        None,
        100,
        "writeNoRsp",
        0,
        serde_json::json!([{"attrId": 0, "dataType": 0, "attrData": null}]),
        serde_json::json!({}),
        0,
    )
}

fn read_frame(disable_default_response: bool) -> zigbee_apiarist::zspec::zcl::zcl_frame::Frame {
    use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
    use zigbee_apiarist::zspec::zcl::zcl_header::{Direction, FrameType};
    Frame::create(
        FrameType::GLOBAL as u8,
        Direction::CLIENT_TO_SERVER as u8,
        disable_default_response,
        None,
        100,
        "read",
        0,
        serde_json::json!([{"attrId": 0}]),
        serde_json::json!({}),
        0,
    )
}

fn data_requests(harness: &Harness) -> Vec<support::Request> {
    harness
        .mock
        .requests()
        .into_iter()
        .filter(|request| request.command == "dataRequest" || request.command == "dataRequestExt")
        .collect()
}

async fn started_3x0() -> Harness {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_3x0_aligned(&harness.mock);
    harness.start().await.expect("start");
    harness
}

#[tokio::test]
async fn start_with_transmit_power_set() {
    let mut harness = Harness::new(
        default_network_options(),
        "backup.json",
        AdapterOptions {
            concurrent: Some(3),
            transmit_power: Some(2),
            disable_led: false,
            ..AdapterOptions::default()
        },
    );
    fixtures::commissioned_3x0_aligned(&harness.mock);
    harness.start().await.expect("start");
    let requests = harness.mock.requests_named("stackTune");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].payload()["operation"], 0);
    assert_eq!(requests[0].payload()["value"], 2);
}

#[tokio::test]
async fn support_led_should_go_to_false_when_led_request_fails() {
    let mut harness = started_3x0().await;
    harness.mock.handle(
        zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::UTIL,
        "ledControl",
        |_, _| support::Reply::payload(serde_json::json!({"status": 1})),
    );
    harness.mock.clear_requests();
    harness
        .adapter
        .permit_join(0, Some(0))
        .await
        .expect("permit join");
    let led = harness.mock.requests_named("ledControl");
    assert_eq!(led.len(), 1);
    assert_eq!(led[0].payload()["ledid"], 3);
    assert_eq!(led[0].payload()["mode"], 0);

    harness.mock.clear_requests();
    harness
        .adapter
        .permit_join(0, Some(0))
        .await
        .expect("permit join");
    assert!(harness.mock.requests_named("ledControl").is_empty());
}

#[tokio::test]
async fn led_behaviour_disable_led_true_firmware_not_handling_leds() {
    let mut harness = Harness::new(
        default_network_options(),
        "backup.json",
        AdapterOptions {
            disable_led: true,
            ..AdapterOptions::default()
        },
    );
    fixtures::commissioned_3x0_aligned(&harness.mock);
    harness.start().await.expect("start");
    let led = harness.mock.requests_named("ledControl");
    assert_eq!(led.len(), 1);
    assert_eq!(led[0].payload()["ledid"], 3);
    assert_eq!(led[0].payload()["mode"], 0);

    harness.mock.clear_requests();
    harness
        .adapter
        .permit_join(255, Some(0))
        .await
        .expect("permit join");
    assert!(harness.mock.requests_named("ledControl").is_empty());
    harness.mock.clear_requests();
    harness
        .adapter
        .permit_join(0, Some(0))
        .await
        .expect("permit join");
    assert!(harness.mock.requests_named("ledControl").is_empty());
}

#[tokio::test]
async fn led_behaviour_disable_led_false_firmware_not_handling_leds() {
    let mut harness = started_3x0().await;
    assert!(harness.mock.requests_named("ledControl").is_empty());
    harness.mock.clear_requests();
    harness
        .adapter
        .permit_join(255, Some(0))
        .await
        .expect("permit join");
    let led = harness.mock.requests_named("ledControl");
    assert_eq!(led.len(), 1);
    assert_eq!(led[0].payload()["mode"], 1);
    harness.mock.clear_requests();
    harness
        .adapter
        .permit_join(0, Some(0))
        .await
        .expect("permit join");
    let led = harness.mock.requests_named("ledControl");
    assert_eq!(led.len(), 1);
    assert_eq!(led[0].payload()["mode"], 0);
}

#[tokio::test]
async fn led_behaviour_disable_led_true_firmware_handling_leds() {
    let mut harness = Harness::new(
        default_network_options(),
        "backup.json",
        AdapterOptions {
            disable_led: true,
            ..AdapterOptions::default()
        },
    );
    fixtures::commissioned_3x0_aligned(&harness.mock);
    harness.mock.handle(
        zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::SYS,
        "version",
        |_, _| support::Reply::payload(serde_json::json!({"product": 2, "revision": 20211030})),
    );
    harness.start().await.expect("start");
    let led = harness.mock.requests_named("ledControl");
    assert_eq!(led.len(), 1);
    assert_eq!(led[0].payload()["ledid"], 0xff);
    assert_eq!(led[0].payload()["mode"], 5);
    harness.mock.clear_requests();
    harness
        .adapter
        .permit_join(255, Some(0))
        .await
        .expect("permit join");
    assert!(harness.mock.requests_named("ledControl").is_empty());
    harness.mock.clear_requests();
    harness
        .adapter
        .permit_join(0, Some(0))
        .await
        .expect("permit join");
    assert!(harness.mock.requests_named("ledControl").is_empty());
}

#[tokio::test]
async fn led_behaviour_disable_led_false_firmware_handling_leds() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_3x0_aligned(&harness.mock);
    harness.mock.handle(
        zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::SYS,
        "version",
        |_, _| support::Reply::payload(serde_json::json!({"product": 2, "revision": 20211030})),
    );
    harness.start().await.expect("start");
    assert!(harness.mock.requests_named("ledControl").is_empty());
    harness.mock.clear_requests();
    harness
        .adapter
        .permit_join(255, Some(0))
        .await
        .expect("permit join");
    assert!(harness.mock.requests_named("ledControl").is_empty());
    harness.mock.clear_requests();
    harness
        .adapter
        .permit_join(0, Some(0))
        .await
        .expect("permit join");
    assert!(harness.mock.requests_named("ledControl").is_empty());
}

#[tokio::test]
async fn permit_join_all() {
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    harness
        .adapter
        .permit_join(100, None)
        .await
        .expect("permit join");
    let zdo = harness.mock.requests_named("mgmtPermitJoinReq");
    assert_eq!(zdo.len(), 1);
    let zdo_payload = zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo::build_request(
        false,
        zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::PERMIT_JOINING_REQUEST,
        &[
            serde_json::json!(100),
            serde_json::json!(1),
            serde_json::json!([]),
        ],
    );
    let mut expected = vec![15, 0xfc, 0xff];
    expected.extend_from_slice(&zdo_payload);
    assert_eq!(
        &zdo[0].frame[4..zdo[0].frame.len() - 1],
        expected.as_slice()
    );
    let led = harness.mock.requests_named("ledControl");
    assert_eq!(led.len(), 1);
    assert_eq!(led[0].payload()["mode"], 1);
}

#[tokio::test]
async fn permit_join_coordinator_only() {
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    harness
        .adapter
        .permit_join(254, Some(0))
        .await
        .expect("permit join");
    let zdo = harness.mock.requests_named("mgmtPermitJoinReq");
    assert_eq!(zdo.len(), 1);
    let zdo_payload = zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo::build_request(
        false,
        zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::PERMIT_JOINING_REQUEST,
        &[
            serde_json::json!(254),
            serde_json::json!(1),
            serde_json::json!([]),
        ],
    );
    let mut expected = vec![2, 0, 0];
    expected.extend_from_slice(&zdo_payload);
    assert_eq!(
        &zdo[0].frame[4..zdo[0].frame.len() - 1],
        expected.as_slice()
    );
}

#[tokio::test]
async fn send_zcl_frame_network_address() {
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    let frame = write_no_rsp_frame();
    harness
        .adapter
        .send_zcl_frame_to_endpoint("0x02", 2, 20, &frame, 10000, false, false, None, None)
        .await
        .expect("send");
    let requests = data_requests(&harness);
    assert_eq!(requests.len(), 1);
    let payload = requests[0].payload();
    assert_eq!(requests[0].command, "dataRequest");
    assert_eq!(payload["clusterid"], 0);
    assert_eq!(support::bytes(&payload["data"]), frame.to_buffer());
    assert_eq!(payload["destendpoint"], 20);
    assert_eq!(payload["dstaddr"], 2);
    assert_eq!(payload["len"], 6);
    assert_eq!(payload["options"], 0);
    assert_eq!(payload["radius"], 30);
    assert_eq!(payload["srcendpoint"], 1);
    assert_eq!(payload["transid"], 1);
}

#[tokio::test]
async fn send_zcl_frame_with_aps_encryption() {
    use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
    use zigbee_apiarist::zspec::zcl::zcl_header::{Direction, FrameType};
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    let frame = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::CLIENT_TO_SERVER as u8,
        true,
        None,
        3,
        "read",
        "zigbeeDirectConfiguration",
        serde_json::json!([{"attrId": 0}]),
        serde_json::json!({}),
        0,
    );
    harness
        .adapter
        .send_zcl_frame_to_endpoint(
            "0x1122334455667788",
            1234,
            232,
            &frame,
            10000,
            true,
            false,
            None,
            None,
        )
        .await
        .expect("send");
    let requests = data_requests(&harness);
    assert_eq!(requests.len(), 1);
    let payload = requests[0].payload();
    assert_eq!(payload["clusterid"], 0x003d);
    assert_eq!(support::bytes(&payload["data"]), frame.to_buffer());
    assert_eq!(payload["destendpoint"], 232);
    assert_eq!(payload["dstaddr"], 1234);
    assert_eq!(payload["len"], 5);
    assert_eq!(payload["options"], 64);
}

#[tokio::test]
async fn send_zcl_frame_network_address_retry_on_mac_channel_access_failure() {
    let mut harness = started_3x0().await;
    harness.mock.set_data_confirm_code(225, true);
    harness.mock.clear_requests();
    let frame = write_no_rsp_frame();
    harness
        .adapter
        .send_zcl_frame_to_endpoint("0x02", 2, 20, &frame, 10000, false, false, None, None)
        .await
        .expect("send");
    let requests = data_requests(&harness);
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].payload()["transid"], 1);
    assert_eq!(requests[1].payload()["transid"], 2);
}

#[tokio::test]
async fn send_zcl_frame_network_address_data_confirm_fails() {
    let mut harness = started_3x0().await;
    harness.mock.set_data_confirm_code(201, false);
    let frame = write_no_rsp_frame();
    let error = harness
        .adapter
        .send_zcl_frame_to_endpoint("0x02", 2, 20, &frame, 10000, false, false, None, None)
        .await
        .expect_err("data confirm failure");
    assert!(error.contains("201"), "unexpected error: {error}");
}

#[tokio::test]
async fn send_zcl_frame_network_address_with_default_response() {
    use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
    use zigbee_apiarist::zspec::zcl::zcl_header::{Direction, FrameType};
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    let default_response = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        true,
        None,
        100,
        "defaultRsp",
        0,
        serde_json::json!({"cmdId": 0, "status": 0}),
        serde_json::json!({}),
        0,
    );
    let frame = read_frame(false);
    {
        let request = harness
            .adapter
            .send_zcl_frame_to_endpoint("0x02", 2, 20, &frame, 10000, false, false, None, None);
        tokio::pin!(request);
        harness.mock.inject_areq(
            zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::AF,
            "incomingMsg",
            serde_json::json!({
            "clusterid": 0,
            "srcendpoint": 20,
            "srcaddr": 2,
            "linkquality": 101,
            "groupid": 12,
            "len": default_response.to_buffer().len(),
            "data": default_response.to_buffer(),
            }),
        );
        request.await.expect("send");
    }
    assert_eq!(data_requests(&harness).len(), 1);
}

#[tokio::test]
async fn send_zcl_frame_network_address_fails_because_mac_transaction_expire_should_retry() {
    let mut harness = started_3x0().await;
    harness.mock.set_data_confirm_code(240, false);
    harness.mock.clear_requests();
    let frame = write_no_rsp_frame();
    let error = harness
        .adapter
        .send_zcl_frame_to_endpoint(
            "0x0000000000000002",
            2,
            20,
            &frame,
            10000,
            false,
            false,
            None,
            None,
        )
        .await
        .expect_err("transaction expired");
    assert!(error.contains("240"), "unexpected error: {error}");
    let commands = harness
        .mock
        .requests()
        .into_iter()
        .filter(|request| request.command != "ping")
        .map(|request| request.command)
        .collect::<Vec<_>>();
    let data_requests = commands
        .iter()
        .filter(|command| *command == "dataRequest")
        .count();
    assert!(data_requests >= 4, "commands: {commands:?}");
    assert!(
        commands.contains(&"assocGetWithAddress".to_string()),
        "commands: {commands:?}"
    );
}

#[tokio::test]
async fn send_zcl_frame_network_address_fails_because_mac_transaction_expire_when_not_parent_should_retry(
) {
    let mut harness = started_3x0().await;
    harness.mock.set_data_confirm_code(240, false);
    harness.mock.set_assoc_node_relation(255);
    harness.mock.clear_requests();
    let frame = write_no_rsp_frame();
    let error = harness
        .adapter
        .send_zcl_frame_to_endpoint(
            "0x0000000000000002",
            2,
            20,
            &frame,
            10000,
            false,
            false,
            None,
            None,
        )
        .await
        .expect_err("transaction expired");
    assert!(error.contains("240"), "unexpected error: {error}");
    let commands = harness
        .mock
        .requests()
        .into_iter()
        .map(|request| request.command)
        .collect::<Vec<_>>();
    assert!(
        commands.contains(&"extRouteDisc".to_string()),
        "commands: {commands:?}"
    );
    assert!(
        !commands.contains(&"assocRemove".to_string()),
        "commands: {commands:?}"
    );
}

#[tokio::test]
async fn send_zcl_frame_network_address_fails_because_mac_no_ack_should_retry() {
    let mut harness = started_3x0().await;
    harness.mock.set_data_confirm_code(233, false);
    harness.mock.clear_requests();
    let frame = write_no_rsp_frame();
    let error = harness
        .adapter
        .send_zcl_frame_to_endpoint(
            "0x0000000000000002",
            2,
            20,
            &frame,
            10000,
            false,
            false,
            None,
            None,
        )
        .await
        .expect_err("no ack");
    assert!(error.contains("233"), "unexpected error: {error}");
    let requests = data_requests(&harness);
    assert!(requests.len() >= 3, "requests: {}", requests.len());
    let commands = harness
        .mock
        .requests()
        .into_iter()
        .map(|request| request.command)
        .collect::<Vec<_>>();
    assert!(
        commands.contains(&"extRouteDisc".to_string()),
        "commands: {commands:?}"
    );
}

#[tokio::test]
async fn send_zcl_frame_network_address_with_network_address_change_should_retry() {
    let mut harness = started_3x0().await;
    harness.mock.set_data_confirm_code(233, false);
    harness.mock.clear_requests();
    let frame = write_no_rsp_frame();
    let error = harness
        .adapter
        .send_zcl_frame_to_endpoint(
            "0x0000000000000003",
            2,
            20,
            &frame,
            10000,
            false,
            false,
            None,
            None,
        )
        .await
        .expect_err("no ack");
    assert!(error.contains("233"), "unexpected error: {error}");
    let requests = data_requests(&harness);
    assert!(requests.len() >= 4, "requests: {}", requests.len());
    let last = requests.last().expect("last request");
    assert!(
        last.payload()
            .get("dstaddr")
            .and_then(serde_json::Value::as_u64)
            == Some(3),
        "last payload: {:?}",
        last.payload()
    );
}

#[tokio::test]
async fn send_zcl_frame_network_address_with_network_address_change_without_recovery() {
    let mut harness = started_3x0().await;
    harness.mock.set_data_confirm_code(233, false);
    harness.mock.clear_requests();
    let frame = write_no_rsp_frame();
    let error = harness
        .adapter
        .send_zcl_frame_to_endpoint("0x03", 2, 20, &frame, 10000, false, true, None, None)
        .await
        .expect_err("no ack");
    assert!(error.contains("233"), "unexpected error: {error}");
    assert_eq!(data_requests(&harness).len(), 1);
}

#[tokio::test]
async fn send_zcl_frame_network_address_fails_because_requested_profile_is_not_configured() {
    let mut harness = started_3x0().await;
    let frame = write_no_rsp_frame();
    let error = harness
        .adapter
        .send_zcl_frame_to_endpoint("0x02", 2, 20, &frame, 10000, false, false, None, Some(9999))
        .await
        .expect_err("unsupported profile");
    assert_eq!(error, "Profile ID 9999 is not supported by this adapter.");
}

#[tokio::test]
async fn source_endpoint_is_selected_based_on_profile_id() {
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    let frame = write_no_rsp_frame();
    harness
        .adapter
        .send_zcl_frame_to_endpoint(
            "0x02",
            2,
            20,
            &frame,
            10000,
            false,
            false,
            None,
            Some(0x0109),
        )
        .await
        .expect("send");
    let requests = data_requests(&harness);
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].payload()["srcendpoint"], 6);
}

#[tokio::test]
async fn send_zcl_frame_network_address_should_retry_on_dataconfirm_timeout() {
    let mut harness = started_3x0().await;
    harness.mock.set_data_confirm_code(9999, true);
    harness.mock.clear_requests();
    let frame = write_no_rsp_frame();
    let error = harness
        .adapter
        .send_zcl_frame_to_endpoint("0x02", 2, 20, &frame, 10000, false, false, None, None)
        .await
        .expect_err("timeout");
    assert!(error.contains("9999"), "unexpected error: {error}");
    assert_eq!(data_requests(&harness).len(), 1);
}

#[tokio::test]
async fn send_zcl_frame_group() {
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    let frame = read_frame(true);
    harness
        .adapter
        .send_zcl_frame_to_group(25, &frame, Some(1), None)
        .await
        .expect("group send");
    let requests = harness.mock.requests_named("dataRequestExt");
    assert_eq!(requests.len(), 1);
    let payload = requests[0].payload();
    assert_eq!(payload["dstaddrmode"], 1);
    assert_eq!(payload["dstaddr"], "0x0000000000000019");
    assert_eq!(payload["destendpoint"], 255);
    assert_eq!(payload["srcendpoint"], 1);
    assert_eq!(payload["clusterid"], 0);
    assert_eq!(payload["len"], 5);
    assert_eq!(payload["dstpanid"], 0);
}

#[tokio::test]
async fn send_zcl_frame_group_retry_on_mac_channel_access_failure() {
    let mut harness = started_3x0().await;
    harness.mock.set_data_confirm_code(225, true);
    harness.mock.clear_requests();
    let frame = write_no_rsp_frame();
    harness
        .adapter
        .send_zcl_frame_to_group(25, &frame, Some(1), None)
        .await
        .expect("group send");
    assert_eq!(harness.mock.requests_named("dataRequestExt").len(), 2);
}

#[tokio::test]
async fn send_zcl_frame_group_data_confirm_fails() {
    let mut harness = started_3x0().await;
    harness.mock.set_data_confirm_code(184, false);
    harness.mock.clear_requests();
    let frame = read_frame(true);
    let error = harness
        .adapter
        .send_zcl_frame_to_group(25, &frame, None, None)
        .await
        .expect_err("group data confirm");
    assert!(error.contains("184"), "unexpected error: {error}");
    assert_eq!(harness.mock.requests_named("dataRequestExt").len(), 1);
}

#[tokio::test]
async fn send_zcl_frame_to_all_default_rx_on_when_idle_and_sleepy() {
    use zigbee_apiarist::zspec::enums::BroadcastAddress;
    let mut harness = started_3x0().await;
    for (destination, address) in [
        (BroadcastAddress::DEFAULT, "0x000000000000fffc"),
        (BroadcastAddress::RX_ON_WHEN_IDLE, "0x000000000000fffd"),
        (BroadcastAddress::SLEEPY, "0x000000000000ffff"),
    ] {
        harness.mock.clear_requests();
        let frame = read_frame(true);
        harness
            .adapter
            .send_zcl_frame_to_all(255, &frame, 1, destination as u16, None)
            .await
            .expect("send to all");
        let requests = harness.mock.requests_named("dataRequestExt");
        assert_eq!(requests.len(), 1);
        let payload = requests[0].payload();
        assert_eq!(payload["dstaddrmode"], 2);
        assert_eq!(payload["dstaddr"], address);
        assert_eq!(payload["destendpoint"], 255);
        assert_eq!(payload["srcendpoint"], 1);
        assert_eq!(payload["len"], 5);
    }
}

#[tokio::test]
async fn send_zcl_frame_network_address_transaction_number_should_not_go_higher_than_255() {
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    for _ in 0..300 {
        let frame = write_no_rsp_frame();
        harness
            .adapter
            .send_zcl_frame_to_endpoint("0x02", 2, 20, &frame, 10000, false, false, None, None)
            .await
            .expect("send");
    }
    let transactions = data_requests(&harness)
        .into_iter()
        .map(|request| request.payload()["transid"].as_u64().unwrap_or(999) as u16)
        .collect::<Vec<_>>();
    assert_eq!(transactions.len(), 300);
    assert_eq!(transactions[0], 1);
    assert!(!transactions.contains(&0));
    assert!(transactions.iter().all(|transaction| *transaction <= 255));
    assert_eq!(
        transactions
            .iter()
            .filter(|transaction| **transaction == 1)
            .count(),
        2
    );
    assert_eq!(
        transactions
            .iter()
            .filter(|transaction| **transaction == 255)
            .count(),
        1
    );
}

#[tokio::test]
async fn send_zcl_frame_network_address_matches_by_tsn() {
    use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
    use zigbee_apiarist::zspec::zcl::zcl_header::{Direction, FrameType};
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();

    let mismatch = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        true,
        None,
        102,
        "readRsp",
        0,
        serde_json::json!([{"attrId": 0, "attrData": 5, "dataType": 32, "status": 0}]),
        serde_json::json!({}),
        0,
    );
    let response = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        true,
        None,
        100,
        "readRsp",
        0,
        serde_json::json!([{"attrId": 0, "attrData": 2, "dataType": 32, "status": 0}]),
        serde_json::json!({}),
        0,
    );
    let frame = read_frame(true);
    let result = {
        let request = harness
            .adapter
            .send_zcl_frame_to_endpoint("0x02", 2, 20, &frame, 10000, false, false, None, None);
        tokio::pin!(request);
        for data in [mismatch.to_buffer(), response.to_buffer()] {
            harness.mock.inject_areq(
                zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::AF,
                "incomingMsg",
                serde_json::json!({
                    "clusterid": 0,
                    "srcendpoint": 20,
                    "srcaddr": 2,
                    "linkquality": 101,
                    "groupid": 12,
                    "len": data.len(),
                    "data": data,
                }),
            );
        }
        request.await.expect("send")
    };
    let payload = result.expect("zcl payload");
    assert_eq!(payload.endpoint, 20);
    assert_eq!(payload.group_id, 12);
    assert_eq!(payload.linkquality, 101);
    assert_eq!(payload.address, serde_json::json!(2));
    assert_eq!(payload.data, vec![24, 100, 1, 0, 0, 0, 32, 2]);
    assert_eq!(data_requests(&harness).len(), 1);
}

#[tokio::test]
async fn send_zcl_frame_network_address_matches_on_default_response() {
    use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
    use zigbee_apiarist::zspec::zcl::zcl_header::{Direction, FrameType};
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();

    let mismatch = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        true,
        None,
        102,
        "readRsp",
        0,
        serde_json::json!([{"attrId": 0, "attrData": 5, "dataType": 32, "status": 0}]),
        serde_json::json!({}),
        0,
    );
    let default_response = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        true,
        None,
        100,
        "defaultRsp",
        0,
        serde_json::json!({"cmdId": 0, "statusCode": 0x7e}),
        serde_json::json!({}),
        0,
    );
    let frame = read_frame(false);
    let result = {
        let request = harness
            .adapter
            .send_zcl_frame_to_endpoint("0x02", 2, 20, &frame, 10000, false, false, None, None);
        tokio::pin!(request);
        for data in [mismatch.to_buffer(), default_response.to_buffer()] {
            harness.mock.inject_areq(
                zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::AF,
                "incomingMsg",
                serde_json::json!({
                    "clusterid": 0,
                    "srcendpoint": 20,
                    "srcaddr": 2,
                    "linkquality": 101,
                    "groupid": 12,
                    "len": data.len(),
                    "data": data,
                }),
            );
        }
        request.await.expect("send")
    };
    let payload = result.expect("zcl payload");
    assert_eq!(payload.data, vec![24, 100, 11, 0, 126]);
}

#[tokio::test]
async fn send_zcl_frame_network_address_data_confirm_fails_with_and_without_default_response() {
    for disable_default_response in [false, true] {
        let mut harness = started_3x0().await;
        harness.mock.set_data_confirm_code(201, false);
        let frame = read_frame(disable_default_response);
        let error = harness
            .adapter
            .send_zcl_frame_to_endpoint("0x02", 2, 20, &frame, 10000, false, false, None, None)
            .await
            .expect_err("data confirm");
        assert!(error.contains("201"), "unexpected error: {error}");
    }
}

#[tokio::test]
async fn send_zcl_frame_network_address_with_default_response_timeout_shouldnt_care_because_command_has_response(
) {
    use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
    use zigbee_apiarist::zspec::zcl::zcl_header::{Direction, FrameType};
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    let response = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        true,
        None,
        100,
        "readRsp",
        0,
        serde_json::json!([{"attrId": 0, "attrData": 2, "dataType": 32, "status": 0}]),
        serde_json::json!({}),
        0,
    );
    let frame = read_frame(false);
    let result = {
        let request = harness
            .adapter
            .send_zcl_frame_to_endpoint("0x02", 2, 20, &frame, 10000, false, false, None, None);
        tokio::pin!(request);
        harness.mock.inject_areq(
            zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::AF,
            "incomingMsg",
            serde_json::json!({
                "clusterid": 0,
                "srcendpoint": 20,
                "srcaddr": 2,
                "linkquality": 101,
                "groupid": 12,
                "len": response.to_buffer().len(),
                "data": response.to_buffer(),
            }),
        );
        request.await
    };
    assert!(result.is_ok());
    assert_eq!(data_requests(&harness).len(), 1);
}

use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn backup_fixture(name: &str) -> serde_json::Value {
    let raw = match name {
        "matching" => include_str!("fixtures/adapter_backups/backupMatchingConfig.json"),
        "matching12" => include_str!("fixtures/adapter_backups/backupMatchingConfig12.json"),
        "notMatching" => include_str!("fixtures/adapter_backups/backupNotMatchingConfig.json"),
        "legacy" => include_str!("fixtures/adapter_backups/legacyBackup.json"),
        other => panic!("unknown fixture {other}"),
    };
    serde_json::from_str(raw).expect("backup fixture")
}

fn temp_backup(name: &str, value: &serde_json::Value) -> String {
    let path = std::env::temp_dir().join(format!(
        "zstack_adapter_{name}_{}_{}.json",
        std::process::id(),
        TEMP_FILE_COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::write(&path, serde_json::to_string(value).expect("json")).expect("write backup");
    path.to_string_lossy().to_string()
}

fn temp_backup_raw(name: &str, contents: &str) -> String {
    let path = std::env::temp_dir().join(format!(
        "zstack_adapter_{name}_{}_{}.json",
        std::process::id(),
        TEMP_FILE_COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::write(&path, contents).expect("write backup");
    path.to_string_lossy().to_string()
}

fn sec_material_entry(pan: [u8; 8], frame_counter: u32) -> Vec<u8> {
    use zigbee_apiarist::adapter::z_stack::structs::entries::nwk_sec_material_descriptor_entry::nwk_sec_material_descriptor_entry;
    use zigbee_apiarist::adapter::z_stack::structs::r#struct::Alignment;
    use zigbee_apiarist::adapter::z_stack::structs::serializable_memory_object::SerializableMemoryObject;
    let mut entry = nwk_sec_material_descriptor_entry(None);
    entry.set_u32("FrameCounter", frame_counter);
    entry.set_bytes("extendedPanID", &pan);
    entry.serialize(Alignment::Aligned, true, 0)
}

#[tokio::test]
async fn should_restore_unified_backup_with_3_0_x_adapter_and_create_backup_empty() {
    let path = temp_backup("restore_empty", &backup_fixture("matching"));
    let mut harness = harness(default_network_options(), &path);
    fixtures::empty_3_aligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Restored);
    harness.adapter.backup(&[]).await.expect("backup");
}

#[tokio::test]
async fn should_restore_unified_backup_with_3_0_x_adapter_and_create_backup_no_tclk_seed() {
    let mut backup = backup_fixture("matching");
    backup["stack_specific"] = serde_json::json!({});
    let path = temp_backup("restore_no_tclk", &backup);
    let mut harness = harness(default_network_options(), &path);
    fixtures::empty_3_aligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Restored);
    harness.adapter.backup(&[]).await.expect("backup");
}

#[tokio::test]
async fn should_restore_unified_backup_with_3_x_0_adapter_and_create_backup_empty() {
    let path = temp_backup("restore_3x0_empty", &backup_fixture("matching"));
    let mut harness = harness(default_network_options(), &path);
    fixtures::empty_3x0_aligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Restored);
    harness.adapter.backup(&[]).await.expect("backup");
}

#[tokio::test]
async fn should_recommission_restore_unified_backup_with_1_2_adapter_and_create_backup_empty() {
    let path = temp_backup("restore_12_empty", &backup_fixture("matching12"));
    let mut harness = Harness::new(
        default_network_options(),
        &path,
        AdapterOptions {
            concurrent: Some(1),
            ..AdapterOptions::default()
        },
    );
    fixtures::empty_12_unaligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Restored);
    let backup = harness.adapter.backup(&[]).await.expect("backup");
    assert_eq!(backup.network_key_info.frame_counter, 0);
}

#[tokio::test]
async fn should_create_backup_with_3_0_x_adapter_default_security_material_table_entry() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_3_aligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Resumed);
    for index in 0..4 {
        harness.mock.nv(
            zigbee_apiarist::adapter::z_stack::constants::common::NvItemsIds::LEGACY_NWK_SEC_MATERIAL_TABLE_START + index,
            Some(vec![0u8; 12]),
        );
    }
    harness.mock.nv(
        zigbee_apiarist::adapter::z_stack::constants::common::NvItemsIds::LEGACY_NWK_SEC_MATERIAL_TABLE_START,
        Some(sec_material_entry([0xff; 8], 2800)),
    );
    let backup = harness.adapter.backup(&[]).await.expect("backup");
    assert_eq!(backup.network_key_info.frame_counter, 2800);
}

#[tokio::test]
async fn should_create_backup_with_3_0_x_adapter_empty_security_material_table() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_3_aligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Resumed);
    for index in 0..4 {
        harness.mock.nv(
            zigbee_apiarist::adapter::z_stack::constants::common::NvItemsIds::LEGACY_NWK_SEC_MATERIAL_TABLE_START + index,
            Some(vec![0u8; 12]),
        );
    }
    let backup = harness.adapter.backup(&[]).await.expect("backup");
    assert_eq!(backup.network_key_info.frame_counter, 1250);
}

#[tokio::test]
async fn should_create_backup_with_3_0_x_adapter_security_material_table_with_generic_record() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_3_aligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Resumed);
    for index in 0..4 {
        harness.mock.nv(
            zigbee_apiarist::adapter::z_stack::constants::common::NvItemsIds::LEGACY_NWK_SEC_MATERIAL_TABLE_START + index,
            Some(vec![0u8; 12]),
        );
    }
    harness.mock.nv(
        zigbee_apiarist::adapter::z_stack::constants::common::NvItemsIds::LEGACY_NWK_SEC_MATERIAL_TABLE_START + 3,
        Some(sec_material_entry([0xff; 8], 8737)),
    );
    let backup = harness.adapter.backup(&[]).await.expect("backup");
    assert_eq!(backup.network_key_info.frame_counter, 8737);
}

#[tokio::test]
async fn should_create_backup_with_1_2_adapter() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_12_unaligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Resumed);
    let backup = harness.adapter.backup(&[]).await.expect("backup");
    assert_eq!(backup.network_key_info.frame_counter, 0);
}

#[tokio::test]
async fn should_keep_missing_devices_in_backup() {
    let path = temp_backup("missing_devices", &backup_fixture("matching"));
    let mut harness = harness(default_network_options(), &path);
    fixtures::empty_3_aligned(&harness.mock);
    harness.start().await.expect("start");

    let mut backup_with_missing = backup_fixture("matching");
    backup_with_missing["devices"]
        .as_array_mut()
        .expect("devices")
        .push(serde_json::json!({
            "nwk_address": "20fa",
            "ieee_address": "00128d11124fa80b",
            "link_key": {
                "key": "bff550908aa1529ee90eea3c3bdc26fc",
                "rx_counter": 0,
                "tx_counter": 2,
            },
        }));
    std::fs::write(&path, serde_json::to_string(&backup_with_missing).unwrap()).expect("write");

    let devices = backup_with_missing["devices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|device| format!("0x{}", device["ieee_address"].as_str().unwrap()))
        .collect::<Vec<_>>();
    eprintln!(
        "FILE {:?}",
        std::fs::read_to_string(&path)
            .unwrap()
            .contains("00128d11124fa80b")
    );
    eprintln!(
        "PARSED {:?}",
        zigbee_apiarist::utils::backup::from_unified_backup(
            &serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
        )
        .map(|b| b.devices.len())
    );
    let backup = harness.adapter.backup(&devices).await.expect("backup");
    eprintln!(
        "DEVICES {:?}",
        backup
            .devices
            .iter()
            .map(|d| (
                d.ieee_address
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>(),
                d.link_key.is_some()
            ))
            .collect::<Vec<_>>()
    );
    let missing = backup
        .devices
        .iter()
        .find(|device| {
            device
                .ieee_address
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
                == "00128d11124fa80b"
        })
        .expect("missing device");
    assert_eq!(
        backup.devices.len(),
        backup_with_missing["devices"].as_array().unwrap().len()
    );
    assert_eq!(
        missing.link_key.as_ref().expect("link key").key,
        fixtures::hex("bff550908aa1529ee90eea3c3bdc26fc")
    );
}

#[tokio::test]
async fn should_fail_when_backup_file_is_corrupted() {
    let path = temp_backup_raw("corrupted", "{");
    let mut harness = harness(default_network_options(), &path);
    fixtures::empty_3_aligned(&harness.mock);
    let error = harness.start().await.expect_err("corrupted backup");
    assert!(
        error.contains("Coordinator backup is corrupted"),
        "unexpected: {error}"
    );
}

#[tokio::test]
async fn should_fail_to_restore_unified_backup_with_invalid_backup_version() {
    let mut backup = backup_fixture("matching");
    backup["metadata"]["version"] = serde_json::json!(99);
    let path = temp_backup("bad_version", &backup);
    let mut harness = harness(default_network_options(), &path);
    fixtures::empty_3_aligned(&harness.mock);
    let error = harness.start().await.expect_err("invalid version");
    assert_eq!(
        error,
        "Unsupported open coordinator backup version (version=99)"
    );
}

#[tokio::test]
async fn should_fail_to_restore_unified_backup_with_unsupported_backup_format() {
    let mut backup = backup_fixture("matching");
    backup["metadata"]
        .as_object_mut()
        .unwrap()
        .remove("version");
    let path = temp_backup("bad_format", &backup);
    let mut harness = harness(default_network_options(), &path);
    fixtures::empty_3_aligned(&harness.mock);
    let error = harness.start().await.expect_err("unknown format");
    assert_eq!(error, "Unknown backup format");
}

#[tokio::test]
async fn should_fail_to_restore_unified_backup_with_insufficient_tclk_table_size() {
    let path = temp_backup("small_tclk", &backup_fixture("matching"));
    let mut harness = harness(default_network_options(), &path);
    fixtures::empty_3_aligned(&harness.mock);
    for index in 0..16 {
        harness.mock.nv(
            zigbee_apiarist::adapter::z_stack::constants::common::NvItemsIds::LEGACY_TCLK_TABLE_START + index,
            None,
        );
    }
    harness.mock.nv(
        zigbee_apiarist::adapter::z_stack::constants::common::NvItemsIds::LEGACY_TCLK_TABLE_START,
        Some(vec![0u8; 20]),
    );
    let error = harness.start().await.expect_err("tclk size");
    assert_eq!(
        error,
        "target adapter tclk table size insufficient (size=1)"
    );
}

#[tokio::test]
async fn should_fail_to_restore_unified_backup_with_insufficient_aps_link_key_data_table_size() {
    let path = temp_backup("small_aps", &backup_fixture("matching"));
    let mut harness = harness(default_network_options(), &path);
    fixtures::empty_3_aligned(&harness.mock);
    for index in 0..16 {
        harness.mock.nv(
            zigbee_apiarist::adapter::z_stack::constants::common::NvItemsIds::APS_LINK_KEY_DATA_START + index,
            None,
        );
    }
    harness.mock.nv(
        zigbee_apiarist::adapter::z_stack::constants::common::NvItemsIds::APS_LINK_KEY_DATA_START,
        Some(vec![0u8; 24]),
    );
    let error = harness.start().await.expect_err("aps size");
    assert_eq!(
        error,
        "target adapter aps link key data table size insufficient (size=1)"
    );
}

#[tokio::test]
async fn should_fail_to_restore_unified_backup_with_insufficient_security_manager_table_size() {
    let path = temp_backup("small_security", &backup_fixture("matching"));
    let mut harness = harness(default_network_options(), &path);
    fixtures::empty_3_aligned(&harness.mock);
    harness.mock.nv(
        zigbee_apiarist::adapter::z_stack::constants::common::NvItemsIds::APS_LINK_KEY_TABLE,
        Some(fixtures::hex("0000feff00000000")),
    );
    let error = harness.start().await.expect_err("security size");
    assert_eq!(
        error,
        "target adapter security manager table size insufficient (size=1)"
    );
}

#[tokio::test]
async fn should_fail_to_restore_unified_backup_with_1_2_adapter_backup_from_newer_adapter() {
    let path = temp_backup("newer_backup_12", &backup_fixture("matching"));
    let mut harness = harness(default_network_options(), &path);
    fixtures::empty_12_unaligned(&harness.mock);
    let error = harness.start().await.expect_err("newer backup");
    assert_eq!(
        error,
        "your backup is from newer platform version (Z-Stack 3.0.x+) and cannot be restored onto Z-Stack 1.2 adapter - please remove backup before proceeding"
    );
}

#[tokio::test]
async fn should_fail_to_create_backup_when_unable_to_read_ieee_address() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_3_aligned(&harness.mock);
    harness.mock.handle(
        zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::SYS,
        "getExtAddr",
        |_, _| support::Reply::raw(Vec::new()),
    );
    assert_eq!(harness.start().await.expect("start"), StartResult::Resumed);
    let error = harness.adapter.backup(&[]).await.expect_err("ieee");
    assert_eq!(error, "Failed to read adapter IEEE address");
}

#[tokio::test]
async fn should_fail_to_create_backup_when_adapter_not_commissioned_missing_nib() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::empty_3_aligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Reset);
    harness
        .mock
        .remove_nv(zigbee_apiarist::adapter::z_stack::constants::common::NvItemsIds::NIB);
    let error = harness.adapter.backup(&[]).await.expect_err("missing nib");
    assert_eq!(error, "Cannot backup - adapter not commissioned");
}

#[tokio::test]
async fn should_fail_to_create_backup_when_missing_active_key_info() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::empty_3_aligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Reset);
    harness.mock.remove_nv(
        zigbee_apiarist::adapter::z_stack::constants::common::NvItemsIds::NWK_ACTIVE_KEY_INFO,
    );
    let error = harness.adapter.backup(&[]).await.expect_err("missing key");
    assert_eq!(error, "Cannot backup - missing active key info");
}
