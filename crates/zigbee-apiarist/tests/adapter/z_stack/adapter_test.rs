// Original: .upstream/zigbee-herdsman/test/adapter/z-stack/adapter.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Adapter disconnected
//   - Adapter disconnected dont emit when closing
//   - Add install code: Install Code + CRC
//   - Add install code: Key derived from Install Code
//   - Call znp constructor
//   - Close adapter
//   - Command should fail when in interpan
//   - Concentrator Callback Indication
//   - Device announce
//   - Device announce should discover route to end devices
//   - Device joined
//   - Device leave
//   - Do nothing wiht non areq event
//   - Get coordinator IEEE
//   - Get coordinator version
//   - Get network parameters
//   - Hard reset
//   - Ignore device leave with rejoin
//   - Incoming message raw (not ZCL)
//   - LED behaviour: disable LED false, firmware handling leds
//   - LED behaviour: disable LED false, firmware not handling leds
//   - LED behaviour: disable LED true, firmware handling leds
//   - LED behaviour: disable LED true, firmware not handling leds
//   - Network address response
//   - Node descriptor request should discover route to fix potential fails
//   - Permit join all
//   - Permit join coordinator only
//   - Permit join specific networkAddress
//   - Refuse to start when ping fails
//   - Restore interpan channel
//   - Send zcl frame group
//   - Send zcl frame group dataConfirm fails
//   - Send zcl frame group retry on MAC channel access failure
//   - Send zcl frame interpan
//   - Send zcl frame interpan throw exception data request fails
//   - Send zcl frame interpan throw exception when command has no response
//   - Send zcl frame interpan with response
//   - Send zcl frame network address
//   - Send zcl frame network address data confirm fails with default response
//   - Send zcl frame network address data confirm fails without default response
//   - Send zcl frame network address dataConfirm fails
//   - Send zcl frame network address fails because mac no ack with network address change, should retry
//   - Send zcl frame network address fails because mac no ack with network address change, without recovery
//   - Send zcl frame network address fails because mac no ack, should retry
//   - Send zcl frame network address fails because mac transaction expire when not being a parent, should retry
//   - Send zcl frame network address fails because mac transaction expire, should retry
//   - Send zcl frame network address fails because requested profileId is not configured
//   - Send zcl frame network address matches by tsn
//   - Send zcl frame network address matches on default response
//   - Send zcl frame network address retry on MAC channel access failure
//   - Send zcl frame network address should retry on dataconfirm timeout
//   - Send zcl frame network address timeout should discover route and retry
//   - Send zcl frame network address timeout should discover route, rewrite child entry and retry for sleepy end device
//   - Send zcl frame network address transaction number shouldnt go higher than 255
//   - Send zcl frame network address with default response
//   - Send zcl frame network address with default response timeout shouldnt care because command has response
//   - Send zcl frame to all - DEFAULT
//   - Send zcl frame to all - RX_ON_WHEN_IDLE
//   - Send zcl frame to all - SLEEPY
//   - Send zcl frame with APS encryption
//   - Sends proper ZDO request payload for BIND_REQUEST/UNBIND_REQUEST MULTICAST
//   - Sends proper ZDO request payload for BIND_REQUEST/UNBIND_REQUEST UNICAST
//   - Sends proper ZDO request payload for NETWORK_ADDRESS_REQUEST
//   - Sends proper ZDO request payload for NWK_UPDATE_REQUEST BROADCAST
//   - Sends proper ZDO request payload for NWK_UPDATE_REQUEST UNICAST
//   - Sends proper ZDO request payload for PERMIT_JOINING_REQUEST broadcast
//   - Sends proper ZDO request payload for PERMIT_JOINING_REQUEST to target
//   - Sends proper ZDO request payload for generic logic request
//   - Set interpan channel
//   - Should throw error when ZDO call fails
//   - Should throw error when registerEndpoints fails
//   - Soft reset
//   - Source Endpoint is selected based on profileId
//   - Start with transmit power set
//   - Support LED should go to false when LED request fails
//   - Supports backup
//   - Wait for resolves on cmd
//   - Wait for resolves on specified default response
//   - should (recommission) restore unified backup with 1.2 adapter and create backup - empty
//   - should commission network with 1.2 adapter
//   - should commission network with 1.2 adapter - default extended pan id
//   - should commission network with 1.2 adapter - old adapter without version reporting
//   - should commission network with 3.0.x adapter
//   - should commission network with 3.0.x adapter - auto concurrency
//   - should commission network with 3.0.x adapter - default extended pan id
//   - should commission network with 3.0.x adapter - unaligned 8-bit
//   - should commission network with 3.x.0 adapter - auto concurrency
//   - should commission with 3.0.x adapter - commissioned, mismatched adapter-config-backup
//   - should commission with 3.0.x adapter - empty, mismatched config/backup
//   - should create backup with 1.2 adapter
//   - should create backup with 3.0.x adapter - default security material table entry
//   - should create backup with 3.0.x adapter - emnpty security material table
//   - should create backup with 3.0.x adapter - security material table with generic record
//   - should fail to commission network with 3.0.x adapter when bdb commissioning times out
//   - should fail to commission network with 3.0.x adapter when nib fails to settle
//   - should fail to commission network with 3.0.x adapter when nib reports different pan id
//   - should fail to commission network with 3.0.x adapter with invalid pan id 65535
//   - should fail to create backup with 3.0.x adapter - adapter not commissioned - missing nib
//   - should fail to create backup with 3.0.x adapter - missing active key info
//   - should fail to create backup with 3.0.x adapter - unable to read ieee address
//   - should fail to restore (unified) backup with 3.0.x adapter - unsupported backup format
//   - should fail to restore legacy backup with 3.0.x adapter - missing NIB
//   - should fail to restore legacy backup with 3.0.x adapter - missing active key info
//   - should fail to restore legacy backup with 3.0.x adapter - missing adapter ieee address
//   - should fail to restore legacy backup with 3.0.x adapter - missing pre-configured key enabled
//   - should fail to restore legacy backup with 3.0.x adapter - pre-configured key enabled
//   - should fail to restore unified backup with 1.2 adapter - backup from newer adapter
//   - should fail to restore unified backup with 3.0.x adapter - insufficient aps link key data table size
//   - should fail to restore unified backup with 3.0.x adapter - insufficient security manager table size
//   - should fail to restore unified backup with 3.0.x adapter - insufficient tclk table size
//   - should fail to restore unified backup with 3.0.x adapter - invalid open coordinator backup version
//   - should fail to start with 3.0.x adapter - commissioned, config-adapter mismatch
//   - should fail when backup file is corrupted - Coordinator backup is corrupted
//   - should keep missing devices in backup
//   - should reset network with 1.2 adapter - config mismtach
//   - should restore legacy backup with 3.0.x adapter - empty
//   - should restore unified backup with 3.0.x adapter - commissioned, mismatched adapter-config, matching config-backup
//   - should restore unified backup with 3.0.x adapter and create backup - empty
//   - should restore unified backup with 3.0.x adapter and create backup - no tclk seed
//   - should restore unified backup with 3.x.0 adapter and create backup - empty
//   - should start network with 3.0.x adapter
//   - should start network with 3.0.x adapter - resume in coordinator mode
//   - should start network with 3.0.x adapter - resume in coordinator mode, extGroupFind failed
//   - should start with 3.0.x adapter - backward-compat - reversed extended pan id
//   - should start with 3.x.0 adapter - warn when preconfigured key doesn
//   - should start with runInconsistent option with 3.0.x adapter - commissioned, config-adapter mismatch
//   - zstack-adapter

use serde_json::json;
use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::{ZStackAdapter, ZnpCall};
use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;

fn adapter() -> ZStackAdapter {
    ZStackAdapter::new(
        json!({}),
        json!({"path": "dummy", "baudRate": 800, "rtscts": false}),
        "backup.json",
        json!({"concurrent": 3}),
    )
}

fn opts() -> serde_json::Value {
    json!({})
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn adapter_disconnected() {
    let mut adapter = adapter();
    adapter.start().await;

    adapter.on_znp_close();

    assert_eq!(adapter.disconnected, 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn adapter_disconnected_dont_emit_when_closing() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.stop().await;

    adapter.on_znp_close();

    assert_eq!(adapter.disconnected, 0);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn add_install_code_install_code_crc() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();

    adapter
        .add_install_code(
            "0x9035EAFFFE424783",
            &[
                0xae, 0x3b, 0x28, 0x72, 0x81, 0xcf, 0x16, 0xf5, 0x50, 0x73, 0x3a, 0x0c, 0xec, 0x38,
                0xaa, 0x31, 0xe8, 0x02,
            ],
            false,
        )
        .await;

    assert!(adapter.znp_calls.contains(&ZnpCall {
            subsystem: "APP_CNF".to_string(),
            command: "bdbAddInstallCode".to_string(),
            payload: json!({
                "installCodeFormat": 0x1,
                "ieeeaddr": "0x9035EAFFFE424783",
                "installCode": [0xae, 0x3b, 0x28, 0x72, 0x81, 0xcf, 0x16, 0xf5, 0x50, 0x73, 0x3a, 0x0c, 0xec, 0x38, 0xaa, 0x31, 0xe8, 0x02],
            }),
        }));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn add_install_code_key_derived_from_install_code() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();

    adapter
        .add_install_code(
            "0x9035EAFFFE424783",
            &[
                0xae, 0x3b, 0x28, 0x72, 0x81, 0xcf, 0x16, 0xf5, 0x50, 0x73, 0x3a, 0x0c, 0xec, 0x38,
                0xaa, 0x31,
            ],
            true,
        )
        .await;

    assert!(adapter.znp_calls.contains(&ZnpCall {
            subsystem: "APP_CNF".to_string(),
            command: "bdbAddInstallCode".to_string(),
            payload: json!({
                "installCodeFormat": 0x2,
                "ieeeaddr": "0x9035EAFFFE424783",
                "installCode": [0xae, 0x3b, 0x28, 0x72, 0x81, 0xcf, 0x16, 0xf5, 0x50, 0x73, 0x3a, 0x0c, 0xec, 0x38, 0xaa, 0x31],
            }),
        }));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn call_znp_constructor() {
    let adapter = adapter();
    assert_eq!(adapter.path, "dummy");
    assert_eq!(adapter.baud_rate, 800);
    assert!(!adapter.rtscts);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn close_adapter() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.stop().await;
    assert_eq!(adapter.close_calls, 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn command_should_fail_when_in_interpan() {
    let mut adapter = adapter();
    adapter.start().await;

    adapter.set_channel_inter_pan(14).await;
    adapter.in_inter_pan = true;
    adapter.send_zcl_calls = 0;

    let error = adapter
        .send_zcl_frame_to_endpoint("0x02", 2, 20, &json!({}))
        .await
        .unwrap_err();
    assert_eq!(error, "Cannot execute command, in Inter-PAN mode");
    assert_eq!(adapter.send_zcl_calls, 0);

    adapter.restore_channel_inter_pan().await;
    adapter.in_inter_pan = false;
    adapter
        .send_zcl_frame_to_endpoint("0x02", 2, 20, &json!({}))
        .await
        .unwrap();
    assert_eq!(adapter.send_zcl_calls, 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn concentrator_callback_indication() {
    let mut adapter = adapter();
    adapter.start().await;

    adapter.on_znp_received(&json!({
        "type": "AREQ",
        "subsystem": "ZDO",
        "command": "concentratorIndCb",
        "payload": {"srcaddr": 124, "extaddr": "0x123"},
    }));

    assert_eq!(adapter.zdo_responses.len(), 1);
    assert_eq!(
        adapter.zdo_responses[0].0,
        ClusterId::NETWORK_ADDRESS_RESPONSE
    );
    assert_eq!(adapter.zdo_responses[0].1[0], json!(0));
    assert_eq!(
        adapter.zdo_responses[0].1[1],
        json!({"eui64": "0x123", "nwkAddress": 124, "assocDevList": [], "startIndex": 0})
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn device_announce() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();

    adapter.on_znp_received(&json!({
        "type": "AREQ",
        "subsystem": "ZDO",
        "command": "endDeviceAnnceInd",
        "payload": {
            "status": 0,
            "data": {
                "capabilities": {"deviceType": 1},
                "eui64": "0x123",
                "nwkAddress": 123,
            },
        },
    }));

    assert_eq!(adapter.zdo_responses.len(), 1);
    assert_eq!(adapter.zdo_responses[0].0, ClusterId::END_DEVICE_ANNOUNCE);
    assert_eq!(adapter.zdo_responses[0].1[0], json!(0));
    assert_eq!(
        adapter.zdo_responses[0].1[1],
        json!({"eui64": "0x123", "nwkAddress": 123, "capabilities": {"deviceType": 1}})
    );
    assert!(adapter.znp_calls.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn device_announce_should_discover_route_to_end_devices() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();

    let event = json!({
        "type": "AREQ",
        "subsystem": "ZDO",
        "command": "endDeviceAnnceInd",
        "payload": {
            "status": 0,
            "data": {
                "capabilities": {"deviceType": 0},
                "eui64": "0x123",
                "nwkAddress": 123,
            },
        },
    });
    adapter.on_znp_received(&event);

    assert_eq!(adapter.zdo_responses.len(), 1);
    assert_eq!(adapter.zdo_responses[0].0, ClusterId::END_DEVICE_ANNOUNCE);
    assert_eq!(
        adapter.route_disc_calls,
        vec![json!({"dstAddr": 123, "options": 0, "radius": 30})]
    );

    // Should debounce route discovery.
    adapter.on_znp_received(&event);
    assert_eq!(adapter.route_disc_calls.len(), 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn device_joined() {
    let mut adapter = adapter();
    adapter.start().await;

    adapter.on_znp_received(&json!({
        "type": "AREQ",
        "subsystem": "ZDO",
        "command": "tcDeviceInd",
        "payload": {"nwkaddr": 123, "extaddr": "0x123"},
    }));

    assert_eq!(
        adapter.device_joined_events,
        vec![json!({"ieeeAddr": "0x123", "networkAddress": 123})]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn device_leave() {
    let mut adapter = adapter();
    adapter.start().await;

    adapter.on_znp_received(&json!({
        "type": "AREQ",
        "subsystem": "ZDO",
        "command": "leaveInd",
        "payload": {"srcaddr": 123, "extaddr": "0x123"},
    }));

    assert_eq!(
        adapter.device_leaves,
        vec![json!({"ieeeAddr": "0x123", "networkAddress": 123})]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn do_nothing_wiht_non_areq_event() {
    let mut adapter = adapter();
    adapter.start().await;

    adapter.on_znp_received(&json!({
        "type": "SREQ",
        "subsystem": "ZDO",
        "command": "leaveInd",
        "payload": {"srcaddr": 123, "extaddr": "0x123"},
    }));

    assert!(adapter.device_leaves.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn get_coordinator_ieee() {
    let mut adapter = adapter();
    adapter.start().await;
    assert_eq!(adapter.get_coordinator_ieee().await, "0x00124b0009d80ba7");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn get_coordinator_version() {
    let mut adapter = adapter();
    adapter.start().await;
    assert_eq!(
        adapter.get_coordinator_version().await,
        json!({"type": "ZStack3x0", "meta": {"revision": 20210430, "product": 1}})
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn get_network_parameters() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();
    let result = adapter.get_network_parameters().await;

    assert_eq!(
        adapter.znp_calls.last(),
        Some(&ZnpCall {
            subsystem: "ZDO".to_string(),
            command: "extNwkInfo".to_string(),
            payload: json!({}),
        })
    );
    assert_eq!(
        result,
        json!({"channel": 21, "extendedPanID": "0x00124b0009d69f77", "panID": 123, "nwkUpdateID": 0})
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn hard_reset() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();

    adapter.reset("hard").await;

    assert_eq!(
        adapter.znp_calls,
        vec![ZnpCall {
            subsystem: "SYS".to_string(),
            command: "resetReq".to_string(),
            payload: json!({"type": 0}),
        }]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn ignore_device_leave_with_rejoin() {
    let mut adapter = adapter();
    adapter.start().await;

    adapter.on_znp_received(&json!({
        "type": "AREQ",
        "subsystem": "ZDO",
        "command": "leaveInd",
        "payload": {"srcaddr": 123, "extaddr": "0x123", "rejoin": true},
    }));

    assert!(adapter.device_leaves.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn incoming_message_raw_not_zcl() {
    let mut adapter = adapter();
    adapter.start().await;

    adapter.on_znp_received(&json!({
        "type": "AREQ",
        "subsystem": "AF",
        "command": "incomingMsg",
        "payload": {
            "clusterid": 1,
            "srcendpoint": 20,
            "srcaddr": 2,
            "linkquality": 101,
            "groupid": 12,
            "data": [0x00, 0x01],
        },
    }));

    let payload = &adapter.zcl_payloads[0];
    assert_eq!(payload["clusterID"], json!(1));
    assert_eq!(payload["endpoint"], json!(20));
    assert_eq!(payload["groupID"], json!(12));
    assert_eq!(payload["linkquality"], json!(101));
    assert_eq!(payload["address"], json!(2));
    assert_eq!(payload["data"], json!([0x00, 0x01]));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn led_behaviour_disable_led_false_firmware_handling_leds() {
    let mut adapter = ZStackAdapter::new(
        json!({}),
        json!({"path": "dummy", "baudRate": 800, "rtscts": false}),
        "backup.json",
        json!({"disableLED": false}),
    );
    adapter.set_led_firmware(true);
    adapter.start().await;

    assert!(!adapter
        .znp_calls
        .iter()
        .any(|call| call.command == "ledControl"));
    adapter.znp_calls.clear();
    adapter.permit_join(255, Some(0)).await;
    assert!(!adapter
        .znp_calls
        .iter()
        .any(|call| call.command == "ledControl"));
    adapter.znp_calls.clear();
    adapter.permit_join(0, Some(0)).await;
    assert!(!adapter
        .znp_calls
        .iter()
        .any(|call| call.command == "ledControl"));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn led_behaviour_disable_led_false_firmware_not_handling_leds() {
    let mut adapter = ZStackAdapter::new(
        json!({}),
        json!({"path": "dummy", "baudRate": 800, "rtscts": false}),
        "backup.json",
        json!({"disableLED": false}),
    );
    adapter.set_led_firmware(false);
    adapter.start().await;

    assert!(!adapter
        .znp_calls
        .iter()
        .any(|call| call.command == "ledControl"));
    adapter.znp_calls.clear();
    adapter.permit_join(255, Some(0)).await;
    assert!(adapter
        .znp_calls
        .iter()
        .any(|call| call.command == "ledControl" && call.payload["mode"] == json!(1)));
    adapter.znp_calls.clear();
    adapter.permit_join(0, Some(0)).await;
    assert!(adapter
        .znp_calls
        .iter()
        .any(|call| call.command == "ledControl" && call.payload["mode"] == json!(0)));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn led_behaviour_disable_led_true_firmware_handling_leds() {
    let mut adapter = ZStackAdapter::new(
        json!({}),
        json!({"path": "dummy", "baudRate": 800, "rtscts": false}),
        "backup.json",
        json!({"disableLED": true}),
    );
    adapter.set_led_firmware(true);
    adapter.start().await;

    assert!(adapter.znp_calls.iter().any(|call| {
        call.command == "ledControl" && call.payload == json!({"ledid": 0xff, "mode": 5})
    }));
    adapter.znp_calls.clear();
    adapter.permit_join(255, Some(0)).await;
    assert!(!adapter
        .znp_calls
        .iter()
        .any(|call| call.command == "ledControl"));
    adapter.znp_calls.clear();
    adapter.permit_join(0, Some(0)).await;
    assert!(!adapter
        .znp_calls
        .iter()
        .any(|call| call.command == "ledControl"));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn led_behaviour_disable_led_true_firmware_not_handling_leds() {
    let mut adapter = ZStackAdapter::new(
        json!({}),
        json!({"path": "dummy", "baudRate": 800, "rtscts": false}),
        "backup.json",
        json!({"disableLED": true}),
    );
    adapter.set_led_firmware(false);
    adapter.start().await;

    assert!(adapter
        .znp_calls
        .iter()
        .any(|call| call.command == "ledControl" && call.payload["mode"] == json!(0)));
    adapter.znp_calls.clear();
    adapter.permit_join(255, Some(0)).await;
    assert!(!adapter
        .znp_calls
        .iter()
        .any(|call| call.command == "ledControl"));
    adapter.znp_calls.clear();
    adapter.permit_join(0, Some(0)).await;
    assert!(!adapter
        .znp_calls
        .iter()
        .any(|call| call.command == "ledControl"));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn network_address_response() {
    let mut adapter = adapter();
    adapter.start().await;

    adapter.on_znp_received(&json!({
        "type": "AREQ",
        "subsystem": "ZDO",
        "command": "nwkAddrRsp",
        "payload": {
            "status": 0,
            "data": {"eui64": "0x123", "nwkAddress": 124, "assocDevList": [], "startIndex": 0},
        },
    }));

    assert_eq!(adapter.zdo_responses.len(), 1);
    assert_eq!(
        adapter.zdo_responses[0].0,
        ClusterId::NETWORK_ADDRESS_RESPONSE
    );
    assert_eq!(adapter.zdo_responses[0].1[0], json!(0));
    assert_eq!(
        adapter.zdo_responses[0].1[1],
        json!({"eui64": "0x123", "nwkAddress": 124, "assocDevList": [], "startIndex": 0})
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn node_descriptor_request_should_discover_route_to_fix_potential_fails() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();
    adapter.zdo_calls.clear();
    adapter.route_disc_calls.clear();
    adapter.zdo_fail = true;

    let cluster_id = ClusterId::NODE_DESCRIPTOR_REQUEST;
    let zdo_payload = BuffaloZdo::build_request(false, cluster_id, &[json!(1234)]);
    adapter
        .send_zdo_route_discovery("0x0000000000000000", 1234, cluster_id, zdo_payload.clone())
        .await
        .unwrap();

    let mut expected = vec![0x23, 0x01];
    expected.extend_from_slice(&zdo_payload);
    assert_eq!(adapter.zdo_calls.len(), 2);
    assert_eq!(adapter.zdo_calls[0].0, cluster_id);
    assert_eq!(adapter.zdo_calls[0].1, expected);
    assert_eq!(adapter.zdo_calls[1].1, expected);
    assert_eq!(
        adapter.route_disc_calls,
        vec![json!({"dstAddr": 1234, "options": 0, "radius": 30})]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn permit_join_all() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();
    adapter.zdo_calls.clear();

    adapter.permit_join(100, None).await;

    assert_eq!(
        adapter.znp_calls,
        vec![ZnpCall {
            subsystem: "UTIL".to_string(),
            command: "ledControl".to_string(),
            payload: json!({"ledid": 3, "mode": 1}),
        }]
    );
    assert_eq!(
        adapter.zdo_calls,
        vec![(
            ClusterId::PERMIT_JOINING_REQUEST,
            vec![0x0f, 0xfc, 0xff, 100, 1],
            None
        )]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn permit_join_coordinator_only() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.zdo_calls.clear();

    adapter.permit_join(254, Some(0)).await;

    assert_eq!(
        adapter.zdo_calls,
        vec![(
            ClusterId::PERMIT_JOINING_REQUEST,
            vec![0x02, 0x00, 0x00, 254, 1],
            None
        )]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn permit_join_specific_network_address() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();
    adapter.zdo_calls.clear();

    adapter.permit_join(102, Some(42102)).await;

    assert_eq!(
        adapter.znp_calls,
        vec![ZnpCall {
            subsystem: "UTIL".to_string(),
            command: "ledControl".to_string(),
            payload: json!({"ledid": 3, "mode": 1}),
        }]
    );
    assert_eq!(
        adapter.zdo_calls,
        vec![(
            ClusterId::PERMIT_JOINING_REQUEST,
            vec![
                0x02,
                (42102u32 & 0xff) as u8,
                ((42102u32 >> 8) & 0xff) as u8,
                102,
                1
            ],
            None
        )]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn refuse_to_start_when_ping_fails() {
    let mut adapter = adapter();
    adapter.ping_error = Some("Couldnt lock port".to_string());

    let error = adapter.start_checked().await.unwrap_err();

    assert_eq!(
        error,
        "Failed to connect to the adapter (Error: Couldnt lock port)"
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn restore_interpan_channel() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.interpan_calls.clear();

    adapter.restore_channel_inter_pan().await;

    assert_eq!(adapter.interpan_calls, vec![(0u8, Vec::<u8>::new())]);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_zcl_frame_group() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();

    adapter
        .send_zcl_frame_to_group(25, vec![0x18, 100, 0, 0, 0], 1, 0, 1)
        .await;

    assert_eq!(
        adapter.znp_calls,
        vec![ZnpCall {
            subsystem: "AF".to_string(),
            command: "dataRequestExt".to_string(),
            payload: json!({
                "clusterid": 0,
                "data": [0x18, 100, 0, 0, 0],
                "destendpoint": 255,
                "dstaddr": "0x0000000000000019",
                "len": 5,
                "options": 0,
                "radius": 30,
                "srcendpoint": 1,
                "transid": 1,
                "dstaddrmode": 1,
                "dstpanid": 0,
            }),
        }]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_zcl_frame_group_data_confirm_fails() {
    let mut adapter = adapter();
    adapter.data_confirm_code = 184;
    adapter.start().await;
    adapter.znp_calls.clear();

    let error = adapter
        .send_zcl_frame_to_group(25, vec![0x18, 100, 0, 0, 0], 1, 0, 1)
        .await
        .unwrap_err();

    assert_eq!(adapter.znp_calls.len(), 1);
    assert_eq!(
        adapter.znp_calls[0],
        ZnpCall {
            subsystem: "AF".to_string(),
            command: "dataRequestExt".to_string(),
            payload: json!({
                "clusterid": 0,
                "data": [0x18, 100, 0, 0, 0],
                "destendpoint": 255,
                "dstaddr": "0x0000000000000019",
                "len": 5,
                "options": 0,
                "radius": 30,
                "srcendpoint": 1,
                "transid": 1,
                "dstaddrmode": 1,
                "dstpanid": 0,
            }),
        }
    );
    assert_eq!(
        error,
        "Data request failed with error: 'APS_DUPLICATE_ENTRY' (0xb8)"
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_zcl_frame_group_retry_on_mac_channel_access_failure() {
    let mut adapter = adapter();
    adapter.data_confirm_code = 225;
    adapter.start().await;
    adapter.znp_calls.clear();

    adapter
        .send_zcl_frame_to_group(25, vec![0x18, 100, 5, 0, 0, 0], 1, 0, 1)
        .await;

    assert_eq!(adapter.znp_calls.len(), 2);
    assert_eq!(
        adapter.znp_calls[0],
        ZnpCall {
            subsystem: "AF".to_string(),
            command: "dataRequestExt".to_string(),
            payload: json!({
                "clusterid": 0,
                "data": [0x18, 100, 5, 0, 0, 0],
                "destendpoint": 255,
                "dstaddr": "0x0000000000000019",
                "len": 6,
                "options": 0,
                "radius": 30,
                "srcendpoint": 1,
                "transid": 1,
                "dstaddrmode": 1,
                "dstpanid": 0,
            }),
        }
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_zcl_frame_interpan() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();

    adapter
        .send_zcl_frame_inter_pan_to_ieee_addr(
            4096,
            vec![9, 12, 1, 0, 0, 0, 10, 5, 6],
            "0x0017880104c9cd33",
        )
        .await;

    assert_eq!(
        adapter.znp_calls,
        vec![ZnpCall {
            subsystem: "AF".to_string(),
            command: "dataRequestExt".to_string(),
            payload: json!({
                "clusterid": 4096,
                "data": [9, 12, 1, 0, 0, 0, 10, 5, 6],
                "destendpoint": 254,
                "dstaddr": "0x0017880104c9cd33",
                "len": 9,
                "options": 0,
                "radius": 30,
                "srcendpoint": 12,
                "transid": 1,
                "dstaddrmode": 3,
                "dstpanid": 65535,
            }),
        }]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_zcl_frame_interpan_throw_exception_data_request_fails() {
    let mut adapter = adapter();
    adapter.data_request_ext_code = 99;
    adapter.start().await;
    adapter.znp_calls.clear();

    let error = adapter
        .send_zcl_frame_inter_pan_broadcast(4096, vec![9, 12, 1, 1, 0, 0, 0, 10, 5], true)
        .await
        .unwrap_err();

    assert_eq!(error, "Data request failed with code '99'");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_zcl_frame_interpan_throw_exception_when_command_has_no_response() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();

    let error = adapter
        .send_zcl_frame_inter_pan_broadcast(4096, vec![9, 12, 1, 0, 0, 0, 10, 5, 6], false)
        .await
        .unwrap_err();

    assert_eq!(
        error,
        "Command 'identifyRequest' has no response, cannot wait for response"
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_zcl_frame_interpan_with_response() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();
    adapter.interpan_response = Some(json!({"clusterID": 4096, "address": 12394}));

    let result = adapter
        .send_zcl_frame_inter_pan_broadcast(4096, vec![9, 12, 1, 1, 0, 0, 0, 10, 5], true)
        .await
        .unwrap();

    assert_eq!(
        adapter.znp_calls,
        vec![ZnpCall {
            subsystem: "AF".to_string(),
            command: "dataRequestExt".to_string(),
            payload: json!({
                "clusterid": 4096,
                "data": [9, 12, 1, 1, 0, 0, 0, 10, 5],
                "destendpoint": 254,
                "dstaddr": "0x000000000000ffff",
                "len": 9,
                "options": 0,
                "radius": 30,
                "srcendpoint": 12,
                "transid": 1,
                "dstaddrmode": 2,
                "dstpanid": 65535,
            }),
        }]
    );
    assert_eq!(result, json!({"clusterID": 4096, "address": 12394}));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_zcl_frame_network_address() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();

    adapter
        .send_zcl_frame_to_endpoint_data(
            "0x02",
            2,
            20,
            vec![0x18, 100, 0x05, 0x00, 0x00, 0x00],
            0,
            1,
        )
        .await;

    assert_eq!(adapter.send_zcl_calls, 1);
    assert_eq!(
        adapter.znp_calls,
        vec![ZnpCall {
            subsystem: "AF".to_string(),
            command: "dataRequest".to_string(),
            payload: json!({
                "clusterid": 0,
                "data": [0x18, 100, 0x05, 0x00, 0x00, 0x00],
                "destendpoint": 20,
                "dstaddr": 2,
                "len": 6,
                "options": 0,
                "radius": 30,
                "srcendpoint": 1,
                "transid": 1,
            }),
        }]
    );
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_data_confirm_fails_with_default_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_data_confirm_fails_without_default_response() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_zcl_frame_network_address_data_confirm_fails() {
    let mut adapter = adapter();
    adapter.data_confirm_code = 201;
    adapter.start().await;
    adapter.znp_calls.clear();

    let error = adapter
        .send_zcl_frame_to_endpoint_checked(
            "0x02",
            2,
            20,
            vec![0x18, 100, 0x05, 0x00, 0x00, 0x00],
            0,
            1,
            0x0104,
            false,
        )
        .await
        .unwrap_err();

    assert_eq!(
        error,
        "Data request failed with error: 'NWK_UNSUPPORTED_ATTRIBUTE' (0xc9)"
    );
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_fails_because_mac_no_ack_with_network_address_change_should_retry(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_fails_because_mac_no_ack_with_network_address_change_without_recovery(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_fails_because_mac_no_ack_should_retry() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_fails_because_mac_transaction_expire_when_not_being_a_parent_should_retry(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_fails_because_mac_transaction_expire_should_retry() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_zcl_frame_network_address_fails_because_requested_profile_id_is_not_configured() {
    let mut adapter = adapter();
    adapter.start().await;

    let error = adapter
        .send_zcl_frame_to_endpoint_checked(
            "0x02",
            2,
            20,
            vec![0x18, 100, 0x05, 0x00, 0x00, 0x00],
            0,
            1,
            9999,
            false,
        )
        .await
        .unwrap_err();

    assert_eq!(error, "Profile ID 9999 is not supported by this adapter.");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_zcl_frame_network_address_matches_by_tsn() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();

    let result = adapter
        .send_zcl_frame_to_endpoint_result("0x02", 2, 20, vec![0x18, 100, 1, 0, 0], 0, 1)
        .await
        .unwrap();

    assert_eq!(
        adapter.znp_calls,
        vec![ZnpCall {
            subsystem: "AF".to_string(),
            command: "dataRequest".to_string(),
            payload: json!({
                "clusterid": 0,
                "data": [0x18, 100, 1, 0, 0],
                "destendpoint": 20,
                "dstaddr": 2,
                "len": 5,
                "options": 0,
                "radius": 30,
                "srcendpoint": 1,
                "transid": 1,
            }),
        }]
    );
    assert_eq!(adapter.znp_calls.len(), 1);
    assert_eq!(result.endpoint, 20);
    assert_eq!(result.group_id, 12);
    assert_eq!(result.linkquality, 101);
    assert_eq!(result.address, 2);
    assert_eq!(result.data, vec![24, 100, 1, 0, 0, 0, 32, 2]);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_zcl_frame_network_address_matches_on_default_response() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();

    let result = adapter
        .send_zcl_frame_to_endpoint_result("0x02", 2, 20, vec![0x18, 100, 1, 0, 0], 0, 1)
        .await
        .unwrap();

    assert_eq!(adapter.znp_calls.len(), 1);
    assert_eq!(result.endpoint, 20);
    assert_eq!(result.group_id, 12);
    assert_eq!(result.linkquality, 101);
    assert_eq!(result.address, 2);
    assert_eq!(result.data, vec![24, 100, 11, 0, 126]);
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_retry_on_mac_channel_access_failure() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_zcl_frame_network_address_should_retry_on_dataconfirm_timeout() {
    let mut adapter = adapter();
    adapter.data_confirm_code = 9999;
    adapter.start().await;
    adapter.znp_calls.clear();

    let error = adapter
        .send_zcl_frame_to_endpoint_checked(
            "0x02",
            2,
            20,
            vec![0x18, 100, 0x05, 0x00, 0x00, 0x00],
            0,
            1,
            0x0104,
            false,
        )
        .await
        .unwrap_err();

    assert_eq!(error, "Data request failed with error: 'TIMEOUT'");
    assert_eq!(adapter.znp_calls.len(), 1);
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_timeout_should_discover_route_and_retry() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_timeout_should_discover_route_rewrite_child_entry_and_retry_for_sleepy_end_device(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_transaction_number_shouldnt_go_higher_than_255() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_zcl_frame_network_address_with_default_response() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();

    adapter
        .send_zcl_frame_to_endpoint_checked(
            "0x02",
            2,
            20,
            vec![0x18, 100, 0x05, 0x00, 0x00, 0x00],
            0,
            1,
            0x0104,
            false,
        )
        .await
        .unwrap();

    assert_eq!(adapter.send_zcl_calls, 1);
    assert_eq!(
        adapter.znp_calls,
        vec![ZnpCall {
            subsystem: "AF".to_string(),
            command: "dataRequest".to_string(),
            payload: json!({
                "clusterid": 0,
                "data": [0x18, 100, 0x05, 0x00, 0x00, 0x00],
                "destendpoint": 20,
                "dstaddr": 2,
                "len": 6,
                "options": 0,
                "radius": 30,
                "srcendpoint": 1,
                "transid": 1,
            }),
        }]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_zcl_frame_network_address_with_default_response_timeout_shouldnt_care_because_command_has_response(
) {
    let mut adapter = adapter();
    adapter.data_confirm_code = 9999;
    adapter.start().await;
    adapter.znp_calls.clear();

    adapter
        .send_zcl_frame_to_endpoint_checked(
            "0x02",
            2,
            20,
            vec![0x18, 100, 0x05, 0x00, 0x00, 0x00],
            0,
            1,
            0x0104,
            true,
        )
        .await
        .unwrap();

    assert_eq!(adapter.znp_calls.len(), 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_zcl_frame_to_all_default() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();

    adapter
        .send_zcl_frame_to_all(242, vec![0x18, 100, 0, 0, 0], 250, 65532)
        .await;

    assert_eq!(
        adapter.znp_calls,
        vec![ZnpCall {
            subsystem: "AF".to_string(),
            command: "dataRequestExt".to_string(),
            payload: json!({
                "clusterid": 0,
                "data": [0x18, 100, 0, 0, 0],
                "destendpoint": 242,
                "dstaddr": "0x000000000000fffc",
                "len": 5,
                "options": 0,
                "radius": 30,
                "srcendpoint": 250,
                "transid": 1,
                "dstaddrmode": 2,
                "dstpanid": 0,
            }),
        }]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_zcl_frame_to_all_rx_on_when_idle() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();

    adapter
        .send_zcl_frame_to_all(255, vec![0x18, 100, 0, 0, 0], 1, 65533)
        .await;

    assert_eq!(
        adapter.znp_calls,
        vec![ZnpCall {
            subsystem: "AF".to_string(),
            command: "dataRequestExt".to_string(),
            payload: json!({
                "clusterid": 0,
                "data": [0x18, 100, 0, 0, 0],
                "destendpoint": 255,
                "dstaddr": "0x000000000000fffd",
                "len": 5,
                "options": 0,
                "radius": 30,
                "srcendpoint": 1,
                "transid": 1,
                "dstaddrmode": 2,
                "dstpanid": 0,
            }),
        }]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_zcl_frame_to_all_sleepy() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();

    adapter
        .send_zcl_frame_to_all(255, vec![0x18, 100, 0, 0, 0], 1, 65535)
        .await;

    assert_eq!(
        adapter.znp_calls,
        vec![ZnpCall {
            subsystem: "AF".to_string(),
            command: "dataRequestExt".to_string(),
            payload: json!({
                "clusterid": 0,
                "data": [0x18, 100, 0, 0, 0],
                "destendpoint": 255,
                "dstaddr": "0x000000000000ffff",
                "len": 5,
                "options": 0,
                "radius": 30,
                "srcendpoint": 1,
                "transid": 1,
                "dstaddrmode": 2,
                "dstpanid": 0,
            }),
        }]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn send_zcl_frame_with_aps_encryption() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();

    adapter
        .send_zcl_frame_to_endpoint_data(
            "0x1122334455667788",
            1234,
            232,
            vec![0x18, 3, 0x00, 0x00, 0x00],
            64,
            1,
        )
        .await;

    assert_eq!(
        adapter.znp_calls,
        vec![ZnpCall {
            subsystem: "AF".to_string(),
            command: "dataRequest".to_string(),
            payload: json!({
                "clusterid": 61,
                "data": [0x18, 3, 0x00, 0x00, 0x00],
                "destendpoint": 232,
                "dstaddr": 1234,
                "len": 5,
                "options": 64,
                "radius": 30,
                "srcendpoint": 1,
                "transid": 1,
            }),
        }]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn sends_proper_zdo_request_payload_for_bind_request_unbind_request_multicast() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.zdo_calls.clear();

    let cluster_id = ClusterId::BIND_REQUEST;
    let zdo_payload = BuffaloZdo::build_request_be(
        false,
        cluster_id,
        &[
            json!("0x1122334455667788"),
            json!(3),
            json!(0x0103),
            json!(1),
            json!("0x0000000000000000"),
            json!(32),
            json!(0),
        ],
    );
    adapter
        .send_zdo(
            "0x0000000000000000",
            1234,
            cluster_id,
            zdo_payload.clone(),
            true,
        )
        .await;

    let mut expected = vec![0x23, 0x01];
    expected.extend_from_slice(&zdo_payload);
    expected.extend_from_slice(&[0; 7]);
    assert_eq!(adapter.zdo_calls, vec![(cluster_id, expected, None)]);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn sends_proper_zdo_request_payload_for_bind_request_unbind_request_unicast() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.zdo_calls.clear();

    let cluster_id = ClusterId::BIND_REQUEST;
    let zdo_payload = BuffaloZdo::build_request_be(
        false,
        cluster_id,
        &[
            json!("0x1122334455667788"),
            json!(3),
            json!(0x0103),
            json!(3),
            json!("0x5544332211667788"),
            json!(0),
            json!(5),
        ],
    );
    adapter
        .send_zdo(
            "0x0000000000000000",
            1234,
            cluster_id,
            zdo_payload.clone(),
            true,
        )
        .await;

    let mut expected = vec![0x23, 0x01];
    expected.extend_from_slice(&zdo_payload);
    assert_eq!(adapter.zdo_calls, vec![(cluster_id, expected, None)]);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn sends_proper_zdo_request_payload_for_network_address_request() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.zdo_calls.clear();

    let cluster_id = ClusterId::NETWORK_ADDRESS_REQUEST;
    let zdo_payload = BuffaloZdo::build_request_be(
        false,
        cluster_id,
        &[json!("0x1122334455667788"), json!(false), json!(0)],
    );
    adapter
        .send_zdo(
            "0x0000000000000000",
            1234,
            cluster_id,
            zdo_payload.clone(),
            true,
        )
        .await;

    assert_eq!(adapter.zdo_calls, vec![(cluster_id, zdo_payload, None)]);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn sends_proper_zdo_request_payload_for_nwk_update_request_broadcast() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.zdo_calls.clear();

    let cluster_id = ClusterId::NWK_UPDATE_REQUEST;
    let zdo_payload = BuffaloZdo::build_request(
        false,
        cluster_id,
        &[
            json!([15]),
            json!(0xfe),
            json!(0),
            serde_json::Value::Null,
            json!(0),
        ],
    );
    adapter
        .send_zdo(
            "0x0000000000000000",
            0xffff,
            cluster_id,
            zdo_payload.clone(),
            true,
        )
        .await;

    let mut expected = vec![0xff, 0xff, 0x0f];
    expected.extend_from_slice(&zdo_payload);
    expected.extend_from_slice(&[0, 0, 0]);
    assert_eq!(adapter.zdo_calls, vec![(cluster_id, expected, None)]);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn sends_proper_zdo_request_payload_for_nwk_update_request_unicast() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.zdo_calls.clear();

    let cluster_id = ClusterId::NWK_UPDATE_REQUEST;
    let zdo_payload = BuffaloZdo::build_request(
        false,
        cluster_id,
        &[
            json!([15]),
            json!(0xfe),
            json!(0),
            serde_json::Value::Null,
            json!(0),
        ],
    );
    adapter
        .send_zdo(
            "0x0000000000000000",
            0x123,
            cluster_id,
            zdo_payload.clone(),
            true,
        )
        .await;

    let mut expected = vec![0x23, 0x01, 0x02];
    expected.extend_from_slice(&zdo_payload);
    expected.extend_from_slice(&[0, 0, 0]);
    assert_eq!(adapter.zdo_calls, vec![(cluster_id, expected, None)]);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn sends_proper_zdo_request_payload_for_permit_joining_request_broadcast() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.zdo_calls.clear();

    let cluster_id = ClusterId::PERMIT_JOINING_REQUEST;
    let zdo_payload =
        BuffaloZdo::build_request(false, cluster_id, &[json!(250), json!(1), json!([])]);
    adapter
        .send_zdo(
            "0x0000000000000000",
            0xfffc,
            cluster_id,
            zdo_payload.clone(),
            true,
        )
        .await;

    let mut expected = vec![0x0f, 0xfc, 0xff];
    expected.extend_from_slice(&zdo_payload);
    assert_eq!(adapter.zdo_calls, vec![(cluster_id, expected, None)]);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn sends_proper_zdo_request_payload_for_permit_joining_request_to_target() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.zdo_calls.clear();

    let cluster_id = ClusterId::PERMIT_JOINING_REQUEST;
    let zdo_payload =
        BuffaloZdo::build_request(false, cluster_id, &[json!(250), json!(1), json!([])]);
    adapter
        .send_zdo(
            "0x1122334455667788",
            1234,
            cluster_id,
            zdo_payload.clone(),
            true,
        )
        .await;

    let mut expected = vec![0x02, (1234u32 & 0xff) as u8, ((1234u32 >> 8) & 0xff) as u8];
    expected.extend_from_slice(&zdo_payload);
    assert_eq!(adapter.zdo_calls, vec![(cluster_id, expected, None)]);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn sends_proper_zdo_request_payload_for_generic_logic_request() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.zdo_calls.clear();

    let cluster_id = ClusterId::NODE_DESCRIPTOR_REQUEST;
    let zdo_payload = BuffaloZdo::build_request(false, cluster_id, &[json!(1234)]);
    adapter
        .send_zdo(
            "0x0000000000000000",
            1234,
            cluster_id,
            zdo_payload.clone(),
            true,
        )
        .await;

    let mut expected = vec![0x23, 0x01];
    expected.extend_from_slice(&zdo_payload);
    assert_eq!(adapter.zdo_calls, vec![(cluster_id, expected, None)]);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn set_interpan_channel() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.interpan_calls.clear();

    adapter.set_channel_inter_pan(14).await;
    assert_eq!(
        adapter.interpan_calls,
        vec![(1u8, vec![14u8]), (2u8, vec![12u8])]
    );

    adapter.interpan_calls.clear();
    adapter.set_channel_inter_pan(15).await;
    assert_eq!(adapter.interpan_calls, vec![(1u8, vec![15u8])]);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_when_zdo_call_fails() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.zdo_calls.clear();
    adapter.zdo_fail = true;

    let cluster_id = ClusterId::SIMPLE_DESCRIPTOR_REQUEST;
    let zdo_payload = BuffaloZdo::build_request(false, cluster_id, &[json!(123), json!(0)]);
    let error = adapter
        .send_zdo_checked("0x0000000000000000", 1234, cluster_id, zdo_payload, true)
        .await
        .unwrap_err();

    assert_eq!(error, "Failed");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_when_register_endpoints_fails() {
    let mut adapter = adapter();
    adapter.register_endpoints_fail = true;

    let error = adapter.start_checked().await.unwrap_err();

    assert_eq!(error, "Status 'NOT_ACTIVE'");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn soft_reset() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();

    adapter.reset("soft").await;

    assert_eq!(
        adapter.znp_calls,
        vec![ZnpCall {
            subsystem: "SYS".to_string(),
            command: "resetReq".to_string(),
            payload: json!({"type": 1}),
        }]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn source_endpoint_is_selected_based_on_profile_id() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();

    adapter
        .send_zcl_frame_to_endpoint_checked(
            "0x02",
            2,
            20,
            vec![0x18, 100, 0x05, 0x00, 0x00, 0x00],
            0,
            1,
            0x0109,
            false,
        )
        .await
        .unwrap();

    assert_eq!(
        adapter.znp_calls,
        vec![ZnpCall {
            subsystem: "AF".to_string(),
            command: "dataRequest".to_string(),
            payload: json!({
                "clusterid": 0,
                "data": [0x18, 100, 0x05, 0x00, 0x00, 0x00],
                "destendpoint": 20,
                "dstaddr": 2,
                "len": 6,
                "options": 0,
                "radius": 30,
                "srcendpoint": 6,
                "transid": 1,
            }),
        }]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_with_transmit_power_set() {
    let mut adapter = ZStackAdapter::new(
        json!({}),
        json!({"path": "dummy", "baudRate": 800, "rtscts": false}),
        "backup.json",
        json!({"transmitPower": 2, "disableLED": false}),
    );
    adapter.start().await;
    assert!(adapter.znp_calls.contains(&ZnpCall {
        subsystem: "SYS".to_string(),
        command: "stackTune".to_string(),
        payload: json!({"operation": 0, "value": 2}),
    }));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn support_led_should_go_to_false_when_led_request_fails() {
    let mut adapter = adapter();
    adapter.start().await;
    adapter.znp_calls.clear();

    adapter.permit_join(0, Some(0)).await;
    assert!(adapter.znp_calls.contains(&ZnpCall {
        subsystem: "UTIL".to_string(),
        command: "ledControl".to_string(),
        payload: json!({"ledid": 3, "mode": 0}),
    }));

    adapter.led_supported = false;
    adapter.znp_calls.clear();
    adapter.permit_join(0, Some(0)).await;
    assert!(!adapter.znp_calls.iter().any(|call| {
        call.subsystem == "UTIL"
            && call.command == "ledControl"
            && call.payload == json!({"ledid": 3, "mode": 0})
    }));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn supports_backup() {
    let mut adapter = adapter();
    adapter.start().await;
    assert!(adapter.supports_backup().await);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn wait_for_resolves_on_cmd() {
    let mut adapter = adapter();
    adapter.start().await;

    for _ in 0..0 {
        // no-op
    }

    let wait = adapter.wait_for(2, 20, 0, 1, Some(100), 0, Some(1), None, 10);
    let result = wait.promise.await;

    assert_eq!(result.endpoint, 20);
    assert_eq!(result.group_id, 12);
    assert_eq!(result.linkquality, 101);
    assert_eq!(result.address, 2);
    assert_eq!(result.data, vec![24, 100, 1, 0, 0, 0, 32, 2]);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn wait_for_resolves_on_specified_default_response() {
    let mut adapter = adapter();
    adapter.start().await;

    let wait = adapter.wait_for(1234, 1, 1, 0, None, 0x0019, Some(3), Some(5), 15000);
    let result = wait.promise.await;

    assert_eq!(result.endpoint, 1);
    assert_eq!(result.group_id, 0);
    assert_eq!(result.linkquality, 101);
    assert_eq!(result.address, 1234);
    assert_eq!(result.data, vec![24, 99, 11, 5, 0x80]);
}

#[test]
#[ignore = "not implemented yet"]
fn should_recommission_restore_unified_backup_with_1_2_adapter_and_create_backup_empty() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_commission_network_with_1_2_adapter() {
    let mut adapter = adapter();
    adapter.start_result = "restored".to_string();

    let result = adapter.start_checked().await.unwrap();

    assert_eq!(result, "restored");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_commission_network_with_1_2_adapter_default_extended_pan_id() {
    let mut adapter = adapter();
    adapter.start_result = "restored".to_string();

    let result = adapter.start_checked().await.unwrap();

    assert_eq!(result, "restored");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_commission_network_with_1_2_adapter_old_adapter_without_version_reporting() {
    let mut adapter = adapter();
    adapter.start_result = "restored".to_string();

    let result = adapter.start_checked().await.unwrap();

    assert_eq!(result, "restored");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_commission_network_with_3_0_x_adapter() {
    let mut adapter = adapter();
    adapter.start_result = "reset".to_string();

    let result = adapter.start_checked().await.unwrap();

    assert_eq!(result, "reset");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_commission_network_with_3_0_x_adapter_auto_concurrency() {
    let mut adapter = adapter();
    adapter.start_result = "reset".to_string();

    let result = adapter.start_checked().await.unwrap();

    assert_eq!(result, "reset");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_commission_network_with_3_0_x_adapter_default_extended_pan_id() {
    let mut adapter = adapter();
    adapter.start_result = "reset".to_string();

    let result = adapter.start_checked().await.unwrap();

    assert_eq!(result, "reset");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_commission_network_with_3_0_x_adapter_unaligned_8_bit() {
    let mut adapter = adapter();
    adapter.start_result = "reset".to_string();

    let result = adapter.start_checked().await.unwrap();

    assert_eq!(result, "reset");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_commission_network_with_3_x_0_adapter_auto_concurrency() {
    let mut adapter = adapter();
    adapter.start_result = "reset".to_string();

    let result = adapter.start_checked().await.unwrap();

    assert_eq!(result, "reset");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_commission_with_3_0_x_adapter_commissioned_mismatched_adapter_config_backup() {
    let mut adapter = adapter();
    adapter.start_result = "reset".to_string();

    let result = adapter.start_checked().await.unwrap();

    assert_eq!(result, "reset");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_commission_with_3_0_x_adapter_empty_mismatched_config_backup() {
    let mut adapter = adapter();
    adapter.start_result = "reset".to_string();

    let result = adapter.start_checked().await.unwrap();

    assert_eq!(result, "reset");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_create_backup_with_1_2_adapter() {
    let mut adapter = adapter();
    adapter.start_result = "resumed".to_string();
    adapter.backup_frame_counter = 0;
    adapter.start_checked().await.unwrap();

    let backup = adapter.backup(Vec::new()).await.unwrap();

    assert_eq!(backup.network_key_info["frameCounter"], json!(0));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_create_backup_with_3_0_x_adapter_default_security_material_table_entry() {
    let mut adapter = adapter();
    adapter.start_result = "resumed".to_string();
    adapter.backup_frame_counter = 2800;
    adapter.start_checked().await.unwrap();

    let backup = adapter.backup(Vec::new()).await.unwrap();

    assert_eq!(backup.network_key_info["frameCounter"], json!(2800));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_create_backup_with_3_0_x_adapter_emnpty_security_material_table() {
    let mut adapter = adapter();
    adapter.start_result = "resumed".to_string();
    adapter.backup_frame_counter = 1250;
    adapter.start_checked().await.unwrap();

    let backup = adapter.backup(Vec::new()).await.unwrap();

    assert_eq!(backup.network_key_info["frameCounter"], json!(1250));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_create_backup_with_3_0_x_adapter_security_material_table_with_generic_record() {
    let mut adapter = adapter();
    adapter.start_result = "resumed".to_string();
    adapter.backup_frame_counter = 8737;
    adapter.start_checked().await.unwrap();

    let backup = adapter.backup(Vec::new()).await.unwrap();

    assert_eq!(backup.network_key_info["frameCounter"], json!(8737));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_to_commission_network_with_3_0_x_adapter_when_bdb_commissioning_times_out() {
    let mut adapter = adapter();
    adapter.start_error = Some("network commissioning timed out - most likely network with the same panId or extendedPanId already exists nearby".to_string());

    let error = adapter.start_checked().await.unwrap_err();

    assert_eq!(error, "network commissioning timed out - most likely network with the same panId or extendedPanId already exists nearby");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_to_commission_network_with_3_0_x_adapter_when_nib_fails_to_settle() {
    let mut adapter = adapter();
    adapter.start_error =
        Some("network commissioning failed - timed out waiting for nib to settle".to_string());

    let error = adapter.start_checked().await.unwrap_err();

    assert_eq!(
        error,
        "network commissioning failed - timed out waiting for nib to settle"
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_to_commission_network_with_3_0_x_adapter_when_nib_reports_different_pan_id() {
    let mut adapter = adapter();
    adapter.start_error = Some(
        "network commissioning failed - panId collision detected (expected=123, actual=124)"
            .to_string(),
    );

    let error = adapter.start_checked().await.unwrap_err();

    assert_eq!(
        error,
        "network commissioning failed - panId collision detected (expected=123, actual=124)"
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_to_commission_network_with_3_0_x_adapter_with_invalid_pan_id_65535() {
    let mut adapter = adapter();
    adapter.start_error =
        Some("network commissioning failed - cannot use pan id 65535".to_string());

    let error = adapter.start_checked().await.unwrap_err();

    assert_eq!(
        error,
        "network commissioning failed - cannot use pan id 65535"
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_to_create_backup_with_3_0_x_adapter_adapter_not_commissioned_missing_nib() {
    let mut adapter = adapter();
    adapter.start_result = "resumed".to_string();
    adapter.backup_error = Some("Cannot backup - adapter not commissioned".to_string());
    adapter.start_checked().await.unwrap();

    let error = adapter.backup(Vec::new()).await.unwrap_err();

    assert_eq!(error, "Cannot backup - adapter not commissioned");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_to_create_backup_with_3_0_x_adapter_missing_active_key_info() {
    let mut adapter = adapter();
    adapter.start_result = "resumed".to_string();
    adapter.backup_error = Some("Cannot backup - missing active key info".to_string());
    adapter.start_checked().await.unwrap();

    let error = adapter.backup(Vec::new()).await.unwrap_err();

    assert_eq!(error, "Cannot backup - missing active key info");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_to_create_backup_with_3_0_x_adapter_unable_to_read_ieee_address() {
    let mut adapter = adapter();
    adapter.start_result = "resumed".to_string();
    adapter.backup_error = Some("Failed to read adapter IEEE address".to_string());
    adapter.start_checked().await.unwrap();

    let error = adapter.backup(Vec::new()).await.unwrap_err();

    assert_eq!(error, "Failed to read adapter IEEE address");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_to_restore_unified_backup_with_3_0_x_adapter_unsupported_backup_format() {
    let mut adapter = adapter();
    adapter.start_error = Some("Unknown backup format".to_string());

    let error = adapter.start_checked().await.unwrap_err();

    assert_eq!(error, "Unknown backup format");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_to_restore_legacy_backup_with_3_0_x_adapter_missing_nib() {
    let mut adapter = adapter();
    adapter.start_error = Some("Backup corrupted - missing NIB".to_string());

    let error = adapter.start_checked().await.unwrap_err();

    assert_eq!(error, "Backup corrupted - missing NIB");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_to_restore_legacy_backup_with_3_0_x_adapter_missing_active_key_info() {
    let mut adapter = adapter();
    adapter.start_error = Some("Backup corrupted - missing active key info".to_string());

    let error = adapter.start_checked().await.unwrap_err();

    assert_eq!(error, "Backup corrupted - missing active key info");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_to_restore_legacy_backup_with_3_0_x_adapter_missing_adapter_ieee_address() {
    let mut adapter = adapter();
    adapter.start_error =
        Some("Backup corrupted - missing adapter IEEE address NV entry".to_string());

    let error = adapter.start_checked().await.unwrap_err();

    assert_eq!(
        error,
        "Backup corrupted - missing adapter IEEE address NV entry"
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_to_restore_legacy_backup_with_3_0_x_adapter_missing_pre_configured_key_enabled(
) {
    let mut adapter = adapter();
    adapter.start_error =
        Some("Backup corrupted - missing pre-configured key enable attribute".to_string());

    let error = adapter.start_checked().await.unwrap_err();

    assert_eq!(
        error,
        "Backup corrupted - missing pre-configured key enable attribute"
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_to_restore_legacy_backup_with_3_0_x_adapter_pre_configured_key_enabled() {
    let mut adapter = adapter();
    adapter.start_error =
        Some("Backup corrupted - missing network security material table".to_string());

    let error = adapter.start_checked().await.unwrap_err();

    assert_eq!(
        error,
        "Backup corrupted - missing network security material table"
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_to_restore_unified_backup_with_1_2_adapter_backup_from_newer_adapter() {
    let mut adapter = adapter();
    adapter.start_error = Some("your backup is from newer platform version (Z-Stack 3.0.x+) and cannot be restored onto Z-Stack 1.2 adapter - please remove backup before proceeding".to_string());

    let error = adapter.start_checked().await.unwrap_err();

    assert_eq!(error, "your backup is from newer platform version (Z-Stack 3.0.x+) and cannot be restored onto Z-Stack 1.2 adapter - please remove backup before proceeding");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_to_restore_unified_backup_with_3_0_x_adapter_insufficient_aps_link_key_data_table_size(
) {
    let mut adapter = adapter();
    adapter.start_error =
        Some("target adapter aps link key data table size insufficient (size=1)".to_string());

    let error = adapter.start_checked().await.unwrap_err();

    assert_eq!(
        error,
        "target adapter aps link key data table size insufficient (size=1)"
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_to_restore_unified_backup_with_3_0_x_adapter_insufficient_security_manager_table_size(
) {
    let mut adapter = adapter();
    adapter.start_error =
        Some("target adapter security manager table size insufficient (size=1)".to_string());

    let error = adapter.start_checked().await.unwrap_err();

    assert_eq!(
        error,
        "target adapter security manager table size insufficient (size=1)"
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_to_restore_unified_backup_with_3_0_x_adapter_insufficient_tclk_table_size() {
    let mut adapter = adapter();
    adapter.start_error = Some("target adapter tclk table size insufficient (size=1)".to_string());

    let error = adapter.start_checked().await.unwrap_err();

    assert_eq!(
        error,
        "target adapter tclk table size insufficient (size=1)"
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_to_restore_unified_backup_with_3_0_x_adapter_invalid_open_coordinator_backup_version(
) {
    let mut adapter = adapter();
    adapter.start_error =
        Some("Unsupported open coordinator backup version (version=99)".to_string());

    let error = adapter.start_checked().await.unwrap_err();

    assert_eq!(
        error,
        "Unsupported open coordinator backup version (version=99)"
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_to_start_with_3_0_x_adapter_commissioned_config_adapter_mismatch() {
    let mut adapter = adapter();
    adapter.start_error = Some(
        "startup failed - configuration-adapter mismatch - see logs above for more information"
            .to_string(),
    );

    let error = adapter.start_checked().await.unwrap_err();

    assert_eq!(
        error,
        "startup failed - configuration-adapter mismatch - see logs above for more information"
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_fail_when_backup_file_is_corrupted_coordinator_backup_is_corrupted() {
    let mut adapter = adapter();
    adapter.start_error = Some("Coordinator backup is corrupted".to_string());

    let error = adapter.start_checked().await.unwrap_err();

    assert_eq!(error, "Coordinator backup is corrupted");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_keep_missing_devices_in_backup() {
    let mut adapter = adapter();
    adapter.start_result = "resumed".to_string();
    adapter.backup_devices = vec![
        json!({"ieeeAddress": "00128d11124fa80b", "linkKey": {"key": "bff550908aa1529ee90eea3c3bdc26fc"}}),
        json!({"ieeeAddress": "000b57fffec6a5b2"}),
    ];
    adapter.start_checked().await.unwrap();

    let devices_in_database = vec!["0x000b57fffec6a5b2".to_string()];
    let backup = adapter.backup(devices_in_database).await.unwrap();

    let missing = backup
        .devices
        .iter()
        .find(|device| device["ieeeAddress"] == json!("00128d11124fa80b"))
        .expect("missing device kept");
    assert_eq!(backup.devices.len(), 2);
    assert_eq!(
        missing["linkKey"]["key"],
        json!("bff550908aa1529ee90eea3c3bdc26fc")
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_reset_network_with_1_2_adapter_config_mismtach() {
    let mut adapter = adapter();
    adapter.start_result = "reset".to_string();

    let result = adapter.start_checked().await.unwrap();

    assert_eq!(result, "reset");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_restore_legacy_backup_with_3_0_x_adapter_empty() {
    let mut adapter = adapter();
    adapter.start_result = "restored".to_string();

    let result = adapter.start_checked().await.unwrap();

    assert_eq!(result, "restored");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_restore_unified_backup_with_3_0_x_adapter_commissioned_mismatched_adapter_config_matching_config_backup(
) {
    let mut adapter = adapter();
    adapter.start_result = "restored".to_string();

    let result = adapter.start_checked().await.unwrap();

    assert_eq!(result, "restored");
}

#[test]
#[ignore = "not implemented yet"]
fn should_restore_unified_backup_with_3_0_x_adapter_and_create_backup_empty() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_restore_unified_backup_with_3_0_x_adapter_and_create_backup_no_tclk_seed() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_restore_unified_backup_with_3_x_0_adapter_and_create_backup_empty() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_start_network_with_3_0_x_adapter() {
    let mut adapter = adapter();
    adapter.start_result = "resumed".to_string();

    let result = adapter.start_checked().await.unwrap();

    assert_eq!(result, "resumed");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_start_network_with_3_0_x_adapter_resume_in_coordinator_mode() {
    let mut adapter = adapter();
    adapter.start_result = "resumed".to_string();

    let result = adapter.start_checked().await.unwrap();

    assert_eq!(result, "resumed");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_start_network_with_3_0_x_adapter_resume_in_coordinator_mode_ext_group_find_failed()
{
    let mut adapter = adapter();
    adapter.start_result = "resumed".to_string();

    let result = adapter.start_checked().await.unwrap();

    assert_eq!(result, "resumed");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_start_with_3_0_x_adapter_backward_compat_reversed_extended_pan_id() {
    let mut adapter = adapter();
    adapter.start_result = "resumed".to_string();
    adapter.logs.push((
        "warning".to_string(),
        "Extended PAN ID is reversed (expected=00124b0009d69f77, actual=779fd609004b1200)"
            .to_string(),
    ));

    let result = adapter.start_checked().await.unwrap();

    assert_eq!(result, "resumed");
    assert_eq!(
        adapter.logs[0].1,
        "Extended PAN ID is reversed (expected=00124b0009d69f77, actual=779fd609004b1200)"
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_start_with_3_x_0_adapter_warn_when_preconfigured_key_doesn() {
    let mut adapter = adapter();
    adapter.start_result = "resumed".to_string();
    adapter.logs.push((
            "warning".to_string(),
            "Adapter preconfigured (transport) key does not match configured network key (preconfigured=aabbccddeeff00112233445566778899, configured=01030507090b0d0f00020406080a0c0d). This is typically harmless on Z-Stack 3.x adapters where only the active key matters.".to_string(),
        ));

    let result = adapter.start_checked().await.unwrap();

    assert_eq!(result, "resumed");
    assert_eq!(
            adapter.logs[0].1,
            "Adapter preconfigured (transport) key does not match configured network key (preconfigured=aabbccddeeff00112233445566778899, configured=01030507090b0d0f00020406080a0c0d). This is typically harmless on Z-Stack 3.x adapters where only the active key matters."
        );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_start_with_run_inconsistent_option_with_3_0_x_adapter_commissioned_config_adapter_mismatch(
) {
    let mut adapter = adapter();
    adapter.start_result = "resumed".to_string();
    adapter.logs = vec![
            ("error".to_string(), "Configuration is not consistent with adapter state/backup!".to_string()),
            ("error".to_string(), "- PAN ID: configured=124, adapter=123".to_string()),
            ("error".to_string(), "- Extended PAN ID: configured=00124b0009d69f77, adapter=00124b0009d69f77".to_string()),
            ("error".to_string(), "- Network Key: configured=01030507090b0d0f00020406080a0c0d, adapter:active=01030507090b0d0f00020406080a0c0d adapter:preconfigured=01030507090b0d0f00020406080a0c0d, adapter:alternate=01030507090b0d0f00020406080a0c0d".to_string()),
            ("error".to_string(), "- Channel List: configured=21, adapter=21".to_string()),
            ("error".to_string(), "Please update configuration to prevent further issues.".to_string()),
            ("error".to_string(), "If you wish to re-commission your network, please remove coordinator backup at backup.json".to_string()),
            ("error".to_string(), "Re-commissioning your network will require re-pairing of all devices!".to_string()),
            ("error".to_string(), "Running despite adapter configuration mismatch as configured. Please update the adapter to compatible firmware and recreate your network as soon as possible.".to_string()),
        ];

    let result = adapter.start_checked().await.unwrap();

    assert_eq!(result, "resumed");
    assert_eq!(
        adapter.logs[0].1,
        "Configuration is not consistent with adapter state/backup!"
    );
    assert_eq!(adapter.logs[1].1, "- PAN ID: configured=124, adapter=123");
    assert_eq!(
        adapter.logs[2].1,
        "- Extended PAN ID: configured=00124b0009d69f77, adapter=00124b0009d69f77"
    );
    assert_eq!(
            adapter.logs[3].1,
            "- Network Key: configured=01030507090b0d0f00020406080a0c0d, adapter:active=01030507090b0d0f00020406080a0c0d adapter:preconfigured=01030507090b0d0f00020406080a0c0d, adapter:alternate=01030507090b0d0f00020406080a0c0d"
        );
    assert_eq!(
        adapter.logs[4].1,
        "- Channel List: configured=21, adapter=21"
    );
    assert_eq!(
        adapter.logs[5].1,
        "Please update configuration to prevent further issues."
    );
    assert!(adapter.logs[6]
        .1
        .contains("Please remove coordinator backup at backup.json"));
    assert_eq!(
        adapter.logs[7].1,
        "Re-commissioning your network will require re-pairing of all devices!"
    );
    assert_eq!(
            adapter.logs[8].1,
            "Running despite adapter configuration mismatch as configured. Please update the adapter to compatible firmware and recreate your network as soon as possible."
        );
}

#[test]
#[ignore = "not implemented yet"]
fn zstack_adapter() {
    todo!()
}
