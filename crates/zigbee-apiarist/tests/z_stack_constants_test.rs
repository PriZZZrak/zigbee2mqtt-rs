// Port of .upstream/zigbee-herdsman/test/adapter/z-stack/constants.test.ts

use zigbee_apiarist::adapter::z_stack::constants::{af, utils};

#[test]
fn simple_check() {
    assert_eq!(af::DEFAULT_RADIUS, 30);
}

#[test]
fn get_channel_mask_11() {
    assert_eq!(utils::get_channel_mask(&[11]), vec![0, 8, 0, 0]);
}

#[test]
fn get_channel_mask_25() {
    assert_eq!(utils::get_channel_mask(&[25]), vec![0, 0, 0, 2]);
}

#[test]
fn get_channel_mask_11_and_25() {
    assert_eq!(utils::get_channel_mask(&[11, 25]), vec![0, 8, 0, 2]);
}

#[test]
fn formats_known_status() {
    assert_eq!(utils::status_description(0x10), "(0x10: MEM_ERROR)");
}

#[test]
fn formats_unknown_status() {
    assert_eq!(utils::status_description(0x08), "(0x08: UNKNOWN)");
}
