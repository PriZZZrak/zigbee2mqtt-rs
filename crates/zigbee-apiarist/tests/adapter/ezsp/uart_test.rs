// Original: .upstream/zigbee-herdsman/test/adapter/ezsp/uart.test.ts

use serde_json::json;
use zigbee_apiarist::adapter::ezsp::driver::frame::FrameType;
use zigbee_apiarist::adapter::ezsp::driver::parser::Parser;
use zigbee_apiarist::adapter::ezsp::driver::uart::{SerialDriver, SerialPortOptions};

fn expected_options() -> SerialPortOptions {
    SerialPortOptions {
        path: "/dev/ttyACM0".to_string(),
        baud_rate: 115200,
        rtscts: false,
        auto_open: false,
        parity: "none".to_string(),
        stop_bits: 1,
        xon: true,
        xoff: true,
    }
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn connect() {
    let mut serial_driver = SerialDriver::new();

    serial_driver.connect(json!({"path": "/dev/ttyACM0"})).await;

    assert_eq!(serial_driver.ports, vec![expected_options()]);
    assert_eq!(serial_driver.pipe_calls, 1);
    assert_eq!(serial_driver.async_open_calls, 1);
    assert_eq!(serial_driver.once_calls, 1);
    assert_eq!(serial_driver.write_buffer_calls, 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_data() {
    let mut serial_driver = SerialDriver::new();
    serial_driver.connect(json!("/dev/ttyACM0")).await;

    // send 8 frames
    for _ in 0..8 {
        serial_driver.send_data(vec![1, 2, 3]).await;
    }
    assert_eq!(serial_driver.write_buffer_calls, 2);

    // send another 2 frames - not counted until the 8 promises resolve
    serial_driver.send_data(vec![1, 2, 3]).await;
    serial_driver.send_data(vec![1, 2, 3]).await;
    assert_eq!(serial_driver.write_buffer_calls, 2);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn receive_data() {
    let mut parser = Parser::new();

    // send 4 frames
    let buffer0 = vec![0xc1, 0x02, 0x0b, 0x0a, 0x52, 0x7e];
    let buffer1 = vec![
        0x22, 0x5b, 0xb1, 0xa9, 0x0d, 0x2a, 0xc1, 0xd8, 0x19, 0x53, 0x4a, 0x14, 0xaa, 0xe9, 0x87,
        0x49, 0xfc, 0xfa, 0x26, 0x7d, 0x5e, 0xc5, 0xaa, 0xc8, 0x7e,
    ];
    let buffer2 = vec![
        0x32, 0x5b, 0xb1, 0xa9, 0x7d, 0x31, 0x2a, 0x15, 0xb6, 0x58, 0x8d, 0x4a, 0x06, 0xab, 0x55,
        0x93, 0x49, 0x9c, 0x45, 0x7b, 0x7d, 0x38, 0x39, 0xa4, 0x98, 0x74, 0xf1, 0xd7, 0x26, 0x88,
        0xfc, 0x6b, 0x2f, 0xf6, 0xe9, 0xc5, 0xde, 0x6b, 0x8f, 0xfb, 0xd8, 0xf9, 0x7e,
    ];
    let buffer3 = vec![0xa2, 0x74, 0x58, 0x7e];

    parser.push(&buffer0);
    parser.push(&buffer1);
    parser.push(&buffer2);
    parser.push(&buffer3);

    assert_eq!(parser.parsed.len(), 4);
    assert_eq!(parser.parsed[0].frame_type, FrameType::RSTACK);
    assert_eq!(parser.parsed[1].frame_type, FrameType::DATA);
    assert_eq!(parser.parsed[2].frame_type, FrameType::DATA);
    assert_eq!(parser.parsed[3].frame_type, FrameType::NAK);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn message_in_two_chunks() {
    let mut parser = Parser::new();

    let buffer1 = vec![
        0x22, 0x5b, 0xb1, 0xa9, 0x0d, 0x2a, 0xc1, 0xd8, 0x19, 0x53, 0x4a, 0x14,
    ];
    parser.push(&buffer1);
    assert_eq!(parser.parsed.len(), 0);

    let buffer2 = vec![
        0xaa, 0xe9, 0x87, 0x49, 0xfc, 0xfa, 0x26, 0x7d, 0x5e, 0xc5, 0xaa, 0xc8, 0x7e,
    ];
    parser.push(&buffer2);
    assert_eq!(parser.parsed.len(), 1);
    assert_eq!(parser.parsed[0].frame_type, FrameType::DATA);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn two_messages_in_one_chunk() {
    let mut parser = Parser::new();

    let buffer1 = vec![
        0x22, 0x5b, 0xb1, 0xa9, 0x0d, 0x2a, 0xc1, 0xd8, 0x19, 0x53, 0x4a, 0x14, 0xaa, 0xe9, 0x87,
        0x49, 0xfc, 0xfa, 0x26, 0x7d, 0x5e, 0xc5, 0xaa, 0xc8, 0x7e, 0x32, 0x5b, 0xb1, 0xa9, 0x7d,
        0x31, 0x2a, 0x15, 0xb6, 0x58, 0x8d, 0x4a, 0x06, 0xab, 0x55, 0x93, 0x49, 0x9c, 0x45, 0x7b,
        0x7d, 0x38, 0x39, 0xa4, 0x98, 0x74, 0xf1, 0xd7, 0x26, 0x88, 0xfc, 0x6b, 0x2f, 0xf6, 0xe9,
        0xc5, 0xde, 0x6b, 0x8f, 0xfb, 0xd8, 0xf9, 0x7e,
    ];
    parser.push(&buffer1);

    assert_eq!(parser.parsed.len(), 2);
    assert_eq!(parser.parsed[0].frame_type, FrameType::DATA);
    assert_eq!(parser.parsed[1].frame_type, FrameType::DATA);
}
