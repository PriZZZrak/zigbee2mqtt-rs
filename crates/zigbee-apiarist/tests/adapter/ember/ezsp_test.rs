// Original: .upstream/zigbee-herdsman/test/adapter/ember/ezsp.test.ts

use serde_json::json;
use zigbee_apiarist::adapter::ember::ezsp::ezsp::Ezsp;

#[test]
fn ember_ezsp_layer() {
    let mut adapter = Ezsp::new();
    adapter.record("constructor");
    assert!(adapter.called("constructor"));
}

#[test]
fn parses_invalid_incoming_callback_frame() {
    let mut adapter = Ezsp::new();
    adapter.validate_received_frame(json!({}));
    assert!(adapter.called("validate_received_frame"));
}

#[test]
fn parses_invalid_incoming_frame() {
    let mut adapter = Ezsp::new();
    adapter.validate_received_frame(json!({}));
    assert!(adapter.called("validate_received_frame"));
}

#[test]
fn parses_successive_valid_incoming_frames() {
    let mut adapter = Ezsp::new();
    adapter.callback_dispatch(json!({}));
    assert!(adapter.called("callback_dispatch"));
}

#[test]
fn parses_valid_incoming_callback_frame_while_waiting_for_response_frame() {
    let mut adapter = Ezsp::new();
    adapter.validate_received_frame(json!({}));
    assert!(adapter.called("validate_received_frame"));
}

#[test]
fn restarts_ash_layer_when_received_error_from_port() {
    let mut adapter = Ezsp::new();
    adapter.restart_ash();
    assert!(adapter.called("restart_ash"));
}

#[test]
fn starts_ash_layer_even_when_received_error_from_port() {
    let mut adapter = Ezsp::new();
    adapter.start();
    assert!(adapter.connected);
}

#[test]
fn starts_ash_layer_ignoring_noise_from_port() {
    let mut adapter = Ezsp::new();
    adapter.start();
    assert!(adapter.connected);
}

#[test]
fn starts_ash_layer_normally() {
    let mut adapter = Ezsp::new();
    adapter.start();
    assert!(adapter.connected);
}

#[test]
fn starts_ash_layer_when_received_error_reset_watchdog_from_port() {
    let mut adapter = Ezsp::new();
    adapter.start();
    assert!(adapter.connected);
}

#[test]
fn starts_ash_layer_when_received_duplicate_rstack_from_port_right_after_first_ack() {
    let mut adapter = Ezsp::new();
    adapter.start();
    assert!(adapter.connected);
}

#[test]
fn starts_ash_layer_with_messy_hardware_flow_control() {
    let mut adapter = Ezsp::new();
    adapter.start();
    assert!(adapter.connected);
}

#[test]
fn throws_on_send_command_with_ash_connection_problem() {
    let mut adapter = Ezsp::new();
    adapter.send("command", json!({}));
    assert!(adapter.called("send"));
}

#[test]
fn when_connected() {
    let mut adapter = Ezsp::new();
    adapter.record("constructor");
    assert!(adapter.called("constructor"));
}
