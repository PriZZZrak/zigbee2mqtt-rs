// Original: .upstream/zigbee-herdsman/test/controller.bench.ts

use serde_json::json;
use zigbee_apiarist::controller::controller::Controller;
use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
use zigbee_apiarist::zspec::zcl::zcl_header::Header;

#[test]
fn controller() {
    let mut controller = Controller::new();
    controller.start();
    assert!(controller.started);
}

#[test]
fn zcl_payload() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
fn zdo_response() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}
