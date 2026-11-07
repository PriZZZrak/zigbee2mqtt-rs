// Original: .upstream/zigbee-herdsman/test/adapter/zigate/patchZdoBuffaloBE.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - ZiGate Patch BuffaloZdo to use BE variants when writing
//   - readIeeeAddr - LE
//   - readUInt16 + readUInt32 - LE
//   - writeIeeeAddr
//   - writeUInt16
//   - writeUInt32

use serde_json::json;
use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;

#[test]
#[ignore = "not implemented yet"]
fn read_ieee_addr_le() {
    let buffer = vec![
        0x01, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x12, 0x34,
    ];
    assert_eq!(
        BuffaloZdo::read_response_be(true, ClusterId::IEEE_ADDRESS_RESPONSE, &buffer),
        BuffaloZdo::read_response(true, ClusterId::IEEE_ADDRESS_RESPONSE, &buffer)
    );
}

#[test]
#[ignore = "not implemented yet"]
fn read_uint16_read_uint32_le() {
    let buffer = vec![
        0x01, 0x00, 0x00, 0x00, 0x80, 0x00, 0x12, 0x34, 0x00, 0x01, 0x01, 0x12,
    ];
    assert_eq!(
        BuffaloZdo::read_response_be(true, ClusterId::NWK_UPDATE_RESPONSE, &buffer),
        BuffaloZdo::read_response(true, ClusterId::NWK_UPDATE_RESPONSE, &buffer)
    );
}

#[test]
#[ignore = "not implemented yet"]
fn write_ieee_addr() {
    assert_eq!(
        BuffaloZdo::build_request_be(
            false,
            ClusterId::NETWORK_ADDRESS_REQUEST,
            &[json!("0x1122334455667788"), json!(false), json!(0)],
        ),
        vec![0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x00, 0x00]
    );

    // ensure regular parsing OK
    assert_eq!(
        BuffaloZdo::build_request(
            false,
            ClusterId::NETWORK_ADDRESS_REQUEST,
            &[json!("0x1122334455667788"), json!(false), json!(0)],
        ),
        vec![0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11, 0x00, 0x00]
    );
}

#[test]
#[ignore = "not implemented yet"]
fn write_uint16() {
    assert_eq!(
        BuffaloZdo::build_request_be(
            false,
            ClusterId::IEEE_ADDRESS_REQUEST,
            &[json!(0x1234), json!(false), json!(0)],
        ),
        vec![0x12, 0x34, 0x00, 0x00]
    );

    // ensure regular parsing OK
    assert_eq!(
        BuffaloZdo::build_request(
            false,
            ClusterId::IEEE_ADDRESS_REQUEST,
            &[json!(0x1234), json!(false), json!(0)],
        ),
        vec![0x34, 0x12, 0x00, 0x00]
    );
}

#[test]
#[ignore = "not implemented yet"]
fn write_uint32() {
    assert_eq!(
        BuffaloZdo::build_request_be(
            false,
            ClusterId::NWK_UPDATE_REQUEST,
            &[
                json!([15]),
                json!(0xfe),
                serde_json::Value::Null,
                serde_json::Value::Null,
                serde_json::Value::Null,
            ],
        ),
        vec![0x00, 0x00, 0x80, 0x00, 0xfe]
    );

    // ensure regular parsing OK
    assert_eq!(
        BuffaloZdo::build_request(
            false,
            ClusterId::NWK_UPDATE_REQUEST,
            &[
                json!([15]),
                json!(0xfe),
                serde_json::Value::Null,
                serde_json::Value::Null,
                serde_json::Value::Null,
            ],
        ),
        vec![0x00, 0x80, 0x00, 0x00, 0xfe]
    );
}
