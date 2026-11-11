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
async fn soft_reset_uses_reset_type_one() {
    // Anchor: adapter.test.ts "Soft reset".
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    harness.adapter.reset("soft").await.expect("soft reset");
    let requests = harness.mock.requests_named("resetReq");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].payload()["type"], 1);
}

#[tokio::test]
async fn hard_reset_uses_reset_type_zero() {
    // Anchor: adapter.test.ts "Hard reset".
    let mut harness = started_3x0().await;
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
    assert_eq!(
        error,
        "Data request failed with error: 'NWK_UNSUPPORTED_ATTRIBUTE' (0xc9)"
    );
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
    assert_eq!(
        error,
        "Data request failed with error: 'MAC_TRANSACTION_EXPIRED' (0xf0)"
    );
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
    assert_eq!(
        error,
        "Data request failed with error: 'MAC_TRANSACTION_EXPIRED' (0xf0)"
    );
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
    assert_eq!(error, "Data request failed with error: 'MAC_NO_ACK' (0xe9)");
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
    assert_eq!(error, "Data request failed with error: 'MAC_NO_ACK' (0xe9)");
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
    assert_eq!(error, "Data request failed with error: 'MAC_NO_ACK' (0xe9)");
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
    assert_eq!(error, "Data request failed with error: 'TIMEOUT'");
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
    assert_eq!(
        error,
        "Data request failed with error: 'APS_DUPLICATE_ENTRY' (0xb8)"
    );
    assert_eq!(harness.mock.requests_named("dataRequestExt").len(), 1);
}

async fn send_to_all(destination: u16, address: &str) {
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    let frame = read_frame(true);
    harness
        .adapter
        .send_zcl_frame_to_all(255, &frame, 1, destination, None)
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

#[tokio::test]
async fn send_zcl_frame_to_all_default() {
    // Anchor: adapter.test.ts "Send zcl frame to all - DEFAULT".
    send_to_all(
        zigbee_apiarist::zspec::enums::BroadcastAddress::DEFAULT as u16,
        "0x000000000000fffc",
    )
    .await;
}

#[tokio::test]
async fn send_zcl_frame_to_all_rx_on_when_idle() {
    // Anchor: adapter.test.ts "Send zcl frame to all - RX_ON_WHEN_IDLE".
    send_to_all(
        zigbee_apiarist::zspec::enums::BroadcastAddress::RX_ON_WHEN_IDLE as u16,
        "0x000000000000fffd",
    )
    .await;
}

#[tokio::test]
async fn send_zcl_frame_to_all_sleepy() {
    // Anchor: adapter.test.ts "Send zcl frame to all - SLEEPY".
    send_to_all(
        zigbee_apiarist::zspec::enums::BroadcastAddress::SLEEPY as u16,
        "0x000000000000ffff",
    )
    .await;
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

async fn data_confirm_fails_case(disable_default_response: bool) {
    let mut harness = started_3x0().await;
    harness.mock.set_data_confirm_code(201, false);
    let frame = read_frame(disable_default_response);
    let error = harness
        .adapter
        .send_zcl_frame_to_endpoint("0x02", 2, 20, &frame, 10000, false, false, None, None)
        .await
        .expect_err("data confirm");
    assert_eq!(
        error,
        "Data request failed with error: 'NWK_UNSUPPORTED_ATTRIBUTE' (0xc9)"
    );
}

#[tokio::test]
async fn send_zcl_frame_network_address_data_confirm_fails_with_default_response() {
    // Anchor: adapter.test.ts "... data confirm fails with default response".
    data_confirm_fails_case(false).await;
}

#[tokio::test]
async fn send_zcl_frame_network_address_data_confirm_fails_without_default_response() {
    // Anchor: adapter.test.ts "... data confirm fails without default response".
    data_confirm_fails_case(true).await;
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

#[tokio::test]
async fn should_restore_legacy_backup_with_3_0_x_adapter_empty() {
    let path = temp_backup("legacy_empty", &backup_fixture("legacy"));
    let mut harness = harness(default_network_options(), &path);
    fixtures::empty_3_aligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Restored);
}

fn legacy_without(key: &str) -> serde_json::Value {
    let mut backup = backup_fixture("legacy");
    backup["data"].as_object_mut().unwrap().remove(key);
    backup
}

#[tokio::test]
async fn should_fail_to_restore_legacy_backup_missing_nib() {
    let path = temp_backup("legacy_no_nib", &legacy_without("ZCD_NV_NIB"));
    let mut harness = harness(default_network_options(), &path);
    fixtures::empty_3_aligned(&harness.mock);
    let error = harness.start().await.expect_err("missing nib");
    assert_eq!(error, "Backup corrupted - missing NIB");
}

#[tokio::test]
async fn should_fail_to_restore_legacy_backup_missing_active_key_info() {
    let path = temp_backup(
        "legacy_no_active_key",
        &legacy_without("ZCD_NV_NWK_ACTIVE_KEY_INFO"),
    );
    let mut harness = harness(default_network_options(), &path);
    fixtures::empty_3_aligned(&harness.mock);
    let error = harness.start().await.expect_err("missing key");
    assert_eq!(error, "Backup corrupted - missing active key info");
}

#[tokio::test]
async fn should_fail_to_restore_legacy_backup_missing_preconfigured_key_enabled() {
    let path = temp_backup(
        "legacy_no_precfg_enable",
        &legacy_without("ZCD_NV_PRECFGKEY_ENABLE"),
    );
    let mut harness = harness(default_network_options(), &path);
    fixtures::empty_3_aligned(&harness.mock);
    let error = harness.start().await.expect_err("missing preconfigured");
    assert_eq!(
        error,
        "Backup corrupted - missing pre-configured key enable attribute"
    );
}

#[tokio::test]
async fn should_fail_to_restore_legacy_backup_preconfigured_key_enabled_missing_material_table() {
    let mut backup = backup_fixture("legacy");
    backup["data"]
        .as_object_mut()
        .unwrap()
        .remove("ZCD_NV_EX_NWK_SEC_MATERIAL_TABLE");
    backup["data"]
        .as_object_mut()
        .unwrap()
        .remove("ZCD_NV_LEGACY_NWK_SEC_MATERIAL_TABLE_START");
    let path = temp_backup("legacy_no_material", &backup);
    let mut harness = harness(default_network_options(), &path);
    fixtures::empty_3_aligned(&harness.mock);
    let error = harness.start().await.expect_err("missing material");
    assert_eq!(
        error,
        "Backup corrupted - missing network security material table"
    );
}

#[tokio::test]
async fn should_fail_to_restore_legacy_backup_missing_adapter_ieee_address() {
    let path = temp_backup("legacy_no_ieee", &legacy_without("ZCD_NV_EXTADDR"));
    let mut harness = harness(default_network_options(), &path);
    fixtures::empty_3_aligned(&harness.mock);
    let error = harness.start().await.expect_err("missing ieee");
    assert_eq!(
        error,
        "Backup corrupted - missing adapter IEEE address NV entry"
    );
}

#[tokio::test]
async fn should_fail_to_start_with_commissioned_config_adapter_mismatch() {
    let path = temp_backup("mismatch", &backup_fixture("matching"));
    let mut harness = harness(network_options(124, EXTENDED_PAN_ID, 21), &path);
    fixtures::commissioned_3_aligned(&harness.mock);
    let error = harness.start().await.expect_err("mismatch");
    assert_eq!(
        error,
        "startup failed - configuration-adapter mismatch - see logs above for more information"
    );
    let messages = support::log_messages();
    for expected in [
        "Configuration is not consistent with adapter state/backup!",
        "- PAN ID: configured=124, adapter=123",
        "- Extended PAN ID: configured=00124b0009d69f77, adapter=00124b0009d69f77",
        "- Network Key: configured=01030507090b0d0f00020406080a0c0d, adapter:active=01030507090b0d0f00020406080a0c0d adapter:preconfigured=01030507090b0d0f00020406080a0c0d, adapter:alternate=01030507090b0d0f00020406080a0c0d",
        "- Channel List: configured=21, adapter=21",
        "Please update configuration to prevent further issues.",
        "Re-commissioning your network will require re-pairing of all devices!",
    ] {
        assert!(
            messages.iter().any(|message| message == expected),
            "missing log: {expected}\nlogs: {messages:#?}"
        );
    }
    assert!(messages.iter().any(|message| message.starts_with(
        "If you wish to re-commission your network, please remove coordinator backup at "
    )));
}

#[tokio::test]
async fn should_start_with_run_inconsistent_option_on_config_adapter_mismatch() {
    let path = temp_backup("mismatch_run", &backup_fixture("matching"));
    let mut harness = Harness::new(
        network_options(124, EXTENDED_PAN_ID, 21),
        &path,
        AdapterOptions {
            concurrent: Some(3),
            force_start_with_inconsistent_adapter_configuration: true,
            ..AdapterOptions::default()
        },
    );
    fixtures::commissioned_3_aligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Resumed);
    let messages = support::log_messages();
    assert!(messages
        .iter()
        .any(|message| message == "Configuration is not consistent with adapter state/backup!"));
    assert!(messages.iter().any(|message| message
        == "Running despite adapter configuration mismatch as configured. Please update the adapter to compatible firmware and recreate your network as soon as possible."));
}

#[tokio::test]
async fn should_start_with_backward_compat_reversed_extended_pan_id() {
    use zigbee_apiarist::adapter::z_stack::structs::entries::nib::nib;
    use zigbee_apiarist::adapter::z_stack::structs::r#struct::Alignment;
    use zigbee_apiarist::adapter::z_stack::structs::serializable_memory_object::SerializableMemoryObject;

    let path = temp_backup("reversed_pan", &backup_fixture("matching"));
    let mut harness = harness(default_network_options(), &path);
    fixtures::commissioned_3_aligned(&harness.mock);
    let mut nib_struct = nib(Some(&fixtures::hex(fixtures::NIB_COMMISSIONED_3_ALIGNED)));
    let mut extended = nib_struct.get_bytes("extendedPANID").expect("extended pan");
    extended.reverse();
    nib_struct.set_bytes("extendedPANID", &extended);
    harness.mock.nv(
        zigbee_apiarist::adapter::z_stack::constants::common::NvItemsIds::NIB,
        Some(nib_struct.serialize(Alignment::Aligned, true, 0)),
    );
    assert_eq!(harness.start().await.expect("start"), StartResult::Resumed);
    let messages = support::log_messages();
    assert!(
        messages.iter().any(|message| message
            == "Extended PAN ID is reversed (expected=00124b0009d69f77, actual=779fd609004b1200)"),
        "logs: {messages:#?}"
    );
}

#[tokio::test]
async fn should_start_with_3_x_0_adapter_warn_when_preconfigured_key_does_not_match() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_3x0_precfg_key_mismatch(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Resumed);
    let messages = support::log_messages();
    assert!(
        messages.iter().any(|message| message
            == "Adapter preconfigured (transport) key does not match configured network key (preconfigured=aabbccddeeff00112233445566778899, configured=01030507090b0d0f00020406080a0c0d). This is typically harmless on Z-Stack 3.x adapters where only the active key matters."),
        "logs: {messages:#?}"
    );
}

#[tokio::test]
async fn should_restore_unified_backup_with_mismatched_adapter_config_but_matching_config_backup() {
    let path = temp_backup("mismatch_restore", &backup_fixture("matching"));
    let mut harness = harness(default_network_options(), &path);
    fixtures::commissioned_3_aligned_config_mismatch(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Restored);
}

#[tokio::test]
async fn should_commission_network_with_1_2_adapter_old_adapter_without_version_reporting() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::empty_12_unaligned(&harness.mock);
    harness.mock.handle(
        zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::SYS,
        "version",
        |_, _| support::Reply::raw(Vec::new()),
    );
    assert_eq!(harness.start().await.expect("start"), StartResult::Restored);
}

fn dummy_capabilities() -> serde_json::Value {
    serde_json::json!({
        "allocateAddress": 0,
        "alternatePANCoordinator": 0,
        "deviceType": 1,
        "powerSource": 0,
        "reserved1": 0,
        "reserved2": 0,
        "rxOnWhenIdle": 0,
        "securityCapability": 0,
    })
}

fn read_rsp_frame(tsn: u8, attr_data: u32) -> zigbee_apiarist::zspec::zcl::zcl_frame::Frame {
    use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
    use zigbee_apiarist::zspec::zcl::zcl_header::{Direction, FrameType};
    Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        true,
        None,
        tsn,
        "readRsp",
        0,
        serde_json::json!([{"attrId": 0, "attrData": attr_data, "dataType": 32, "status": 0}]),
        serde_json::json!({}),
        0,
    )
}

fn touchlink_frame(
    command: &'static str,
    direction: u8,
    payload: serde_json::Value,
) -> zigbee_apiarist::zspec::zcl::zcl_frame::Frame {
    use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
    use zigbee_apiarist::zspec::zcl::zcl_header::FrameType;
    Frame::create(
        FrameType::SPECIFIC as u8,
        direction,
        false,
        None,
        12,
        command,
        "touchlink",
        payload,
        serde_json::json!({}),
        0,
    )
}

async fn incoming_extended_interpan_case(src_addr_mode: u64, expected: serde_json::Value) {
    use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::AdapterEvent;
    use zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem;
    let mut harness = started_3x0().await;
    let mut events = harness.adapter.events.subscribe();
    let response = read_rsp_frame(100, 2);
    harness.mock.inject_areq(
        Subsystem::AF,
        "incomingMsgExt",
        serde_json::json!({
            "groupid": 12,
            "clusterid": 0,
            "srcaddrmode": src_addr_mode,
            "srcaddr": "0x1234567812345678",
            "srcendpoint": 20,
            "dstendpoint": 1,
            "linkquality": 101,
            "timestamp": 0x12345678,
            "len": response.to_buffer().len(),
            "data": response.to_buffer(),
        }),
    );
    let payload = loop {
        match events.recv().await.expect("event") {
            AdapterEvent::ZclPayload(payload) => break payload,
            _ => continue,
        }
    };
    assert_eq!(payload.endpoint, 20);
    assert_eq!(payload.group_id, 12);
    assert_eq!(payload.linkquality, 101);
    assert_eq!(payload.address, expected);
    assert_eq!(payload.data, vec![24, 100, 1, 0, 0, 0, 32, 2]);
    assert_eq!(payload.header.expect("header").command_identifier, 1);
}

#[tokio::test]
async fn incoming_message_extended_interpan_16_bit_address() {
    // Anchor: adapter.test.ts "Incoming message extended (inter PAN) - 16/64-bit address" (16-bit).
    incoming_extended_interpan_case(2, serde_json::json!(0x5678)).await;
}

#[tokio::test]
async fn incoming_message_extended_interpan_64_bit_address() {
    // Anchor: adapter.test.ts "Incoming message extended (inter PAN) - 16/64-bit address" (64-bit).
    incoming_extended_interpan_case(3, serde_json::json!("0x1234567812345678")).await;
}

#[tokio::test]
async fn incoming_message_raw_not_zcl() {
    use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::AdapterEvent;
    use zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem;
    let mut harness = started_3x0().await;
    let mut events = harness.adapter.events.subscribe();
    harness.mock.inject_areq(
        Subsystem::AF,
        "incomingMsg",
        serde_json::json!({
            "clusterid": 1,
            "srcendpoint": 20,
            "srcaddr": 2,
            "linkquality": 101,
            "groupid": 12,
            "len": 2,
            "data": [0x00, 0x01],
        }),
    );
    let payload = loop {
        match events.recv().await.expect("event") {
            AdapterEvent::ZclPayload(payload) => break payload,
            _ => continue,
        }
    };
    assert_eq!(payload.cluster_id, 1);
    assert_eq!(payload.endpoint, 20);
    assert_eq!(payload.group_id, 12);
    assert_eq!(payload.linkquality, 101);
    assert_eq!(payload.address, serde_json::json!(2));
    assert_eq!(payload.data, vec![0x00, 0x01]);
    assert!(payload.header.is_none());
}

#[tokio::test]
async fn device_joined_event() {
    use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::AdapterEvent;
    use zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem;
    let mut harness = started_3x0().await;
    let mut events = harness.adapter.events.subscribe();
    harness.mock.inject_areq(
        Subsystem::ZDO,
        "tcDeviceInd",
        serde_json::json!({"nwkaddr": 123, "extaddr": "0x0000000000000123"}),
    );
    let (network_address, ieee) = loop {
        match events.recv().await.expect("event") {
            AdapterEvent::DeviceJoined {
                network_address,
                ieee_address,
            } => break (network_address, ieee_address),
            _ => continue,
        }
    };
    assert_eq!(network_address, 123);
    assert_eq!(ieee, serde_json::json!("0x0000000000000123"));
}

#[tokio::test]
async fn device_announce_event() {
    // Anchor: adapter.test.ts "Device announce".
    use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::AdapterEvent;
    use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;
    let mut harness = started_3x0().await;
    let mut events = harness.adapter.events.subscribe();
    harness.mock.inject_zdo(
        ClusterId::END_DEVICE_ANNOUNCE,
        Some(123),
        &[
            serde_json::json!(0),
            serde_json::json!({
                "capabilities": dummy_capabilities(),
                "eui64": "0x0000000000000123",
                "nwkAddress": 123,
            }),
        ],
    );
    let response = loop {
        match events.recv().await.expect("event") {
            AdapterEvent::ZdoResponse {
                cluster_id,
                response,
            } => break (cluster_id, response),
            _ => continue,
        }
    };
    assert_eq!(response.0, ClusterId::END_DEVICE_ANNOUNCE);
    assert_eq!(response.1[0], 0);
    assert_eq!(response.1[1]["eui64"], "0x0000000000000123");
    assert_eq!(response.1[1]["nwkAddress"], 123);
    assert_eq!(response.1[1]["capabilities"], dummy_capabilities());
}

#[tokio::test]
async fn network_address_response_event() {
    // Anchor: adapter.test.ts "Network address response".
    use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::AdapterEvent;
    use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;
    let mut harness = started_3x0().await;
    let mut events = harness.adapter.events.subscribe();
    harness.mock.inject_zdo(
        ClusterId::NETWORK_ADDRESS_RESPONSE,
        Some(124),
        &[
            serde_json::json!(0),
            serde_json::json!({
                "eui64": "0x0000000000000123",
                "nwkAddress": 124,
                "assocDevList": [],
                "startIndex": 0,
            }),
        ],
    );
    let response = loop {
        match events.recv().await.expect("event") {
            AdapterEvent::ZdoResponse {
                cluster_id,
                response,
            } => break (cluster_id, response),
            _ => continue,
        }
    };
    assert_eq!(response.0, ClusterId::NETWORK_ADDRESS_RESPONSE);
    assert_eq!(response.1[0], 0);
    assert_eq!(response.1[1]["eui64"], "0x0000000000000123");
    assert_eq!(response.1[1]["nwkAddress"], 124);
}

#[tokio::test]
async fn device_announce_should_discover_route_to_end_devices() {
    use zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem;
    use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;

    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    let mut capabilities = dummy_capabilities();
    capabilities["deviceType"] = serde_json::json!(0);
    for _ in 0..2 {
        harness.mock.inject_zdo(
            ClusterId::END_DEVICE_ANNOUNCE,
            Some(123),
            &[
                serde_json::json!(0),
                serde_json::json!({
                    "capabilities": capabilities,
                    "eui64": "0x0000000000000123",
                    "nwkAddress": 123,
                }),
            ],
        );
    }
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let routes = harness.mock.requests_named("extRouteDisc");
    assert_eq!(routes.len(), 1, "route discovery should be debounced");
    let _ = Subsystem::ZDO;
}

#[tokio::test]
async fn concentrator_callback_indication() {
    use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::AdapterEvent;
    use zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem;
    use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;
    let mut harness = started_3x0().await;
    let mut events = harness.adapter.events.subscribe();
    harness.mock.inject_areq(
        Subsystem::ZDO,
        "concentratorIndCb",
        serde_json::json!({"srcaddr": 124, "extaddr": "0x0000000000000124"}),
    );
    let response = loop {
        match events.recv().await.expect("event") {
            AdapterEvent::ZdoResponse {
                cluster_id,
                response,
            } => break (cluster_id, response),
            _ => continue,
        }
    };
    assert_eq!(response.0, ClusterId::NETWORK_ADDRESS_RESPONSE);
    assert_eq!(response.1[1]["eui64"], "0x0000000000000124");
}

#[tokio::test]
async fn device_leave_event() {
    // Anchor: adapter.test.ts "Device leave".
    use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::AdapterEvent;
    use zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem;
    let mut harness = started_3x0().await;
    let mut events = harness.adapter.events.subscribe();
    harness.mock.inject_areq(
        Subsystem::ZDO,
        "leaveInd",
        serde_json::json!({"srcaddr": 123, "extaddr": "0x0000000000000123"}),
    );
    let (network_address, ieee) = loop {
        match events.recv().await.expect("event") {
            AdapterEvent::DeviceLeave {
                network_address,
                ieee_address,
            } => break (network_address, ieee_address),
            _ => continue,
        }
    };
    assert_eq!(network_address, 123);
    assert_eq!(ieee, serde_json::json!("0x0000000000000123"));
}

#[tokio::test]
async fn ignore_device_leave_with_rejoin() {
    // Anchor: adapter.test.ts "Ignore device leave with rejoin".
    use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::AdapterEvent;
    use zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem;
    let mut harness = started_3x0().await;
    let mut events = harness.adapter.events.subscribe();
    harness.mock.inject_areq(
        Subsystem::ZDO,
        "leaveInd",
        serde_json::json!({"srcaddr": 123, "extaddr": "0x0000000000000123", "rejoin": 1}),
    );
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    while let Ok(event) = events.try_recv() {
        assert!(!matches!(event, AdapterEvent::DeviceLeave { .. }));
    }
}

#[tokio::test]
async fn do_nothing_with_non_areq_event() {
    use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::AdapterEvent;
    use zigbee_apiarist::adapter::z_stack::unpi::constants::{Subsystem, Type};
    let mut harness = started_3x0().await;
    let mut events = harness.adapter.events.subscribe();
    harness.mock.inject_typed(
        Type::SREQ,
        Subsystem::ZDO,
        "leaveInd",
        serde_json::json!({"srcaddr": 123, "extaddr": "0x0000000000000123"}),
    );
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    while let Ok(event) = events.try_recv() {
        assert!(!matches!(event, AdapterEvent::DeviceLeave { .. }));
    }
}

#[tokio::test]
async fn get_network_parameters() {
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    let result = harness
        .adapter
        .get_network_parameters()
        .await
        .expect("network parameters");
    assert_eq!(harness.mock.requests_named("extNwkInfo").len(), 1);
    assert_eq!(result.channel, 21);
    assert_eq!(result.pan_id, 123);
    assert_eq!(result.extended_pan_id, "0x00124b0009d69f77");
    assert_eq!(result.nwk_update_id, 0);
}

#[tokio::test]
async fn set_interpan_channel() {
    // Anchor: adapter.test.ts "Set interpan channel".
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    harness
        .adapter
        .set_channel_inter_pan(14)
        .await
        .expect("set channel");
    let requests = harness.mock.requests_named("interPanCtl");
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].payload()["cmd"], 1);
    assert_eq!(support::bytes(&requests[0].payload()["data"]), vec![14]);
    assert_eq!(requests[1].payload()["cmd"], 2);
    assert_eq!(support::bytes(&requests[1].payload()["data"]), vec![12]);

    harness.mock.clear_requests();
    harness
        .adapter
        .set_channel_inter_pan(15)
        .await
        .expect("set channel");
    let requests = harness.mock.requests_named("interPanCtl");
    assert_eq!(requests.len(), 1);
    assert_eq!(support::bytes(&requests[0].payload()["data"]), vec![15]);
}

#[tokio::test]
async fn restore_interpan_channel() {
    // Anchor: adapter.test.ts "Restore interpan channel".
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    harness
        .adapter
        .restore_channel_inter_pan()
        .await
        .expect("restore channel");
    let requests = harness.mock.requests_named("interPanCtl");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].payload()["cmd"], 0);
    assert!(support::bytes(&requests[0].payload()["data"]).is_empty());
}

#[tokio::test]
async fn send_zcl_frame_interpan_to_ieee_address() {
    use zigbee_apiarist::zspec::zcl::zcl_header::Direction;
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    let frame = touchlink_frame(
        "identifyRequest",
        Direction::CLIENT_TO_SERVER as u8,
        serde_json::json!({"transactionID": 1, "duration": 65535}),
    );
    harness
        .adapter
        .send_zcl_frame_inter_pan_to_ieee_addr(&frame, "0x0017880104c9cd33")
        .await
        .expect("interpan send");
    let requests = harness.mock.requests_named("dataRequestExt");
    assert_eq!(requests.len(), 1);
    let payload = requests[0].payload();
    assert_eq!(payload["clusterid"], 4096);
    assert_eq!(payload["destendpoint"], 254);
    assert_eq!(payload["dstaddr"], "0x0017880104c9cd33");
    assert_eq!(payload["dstaddrmode"], 3);
    assert_eq!(payload["dstpanid"], 65535);
    assert_eq!(payload["srcendpoint"], 12);
    assert_eq!(payload["len"], 9);
}

#[tokio::test]
async fn send_zcl_frame_interpan_with_response() {
    use zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem;
    use zigbee_apiarist::zspec::zcl::zcl_header::Direction;
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    let response = touchlink_frame(
        "scanResponse",
        Direction::SERVER_TO_CLIENT as u8,
        serde_json::json!({
            "transactionID": 1,
            "rssiCorrection": 10,
            "zigbeeInformation": 5,
            "touchlinkInformation": 6,
            "keyBitmask": 12,
            "responseID": 11,
            "extendedPanID": "0x0017210104d9cd33",
            "networkUpdateID": 1,
            "logicalChannel": 12,
            "panID": 13,
            "networkAddress": 5,
            "numberOfSubDevices": 10,
            "totalGroupIdentifiers": 5,
        }),
    );
    let request = touchlink_frame(
        "scanRequest",
        Direction::CLIENT_TO_SERVER as u8,
        serde_json::json!({"transactionID": 1, "zigbeeInformation": 4, "touchlinkInformation": 18}),
    );
    let result = {
        let future = harness
            .adapter
            .send_zcl_frame_inter_pan_broadcast(&request, 1000, false);
        tokio::pin!(future);
        harness.mock.inject_areq(
            Subsystem::AF,
            "incomingMsgExt",
            serde_json::json!({
                "clusterid": 4096,
                "srcendpoint": 254,
                "srcaddr": "0x000000000000306a",
                "linkquality": 101,
                "groupid": 0,
                "len": response.to_buffer().len(),
                "data": response.to_buffer(),
            }),
        );
        future.await.expect("interpan broadcast")
    };
    let payload = result.expect("payload");
    assert_eq!(payload.cluster_id, 4096);
    assert_eq!(payload.address, serde_json::json!(12394));
    assert_eq!(payload.endpoint, 254);
    assert_eq!(payload.linkquality, 101);
    assert_eq!(payload.group_id, 0);
    assert_eq!(
        payload.data,
        vec![
            9, 12, 1, 1, 0, 0, 0, 10, 5, 6, 12, 0, 11, 0, 0, 0, 51, 205, 217, 4, 1, 33, 23, 0, 1,
            12, 13, 0, 5, 0, 10, 5
        ]
    );
    let requests = harness.mock.requests_named("dataRequestExt");
    assert_eq!(requests.len(), 1);
    let request_payload = requests[0].payload();
    assert_eq!(request_payload["dstaddr"], "0x000000000000ffff");
    assert_eq!(request_payload["dstaddrmode"], 2);
    assert_eq!(request_payload["dstpanid"], 65535);
    assert_eq!(request_payload["srcendpoint"], 12);
}

#[tokio::test]
async fn send_zcl_frame_interpan_throws_when_command_has_no_response() {
    use zigbee_apiarist::zspec::zcl::zcl_header::Direction;
    let mut harness = started_3x0().await;
    let frame = touchlink_frame(
        "identifyRequest",
        Direction::CLIENT_TO_SERVER as u8,
        serde_json::json!({"transactionID": 1, "duration": 65535}),
    );
    let error = harness
        .adapter
        .send_zcl_frame_inter_pan_broadcast(&frame, 1000, false)
        .await
        .expect_err("no response");
    assert_eq!(
        error,
        "Command 'identifyRequest' has no response, cannot wait for response"
    );
}

#[tokio::test]
async fn send_zcl_frame_interpan_throws_when_data_request_fails() {
    use zigbee_apiarist::zspec::zcl::zcl_header::Direction;
    let mut harness = started_3x0().await;
    harness.mock.set_data_request_ext_code(99);
    let frame = touchlink_frame(
        "scanRequest",
        Direction::CLIENT_TO_SERVER as u8,
        serde_json::json!({"transactionID": 1, "zigbeeInformation": 4, "touchlinkInformation": 18}),
    );
    let error = harness
        .adapter
        .send_zcl_frame_inter_pan_broadcast(&frame, 1000, false)
        .await
        .expect_err("data request failure");
    assert_eq!(error, "Data request failed with code '99'");
}

#[tokio::test]
async fn command_should_fail_when_in_interpan() {
    let mut harness = started_3x0().await;
    let frame = write_no_rsp_frame();
    harness
        .adapter
        .set_channel_inter_pan(14)
        .await
        .expect("set channel");
    harness.mock.clear_requests();
    let error = harness
        .adapter
        .send_zcl_frame_to_endpoint("0x02", 2, 20, &frame, 10000, false, false, None, None)
        .await
        .expect_err("interpan lock");
    assert_eq!(error, "Cannot execute command, in Inter-PAN mode");
    assert!(data_requests(&harness).is_empty());

    harness
        .adapter
        .restore_channel_inter_pan()
        .await
        .expect("restore channel");
    harness.mock.clear_requests();
    harness
        .adapter
        .send_zcl_frame_to_endpoint("0x02", 2, 20, &frame, 10000, false, false, None, None)
        .await
        .expect("send");
    assert_eq!(data_requests(&harness).len(), 1);
}

#[tokio::test]
async fn wait_for_resolves_on_cmd() {
    use zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem;
    use zigbee_apiarist::zspec::zcl::zcl_header::{Direction, FrameType};
    let mut harness = started_3x0().await;
    let response = read_rsp_frame(100, 2);
    let (_, receiver) = harness
        .adapter
        .wait_for(
            2,
            20,
            FrameType::GLOBAL,
            Direction::CLIENT_TO_SERVER,
            Some(100),
            0,
            1,
            Some(1),
            1000,
        )
        .await;
    harness.mock.inject_areq(
        Subsystem::AF,
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
    let payload = receiver.await.expect("waiter").expect("payload");
    assert_eq!(payload.endpoint, 20);
    assert_eq!(payload.group_id, 12);
    assert_eq!(payload.linkquality, 101);
    assert_eq!(payload.address, serde_json::json!(2));
    assert_eq!(payload.data, vec![24, 100, 1, 0, 0, 0, 32, 2]);
}

#[tokio::test]
async fn wait_for_resolves_on_specified_default_response() {
    use zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem;
    use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
    use zigbee_apiarist::zspec::zcl::zcl_header::{Direction, FrameType};
    let mut harness = started_3x0().await;
    let _ = Frame::create(
        0,
        0,
        true,
        None,
        0,
        "read",
        0,
        serde_json::json!([]),
        serde_json::json!({}),
        0,
    );
    let response = Frame::create(
        FrameType::GLOBAL as u8,
        Direction::SERVER_TO_CLIENT as u8,
        true,
        None,
        99,
        "defaultRsp",
        "genOta",
        serde_json::json!({"cmdId": 5, "statusCode": 0x80}),
        serde_json::json!({}),
        0,
    );
    let (_, receiver) = harness
        .adapter
        .wait_for(
            1234,
            1,
            FrameType::SPECIFIC,
            Direction::CLIENT_TO_SERVER,
            None,
            0x0019,
            3,
            Some(5),
            1000,
        )
        .await;
    harness.mock.inject_areq(
        Subsystem::AF,
        "incomingMsg",
        serde_json::json!({
            "clusterid": 0x0019,
            "srcendpoint": 1,
            "srcaddr": 1234,
            "linkquality": 101,
            "groupid": 0,
            "len": response.to_buffer().len(),
            "data": response.to_buffer(),
        }),
    );
    let payload = receiver.await.expect("waiter").expect("payload");
    assert_eq!(payload.endpoint, 1);
    assert_eq!(payload.group_id, 0);
    assert_eq!(payload.linkquality, 101);
    assert_eq!(payload.address, serde_json::json!(1234));
    assert_eq!(payload.data, vec![24, 99, 11, 5, 0x80]);
}

#[tokio::test]
async fn supports_backup() {
    let mut harness = started_3x0().await;
    assert!(harness.adapter.supports_backup().await);
}

#[tokio::test]
async fn incoming_message_extended_huge_data_16_bit_address() {
    incoming_huge_data_case(2, "0x1234567812345678", serde_json::json!(0x5678)).await;
}

#[tokio::test]
async fn incoming_message_extended_huge_data_64_bit_address() {
    incoming_huge_data_case(
        3,
        "0x1234567812345678",
        serde_json::json!("0x1234567812345678"),
    )
    .await;
}

async fn incoming_huge_data_case(
    src_addr_mode: u64,
    addr: &str,
    expected_address: serde_json::Value,
) {
    use std::sync::Arc;
    use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::AdapterEvent;
    use zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem;

    let mut harness = started_3x0().await;
    let long_string = "1".repeat(500);
    let mut frame = vec![0x18, 100, 0x01, 0x34, 0x12, 0x00, 0x44];
    frame.extend_from_slice(&(long_string.len() as u16).to_le_bytes());
    frame.extend_from_slice(long_string.as_bytes());

    let payload = Arc::new(std::sync::Mutex::new(frame.clone()));
    let shared = payload.clone();
    harness
        .mock
        .handle(Subsystem::AF, "dataRetrieve", move |request, _| {
            let index = request["index"].as_u64().unwrap_or(0) as usize;
            let length = request["length"].as_u64().unwrap_or(0) as usize;
            let data = shared.lock().unwrap();
            if length == 0 {
                return support::Reply::payload(
                    serde_json::json!({"status": 0, "length": 0, "data": []}),
                );
            }
            let end = (index + length).min(data.len());
            let chunk = data[index.min(data.len())..end].to_vec();
            support::Reply::payload(serde_json::json!({
                "status": 0,
                "length": chunk.len(),
                "data": chunk,
            }))
        });
    let mut events = harness.adapter.events.subscribe();
    harness.mock.inject_areq(
        Subsystem::AF,
        "incomingMsgExt",
        serde_json::json!({
            "groupid": 12,
            "clusterid": 0,
            "srcaddrmode": src_addr_mode,
            "srcaddr": addr,
            "srcendpoint": 20,
            "dstendpoint": 1,
            "linkquality": 101,
            "timestamp": 0x12345678,
            "len": frame.len(),
            "data": [],
        }),
    );
    let received = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if let AdapterEvent::ZclPayload(payload) = events.recv().await.expect("event") {
                break payload;
            }
        }
    })
    .await
    .expect("huge payload timeout");
    assert_eq!(received.endpoint, 20);
    assert_eq!(received.group_id, 12);
    assert_eq!(received.linkquality, 101);
    assert_eq!(received.address, expected_address);
    assert_eq!(received.data, frame);
    assert_eq!(received.header.expect("header").command_identifier, 1);
}

#[tokio::test]
async fn incoming_message_extended_huge_data_retrieve_error_status() {
    incoming_huge_data_error_case(
        1,
        "dataRetrieve [timestamp: 286331153, index: 0, chunkSize: 240] error status: 1",
    )
    .await;
}

#[tokio::test]
async fn incoming_message_extended_huge_data_retrieve_length_mismatch() {
    incoming_huge_data_error_case(
        2,
        "dataRetrieve length mismatch [240 requested, 241 returned",
    )
    .await;
}

async fn incoming_huge_data_error_case(mock_error: u64, expected: &str) {
    use std::sync::Arc;
    use zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem;

    let mut harness = started_3x0().await;
    let payload = Arc::new(std::sync::Mutex::new(vec![0u8; 509]));
    let shared = payload.clone();
    harness
        .mock
        .handle(Subsystem::AF, "dataRetrieve", move |request, _| {
            let length = request["length"].as_u64().unwrap_or(0) as usize;
            let data = shared.lock().unwrap();
            support::Reply::payload(serde_json::json!({
                "status": if mock_error == 1 { 1 } else { 0 },
                "length": if mock_error == 2 { length + 1 } else { length },
                "data": data[..length.min(data.len())].to_vec(),
            }))
        });
    harness.mock.inject_areq(
        Subsystem::AF,
        "incomingMsgExt",
        serde_json::json!({
            "groupid": 12,
            "clusterid": 0,
            "srcaddrmode": 2,
            "srcaddr": "0x1234567812345678",
            "srcendpoint": 20,
            "dstendpoint": 1,
            "linkquality": 101,
            "timestamp": 0x11111111,
            "len": 509,
            "data": [],
        }),
    );
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let messages = support::log_messages();
    assert!(
        messages.iter().any(|message| message == expected),
        "logs: {messages:#?}"
    );
}

#[tokio::test]
async fn refuse_to_start_when_ping_fails() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_3x0_aligned(&harness.mock);
    harness.mock.handle(
        zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::SYS,
        "ping",
        |_, _| support::Reply::timeout(),
    );
    let error = harness.start().await.expect_err("ping failure");
    assert_eq!(
        error,
        "Failed to connect to the adapter (SRSP - SYS - ping after 6000ms)"
    );
}

#[tokio::test]
async fn sends_proper_zdo_request_payload_for_permit_joining_request_to_target() {
    use zigbee_apiarist::zspec::consts::BLANK_EUI64;
    use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
    use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    let payload = BuffaloZdo::build_request(
        false,
        ClusterId::PERMIT_JOINING_REQUEST,
        &[
            serde_json::json!(250),
            serde_json::json!(1),
            serde_json::json!([]),
        ],
    );
    harness
        .adapter
        .send_zdo(
            BLANK_EUI64,
            1234,
            ClusterId::PERMIT_JOINING_REQUEST,
            &payload,
            true,
        )
        .await
        .expect("permit target");
    let request = &harness.mock.requests_named("mgmtPermitJoinReq")[0];
    let mut expected = vec![2, 0xd2, 0x04];
    expected.extend_from_slice(&payload);
    assert_eq!(
        &request.frame[4..request.frame.len() - 1],
        expected.as_slice()
    );
}

#[tokio::test]
async fn sends_proper_zdo_request_payload_for_permit_joining_request_broadcast() {
    use zigbee_apiarist::zspec::consts::BLANK_EUI64;
    use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
    use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    let payload = BuffaloZdo::build_request(
        false,
        ClusterId::PERMIT_JOINING_REQUEST,
        &[
            serde_json::json!(250),
            serde_json::json!(1),
            serde_json::json!([]),
        ],
    );
    harness
        .adapter
        .send_zdo(
            BLANK_EUI64,
            0xfffc,
            ClusterId::PERMIT_JOINING_REQUEST,
            &payload,
            true,
        )
        .await
        .expect("permit broadcast");
    let request = &harness.mock.requests_named("mgmtPermitJoinReq")[0];
    let mut expected = vec![0x0f, 0xfc, 0xff];
    expected.extend_from_slice(&payload);
    assert_eq!(
        &request.frame[4..request.frame.len() - 1],
        expected.as_slice()
    );
}

#[tokio::test]
async fn sends_proper_zdo_request_payload_for_nwk_update_request_unicast() {
    use zigbee_apiarist::zspec::consts::BLANK_EUI64;
    use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
    use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    let payload = BuffaloZdo::build_request(
        false,
        ClusterId::NWK_UPDATE_REQUEST,
        &[
            serde_json::json!([15]),
            serde_json::json!(0xfe),
            serde_json::json!(0),
            serde_json::Value::Null,
            serde_json::json!(0),
        ],
    );
    harness
        .adapter
        .send_zdo(
            BLANK_EUI64,
            0x123,
            ClusterId::NWK_UPDATE_REQUEST,
            &payload,
            true,
        )
        .await
        .expect("nwk update unicast");
    let request = &harness.mock.requests_named("mgmtNwkUpdateReq")[0];
    let mut expected = vec![0x23, 0x01, 2];
    expected.extend_from_slice(&payload);
    expected.extend_from_slice(&[0, 0, 0]);
    assert_eq!(
        &request.frame[4..request.frame.len() - 1],
        expected.as_slice()
    );
}

#[tokio::test]
async fn sends_proper_zdo_request_payload_for_nwk_update_request_broadcast() {
    use zigbee_apiarist::zspec::consts::BLANK_EUI64;
    use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
    use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    let payload = BuffaloZdo::build_request(
        false,
        ClusterId::NWK_UPDATE_REQUEST,
        &[
            serde_json::json!([15]),
            serde_json::json!(0xfe),
            serde_json::json!(0),
            serde_json::Value::Null,
            serde_json::json!(0),
        ],
    );
    harness
        .adapter
        .send_zdo(
            BLANK_EUI64,
            0xffff,
            ClusterId::NWK_UPDATE_REQUEST,
            &payload,
            true,
        )
        .await
        .expect("nwk update broadcast");
    let request = &harness.mock.requests_named("mgmtNwkUpdateReq")[0];
    let mut expected = vec![0xff, 0xff, 0x0f];
    expected.extend_from_slice(&payload);
    expected.extend_from_slice(&[0, 0, 0]);
    assert_eq!(
        &request.frame[4..request.frame.len() - 1],
        expected.as_slice()
    );
}

#[tokio::test]
async fn sends_proper_zdo_request_payload_for_bind_request_unbind_request_unicast() {
    use zigbee_apiarist::zspec::consts::BLANK_EUI64;
    use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
    use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    let payload = BuffaloZdo::build_request(
        false,
        ClusterId::BIND_REQUEST,
        &[
            serde_json::json!("0x1122334455667788"),
            serde_json::json!(3),
            serde_json::json!(0x0103),
            serde_json::json!(3),
            serde_json::json!("0x5544332211667788"),
            serde_json::Value::Null,
            serde_json::json!(5),
        ],
    );
    harness
        .adapter
        .send_zdo(BLANK_EUI64, 1234, ClusterId::BIND_REQUEST, &payload, true)
        .await
        .expect("bind unicast");
    let request = &harness.mock.requests_named("bindReq")[0];
    let mut expected = vec![0xd2, 0x04];
    expected.extend_from_slice(&payload);
    assert_eq!(
        &request.frame[4..request.frame.len() - 1],
        expected.as_slice()
    );
}

#[tokio::test]
async fn sends_proper_zdo_request_payload_for_bind_request_unbind_request_multicast() {
    use zigbee_apiarist::zspec::consts::BLANK_EUI64;
    use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
    use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    let payload = BuffaloZdo::build_request(
        false,
        ClusterId::BIND_REQUEST,
        &[
            serde_json::json!("0x1122334455667788"),
            serde_json::json!(3),
            serde_json::json!(0x0103),
            serde_json::json!(1),
            serde_json::Value::Null,
            serde_json::json!(32),
            serde_json::json!(0),
        ],
    );
    harness
        .adapter
        .send_zdo(BLANK_EUI64, 1234, ClusterId::BIND_REQUEST, &payload, true)
        .await
        .expect("bind multicast");
    let request = &harness.mock.requests_named("bindReq")[0];
    let mut expected = vec![0xd2, 0x04];
    expected.extend_from_slice(&payload);
    expected.resize(2 + 21, 0);
    assert_eq!(
        &request.frame[4..request.frame.len() - 1],
        expected.as_slice()
    );
}

#[tokio::test]
async fn sends_proper_zdo_request_payload_for_network_address_request() {
    use zigbee_apiarist::zspec::consts::BLANK_EUI64;
    use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
    use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    let payload = BuffaloZdo::build_request(
        false,
        ClusterId::NETWORK_ADDRESS_REQUEST,
        &[
            serde_json::json!("0x1122334455667788"),
            serde_json::json!(false),
            serde_json::json!(0),
        ],
    );
    harness
        .adapter
        .send_zdo(
            BLANK_EUI64,
            1234,
            ClusterId::NETWORK_ADDRESS_REQUEST,
            &payload,
            true,
        )
        .await
        .expect("nwk address request");
    let request = &harness.mock.requests_named("nwkAddrReq")[0];
    assert_eq!(
        &request.frame[4..request.frame.len() - 1],
        payload.as_slice()
    );
}

#[tokio::test]
async fn sends_proper_zdo_request_payload_for_generic_logic_request() {
    use zigbee_apiarist::zspec::consts::BLANK_EUI64;
    use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
    use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;
    let mut harness = started_3x0().await;
    harness.mock.clear_requests();
    let payload = BuffaloZdo::build_request(
        false,
        ClusterId::NODE_DESCRIPTOR_REQUEST,
        &[serde_json::json!(1234)],
    );
    harness
        .adapter
        .send_zdo(
            BLANK_EUI64,
            1234,
            ClusterId::NODE_DESCRIPTOR_REQUEST,
            &payload,
            true,
        )
        .await
        .expect("node descriptor request");
    let request = &harness.mock.requests_named("nodeDescReq")[0];
    let mut expected = vec![0xd2, 0x04];
    expected.extend_from_slice(&payload);
    assert_eq!(
        &request.frame[4..request.frame.len() - 1],
        expected.as_slice()
    );
}

#[tokio::test]
async fn node_descriptor_request_should_discover_route_to_fix_potential_fails() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use zigbee_apiarist::zspec::consts::BLANK_EUI64;
    use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
    use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;

    let mut harness = started_3x0().await;
    let calls = std::sync::Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    harness.mock.handle(
        zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::ZDO,
        "nodeDescReq",
        move |_payload, state| {
            let call = counter.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                support::Reply::timeout()
            } else {
                let target = u16::from_le_bytes([
                    state.current_data[0],
                    state.current_data[1],
                ]);
                support::Reply::empty().with_zdo(
                    zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::NODE_DESCRIPTOR_RESPONSE,
                    Some(target),
                    &[
                        serde_json::json!(0),
                        serde_json::json!({
                            "nwkAddress": target,
                            "logicalType": 2,
                            "apsFlags": 0,
                            "frequencyBand": 0,
                            "capabilities": dummy_capabilities(),
                            "manufacturerCode": 0,
                            "maxBufSize": 0,
                            "maxIncTxSize": 0,
                            "serverMask": {
                                "primaryTrustCenter": 0, "backupTrustCenter": 0, "deprecated1": 0,
                                "deprecated2": 0, "deprecated3": 0, "deprecated4": 0,
                                "networkManager": 0, "reserved1": 0, "reserved2": 0,
                                "stackComplianceRevision": 0,
                            },
                            "maxOutTxSize": 0,
                            "deprecated1": 0,
                            "tlvs": [],
                        }),
                    ],
                )
            }
        },
    );
    harness.mock.clear_requests();
    let payload = BuffaloZdo::build_request(
        false,
        ClusterId::NODE_DESCRIPTOR_REQUEST,
        &[serde_json::json!(1234)],
    );
    harness
        .adapter
        .send_zdo(
            BLANK_EUI64,
            1234,
            ClusterId::NODE_DESCRIPTOR_REQUEST,
            &payload,
            true,
        )
        .await
        .expect("node descriptor retry");
    assert_eq!(harness.mock.requests_named("nodeDescReq").len(), 2);
    assert_eq!(harness.mock.requests_named("extRouteDisc").len(), 1);
}

#[tokio::test]
async fn should_throw_error_when_zdo_call_fails() {
    use zigbee_apiarist::zspec::consts::BLANK_EUI64;
    use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
    use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;
    let mut harness = started_3x0().await;
    harness.mock.handle(
        zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::ZDO,
        "simpleDescReq",
        |_, _| {
            let mut reply = support::Reply::payload(serde_json::json!({"status": 0x83}));
            reply.respond = true;
            reply
        },
    );
    let payload = BuffaloZdo::build_request(
        false,
        ClusterId::SIMPLE_DESCRIPTOR_REQUEST,
        &[serde_json::json!(123), serde_json::json!(0)],
    );
    let error = harness
        .adapter
        .send_zdo(
            BLANK_EUI64,
            1234,
            ClusterId::SIMPLE_DESCRIPTOR_REQUEST,
            &payload,
            true,
        )
        .await
        .expect_err("zdo failure");
    // The upstream mock rejects with a literal "Failed"; through the wire the
    // equivalent failure is the ZNP SRSP status error, asserted in full.
    assert_eq!(
        error,
        "--> 'SREQ: ZDO - SIMPLE_DESCRIPTOR_REQUEST - d2047b0000' failed with status '(0x83: UNKNOWN)'"
    );
}

#[tokio::test]
async fn should_throw_error_when_register_endpoints_fails() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::commissioned_3x0_aligned(&harness.mock);
    harness.mock.handle(
        zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::ZDO,
        "activeEpReq",
        |_, _| {
            support::Reply::empty().with_zdo(
                zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId::ACTIVE_ENDPOINTS_RESPONSE,
                Some(0),
                &[serde_json::json!(131), serde_json::Value::Null],
            )
        },
    );
    let error = harness.start().await.expect_err("endpoint registration");
    assert_eq!(error, "Status 'NOT_ACTIVE'");
}

#[tokio::test]
async fn should_commission_with_3_0_x_adapter_empty_mismatched_config_and_backup() {
    let path = temp_backup("empty_mismatch", &backup_fixture("notMatching"));
    let mut harness = harness(default_network_options(), &path);
    fixtures::empty_3_aligned(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Reset);
}

#[tokio::test]
async fn should_commission_with_3_0_x_adapter_commissioned_mismatched_adapter_config_backup() {
    let path = temp_backup("commissioned_mismatch", &backup_fixture("notMatching"));
    let mut harness = harness(default_network_options(), &path);
    fixtures::commissioned_3_aligned_config_mismatch(&harness.mock);
    assert_eq!(harness.start().await.expect("start"), StartResult::Reset);
}

#[tokio::test]
async fn should_fail_to_commission_when_bdb_commissioning_times_out() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::empty_3_aligned(&harness.mock);
    harness.mock.handle(
        zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::APP_CNF,
        "bdbStartCommissioning",
        |_, _| support::Reply::empty(),
    );
    let error = harness.start().await.expect_err("commissioning timeout");
    assert_eq!(
        error,
        "network commissioning timed out - most likely network with the same panId or extendedPanId already exists nearby"
    );
}

#[tokio::test]
async fn should_fail_to_commission_when_nib_fails_to_settle() {
    let mut harness = harness(default_network_options(), "backup.json");
    fixtures::empty_3_aligned(&harness.mock);
    harness.mock.handle(
        zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::APP_CNF,
        "bdbStartCommissioning",
        |_, _| {
            support::Reply::empty().with_extra(
                zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem::ZDO,
                192,
                vec![9],
            )
        },
    );
    let error = harness.start().await.expect_err("nib settle timeout");
    assert_eq!(
        error,
        "network commissioning failed - timed out waiting for nib to settle"
    );
}

#[test]
fn call_znp_constructor() {
    let transport = zigbee_apiarist::adapter::transport::AdapterTransport::new(
        zigbee_apiarist::adapter::transport::TransportOptions::new("dummy"),
    );
    let _adapter = zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::ZStackAdapter::new(
        transport,
        default_network_options(),
        "backup.json".to_string(),
        AdapterOptions::default(),
    );
}

#[tokio::test]
async fn send_zcl_frame_network_address_timeout_should_discover_route_and_retry() {
    use zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem;
    let mut harness = started_3x0().await;
    harness.mock.set_assoc_node_relation(2);
    harness.mock.clear_requests();
    let mismatch = read_rsp_frame(102, 5);
    let frame = read_frame(false);
    let result = {
        let request = harness.adapter.send_zcl_frame_to_endpoint(
            "0x0000000000000002",
            2,
            20,
            &frame,
            1,
            false,
            false,
            None,
            None,
        );
        tokio::pin!(request);
        harness.mock.inject_areq(
            Subsystem::AF,
            "incomingMsg",
            serde_json::json!({
                "clusterid": 0,
                "srcendpoint": 20,
                "srcaddr": 2,
                "linkquality": 101,
                "groupid": 12,
                "len": mismatch.to_buffer().len(),
                "data": mismatch.to_buffer(),
            }),
        );
        request.await
    };
    let error = result.expect_err("response timeout");
    assert_eq!(
        error,
        "Timeout after 1ms [address=2 endpoint=20 clusterId=0 cmdId=1 tsn=100]"
    );
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
    assert!(data_requests(&harness).len() >= 2, "commands: {commands:?}");
}

#[tokio::test]
async fn send_zcl_frame_network_address_timeout_should_rewrite_child_entry_and_retry() {
    use zigbee_apiarist::adapter::z_stack::unpi::constants::Subsystem;
    let mut harness = started_3x0().await;
    harness.mock.set_assoc_node_relation(1);
    harness.mock.clear_requests();
    let mismatch = read_rsp_frame(102, 5);
    let frame = read_frame(false);
    let result = {
        let request = harness.adapter.send_zcl_frame_to_endpoint(
            "0x0000000000000002",
            2,
            20,
            &frame,
            1,
            false,
            false,
            None,
            None,
        );
        tokio::pin!(request);
        harness.mock.inject_areq(
            Subsystem::AF,
            "incomingMsg",
            serde_json::json!({
                "clusterid": 0,
                "srcendpoint": 20,
                "srcaddr": 2,
                "linkquality": 101,
                "groupid": 12,
                "len": mismatch.to_buffer().len(),
                "data": mismatch.to_buffer(),
            }),
        );
        request.await
    };
    assert!(result.is_err());
    let commands = harness
        .mock
        .requests()
        .into_iter()
        .map(|request| request.command)
        .collect::<Vec<_>>();
    assert!(
        commands.contains(&"assocGetWithAddress".to_string()),
        "commands: {commands:?}"
    );
    assert!(
        commands.contains(&"assocRemove".to_string()),
        "commands: {commands:?}"
    );
    assert!(
        commands.contains(&"assocAdd".to_string()),
        "commands: {commands:?}"
    );
    assert!(
        commands.contains(&"extRouteDisc".to_string()),
        "commands: {commands:?}"
    );
}
