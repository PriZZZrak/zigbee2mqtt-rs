// Original: .upstream/zigbee-herdsman/test/zspec/zdo/buffalo.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Checks if more available to by amount
//   - Ignores invalid TLV tag and reads next TLV
//   - Sets & Gets bytes without changing internal position
//   - Sets & Gets position
//   - Throws when buildBindRequest/buildUnbindRequest invalid type
//   - Throws when building unknown cluster ID
//   - Throws when duplicate TLV tag found and not valid
//   - Throws when encapsulated TLV tag found inside encapsulated
//   - Throws when not enough bytes to read in TLV
//   - Throws when reading unknown cluster ID
//   - Throws when writing invalid TLV tag
//   - ZDO Buffalo
//   - buildActiveEndpointsRequest
//   - buildBindRequest
//   - buildBindingTableRequest
//   - buildChallengeRequest
//   - buildChannelChangeRequest
//   - buildClearAllBindingsRequest
//   - buildDecommissionRequest
//   - buildEnhancedChannelChangeRequest
//   - buildEnhancedScanChannelsRequest
//   - buildEnhancedSetActiveChannelsAndNwkManagerIdRequest
//   - buildGetAuthenticationLevelRequest
//   - buildGetConfigurationRequest
//   - buildIeeeAddressRequest
//   - buildLeaveRequest
//   - buildLqiTableRequest
//   - buildMatchDescriptorRequest
//   - buildNetworkAddressRequest
//   - buildNetworkAddressRequest without ZDO_MESSAGE_OVERHEAD
//   - buildNodeDescriptorRequest
//   - buildNwkBeaconSurveyRequest
//   - buildNwkIEEEJoiningListRequest
//   - buildParentAnnounce
//   - buildPermitJoining
//   - buildPowerDescriptorRequest
//   - buildRetrieveAuthenticationTokenRequest
//   - buildRoutingTableRequest
//   - buildScanChannelsRequest
//   - buildSetActiveChannelsAndNwkManagerIdRequest
//   - buildSimpleDescriptorRequest
//   - buildStartKeyNegotiationRequest
//   - buildStartKeyUpdateRequest
//   - buildSystemServiceDiscoveryRequest
//   - buildUnbindRequest
//   - readActiveEndpointsResponse
//   - readBindingTableResponse
//   - readChallengeResponse
//   - readEndDeviceAnnounce
//   - readGetAuthenticationLevelResponse
//   - readGetConfigurationResponse
//   - readIEEEAddressResponse
//   - readLQITableResponse
//   - readMatchDescriptorsResponse
//   - readNetworkAddressResponse
//   - readNodeDescriptorResponse
//   - readNwkBeaconSurveyResponse
//   - readNwkEnhancedUpdateResponse
//   - readNwkIEEEJoiningListResponse
//   - readNwkUnsolicitedEnhancedUpdateResponse
//   - readNwkUpdateResponse
//   - readParentAnnounceResponse
//   - readPowerDescriptorResponse
//   - readRetrieveAuthenticationTokenResponse
//   - readRoutingTableResponse
//   - readSetConfigurationResponse
//   - readSimpleDescriptorResponse
//   - readStartKeyNegotiationResponse
//   - readSystemServerDiscoveryResponse

use serde_json::json;
use zigbee_apiarist::zspec::consts::{
    ALL_802_15_4_CHANNELS, ALL_802_15_4_CHANNELS_MASK, PREFERRED_802_15_4_CHANNELS,
    PREFERRED_802_15_4_CHANNELS_MASK,
};
use zigbee_apiarist::zspec::zdo::buffalo_zdo::BuffaloZdo;
use zigbee_apiarist::zspec::zdo::definition::clusters::ClusterId;

const IEEE_ADDRESS1: &str = "0xfe34ac2385ff8311";
const IEEE_ADDRESS1_BYTES: [u8; 8] = [0x11, 0x83, 0xff, 0x85, 0x23, 0xac, 0x34, 0xfe];
const IEEE_ADDRESS2: &str = "0x28373fecd834ba37";
const IEEE_ADDRESS2_BYTES: [u8; 8] = [0x37, 0xba, 0x34, 0xd8, 0xec, 0x3f, 0x37, 0x28];
const NODE_ID1: u16 = 0xfe32;
const NODE_ID1_BYTES: [u8; 2] = [0x32, 0xfe];
const CLUSTER_LIST1_BYTES: [u8; 6] = [0x09, 0x00, 0x02, 0x07, 0x03, 0x0b];
const CLUSTER_LIST2_BYTES: [u8; 8] = [0x06, 0x00, 0x00, 0x00, 0x00, 0x05, 0x08, 0x00];
const HA_PROFILE_ID: u16 = 0x0104;
const UNICAST_BINDING: u8 = 0x03;
const MULTICAST_BINDING: u8 = 0x01;
const LEAVE_WITHOUT_REJOIN: u8 = 0x00;
const LEAVE_AND_REJOIN: u8 = 0x80;
const SE_METERING_ID: u16 = 0x0702;

#[test]
#[ignore = "not implemented yet"]
fn checks_if_more_available_to_by_amount() {
    let mut buffalo = BuffaloZdo::new(vec![1, 2, 3, 255]);
    buffalo.set_position(4);
    assert!(buffalo.is_more_by(0));
    assert!(!buffalo.is_more_by(1));
    buffalo.set_position(1);
    assert!(buffalo.is_more_by(0));
    assert!(buffalo.is_more_by(3));
    assert!(!buffalo.is_more_by(4));
}

#[test]
#[ignore = "not implemented yet"]
fn ignores_invalid_tlv_tag_and_reads_next_tlv() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn sets_gets_bytes_without_changing_internal_position() {
    let mut buffalo = BuffaloZdo::new(vec![1, 2, 3, 255]);
    assert_eq!(buffalo.get_byte(0), 1);
    assert_eq!(buffalo.get_byte(1), 2);
    assert_eq!(buffalo.get_byte(2), 3);
    assert_eq!(buffalo.get_byte(3), 255);
    assert_eq!(buffalo.get_position(), 0);
    buffalo.set_byte(3, 127);
    assert_eq!(buffalo.get_byte(3), 127);
    buffalo.set_byte(0, 7);
    assert_eq!(buffalo.get_byte(0), 7);
}

#[test]
#[ignore = "not implemented yet"]
fn sets_gets_position() {
    let mut buffalo = BuffaloZdo::new(vec![0; 3]);
    assert_eq!(buffalo.get_position(), 0);
    buffalo.set_position(3);
    assert_eq!(buffalo.get_position(), 3);
    buffalo.set_position(1);
    assert_eq!(buffalo.get_position(), 1);
    buffalo.set_position(0);
    assert_eq!(buffalo.get_position(), 0);
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_build_bind_request_build_unbind_request_invalid_type() {
    let bind = std::panic::catch_unwind(|| {
        BuffaloZdo::build_request(
            true,
            ClusterId::BIND_REQUEST,
            &[
                json!(IEEE_ADDRESS1),
                json!(2),
                json!(SE_METERING_ID),
                json!(99),
                json!(IEEE_ADDRESS2),
                json!(123),
                json!(64),
            ],
        );
    });
    assert!(bind.is_err());

    let unbind = std::panic::catch_unwind(|| {
        BuffaloZdo::build_request(
            true,
            ClusterId::UNBIND_REQUEST,
            &[
                json!(IEEE_ADDRESS1),
                json!(2),
                json!(SE_METERING_ID),
                json!(99),
                json!(IEEE_ADDRESS2),
                json!(123),
                json!(64),
            ],
        );
    });
    assert!(unbind.is_err());
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_building_unknown_cluster_id() {
    let cluster_id = 0x8005u16;
    let result = std::panic::catch_unwind(|| {
        BuffaloZdo::build_request(true, cluster_id, &[json!([1, 123])]);
    });
    assert!(result.is_err());
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_duplicate_tlv_tag_found_and_not_valid() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_encapsulated_tlv_tag_found_inside_encapsulated() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_not_enough_bytes_to_read_in_tlv() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_reading_unknown_cluster_id() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_writing_invalid_tlv_tag() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn build_active_endpoints_request() {
    let mut expected = vec![0];
    expected.extend_from_slice(&NODE_ID1_BYTES);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::ACTIVE_ENDPOINTS_REQUEST,
            &[json!(NODE_ID1)],
        ),
        expected
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_bind_request() {
    let mut expected = vec![0];
    expected.extend_from_slice(&IEEE_ADDRESS1_BYTES);
    expected.push(2);
    expected.extend_from_slice(&SE_METERING_ID.to_le_bytes());
    expected.push(UNICAST_BINDING);
    expected.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    expected.push(64);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::BIND_REQUEST,
            &[
                json!(IEEE_ADDRESS1),
                json!(2),
                json!(SE_METERING_ID),
                json!(UNICAST_BINDING),
                json!(IEEE_ADDRESS2),
                json!(123),
                json!(64),
            ],
        ),
        expected
    );

    let mut expected = vec![0];
    expected.extend_from_slice(&IEEE_ADDRESS1_BYTES);
    expected.push(3);
    expected.extend_from_slice(&SE_METERING_ID.to_le_bytes());
    expected.push(MULTICAST_BINDING);
    expected.extend_from_slice(&123u16.to_le_bytes());
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::BIND_REQUEST,
            &[
                json!(IEEE_ADDRESS1),
                json!(3),
                json!(SE_METERING_ID),
                json!(MULTICAST_BINDING),
                json!(IEEE_ADDRESS2),
                json!(123),
                json!(64),
            ],
        ),
        expected
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_binding_table_request() {
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::BINDING_TABLE_REQUEST, &[json!(1)]),
        vec![0, 1]
    );
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::BINDING_TABLE_REQUEST, &[json!(254)]),
        vec![0, 254]
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_challenge_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn build_channel_change_request() {
    let mut expected = vec![0];
    expected.extend_from_slice(&(1u32 << 15).to_le_bytes());
    expected.extend_from_slice(&[0xfe, 1]);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::NWK_UPDATE_REQUEST,
            &[
                json!([15]),
                json!(0xfe),
                serde_json::Value::Null,
                json!(1),
                serde_json::Value::Null,
            ],
        ),
        expected
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_clear_all_bindings_request() {
    let eui64_list = [IEEE_ADDRESS1, IEEE_ADDRESS2];
    let mut expected = vec![
        0,
        0,
        8 * eui64_list.len() as u8 + 1 - 1,
        eui64_list.len() as u8,
    ];
    expected.extend_from_slice(&IEEE_ADDRESS1_BYTES);
    expected.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::CLEAR_ALL_BINDINGS_REQUEST,
            &[json!({"eui64List": eui64_list})],
        ),
        expected
    );

    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::CLEAR_ALL_BINDINGS_REQUEST,
            &[json!({"eui64List": []})],
        ),
        vec![0, 0, 0, 0]
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_decommission_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn build_enhanced_channel_change_request() {
    let channel_page = 54394u32;
    let mut expected = vec![0, 1];
    expected.extend_from_slice(&channel_page.to_le_bytes());
    expected.extend_from_slice(&[0xfe, 3, 1]);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::NWK_ENHANCED_UPDATE_REQUEST,
            &[
                json!([channel_page]),
                json!(0xfe),
                serde_json::Value::Null,
                json!(3),
                serde_json::Value::Null,
                json!(1),
            ],
        ),
        expected
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_enhanced_scan_channels_request() {
    let channel_pages = [123u32, 54394, 29344];
    let mut expected = vec![0, channel_pages.len() as u8];
    for page in channel_pages {
        expected.extend_from_slice(&page.to_le_bytes());
    }
    expected.extend_from_slice(&[5, 3, 1]);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::NWK_ENHANCED_UPDATE_REQUEST,
            &[
                json!(channel_pages.to_vec()),
                json!(5),
                json!(3),
                serde_json::Value::Null,
                serde_json::Value::Null,
                json!(1),
            ],
        ),
        expected
    );

    let mut expected = vec![0, channel_pages.len() as u8];
    for page in channel_pages {
        expected.extend_from_slice(&page.to_le_bytes());
    }
    expected.extend_from_slice(&[6, 1]);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::NWK_ENHANCED_UPDATE_REQUEST,
            &[
                json!(channel_pages.to_vec()),
                json!(6),
                json!(3),
                serde_json::Value::Null,
                serde_json::Value::Null,
                json!(1),
            ],
        ),
        expected
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_enhanced_set_active_channels_and_nwk_manager_id_request() {
    let channel_pages = [123u32, 54394, 29344];
    let nwk_manager_addr = 0xfe01u16;
    let mut expected = vec![0, channel_pages.len() as u8];
    for page in channel_pages {
        expected.extend_from_slice(&page.to_le_bytes());
    }
    expected.push(0xff);
    expected.push(2);
    expected.extend_from_slice(&nwk_manager_addr.to_le_bytes());
    expected.push(1);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::NWK_ENHANCED_UPDATE_REQUEST,
            &[
                json!(channel_pages.to_vec()),
                json!(0xff),
                serde_json::Value::Null,
                json!(2),
                json!(nwk_manager_addr),
                json!(1),
            ],
        ),
        expected
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_get_authentication_level_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn build_get_configuration_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn build_ieee_address_request() {
    let mut expected = vec![0];
    expected.extend_from_slice(&NODE_ID1_BYTES);
    expected.extend_from_slice(&[0, 1]);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::IEEE_ADDRESS_REQUEST,
            &[json!(NODE_ID1), json!(false), json!(1)],
        ),
        expected
    );

    let mut expected = vec![0];
    expected.extend_from_slice(&NODE_ID1_BYTES);
    expected.extend_from_slice(&[1, 3]);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::IEEE_ADDRESS_REQUEST,
            &[json!(NODE_ID1), json!(true), json!(3)],
        ),
        expected
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_leave_request() {
    let without_rejoin = {
        let mut expected = vec![0];
        expected.extend_from_slice(&IEEE_ADDRESS2_BYTES);
        expected.push(LEAVE_WITHOUT_REJOIN);
        expected
    };
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::LEAVE_REQUEST,
            &[json!(IEEE_ADDRESS2), json!(LEAVE_WITHOUT_REJOIN)],
        ),
        without_rejoin
    );

    let and_rejoin = {
        let mut expected = vec![0];
        expected.extend_from_slice(&IEEE_ADDRESS2_BYTES);
        expected.push(LEAVE_AND_REJOIN);
        expected
    };
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::LEAVE_REQUEST,
            &[json!(IEEE_ADDRESS2), json!(LEAVE_AND_REJOIN)],
        ),
        and_rejoin
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_lqi_table_request() {
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::LQI_TABLE_REQUEST, &[json!(1)]),
        vec![0, 1]
    );
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::LQI_TABLE_REQUEST, &[json!(254)]),
        vec![0, 254]
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_match_descriptor_request() {
    let mut expected = vec![0];
    expected.extend_from_slice(&NODE_ID1_BYTES);
    expected.extend_from_slice(&HA_PROFILE_ID.to_le_bytes());
    expected.push(3);
    expected.extend_from_slice(&CLUSTER_LIST1_BYTES);
    expected.push(4);
    expected.extend_from_slice(&CLUSTER_LIST2_BYTES);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::MATCH_DESCRIPTORS_REQUEST,
            &[
                json!(NODE_ID1),
                json!(HA_PROFILE_ID),
                json!([0x0009, 0x0702, 0x0b03]),
                json!([0x0006, 0x0000, 0x0500, 0x0008]),
            ],
        ),
        expected
    );

    let mut expected = vec![0];
    expected.extend_from_slice(&NODE_ID1_BYTES);
    expected.extend_from_slice(&HA_PROFILE_ID.to_le_bytes());
    expected.push(3);
    expected.extend_from_slice(&CLUSTER_LIST1_BYTES);
    expected.push(0);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::MATCH_DESCRIPTORS_REQUEST,
            &[
                json!(NODE_ID1),
                json!(HA_PROFILE_ID),
                json!([0x0009, 0x0702, 0x0b03]),
                json!([]),
            ],
        ),
        expected
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_network_address_request() {
    let mut expected = vec![0];
    expected.extend_from_slice(&IEEE_ADDRESS1_BYTES);
    expected.extend_from_slice(&[0, 1]);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::NETWORK_ADDRESS_REQUEST,
            &[json!(IEEE_ADDRESS1), json!(false), json!(1)],
        ),
        expected
    );

    let mut expected = vec![0];
    expected.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    expected.extend_from_slice(&[1, 3]);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::NETWORK_ADDRESS_REQUEST,
            &[json!(IEEE_ADDRESS2), json!(true), json!(3)],
        ),
        expected
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_network_address_request_without_zdo_message_overhead() {
    let mut expected = IEEE_ADDRESS1_BYTES.to_vec();
    expected.extend_from_slice(&[0, 1]);
    assert_eq!(
        BuffaloZdo::build_request(
            false,
            ClusterId::NETWORK_ADDRESS_REQUEST,
            &[json!(IEEE_ADDRESS1), json!(false), json!(1)],
        ),
        expected
    );

    let mut expected = IEEE_ADDRESS2_BYTES.to_vec();
    expected.extend_from_slice(&[1, 3]);
    assert_eq!(
        BuffaloZdo::build_request(
            false,
            ClusterId::NETWORK_ADDRESS_REQUEST,
            &[json!(IEEE_ADDRESS2), json!(true), json!(3)],
        ),
        expected
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_node_descriptor_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn build_nwk_beacon_survey_request() {
    let tlv = json!({"scanChannelList": [], "configurationBitmask": 0});
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::NWK_BEACON_SURVEY_REQUEST, &[tlv]),
        vec![0, 0, 1, 0, 0]
    );

    let tlv2 = json!({"scanChannelList": [34252], "configurationBitmask": 1});
    let mut expected = vec![0, 0, 5, 1];
    expected.extend_from_slice(&34252u32.to_le_bytes());
    expected.push(1);
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::NWK_BEACON_SURVEY_REQUEST, &[tlv2]),
        expected
    );

    let tlv3 = json!({"scanChannelList": [34252, 123], "configurationBitmask": 1});
    let mut expected = vec![0, 0, 9, 2];
    expected.extend_from_slice(&34252u32.to_le_bytes());
    expected.extend_from_slice(&123u32.to_le_bytes());
    expected.push(1);
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::NWK_BEACON_SURVEY_REQUEST, &[tlv3]),
        expected
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_nwk_ieeejoining_list_request() {
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::NWK_IEEE_JOINING_LIST_REQUEST, &[json!(3)],),
        vec![0, 3]
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_parent_announce() {
    let children = [IEEE_ADDRESS1, IEEE_ADDRESS2];
    let mut expected = vec![0, children.len() as u8];
    expected.extend_from_slice(&IEEE_ADDRESS1_BYTES);
    expected.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::PARENT_ANNOUNCE, &[json!(children)]),
        expected
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_permit_joining() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn build_power_descriptor_request() {
    let mut expected = vec![0];
    expected.extend_from_slice(&NODE_ID1_BYTES);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::POWER_DESCRIPTOR_REQUEST,
            &[json!(NODE_ID1)],
        ),
        expected
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_retrieve_authentication_token_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn build_routing_table_request() {
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::ROUTING_TABLE_REQUEST, &[json!(1)]),
        vec![0, 1]
    );
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::ROUTING_TABLE_REQUEST, &[json!(254)]),
        vec![0, 254]
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_scan_channels_request() {
    let mut expected = vec![0];
    expected.extend_from_slice(&ALL_802_15_4_CHANNELS_MASK.to_le_bytes());
    expected.extend_from_slice(&[3, 3]);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::NWK_UPDATE_REQUEST,
            &[
                json!(ALL_802_15_4_CHANNELS.to_vec()),
                json!(3),
                json!(3),
                serde_json::Value::Null,
                serde_json::Value::Null,
            ],
        ),
        expected
    );

    let mut expected = vec![0];
    expected.extend_from_slice(&ALL_802_15_4_CHANNELS_MASK.to_le_bytes());
    expected.push(64);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::NWK_UPDATE_REQUEST,
            &[
                json!(ALL_802_15_4_CHANNELS.to_vec()),
                json!(64),
                json!(3),
                serde_json::Value::Null,
                serde_json::Value::Null,
            ],
        ),
        expected
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_set_active_channels_and_nwk_manager_id_request() {
    let mut expected = vec![0];
    expected.extend_from_slice(&PREFERRED_802_15_4_CHANNELS_MASK.to_le_bytes());
    expected.extend_from_slice(&[0xff, 3]);
    expected.extend_from_slice(&123u16.to_le_bytes());
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::NWK_UPDATE_REQUEST,
            &[
                json!(PREFERRED_802_15_4_CHANNELS.to_vec()),
                json!(0xff),
                serde_json::Value::Null,
                json!(3),
                json!(123),
            ],
        ),
        expected
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_simple_descriptor_request() {
    let mut expected = vec![0];
    expected.extend_from_slice(&NODE_ID1_BYTES);
    expected.push(3);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::SIMPLE_DESCRIPTOR_REQUEST,
            &[json!(NODE_ID1), json!(3)],
        ),
        expected
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_start_key_negotiation_request() {
    let tlv = json!({"eui64": IEEE_ADDRESS1, "publicPoint": vec![0xcdu8; 32]});
    let mut expected = vec![0, 0, 8 + 32 - 1];
    expected.extend_from_slice(&IEEE_ADDRESS1_BYTES);
    expected.extend_from_slice(&[0xcdu8; 32]);
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::START_KEY_NEGOTIATION_REQUEST, &[tlv],),
        expected
    );

    let tlv2 = json!({"eui64": IEEE_ADDRESS2, "publicPoint": vec![0x3cu8; 32]});
    let mut expected = vec![0, 0, 8 + 32 - 1];
    expected.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    expected.extend_from_slice(&[0x3cu8; 32]);
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::START_KEY_NEGOTIATION_REQUEST, &[tlv2],),
        expected
    );
}

#[test]
#[ignore = "not implemented yet"]
fn build_start_key_update_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn build_system_service_discovery_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn build_unbind_request() {
    let mut expected = vec![0];
    expected.extend_from_slice(&IEEE_ADDRESS1_BYTES);
    expected.push(2);
    expected.extend_from_slice(&SE_METERING_ID.to_le_bytes());
    expected.push(UNICAST_BINDING);
    expected.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    expected.push(64);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::UNBIND_REQUEST,
            &[
                json!(IEEE_ADDRESS1),
                json!(2),
                json!(SE_METERING_ID),
                json!(UNICAST_BINDING),
                json!(IEEE_ADDRESS2),
                json!(123),
                json!(64),
            ],
        ),
        expected
    );

    let mut expected = vec![0];
    expected.extend_from_slice(&IEEE_ADDRESS1_BYTES);
    expected.push(3);
    expected.extend_from_slice(&SE_METERING_ID.to_le_bytes());
    expected.push(MULTICAST_BINDING);
    expected.extend_from_slice(&123u16.to_le_bytes());
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::UNBIND_REQUEST,
            &[
                json!(IEEE_ADDRESS1),
                json!(3),
                json!(SE_METERING_ID),
                json!(MULTICAST_BINDING),
                json!(IEEE_ADDRESS2),
                json!(123),
                json!(64),
            ],
        ),
        expected
    );
}

#[test]
#[ignore = "not implemented yet"]
fn read_active_endpoints_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_binding_table_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_challenge_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_end_device_announce() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_get_authentication_level_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_get_configuration_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_ieeeaddress_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_lqitable_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_match_descriptors_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_network_address_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_node_descriptor_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_nwk_beacon_survey_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_nwk_enhanced_update_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_nwk_ieeejoining_list_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_nwk_unsolicited_enhanced_update_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_nwk_update_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_parent_announce_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_power_descriptor_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_retrieve_authentication_token_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_routing_table_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_set_configuration_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_simple_descriptor_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_start_key_negotiation_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn read_system_server_discovery_response() {
    todo!()
}
