// Original: .upstream/zigbee-herdsman/test/adapter/zboss/fixZdoResponse.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - ACTIVE_ENDPOINTS_RESPONSE
//   - MATCH_DESCRIPTORS_RESPONSE
//   - NODE_DESCRIPTOR_RESPONSE
//   - No fix needed FrameType.INDICATION
//   - No fix needed FrameType.RESPONSE
//   - POWER_DESCRIPTOR_RESPONSE
//   - SIMPLE_DESCRIPTOR_RESPONSE
//   - ZBOSS fix non-standard ZDO response payloads

use serde_json::json;
use zigbee_apiarist::adapter::zboss::frame::{read_zboss_frame, CommandId, FrameType, ZBOSSFrame};

fn hex_decode(value: &str) -> Vec<u8> {
    (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
#[ignore = "not implemented yet"]
fn active_endpoints_response() {
    let frame = read_zboss_frame(&hex_decode("0001060211000002f2013412"));
    let expected = ZBOSSFrame {
        version: 0,
        r#type: FrameType::RESPONSE,
        command_id: CommandId::ZDO_ACTIVE_EP_REQ,
        tsn: 17,
        payload: json!({
            "category": 0,
            "zdoClusterId": 0x8005,
            "zdo": [0, {"nwkAddress": 0x1234, "endpointList": [242, 1]}],
        }),
    };
    assert_eq!(frame, expected);
}

#[test]
#[ignore = "not implemented yet"]
fn match_descriptors_response() {
    let frame = read_zboss_frame(&hex_decode("0001070211000002f2013412"));
    let expected = ZBOSSFrame {
        version: 0,
        r#type: FrameType::RESPONSE,
        command_id: CommandId::ZDO_MATCH_DESC_REQ,
        tsn: 17,
        payload: json!({
            "category": 0,
            "zdoClusterId": 0x8006,
            "zdo": [0, {"nwkAddress": 0x1234, "endpointList": [242, 1]}],
        }),
    };
    assert_eq!(frame, expected);
}

#[test]
#[ignore = "not implemented yet"]
fn node_descriptor_response() {
    let frame = read_zboss_frame(&hex_decode("000104021100000000000000000000432c0000003412"));
    let expected = ZBOSSFrame {
        version: 0,
        r#type: FrameType::RESPONSE,
        command_id: CommandId::ZDO_NODE_DESC_REQ,
        tsn: 17,
        payload: json!({
            "category": 0,
            "zdoClusterId": 0x8002,
            "zdo": [0, {
                "nwkAddress": 0x1234,
                "logicalType": 0,
                "fragmentationSupported": serde_json::Value::Null,
                "apsFlags": 0,
                "frequencyBand": 0,
                "capabilities": {
                    "alternatePANCoordinator": 0,
                    "deviceType": 0,
                    "powerSource": 0,
                    "rxOnWhenIdle": 0,
                    "reserved1": 0,
                    "reserved2": 0,
                    "securityCapability": 0,
                    "allocateAddress": 0,
                },
                "manufacturerCode": 0,
                "maxBufSize": 0,
                "maxIncTxSize": 0,
                "serverMask": {
                    "primaryTrustCenter": 1,
                    "backupTrustCenter": 1,
                    "deprecated1": 0,
                    "deprecated2": 0,
                    "deprecated3": 0,
                    "deprecated4": 0,
                    "networkManager": 1,
                    "reserved1": 0,
                    "reserved2": 0,
                    "stackComplianceRevision": 22,
                },
                "maxOutTxSize": 0,
                "deprecated1": 0,
                "tlvs": [],
            }],
        }),
    };
    assert_eq!(frame, expected);
}

#[test]
#[ignore = "not implemented yet"]
fn no_fix_needed_frame_type_indication() {
    let frame = read_zboss_frame(&hex_decode("00020c020cda603602602bd5b3708e"));
    let expected = ZBOSSFrame {
        version: 0,
        r#type: FrameType::INDICATION,
        command_id: CommandId::ZDO_DEV_ANNCE_IND,
        tsn: 0,
        payload: json!({
            "category": serde_json::Value::Null,
            "zdoClusterId": 0x0013,
            "zdo": [0, {
                "nwkAddress": 0xda0c,
                "eui64": "0x70b3d52b60023660",
                "capabilities": {
                    "alternatePANCoordinator": 0,
                    "deviceType": 1,
                    "powerSource": 1,
                    "rxOnWhenIdle": 1,
                    "reserved1": 0,
                    "reserved2": 0,
                    "securityCapability": 0,
                    "allocateAddress": 1,
                },
            }],
        }),
    };
    assert_eq!(frame, expected);
}

#[test]
#[ignore = "not implemented yet"]
fn no_fix_needed_frame_type_response() {
    let frame = read_zboss_frame(&hex_decode("0001010211000088776655443322113412"));
    let expected = ZBOSSFrame {
        version: 0,
        r#type: FrameType::RESPONSE,
        command_id: CommandId::ZDO_NWK_ADDR_REQ,
        tsn: 17,
        payload: json!({
            "category": 0,
            "zdoClusterId": 0x8000,
            "zdo": [0, {
                "nwkAddress": 0x1234,
                "eui64": "0x1122334455667788",
                "startIndex": 0,
                "assocDevList": [],
            }],
        }),
    };
    assert_eq!(frame, expected);
}

#[test]
#[ignore = "not implemented yet"]
fn power_descriptor_response() {
    let frame = read_zboss_frame(&hex_decode("0001030211000001023412"));
    let expected = ZBOSSFrame {
        version: 0,
        r#type: FrameType::RESPONSE,
        command_id: CommandId::ZDO_POWER_DESC_REQ,
        tsn: 17,
        payload: json!({
            "category": 0,
            "zdoClusterId": 0x8003,
            "zdo": [0, {
                "nwkAddress": 0x1234,
                "currentPowerMode": 1,
                "availPowerSources": 0,
                "currentPowerSource": 2,
                "currentPowerSourceLevel": 0,
            }],
        }),
    };
    assert_eq!(frame, expected);
}

#[test]
#[ignore = "not implemented yet"]
fn simple_descriptor_response() {
    let frame = read_zboss_frame(&hex_decode(
        "0001050211000001040100000301022c2ffefebcbc3412",
    ));
    let expected = ZBOSSFrame {
        version: 0,
        r#type: FrameType::RESPONSE,
        command_id: CommandId::ZDO_SIMPLE_DESC_REQ,
        tsn: 17,
        payload: json!({
            "category": 0,
            "zdoClusterId": 0x8004,
            "zdo": [0, {
                "nwkAddress": 0x1234,
                "length": 14,
                "endpoint": 1,
                "profileId": 0x0104,
                "deviceId": 0x0000,
                "deviceVersion": 3,
                "inClusterList": [0x2f2c],
                "outClusterList": [0xfefe, 0xbcbc],
            }],
        }),
    };
    assert_eq!(frame, expected);
}
