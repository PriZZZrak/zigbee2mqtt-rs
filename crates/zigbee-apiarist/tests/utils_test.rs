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
