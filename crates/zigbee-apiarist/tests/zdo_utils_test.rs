// Port of .upstream/zigbee-herdsman/test/zspec/zdo/utils.test.ts

use zigbee_apiarist::zspec::zdo::definition::status::Status;
use zigbee_apiarist::zspec::zdo::utils;
use zigbee_apiarist::zspec::zdo::zdo_status_error::ZdoStatusError;

#[test]
#[ignore = "not implemented yet"]
fn creates_status_error() {
    let zdo_error = ZdoStatusError::new(Status::INVALID_INDEX);

    assert_eq!(zdo_error.code, Status::INVALID_INDEX);
    assert_eq!(zdo_error.message(), "Status 'INVALID_INDEX'");
}

#[test]
#[ignore = "not implemented yet"]
fn gets_response_cluster_id_for_request() {
    // ZDO cluster IDs from the pinned upstream definition.
    let cases: [(u16, Option<u16>); 10] = [
        (0x0005, Some(0x8005)), // ACTIVE_ENDPOINTS_REQUEST -> RESPONSE
        (0x0033, Some(0x8033)), // BINDING_TABLE_REQUEST -> RESPONSE
        (0x0047, Some(0x8047)), // CHALLENGE_REQUEST -> RESPONSE
        (0x0002, Some(0x8002)), // NODE_DESCRIPTOR_REQUEST -> RESPONSE
        (0x0000, Some(0x8000)), // NETWORK_ADDRESS_REQUEST -> RESPONSE
        (0x0013, None),         // END_DEVICE_ANNOUNCE is not a request
        (0x803b, None),         // NWK_UNSOLICITED_ENHANCED_UPDATE_RESPONSE
        (0x8005, None),         // ACTIVE_ENDPOINTS_RESPONSE is not a request
        (0x8047, None),         // CHALLENGE_RESPONSE is not a request
        (0x7999, None),
    ];

    for (request, response) in cases {
        assert_eq!(utils::get_response_cluster_id(request), response, "request=0x{request:04x}");
    }
}
