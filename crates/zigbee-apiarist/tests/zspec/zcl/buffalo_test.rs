// Original: .upstream/zigbee-herdsman/test/zspec/zcl/buffalo.test.ts

use serde_json::json;
use zigbee_apiarist::zspec::zcl::buffalo_zcl::BuffaloZcl;
use zigbee_apiarist::zspec::zcl::definition::enums::{BuffaloZclDataType, DataType};
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - GPD Frame
//   - Reads Mi Struct
//   - Reads Mi Struct with padding
//   - Reads attribute report
//   - Reads attribute request
//   - Reads attribute response
//   - Reads channel request
//   - Reads commissioning
//   - Reads commissioning all options
//   - Reads invalid Tuya Data Point Values List as empty array
//   - Reads manufacturer-specific attribute report
//   - Reads manufacturer-specific multi-cluster attribute report
//   - Reads multi-cluster attribute report
//   - Reads nothing
//   - Reads partial buffer with length option
//   - Reads unhandled command as empty object if buffer finished reading
//   - Reads unhandled command as object[raw] and ignores GPP data/mic
//   - Reads unhandled command as object[raw] if buffer still has bytes to read
//   - Reads whole buffer without length option
//   - Throws when read Strctured Select indicator is outside range
//   - Throws when read Thermo Transitions List is missing required payload options
//   - Throws when read is missing payload.payloadSize option
//   - Throws when write Use Data Type is missing dataType option and value isnt buffer or number array
//   - Throws when write/read invalid data type, except for write if buffer-like
//   - Writes & Reads Date
//   - Writes & Reads Mi Boxer Zones List
//   - Writes & Reads Time of Day
//   - Writes & Reads Use Data Type as buffer when missing dataType option
//   - Writes & Reads big endian uint24
//   - Writes & Reads char str from string
//   - Writes & Reads long char str
//   - Writes & Reads long octet str
//   - Writes & Reads octet str
//   - Writes & Reads zone info list
//   - Writes channel configuration
//   - Writes channel configuration basic
//   - Writes char str from number array
//   - Writes commissioning
//   - Writes commissioning all options
//   - Writes custom reply
//   - Writes invalida data type as buffer if buffer-like
//   - Writes nothing
//   - Writes nothing for unhandled command
//   - ZCL Buffalo
//   - [workaround] Reads char str as Mi struct for Xiaomi attridId=65281

#[test]
#[ignore = "not implemented yet"]
fn gpd_frame() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_mi_struct() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_mi_struct_with_padding() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_attribute_report() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_attribute_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_attribute_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_channel_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_commissioning() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_commissioning_all_options() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_invalid_tuya_data_point_values_list_as_empty_array() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_manufacturer_specific_attribute_report() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_manufacturer_specific_multi_cluster_attribute_report() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_multi_cluster_attribute_report() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_nothing() {
        for data_type in [DataType::NO_DATA as u32, DataType::UNKNOWN as u32] {
            let buffer = vec![1, 2];
            let mut buffalo = BuffaloZcl::new(buffer);
            assert_eq!(buffalo.read(data_type, &json!({})), json!(null));
            assert_eq!(buffalo.get_position(), 0);
        }
    
        // what ZCL spec calls "empty string"
        let buffer = vec![0, 2, 3];
        let mut buffalo = BuffaloZcl::new(buffer);
        assert_eq!(buffalo.read(DataType::CHAR_STR as u32, &json!({})), json!(""));
        assert_eq!(buffalo.get_position(), 1);
    
        // what ZCL spec calls "empty string"
        let buffer = vec![0, 2, 3];
        let mut buffalo = BuffaloZcl::new(buffer);
        assert_eq!(buffalo.read(DataType::OCTET_STR as u32, &json!({})), json!([]));
        assert_eq!(buffalo.get_position(), 1);
}

#[test]
#[ignore = "not implemented yet"]
fn reads_partial_buffer_with_length_option() {
        let value = vec![1, 2, 3, 4];
        let length = 2;
        let mut buffalo = BuffaloZcl::new(vec![0; 255]);
        buffalo.write(
            BuffaloZclDataType::BUFFER as u32,
            &json!(value),
            &json!({"length": length}),
        );
        assert_eq!(buffalo.get_position(), value.len());
        buffalo.set_position(0);
        assert_eq!(
            buffalo.read(BuffaloZclDataType::BUFFER as u32, &json!({"length": length})),
            json!([value[0], value[1]])
        );
        assert_eq!(buffalo.get_position(), length);
}

#[test]
#[ignore = "not implemented yet"]
fn reads_unhandled_command_as_empty_object_if_buffer_finished_reading() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_unhandled_command_as_object_raw_and_ignores_gpp_data_mic() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_unhandled_command_as_object_raw_if_buffer_still_has_bytes_to_read() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_whole_buffer_without_length_option() {
        let value = vec![1, 2, 3, 4];
        let mut buffalo = BuffaloZcl::new(vec![0; 4]);
        buffalo.write(BuffaloZclDataType::BUFFER as u32, &json!(value), &json!({}));
        assert_eq!(buffalo.get_position(), value.len());
        buffalo.set_position(0);
        assert_eq!(buffalo.read(BuffaloZclDataType::BUFFER as u32, &json!({})), json!(value));
        assert_eq!(buffalo.get_position(), value.len());
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_read_strctured_select_indicator_is_outside_range() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_read_thermo_transitions_list_is_missing_required_payload_options() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_read_is_missing_payload_payload_size_option() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_write_use_data_type_is_missing_data_type_option_and_value_isnt_buffer_or_number_array() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_write_read_invalid_data_type_except_for_write_if_buffer_like() {
        let write_result = std::panic::catch_unwind(|| {
            let mut buffalo = BuffaloZcl::new(vec![0; 1]);
            buffalo.write(99999, &json!(127), &json!({}));
        });
        assert!(write_result.is_err());
    
        let read_result = std::panic::catch_unwind(|| {
            let mut buffalo = BuffaloZcl::new(vec![0; 1]);
            buffalo.read(99999, &json!({}));
        });
        assert!(read_result.is_err());
}

#[test]
#[ignore = "not implemented yet"]
fn writes_reads_date() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_reads_mi_boxer_zones_list() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_reads_time_of_day() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_reads_use_data_type_as_buffer_when_missing_data_type_option() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_reads_big_endian_uint24() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_reads_char_str_from_string() {
        let value = "abcd";
        let expected_value = vec![4, 0x61, 0x62, 0x63, 0x64];
        let expected_position = 5;
        let mut buffalo = BuffaloZcl::new(vec![0; 10]);
        buffalo.write(DataType::CHAR_STR as u32, &json!(value), &json!({}));
        assert_eq!(buffalo.get_position(), expected_position);
        assert_eq!(buffalo.get_written(), expected_value);
        buffalo.set_position(0);
        assert_eq!(buffalo.read(DataType::CHAR_STR as u32, &json!({})), json!(value));
        assert_eq!(buffalo.get_position(), expected_position);
}

#[test]
#[ignore = "not implemented yet"]
fn writes_reads_long_char_str() {
        let value = "abcd";
        let expected_value = vec![4, 0, 0x61, 0x62, 0x63, 0x64];
        let expected_position = 6;
        let mut buffalo = BuffaloZcl::new(vec![0; 10]);
        buffalo.write(DataType::LONG_CHAR_STR as u32, &json!(value), &json!({}));
        assert_eq!(buffalo.get_position(), expected_position);
        assert_eq!(buffalo.get_written(), expected_value);
        buffalo.set_position(0);
        assert_eq!(buffalo.read(DataType::LONG_CHAR_STR as u32, &json!({})), json!(value));
        assert_eq!(buffalo.get_position(), expected_position);
}

#[test]
#[ignore = "not implemented yet"]
fn writes_reads_long_octet_str() {
        let value = vec![0xfe, 0x01, 0xab, 0x98];
        let expected_position = 6;
        let mut buffalo = BuffaloZcl::new(vec![0; 10]);
        buffalo.write(DataType::LONG_OCTET_STR as u32, &json!(value), &json!({}));
        assert_eq!(buffalo.get_position(), expected_position);
        assert_eq!(buffalo.get_written(), vec![4, 0, 0xfe, 0x01, 0xab, 0x98]);
        buffalo.set_position(0);
        assert_eq!(buffalo.read(DataType::LONG_OCTET_STR as u32, &json!({})), json!(value));
        assert_eq!(buffalo.get_position(), expected_position);
}

#[test]
#[ignore = "not implemented yet"]
fn writes_reads_octet_str() {
        let value = vec![0xfe, 0x01, 0xab, 0x98];
        let expected_position = 5;
        let mut buffalo = BuffaloZcl::new(vec![0; 10]);
        buffalo.write(DataType::OCTET_STR as u32, &json!(value), &json!({}));
        assert_eq!(buffalo.get_position(), expected_position);
        assert_eq!(buffalo.get_written(), vec![4, 0xfe, 0x01, 0xab, 0x98]);
        buffalo.set_position(0);
        assert_eq!(buffalo.read(DataType::OCTET_STR as u32, &json!({})), json!(value));
        assert_eq!(buffalo.get_position(), expected_position);
}

#[test]
#[ignore = "not implemented yet"]
fn writes_reads_zone_info_list() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_channel_configuration() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_channel_configuration_basic() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_char_str_from_number_array() {
        let value = vec![0x61, 0x62, 0x63, 0x64];
        let expected_position = 4;
        let mut buffalo = BuffaloZcl::new(vec![0; 10]);
        buffalo.write(DataType::CHAR_STR as u32, &json!(value), &json!({}));
        assert_eq!(buffalo.get_position(), expected_position);
        assert_eq!(buffalo.get_written(), value);
}

#[test]
#[ignore = "not implemented yet"]
fn writes_commissioning() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_commissioning_all_options() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_custom_reply() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_invalida_data_type_as_buffer_if_buffer_like() {
        let value = vec![1, 2, 3];
        let mut buffalo = BuffaloZcl::new(vec![0; 3]);
        buffalo.write(99999, &json!(value), &json!({}));
        assert_eq!(buffalo.get_position(), 3);
        assert_eq!(buffalo.get_buffer(), vec![1, 2, 3]);
    
        buffalo.set_position(0);
        buffalo.write(99999, &json!([1, 2, 3]), &json!({}));
        assert_eq!(buffalo.get_position(), 3);
        assert_eq!(buffalo.get_buffer(), vec![1, 2, 3]);
}

#[test]
#[ignore = "not implemented yet"]
fn writes_nothing() {
        for data_type in [DataType::NO_DATA as u32, DataType::UNKNOWN as u32] {
            let buffer = vec![0; 3];
            let mut buffalo = BuffaloZcl::new(buffer.clone());
            buffalo.write(data_type, &json!(123), &json!({}));
            assert_eq!(buffalo.get_buffer(), buffer);
            assert_eq!(buffalo.get_position(), 0);
        }
    
        let buffer = vec![0; 3];
        let mut buffalo = BuffaloZcl::new(buffer.clone());
        buffalo.write(
            BuffaloZclDataType::STRUCTURED_SELECTOR as u32,
            &json!(null),
            &json!({}),
        );
        assert_eq!(buffalo.get_buffer(), buffer);
        assert_eq!(buffalo.get_position(), 0);
    
        // what ZCL spec calls "empty string"
        let buffer = vec![0; 3];
        let mut buffalo = BuffaloZcl::new(buffer.clone());
        buffalo.write(DataType::CHAR_STR as u32, &json!(""), &json!({}));
        assert_eq!(buffalo.get_buffer(), buffer);
        assert_eq!(buffalo.get_position(), 1);
    
        // what ZCL spec calls "empty string"
        let buffer = vec![0; 3];
        let mut buffalo = BuffaloZcl::new(buffer.clone());
        buffalo.write(DataType::OCTET_STR as u32, &json!([]), &json!({}));
        assert_eq!(buffalo.get_buffer(), buffer);
        assert_eq!(buffalo.get_position(), 1);
}

#[test]
#[ignore = "not implemented yet"]
fn writes_nothing_for_unhandled_command() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_buffalo() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn workaround_reads_char_str_as_mi_struct_for_xiaomi_attrid_id_65281() {
    todo!()
}

// Upstream: it.each([...])("Writes & Reads using base class for %s", ...)
// The table rows keep the upstream `%s` names. `write_method`/`read_method`
// are the upstream spied method names.
#[test]
#[ignore = "not implemented yet"]
fn writes_reads_using_base_class_for() {
    #[allow(clippy::type_complexity)]
    let cases: Vec<(
        &str,
        serde_json::Value,
        Vec<u32>,
        usize,
        &str,
        &str,
    )> = vec![
        (
            "boolean",
            json!(1),
            vec![DataType::BOOLEAN as u32],
            1,
            "writeUInt8",
            "readUInt8",
        ),
        (
            "boolean",
            json!(0),
            vec![DataType::BOOLEAN as u32],
            1,
            "writeUInt8",
            "readUInt8",
        ),
        (
            "uint8-like",
            json!(250),
            vec![
                DataType::DATA8 as u32,
                DataType::BITMAP8 as u32,
                DataType::UINT8 as u32,
                DataType::ENUM8 as u32,
            ],
            1,
            "writeUInt8",
            "readUInt8",
        ),
        (
            "uint16-like",
            json!(65530),
            vec![
                DataType::DATA16 as u32,
                DataType::BITMAP16 as u32,
                DataType::UINT16 as u32,
                DataType::ENUM16 as u32,
                DataType::CLUSTER_ID as u32,
                DataType::ATTR_ID as u32,
            ],
            2,
            "writeUInt16",
            "readUInt16",
        ),
        (
            "uint24-like",
            json!(16777210),
            vec![
                DataType::DATA24 as u32,
                DataType::BITMAP24 as u32,
                DataType::UINT24 as u32,
            ],
            3,
            "writeUInt24",
            "readUInt24",
        ),
        (
            "uint32-like",
            json!(4294967290u64),
            vec![
                DataType::DATA32 as u32,
                DataType::BITMAP32 as u32,
                DataType::UINT32 as u32,
                DataType::UTC as u32,
                DataType::BAC_OID as u32,
            ],
            4,
            "writeUInt32",
            "readUInt32",
        ),
        (
            "uint40-like",
            json!(1099511627770u64),
            vec![
                DataType::DATA40 as u32,
                DataType::BITMAP40 as u32,
                DataType::UINT40 as u32,
            ],
            5,
            "writeUInt40",
            "readUInt40",
        ),
        (
            "uint48-like",
            json!(281474976710650u64),
            vec![
                DataType::DATA48 as u32,
                DataType::BITMAP48 as u32,
                DataType::UINT48 as u32,
            ],
            6,
            "writeUInt48",
            "readUInt48",
        ),
        (
            "uint56-like",
            json!(72057594037927935u64),
            vec![
                DataType::DATA56 as u32,
                DataType::BITMAP56 as u32,
                DataType::UINT56 as u32,
            ],
            7,
            "writeUInt56",
            "readUInt56",
        ),
        (
            "uint64-like",
            json!(18446744073709551610u64),
            vec![
                DataType::DATA64 as u32,
                DataType::BITMAP64 as u32,
                DataType::UINT64 as u32,
            ],
            8,
            "writeUInt64",
            "readUInt64",
        ),
        (
            "int8-like",
            json!(-120),
            vec![DataType::INT8 as u32],
            1,
            "writeInt8",
            "readInt8",
        ),
        (
            "int16-like",
            json!(-32760),
            vec![DataType::INT16 as u32],
            2,
            "writeInt16",
            "readInt16",
        ),
        (
            "int24-like",
            json!(-8388600),
            vec![DataType::INT24 as u32],
            3,
            "writeInt24",
            "readInt24",
        ),
        (
            "int32-like",
            json!(-2147483640),
            vec![DataType::INT32 as u32],
            4,
            "writeInt32",
            "readInt32",
        ),
        (
            "int40-like",
            json!(-549755813887i64),
            vec![DataType::INT40 as u32],
            5,
            "writeInt40",
            "readInt40",
        ),
        (
            "int48-like",
            json!(-140737488355320i64),
            vec![DataType::INT48 as u32],
            6,
            "writeInt48",
            "readInt48",
        ),
        (
            "int56-like",
            json!(-36028797018963960i64),
            vec![DataType::INT56 as u32],
            7,
            "writeInt56",
            "readInt56",
        ),
        (
            "int64-like",
            json!(-9223372036854775800i64),
            vec![DataType::INT64 as u32],
            8,
            "writeInt64",
            "readInt64",
        ),
        (
            "float-like",
            json!(1.539989614439558e-36_f64),
            vec![DataType::SINGLE_PREC as u32],
            4,
            "writeFloatLE",
            "readFloatLE",
        ),
        (
            "double-like",
            json!(5.447603722011605e-270_f64),
            vec![DataType::DOUBLE_PREC as u32],
            8,
            "writeDoubleLE",
            "readDoubleLE",
        ),
        (
            "IEEE address",
            json!("0xfe1234abcd9876ff"),
            vec![DataType::IEEE_ADDR as u32],
            8,
            "writeIeeeAddr",
            "readIeeeAddr",
        ),
        (
            "uint8-like list",
            json!([250, 25, 50]),
            vec![BuffaloZclDataType::LIST_UINT8 as u32],
            3,
            "writeListUInt8",
            "readListUInt8",
        ),
        (
            "uint16-like list",
            json!([65530, 6553, 5530]),
            vec![BuffaloZclDataType::LIST_UINT16 as u32],
            6,
            "writeListUInt16",
            "readListUInt16",
        ),
        (
            "uint24-like list",
            json!([16777210, 1677721, 6777210]),
            vec![BuffaloZclDataType::LIST_UINT24 as u32],
            9,
            "writeListUInt24",
            "readListUInt24",
        ),
        (
            "uint32-like list",
            json!([4294967290u64, 429496729u64, 294967290u64]),
            vec![BuffaloZclDataType::LIST_UINT32 as u32],
            12,
            "writeListUInt32",
            "readListUInt32",
        ),
        (
            "buffer",
            json!([1, 2, 3, 4]),
            vec![BuffaloZclDataType::BUFFER as u32],
            4,
            "writeBuffer",
            "readBuffer",
        ),
        (
            "security key",
            json!([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
            vec![DataType::SEC_KEY as u32],
            16,
            "writeBuffer",
            "readBuffer",
        ),
    ];

    for (name, payload, types, position, write_method, read_method) in cases {
        let mut read_options = serde_json::Map::new();

        if let Some(array) = payload.as_array() {
            read_options.insert("length".to_string(), json!(array.len()));
        }

        let read_options = serde_json::Value::Object(read_options);

        for data_type in types {
            let mut buffalo = BuffaloZcl::new(vec![0; 255]);

            buffalo.write(data_type, &payload, &json!({}));
            assert_eq!(
                buffalo.get_position(),
                position,
                "{name}: {write_method} should advance to {position}"
            );

            buffalo.set_position(0);

            assert_eq!(
                buffalo.read(data_type, &read_options),
                payload,
                "{name}: {read_method} should round-trip"
            );
            assert_eq!(
                buffalo.get_position(),
                position,
                "{name}: {read_method} should advance to {position}"
            );
        }
    }
}

// Upstream: it.each([...])("Writes & Reads non-value for %s", ...)
// `written` is the raw non-value prefix expected in the buffer.
#[test]
#[ignore = "not implemented yet"]
fn writes_reads_non_value_for() {
    #[allow(clippy::type_complexity)]
    let cases: Vec<(
        &str,
        Vec<u32>,
        Vec<u8>,
        serde_json::Value,
        usize,
        &str,
        &str,
    )> = vec![
        (
            "octectStr",
            vec![DataType::OCTET_STR as u32],
            vec![0xff],
            json!([]),
            1,
            "writeUInt8",
            "readUInt8",
        ),
        (
            "longOctectStr",
            vec![DataType::LONG_OCTET_STR as u32],
            vec![0xff, 0xff],
            json!([]),
            2,
            "writeUInt16",
            "readUInt16",
        ),
        (
            "charStr",
            vec![DataType::CHAR_STR as u32],
            vec![0xff],
            json!(""),
            1,
            "writeUInt8",
            "readUInt8",
        ),
        (
            "longCharStr",
            vec![DataType::LONG_CHAR_STR as u32],
            vec![0xff, 0xff],
            json!(""),
            2,
            "writeUInt16",
            "readUInt16",
        ),
        (
            "array",
            vec![DataType::ARRAY as u32],
            vec![0x00, 0xff, 0xff],
            json!([]),
            3,
            "writeUInt16",
            "readUInt16",
        ),
        (
            "struct",
            vec![DataType::STRUCT as u32],
            vec![0xff, 0xff],
            json!([]),
            2,
            "writeUInt16",
            "readUInt16",
        ),
    ];

    for (name, types, written, value_read, position, write_method, read_method) in cases {
        for data_type in types {
            let mut buffalo = BuffaloZcl::new(vec![0; 255]);

            buffalo.write(data_type, &serde_json::Value::Null, &json!({}));
            assert_eq!(
                buffalo.get_position(),
                position,
                "{name}: {write_method} should advance to {position}"
            );
            assert_eq!(buffalo.get_written(), written, "{name}: written prefix");

            buffalo.set_position(0);

            assert_eq!(
                buffalo.read(data_type, &json!({})),
                value_read,
                "{name}: {read_method} should return the non-value"
            );
            assert_eq!(
                buffalo.get_position(),
                position,
                "{name}: {read_method} should advance to {position}"
            );
        }
    }
}

