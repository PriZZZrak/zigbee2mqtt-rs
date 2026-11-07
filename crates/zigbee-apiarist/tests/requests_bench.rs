// Original: .upstream/zigbee-herdsman/test/requests.bench.ts

use serde_json::json;
use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;

#[test]
fn requests() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    assert_eq!(frame.header.transaction_sequence_number, 1);
}
