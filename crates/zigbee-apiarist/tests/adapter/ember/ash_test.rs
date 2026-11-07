// Original: .upstream/zigbee-herdsman/test/adapter/ember/ash.test.ts

use serde_json::json;
use zigbee_apiarist::adapter::ember::uart::ash::{AshCounters, UartAsh, CONFIG_TX_K};
use zigbee_apiarist::adapter::ember::uart::queues::{
    EzspBuffer, EZSP_HOST_RX_POOL_SIZE, EZSP_MAX_FRAME_LENGTH, TX_POOL_BUFFERS,
};
use zigbee_apiarist::adapter::ezsp::driver::types::basic::EzspStatus;

const SEND_RST_BYTES: [u8; 5] = [0x1a, 0xc0, 0x38, 0xbc, 0x7e];
const RECD_RSTACK_BYTES: [u8; 6] = [0x1a, 0xc1, 0x02, 0x0b, 0x0a, 0x52];
const ASH_ACK_FIRST_BYTES: [u8; 4] = [0x80, 0x70, 0x78, 0x7e];
const SEND_ACK_FIRST_BYTES: [u8; 4] = [0x81, 0x60, 0x59, 0x7e];

fn open_opts() -> serde_json::Value {
    json!({"path": "/dev/ttyACM0", "baudRate": 115200})
}

fn data_frame(sequence_byte: u8) -> Vec<u8> {
    vec![sequence_byte, 0x42, 0x21, 0xa8, 0x59, 0x7c, 0x05, 0x7e]
}

async fn reach_connected(uart_ash: &mut UartAsh) {
    let reset_result = uart_ash.reset_ncp().await;
    uart_ash.port_emit_data(&RECD_RSTACK_BYTES);
    let start_result = uart_ash.start().await;

    assert_eq!(reset_result, EzspStatus::SUCCESS);
    assert_eq!(start_result, EzspStatus::SUCCESS);
    assert!(uart_ash.connected);

    uart_ash.send_exec();
    assert!(uart_ash.idle);
    assert_eq!(uart_ash.counters.tx_ack_frames, 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn inits_properly_and_allocates_buffers_as_needed() {
    let mut uart_ash = UartAsh::new(open_opts());

    assert!(!uart_ash.connected);
    assert!(!uart_ash.ncp_sleep_enabled);
    assert!(!uart_ash.ncp_has_callbacks);
    assert_eq!(uart_ash.tx_queue.length, 0);
    assert_eq!(uart_ash.re_tx_queue.length, 0);
    assert_eq!(uart_ash.tx_free.length, TX_POOL_BUFFERS);
    assert_eq!(uart_ash.rx_queue.length, 0);
    assert_eq!(uart_ash.rx_free.length, EZSP_HOST_RX_POOL_SIZE);
    assert_eq!(uart_ash.tx_queue.tail, None);
    assert_eq!(uart_ash.re_tx_queue.tail, None);
    assert_eq!(uart_ash.rx_queue.tail, None);

    assert_eq!(
        uart_ash.tx_free.link.as_ref().unwrap().data.len(),
        EZSP_MAX_FRAME_LENGTH
    );
    assert_eq!(
        uart_ash.rx_free.link.as_ref().unwrap().data.len(),
        EZSP_MAX_FRAME_LENGTH
    );
    assert_eq!(uart_ash.counters, AshCounters::default());

    // this is mostly Queues testing, but make sure it works in "real" context
    let link: EzspBuffer = uart_ash.tx_free.link.clone().unwrap();
    let buffer = uart_ash.tx_free.alloc_buffer().unwrap();

    assert_eq!(buffer, link);
    assert_eq!(uart_ash.tx_free.link.as_ref(), buffer.link.as_deref());
    assert_eq!(uart_ash.tx_free.length, TX_POOL_BUFFERS - 1);

    uart_ash.tx_queue.add_tail(buffer.clone());

    assert_eq!(buffer.link, None);
    assert_eq!(uart_ash.tx_queue.tail, Some(buffer.clone()));
    assert_eq!(uart_ash.tx_queue.length, 1);

    let head = uart_ash.tx_queue.remove_head();

    assert_eq!(head, Some(buffer.clone()));
    assert_eq!(head, Some(link));
    assert_eq!(uart_ash.tx_queue.tail, None);
    assert_eq!(uart_ash.tx_queue.length, 0);

    uart_ash.tx_free.free_buffer(head);
    uart_ash
        .tx_queue
        .add_tail(uart_ash.tx_free.alloc_buffer().unwrap());
    uart_ash
        .tx_queue
        .add_tail(uart_ash.tx_free.alloc_buffer().unwrap());

    assert_eq!(uart_ash.tx_queue.length, 2);
    assert_eq!(uart_ash.tx_free.length, TX_POOL_BUFFERS - 2);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn reaches_connected_state() {
    let mut uart_ash = UartAsh::new(open_opts());

    let reset_result = uart_ash.reset_ncp().await;

    assert_eq!(reset_result, EzspStatus::SUCCESS);
    assert_eq!(uart_ash.reset_ncp_calls, 1);
    assert_eq!(uart_ash.init_port_calls, 1);
    assert_eq!(uart_ash.flags, 48); // RST|CAN
    assert!(uart_ash.port_open);

    uart_ash.port_emit_data(&RECD_RSTACK_BYTES);
    let start_result = uart_ash.start().await;

    assert_eq!(start_result, EzspStatus::SUCCESS);
    assert!(uart_ash.send_exec_calls > 0);
    let mut expected_recording = SEND_RST_BYTES.to_vec();
    expected_recording.extend_from_slice(&ASH_ACK_FIRST_BYTES);
    assert_eq!(uart_ash.recording, expected_recording);
    assert!(uart_ash.connected);
    assert_eq!(
        uart_ash.counters,
        AshCounters {
            tx_all_frames: 2, // RST + ACK
            tx_ack_frames: 1, // post-RSTACK ACK
            rx_all_frames: 1, // RSTACK
            ..AshCounters::default()
        }
    );

    uart_ash.stop().await;

    assert_eq!(uart_ash.on_port_error_calls, 0);
    assert_eq!(uart_ash.on_port_close_calls, 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn sends_data_frame_to_ncp() {
    let mut uart_ash = UartAsh::new(open_opts());
    reach_connected(&mut uart_ash).await;

    // buffalo payload of `EzspFrameID.VERSION` with desiredProtocolVersion 13
    let send_buf = vec![0x00, 0x42, 0x21, 0xa8, 0x59];
    uart_ash.send(send_buf.len(), &send_buf);

    assert_eq!(uart_ash.counters.tx_data_frames, 1);
    let mut expected_recording = SEND_RST_BYTES.to_vec();
    expected_recording.extend_from_slice(&ASH_ACK_FIRST_BYTES);
    expected_recording.extend_from_slice(&data_frame(0x00));
    assert_eq!(uart_ash.recording, expected_recording);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn sends_data_frame_and_receives_response_from_ncp() {
    let mut uart_ash = UartAsh::new(open_opts());
    reach_connected(&mut uart_ash).await;

    let send_buf = vec![0x00, 0x42, 0x21, 0xa8, 0x59];
    uart_ash.send(send_buf.len(), &send_buf);

    // just an ACK, doesn't matter what it is
    uart_ash.port_emit_data(&SEND_ACK_FIRST_BYTES);

    assert_eq!(uart_ash.counters.tx_ack_frames, 1);
    assert_eq!(uart_ash.counters.rx_ack_frames, 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn todo_sends_data_frame_with_nr_flags_when_buffers_are_low_on_host() {
    // Upstream: TODO (empty test).
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn todo_sends_data_frame_but_times_out_waiting_for_response() {
    // Upstream: TODO (empty test).
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn todo_resends_data_frame() {
    // Upstream: TODO (empty test).
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn allows_sending_up_to_tx_k_frames_before_receiving_ack() {
    let mut uart_ash = UartAsh::new(open_opts());
    reach_connected(&mut uart_ash).await;

    let send_buf = vec![0x00, 0x42, 0x21, 0xa8, 0x59];
    for _ in 0..=CONFIG_TX_K {
        uart_ash.send(send_buf.len(), &send_buf);
    }

    assert_eq!(uart_ash.counters.tx_data_frames, 3);
    assert_eq!(uart_ash.tx_queue.length, 1);

    let mut expected_recording = SEND_RST_BYTES.to_vec();
    expected_recording.extend_from_slice(&ASH_ACK_FIRST_BYTES);
    expected_recording.extend_from_slice(&data_frame(0x00)); // DATA 1
    expected_recording.extend_from_slice(&data_frame(0x10)); // DATA 2
    expected_recording.extend_from_slice(&data_frame(0x20)); // DATA 3
    assert_eq!(uart_ash.recording, expected_recording);
}
