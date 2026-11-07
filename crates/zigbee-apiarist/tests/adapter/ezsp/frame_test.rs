// Original: .upstream/zigbee-herdsman/test/adapter/ezsp/frame.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - FRAME Parsing
//   - changeSourceRouteHandler
//   - incomingNetworkStatusHandler

use zigbee_apiarist::adapter::ezsp::driver::ezsp::EZSPFrameData;

fn hex_decode(value: &str) -> Vec<u8> {
    (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn frame_parsing() {
    let frame = EZSPFrameData::create_frame(9, 0x00c4, false, &hex_decode("0b044e")).unwrap();
    assert_eq!(frame.cls, "incomingNetworkStatusHandler");
    assert_eq!(frame.id, 0x00c4);
    assert_eq!(frame.fields["errorCode"], serde_json::json!(0x0b));
    assert_eq!(frame.fields["target"], serde_json::json!(0x4e04));
}

#[test]
fn change_source_route_handler() {
    let frame = EZSPFrameData::create_frame(8, 0x00c4, false, &hex_decode("05e399a000")).unwrap();
    assert_eq!(frame.cls, "changeSourceRouteHandler");
    assert_eq!(frame.id, 0x00c4);
    assert_eq!(frame.fields["newChildId"], serde_json::json!(0xe305));
    assert_eq!(frame.fields["newParentId"], serde_json::json!(0xa099));
}

#[test]
fn incoming_network_status_handler() {
    let frame = EZSPFrameData::create_frame(9, 0x00c4, false, &hex_decode("05e399a000")).unwrap();
    assert_eq!(frame.cls, "incomingNetworkStatusHandler");
    assert_eq!(frame.id, 0x00c4);
    assert_eq!(frame.fields["errorCode"], serde_json::json!(0x05));
    assert_eq!(frame.fields["target"], serde_json::json!(0x99e3));
}

// Upstream has a second test with the same title `incomingNetworkStatusHandler`.
#[test]
fn incoming_network_status_handler_2() {
    let frame = EZSPFrameData::create_frame(8, 0x00c4, false, &hex_decode("0b044e"));
    assert!(frame.is_none());
}
