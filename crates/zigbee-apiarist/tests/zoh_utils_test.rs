// Port of .upstream/zigbee-herdsman/test/adapter/zoh/utils.test.ts

use zigbee_apiarist::adapter::zoh::adapter::utils::{
    big_u_int64_to_buffer_be, big_u_int64_to_buffer_le, big_u_int64_to_hex_be,
};

#[test]
#[ignore = "not implemented yet"]
fn handles_bigint_conversions() {
    let v1_0x = "0x90395efffec7fd21";
    let v1_buf = vec![0x21, 0xfd, 0xc7, 0xfe, 0xff, 0x5e, 0x39, 0x90];
    let v1_big_int: u64 = 10392442068718320929;

    let v2_0x = "0x9986ffbb4523acef";
    let v2_buf = vec![0xef, 0xac, 0x23, 0x45, 0xbb, 0xff, 0x86, 0x99];
    let v2_big_int: u64 = 11062810714466135279;

    let v3_0x = "0x0322334455667788";
    let v3_buf = vec![0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x03];
    let v3_big_int: u64 = 225799299905517448;

    for (value, hex, buf) in [
        (v1_big_int, v1_0x, v1_buf),
        (v2_big_int, v2_0x, v2_buf),
        (v3_big_int, v3_0x, v3_buf),
    ] {
        assert_eq!(big_u_int64_to_hex_be(value), hex.trim_start_matches("0x"));
        assert_eq!(big_u_int64_to_buffer_le(value), buf);
        let mut reversed = buf.clone();
        reversed.reverse();
        assert_eq!(big_u_int64_to_buffer_be(value), reversed);
        assert_eq!(u64::from_str_radix(hex.trim_start_matches("0x"), 16).unwrap(), value);
    }
}
