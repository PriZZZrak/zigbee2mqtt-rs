// Original: .upstream/zigbee-herdsman/src/adapter/ember/utils/math.ts

pub fn inc8(n: u8) -> u8 {
    n.wrapping_add(1) & 7
}

pub fn within_range(lo: u8, n: u8, hi: u8) -> bool {
    n.wrapping_sub(lo) & 7 <= hi.wrapping_sub(lo) & 7
}

/// Upstream `halCommonCrc16`.
pub fn hal_common_crc16(new_byte: u8, prev_result: u16) -> u16 {
    let mut result = u32::from(prev_result);
    result = ((result >> 8) & 0xffff) | ((result << 8) & 0xffff);
    result ^= u32::from(new_byte);
    result ^= (result & 0xff) >> 4;
    result ^= (((result << 8) & 0xffff) << 4) & 0xffff;
    result ^= (((result & 0xff) << 5) & 0xff) | (((((result & 0xff) >> 3) & 0xffff) << 8) & 0xffff);
    result as u16
}

pub fn low_bits(n: u8) -> u8 {
    n & 0xf
}

pub fn high_bits(n: u8) -> u8 {
    (n >> 4) & 0xf
}

pub fn low_byte(n: u16) -> u8 {
    (n & 0xff) as u8
}

pub fn high_byte(n: u16) -> u8 {
    ((n >> 8) & 0xff) as u8
}

pub fn high_low_to_int(high: u16, low: u16) -> u32 {
    (u32::from(high & 0xffff) << 8) + u32::from(low & 0xff)
}

pub fn bit(x: u8) -> u32 {
    1u32 << x
}

pub fn bit32(x: u8) -> i32 {
    (1u32 << x) as i32
}

pub fn low_high_bytes(n: u16) -> (u8, u8) {
    (low_byte(n), high_byte(n))
}

pub fn low_high_bits(n: u8) -> (u8, u8) {
    (low_bits(n), high_bits(n))
}

pub fn byte_to_bits(n: u8) -> String {
    format!("{n:08b}")
}
