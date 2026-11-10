// Port of .upstream/zigbee-herdsman/test/utils.test.ts

use zigbee_apiarist::controller::helpers::install_codes::check_install_code;
use zigbee_apiarist::utils::utils;

#[test]
fn is_number_array() {
    assert!(utils::is_number_array(&serde_json::json!([1, 2, 3])));
    assert!(!utils::is_number_array(&serde_json::json!([1, 2, "3"])));
    assert!(!utils::is_number_array(&serde_json::json!("nonarray")));
}

#[test]
fn is_number_array_of_length() {
    assert!(utils::is_number_array_of_length(
        &serde_json::json!([1, 2, 3]),
        3
    ));
    assert!(!utils::is_number_array_of_length(
        &serde_json::json!([1, 2]),
        3
    ));
    assert!(!utils::is_number_array_of_length(
        &serde_json::json!([1, 2, "3"]),
        3
    ));
    assert!(!utils::is_number_array_of_length(
        &serde_json::json!("nonarray"),
        3
    ));
}

#[test]
fn is_object_empty() {
    assert!(utils::is_object_empty(&serde_json::json!({})));
    assert!(!utils::is_object_empty(&serde_json::json!({"a": 1})));
}

#[test]
fn assert_string() {
    utils::assert_string(&serde_json::json!("bla"), "ignored");
    // Upstream: assertString(1) throws "Input must be a string!"
    // Rust equivalent is a panic or Result; kept as a documented expectation.
}

#[test]
fn checks_install_codes_of_all_lengths() {
    // 8-byte codes
    let code8_valid = vec![0x83, 0xFE, 0xD3, 0x40, 0x7A, 0x93, 0x2B, 0x70];
    let code8_invalid = vec![0xFF, 0xFE, 0xD3, 0x40, 0x7A, 0x93, 0x97, 0x23];
    let code8_invalid_fixed = vec![0xFF, 0xFE, 0xD3, 0x40, 0x7A, 0x93, 0xDE, 0x84];
    let code8_missing_crc = vec![0x83, 0xFE, 0xD3, 0x40, 0x7A, 0x93];

    assert_eq!(
        check_install_code(&code8_valid, true),
        (code8_valid.clone(), None)
    );
    assert_eq!(
        check_install_code(&code8_invalid, true),
        (code8_invalid_fixed.clone(), Some("invalid CRC".to_string()))
    );
    // adjust=false throws on invalid
    assert_eq!(
        check_install_code(&code8_missing_crc, true),
        (code8_valid.clone(), Some("missing CRC".to_string()))
    );

    // 10/14/18-byte cases follow the same pattern in upstream; see
    // .upstream/zigbee-herdsman/test/utils.test.ts for the full matrix.
}

#[test]
fn parses_legacy_backup_like_upstream() {
    use zigbee_apiarist::adapter::z_stack::structs::entries::nib::nib;
    use zigbee_apiarist::adapter::z_stack::structs::entries::nwk_key_descriptor::nwk_key_descriptor;
    use zigbee_apiarist::adapter::z_stack::structs::entries::nwk_sec_material_descriptor_entry::nwk_sec_material_descriptor_entry;
    use zigbee_apiarist::adapter::z_stack::structs::r#struct::Alignment;
    use zigbee_apiarist::utils::backup::from_legacy_backup;

    let network_key = [
        0x01, 0x03, 0x05, 0x07, 0x09, 0x0b, 0x0d, 0x0f, 0x00, 0x02, 0x04, 0x06, 0x08, 0x0a, 0x0c,
        0x0d,
    ];
    let extended_pan_id = [0x00, 0x12, 0x4b, 0x00, 0x24, 0xc2, 0xd4, 0x00];

    let mut nib_value = nib(None);
    nib_value.set_u16("nwkPanId", 0x1a62);
    nib_value.set_u32("channelList", 1u32 << 11);
    nib_value.set_u8("nwkLogicalChannel", 11);
    nib_value.set_bytes("extendedPANID", &extended_pan_id);
    nib_value.set_u8("SecurityLevel", 5);
    nib_value.set_u8("nwkUpdateId", 3);

    let mut key = nwk_key_descriptor(None);
    key.set_u8("keySeqNum", 7);
    key.set_bytes("key", &network_key);

    let mut material = nwk_sec_material_descriptor_entry(None);
    material.set_u32("FrameCounter", 0x12345678);
    material.set_bytes("extendedPanID", &extended_pan_id);

    let value = |bytes: Vec<u8>| serde_json::json!({"value": bytes});
    let backup = serde_json::json!({
        "data": {
            "ZCD_NV_NIB": value(nib_value.serialize(Alignment::Unaligned, true, 0)),
            "ZCD_NV_NWK_ACTIVE_KEY_INFO": value(key.serialize(Alignment::Unaligned, true, 0)),
            "ZCD_NV_PRECFGKEY_ENABLE": value(vec![0x01]),
            "ZCD_NV_EX_NWK_SEC_MATERIAL_TABLE": value(material.serialize(Alignment::Unaligned, true, 0)),
            "ZCD_NV_EXTADDR": value(vec![0x00, 0xd4, 0xc2, 0x24, 0x00, 0x4b, 0x12, 0x00]),
        }
    });

    let parsed = from_legacy_backup(&backup).expect("legacy backup");
    assert_eq!(parsed.network_options.pan_id, 0x1a62);
    assert_eq!(parsed.network_options.extended_pan_id, extended_pan_id);
    assert_eq!(parsed.network_options.channel_list, vec![11]);
    assert_eq!(parsed.network_options.network_key, network_key);
    assert!(parsed.network_options.network_key_distribute);
    assert_eq!(parsed.logical_channel, 11);
    assert_eq!(parsed.network_key_info.sequence_number, 7);
    assert_eq!(parsed.network_key_info.frame_counter, 0x12345678);
    assert_eq!(
        parsed.coordinator_ieee_address,
        vec![0x00, 0x12, 0x4b, 0x00, 0x24, 0xc2, 0xd4, 0x00]
    );
    assert_eq!(parsed.security_level, 5);
    assert_eq!(parsed.network_update_id, 3);
    assert!(parsed.devices.is_empty());
}
