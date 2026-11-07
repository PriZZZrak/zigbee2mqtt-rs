// Original: .upstream/zigbee-herdsman/test/buffalo.test.ts

use zigbee_apiarist::buffalo::buffalo::Buffalo;
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - BUFFER read
//   - BUFFER write
//   - BUFFER write as array
//   - Buffalo
//   - DOUBLELE read
//   - DOUBLELE write
//   - FLOATLE read
//   - FLOATLE write
//   - IEEEADDR read
//   - IEEEADDR write
//   - INT16 read
//   - INT16 write
//   - INT24 read +
//   - INT24 read -
//   - INT24 write +
//   - INT24 write -
//   - INT32 read
//   - INT32 write
//   - INT40 read
//   - INT40 write
//   - INT48 read
//   - INT48 write
//   - INT64 read
//   - INT64 write
//   - INT8 read
//   - INT8 write
//   - LIST_UINT16 read
//   - LIST_UINT16 write
//   - LIST_UINT24 read
//   - LIST_UINT24 write
//   - LIST_UINT32 read
//   - LIST_UINT32 write
//   - LIST_UINT8 read
//   - LIST_UINT8 write
//   - UINT16 read
//   - UINT16 write
//   - UINT24 read
//   - UINT24 write
//   - UINT32 read
//   - UINT32 write
//   - UINT40 read
//   - UINT40 write
//   - UINT48 read
//   - UINT48 write
//   - UINT56 read
//   - UINT56 write
//   - UINT64 read
//   - UINT64 write
//   - UINT8 read
//   - UINT8 write
//   - UTF8 STRING read
//   - UTF8 STRING write
//   - gets written
//   - is more

#[test]
fn buffer_read() {
    let mut buffalo = Buffalo::new(
        vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09],
        2,
    );
    let value = buffalo.read_buffer(5);
    assert_eq!(buffalo.get_position(), 7);
    assert_eq!(value, vec![0x02, 0x03, 0x04, 0x05, 0x06]);
}

#[test]
fn buffer_write() {
    let mut buffalo = Buffalo::new(vec![0; 5], 1);
    let payload = vec![0x00, 0x01, 0x02];
    buffalo.write_buffer(&payload, payload.len());
    assert_eq!(buffalo.get_position(), 4);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0x00, 0x01, 0x02, 0x00]);
}

#[test]
fn buffer_write_as_array() {
    let mut buffalo = Buffalo::new(vec![0; 5], 1);
    let payload = vec![0x00, 0x01, 0x02];
    buffalo.write_buffer(&payload, payload.len());
    assert_eq!(buffalo.get_position(), 4);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0x00, 0x01, 0x02, 0x00]);
}

#[test]
#[test]
fn doublele_read() {
    let mut buffalo = Buffalo::new(vec![0x33, 0x33, 0x33, 0x33, 0x31, 0xb7, 0x28, 0x41], 0);
    let value = buffalo.read_double_le();
    assert_eq!(buffalo.get_position(), 8);
    assert_eq!(value, 809880.6);
}

#[test]
fn doublele_write() {
    let mut buffalo = Buffalo::new(vec![0; 8], 0);
    buffalo.write_double_le(809880.6);
    assert_eq!(buffalo.get_position(), 8);
    assert_eq!(
        buffalo.get_buffer(),
        vec![0x33, 0x33, 0x33, 0x33, 0x31, 0xb7, 0x28, 0x41]
    );
}

#[test]
fn floatle_read() {
    let mut buffalo = Buffalo::new(vec![0x00, 0x40, 0xc8, 0x43], 0);
    let value = buffalo.read_float_le();
    assert_eq!(buffalo.get_position(), 4);
    assert_eq!(value, 400.5);
}

#[test]
fn floatle_write() {
    let mut buffalo = Buffalo::new(vec![0; 4], 0);
    buffalo.write_float_le(400.5);
    assert_eq!(buffalo.get_position(), 4);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0x40, 0xc8, 0x43]);
}

#[test]
fn ieeeaddr_read() {
    let mut buffalo = Buffalo::new(vec![0x00, 0x12, 0x4b, 0x00, 0x12, 0x01, 0x44, 0xae], 0);
    let value = buffalo.read_ieee_addr();
    assert_eq!(buffalo.get_position(), 8);
    assert_eq!(value, "0xae440112004b1200");
}

#[test]
fn ieeeaddr_write() {
    let mut buffalo = Buffalo::new(vec![0; 8], 0);
    buffalo.write_ieee_addr("0xae440112004b1200");
    assert_eq!(buffalo.get_position(), 8);
    assert_eq!(
        buffalo.get_buffer(),
        vec![0x00, 0x12, 0x4b, 0x00, 0x12, 0x01, 0x44, 0xae]
    );
}

#[test]
fn int16_read() {
    let mut buffalo = Buffalo::new(vec![0x00, 0xff, 0xff, 0x00], 1);
    let value = buffalo.read_int16();
    assert_eq!(buffalo.get_position(), 3);
    assert_eq!(value, -1);
}

#[test]
fn int16_write() {
    let mut buffalo = Buffalo::new(vec![0; 3], 1);
    buffalo.write_int16(256);
    assert_eq!(buffalo.get_position(), 3);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0x00, 0x01]);
}

#[test]
fn int24_read() {
    let mut buffalo = Buffalo::new(vec![0x7f, 0xff, 0xff, 0x7f], 1);
    let value = buffalo.read_int24();
    assert_eq!(buffalo.get_position(), 4);
    assert_eq!(value, 8388607);
}

#[test]
fn int24_read_2() {
    let mut buffalo = Buffalo::new(vec![0x00, 0x00, 0xff], 0);
    let value = buffalo.read_int24();
    assert_eq!(buffalo.get_position(), 3);
    assert_eq!(value, -65536);
}

#[test]
fn int24_write() {
    let mut buffalo = Buffalo::new(vec![0; 3], 0);
    buffalo.write_int24(65536);
    assert_eq!(buffalo.get_position(), 3);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0x00, 0x01]);
}

#[test]
fn int24_write_2() {
    let mut buffalo = Buffalo::new(vec![0; 3], 0);
    buffalo.write_int24(-65536);
    assert_eq!(buffalo.get_position(), 3);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0x00, 0xff]);
}

#[test]
fn int32_read() {
    let mut buffalo = Buffalo::new(vec![0x01, 0x03, 0xff, 0xff], 0);
    let value = buffalo.read_int32();
    assert_eq!(buffalo.get_position(), 4);
    assert_eq!(value, -64767);
}

#[test]
fn int32_write() {
    let mut buffalo = Buffalo::new(vec![0; 6], 2);
    buffalo.write_int32(1065283);
    assert_eq!(buffalo.get_position(), 6);
    assert_eq!(
        buffalo.get_buffer(),
        vec![0x00, 0x00, 0x43, 0x41, 0x10, 0x00]
    );
}

#[test]
fn int40_read() {
    let mut buffalo = Buffalo::new(vec![1, 2, 3, 4, 5], 0);
    let value = buffalo.read_int40();
    assert_eq!(buffalo.get_position(), 5);
    assert_eq!(value, 21542142465);
}

#[test]
fn int40_write() {
    let mut buffalo = Buffalo::new(vec![0; 7], 2);
    buffalo.write_int40(21542142465);
    assert_eq!(buffalo.get_position(), 7);
    assert_eq!(buffalo.get_buffer(), vec![0, 0, 1, 2, 3, 4, 5]);
}

#[test]
fn int48_read() {
    let mut buffalo = Buffalo::new(vec![0x00, 0xff, 0x03, 0xff, 0x03, 0xff], 0);
    let value = buffalo.read_int48();
    assert_eq!(buffalo.get_position(), 6);
    assert_eq!(value, -1082348273920);
}

#[test]
fn int48_write() {
    let mut buffalo = Buffalo::new(vec![0; 6], 0);
    buffalo.write_int48(-1082348273920);
    assert_eq!(buffalo.get_position(), 6);
    assert_eq!(
        buffalo.get_buffer(),
        vec![0x00, 0xff, 0x03, 0xff, 0x03, 0xff]
    );
}

#[test]
fn int64_read() {
    let mut buffalo = Buffalo::new(vec![254, 255, 255, 255, 255, 255, 255, 127], 0);
    let value = buffalo.read_int64();
    assert_eq!(buffalo.get_position(), 8);
    assert_eq!(value, 9223372036854775806);
}

#[test]
fn int64_write() {
    let mut buffalo = Buffalo::new(vec![0; 10], 2);
    buffalo.write_int64(9223372036854775806);
    assert_eq!(buffalo.get_position(), 10);
    assert_eq!(
        buffalo.get_buffer(),
        vec![0, 0, 254, 255, 255, 255, 255, 255, 255, 127]
    );
}

#[test]
fn int8_read() {
    let mut buffalo = Buffalo::new(vec![0x00, 0xf0, 0x00, 0x00], 1);
    let value = buffalo.read_int8();
    assert_eq!(buffalo.get_position(), 2);
    assert_eq!(value, -16);
}

#[test]
fn int8_write() {
    let mut buffalo = Buffalo::new(vec![0; 3], 1);
    buffalo.write_int8(127);
    assert_eq!(buffalo.get_position(), 2);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0x7f, 0x00]);
}

#[test]
fn list_uint16_read() {
    let mut buffalo = Buffalo::new(vec![0x00, 0x00, 0x04, 0x00, 0x08], 1);
    let value = buffalo.read_list_uint16(2);
    assert_eq!(buffalo.get_position(), 5);
    assert_eq!(value, vec![1024, 2048]);
}

#[test]
fn list_uint16_write() {
    let mut buffalo = Buffalo::new(vec![0; 5], 1);
    let payload = vec![1024, 2048];
    buffalo.write_list_uint16(&payload);
    assert_eq!(buffalo.get_position(), 5);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0x00, 0x04, 0x00, 0x08]);
}

#[test]
fn list_uint24_read() {
    let mut buffalo = Buffalo::new(vec![0x00, 0xa0, 0x86, 0x01, 0x1e, 0xae, 0x01, 0x00], 1);
    let value = buffalo.read_list_uint24(2);
    assert_eq!(buffalo.get_position(), 7);
    assert_eq!(value, vec![100000, 110110]);
}

#[test]
fn list_uint24_write() {
    let mut buffalo = Buffalo::new(vec![0; 8], 1);
    let payload = vec![100000, 110110];
    buffalo.write_list_uint24(&payload);
    assert_eq!(buffalo.get_position(), 7);
    assert_eq!(
        buffalo.get_buffer(),
        vec![0x00, 0xa0, 0x86, 0x01, 0x1e, 0xae, 0x01, 0x00]
    );
}

#[test]
fn list_uint32_read() {
    let mut buffalo = Buffalo::new(vec![0xff, 0xff, 0xff, 0xff, 0x0a, 0x00, 0x00, 0x00], 0);
    let value = buffalo.read_list_uint32(2);
    assert_eq!(buffalo.get_position(), 8);
    assert_eq!(value, vec![4294967295, 10]);
}

#[test]
fn list_uint32_write() {
    let mut buffalo = Buffalo::new(vec![0; 8], 0);
    let payload = vec![4294967295, 10];
    buffalo.write_list_uint32(&payload);
    assert_eq!(buffalo.get_position(), 8);
    assert_eq!(
        buffalo.get_buffer(),
        vec![0xff, 0xff, 0xff, 0xff, 0x0a, 0x00, 0x00, 0x00]
    );
}

#[test]
fn list_uint8_read() {
    let mut buffalo = Buffalo::new(vec![0x00, 0x00, 0x04, 0x08], 2);
    let value = buffalo.read_list_uint8(2);
    assert_eq!(buffalo.get_position(), 4);
    assert_eq!(value, vec![4, 8]);
}

#[test]
fn list_uint8_write() {
    let mut buffalo = Buffalo::new(vec![0; 4], 1);
    let payload = vec![200, 100];
    buffalo.write_list_uint8(&payload);
    assert_eq!(buffalo.get_position(), 3);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0xc8, 0x64, 0x00]);
}

#[test]
fn uint16_read() {
    let mut buffalo = Buffalo::new(vec![0x00, 0x03, 0xff, 0x00], 1);
    let value = buffalo.read_uint16();
    assert_eq!(buffalo.get_position(), 3);
    assert_eq!(value, 65283);
}

#[test]
fn uint16_write() {
    let mut buffalo = Buffalo::new(vec![0; 3], 1);
    buffalo.write_uint16(1020);
    assert_eq!(buffalo.get_position(), 3);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0xfc, 0x03]);
}

#[test]
fn uint24_read() {
    let mut buffalo = Buffalo::new(vec![0x01, 0xa0, 0x86, 0x01, 0x02], 1);
    let value = buffalo.read_uint24();
    assert_eq!(buffalo.get_position(), 4);
    assert_eq!(value, 100000);
}

#[test]
fn uint24_write() {
    let mut buffalo = Buffalo::new(vec![0; 4], 1);
    buffalo.write_uint24(16777200);
    assert_eq!(buffalo.get_position(), 4);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0xf0, 0xff, 0xff]);
}

#[test]
fn uint32_read() {
    let mut buffalo = Buffalo::new(vec![0x01, 0x03, 0xff, 0xff], 0);
    let value = buffalo.read_uint32();
    assert_eq!(buffalo.get_position(), 4);
    assert_eq!(value, 4294902529);
}

#[test]
fn uint32_write() {
    let mut buffalo = Buffalo::new(vec![0; 6], 2);
    buffalo.write_uint32(1065283);
    assert_eq!(buffalo.get_position(), 6);
    assert_eq!(
        buffalo.get_buffer(),
        vec![0x00, 0x00, 0x43, 0x41, 0x10, 0x00]
    );
}

#[test]
fn uint40_read() {
    let mut buffalo = Buffalo::new(vec![1, 2, 3, 4, 5], 0);
    let value = buffalo.read_uint40();
    assert_eq!(buffalo.get_position(), 5);
    assert_eq!(value, 21542142465);
}

#[test]
fn uint40_write() {
    let mut buffalo = Buffalo::new(vec![0; 7], 2);
    buffalo.write_uint40(21542142465);
    assert_eq!(buffalo.get_position(), 7);
    assert_eq!(buffalo.get_buffer(), vec![0, 0, 1, 2, 3, 4, 5]);
}

#[test]
fn uint48_read() {
    let mut buffalo = Buffalo::new(vec![1, 2, 3, 4, 5, 6], 0);
    let value = buffalo.read_uint48();
    assert_eq!(buffalo.get_position(), 6);
    assert_eq!(value, 6618611909121);
}

#[test]
fn uint48_write() {
    let mut buffalo = Buffalo::new(vec![0; 8], 2);
    buffalo.write_uint48(6618611909121);
    assert_eq!(buffalo.get_position(), 8);
    assert_eq!(buffalo.get_buffer(), vec![0, 0, 1, 2, 3, 4, 5, 6]);
}

#[test]
fn uint56_read() {
    let mut buffalo = Buffalo::new(vec![254, 255, 255, 255, 255, 255, 255], 0);
    let value = buffalo.read_uint56();
    assert_eq!(buffalo.get_position(), 7);
    assert_eq!(value, 72057594037927934);
}

#[test]
fn uint56_write() {
    let mut buffalo = Buffalo::new(vec![0; 9], 2);
    buffalo.write_uint56(72057594037927934);
    assert_eq!(buffalo.get_position(), 9);
    assert_eq!(
        buffalo.get_buffer(),
        vec![0, 0, 254, 255, 255, 255, 255, 255, 255]
    );
}

#[test]
fn uint64_read() {
    let mut buffalo = Buffalo::new(vec![253, 255, 255, 255, 255, 255, 255, 255], 0);
    let value = buffalo.read_uint64();
    assert_eq!(buffalo.get_position(), 8);
    assert_eq!(value, 18446744073709551613);
}

#[test]
fn uint64_write() {
    let mut buffalo = Buffalo::new(vec![0; 10], 2);
    buffalo.write_uint64(18446744073709551613);
    assert_eq!(buffalo.get_position(), 10);
    assert_eq!(
        buffalo.get_buffer(),
        vec![0, 0, 253, 255, 255, 255, 255, 255, 255, 255]
    );
}

#[test]
fn uint8_read() {
    let mut buffalo = Buffalo::new(vec![0x00, 0x03, 0x00, 0x00], 1);
    let value = buffalo.read_uint8();
    assert_eq!(buffalo.get_position(), 2);
    assert_eq!(value, 3);
}

#[test]
fn uint8_write() {
    let mut buffalo = Buffalo::new(vec![0; 3], 1);
    buffalo.write_uint8(240);
    assert_eq!(buffalo.get_position(), 2);
    assert_eq!(buffalo.get_buffer(), vec![0x00, 0xf0, 0x00]);
}

#[test]
fn utf8_string_read() {
    let mut buffalo = Buffalo::new(vec![0x61, 0x62, 0x63, 0x64], 0);
    let value = buffalo.read_utf8_string(4);
    assert_eq!(buffalo.get_position(), 4);
    assert_eq!(value, "abcd");
}

#[test]
fn utf8_string_write() {
    let mut buffalo = Buffalo::new(vec![0; 4], 0);
    buffalo.write_utf8_string("abcd");
    assert_eq!(buffalo.get_position(), 4);
    assert_eq!(buffalo.get_buffer(), vec![0x61, 0x62, 0x63, 0x64]);
}

#[test]
fn gets_written() {
    let mut buffalo = Buffalo::new(vec![0; 3], 1);
    buffalo.write_uint8(240);
    assert_eq!(buffalo.get_written(), vec![0, 240]);
}

#[test]
fn is_more() {
    let mut buffalo = Buffalo::new(vec![0, 1, 2], 1);
    assert_eq!(buffalo.get_position(), 1);
    assert!(buffalo.is_more());
    buffalo.read_uint16();
    assert!(!buffalo.is_more());
}
