// Port of .upstream/zigbee-herdsman/test/adapter/ember/math.test.ts

use zigbee_apiarist::adapter::ember::utils::math as m;

const ASH_CRC_INIT: u16 = 0xffff;
const B32: u32 = 0xbeefface;

#[test]
#[ignore = "not implemented yet"]
fn inc8() {
    assert_eq!(m::inc8(0x00), 1);
    assert_eq!(m::inc8(0x03), 4);
    assert_eq!(m::inc8(0x07), 0);
    assert_eq!(m::inc8(0x08), 1);
    assert_eq!(m::inc8(0x10), 1);
    assert_eq!(m::inc8(0xfe), 7);
    assert_eq!(m::inc8(0xff), 0);
}

#[test]
#[ignore = "not implemented yet"]
fn within_range() {
    assert!(m::within_range(0x00, 0x04, 0x07));
    assert!(!m::within_range(0x00, 0x04, 0x08));
    assert!(m::within_range(0xaa, 0xac, 0xfe));
    assert!(!m::within_range(0x00, 0x04, 0xf8));
}

#[test]
#[ignore = "not implemented yet"]
fn hal_common_crc16() {
    let mut t = m::hal_common_crc16(0x00, ASH_CRC_INIT);
    assert_eq!(t, 57840);
    t = m::hal_common_crc16(0x03, t);
    assert_eq!(t, 11628);
    t = m::hal_common_crc16(0xfe, t);
    assert_eq!(t, 38686);
    t = m::hal_common_crc16(0xa5, t);
    assert_eq!(t, 2065);
}

#[test]
#[ignore = "not implemented yet"]
fn low_bits() {
    assert_eq!(m::low_bits(10), 10);
    assert_eq!(m::low_bits(100), 4);
}

#[test]
#[ignore = "not implemented yet"]
fn high_bits() {
    assert_eq!(m::high_bits(10), 0);
    assert_eq!(m::high_bits(100), 6);
}

#[test]
#[ignore = "not implemented yet"]
fn low_byte() {
    assert_eq!(m::low_byte(10), 10);
    assert_eq!(m::low_byte(100), 100);
    assert_eq!(m::low_byte(1000), 232);
    assert_eq!(m::low_byte(255), 255);
    assert_eq!(m::low_byte(1024), 0);
    assert_eq!(m::low_byte(B32 as u16), 206);
}

#[test]
#[ignore = "not implemented yet"]
fn high_byte() {
    assert_eq!(m::high_byte(10), 0);
    assert_eq!(m::high_byte(100), 0);
    assert_eq!(m::high_byte(1000), 3);
    assert_eq!(m::high_byte(255), 0);
    assert_eq!(m::high_byte(1024), 4);
    assert_eq!(m::high_byte(B32 as u16), 250);
}

#[test]
#[ignore = "not implemented yet"]
fn high_low_to_int() {
    assert_eq!(m::high_low_to_int(254, 10), 65034);
    assert_eq!(m::high_low_to_int(10, 100), 2660);
    assert_eq!(m::high_low_to_int(1000, 2000), 256208);
    assert_eq!(m::high_low_to_int(355, 255), 91135);
    assert_eq!(m::high_low_to_int(123, 1024), 31488);
    assert_eq!(m::high_low_to_int(1, B32 as u16), 462);
    assert_eq!(m::high_low_to_int(B32 as u16, 1), 16436737);
}

#[test]
#[ignore = "not implemented yet"]
fn bit() {
    assert_eq!(m::bit(11), 2048);
    assert_eq!(m::bit(15), 32768);
    assert_eq!(m::bit(26), 67108864);
    assert_eq!(m::bit(11) | m::bit(15) | m::bit(20) | m::bit(25), 34637824);
    assert_eq!(
        m::bit(12) | m::bit(13) | m::bit(14) | m::bit(16) | m::bit(17) | m::bit(18)
            | m::bit(19) | m::bit(21) | m::bit(22) | m::bit(23) | m::bit(24) | m::bit(26),
        99577856
    );
    assert_eq!(53 & m::bit(0), 1);
    assert_eq!(53 & m::bit(3), 0);
    assert_eq!(53 | m::bit(0), 53);
    assert_eq!(53 | m::bit(3), 61);
}

#[test]
#[ignore = "not implemented yet"]
fn bit32() {
    assert_eq!(m::bit32(11), 2048);
    assert_eq!(m::bit32(15), 32768);
    assert_eq!(m::bit32(26), 67108864);
    assert_eq!(m::bit32(11) | m::bit32(15) | m::bit32(20) | m::bit32(25), 34637824);
    assert_eq!(
        m::bit32(12) | m::bit32(13) | m::bit32(14) | m::bit32(16) | m::bit32(17)
            | m::bit32(18) | m::bit32(19) | m::bit32(21) | m::bit32(22) | m::bit32(23)
            | m::bit32(24) | m::bit32(26),
        99577856
    );
    // Signed 32-bit edge cases from upstream are JS-specific; kept as comments:
    // B32 & bit32(0) == 0, B32 & bit32(3) == 8,
    // B32 | bit32(0) == -1091568945, B32 | bit32(3) == -1091568946.
}

#[test]
#[ignore = "not implemented yet"]
fn low_high_bytes() {
    let (l, h) = m::low_high_bytes(1024);
    assert_eq!(l, 0);
    assert_eq!(h, 4);
    assert_eq!(l, m::low_byte(1024));
    assert_eq!(h, m::high_byte(1024));
    let (l, h) = m::low_high_bytes(255);
    assert_eq!(l, 255);
    assert_eq!(h, 0);
}

#[test]
#[ignore = "not implemented yet"]
fn low_high_bits() {
    let (l, h) = m::low_high_bits(10);
    assert_eq!(l, 10);
    assert_eq!(h, 0);
    assert_eq!(l, m::low_bits(10));
    assert_eq!(h, m::high_bits(0));
    let (l, h) = m::low_high_bits(100);
    assert_eq!(l, 4);
    assert_eq!(h, 6);
}

#[test]
#[ignore = "not implemented yet"]
fn byte_to_bits() {
    assert_eq!(m::byte_to_bits(2), "00000010");
    assert_eq!(m::byte_to_bits(4), "00000100");
}
