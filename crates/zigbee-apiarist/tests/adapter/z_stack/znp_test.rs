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
use zigbee_apiarist::adapter::z_stack::unpi::constants::{Subsystem, Type};
use zigbee_apiarist::adapter::z_stack::unpi::frame::Frame;
use zigbee_apiarist::adapter::z_stack::znp::buffalo_znp::BuffaloZnp;
use zigbee_apiarist::adapter::z_stack::znp::parameter_type::ParameterType;
use zigbee_apiarist::adapter::z_stack::znp::znp::Znp;
use zigbee_apiarist::adapter::z_stack::znp::zpi_object::{ParameterDef, ZpiObject};
use zigbee_apiarist::utils::logger::console_logger;
use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;

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

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
#[test]
fn buffer_read() {
    let buffer = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09];
    let mut buffalo = BuffaloZnp::new(buffer.clone(), 2);
    let value = buffalo.read(ParameterType::BUFFER as u32, &json!({"length": 1}));
    assert_eq!(buffalo.get_position(), 3);
    assert_eq!(value, json!([buffer[2]]));
}

#[test]
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
fn buffer100_write() {
    let mut buffalo = BuffaloZnp::new(vec![0; 100], 0);
    let payload = duplicate_array(20, &[0x00, 0x01, 0x02, 0x03, 0x04]);
    buffalo.write(ParameterType::BUFFER100 as u32, &json!(payload), &json!({}));
    assert_eq!(buffalo.get_position(), 100);
    assert_eq!(buffalo.get_buffer(), payload);
}

#[test]
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
fn buffer8_read() {
    let buffer = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09];
    let mut buffalo = BuffaloZnp::new(buffer.clone(), 2);
    let value = buffalo.read(ParameterType::BUFFER8 as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 10);
    assert_eq!(value, json!(&buffer[2..]));
}

#[test]
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
fn buffer8_write_length_consistent() {
    let result = std::panic::catch_unwind(|| {
        let mut buffalo = BuffaloZnp::new(vec![0; 9], 0);
        let payload: Vec<u8> = (0..=8).collect();
        buffalo.write(ParameterType::BUFFER8 as u32, &json!(payload), &json!({}));
    });
    assert!(result.is_err());
}

#[test]
fn cant_read_unsupported_type() {
    let result = std::panic::catch_unwind(|| {
        let mut buffalo = BuffaloZnp::new(vec![0; 0], 0);
        buffalo.read(9999, &json!({}));
    });
    assert!(result.is_err());
}

#[tokio::test]
async fn close_without_initialization() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.on_close();
    znp.flush_and_close_error = Some("failed!".to_string());
    znp.close().await.unwrap();

    assert_eq!(znp.serial.flush_and_close_calls, 0);
    assert_eq!(znp.close_calls, 1);
}

#[test]
fn coverage_logger() {
    let logger = console_logger();
    logger.warning("Test warning", "TestNS");
    logger.error("Test error", "TestNS");
}

#[test]
fn ieeeaddr_read() {
    let mut buffalo = BuffaloZnp::new(IEEE_ADDR2_HEX.to_vec(), 0);
    let value = buffalo.read(ParameterType::IEEEADDR as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 8);
    assert_eq!(value, json!(IEEE_ADDR2));
}

#[test]
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
fn int8_read() {
    let mut buffalo = BuffaloZnp::new(vec![0x00, 0xf0, 0x00, 0x00], 1);
    let value = buffalo.read(ParameterType::INT8 as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 2);
    assert_eq!(value, json!(-16));
}

#[test]
fn int8_write() {
    let mut buffalo = BuffaloZnp::new(vec![0; 3], 1);
    buffalo.write(ParameterType::INT8 as u32, &json!(127), &json!({}));
    assert_eq!(buffalo.get_position(), 2);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0x7f, 0x00]);
}

#[test]
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
fn list_network_write() {
    let result = std::panic::catch_unwind(|| {
        let mut buffalo = BuffaloZnp::new(vec![0; 10], 0);
        buffalo.write(ParameterType::LIST_NETWORK as u32, &json!([]), &json!({}));
    });
    assert!(result.is_err());
}

#[test]
fn list_uint16_read() {
    let mut buffalo = BuffaloZnp::new(vec![0x00, 0x00, 0x04, 0x00, 0x08], 1);
    let value = buffalo.read(ParameterType::LIST_UINT16 as u32, &json!({"length": 2}));
    assert_eq!(buffalo.get_position(), 5);
    assert_eq!(value, json!([1024, 2048]));
}

#[test]
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
fn list_uint8_read() {
    let mut buffalo = BuffaloZnp::new(vec![0x00, 0x00, 0x04, 0x08], 2);
    let value = buffalo.read(ParameterType::LIST_UINT8 as u32, &json!({"length": 2}));
    assert_eq!(buffalo.get_position(), 4);
    assert_eq!(value, json!([4, 8]));
}

#[test]
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

#[tokio::test]
async fn open() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();

    assert_eq!(znp.serial.constructor_calls.len(), 1);
    assert_eq!(
        znp.serial.constructor_calls[0],
        ("/dev/ttyACM0".to_string(), 100, true)
    );
    assert_eq!(znp.serial.pipe_calls, 1);
    assert_eq!(znp.serial.async_open_calls, 1);
    assert_eq!(znp.serial.once_calls, 2);
    assert_eq!(znp.serial.write_buffer_calls, 0);
}

#[tokio::test]
async fn open_first_ping_fails_should_send_reset_bootloader() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.request_error = Some("failed".to_string());
    znp.open().await.unwrap();

    assert_eq!(znp.serial.constructor_calls.len(), 1);
    assert_eq!(
        znp.serial.constructor_calls[0],
        ("/dev/ttyACM0".to_string(), 100, true)
    );
    assert_eq!(znp.serial.pipe_calls, 1);
    assert_eq!(znp.serial.async_open_calls, 1);
    assert_eq!(znp.serial.write_buffer_calls, 1);
    assert_eq!(znp.serial.once_calls, 2);
}

#[tokio::test]
async fn open_and_close() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.on_close();
    assert!(!znp.is_initialized());
    znp.open().await.unwrap();
    assert!(znp.is_initialized());
    znp.close().await.unwrap();
    assert!(!znp.is_initialized());

    assert_eq!(znp.serial.flush_and_close_calls, 1);
    assert_eq!(znp.close_calls, 1);
}

#[tokio::test]
async fn open_and_close_by_serialport_event() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.on_close();
    znp.open().await.unwrap();

    znp.emit_serial_close();

    assert_eq!(znp.close_calls, 1);
}

#[tokio::test]
async fn open_and_close_error() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.on_close();
    znp.flush_and_close_error = Some("failed!".to_string());
    znp.open().await.unwrap();

    let error = znp.close().await.unwrap_err();

    assert_eq!(znp.serial.flush_and_close_calls, 1);
    assert_eq!(error, "failed!");
    assert_eq!(znp.close_calls, 1);
}

#[tokio::test]
async fn open_and_close_tcp_port() {
    let mut znp = Znp::new("tcp://localhost:8080", Some(100), Some(false));
    znp.open().await.unwrap();

    assert_eq!(znp.serial.socket_connect_calls, 1);
    assert_eq!(
        znp.serial.socket_connect,
        Some((8080, "localhost".to_string()))
    );
    assert!(znp.is_initialized());
    assert_eq!(znp.serial.write_buffer_calls, 1);

    znp.close().await.unwrap();
    assert_eq!(znp.serial.socket_destroy_calls, 1);
}

#[tokio::test]
async fn open_tcp_port_with_socket_error() {
    let mut znp = Znp::new("tcp://localhost:666", Some(100), Some(false));
    znp.socket_connect_error = true;

    let error = znp.open().await.unwrap_err();

    assert_eq!(error, "Error while opening socket");
    assert!(!znp.is_initialized());
}

#[tokio::test]
async fn open_with_defaults() {
    let mut znp = Znp::new("/dev/ttyACM0", None, None);
    znp.open().await.unwrap();

    assert_eq!(znp.serial.constructor_calls.len(), 1);
    assert_eq!(
        znp.serial.constructor_calls[0],
        ("/dev/ttyACM0".to_string(), 115200, false)
    );
    assert_eq!(znp.serial.pipe_calls, 1);
    assert_eq!(znp.serial.async_open_calls, 1);
    assert_eq!(znp.serial.once_calls, 2);
}

#[tokio::test]
async fn open_with_error() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.async_open_error = Some("failed!".to_string());
    znp.serial.is_open = true;

    let error = znp.open().await.unwrap_err();

    assert_eq!(znp.serial.constructor_calls.len(), 1);
    assert_eq!(
        znp.serial.constructor_calls[0],
        ("/dev/ttyACM0".to_string(), 100, true)
    );
    assert_eq!(error, "failed!");
    assert_eq!(znp.serial.pipe_calls, 1);
    assert_eq!(znp.serial.async_open_calls, 1);
    assert_eq!(znp.serial.close_calls, 1);
    assert_eq!(znp.serial.write_buffer_calls, 0);
    assert_eq!(znp.serial.once_calls, 0);
}

#[tokio::test]
async fn open_with_error_when_serialport_is_not_open() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.async_open_error = Some("failed!".to_string());
    znp.serial.is_open = false;

    let error = znp.open().await.unwrap_err();

    assert_eq!(znp.serial.constructor_calls.len(), 1);
    assert_eq!(
        znp.serial.constructor_calls[0],
        ("/dev/ttyACM0".to_string(), 100, true)
    );
    assert_eq!(error, "failed!");
    assert_eq!(znp.serial.pipe_calls, 1);
    assert_eq!(znp.serial.async_open_calls, 1);
    assert_eq!(znp.serial.close_calls, 0);
    assert_eq!(znp.serial.write_buffer_calls, 0);
    assert_eq!(znp.serial.once_calls, 0);
}

#[tokio::test]
async fn serialport_error_do_nothing() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();

    znp.emit_serial_error();
    // Upstream asserts nothing; the handler must not panic or close.
    assert!(znp.is_initialized());
}

#[test]
fn uint16_read() {
    let mut buffalo = BuffaloZnp::new(vec![0x00, 0x03, 0xff, 0x00], 1);
    let value = buffalo.read(ParameterType::UINT16 as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 3);
    assert_eq!(value, json!(65283));
}

#[test]
fn uint16_write() {
    let mut buffalo = BuffaloZnp::new(vec![0; 3], 1);
    buffalo.write(ParameterType::UINT16 as u32, &json!(1020), &json!({}));
    assert_eq!(buffalo.get_position(), 3);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0xfc, 0x03]);
}

#[test]
fn uint32_read() {
    let mut buffalo = BuffaloZnp::new(vec![0x01, 0x03, 0xff, 0xff], 0);
    let value = buffalo.read(ParameterType::UINT32 as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 4);
    assert_eq!(value, json!(4294902529u64));
}

#[test]
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
fn uint8_read() {
    let mut buffalo = BuffaloZnp::new(vec![0x00, 0x03, 0x00, 0x00], 1);
    let value = buffalo.read(ParameterType::UINT8 as u32, &json!({}));
    assert_eq!(buffalo.get_position(), 2);
    assert_eq!(value, json!(3));
}

#[test]
fn uint8_write() {
    let mut buffalo = BuffaloZnp::new(vec![0; 3], 1);
    buffalo.write(ParameterType::UINT8 as u32, &json!(240), &json!({}));
    assert_eq!(buffalo.get_position(), 2);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0xf0, 0x00]);
}

#[test]
fn zpi_object_parse_payload_for_end_device_annce_ind() {
    let buffer = vec![0, 0, 0, 1, 1, 2, 3, 4, 5, 6, 7, 8, 5];
    let frame = Frame::new(Type::AREQ, Subsystem::ZDO, 193, buffer, 0, 0);
    let obj = ZpiObject::from_unpi_frame(&frame);
    assert_eq!(
        obj.payload["zdo"],
        json!([
            0,
            {
                "capabilities": {
                    "allocateAddress": 0,
                    "alternatePANCoordinator": 1,
                    "deviceType": 0,
                    "powerSource": 1,
                    "reserved1": 0,
                    "reserved2": 0,
                    "rxOnWhenIdle": 0,
                    "securityCapability": 0,
                },
                "eui64": "0x0807060504030201",
                "nwkAddress": 256,
            },
        ])
    );
}

#[test]
fn zpi_object_parse_payload_for_nwk_addr_rsp() {
    let buffer = vec![
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x01, 0x01, 0x00, 0x02, 0x10, 0x10,
        0x11, 0x11,
    ];
    let frame = Frame::new(Type::AREQ, Subsystem::ZDO, 128, buffer, 0, 0);
    let obj = ZpiObject::from_unpi_frame(&frame);
    assert_eq!(
        obj.payload["zdo"],
        json!([
            0,
            {
                "assocDevList": [4112, 4369],
                "eui64": "0x0807060504030201",
                "nwkAddress": 257,
                "startIndex": 0,
            },
        ])
    );
}

#[test]
fn zpi_object_throw_error_on_missing_write_parser() {
    // Upstream passes parameterType 9999999.
    let obj = ZpiObject::new(
        Type::POLL,
        Subsystem::RESERVED,
        "dummy",
        0,
        json!({}),
        vec![ParameterDef {
            name: "nonExisting".to_string(),
            parameter_type: 9999999,
        }],
    );
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        obj.create_payload_buffer();
    }));
    assert!(result.is_err());
}

#[test]
fn zpi_object_throw_error_on_unknown_command() {
    // Upstream uses command ID 99999, which does not fit the 1-byte UNPI field.
    let frame = Frame::new(Type::SREQ, Subsystem::AF, 99, Vec::new(), 0, 0);
    let result = std::panic::catch_unwind(|| {
        ZpiObject::from_unpi_frame(&frame);
    });
    assert!(result.is_err());
}

#[test]
fn zpi_object_throw_error_on_unknown_parameters() {
    let frame = Frame::new(Type::SRSP, Subsystem::AF, 128, Vec::new(), 0, 0);
    let result = std::panic::catch_unwind(|| {
        ZpiObject::from_unpi_frame(&frame);
    });
    assert!(result.is_err());
}

#[test]
fn zpi_object_with_cmd_and_non_sapi_is_not_reset_command() {
    let obj = ZpiObject::new(
        Type::SREQ,
        Subsystem::AF,
        "systemReset",
        0,
        json!({}),
        Vec::new(),
    );
    assert!(!obj.is_reset_command());
}

#[tokio::test]
async fn znp_receive() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();

    let frame = Frame::new(
        Type::SRSP,
        Subsystem::SYS,
        0x02,
        vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x01, 0x01, 0x01, 0x01],
        0,
        0,
    );
    znp.receive_frame(&frame);

    assert_eq!(znp.received.len(), 1);
    let obj = &znp.received[0];
    assert_eq!(obj.command.name, "version");
    assert_eq!(obj.command.id, 2);
    assert_eq!(
        obj.payload,
        json!({
            "maintrel": 5,
            "majorrel": 3,
            "minorrel": 4,
            "product": 2,
            "revision": 16843009,
            "transportrev": 1,
        })
    );
    assert_eq!(obj.subsystem, Subsystem::SYS);
    assert_eq!(obj.r#type, Type::SRSP);
}

#[tokio::test]
async fn znp_receive_malformed() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();

    let frame = Frame::new(
        Type::SRSP,
        Subsystem::SYS,
        0x02,
        vec![0x01, 0x02, 0x03, 0x04],
        0,
        0,
    );
    znp.receive_frame(&frame);

    assert_eq!(znp.received.len(), 0);
}

#[tokio::test]
async fn znp_request_areq() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();

    let result = znp
        .request(
            Subsystem::SAPI as u16,
            "startConfirm",
            json!({"status": 1}),
            None,
        )
        .await
        .unwrap();

    let frame = &znp.written_frames[0];
    assert_eq!(znp.written_frames.len(), 1);
    assert_eq!(frame.commandID, 128);
    assert_eq!(frame.subsystem, Subsystem::SAPI);
    assert_eq!(frame.r#type, Type::AREQ);
    assert_eq!(frame.data, vec![1]);

    assert!(result.is_none());
}

#[tokio::test]
async fn znp_request_areq_reset() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();

    let result = znp
        .request(Subsystem::SYS as u16, "resetReq", json!({"type": 1}), None)
        .await
        .unwrap()
        .expect("reset indication");

    let frame = &znp.written_frames[0];
    assert_eq!(znp.written_frames.len(), 1);
    assert_eq!(frame.commandID, 0);
    assert_eq!(frame.subsystem, Subsystem::SYS);
    assert_eq!(frame.r#type, Type::AREQ);
    assert_eq!(frame.data, vec![1]);

    assert_eq!(result.command.name, "resetInd");
    assert_eq!(result.command.id, 0x80);
    assert_eq!(
        result.payload,
        json!({"reason": 1, "transportrev": 2, "productid": 3, "majorrel": 4, "minorrel": 5, "hwrev": 6})
    );
    assert_eq!(result.subsystem, Subsystem::SYS);
    assert_eq!(result.r#type, Type::AREQ);
}

#[tokio::test]
async fn znp_request_sreq() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();
    znp.mock_reply(Frame::new(
        Type::SRSP,
        Subsystem::SYS,
        8,
        vec![0x00, 0x02, 0x01, 0x02],
        0,
        0,
    ));

    let result = znp
        .request_with_reply(
            Subsystem::SYS as u16,
            "osalNvRead",
            json!({"id": 1, "offset": 2}),
        )
        .await
        .unwrap();

    let frame = &znp.written_frames[0];
    assert_eq!(znp.written_frames.len(), 1);
    assert_eq!(frame.commandID, 8);
    assert_eq!(frame.subsystem, Subsystem::SYS);
    assert_eq!(frame.r#type, Type::SREQ);
    assert_eq!(frame.data, vec![0x01, 0x00, 0x02]);

    assert_eq!(result.command.name, "osalNvRead");
    assert_eq!(result.command.id, 0x08);
    assert_eq!(
        result.payload,
        json!({"status": 0, "len": 2, "value": [0x01, 0x02]})
    );
    assert_eq!(result.subsystem, Subsystem::SYS);
    assert_eq!(result.r#type, Type::SRSP);
}

#[tokio::test]
async fn znp_request_sreq_failed() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();
    znp.mock_reply(Frame::new(
        Type::SRSP,
        Subsystem::SYS,
        8,
        vec![0x01, 0x02, 0x01, 0x02],
        0,
        0,
    ));

    assert_eq!(znp.waiters_size(), 0);

    let error = znp
        .request(
            Subsystem::SYS as u16,
            "osalNvRead",
            json!({"id": 1, "offset": 2}),
            None,
        )
        .await
        .unwrap_err();

    assert_eq!(znp.waiters_size(), 0);
    assert_eq!(
            error,
            "--> 'SREQ: SYS - osalNvRead - {\"id\":1,\"offset\":2}' failed with status '(0x01: FAILURE)' (expected '(0x00: SUCCESS)')"
        );
}

#[tokio::test]
async fn znp_request_sreq_failed_should_cancel_waiter_when_provided() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();
    znp.mock_reply(Frame::new(
        Type::SRSP,
        Subsystem::SYS,
        8,
        vec![0x01, 0x02, 0x01, 0x02],
        0,
        0,
    ));

    assert_eq!(znp.waiters_size(), 0);
    let waiter = znp.wait_for(Type::SRSP, Subsystem::SYS, "osalNvRead", None, None, None);
    assert_eq!(znp.waiters_size(), 1);

    let error = znp
        .request(
            Subsystem::SYS as u16,
            "osalNvRead",
            json!({"id": 1, "offset": 2}),
            Some(waiter.id),
        )
        .await
        .unwrap_err();

    assert_eq!(znp.waiters_size(), 0);
    assert_eq!(
            error,
            "--> 'SREQ: SYS - osalNvRead - {\"id\":1,\"offset\":2}' failed with status '(0x01: FAILURE)' (expected '(0x00: SUCCESS)')"
        );
}

#[tokio::test]
async fn znp_request_sreq_with_parsed_in_between() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();
    znp.mock_reply(Frame::new(Type::SRSP, Subsystem::AF, 0, vec![0x00], 0, 0));
    znp.mock_reply(Frame::new(
        Type::SRSP,
        Subsystem::SYS,
        8,
        vec![0x00, 0x02, 0x01, 0x02],
        0,
        0,
    ));

    let result = znp
        .request(
            Subsystem::SYS as u16,
            "osalNvRead",
            json!({"id": 1, "offset": 2}),
            None,
        )
        .await
        .unwrap()
        .expect("SREQ reply");

    let frame = &znp.written_frames[0];
    assert_eq!(znp.written_frames.len(), 1);
    assert_eq!(frame.commandID, 8);
    assert_eq!(frame.subsystem, Subsystem::SYS);
    assert_eq!(frame.r#type, Type::SREQ);
    assert_eq!(frame.data, vec![0x01, 0x00, 0x02]);

    assert_eq!(result.command.name, "osalNvRead");
    assert_eq!(result.command.id, 0x08);
    assert_eq!(
        result.payload,
        json!({"status": 0, "len": 2, "value": [0x01, 0x02]})
    );
    assert_eq!(result.subsystem, Subsystem::SYS);
    assert_eq!(result.r#type, Type::SRSP);
}

#[tokio::test]
async fn znp_request_zdo() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();

    let zdo_payload = vec![0x02, 0x00, 0x02, 0x00];
    znp.request_zdo(
        ClusterId::NODE_DESCRIPTOR_REQUEST,
        zdo_payload.clone(),
        Some(1),
    )
    .await
    .unwrap();

    let frame = &znp.written_frames[0];
    assert_eq!(znp.written_frames.len(), 1);
    assert_eq!(frame.commandID, 2);
    assert_eq!(frame.subsystem, Subsystem::ZDO);
    assert_eq!(frame.r#type, Type::SREQ);
    assert_eq!(frame.data, zdo_payload);
}

#[tokio::test]
async fn znp_request_zdo_failure() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();
    znp.mock_reply(Frame::new(Type::SRSP, Subsystem::ZDO, 2, vec![0x01], 0, 0));

    let zdo_payload = vec![0x02, 0x00, 0x02, 0x00];
    let error = znp
        .request_zdo(
            ClusterId::NODE_DESCRIPTOR_REQUEST,
            zdo_payload.clone(),
            None,
        )
        .await
        .unwrap_err();

    assert_eq!(
        error,
        format!(
            "--> 'SREQ: ZDO - NODE_DESCRIPTOR_REQUEST - {}' failed with status '(0x01: FAILURE)'",
            hex_encode(&zdo_payload)
        )
    );
}

#[tokio::test]
async fn znp_request_zdo_success() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();
    znp.mock_reply(Frame::new(Type::SRSP, Subsystem::ZDO, 2, vec![0x00], 0, 0));

    let waiter = znp.wait_for(Type::SRSP, Subsystem::ZDO, "nodeDescReq", None, None, None);
    let zdo_payload = vec![0x02, 0x00, 0x02, 0x00];
    znp.request_zdo(ClusterId::NODE_DESCRIPTOR_REQUEST, zdo_payload, Some(1))
        .await
        .unwrap();

    let object = waiter.start().promise.await.unwrap();
    assert_eq!(object.payload, json!({"status": 0x00}));
}

#[tokio::test]
async fn znp_request_zdo_failed_should_cancel_waiter_when_provided() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();
    znp.mock_reply(Frame::new(Type::SRSP, Subsystem::ZDO, 2, vec![0x01], 0, 0));

    assert_eq!(znp.waiters_size(), 0);
    let waiter = znp.wait_for(Type::AREQ, Subsystem::ZDO, "nodeDescRsp", None, None, None);
    assert_eq!(znp.waiters_size(), 1);

    let zdo_payload = vec![0x02, 0x00, 0x02, 0x00];
    let error = znp
        .request_zdo(
            ClusterId::NODE_DESCRIPTOR_REQUEST,
            zdo_payload.clone(),
            Some(waiter.id),
        )
        .await
        .unwrap_err();

    assert_eq!(znp.waiters_size(), 0);
    assert_eq!(
        error,
        format!(
            "--> 'SREQ: ZDO - NODE_DESCRIPTOR_REQUEST - {}' failed with status '(0x01: FAILURE)'",
            hex_encode(&zdo_payload)
        )
    );
}

#[tokio::test]
async fn znp_request_timeout() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();
    znp.mock_no_reply();

    let error = znp
        .request(
            Subsystem::SYS as u16,
            "osalNvRead",
            json!({"id": 1, "offset": 2}),
            None,
        )
        .await
        .unwrap_err();

    assert_eq!(error, "SRSP - SYS - osalNvRead after 6000ms");
}

#[tokio::test]
async fn znp_request_timeout_for_startup_from_app_is_longer() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();
    znp.mock_no_reply();

    let error = znp
        .request(
            Subsystem::ZDO as u16,
            "startupFromApp",
            json!({"startdelay": 100}),
            None,
        )
        .await
        .unwrap_err();

    assert_eq!(error, "SRSP - ZDO - startupFromApp after 40000ms");
}

#[tokio::test]
async fn znp_request_with_non_existing_cmd() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();

    let error = znp
        .request(
            Subsystem::SAPI as u16,
            "nonExisting",
            json!({"status": 1}),
            None,
        )
        .await
        .unwrap_err();

    assert_eq!(
        error,
        "Command request 'nonExisting' from subsystem '6' not found"
    );
}

#[tokio::test]
async fn znp_request_with_non_existing_subsystem() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();

    let error = znp
        .request(999, "startConfirm", json!({"status": 1}), None)
        .await
        .unwrap_err();

    assert_eq!(error, "Subsystem '999' does not exist");
}

#[tokio::test]
async fn znp_request_without_init() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));

    let error = znp
        .request(
            Subsystem::SAPI as u16,
            "startConfirm",
            json!({"status": 1}),
            None,
        )
        .await
        .unwrap_err();

    assert_eq!(
        error,
        "Cannot request when znp has not been initialized yet"
    );
}

#[tokio::test]
async fn znp_request_responses_comes_after_timeout() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();
    znp.mock_no_reply();

    let error = znp
        .request(
            Subsystem::SYS as u16,
            "osalNvRead",
            json!({"id": 1, "offset": 2}),
            None,
        )
        .await
        .unwrap_err();

    assert_eq!(error, "SRSP - SYS - osalNvRead after 6000ms");
}

#[tokio::test]
async fn znp_request_wait_for() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();

    let waiter = znp.wait_for(Type::SRSP, Subsystem::SYS, "osalNvRead", None, None, None);
    let object = waiter.start().promise.await.unwrap();
    assert_eq!(
        object.payload,
        json!({"len": 2, "status": 0, "value": [1, 2]})
    );
}

#[tokio::test]
async fn znp_request_with_reply_should_throw_error_when_request_as_no_reply() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();

    let error = znp
        .request_with_reply(Subsystem::ZDO as u16, "autoFindDestination", json!({}))
        .await
        .unwrap_err();

    assert_eq!(error, "Command autoFindDestination has no reply");
}

#[tokio::test]
async fn znp_wait_for_with_payload_mismatch() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();

    let waiter = znp.wait_for(
        Type::SRSP,
        Subsystem::SYS,
        "osalNvRead",
        Some(json!("abcd")),
        None,
        None,
    );
    let error = waiter.start().promise.await.unwrap_err();
    assert_eq!(error, "SRSP - SYS - osalNvRead after 10000ms");
}

#[tokio::test]
async fn znp_wait_for_with_state() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();

    let waiter = znp.wait_for(
        Type::AREQ,
        Subsystem::ZDO,
        "stateChangeInd",
        None,
        None,
        Some(9),
    );
    let object = waiter.start().promise.await.unwrap();
    assert_eq!(object.payload, json!({"state": 9}));
}

#[tokio::test]
async fn znp_wait_for_with_target_as_ieee() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();

    let waiter = znp.wait_for(
        Type::AREQ,
        Subsystem::ZDO,
        "nwkAddrRsp",
        Some(json!("0x0807060504030201")),
        None,
        None,
    );
    let object = waiter.start().promise.await.unwrap();
    assert_eq!(
        object.payload["zdo"],
        json!([
            0,
            {
                "assocDevList": [4112, 4369],
                "eui64": "0x0807060504030201",
                "nwkAddress": 257,
                "startIndex": 0,
            },
        ])
    );
}

#[tokio::test]
async fn znp_wait_for_with_target_as_ieee_forced_to_timeout_because_invalid_zdo_status_no_payload_to_match_against(
) {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();

    znp.next_zdo_status = Some(1);
    let waiter = znp.wait_for(
        Type::AREQ,
        Subsystem::ZDO,
        "nwkAddrRsp",
        Some(json!("0x0807060504030201")),
        None,
        None,
    );
    let error = waiter.start().promise.await.unwrap_err();
    assert_eq!(error, "AREQ - ZDO - nwkAddrRsp after 10000ms");
}

#[tokio::test]
async fn znp_wait_for_with_target_as_network_address() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();

    let waiter = znp.wait_for(
        Type::AREQ,
        Subsystem::ZDO,
        "activeEpRsp",
        Some(json!(0x1234)),
        None,
        None,
    );
    let object = waiter.start().promise.await.unwrap();
    assert_eq!(
        object.payload["zdo"],
        json!([0, {"nwkAddress": 0x1234, "endpointList": []}])
    );
}

#[tokio::test]
async fn znp_wait_for_with_transid() {
    let mut znp = Znp::new("/dev/ttyACM0", Some(100), Some(true));
    znp.open().await.unwrap();

    let waiter = znp.wait_for(
        Type::AREQ,
        Subsystem::AF,
        "dataConfirm",
        None,
        Some(123),
        None,
    );
    let object = waiter.start().promise.await.unwrap();
    assert_eq!(
        object.payload,
        json!({"status": 0, "endpoint": 1, "transid": 123})
    );
}
