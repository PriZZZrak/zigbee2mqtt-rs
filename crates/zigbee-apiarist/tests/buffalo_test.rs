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
#[ignore = "not implemented yet"]
fn buffer_read() {
        let mut buffalo = Buffalo::new(vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09], 2);
        let value = buffalo.read_buffer(5);
        assert_eq!(buffalo.get_position(), 7);
        assert_eq!(value, vec![0x02, 0x03, 0x04, 0x05, 0x06]);
}

#[test]
#[ignore = "not implemented yet"]
fn buffer_write() {
        let mut buffalo = Buffalo::new(vec![0; 5], 1);
        let payload = vec![0x00, 0x01, 0x02];
        buffalo.write_buffer(&payload, payload.len());
        assert_eq!(buffalo.get_position(), 4);
        assert_eq!(buffalo.get_buffer(), vec![0x00, 0x00, 0x01, 0x02, 0x00]);
}

#[test]
#[ignore = "not implemented yet"]
fn buffer_write_as_array() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn doublele_read() {
        let mut buffalo = Buffalo::new(vec![0x33, 0x33, 0x33, 0x33, 0x31, 0xb7, 0x28, 0x41], 0);
        let value = buffalo.read_double_le();
        assert_eq!(buffalo.get_position(), 8);
        assert_eq!(value, 809880.6);
}

#[test]
#[ignore = "not implemented yet"]
fn doublele_write() {
        let mut buffalo = Buffalo::new(vec![0; 8], 0);
        buffalo.write_double_le(809880.6);
        assert_eq!(buffalo.get_position(), 8);
        assert_eq!(buffalo.get_buffer(), vec![0x33, 0x33, 0x33, 0x33, 0x31, 0xb7, 0x28, 0x41]);
}

#[test]
#[ignore = "not implemented yet"]
fn floatle_read() {
        let mut buffalo = Buffalo::new(vec![0x00, 0x40, 0xc8, 0x43], 0);
        let value = buffalo.read_float_le();
        assert_eq!(buffalo.get_position(), 4);
        assert_eq!(value, 400.5);
}

#[test]
#[ignore = "not implemented yet"]
fn floatle_write() {
        let mut buffalo = Buffalo::new(vec![0; 4], 0);
        buffalo.write_float_le(400.5);
        assert_eq!(buffalo.get_position(), 4);
        assert_eq!(buffalo.get_buffer(), vec![0x00, 0x40, 0xc8, 0x43]);
}

#[test]
#[ignore = "not implemented yet"]
fn ieeeaddr_read() {
        let mut buffalo = Buffalo::new(vec![0x00, 0x12, 0x4b, 0x00, 0x12, 0x01, 0x44, 0xae], 0);
        let value = buffalo.read_ieee_addr();
        assert_eq!(buffalo.get_position(), 8);
        assert_eq!(value, "0xae440112004b1200");
}

#[test]
#[ignore = "not implemented yet"]
fn ieeeaddr_write() {
        let mut buffalo = Buffalo::new(vec![0; 8], 0);
        buffalo.write_ieee_addr("0xae440112004b1200");
        assert_eq!(buffalo.get_position(), 8);
        assert_eq!(buffalo.get_buffer(), vec![0x00, 0x12, 0x4b, 0x00, 0x12, 0x01, 0x44, 0xae]);
}

#[test]
#[ignore = "not implemented yet"]
fn int16_read() {
        let mut buffalo = Buffalo::new(vec![0x00, 0xff, 0xff, 0x00], 1);
        let value = buffalo.read_int16();
        assert_eq!(buffalo.get_position(), 3);
        assert_eq!(value, -1);
}

#[test]
#[ignore = "not implemented yet"]
fn int16_write() {
        let mut buffalo = Buffalo::new(vec![0; 3], 1);
        buffalo.write_int16(256);
        assert_eq!(buffalo.get_position(), 3);
        assert_eq!(buffalo.get_buffer(), vec![0x00, 0x00, 0x01]);
}

#[test]
#[ignore = "not implemented yet"]
fn int24_read() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn int24_read_2() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn int24_write() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn int24_write_2() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn int32_read() {
        let mut buffalo = Buffalo::new(vec![0x01, 0x03, 0xff, 0xff], 0);
        let value = buffalo.read_int32();
        assert_eq!(buffalo.get_position(), 4);
        assert_eq!(value, -64767);
}

#[test]
#[ignore = "not implemented yet"]
fn int32_write() {
        let mut buffalo = Buffalo::new(vec![0; 6], 2);
        buffalo.write_int32(1065283);
        assert_eq!(buffalo.get_position(), 6);
        assert_eq!(buffalo.get_buffer(), vec![0x00, 0x00, 0x43, 0x41, 0x10, 0x00]);
}

#[test]
#[ignore = "not implemented yet"]
fn int40_read() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn int40_write() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn int48_read() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn int48_write() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn int64_read() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn int64_write() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn int8_read() {
        let mut buffalo = Buffalo::new(vec![0x00, 0xf0, 0x00, 0x00], 1);
        let value = buffalo.read_int8();
        assert_eq!(buffalo.get_position(), 2);
        assert_eq!(value, -16);
}

#[test]
#[ignore = "not implemented yet"]
fn int8_write() {
        let mut buffalo = Buffalo::new(vec![0; 3], 1);
        buffalo.write_int8(127);
        assert_eq!(buffalo.get_position(), 2);
        assert_eq!(buffalo.get_buffer(), vec![0x00, 0x7f, 0x00]);
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint16_read() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint16_write() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint24_read() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint24_write() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint32_read() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint32_write() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint8_read() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint8_write() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uint16_read() {
        let mut buffalo = Buffalo::new(vec![0x00, 0x03, 0xff, 0x00], 1);
        let value = buffalo.read_uint16();
        assert_eq!(buffalo.get_position(), 3);
        assert_eq!(value, 65283);
}

#[test]
#[ignore = "not implemented yet"]
fn uint16_write() {
        let mut buffalo = Buffalo::new(vec![0; 3], 1);
        buffalo.write_uint16(1020);
        assert_eq!(buffalo.get_position(), 3);
        assert_eq!(buffalo.get_buffer(), vec![0x00, 0xfc, 0x03]);
}

#[test]
#[ignore = "not implemented yet"]
fn uint24_read() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uint24_write() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uint32_read() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uint32_write() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uint40_read() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uint40_write() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uint48_read() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uint48_write() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uint56_read() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uint56_write() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uint64_read() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uint64_write() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uint8_read() {
        let mut buffalo = Buffalo::new(vec![0x00, 0x03, 0x00, 0x00], 1);
        let value = buffalo.read_uint8();
        assert_eq!(buffalo.get_position(), 2);
        assert_eq!(value, 3);
}

#[test]
#[ignore = "not implemented yet"]
fn uint8_write() {
        let mut buffalo = Buffalo::new(vec![0; 3], 1);
        buffalo.write_uint8(240);
        assert_eq!(buffalo.get_position(), 2);
        assert_eq!(buffalo.get_buffer(), vec![0x00, 0xf0, 0x00]);
}

#[test]
#[ignore = "not implemented yet"]
fn utf8_string_read() {
        let mut buffalo = Buffalo::new(vec![0x61, 0x62, 0x63, 0x64], 0);
        let value = buffalo.read_utf8_string(4);
        assert_eq!(buffalo.get_position(), 4);
        assert_eq!(value, "abcd");
}

#[test]
#[ignore = "not implemented yet"]
fn utf8_string_write() {
        let mut buffalo = Buffalo::new(vec![0; 4], 0);
        buffalo.write_utf8_string("abcd");
        assert_eq!(buffalo.get_position(), 4);
        assert_eq!(buffalo.get_buffer(), vec![0x61, 0x62, 0x63, 0x64]);
}

#[test]
#[ignore = "not implemented yet"]
fn gets_written() {
        let mut buffalo = Buffalo::new(vec![0; 3], 1);
        buffalo.write_uint8(240);
        assert_eq!(buffalo.get_written(), vec![0, 240]);
}

#[test]
#[ignore = "not implemented yet"]
fn is_more() {
        let mut buffalo = Buffalo::new(vec![0, 1, 2], 1);
        assert_eq!(buffalo.get_position(), 1);
        assert!(buffalo.is_more());
        buffalo.read_uint16();
        assert!(!buffalo.is_more());
}
