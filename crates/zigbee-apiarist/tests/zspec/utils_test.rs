// Original: .upstream/zigbee-herdsman/test/zspec/utils.test.ts

use zigbee_apiarist::zspec::consts::{
    ALL_802_15_4_CHANNELS, ALL_802_15_4_CHANNELS_MASK, PREFERRED_802_15_4_CHANNELS,
    PREFERRED_802_15_4_CHANNELS_MASK,
};
use zigbee_apiarist::zspec::enums::BroadcastAddress;
use zigbee_apiarist::zspec::utils;

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).unwrap())
        .collect()
}

#[test]
#[ignore = "not implemented yet"]
fn converts_channels_number_array_to_uint32_mask() {
    assert_eq!(
        utils::channels_to_uint32_mask(&ALL_802_15_4_CHANNELS),
        ALL_802_15_4_CHANNELS_MASK
    );
    assert_eq!(
        utils::channels_to_uint32_mask(&PREFERRED_802_15_4_CHANNELS),
        PREFERRED_802_15_4_CHANNELS_MASK
    );
}

#[test]
#[ignore = "not implemented yet"]
fn converts_channels_uint32_mask_to_number_array() {
    assert_eq!(
        utils::uint32_mask_to_channels(ALL_802_15_4_CHANNELS_MASK),
        ALL_802_15_4_CHANNELS.to_vec()
    );
    assert_eq!(
        utils::uint32_mask_to_channels(PREFERRED_802_15_4_CHANNELS_MASK),
        PREFERRED_802_15_4_CHANNELS.to_vec()
    );
}

#[test]
#[ignore = "not implemented yet"]
fn checks_if_address_is_broadcast() {
    assert!(utils::is_broadcast_address(
        BroadcastAddress::DEFAULT as u16
    ));
    assert!(utils::is_broadcast_address(
        BroadcastAddress::RX_ON_WHEN_IDLE as u16
    ));
    assert!(utils::is_broadcast_address(BroadcastAddress::SLEEPY as u16));
    assert!(utils::is_broadcast_address(
        BroadcastAddress::LOW_POWER_ROUTERS as u16
    ));
    assert!(!utils::is_broadcast_address(0x0f30));
}

#[test]
#[ignore = "not implemented yet"]
fn converts_eui64_le_buffer_to_0x() {
    let buffer = [0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11];

    assert_eq!(utils::eui64_le_buffer_to_hex(&buffer), "0x1122334455667788");
    // reference not reversed
    assert_eq!(
        buffer
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        "8877665544332211"
    );
}

#[test]
#[ignore = "not implemented yet"]
fn converts_eui64_be_buffer_to_0x() {
    let buffer = [0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11];

    assert_eq!(utils::eui64_be_buffer_to_hex(&buffer), "0x8877665544332211");
}

#[test]
#[ignore = "not implemented yet"]
fn calculates_crc_variants() {
    // see https://www.crccalc.com/
    let val1 = &hex_to_bytes("83FED3407A939723A5C639FF4C12")[..12]; // crc-appended

    assert_eq!(utils::crc16_x25(val1), 0x124c);
    assert_eq!(utils::crc16_xmodem(val1), 0xc4b8);
    assert_eq!(utils::crc16_ccitt(val1), 0x7292);
    assert_eq!(utils::crc16_ccitt_false(val1), 0x4041);

    let val2 = &hex_to_bytes("83FED3407A939723A5C639B26916D505C3B5")[..18]; // crc-appended

    assert_eq!(utils::crc16_x25(val2), 0xb5c3);
    assert_eq!(utils::crc16_xmodem(val2), 0xff08);
    assert_eq!(utils::crc16_ccitt(val2), 0x1a6a);
    assert_eq!(utils::crc16_ccitt_false(val2), 0x9502);
}

#[test]
#[ignore = "not implemented yet"]
fn hashes_using_aes_128_mmo() {
    let val1 = hex_to_bytes("83FED3407A939723A5C639FF4C12");
    // Example from Zigbee spec
    let val2 = hex_to_bytes("83FED3407A939723A5C639B26916D505C3B5");
    // Example from Zigbee spec C.6.1
    let val3 = hex_to_bytes("76777475727370717E7F7C7D7A7B7879C0");
    // Example from Zigbee spec C.6.1
    let val4 = hex_to_bytes("1C1D1E1F18191A1B14151617101112133C3D537529A7A9A03F669DCD886CB52C");

    assert_eq!(
        utils::aes128_mmo_hash(&val1),
        hex_to_bytes("58C1828CF7F1C3FE29E7B1024AD84BFA")
    );
    assert_eq!(
        utils::aes128_mmo_hash(&val2),
        hex_to_bytes("66B6900981E1EE3CA4206B6B861C02BB")
    );
    assert_eq!(
        utils::aes128_mmo_hash(&val3),
        hex_to_bytes("3C3D537529A7A9A03F669DCD886CB52C")
    );
    assert_eq!(
        utils::aes128_mmo_hash(&val4),
        hex_to_bytes("4512807BF94CB3400F0E2C25FB76E999")
    );
}
