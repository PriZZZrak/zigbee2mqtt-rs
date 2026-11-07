// Original: .upstream/zigbee-herdsman/test/adapter/z-stack/znp.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - BUFFER read
//   - BUFFER write
//   - BUFFER100 read
//   - BUFFER100 write
//   - BUFFER16 read
//   - BUFFER16 write
//   - BUFFER18 read
//   - BUFFER18 write
//   - BUFFER32 read
//   - BUFFER32 write
//   - BUFFER42 read
//   - BUFFER42 write
//   - BUFFER8 read
//   - BUFFER8 write
//   - BUFFER8 write length consistent
//   - Cant read unsupported type
//   - Close without initialization
//   - Coverage logger
//   - IEEEADDR read
//   - IEEEADDR write
//   - INT8 read
//   - INT8 write
//   - LIST_NETWORK read
//   - LIST_NETWORK write
//   - LIST_UINT16 read
//   - LIST_UINT16 write
//   - LIST_UINT8 read
//   - LIST_UINT8 write
//   - Open
//   - Open - first ping fails should send reset bootloader
//   - Open and close
//   - Open and close by serialport event
//   - Open and close error
//   - Open and close tcp port
//   - Open tcp port with socket error
//   - Open with defaults
//   - Open with error
//   - Open with error when serialport is not open
//   - Serialport error (do nothing)
//   - UINT16 read
//   - UINT16 write
//   - UINT32 read
//   - UINT32 write
//   - UINT8 read
//   - UINT8 write
//   - ZNP
//   - ZpiObject parse payload for endDeviceAnnceInd
//   - ZpiObject parse payload for nwkAddrRsp
//   - ZpiObject throw error on missing write parser
//   - ZpiObject throw error on unknown command
//   - ZpiObject throw error on unknown parameters
//   - ZpiObject with cmd and non sapi is not reset command
//   - znp receive
//   - znp receive malformed
//   - znp request AREQ
//   - znp request AREQ reset
//   - znp request SREQ
//   - znp request SREQ failed
//   - znp request SREQ failed should cancel waiter when provided
//   - znp request SREQ with parsed in between
//   - znp request ZDO
//   - znp request ZDO FAILURE
//   - znp request ZDO SUCCESS
//   - znp request ZDO failed should cancel waiter when provided
//   - znp request timeout
//   - znp request timeout for startupFromApp is longer
//   - znp request with non-existing cmd
//   - znp request with non-existing subsystem
//   - znp request without init
//   - znp request, responses comes after timeout
//   - znp request, waitFor
//   - znp requestWithReply should throw error when request as no reply
//   - znp waitFor with payload mismatch
//   - znp waitFor with state
//   - znp waitFor with target as IEEE
//   - znp waitFor with target as IEEE forced to timeout because invalid ZDO status (no payload to match against)
//   - znp waitFor with target as network address
//   - znp waitFor with transid

use serde_json::json;
use zigbee_apiarist::adapter::z_stack::znp::buffalo_znp::BuffaloZnp;
use zigbee_apiarist::adapter::z_stack::znp::parameter_type::ParameterType;

const IEEE_ADDR1: &str = "0xae440112004b1200";
const IEEE_ADDR1_HEX: [u8; 8] = [0x00, 0x12, 0x4b, 0x00, 0x12, 0x01, 0x44, 0xae];
const IEEE_ADDR2: &str = "0xaf440112005b1200";
const IEEE_ADDR2_HEX: [u8; 8] = [0x00, 0x12, 0x5b, 0x00, 0x12, 0x01, 0x44, 0xaf];

fn duplicate_array<T: Clone>(times: usize, item: &[T]) -> Vec<T> {
    let mut out = Vec::with_capacity(times * item.len());
    for _ in 0..times {
        out.extend_from_slice(item);
    }
    out
}

#[test]
#[ignore = "not implemented yet"]
fn buffer_read() {
    let buffer = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09];
    let mut buffalo = BuffaloZnp::new(buffer.clone(), 2);
    let value = buffalo.read(ParameterType::BUFFER as u32, &json!({"length": 1}));
    assert_eq!(buffalo.get_position(), 3);
    assert_eq!(value, json!([buffer[2]]));
}

#[test]
#[ignore = "not implemented yet"]
fn buffer_write() {
    let mut buffalo = BuffaloZnp::new(vec![0; 9], 1);
    let payload = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
    buffalo.write(ParameterType::BUFFER as u32, &json!(payload), &json!({}));
    assert_eq!(buffalo.get_position(), 9);
    assert_eq!(
        buffalo.get_buffer(),
        vec![0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]
    );
}

#[test]
#[ignore = "not implemented yet"]
fn buffer100_read() {
    let payload = duplicate_array(20, &[0x00, 0x01, 0x02, 0x03, 0x04]);
    let mut buffer = vec![0x00];
    buffer.extend_from_slice(&payload);
    let mut buffalo = BuffaloZnp::new(buffer, 1);
    let value = buffalo.read(ParameterType::BUFFER100 as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 101);
    assert_eq!(value, json!(payload));
}

#[test]
#[ignore = "not implemented yet"]
fn buffer100_write() {
    let mut buffalo = BuffaloZnp::new(vec![0; 100], 0);
    let payload = duplicate_array(20, &[0x00, 0x01, 0x02, 0x03, 0x04]);
    buffalo.write(ParameterType::BUFFER100 as u32, &json!(payload), &json!({}));
    assert_eq!(buffalo.get_position(), 100);
    assert_eq!(buffalo.get_buffer(), payload);
}

#[test]
#[ignore = "not implemented yet"]
fn buffer16_read() {
    let payload = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
    let mut buffer = vec![0x00];
    buffer.extend_from_slice(&payload);
    buffer.extend_from_slice(&payload);
    let mut buffalo = BuffaloZnp::new(buffer, 1);
    let value = buffalo.read(ParameterType::BUFFER16 as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 17);
    let mut expected = payload.clone();
    expected.extend_from_slice(&payload);
    assert_eq!(value, json!(expected));
}

#[test]
#[ignore = "not implemented yet"]
fn buffer16_write() {
    let mut buffalo = BuffaloZnp::new(vec![0; 20], 1);
    let payload = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
    let mut full = payload.clone();
    full.extend_from_slice(&payload);
    buffalo.write(ParameterType::BUFFER16 as u32, &json!(full), &json!({}));
    assert_eq!(buffalo.get_position(), 17);
    let mut expected = vec![0x00];
    expected.extend_from_slice(&full);
    expected.extend_from_slice(&[0x00, 0x00, 0x00]);
    assert_eq!(buffalo.get_buffer(), expected);
}

#[test]
#[ignore = "not implemented yet"]
fn buffer18_read() {
    let payload = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    let mut buffer = vec![0x00];
    buffer.extend_from_slice(&payload);
    buffer.extend_from_slice(&payload);
    let mut buffalo = BuffaloZnp::new(buffer, 1);
    let value = buffalo.read(ParameterType::BUFFER18 as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 19);
    let mut expected = payload.clone();
    expected.extend_from_slice(&payload);
    assert_eq!(value, json!(expected));
}

#[test]
#[ignore = "not implemented yet"]
fn buffer18_write() {
    let mut buffalo = BuffaloZnp::new(vec![0; 20], 1);
    let payload = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    let mut full = payload.clone();
    full.extend_from_slice(&payload);
    buffalo.write(ParameterType::BUFFER18 as u32, &json!(full), &json!({}));
    assert_eq!(buffalo.get_position(), 19);
    let mut expected = vec![0x00];
    expected.extend_from_slice(&full);
    expected.push(0x00);
    assert_eq!(buffalo.get_buffer(), expected);
}

#[test]
#[ignore = "not implemented yet"]
fn buffer32_read() {
    let payload = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
    let full = duplicate_array(4, &payload);
    let mut buffer = vec![0x00];
    buffer.extend_from_slice(&full);
    let mut buffalo = BuffaloZnp::new(buffer, 1);
    let value = buffalo.read(ParameterType::BUFFER32 as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 33);
    assert_eq!(value, json!(full));
}

#[test]
#[ignore = "not implemented yet"]
fn buffer32_write() {
    let mut buffalo = BuffaloZnp::new(vec![0; 34], 1);
    let payload = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
    let full = duplicate_array(4, &payload);
    buffalo.write(ParameterType::BUFFER32 as u32, &json!(full), &json!({}));
    assert_eq!(buffalo.get_position(), 33);
    let mut expected = vec![0x00];
    expected.extend_from_slice(&full);
    expected.push(0x00);
    assert_eq!(buffalo.get_buffer(), expected);
}

#[test]
#[ignore = "not implemented yet"]
fn buffer42_read() {
    let payload = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
    let mut full = duplicate_array(5, &payload);
    full.extend_from_slice(&[0x08, 0x09]);
    let mut buffer = vec![0x00];
    buffer.extend_from_slice(&full);
    let mut buffalo = BuffaloZnp::new(buffer, 1);
    let value = buffalo.read(ParameterType::BUFFER42 as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 43);
    assert_eq!(value, json!(full));
}

#[test]
#[ignore = "not implemented yet"]
fn buffer42_write() {
    let mut buffalo = BuffaloZnp::new(vec![0; 44], 1);
    let payload = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
    let mut full = duplicate_array(5, &payload);
    full.extend_from_slice(&[0x01, 0xff]);
    buffalo.write(ParameterType::BUFFER42 as u32, &json!(full), &json!({}));
    assert_eq!(buffalo.get_position(), 43);
    let mut expected = vec![0x00];
    expected.extend_from_slice(&full);
    expected.push(0x00);
    assert_eq!(buffalo.get_buffer(), expected);
}

#[test]
#[ignore = "not implemented yet"]
fn buffer8_read() {
    let buffer = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09];
    let mut buffalo = BuffaloZnp::new(buffer.clone(), 2);
    let value = buffalo.read(ParameterType::BUFFER8 as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 10);
    assert_eq!(value, json!(&buffer[2..]));
}

#[test]
#[ignore = "not implemented yet"]
fn buffer8_write() {
    let mut buffalo = BuffaloZnp::new(vec![0; 9], 1);
    let payload = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
    buffalo.write(ParameterType::BUFFER8 as u32, &json!(payload), &json!({}));
    assert_eq!(buffalo.get_position(), 9);
    let mut expected = vec![0x00];
    expected.extend_from_slice(&payload);
    assert_eq!(buffalo.get_buffer(), expected);
}

#[test]
#[ignore = "not implemented yet"]
fn buffer8_write_length_consistent() {
    let result = std::panic::catch_unwind(|| {
        let mut buffalo = BuffaloZnp::new(vec![0; 9], 0);
        let payload: Vec<u8> = (0..=8).collect();
        buffalo.write(ParameterType::BUFFER8 as u32, &json!(payload), &json!({}));
    });
    assert!(result.is_err());
}

#[test]
#[ignore = "not implemented yet"]
fn cant_read_unsupported_type() {
    let result = std::panic::catch_unwind(|| {
        let mut buffalo = BuffaloZnp::new(vec![0; 0], 0);
        buffalo.read(9999, &json!({}));
    });
    assert!(result.is_err());
}

#[test]
#[ignore = "not implemented yet"]
fn close_without_initialization() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn coverage_logger() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ieeeaddr_read() {
    let mut buffalo = BuffaloZnp::new(IEEE_ADDR2_HEX.to_vec(), 0);
    let value = buffalo.read(ParameterType::IEEEADDR as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 8);
    assert_eq!(value, json!(IEEE_ADDR2));
}

#[test]
#[ignore = "not implemented yet"]
fn ieeeaddr_write() {
    let mut buffalo = BuffaloZnp::new(vec![0; 8], 0);
    buffalo.write(
        ParameterType::IEEEADDR as u32,
        &json!(IEEE_ADDR1),
        &json!({}),
    );
    assert_eq!(buffalo.get_position(), 8);
    assert_eq!(buffalo.get_buffer(), IEEE_ADDR1_HEX.to_vec());
}

#[test]
#[ignore = "not implemented yet"]
fn int8_read() {
    let mut buffalo = BuffaloZnp::new(vec![0x00, 0xf0, 0x00, 0x00], 1);
    let value = buffalo.read(ParameterType::INT8 as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 2);
    assert_eq!(value, json!(-16));
}

#[test]
#[ignore = "not implemented yet"]
fn int8_write() {
    let mut buffalo = BuffaloZnp::new(vec![0; 3], 1);
    buffalo.write(ParameterType::INT8 as u32, &json!(127), &json!({}));
    assert_eq!(buffalo.get_position(), 2);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0x7f, 0x00]);
}

#[test]
#[ignore = "not implemented yet"]
fn list_network_read() {
    let buffer = vec![
        0x05, 0x10, 0x10, 0x09, 0x31, 0x13, 0x01, 0x10, 0x10, 0x09, 0x31, 0x13, 0x00, 0x01,
    ];
    let mut buffalo = BuffaloZnp::new(buffer, 1);
    let value = buffalo.read(ParameterType::LIST_NETWORK as u32, &json!({"length": 2}));
    assert_eq!(buffalo.get_position(), 13);
    assert_eq!(
        value,
        json!([
            {
                "beaconOrder": 3,
                "logicalChannel": 9,
                "neightborPanId": 4112,
                "permitJoin": 1,
                "stackProfile": 1,
                "superFrameOrder": 1,
                "zigbeeVersion": 3,
            },
            {
                "beaconOrder": 3,
                "logicalChannel": 9,
                "neightborPanId": 4112,
                "permitJoin": 0,
                "stackProfile": 1,
                "superFrameOrder": 1,
                "zigbeeVersion": 3,
            },
        ])
    );
}

#[test]
#[ignore = "not implemented yet"]
fn list_network_write() {
    let result = std::panic::catch_unwind(|| {
        let mut buffalo = BuffaloZnp::new(vec![0; 10], 0);
        buffalo.write(ParameterType::LIST_NETWORK as u32, &json!([]), &json!({}));
    });
    assert!(result.is_err());
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint16_read() {
    let mut buffalo = BuffaloZnp::new(vec![0x00, 0x00, 0x04, 0x00, 0x08], 1);
    let value = buffalo.read(ParameterType::LIST_UINT16 as u32, &json!({"length": 2}));
    assert_eq!(buffalo.get_position(), 5);
    assert_eq!(value, json!([1024, 2048]));
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint16_write() {
    let mut buffalo = BuffaloZnp::new(vec![0; 5], 1);
    let payload = vec![1024, 2048];
    buffalo.write(
        ParameterType::LIST_UINT16 as u32,
        &json!(payload),
        &json!({}),
    );
    assert_eq!(buffalo.get_position(), 5);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0x00, 0x04, 0x00, 0x08]);
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint8_read() {
    let mut buffalo = BuffaloZnp::new(vec![0x00, 0x00, 0x04, 0x08], 2);
    let value = buffalo.read(ParameterType::LIST_UINT8 as u32, &json!({"length": 2}));
    assert_eq!(buffalo.get_position(), 4);
    assert_eq!(value, json!([4, 8]));
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint8_write() {
    let mut buffalo = BuffaloZnp::new(vec![0; 4], 1);
    let payload = vec![200, 100];
    buffalo.write(
        ParameterType::LIST_UINT8 as u32,
        &json!(payload),
        &json!({}),
    );
    assert_eq!(buffalo.get_position(), 3);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0xc8, 0x64, 0x00]);
}

#[test]
#[ignore = "not implemented yet"]
fn open() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn open_first_ping_fails_should_send_reset_bootloader() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn open_and_close() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn open_and_close_by_serialport_event() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn open_and_close_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn open_and_close_tcp_port() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn open_tcp_port_with_socket_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn open_with_defaults() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn open_with_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn open_with_error_when_serialport_is_not_open() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn serialport_error_do_nothing() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uint16_read() {
    let mut buffalo = BuffaloZnp::new(vec![0x00, 0x03, 0xff, 0x00], 1);
    let value = buffalo.read(ParameterType::UINT16 as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 3);
    assert_eq!(value, json!(65283));
}

#[test]
#[ignore = "not implemented yet"]
fn uint16_write() {
    let mut buffalo = BuffaloZnp::new(vec![0; 3], 1);
    buffalo.write(ParameterType::UINT16 as u32, &json!(1020), &json!({}));
    assert_eq!(buffalo.get_position(), 3);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0xfc, 0x03]);
}

#[test]
#[ignore = "not implemented yet"]
fn uint32_read() {
    let mut buffalo = BuffaloZnp::new(vec![0x01, 0x03, 0xff, 0xff], 0);
    let value = buffalo.read(ParameterType::UINT32 as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 4);
    assert_eq!(value, json!(4294902529u64));
}

#[test]
#[ignore = "not implemented yet"]
fn uint32_write() {
    let mut buffalo = BuffaloZnp::new(vec![0; 6], 2);
    buffalo.write(ParameterType::UINT32 as u32, &json!(1065283), &json!({}));
    assert_eq!(buffalo.get_position(), 6);
    assert_eq!(
        buffalo.get_buffer(),
        vec![0x00, 0x00, 0x43, 0x41, 0x10, 0x00]
    );
}

#[test]
#[ignore = "not implemented yet"]
fn uint8_read() {
    let mut buffalo = BuffaloZnp::new(vec![0x00, 0x03, 0x00, 0x00], 1);
    let value = buffalo.read(ParameterType::UINT8 as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 2);
    assert_eq!(value, json!(3));
}

#[test]
#[ignore = "not implemented yet"]
fn uint8_write() {
    let mut buffalo = BuffaloZnp::new(vec![0; 3], 1);
    buffalo.write(ParameterType::UINT8 as u32, &json!(240), &json!({}));
    assert_eq!(buffalo.get_position(), 2);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0xf0, 0x00]);
}

#[test]
#[ignore = "not implemented yet"]
fn znp() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zpi_object_parse_payload_for_end_device_annce_ind() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zpi_object_parse_payload_for_nwk_addr_rsp() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zpi_object_throw_error_on_missing_write_parser() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zpi_object_throw_error_on_unknown_command() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zpi_object_throw_error_on_unknown_parameters() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zpi_object_with_cmd_and_non_sapi_is_not_reset_command() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_receive() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_receive_malformed() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_request_areq() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_request_areq_reset() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_request_sreq() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_request_sreq_failed() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_request_sreq_failed_should_cancel_waiter_when_provided() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_request_sreq_with_parsed_in_between() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_request_zdo() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_request_zdo_failure() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_request_zdo_success() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_request_zdo_failed_should_cancel_waiter_when_provided() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_request_timeout() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_request_timeout_for_startup_from_app_is_longer() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_request_with_non_existing_cmd() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_request_with_non_existing_subsystem() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_request_without_init() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_request_responses_comes_after_timeout() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_request_wait_for() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_request_with_reply_should_throw_error_when_request_as_no_reply() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_wait_for_with_payload_mismatch() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_wait_for_with_state() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_wait_for_with_target_as_ieee() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_wait_for_with_target_as_ieee_forced_to_timeout_because_invalid_zdo_status_no_payload_to_match_against(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_wait_for_with_target_as_network_address() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znp_wait_for_with_transid() {
    todo!()
}
