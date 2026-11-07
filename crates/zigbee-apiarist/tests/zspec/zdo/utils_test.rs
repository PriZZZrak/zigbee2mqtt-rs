// Original: .upstream/zigbee-herdsman/test/zspec/zdo/utils.test.ts

use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;
use zigbee_apiarist::zspec::zdo::definition::status::Status;
use zigbee_apiarist::zspec::zdo::utils;
use zigbee_apiarist::zspec::zdo::utils::{MacCapabilities, ServerMask};
use zigbee_apiarist::zspec::zdo::zdo_status_error::ZdoStatusError;

#[test]
fn creates_status_error() {
    let zdo_error = ZdoStatusError::new(Status::INVALID_INDEX);

    assert_eq!(zdo_error.code, Status::INVALID_INDEX);
    assert_eq!(zdo_error.message(), "Status 'INVALID_INDEX'");
}

#[test]
fn gets_response_cluster_id_for_request() {
    let cases = [
        (
            ClusterId::ACTIVE_ENDPOINTS_REQUEST,
            Some(ClusterId::ACTIVE_ENDPOINTS_RESPONSE),
        ),
        (
            ClusterId::BINDING_TABLE_REQUEST,
            Some(ClusterId::BINDING_TABLE_RESPONSE),
        ),
        (
            ClusterId::CHALLENGE_REQUEST,
            Some(ClusterId::CHALLENGE_RESPONSE),
        ),
        (
            ClusterId::NODE_DESCRIPTOR_REQUEST,
            Some(ClusterId::NODE_DESCRIPTOR_RESPONSE),
        ),
        (
            ClusterId::NETWORK_ADDRESS_REQUEST,
            Some(ClusterId::NETWORK_ADDRESS_RESPONSE),
        ),
        (ClusterId::END_DEVICE_ANNOUNCE, None), // not a request
        (ClusterId::NWK_UNSOLICITED_ENHANCED_UPDATE_RESPONSE, None),
        (ClusterId::ACTIVE_ENDPOINTS_RESPONSE, None),
        (ClusterId::CHALLENGE_RESPONSE, None),
        (0x7999, None),
    ];

    for (request, response) in cases {
        assert_eq!(utils::get_response_cluster_id(request), response);
    }
}

fn capabilities(
    alternate_pan_coordinator: u8,
    device_type: u8,
    power_source: u8,
    rx_on_when_idle: u8,
    reserved1: u8,
    reserved2: u8,
    security_capability: u8,
    allocate_address: u8,
) -> MacCapabilities {
    MacCapabilities {
        alternate_pan_coordinator,
        device_type,
        power_source,
        rx_on_when_idle,
        reserved1,
        reserved2,
        security_capability,
        allocate_address,
    }
}

#[test]
fn gets_mac_capabilities_flags() {
    let cases: [(u8, MacCapabilities); 11] = [
        (0, capabilities(0, 0, 0, 0, 0, 0, 0, 0)),
        (0b00000001, capabilities(1, 0, 0, 0, 0, 0, 0, 0)),
        (0b00000010, capabilities(0, 1, 0, 0, 0, 0, 0, 0)),
        (0b00000100, capabilities(0, 0, 1, 0, 0, 0, 0, 0)),
        (0b00001000, capabilities(0, 0, 0, 1, 0, 0, 0, 0)),
        (0b00010000, capabilities(0, 0, 0, 0, 1, 0, 0, 0)),
        (0b00100000, capabilities(0, 0, 0, 0, 0, 1, 0, 0)),
        (0b01000000, capabilities(0, 0, 0, 0, 0, 0, 1, 0)),
        (0b10000000, capabilities(0, 0, 0, 0, 0, 0, 0, 1)),
        (0b10101011, capabilities(1, 1, 0, 1, 0, 1, 0, 1)),
        (0b11111111, capabilities(1, 1, 1, 1, 1, 1, 1, 1)),
    ];

    for (value, expected) in cases {
        assert_eq!(utils::get_mac_cap_flags(value), expected);
    }
}

fn server_mask(
    primary_trust_center: u16,
    backup_trust_center: u16,
    deprecated1: u16,
    deprecated2: u16,
    deprecated3: u16,
    deprecated4: u16,
    network_manager: u16,
    reserved1: u16,
    reserved2: u16,
    stack_compliance_revision: u16,
) -> ServerMask {
    ServerMask {
        primary_trust_center,
        backup_trust_center,
        deprecated1,
        deprecated2,
        deprecated3,
        deprecated4,
        network_manager,
        reserved1,
        reserved2,
        stack_compliance_revision,
    }
}

#[test]
fn gets_and_creates_server_mask() {
    let cases: [(u16, ServerMask); 4] = [
        (0, server_mask(0, 0, 0, 0, 0, 0, 0, 0, 0, 0)),
        (
            0b0000000000000001,
            server_mask(1, 0, 0, 0, 0, 0, 0, 0, 0, 0),
        ),
        (
            0b0010111001000001,
            server_mask(1, 0, 0, 0, 0, 0, 1, 0, 0, 23),
        ),
        (
            0b0010101001000010,
            server_mask(0, 1, 0, 0, 0, 0, 1, 0, 0, 21),
        ),
    ];

    for (value, expected) in cases {
        assert_eq!(utils::get_server_mask(value), expected);
        assert_eq!(utils::create_server_mask(expected), value);
    }
}
