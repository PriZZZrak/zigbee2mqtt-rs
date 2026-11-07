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
use zigbee_apiarist::zspec::zdo::definition::enums::{
    ActiveLinkKeyType, GlobalTLV, InitialJoinMethod, JoiningPolicy,
};

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
const NODE_ID2: u16 = 0xab39;
const NODE_ID2_BYTES: [u8; 2] = [0x39, 0xab];
const EXT_PAN_ID1: [u8; 8] = [3, 43, 56, 23, 65, 23, 67, 23];
const EXT_PAN_ID2: [u8; 8] = [253, 231, 21, 3, 0, 44, 24, 46];
const HA_ENDPOINT: u8 = 0x01;
const GP_ENDPOINT: u8 = 0xf2;
const MANUFACTURER_BARACODA_SA: u16 = 0x1355;
const MANUFACTURER_BEIJING_RUYING_TECH_LIMITED: u16 = 0x128e;
const MANUFACTURER_ABB: u16 = 0x10eb;
const SPEKE_CURVE25519_SHA256: u8 = 2;
const BASIC_AUTHORIZATION_KEY: u8 = 3;

fn server_mask(revision: u16) -> serde_json::Value {
    json!({
        "primaryTrustCenter": 1,
        "backupTrustCenter": 1,
        "deprecated1": 0,
        "deprecated2": 0,
        "deprecated3": 0,
        "deprecated4": 0,
        "networkManager": 1,
        "reserved1": 0,
        "reserved2": 0,
        "stackComplianceRevision": revision,
    })
}

fn server_mask_byte(revision: u16) -> u16 {
    0x01 | 0x02 | 0x40 | ((revision << 9) & 0xfe00)
}

fn capabilities_device() -> serde_json::Value {
    json!({
        "alternatePANCoordinator": 0,
        "deviceType": 1,
        "powerSource": 1,
        "rxOnWhenIdle": 1,
        "reserved1": 0,
        "reserved2": 0,
        "securityCapability": 0,
        "allocateAddress": 0,
    })
}

#[test]
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
fn ignores_invalid_tlv_tag_and_reads_next_tlv() {
    let mut buffer = vec![0xff, 5 - 1];
    buffer.extend_from_slice(&MANUFACTURER_ABB.to_le_bytes());
    buffer.extend_from_slice(&[4, 5, 6]);
    buffer.push(GlobalTLV::MANUFACTURER_SPECIFIC);
    buffer.push(5 - 1);
    buffer.extend_from_slice(&MANUFACTURER_ABB.to_le_bytes());
    buffer.extend_from_slice(&[1, 2, 3]);
    let mut buffalo = BuffaloZdo::new(buffer);
    let tlvs = buffalo.read_tlvs();

    assert_eq!(
        tlvs,
        vec![json!({
            "tagId": GlobalTLV::MANUFACTURER_SPECIFIC,
            "length": 5,
            "tlv": {
                "zigbeeManufacturerId": MANUFACTURER_ABB,
                "additionalData": [1, 2, 3],
            },
        })]
    );
}

#[test]
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
fn throws_when_building_unknown_cluster_id() {
    let cluster_id = 0x8005u16;
    let result = std::panic::catch_unwind(|| {
        BuffaloZdo::build_request(true, cluster_id, &[json!([1, 123])]);
    });
    assert!(result.is_err());
}

#[test]
fn throws_when_duplicate_tlv_tag_found_and_not_valid() {
    let mut buffer = vec![GlobalTLV::FRAGMENTATION_PARAMETERS, 5 - 1];
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    buffer.push(1);
    buffer.extend_from_slice(&213u16.to_le_bytes());
    buffer.push(GlobalTLV::FRAGMENTATION_PARAMETERS);
    buffer.push(5 - 1);
    buffer.extend_from_slice(&NODE_ID2_BYTES);
    buffer.push(0);
    buffer.extend_from_slice(&344u16.to_le_bytes());
    let result = std::panic::catch_unwind(|| {
        BuffaloZdo::new(buffer).read_tlvs();
    });
    assert!(result.is_err());

    let mut buffer = vec![GlobalTLV::MANUFACTURER_SPECIFIC, 2 - 1];
    buffer.extend_from_slice(&MANUFACTURER_ABB.to_le_bytes());
    buffer.push(GlobalTLV::MANUFACTURER_SPECIFIC);
    buffer.push(2 - 1);
    buffer.extend_from_slice(&0x1162u16.to_le_bytes());
    let result = std::panic::catch_unwind(|| {
        BuffaloZdo::new(buffer).read_tlvs();
    });
    assert!(result.is_ok());
}

#[test]
fn throws_when_encapsulated_tlv_tag_found_inside_encapsulated() {
    let buffer = vec![
        GlobalTLV::BEACON_APPENDIX_ENCAPSULATION,
        4 - 1,
        GlobalTLV::JOINER_ENCAPSULATION,
        2 - 1,
        123,
        0xc8,
    ];
    let result = std::panic::catch_unwind(|| {
        BuffaloZdo::new(buffer).read_tlvs();
    });
    assert!(result.is_err());
}

#[test]
fn throws_when_not_enough_bytes_to_read_in_tlv() {
    let mut buffer = vec![GlobalTLV::MANUFACTURER_SPECIFIC, 6 - 1];
    buffer.extend_from_slice(&MANUFACTURER_ABB.to_le_bytes());
    let result = std::panic::catch_unwind(|| {
        BuffaloZdo::new(buffer).read_tlvs();
    });
    assert!(result.is_err());
}

#[test]
fn throws_when_reading_unknown_cluster_id() {
    let cluster_id = 0x0005u16;
    let result = std::panic::catch_unwind(|| {
        BuffaloZdo::read_response(true, cluster_id, &[1, 123]);
    });
    assert!(result.is_err());
}

#[test]
fn throws_when_writing_invalid_tlv_tag() {
    let mut buffalo = BuffaloZdo::new(vec![0; 3]);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        buffalo.write_global_tlv(&json!({
            "tagId": 0xfe,
            "length": 2,
            "tlv": {"nwkPanIdConflictCount": NODE_ID2},
        }));
    }));
    assert!(result.is_err());
}

#[test]
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
fn build_challenge_request() {
    let tlv = json!({"senderEui64": IEEE_ADDRESS2, "challengeValue": vec![0xfeu8; 8]});
    let mut expected = vec![0, 0, 8 + 8 - 1];
    expected.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    expected.extend_from_slice(&[0xfeu8; 8]);
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::CHALLENGE_REQUEST, &[tlv]),
        expected
    );

    let tlv2 = json!({
        "senderEui64": IEEE_ADDRESS1,
        "challengeValue": [0xfe, 0xac, 0x12, 0x23, 0x85, 0x8c, 0x7c, 0xa3],
    });
    let mut expected = vec![0, 0, 8 + 8 - 1];
    expected.extend_from_slice(&IEEE_ADDRESS1_BYTES);
    expected.extend_from_slice(&[0xfe, 0xac, 0x12, 0x23, 0x85, 0x8c, 0x7c, 0xa3]);
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::CHALLENGE_REQUEST, &[tlv2]),
        expected
    );
}

#[test]
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
fn build_decommission_request() {
    let tlv = json!({"eui64List": [IEEE_ADDRESS1]});
    let mut expected = vec![0, 0, 8 * 1 + 1 - 1, 1];
    expected.extend_from_slice(&IEEE_ADDRESS1_BYTES);
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::DECOMMISSION_REQUEST, &[tlv]),
        expected
    );

    let tlv2 = json!({"eui64List": [IEEE_ADDRESS2, IEEE_ADDRESS1]});
    let mut expected = vec![0, 0, 8 * 2 + 1 - 1, 2];
    expected.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    expected.extend_from_slice(&IEEE_ADDRESS1_BYTES);
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::DECOMMISSION_REQUEST, &[tlv2]),
        expected
    );

    let tlv3 = json!({"eui64List": []});
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::DECOMMISSION_REQUEST, &[tlv3]),
        vec![0, 0, 8 * 0 + 1 - 1, 0]
    );
}

#[test]
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
fn build_get_authentication_level_request() {
    let tlv = json!({"ieee": IEEE_ADDRESS2});
    let mut expected = vec![0, 0, 8 - 1];
    expected.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::GET_AUTHENTICATION_LEVEL_REQUEST, &[tlv],),
        expected
    );
}

#[test]
fn build_get_configuration_request() {
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::GET_CONFIGURATION_REQUEST, &[json!([84])]),
        vec![0, 1, 84]
    );
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::GET_CONFIGURATION_REQUEST,
            &[json!([67, 71])],
        ),
        vec![0, 2, 67, 71]
    );
}

#[test]
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
fn build_node_descriptor_request() {
    let mut expected = vec![0];
    expected.extend_from_slice(&NODE_ID1_BYTES);
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::NODE_DESCRIPTOR_REQUEST, &[json!(NODE_ID1)],),
        expected
    );

    let tlv = json!({"nwkAddress": NODE_ID1});
    let mut expected = vec![0];
    expected.extend_from_slice(&NODE_ID1_BYTES);
    expected.push(GlobalTLV::FRAGMENTATION_PARAMETERS);
    expected.push(1);
    expected.extend_from_slice(&NODE_ID1_BYTES);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::NODE_DESCRIPTOR_REQUEST,
            &[json!(NODE_ID1), tlv],
        ),
        expected
    );

    let tlv2 = json!({"nwkAddress": NODE_ID1, "fragmentationOptions": 1});
    let mut expected = vec![0];
    expected.extend_from_slice(&NODE_ID1_BYTES);
    expected.push(GlobalTLV::FRAGMENTATION_PARAMETERS);
    expected.push(2);
    expected.extend_from_slice(&NODE_ID1_BYTES);
    expected.push(1);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::NODE_DESCRIPTOR_REQUEST,
            &[json!(NODE_ID1), tlv2],
        ),
        expected
    );

    let tlv3 = json!({"nwkAddress": NODE_ID1, "maxIncomingTransferUnit": 256});
    let mut expected = vec![0];
    expected.extend_from_slice(&NODE_ID1_BYTES);
    expected.push(GlobalTLV::FRAGMENTATION_PARAMETERS);
    expected.push(3);
    expected.extend_from_slice(&NODE_ID1_BYTES);
    expected.extend_from_slice(&256u16.to_le_bytes());
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::NODE_DESCRIPTOR_REQUEST,
            &[json!(NODE_ID1), tlv3],
        ),
        expected
    );

    let tlv4 = json!({
        "nwkAddress": NODE_ID1,
        "fragmentationOptions": 1,
        "maxIncomingTransferUnit": 65352,
    });
    let mut expected = vec![0];
    expected.extend_from_slice(&NODE_ID1_BYTES);
    expected.push(GlobalTLV::FRAGMENTATION_PARAMETERS);
    expected.push(4);
    expected.extend_from_slice(&NODE_ID1_BYTES);
    expected.push(1);
    expected.extend_from_slice(&65352u16.to_le_bytes());
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::NODE_DESCRIPTOR_REQUEST,
            &[json!(NODE_ID1), tlv4],
        ),
        expected
    );
}

#[test]
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
fn build_nwk_ieeejoining_list_request() {
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::NWK_IEEE_JOINING_LIST_REQUEST, &[json!(3)],),
        vec![0, 3]
    );
}

#[test]
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
fn build_permit_joining() {
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::PERMIT_JOINING_REQUEST,
            &[json!(254), json!(1), json!([])],
        ),
        vec![0, 254, 1]
    );

    let tlvs = json!([{
        "tagId": GlobalTLV::BEACON_APPENDIX_ENCAPSULATION,
        "length": 4,
        "tlv": {
            "additionalTLVs": [{
                "tagId": GlobalTLV::FRAGMENTATION_PARAMETERS,
                "length": 2,
                "tlv": {"nwkAddress": NODE_ID1},
            }],
        },
    }]);
    let mut expected = vec![0, 255, 1, GlobalTLV::BEACON_APPENDIX_ENCAPSULATION, 4 - 1];
    expected.push(GlobalTLV::FRAGMENTATION_PARAMETERS);
    expected.push(2 - 1);
    expected.extend_from_slice(&NODE_ID1_BYTES);
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::PERMIT_JOINING_REQUEST,
            &[json!(255), json!(1), tlvs],
        ),
        expected
    );
}

#[test]
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
fn build_retrieve_authentication_token_request() {
    let tlv = json!({"tlvTypeTagId": 0});
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::RETRIEVE_AUTHENTICATION_TOKEN_REQUEST,
            &[tlv],
        ),
        vec![0, 0, 0, 0]
    );

    let tlv2 = json!({"tlvTypeTagId": 31});
    assert_eq!(
        BuffaloZdo::build_request(
            true,
            ClusterId::RETRIEVE_AUTHENTICATION_TOKEN_REQUEST,
            &[tlv2],
        ),
        vec![0, 0, 0, 31]
    );
}

#[test]
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
fn build_start_key_update_request() {
    let method = json!({
        "protocol": SPEKE_CURVE25519_SHA256,
        "presharedSecret": BASIC_AUTHORIZATION_KEY,
        "sendingDeviceEui64": IEEE_ADDRESS2,
    });
    let params = json!({
        "nwkAddress": NODE_ID1,
        "fragmentationOptions": 1,
        "maxIncomingTransferUnit": 2345,
    });
    let mut expected = vec![
        0,
        0,
        8 + 2 - 1,
        SPEKE_CURVE25519_SHA256,
        BASIC_AUTHORIZATION_KEY,
    ];
    expected.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    expected.push(GlobalTLV::FRAGMENTATION_PARAMETERS);
    expected.push(5 - 1);
    expected.extend_from_slice(&NODE_ID1_BYTES);
    expected.push(1);
    expected.extend_from_slice(&2345u16.to_le_bytes());
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::START_KEY_UPDATE_REQUEST, &[method, params],),
        expected
    );
}

#[test]
fn build_system_service_discovery_request() {
    let mask0 = json!({
        "primaryTrustCenter": 1,
        "backupTrustCenter": 0,
        "deprecated1": 0,
        "deprecated2": 0,
        "deprecated3": 0,
        "deprecated4": 0,
        "networkManager": 0,
        "reserved1": 0,
        "reserved2": 0,
        "stackComplianceRevision": 0,
    });
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::SYSTEM_SERVER_DISCOVERY_REQUEST, &[mask0],),
        vec![0, 0b00000001, 0]
    );

    let mask23 = json!({
        "primaryTrustCenter": 1,
        "backupTrustCenter": 0,
        "deprecated1": 0,
        "deprecated2": 0,
        "deprecated3": 0,
        "deprecated4": 0,
        "networkManager": 1,
        "reserved1": 0,
        "reserved2": 0,
        "stackComplianceRevision": 23,
    });
    let byte = 0b0010111001000001u16;
    assert_eq!(
        BuffaloZdo::build_request(true, ClusterId::SYSTEM_SERVER_DISCOVERY_REQUEST, &[mask23],),
        vec![0, (byte & 0xff) as u8, (byte >> 8) as u8]
    );
}

#[test]
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
fn read_active_endpoints_response() {
    let mut buffer = vec![1, 0];
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    buffer.extend_from_slice(&[2, 0xef, 0x87]);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::ACTIVE_ENDPOINTS_RESPONSE, &buffer),
        json!([0, {"nwkAddress": NODE_ID1, "endpointList": [0xef, 0x87]}])
    );

    let mut buffer = vec![1, 0];
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    buffer.push(0);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::ACTIVE_ENDPOINTS_RESPONSE, &buffer),
        json!([0, {"nwkAddress": NODE_ID1, "endpointList": []}])
    );
}

#[test]
fn read_binding_table_response() {
    let mut buffer = vec![1, 0, 1, 0, 3];
    buffer.extend_from_slice(&IEEE_ADDRESS1_BYTES);
    buffer.push(0xf0);
    buffer.extend_from_slice(&0x0103u16.to_le_bytes());
    buffer.push(0x03);
    buffer.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    buffer.push(GP_ENDPOINT);
    buffer.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    buffer.push(0x34);
    buffer.extend_from_slice(&0x0100u16.to_le_bytes());
    buffer.push(0x01);
    buffer.extend_from_slice(&NODE_ID2_BYTES);
    buffer.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    buffer.push(0xf4);
    buffer.extend_from_slice(&0x000cu16.to_le_bytes());
    buffer.push(0x02);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::BINDING_TABLE_RESPONSE, &buffer),
        json!([0, {
            "bindingTableEntries": 1,
            "startIndex": 0,
            "entryList": [
                {
                    "sourceEui64": IEEE_ADDRESS1,
                    "sourceEndpoint": 0xf0,
                    "clusterId": 0x0103,
                    "destAddrMode": 0x03,
                    "dest": IEEE_ADDRESS2,
                    "destEndpoint": GP_ENDPOINT,
                },
                {
                    "sourceEui64": IEEE_ADDRESS2,
                    "sourceEndpoint": 0x34,
                    "clusterId": 0x0100,
                    "destAddrMode": 0x01,
                    "dest": NODE_ID2,
                    "destEndpoint": serde_json::Value::Null,
                },
            ],
        }])
    );

    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::BINDING_TABLE_RESPONSE, &[1, 0, 30, 2, 0]),
        json!([0, {
            "bindingTableEntries": 30,
            "startIndex": 2,
            "entryList": [],
        }])
    );
}

#[test]
fn read_challenge_response() {
    let mut buffer = vec![1, 0, 0x00, 32 - 1];
    buffer.extend_from_slice(&IEEE_ADDRESS1_BYTES);
    buffer.extend_from_slice(&[0x39u8; 8]);
    buffer.extend_from_slice(&4302952u32.to_le_bytes());
    buffer.extend_from_slice(&12435682u32.to_le_bytes());
    buffer.extend_from_slice(&[0xff, 0xfe, 0x34, 0x04, 0x49, 0x9f, 0x03, 0xbc]);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::CHALLENGE_RESPONSE, &buffer),
        json!([0, {
            "tlvs": [{
                "tagId": 0x00,
                "length": 32,
                "tlv": {
                    "responderEui64": IEEE_ADDRESS1,
                    "receivedChallengeValue": vec![0x39u8; 8],
                    "apsFrameCounter": 4302952u64,
                    "challengeSecurityFrameCounter": 12435682u64,
                    "mic": [0xff, 0xfe, 0x34, 0x04, 0x49, 0x9f, 0x03, 0xbc],
                },
            }],
        }])
    );

    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::CHALLENGE_RESPONSE, &[1, 0]),
        json!([0, {"tlvs": []}])
    );
}

#[test]
fn read_end_device_announce() {
    let mut buffer = vec![1];
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    buffer.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    buffer.push(0b01000000);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::END_DEVICE_ANNOUNCE, &buffer),
        json!([0, {
            "nwkAddress": NODE_ID1,
            "eui64": IEEE_ADDRESS2,
            "capabilities": {
                "alternatePANCoordinator": 0,
                "deviceType": 0,
                "powerSource": 0,
                "rxOnWhenIdle": 0,
                "reserved1": 0,
                "reserved2": 0,
                "securityCapability": 1,
                "allocateAddress": 0,
            },
        }])
    );
}

#[test]
fn read_get_authentication_level_response() {
    let mut buffer = vec![1, 0, 0x00, 10 - 1];
    buffer.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    buffer.push(InitialJoinMethod::INSTALL_CODE_KEY);
    buffer.push(ActiveLinkKeyType::AUTHENTICATED_KEY_NEGOTIATION);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::GET_AUTHENTICATION_LEVEL_RESPONSE, &buffer,),
        json!([0, {
            "tlvs": [{
                "tagId": 0x00,
                "length": 10,
                "tlv": {
                    "remoteNodeIeee": IEEE_ADDRESS2,
                    "initialJoinMethod": InitialJoinMethod::INSTALL_CODE_KEY,
                    "activeLinkKeyType": ActiveLinkKeyType::AUTHENTICATED_KEY_NEGOTIATION,
                },
            }],
        }])
    );
}

#[test]
fn read_get_configuration_response() {
    let mut buffer = vec![1, 0, GlobalTLV::FRAGMENTATION_PARAMETERS, 5 - 1];
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    buffer.push(1);
    buffer.extend_from_slice(&65352u16.to_le_bytes());
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::GET_CONFIGURATION_RESPONSE, &buffer),
        json!([0, {
            "tlvs": [{
                "tagId": GlobalTLV::FRAGMENTATION_PARAMETERS,
                "length": 5,
                "tlv": {
                    "nwkAddress": NODE_ID1,
                    "fragmentationOptions": 1,
                    "maxIncomingTransferUnit": 65352,
                },
            }],
        }])
    );

    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::GET_CONFIGURATION_RESPONSE, &[1, 0]),
        json!([0, {"tlvs": []}])
    );
}

#[test]
fn read_ieeeaddress_response() {
    let mut buffer = vec![1, 0];
    buffer.extend_from_slice(&IEEE_ADDRESS1_BYTES);
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::IEEE_ADDRESS_RESPONSE, &buffer),
        json!([0, {
            "eui64": IEEE_ADDRESS1,
            "nwkAddress": NODE_ID1,
            "startIndex": 0,
            "assocDevList": [],
        }])
    );

    let mut buffer = vec![1, 0];
    buffer.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    buffer.extend_from_slice(&[2, 3]);
    buffer.extend_from_slice(&123u16.to_le_bytes());
    buffer.extend_from_slice(&52523u16.to_le_bytes());
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::IEEE_ADDRESS_RESPONSE, &buffer),
        json!([0, {
            "eui64": IEEE_ADDRESS2,
            "nwkAddress": NODE_ID1,
            "startIndex": 3,
            "assocDevList": [123, 52523],
        }])
    );
}

#[test]
fn read_lqitable_response() {
    let mut buffer = vec![1, 0, 16, 3, 2];
    buffer.extend_from_slice(&EXT_PAN_ID2);
    buffer.extend_from_slice(&IEEE_ADDRESS1_BYTES);
    buffer.extend_from_slice(&NODE_ID2_BYTES);
    buffer.extend_from_slice(&[0b00100101, 0b00000001, 1, 235]);
    buffer.extend_from_slice(&EXT_PAN_ID1);
    buffer.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    buffer.extend_from_slice(&[0b01000010, 0b00000000, 1, 179]);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::LQI_TABLE_RESPONSE, &buffer),
        json!([0, {
            "neighborTableEntries": 16,
            "startIndex": 3,
            "entryList": [
                {
                    "extendedPanId": EXT_PAN_ID2.to_vec(),
                    "eui64": IEEE_ADDRESS1,
                    "nwkAddress": NODE_ID2,
                    "deviceType": 1,
                    "rxOnWhenIdle": 1,
                    "relationship": 2,
                    "reserved1": 0,
                    "permitJoining": 1,
                    "reserved2": 0,
                    "depth": 1,
                    "lqi": 235,
                },
                {
                    "extendedPanId": EXT_PAN_ID1.to_vec(),
                    "eui64": IEEE_ADDRESS2,
                    "nwkAddress": NODE_ID1,
                    "deviceType": 2,
                    "rxOnWhenIdle": 0,
                    "relationship": 4,
                    "reserved1": 0,
                    "permitJoining": 0,
                    "reserved2": 0,
                    "depth": 1,
                    "lqi": 179,
                },
            ],
        }])
    );

    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::LQI_TABLE_RESPONSE, &[1, 0, 5, 4, 0]),
        json!([0, {
            "neighborTableEntries": 5,
            "startIndex": 4,
            "entryList": [],
        }])
    );
}

#[test]
fn read_match_descriptors_response() {
    let mut buffer = vec![1, 0];
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    buffer.extend_from_slice(&[2, 0xef, 0x87]);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::MATCH_DESCRIPTORS_RESPONSE, &buffer),
        json!([0, {"nwkAddress": NODE_ID1, "endpointList": [0xef, 0x87]}])
    );

    let mut buffer = vec![1, 0];
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    buffer.push(0);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::MATCH_DESCRIPTORS_RESPONSE, &buffer),
        json!([0, {"nwkAddress": NODE_ID1, "endpointList": []}])
    );
}

#[test]
fn read_network_address_response() {
    let mut buffer = vec![1, 0];
    buffer.extend_from_slice(&IEEE_ADDRESS1_BYTES);
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::NETWORK_ADDRESS_RESPONSE, &buffer),
        json!([0, {
            "eui64": IEEE_ADDRESS1,
            "nwkAddress": NODE_ID1,
            "startIndex": 0,
            "assocDevList": [],
        }])
    );

    let mut buffer = vec![1, 0];
    buffer.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    buffer.extend_from_slice(&[2, 3]);
    buffer.extend_from_slice(&123u16.to_le_bytes());
    buffer.extend_from_slice(&52523u16.to_le_bytes());
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::NETWORK_ADDRESS_RESPONSE, &buffer),
        json!([0, {
            "eui64": IEEE_ADDRESS2,
            "nwkAddress": NODE_ID1,
            "startIndex": 3,
            "assocDevList": [123, 52523],
        }])
    );
}

#[test]
fn read_node_descriptor_response() {
    let mut buffer = vec![1, 0];
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    buffer.extend_from_slice(&[0b00100010, 0b00100000, 0b00001110]);
    buffer.extend_from_slice(&MANUFACTURER_BARACODA_SA.to_le_bytes());
    buffer.push(0x7c);
    buffer.extend_from_slice(&0x7effu16.to_le_bytes());
    buffer.extend_from_slice(&server_mask_byte(22).to_le_bytes());
    buffer.extend_from_slice(&0x3cffu16.to_le_bytes());
    buffer.push(0);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::NODE_DESCRIPTOR_RESPONSE, &buffer),
        json!([0, {
            "nwkAddress": NODE_ID1,
            "logicalType": 0b010,
            "fragmentationSupported": serde_json::Value::Null,
            "apsFlags": 0,
            "frequencyBand": 0b00100,
            "capabilities": capabilities_device(),
            "manufacturerCode": MANUFACTURER_BARACODA_SA,
            "maxBufSize": 0x7c,
            "maxIncTxSize": 0x7eff,
            "serverMask": server_mask(22),
            "maxOutTxSize": 0x3cff,
            "deprecated1": 0,
            "tlvs": [],
        }])
    );

    let mut buffer = vec![1, 0];
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    buffer.extend_from_slice(&[0b00100010, 0b00100000, 0b00001110]);
    buffer.extend_from_slice(&MANUFACTURER_BEIJING_RUYING_TECH_LIMITED.to_le_bytes());
    buffer.push(0x3a);
    buffer.extend_from_slice(&0x7cffu16.to_le_bytes());
    buffer.extend_from_slice(&server_mask_byte(23).to_le_bytes());
    buffer.extend_from_slice(&0x11ffu16.to_le_bytes());
    buffer.push(0);
    buffer.push(GlobalTLV::FRAGMENTATION_PARAMETERS);
    buffer.push(5 - 1);
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    buffer.push(1);
    buffer.extend_from_slice(&65352u16.to_le_bytes());
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::NODE_DESCRIPTOR_RESPONSE, &buffer),
        json!([0, {
            "nwkAddress": NODE_ID1,
            "logicalType": 0b010,
            "fragmentationSupported": true,
            "apsFlags": 0,
            "frequencyBand": 0b00100,
            "capabilities": capabilities_device(),
            "manufacturerCode": MANUFACTURER_BEIJING_RUYING_TECH_LIMITED,
            "maxBufSize": 0x3a,
            "maxIncTxSize": 0x7cff,
            "serverMask": server_mask(23),
            "maxOutTxSize": 0x11ff,
            "deprecated1": 0,
            "tlvs": [{
                "tagId": GlobalTLV::FRAGMENTATION_PARAMETERS,
                "length": 5,
                "tlv": {
                    "nwkAddress": NODE_ID1,
                    "fragmentationOptions": 1,
                    "maxIncomingTransferUnit": 65352,
                },
            }],
        }])
    );
}

#[test]
fn read_nwk_beacon_survey_response() {
    let mut buffer = vec![1, 0, 0x01, 4 - 1, 14, 7, 5, 2, 0x02, 7 - 1];
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    buffer.extend_from_slice(&[234, 1]);
    buffer.extend_from_slice(&NODE_ID2_BYTES);
    buffer.push(223);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::NWK_BEACON_SURVEY_RESPONSE, &buffer),
        json!([0, {
            "tlvs": [
                {
                    "tagId": 0x01,
                    "length": 4,
                    "tlv": {
                        "totalBeaconsReceived": 14,
                        "onNetworkBeacons": 7,
                        "potentialParentBeacons": 5,
                        "otherNetworkBeacons": 2,
                    },
                },
                {
                    "tagId": 0x02,
                    "length": 7,
                    "tlv": {
                        "currentParentNwkAddress": NODE_ID1,
                        "currentParentLQA": 234,
                        "entryCount": 1,
                        "potentialParents": [{"nwkAddress": NODE_ID2, "lqa": 223}],
                    },
                },
            ],
        }])
    );

    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::NWK_BEACON_SURVEY_RESPONSE, &[1, 0]),
        json!([0, {"tlvs": []}])
    );
}

#[test]
fn read_nwk_enhanced_update_response() {
    let mut buffer = vec![1, 0];
    buffer.extend_from_slice(&34732495u32.to_le_bytes());
    buffer.extend_from_slice(&445u16.to_le_bytes());
    buffer.extend_from_slice(&34u16.to_le_bytes());
    buffer.extend_from_slice(&[3, 0x43, 0xff, 0x6f]);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::NWK_ENHANCED_UPDATE_RESPONSE, &buffer),
        json!([0, {
            "scannedChannels": 34732495u64,
            "totalTransmissions": 445,
            "totalFailures": 34,
            "entryList": [0x43, 0xff, 0x6f],
        }])
    );
}

#[test]
fn read_nwk_ieeejoining_list_response() {
    let mut buffer = vec![1, 0, 3, JoiningPolicy::IEEELIST_JOIN, 4, 0, 2];
    buffer.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    buffer.extend_from_slice(&IEEE_ADDRESS1_BYTES);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::NWK_IEEE_JOINING_LIST_RESPONSE, &buffer),
        json!([0, {
            "updateId": 3,
            "joiningPolicy": JoiningPolicy::IEEELIST_JOIN,
            "entryListTotal": 4,
            "startIndex": 0,
            "entryList": [IEEE_ADDRESS2, IEEE_ADDRESS1],
        }])
    );

    assert_eq!(
        BuffaloZdo::read_response(
            true,
            ClusterId::NWK_IEEE_JOINING_LIST_RESPONSE,
            &[1, 0, 0xff, JoiningPolicy::ALL_JOIN, 0],
        ),
        json!([0, {
            "updateId": 0xff,
            "joiningPolicy": JoiningPolicy::ALL_JOIN,
            "entryListTotal": 0,
            "startIndex": serde_json::Value::Null,
            "entryList": serde_json::Value::Null,
        }])
    );
}

#[test]
fn read_nwk_unsolicited_enhanced_update_response() {
    let mut buffer = vec![1, 0];
    buffer.extend_from_slice(&9023342u32.to_le_bytes());
    buffer.extend_from_slice(&3454u16.to_le_bytes());
    buffer.extend_from_slice(&435u16.to_le_bytes());
    buffer.extend_from_slice(&1239u16.to_le_bytes());
    buffer.push(32);
    assert_eq!(
        BuffaloZdo::read_response(
            true,
            ClusterId::NWK_UNSOLICITED_ENHANCED_UPDATE_RESPONSE,
            &buffer,
        ),
        json!([0, {
            "channelInUse": 9023342u64,
            "macTxUCastTotal": 3454,
            "macTxUCastFailures": 435,
            "macTxUCastRetries": 1239,
            "timePeriod": 32,
        }])
    );
}

#[test]
fn read_nwk_update_response() {
    let mut buffer = vec![1, 0];
    buffer.extend_from_slice(&34732495u32.to_le_bytes());
    buffer.extend_from_slice(&445u16.to_le_bytes());
    buffer.extend_from_slice(&34u16.to_le_bytes());
    buffer.extend_from_slice(&[3, 0x43, 0xff, 0x6f]);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::NWK_UPDATE_RESPONSE, &buffer),
        json!([0, {
            "scannedChannels": 34732495u64,
            "totalTransmissions": 445,
            "totalFailures": 34,
            "entryList": [0x43, 0xff, 0x6f],
        }])
    );
}

#[test]
fn read_parent_announce_response() {
    let mut buffer = vec![1, 0, 2];
    buffer.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    buffer.extend_from_slice(&IEEE_ADDRESS1_BYTES);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::PARENT_ANNOUNCE_RESPONSE, &buffer),
        json!([0, {"children": [IEEE_ADDRESS2, IEEE_ADDRESS1]}])
    );

    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::PARENT_ANNOUNCE_RESPONSE, &[1, 0, 0]),
        json!([0, {"children": []}])
    );
}

#[test]
fn read_power_descriptor_response() {
    let mut buffer = vec![1, 0];
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    buffer.extend_from_slice(&[0b10100100, 0b11110100]);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::POWER_DESCRIPTOR_RESPONSE, &buffer),
        json!([0, {
            "nwkAddress": NODE_ID1,
            "currentPowerMode": 0b0100,
            "availPowerSources": 0b1010,
            "currentPowerSource": 0b0100,
            "currentPowerSourceLevel": 0b1111,
        }])
    );
}

#[test]
fn read_retrieve_authentication_token_response() {
    let mut buffer = vec![1, 0, GlobalTLV::FRAGMENTATION_PARAMETERS, 5 - 1];
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    buffer.push(1);
    buffer.extend_from_slice(&65352u16.to_le_bytes());
    assert_eq!(
        BuffaloZdo::read_response(
            true,
            ClusterId::RETRIEVE_AUTHENTICATION_TOKEN_RESPONSE,
            &buffer,
        ),
        json!([0, {
            "tlvs": [{
                "tagId": GlobalTLV::FRAGMENTATION_PARAMETERS,
                "length": 5,
                "tlv": {
                    "nwkAddress": NODE_ID1,
                    "fragmentationOptions": 1,
                    "maxIncomingTransferUnit": 65352,
                },
            }],
        }])
    );

    assert_eq!(
        BuffaloZdo::read_response(
            true,
            ClusterId::RETRIEVE_AUTHENTICATION_TOKEN_RESPONSE,
            &[1, 0],
        ),
        json!([0, {"tlvs": []}])
    );
}

#[test]
fn read_routing_table_response() {
    let mut buffer = vec![1, 0, 4, 3, 1];
    buffer.extend_from_slice(&NODE_ID2_BYTES);
    buffer.push(0b00101000);
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::ROUTING_TABLE_RESPONSE, &buffer),
        json!([0, {
            "routingTableEntries": 4,
            "startIndex": 3,
            "entryList": [{
                "destinationAddress": NODE_ID2,
                "status": "ACTIVE",
                "memoryConstrained": 1,
                "manyToOne": 0,
                "routeRecordRequired": 1,
                "reserved1": 0,
                "nextHopAddress": NODE_ID1,
            }],
        }])
    );

    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::ROUTING_TABLE_RESPONSE, &[1, 0, 0, 0, 0]),
        json!([0, {
            "routingTableEntries": 0,
            "startIndex": 0,
            "entryList": [],
        }])
    );
}

#[test]
fn read_set_configuration_response() {
    let buffer = vec![
        1,
        0,
        0x00,
        7 - 1,
        3,
        0x00,
        0,
        GlobalTLV::CONFIGURATION_PARAMETERS,
        0,
        GlobalTLV::ROUTER_INFORMATION,
        0x80,
    ];
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::SET_CONFIGURATION_RESPONSE, &buffer),
        json!([0, {
            "tlvs": [{
                "tagId": 0x00,
                "length": 7,
                "tlv": {
                    "count": 3,
                    "tlvs": [
                        {"tagId": 0x00, "processingStatus": 0},
                        {"tagId": GlobalTLV::CONFIGURATION_PARAMETERS, "processingStatus": 0},
                        {"tagId": GlobalTLV::ROUTER_INFORMATION, "processingStatus": 0x80},
                    ],
                },
            }],
        }])
    );

    assert_eq!(
        BuffaloZdo::read_response(
            true,
            ClusterId::SET_CONFIGURATION_RESPONSE,
            &[1, 0, 0x00, 0, 0]
        ),
        json!([0, {
            "tlvs": [{
                "tagId": 0x00,
                "length": 1,
                "tlv": {"count": 0, "tlvs": []},
            }],
        }])
    );
}

#[test]
fn read_simple_descriptor_response() {
    let mut buffer = vec![1, 0];
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    buffer.push(8);
    buffer.push(HA_ENDPOINT);
    buffer.extend_from_slice(&HA_PROFILE_ID.to_le_bytes());
    buffer.extend_from_slice(&123u16.to_le_bytes());
    buffer.push(0);
    buffer.push(2);
    buffer.extend_from_slice(&7653u16.to_le_bytes());
    buffer.extend_from_slice(&624u16.to_le_bytes());
    buffer.push(1);
    buffer.extend_from_slice(&5322u16.to_le_bytes());
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::SIMPLE_DESCRIPTOR_RESPONSE, &buffer),
        json!([0, {
            "nwkAddress": NODE_ID1,
            "length": 8,
            "endpoint": HA_ENDPOINT,
            "profileId": HA_PROFILE_ID,
            "deviceId": 123,
            "deviceVersion": 0,
            "inClusterList": [7653, 624],
            "outClusterList": [5322],
        }])
    );

    let mut buffer = vec![1, 0];
    buffer.extend_from_slice(&NODE_ID1_BYTES);
    buffer.push(8);
    buffer.push(HA_ENDPOINT);
    buffer.extend_from_slice(&HA_PROFILE_ID.to_le_bytes());
    buffer.extend_from_slice(&123u16.to_le_bytes());
    buffer.push(0);
    buffer.push(0);
    buffer.push(0);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::SIMPLE_DESCRIPTOR_RESPONSE, &buffer),
        json!([0, {
            "nwkAddress": NODE_ID1,
            "length": 8,
            "endpoint": HA_ENDPOINT,
            "profileId": HA_PROFILE_ID,
            "deviceId": 123,
            "deviceVersion": 0,
            "inClusterList": [],
            "outClusterList": [],
        }])
    );
}

#[test]
fn read_start_key_negotiation_response() {
    let mut buffer = vec![1, 0, 0x00, 8 + 32 - 1];
    buffer.extend_from_slice(&IEEE_ADDRESS2_BYTES);
    buffer.extend_from_slice(&[0xabu8; 32]);
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::START_KEY_NEGOTIATION_RESPONSE, &buffer),
        json!([0, {
            "tlvs": [{
                "tagId": 0x00,
                "length": 8 + 32,
                "tlv": {
                    "eui64": IEEE_ADDRESS2,
                    "publicPoint": vec![0xabu8; 32],
                },
            }],
        }])
    );
}

#[test]
fn read_system_server_discovery_response() {
    let mut buffer = vec![1, 0];
    buffer.extend_from_slice(&server_mask_byte(23).to_le_bytes());
    assert_eq!(
        BuffaloZdo::read_response(true, ClusterId::SYSTEM_SERVER_DISCOVERY_RESPONSE, &buffer),
        json!([0, {"serverMask": server_mask(23)}])
    );
}
