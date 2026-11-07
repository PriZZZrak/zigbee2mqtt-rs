// Original: .upstream/zigbee-herdsman/test/adapter/zigate/zdo.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - ZiGate ZDO payloads
//   - ZiGateCommandCode.ActiveEndpoint
//   - ZiGateCommandCode.Bind - MULTICAST
//   - ZiGateCommandCode.Bind - UNICAST
//   - ZiGateCommandCode.LeaveRequest
//   - ZiGateCommandCode.ManagementLQI
//   - ZiGateCommandCode.NodeDescriptor
//   - ZiGateCommandCode.PermitJoin
//   - ZiGateCommandCode.SimpleDescriptor
//   - ZiGateCommandCode.UnBind - MULTICAST
//   - ZiGateCommandCode.UnBind - UNICAST

use serde_json::json;
use zigbee_apiarist::adapter::zigate::adapter::zigate_adapter::ZiGateAdapter;
use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;

const BLANK_EUI64: &str = "0x0000000000000000";
const UNICAST_BINDING: u8 = 0x03;
const MULTICAST_BINDING: u8 = 0x01;
const LEAVE_WITHOUT_REJOIN: u8 = 0x00;

fn create_adapter() -> ZiGateAdapter {
    ZiGateAdapter::new(
        json!({"panID": 0, "channelList": [11]}),
        json!({}),
        "tmp.db.backup",
        json!({"disableLED": false}),
    )
}

fn hex_decode(value: &str) -> Vec<u8> {
    (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
        .collect()
}

#[tokio::test]
async fn zi_gate_command_code_active_endpoint() {
    let adapter = create_adapter();
    let cluster_id = ClusterId::ACTIVE_ENDPOINTS_REQUEST;
    let zdo_payload = BuffaloZdo::build_request_be(false, cluster_id, &[json!(0x1122)]);
    adapter
        .send_zdo("0x1122334455667788", 0x1122, cluster_id, zdo_payload, true)
        .await;
    assert_eq!(
        adapter.driver_calls.borrow().as_slice(),
        &[(cluster_id, hex_decode("1122"))]
    );
}

#[tokio::test]
async fn zi_gate_command_code_bind_multicast() {
    let adapter = create_adapter();
    let cluster_id = ClusterId::BIND_REQUEST;
    let zdo_payload = BuffaloZdo::build_request_be(
        false,
        cluster_id,
        &[
            json!("0x1122334455667788"),
            json!(5),
            json!(0x4567),
            json!(MULTICAST_BINDING),
            json!(BLANK_EUI64),
            json!(0x3456),
            json!(0),
        ],
    );
    adapter
        .send_zdo("0x1122334455667788", 0x1122, cluster_id, zdo_payload, true)
        .await;
    assert_eq!(
        adapter.driver_calls.borrow().as_slice(),
        &[(cluster_id, hex_decode("112233445566778805456701345600"))]
    );
}

#[tokio::test]
async fn zi_gate_command_code_bind_unicast() {
    let adapter = create_adapter();
    let cluster_id = ClusterId::BIND_REQUEST;
    let zdo_payload = BuffaloZdo::build_request_be(
        false,
        cluster_id,
        &[
            json!("0x1122334455667788"),
            json!(5),
            json!(0x4567),
            json!(UNICAST_BINDING),
            json!("0x9911882277336644"),
            json!(0),
            json!(3),
        ],
    );
    adapter
        .send_zdo("0x1122334455667788", 0x1122, cluster_id, zdo_payload, true)
        .await;
    assert_eq!(
        adapter.driver_calls.borrow().as_slice(),
        &[(
            cluster_id,
            hex_decode("112233445566778805456703991188227733664403")
        )]
    );
}

#[tokio::test]
async fn zi_gate_command_code_leave_request() {
    let adapter = create_adapter();
    let cluster_id = ClusterId::LEAVE_REQUEST;
    let zdo_payload = BuffaloZdo::build_request_be(
        false,
        cluster_id,
        &[json!("0x1122334455667788"), json!(LEAVE_WITHOUT_REJOIN)],
    );
    adapter
        .send_zdo("0x1122334455667788", 0x1122, cluster_id, zdo_payload, true)
        .await;
    assert_eq!(
        adapter.driver_calls.borrow().as_slice(),
        &[(cluster_id, hex_decode("11223344556677880000"))]
    );
}

#[tokio::test]
async fn zi_gate_command_code_management_lqi() {
    let adapter = create_adapter();
    let cluster_id = ClusterId::LQI_TABLE_REQUEST;
    let zdo_payload = BuffaloZdo::build_request_be(false, cluster_id, &[json!(0)]);
    adapter
        .send_zdo("0x1122334455667788", 0x1122, cluster_id, zdo_payload, true)
        .await;
    assert_eq!(
        adapter.driver_calls.borrow().as_slice(),
        &[(cluster_id, hex_decode("112200"))]
    );
}

#[tokio::test]
async fn zi_gate_command_code_node_descriptor() {
    let adapter = create_adapter();
    let cluster_id = ClusterId::NODE_DESCRIPTOR_REQUEST;
    let zdo_payload = BuffaloZdo::build_request_be(false, cluster_id, &[json!(0x1122)]);
    adapter
        .send_zdo("0x1122334455667788", 0x1122, cluster_id, zdo_payload, true)
        .await;
    assert_eq!(
        adapter.driver_calls.borrow().as_slice(),
        &[(cluster_id, hex_decode("1122"))]
    );
}

#[tokio::test]
async fn zi_gate_command_code_permit_join() {
    let adapter = create_adapter();
    let cluster_id = ClusterId::PERMIT_JOINING_REQUEST;
    let zdo_payload =
        BuffaloZdo::build_request_be(false, cluster_id, &[json!(254), json!(1), json!([])]);
    adapter
        .send_zdo("0x1122334455667788", 0x1122, cluster_id, zdo_payload, true)
        .await;
    assert_eq!(
        adapter.driver_calls.borrow().as_slice(),
        &[(cluster_id, hex_decode("1122fe01"))]
    );
}

#[tokio::test]
async fn zi_gate_command_code_simple_descriptor() {
    let adapter = create_adapter();
    let cluster_id = ClusterId::SIMPLE_DESCRIPTOR_REQUEST;
    let zdo_payload = BuffaloZdo::build_request_be(false, cluster_id, &[json!(0x1122), json!(3)]);
    adapter
        .send_zdo("0x1122334455667788", 0x1122, cluster_id, zdo_payload, true)
        .await;
    assert_eq!(
        adapter.driver_calls.borrow().as_slice(),
        &[(cluster_id, hex_decode("112203"))]
    );
}

#[tokio::test]
async fn zi_gate_command_code_un_bind_multicast() {
    let adapter = create_adapter();
    let cluster_id = ClusterId::UNBIND_REQUEST;
    let zdo_payload = BuffaloZdo::build_request_be(
        false,
        cluster_id,
        &[
            json!("0x1122334455667788"),
            json!(5),
            json!(0x4567),
            json!(MULTICAST_BINDING),
            json!(BLANK_EUI64),
            json!(0x3456),
            json!(0),
        ],
    );
    adapter
        .send_zdo("0x1122334455667788", 0x1122, cluster_id, zdo_payload, true)
        .await;
    assert_eq!(
        adapter.driver_calls.borrow().as_slice(),
        &[(cluster_id, hex_decode("112233445566778805456701345600"))]
    );
}

#[tokio::test]
async fn zi_gate_command_code_un_bind_unicast() {
    let adapter = create_adapter();
    let cluster_id = ClusterId::UNBIND_REQUEST;
    let zdo_payload = BuffaloZdo::build_request_be(
        false,
        cluster_id,
        &[
            json!("0x1122334455667788"),
            json!(5),
            json!(0x4567),
            json!(UNICAST_BINDING),
            json!("0x9911882277336644"),
            json!(0),
            json!(3),
        ],
    );
    adapter
        .send_zdo("0x1122334455667788", 0x1122, cluster_id, zdo_payload, true)
        .await;
    assert_eq!(
        adapter.driver_calls.borrow().as_slice(),
        &[(
            cluster_id,
            hex_decode("112233445566778805456703991188227733664403")
        )]
    );
}
