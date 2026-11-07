// Original: .upstream/zigbee-herdsman/test/zspec/zcl/buffalo.test.ts

use serde_json::json;
use zigbee_apiarist::zspec::zcl::buffalo_zcl::BuffaloZcl;
use zigbee_apiarist::zspec::zcl::definition::enums::{
    BuffaloZclDataType, DataType, StructuredIndicatorType,
};
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
fn reads_mi_struct() {
    let buffer = vec![
        34u8, 1, 33, 213, 12, 3, 40, 33, 4, 33, 168, 19, 5, 33, 43, 0, 6, 36, 0, 0, 5, 0, 0, 8, 33,
        4, 2, 10, 33, 0, 0, 100, 16, 0,
    ];
    let mut buffalo = BuffaloZcl::new(buffer.clone());
    assert_eq!(
        buffalo.read(BuffaloZclDataType::MI_STRUCT as u32, &json!({})),
        json!({
            "1": 3285,
            "3": 33,
            "4": 5032,
            "5": 43,
            "6": 327680,
            "8": 516,
            "10": 0,
            "100": 0,
        })
    );
    assert_eq!(buffalo.get_position(), buffer.len());
}

#[test]
#[ignore = "not implemented yet"]
fn reads_mi_struct_with_padding() {
    let buffer = vec![
        68u8, 3, 40, 29, 5, 33, 190, 45, 8, 33, 47, 18, 9, 33, 2, 21, 100, 16, 1, 101, 16, 0, 110,
        32, 255, 111, 32, 255, 148, 32, 4, 149, 57, 184, 30, 21, 62, 150, 57, 211, 249, 17, 69,
        151, 57, 0, 48, 104, 59, 152, 57, 0, 0, 0, 0, 155, 33, 1, 0, 156, 32, 1, 10, 33, 56, 38,
        12, 40, 0, 0,
    ];
    let mut buffalo = BuffaloZcl::new(buffer.clone());
    assert_eq!(
        buffalo.read(BuffaloZclDataType::MI_STRUCT as u32, &json!({})),
        json!({
            "3": 29,
            "5": 11710,
            "8": 4655,
            "9": 5378,
            "10": 9784,
            "12": 0,
            "100": 1,
            "101": 0,
            "110": 255,
            "111": 255,
            "148": 4,
            "149": 0.14562499523162842,
            "150": 2335.614013671875,
            "151": 0.0035429000854492188,
            "152": 0,
            "155": 1,
            "156": 1,
        })
    );
    assert_eq!(buffalo.get_position(), buffer.len());
}

#[test]
#[ignore = "not implemented yet"]
fn reads_attribute_report() {
    let value = vec![
        0x00u8,
        0x00,
        0x01,
        0x00,
        DataType::UINT8 as u8,
        0xff,
        0x01,
        0x10,
        DataType::CHAR_STR as u8,
        0x06,
        0x5a,
        0x49,
        0x47,
        0x42,
        0x45,
        0x45,
        0x02,
        0x10,
        DataType::BOOLEAN as u8,
        0x01,
    ];
    let mut buffalo = BuffaloZcl::new(value.clone());
    assert_eq!(
        buffalo.read(
            BuffaloZclDataType::GPD_FRAME as u32,
            &json!({"payload": {"commandID": 0xa0, "payloadSize": value.len()}}),
        ),
        json!({
            "clusterID": 0x0000,
            "clusterName": "genBasic",
            "attributes": {"appVersion": 0xff, "4097": "ZIGBEE", "4098": 1},
        })
    );
}

#[test]
#[ignore = "not implemented yet"]
fn reads_attribute_request() {
    let value = vec![0x02u8, 0xfe, 0xef, 0xff, 0xff, 4, 0x00, 0x00, 0x00, 0x01];
    let mut buffalo = BuffaloZcl::new(value.clone());
    assert_eq!(
        buffalo.read(
            BuffaloZclDataType::GPD_FRAME as u32,
            &json!({"payload": {"commandID": 0xa4, "payloadSize": value.len()}}),
        ),
        json!({
            "options": 0x02,
            "manufacturerID": 0xeffe,
            "records": [{"clusterID": 0xffff, "attributes": [0x0000, 0x0100]}],
        })
    );
}

#[test]
#[ignore = "not implemented yet"]
fn reads_attribute_response() {
    let value = vec![
        0x02u8,
        0xfe,
        0xef,
        0x00,
        0x00,
        11,
        0x00,
        0x00,
        0x00,
        DataType::UINT8 as u8,
        0x08,
        0xfc,
        0xff,
        0x00,
        DataType::UINT16 as u8,
        0x00,
        0x10,
    ];
    let mut buffalo = BuffaloZcl::new(value.clone());
    assert_eq!(
        buffalo.read(
            BuffaloZclDataType::GPD_FRAME as u32,
            &json!({"payload": {"commandID": 0xa5, "payloadSize": value.len()}}),
        ),
        json!({
            "options": 0x02,
            "manufacturerID": 0xeffe,
            "records": [
                {
                    "clusterID": 0x0000,
                    "clusterName": "genBasic",
                    "attributes": {
                        "zclVersion": {"status": 0x00, "attrData": 0x08},
                        "65532": {"status": 0x00, "attrData": 0x1000},
                    },
                },
            ],
        })
    );
}

#[test]
#[ignore = "not implemented yet"]
fn reads_channel_request() {
    let value = vec![0xfau8];
    let mut buffalo = BuffaloZcl::new(value.clone());
    assert_eq!(
        buffalo.read(
            BuffaloZclDataType::GPD_FRAME as u32,
            &json!({"payload": {"payloadSize": value.len(), "commandID": 0xe3}}),
        ),
        json!({"nextChannel": 0xa, "nextNextChannel": 0xf})
    );
}

#[test]
#[ignore = "not implemented yet"]
fn reads_commissioning() {
    let value = vec![0xffu8, 0x00];
    let mut buffalo = BuffaloZcl::new(value.clone());
    assert_eq!(
        buffalo.read(
            BuffaloZclDataType::GPD_FRAME as u32,
            &json!({"payload": {"payloadSize": value.len(), "commandID": 0xe0}}),
        ),
        json!({
            "deviceID": 0xff,
            "options": 0x00,
            "extendedOptions": 0x00,
            "securityKey": vec![0u8; 16],
            "keyMic": 0,
            "outgoingCounter": 0,
            "manufacturerID": 0,
            "modelID": 0,
            "numGpdCommands": 0,
            "gpdCommandIdList": [],
            "numServerClusters": 0,
            "numClientClusters": 0,
            "gpdServerClusters": [],
            "gpdClientClusters": [],
            "applicationInfo": 0x00,
            "genericSwitchConfig": 0,
            "currentContactStatus": 0,
        })
    );
}

#[test]
#[ignore = "not implemented yet"]
fn reads_commissioning_all_options() {
    let value = vec![
        0xffu8,
        0x80 | 0x04,
        0x20 | 0x40 | 0x80,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0x01 | 0x02 | 0x04 | 0x08 | 0x10,
        0,
        0,
        0,
        0,
        0,
        0,
        2,
        5,
        2,
    ];
    let mut buffalo = BuffaloZcl::new(value.clone());
    assert_eq!(
        buffalo.read(
            BuffaloZclDataType::GPD_FRAME as u32,
            &json!({"payload": {"payloadSize": value.len(), "commandID": 0xe0}}),
        ),
        json!({
            "deviceID": 0xff,
            "options": 0x80 | 0x04,
            "extendedOptions": 0x20 | 0x40 | 0x80,
            "securityKey": vec![0u8; 16],
            "keyMic": 0,
            "outgoingCounter": 0,
            "manufacturerID": 0,
            "modelID": 0,
            "numGpdCommands": 0,
            "gpdCommandIdList": [],
            "numServerClusters": 0,
            "numClientClusters": 0,
            "gpdServerClusters": [],
            "gpdClientClusters": [],
            "applicationInfo": 0x01 | 0x02 | 0x04 | 0x08 | 0x10,
            "genericSwitchConfig": 5,
            "currentContactStatus": 2,
        })
    );
}

#[test]
#[ignore = "not implemented yet"]
fn reads_invalid_tuya_data_point_values_list_as_empty_array() {
    let mut buffalo = BuffaloZcl::new(vec![254, 125]);
    assert_eq!(
        buffalo.read(
            BuffaloZclDataType::LIST_TUYA_DATAPOINT_VALUES as u32,
            &json!({}),
        ),
        json!([])
    );
}

#[test]
#[ignore = "not implemented yet"]
fn reads_manufacturer_specific_attribute_report() {
    let value = vec![
        0x12u8,
        0x34,
        0x00,
        0x00,
        0x00,
        0x00,
        DataType::UINT8 as u8,
        0x08,
        0x01,
        0x20,
        DataType::CHAR_STR as u8,
        0x06,
        0x5a,
        0x49,
        0x47,
        0x42,
        0x45,
        0x45,
        0x02,
        0x20,
        DataType::BOOLEAN as u8,
        0x01,
    ];
    let mut buffalo = BuffaloZcl::new(value.clone());
    assert_eq!(
        buffalo.read(
            BuffaloZclDataType::GPD_FRAME as u32,
            &json!({"payload": {"commandID": 0xa1, "payloadSize": value.len()}}),
        ),
        json!({
            "manufacturerCode": 0x3412,
            "clusterID": 0x0000,
            "clusterName": "genBasic",
            "attributes": {"zclVersion": 8, "8193": "ZIGBEE", "8194": 1},
        })
    );
}

#[test]
#[ignore = "not implemented yet"]
fn reads_manufacturer_specific_multi_cluster_attribute_report() {
    let value = vec![
        0xefu8,
        0xf3,
        0x00,
        0x00,
        0x00,
        0x00,
        DataType::UINT8 as u8,
        0x08,
        0xff,
        0xfe,
        0x00,
        0x01,
        DataType::UINT16 as u8,
        0x10,
        0x11,
    ];
    let mut buffalo = BuffaloZcl::new(value.clone());
    assert_eq!(
        buffalo.read(
            BuffaloZclDataType::GPD_FRAME as u32,
            &json!({"payload": {"commandID": 0xa3, "payloadSize": value.len()}}),
        ),
        json!({
            "manufacturerCode": 0xf3ef,
            "reports": [
                {
                    "clusterID": 0x0000,
                    "clusterName": "genBasic",
                    "attribute": "zclVersion",
                    "attrData": 8,
                },
                {
                    "clusterID": 0xfeff,
                    "clusterName": "65279",
                    "attribute": 0x0100,
                    "attrData": 4368,
                },
            ],
        })
    );
}

#[test]
#[ignore = "not implemented yet"]
fn reads_multi_cluster_attribute_report() {
    let value = vec![
        0xffu8,
        0xff,
        0x00,
        0x00,
        DataType::UINT32 as u8,
        0x00,
        0x01,
        0x02,
        0x03,
        0x00,
        0x00,
        0x01,
        0x00,
        DataType::UINT8 as u8,
        0x10,
    ];
    let mut buffalo = BuffaloZcl::new(value.clone());
    assert_eq!(
        buffalo.read(
            BuffaloZclDataType::GPD_FRAME as u32,
            &json!({"payload": {"commandID": 0xa2, "payloadSize": value.len()}}),
        ),
        json!({
            "reports": [
                {
                    "clusterID": 0xffff,
                    "clusterName": "65535",
                    "attribute": 0,
                    "attrData": 50462976u64,
                },
                {
                    "clusterID": 0x0000,
                    "clusterName": "genBasic",
                    "attribute": "appVersion",
                    "attrData": 0x10,
                },
            ],
        })
    );
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
    assert_eq!(
        buffalo.read(DataType::CHAR_STR as u32, &json!({})),
        json!("")
    );
    assert_eq!(buffalo.get_position(), 1);

    // what ZCL spec calls "empty string"
    let buffer = vec![0, 2, 3];
    let mut buffalo = BuffaloZcl::new(buffer);
    assert_eq!(
        buffalo.read(DataType::OCTET_STR as u32, &json!({})),
        json!([])
    );
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
        buffalo.read(
            BuffaloZclDataType::BUFFER as u32,
            &json!({"length": length})
        ),
        json!([value[0], value[1]])
    );
    assert_eq!(buffalo.get_position(), length);
}

#[test]
#[ignore = "not implemented yet"]
fn reads_unhandled_command_as_empty_object_if_buffer_finished_reading() {
    let value = vec![0xff, 0x00];
    let mut buffalo = BuffaloZcl::new(value.clone());
    buffalo.set_position(value.len());
    assert_eq!(
        buffalo.read(
            BuffaloZclDataType::GPD_FRAME as u32,
            &json!({"payload": {"commandID": 0x1ff, "payloadSize": value.len()}}),
        ),
        json!({"raw": []})
    );
}

#[test]
#[ignore = "not implemented yet"]
fn reads_unhandled_command_as_object_raw_and_ignores_gpp_data_mic() {
    let value = vec![0xff, 0x00, 0x21, 0x43, 0xfe];
    let mut buffalo = BuffaloZcl::new(value.clone());
    assert_eq!(
        buffalo.read(
            BuffaloZclDataType::GPD_FRAME as u32,
            &json!({"payload": {"commandID": 0x1ff, "payloadSize": 2}}),
        ),
        json!({"raw": &value[..value.len() - 3]})
    );

    let value = vec![0xff, 0x00, 0x21, 0x43, 0xfe, 0xf1, 0xf2, 0xf3, 0xf4];
    let mut buffalo = BuffaloZcl::new(value.clone());
    assert_eq!(
        buffalo.read(
            BuffaloZclDataType::GPD_FRAME as u32,
            &json!({"payload": {"commandID": 0x1ff, "payloadSize": 2}}),
        ),
        json!({"raw": &value[..value.len() - 7]})
    );
}

#[test]
#[ignore = "not implemented yet"]
fn reads_unhandled_command_as_object_raw_if_buffer_still_has_bytes_to_read() {
    let value = vec![0xff, 0x00];
    let mut buffalo = BuffaloZcl::new(value.clone());
    assert_eq!(
        buffalo.read(
            BuffaloZclDataType::GPD_FRAME as u32,
            &json!({"payload": {"commandID": 0x1ff, "payloadSize": 2}}),
        ),
        json!({"raw": value})
    );
}

#[test]
#[ignore = "not implemented yet"]
fn reads_whole_buffer_without_length_option() {
    let value = vec![1, 2, 3, 4];
    let mut buffalo = BuffaloZcl::new(vec![0; 4]);
    buffalo.write(BuffaloZclDataType::BUFFER as u32, &json!(value), &json!({}));
    assert_eq!(buffalo.get_position(), value.len());
    buffalo.set_position(0);
    assert_eq!(
        buffalo.read(BuffaloZclDataType::BUFFER as u32, &json!({})),
        json!(value)
    );
    assert_eq!(buffalo.get_position(), value.len());
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_read_strctured_select_indicator_is_outside_range() {
    let result = std::panic::catch_unwind(|| {
        let mut buffalo = BuffaloZcl::new(vec![17]);
        buffalo.read(BuffaloZclDataType::STRUCTURED_SELECTOR as u32, &json!({}));
    });
    assert!(result.is_err());

    let result = std::panic::catch_unwind(|| {
        let mut buffalo = BuffaloZcl::new(vec![17, 1, 2, 3]);
        buffalo.read(BuffaloZclDataType::STRUCTURED_SELECTOR as u32, &json!({}));
    });
    assert!(result.is_err());
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_read_thermo_transitions_list_is_missing_required_payload_options() {
    let options = [
        json!({}),
        json!({"payload": {}}),
        json!({"payload": {"mode": 1}}),
        json!({"payload": {"numoftrans": 1}}),
    ];

    for options in options {
        let result = std::panic::catch_unwind(|| {
            let mut buffalo = BuffaloZcl::new(vec![0; 1]);
            buffalo.read(BuffaloZclDataType::LIST_THERMO_TRANSITIONS as u32, &options);
        });
        assert!(result.is_err());
    }
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_read_is_missing_payload_payload_size_option() {
    let result = std::panic::catch_unwind(|| {
        let mut buffalo = BuffaloZcl::new(vec![0; 1]);
        buffalo.read(
            BuffaloZclDataType::GPD_FRAME as u32,
            &json!({"payload": {"commandID": 0xa1}}),
        );
    });
    assert!(result.is_err());
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_write_use_data_type_is_missing_data_type_option_and_value_isnt_buffer_or_number_array(
) {
    let result = std::panic::catch_unwind(|| {
        let payload = json!(1);
        let mut buffalo = BuffaloZcl::new(vec![0; 2]);
        buffalo.write(
            BuffaloZclDataType::USE_DATA_TYPE as u32,
            &payload,
            &json!({}),
        );
    });
    assert!(result.is_err());
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
    let value = json!({"year": 2000, "month": 8, "dayOfMonth": 31, "dayOfWeek": 3});
    let expected_written = vec![100, 8, 31, 3];
    let mut buffalo = BuffaloZcl::new(vec![0; 10]);
    buffalo.write(DataType::DATE as u32, &value, &json!({}));
    assert_eq!(buffalo.get_position(), expected_written.len());
    assert_eq!(buffalo.get_written(), expected_written);
    buffalo.set_position(0);
    assert_eq!(buffalo.read(DataType::DATE as u32, &json!({})), value);
    assert_eq!(buffalo.get_position(), expected_written.len());
}

#[test]
#[ignore = "not implemented yet"]
fn writes_reads_mi_boxer_zones_list() {
    let value = json!([
        {"zoneNum": 1, "groupId": 0x2b84},
        {"zoneNum": 2, "groupId": 0x2b98},
        {"zoneNum": 3, "groupId": 0x2bac},
        {"zoneNum": 4, "groupId": 0x2bc0},
        {"zoneNum": 5, "groupId": 0x2bd4},
        {"zoneNum": 6, "groupId": 0x2be8},
        {"zoneNum": 7, "groupId": 0x2bfc},
        {"zoneNum": 8, "groupId": 0x2c10},
    ]);
    let expected_written = vec![
        8, 0x84, 0x2b, 1, 0x98, 0x2b, 2, 0xac, 0x2b, 3, 0xc0, 0x2b, 4, 0xd4, 0x2b, 5, 0xe8, 0x2b,
        6, 0xfc, 0x2b, 7, 0x10, 0x2c, 8,
    ];
    let mut buffalo = BuffaloZcl::new(vec![0; 50]);
    buffalo.write(
        BuffaloZclDataType::LIST_MIBOXER_ZONES as u32,
        &value,
        &json!({}),
    );
    assert_eq!(buffalo.get_position(), expected_written.len());
    assert_eq!(buffalo.get_written(), expected_written);

    buffalo.set_position(0);
    assert_eq!(
        buffalo.read(
            BuffaloZclDataType::LIST_MIBOXER_ZONES as u32,
            &json!({"length": 8}),
        ),
        value
    );
}

#[test]
#[ignore = "not implemented yet"]
fn writes_reads_time_of_day() {
    let value = json!({"hours": 0, "minutes": 59, "seconds": 34, "hundredths": 88});
    let expected_written = vec![0, 59, 34, 88];
    let mut buffalo = BuffaloZcl::new(vec![0; 10]);
    buffalo.write(DataType::TOD as u32, &value, &json!({}));
    assert_eq!(buffalo.get_position(), expected_written.len());
    assert_eq!(buffalo.get_written(), expected_written);
    buffalo.set_position(0);
    assert_eq!(buffalo.read(DataType::TOD as u32, &json!({})), value);
    assert_eq!(buffalo.get_position(), expected_written.len());
}

#[test]
#[ignore = "not implemented yet"]
fn writes_reads_use_data_type_as_buffer_when_missing_data_type_option() {
    let value = vec![12, 34];
    let mut buffalo = BuffaloZcl::new(vec![0; 2]);
    buffalo.write(
        BuffaloZclDataType::USE_DATA_TYPE as u32,
        &json!(value),
        &json!({}),
    );
    assert_eq!(buffalo.get_position(), value.len());
    assert_eq!(buffalo.get_written(), value);

    buffalo.set_position(0);
    assert_eq!(
        buffalo.read(BuffaloZclDataType::USE_DATA_TYPE as u32, &json!({})),
        json!(value)
    );

    buffalo.set_position(0);
    assert_eq!(
        buffalo.read(
            BuffaloZclDataType::USE_DATA_TYPE as u32,
            &json!({"length": 1}),
        ),
        json!([value[0]])
    );
}

#[test]
#[ignore = "not implemented yet"]
fn writes_reads_big_endian_uint24() {
    let value = json!(16777200);
    let expected_written = vec![0xff, 0xff, 0xf0];
    let mut buffalo = BuffaloZcl::new(vec![0; 10]);
    buffalo.write(
        BuffaloZclDataType::BIG_ENDIAN_UINT24 as u32,
        &value,
        &json!({}),
    );
    assert_eq!(buffalo.get_position(), expected_written.len());
    assert_eq!(buffalo.get_written(), expected_written);

    buffalo.set_position(0);
    assert_eq!(
        buffalo.read(BuffaloZclDataType::BIG_ENDIAN_UINT24 as u32, &json!({})),
        value
    );
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
    assert_eq!(
        buffalo.read(DataType::CHAR_STR as u32, &json!({})),
        json!(value)
    );
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
    assert_eq!(
        buffalo.read(DataType::LONG_CHAR_STR as u32, &json!({})),
        json!(value)
    );
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
    assert_eq!(
        buffalo.read(DataType::LONG_OCTET_STR as u32, &json!({})),
        json!(value)
    );
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
    assert_eq!(
        buffalo.read(DataType::OCTET_STR as u32, &json!({})),
        json!(value)
    );
    assert_eq!(buffalo.get_position(), expected_position);
}

#[test]
#[ignore = "not implemented yet"]
fn writes_reads_zone_info_list() {
    let value = json!([
        {"zoneID": 1, "zoneStatus": 5},
        {"zoneID": 2, "zoneStatus": 6},
    ]);
    let expected_written = vec![1, 5, 0, 2, 6, 0];
    let mut buffalo = BuffaloZcl::new(vec![0; 10]);
    buffalo.write(BuffaloZclDataType::LIST_ZONEINFO as u32, &value, &json!({}));
    assert_eq!(buffalo.get_position(), expected_written.len());
    assert_eq!(buffalo.get_written(), expected_written);
    buffalo.set_position(0);
    assert_eq!(
        buffalo.read(
            BuffaloZclDataType::LIST_ZONEINFO as u32,
            &json!({"length": value.as_array().unwrap().len()}),
        ),
        value
    );
    assert_eq!(buffalo.get_position(), expected_written.len());
}

#[test]
#[ignore = "not implemented yet"]
fn writes_channel_configuration() {
    let expected = vec![1u8, 0xf];
    let mut buffalo = BuffaloZcl::new(vec![0; 2]);
    buffalo.write(
        BuffaloZclDataType::GPD_FRAME as u32,
        &json!({"commandID": 0xf3, "operationalChannel": 0xf, "basic": false}),
        &json!({}),
    );
    assert_eq!(buffalo.get_written(), expected);
}

#[test]
#[ignore = "not implemented yet"]
fn writes_channel_configuration_basic() {
    let expected = vec![1u8, 0x1f];
    let mut buffalo = BuffaloZcl::new(vec![0; 2]);
    buffalo.write(
        BuffaloZclDataType::GPD_FRAME as u32,
        &json!({"commandID": 0xf3, "operationalChannel": 0xf, "basic": true}),
        &json!({}),
    );
    assert_eq!(buffalo.get_written(), expected);
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
    let expected = vec![1u8, 0];
    let mut buffalo = BuffaloZcl::new(vec![0; 2]);
    buffalo.write(
        BuffaloZclDataType::GPD_FRAME as u32,
        &json!({
            "commandID": 0xf0,
            "options": 0,
            "panID": 0,
            "securityKey": vec![0u8; 16],
            "keyMic": 0,
            "frameCounter": 0,
        }),
        &json!({}),
    );
    assert_eq!(buffalo.get_written(), expected);
}

#[test]
#[ignore = "not implemented yet"]
fn writes_commissioning_all_options() {
    let expected = vec![
        27, 0b11111, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0,
    ];
    let mut buffalo = BuffaloZcl::new(vec![0; 28]);
    buffalo.write(
        BuffaloZclDataType::GPD_FRAME as u32,
        &json!({
            "commandID": 0xf0,
            "options": 0b11111,
            "panID": 0xffff,
            "securityKey": vec![0u8; 16],
            "keyMic": 0,
            "frameCounter": 0,
        }),
        &json!({}),
    );
    assert_eq!(buffalo.get_written(), expected);
}

#[test]
#[ignore = "not implemented yet"]
fn writes_custom_reply() {
    let expected = vec![6u8, 90, 73, 71, 66, 69, 69];
    let mut buffalo = BuffaloZcl::new(vec![0; 7]);
    buffalo.write(
        BuffaloZclDataType::GPD_FRAME as u32,
        &json!({"commandID": 0xf4, "buffer": b"ZIGBEE".to_vec()}),
        &json!({}),
    );
    assert_eq!(buffalo.get_written(), expected);
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
    let mut buffalo = BuffaloZcl::new(vec![0; 7]);
    buffalo.write(
        BuffaloZclDataType::GPD_FRAME as u32,
        &json!({"commandID": 0x1ff}),
        &json!({}),
    );
    assert_eq!(buffalo.get_written(), Vec::<u8>::new());
}

#[test]
#[ignore = "not implemented yet"]
fn workaround_reads_char_str_as_mi_struct_for_xiaomi_attrid_id_65281() {
    let expected_value = json!({
        "1": 3285,
        "3": 33,
        "4": 5032,
        "5": 43,
        "6": 327680,
        "8": 516,
        "10": 0,
        "100": 0,
    });
    let buffer = vec![
        34u8, 1, 0x21, 0xd5, 0x0c, 3, 0x28, 33, 4, 0x21, 0xa8, 0x13, 5, 0x21, 0x2b, 0x00, 6, 0x24,
        0x00, 0x00, 0x05, 0x00, 0, 8, 0x21, 0x04, 0x02, 10, 0x21, 0x00, 0x00, 100, 0x10, 0,
    ];
    let mut buffalo = BuffaloZcl::new(buffer.clone());

    assert_eq!(
        buffalo.read(BuffaloZclDataType::MI_STRUCT as u32, &json!({})),
        expected_value
    );
    assert_eq!(buffalo.get_position(), buffer.len());
}

// Upstream: it.each([...])("Writes & Reads using base class for %s", ...)
// The table rows keep the upstream `%s` names. `write_method`/`read_method`
// are the upstream spied method names.
#[test]
#[ignore = "not implemented yet"]
fn writes_reads_using_base_class_for() {
    #[allow(clippy::type_complexity)]
    let cases: Vec<(&str, serde_json::Value, Vec<u32>, usize, &str, &str)> = vec![
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

// Upstream: it.each([...])("Writes empty %s", ...)
#[test]
#[ignore = "not implemented yet"]
fn writes_empty() {
    let cases: [(&str, u32, Vec<u8>, usize); 2] = [
        ("char str", DataType::CHAR_STR as u32, vec![0], 1),
        (
            "long char str",
            DataType::LONG_CHAR_STR as u32,
            vec![0, 0],
            2,
        ),
    ];

    for (_name, data_type, expected_written, expected_position) in cases {
        let mut buffalo = BuffaloZcl::new(vec![0; 10]);
        buffalo.write(data_type, &json!(""), &json!({}));
        assert_eq!(buffalo.get_position(), expected_position);
        assert_eq!(buffalo.get_written(), expected_written);
    }
}

// Upstream: it.each([...])("Writes & Reads array of %s", ...)
#[test]
#[ignore = "not implemented yet"]
fn writes_reads_array_of() {
    let cases: [(&str, serde_json::Value, Vec<u8>); 3] = [
        (
            "uint16",
            json!({"elementType": DataType::UINT16 as u32, "elements": [256, 1, 65530, 0]}),
            vec![DataType::UINT16 as u8, 4, 0, 0, 1, 1, 0, 0xfa, 0xff, 0, 0],
        ),
        (
            "char str",
            json!({"elementType": DataType::CHAR_STR as u32, "elements": ["abcd", "cd", "a"]}),
            vec![
                DataType::CHAR_STR as u8,
                3,
                0,
                4,
                0x61,
                0x62,
                0x63,
                0x64,
                2,
                0x63,
                0x64,
                1,
                0x61,
            ],
        ),
        (
            "uint16 with element type passed as string key of Zcl.DataType",
            json!({"elementType": "UINT16", "elements": [256, 1, 65530, 0]}),
            vec![DataType::UINT16 as u8, 4, 0, 0, 1, 1, 0, 0xfa, 0xff, 0, 0],
        ),
    ];

    for (_name, payload, expected_written) in cases {
        let mut buffalo = BuffaloZcl::new(vec![0; 50]);
        buffalo.write(DataType::ARRAY as u32, &payload, &json!({}));
        assert_eq!(buffalo.get_position(), expected_written.len());
        assert_eq!(buffalo.get_written(), expected_written);

        buffalo.set_position(0);
        assert_eq!(
            buffalo.read(DataType::ARRAY as u32, &json!({})),
            payload["elements"]
        );
        assert_eq!(buffalo.get_position(), expected_written.len());
    }
}

// Upstream: it.each([...])("Writes & Reads struct of %s", ...)
#[test]
#[ignore = "not implemented yet"]
fn writes_reads_struct_of() {
    let cases: [(&str, serde_json::Value, Vec<u8>); 3] = [
        (
            "uint16",
            json!([
                {"elmType": DataType::UINT16 as u32, "elmVal": 256},
                {"elmType": DataType::UINT16 as u32, "elmVal": 1},
                {"elmType": DataType::UINT16 as u32, "elmVal": 65530},
                {"elmType": DataType::UINT16 as u32, "elmVal": 0},
            ]),
            vec![
                4,
                0,
                DataType::UINT16 as u8,
                0,
                1,
                DataType::UINT16 as u8,
                1,
                0,
                DataType::UINT16 as u8,
                0xfa,
                0xff,
                DataType::UINT16 as u8,
                0,
                0,
            ],
        ),
        (
            "char str",
            json!([
                {"elmType": DataType::CHAR_STR as u32, "elmVal": "abcd"},
                {"elmType": DataType::CHAR_STR as u32, "elmVal": "cd"},
                {"elmType": DataType::CHAR_STR as u32, "elmVal": "a"},
            ]),
            vec![
                3,
                0,
                DataType::CHAR_STR as u8,
                4,
                0x61,
                0x62,
                0x63,
                0x64,
                DataType::CHAR_STR as u8,
                2,
                0x63,
                0x64,
                DataType::CHAR_STR as u8,
                1,
                0x61,
            ],
        ),
        (
            "mixed",
            json!([
                {"elmType": DataType::UINT16 as u32, "elmVal": 256},
                {"elmType": DataType::CHAR_STR as u32, "elmVal": "abcd"},
                {"elmType": DataType::BITMAP8 as u32, "elmVal": 3},
            ]),
            vec![
                3,
                0,
                DataType::UINT16 as u8,
                0,
                1,
                DataType::CHAR_STR as u8,
                4,
                0x61,
                0x62,
                0x63,
                0x64,
                DataType::BITMAP8 as u8,
                3,
            ],
        ),
    ];

    for (_name, payload, expected_written) in cases {
        let mut buffalo = BuffaloZcl::new(vec![0; 50]);
        buffalo.write(DataType::STRUCT as u32, &payload, &json!({}));
        assert_eq!(buffalo.get_position(), expected_written.len());
        assert_eq!(buffalo.get_written(), expected_written);

        buffalo.set_position(0);
        assert_eq!(buffalo.read(DataType::STRUCT as u32, &json!({})), payload);
        assert_eq!(buffalo.get_position(), expected_written.len());
    }
}

// Upstream: it.each([...])("Reads Non-Value for %s", ...)
#[test]
#[ignore = "not implemented yet"]
fn reads_non_value_for() {
    // Upstream uses Number.NaN as the non-value marker; kept verbatim.
    let cases: [(&str, u32, usize, serde_json::Value); 3] = [
        (
            "time of day",
            DataType::TOD as u32,
            4,
            json!({
                "hours": f64::NAN,
                "minutes": f64::NAN,
                "seconds": f64::NAN,
                "hundredths": f64::NAN
            }),
        ),
        (
            "date",
            DataType::DATE as u32,
            4,
            json!({
                "year": f64::NAN,
                "month": f64::NAN,
                "dayOfMonth": f64::NAN,
                "dayOfWeek": f64::NAN
            }),
        ),
        (
            "mi struct",
            BuffaloZclDataType::MI_STRUCT as u32,
            1,
            json!({}),
        ),
    ];

    for (_name, data_type, position, returned) in cases {
        let mut buffalo = BuffaloZcl::new(vec![0xff; 50]);
        assert_eq!(buffalo.read(data_type, &json!({})), returned);
        assert_eq!(buffalo.get_position(), position);
    }
}

// Upstream: it.each([...])("Writes Non-Value for %s", ...)
#[test]
#[ignore = "not implemented yet"]
fn writes_non_value_for() {
    let cases: [(&str, u32, serde_json::Value, Vec<u8>); 2] = [
        (
            "time of day",
            DataType::TOD as u32,
            json!({
                "hours": f64::NAN,
                "minutes": f64::NAN,
                "seconds": f64::NAN,
                "hundredths": f64::NAN
            }),
            vec![0xff, 0xff, 0xff, 0xff],
        ),
        (
            "date",
            DataType::DATE as u32,
            json!({
                "year": f64::NAN,
                "month": f64::NAN,
                "dayOfMonth": f64::NAN,
                "dayOfWeek": f64::NAN
            }),
            vec![0xff, 0xff, 0xff, 0xff],
        ),
    ];

    for (_name, data_type, value, written) in cases {
        let mut buffalo = BuffaloZcl::new(vec![0; 10]);
        buffalo.write(data_type, &value, &json!({}));
        assert_eq!(buffalo.get_position(), written.len());
        assert_eq!(buffalo.get_written(), written);
    }
}

// Upstream: it.each([...])("Writes & Reads partial Non-Value for %s", ...)
#[test]
#[ignore = "not implemented yet"]
fn writes_reads_partial_non_value_for() {
    let cases: [(&str, u32, serde_json::Value, Vec<u8>); 2] = [
        (
            "time of day",
            DataType::TOD as u32,
            json!({"hours": 1, "minutes": 2, "seconds": f64::NAN, "hundredths": 3}),
            vec![1, 2, 0xff, 3],
        ),
        (
            "date",
            DataType::DATE as u32,
            json!({"year": 1901, "month": 2, "dayOfMonth": f64::NAN, "dayOfWeek": 3}),
            vec![1, 2, 0xff, 3],
        ),
    ];

    for (_name, data_type, value, written) in cases {
        let mut buffalo = BuffaloZcl::new(vec![0; 10]);
        buffalo.write(data_type, &value, &json!({}));
        assert_eq!(buffalo.get_position(), written.len());
        assert_eq!(buffalo.get_written(), written);

        buffalo.set_position(0);
        assert_eq!(buffalo.read(data_type, &json!({})), value);
        assert_eq!(buffalo.get_position(), written.len());
    }
}

// Upstream: it.each([...])("Writes & Reads Use Data Type for %s", ...)
#[test]
#[ignore = "not implemented yet"]
fn writes_reads_use_data_type_for() {
    let cases: [(&str, u32, serde_json::Value, usize, &str, &str); 3] = [
        (
            "uint32",
            DataType::UINT32 as u32,
            json!(32902534u64),
            4,
            "writeUInt32",
            "readUInt32",
        ),
        (
            "single prec",
            DataType::SINGLE_PREC as u32,
            json!(1.539989614439558e-36_f64),
            4,
            "writeFloatLE",
            "readFloatLE",
        ),
        (
            "IEEE address",
            DataType::IEEE_ADDR as u32,
            json!("0xfe1234abcd9876ff"),
            8,
            "writeIeeeAddr",
            "readIeeeAddr",
        ),
    ];

    for (_name, data_type, value, position, write_method, read_method) in cases {
        let options = json!({"dataType": data_type});
        let mut buffalo = BuffaloZcl::new(vec![0; 255]);

        buffalo.write(BuffaloZclDataType::USE_DATA_TYPE as u32, &value, &options);
        assert_eq!(
            buffalo.get_position(),
            position,
            "{write_method} should advance to {position}"
        );

        buffalo.set_position(0);
        assert_eq!(
            buffalo.read(BuffaloZclDataType::USE_DATA_TYPE as u32, &options),
            value,
            "{read_method} should round-trip"
        );
        assert_eq!(buffalo.get_position(), position);
    }
}

// Upstream: it.each([...])("Throws when read %s is missing required length option", ...)
#[test]
#[ignore = "not implemented yet"]
fn throws_when_read_is_missing_required_length_option() {
    let types = [
        BuffaloZclDataType::LIST_UINT8 as u32,
        BuffaloZclDataType::LIST_UINT16 as u32,
        BuffaloZclDataType::LIST_UINT24 as u32,
        BuffaloZclDataType::LIST_UINT32 as u32,
        BuffaloZclDataType::LIST_ZONEINFO as u32,
    ];

    for data_type in types {
        let result = std::panic::catch_unwind(|| {
            let mut buffalo = BuffaloZcl::new(vec![0; 1]);
            buffalo.read(data_type, &json!({}));
        });
        assert!(
            result.is_err(),
            "type {data_type} should throw without length"
        );
    }
}

// Upstream: it.each([...])("Writes & Reads Extension Field Sets for data type %s", ...)
#[test]
#[ignore = "not implemented yet"]
fn writes_reads_extension_field_sets_for_data_type() {
    let cases: [(&str, serde_json::Value, Vec<u8>); 4] = [
        (
            "6",
            json!([{"clstId": 6, "len": 1, "extField": [1]}]),
            vec![6, 0, 1, 1],
        ),
        (
            "8",
            json!([{"clstId": 8, "len": 1, "extField": [2]}]),
            vec![8, 0, 1, 2],
        ),
        (
            "258",
            json!([{"clstId": 258, "len": 2, "extField": [1, 2]}]),
            vec![2, 1, 2, 1, 2],
        ),
        (
            "768",
            json!([{"clstId": 768, "len": 13, "extField": [1, 2, 3, 4, 5, 6, 7, 8]}]),
            vec![0, 3, 13, 1, 0, 2, 0, 3, 0, 4, 5, 6, 7, 0, 8, 0],
        ),
    ];

    for (_name, value, written) in cases {
        let mut buffalo = BuffaloZcl::new(vec![0; written.len()]);
        buffalo.write(
            BuffaloZclDataType::EXTENSION_FIELD_SETS as u32,
            &value,
            &json!({}),
        );
        assert_eq!(buffalo.get_position(), written.len());
        assert_eq!(buffalo.get_written(), written);

        buffalo.set_position(0);
        assert_eq!(
            buffalo.read(BuffaloZclDataType::EXTENSION_FIELD_SETS as u32, &json!({})),
            value
        );
        assert_eq!(buffalo.get_position(), written.len());
    }
}

// Upstream: it.each([...])("Writes & Reads Thermo Transitions List", ...)
#[test]
#[ignore = "not implemented yet"]
fn writes_reads_thermo_transitions_list() {
    let cases: [(&str, serde_json::Value, serde_json::Value, Vec<u8>); 6] = [
        (
            "single all options",
            json!([{"transitionTime": 3, "heatSetpoint": 20, "coolSetpoint": 15}]),
            json!({"payload": {"mode": 0b11, "numoftrans": 1}}),
            vec![3, 0, 20, 0, 15, 0],
        ),
        (
            "single heat-only",
            json!([{"transitionTime": 60, "heatSetpoint": 25}]),
            json!({"payload": {"mode": 0b01, "numoftrans": 1}}),
            vec![60, 0, 25, 0],
        ),
        (
            "single cool-only",
            json!([{"transitionTime": 256, "coolSetpoint": 15}]),
            json!({"payload": {"mode": 0b10, "numoftrans": 1}}),
            vec![0, 1, 15, 0],
        ),
        (
            "multiple all options",
            json!([
                {"transitionTime": 3, "heatSetpoint": 20, "coolSetpoint": 15},
                {"transitionTime": 7, "heatSetpoint": 8, "coolSetpoint": 3},
                {"transitionTime": 257, "heatSetpoint": 256, "coolSetpoint": 0},
            ]),
            json!({"payload": {"mode": 0b11, "numoftrans": 3}}),
            vec![3, 0, 20, 0, 15, 0, 7, 0, 8, 0, 3, 0, 1, 1, 0, 1, 0, 0],
        ),
        (
            "multiple heat-only",
            json!([
                {"transitionTime": 3, "heatSetpoint": 20},
                {"transitionTime": 70, "heatSetpoint": 8},
            ]),
            json!({"payload": {"mode": 0b01, "numoftrans": 2}}),
            vec![3, 0, 20, 0, 70, 0, 8, 0],
        ),
        (
            "multiple cool-only",
            json!([
                {"transitionTime": 3, "coolSetpoint": 15},
                {"transitionTime": 65000, "coolSetpoint": 3},
            ]),
            json!({"payload": {"mode": 0b10, "numoftrans": 2}}),
            vec![3, 0, 15, 0, 0xe8, 0xfd, 3, 0],
        ),
    ];

    for (_name, value, read_options, written) in cases {
        let mut buffalo = BuffaloZcl::new(vec![0; 50]);
        buffalo.write(
            BuffaloZclDataType::LIST_THERMO_TRANSITIONS as u32,
            &value,
            &json!({}),
        );
        assert_eq!(buffalo.get_position(), written.len());
        assert_eq!(buffalo.get_written(), written);

        buffalo.set_position(0);
        assert_eq!(
            buffalo.read(
                BuffaloZclDataType::LIST_THERMO_TRANSITIONS as u32,
                &read_options,
            ),
            value
        );
        assert_eq!(buffalo.get_position(), written.len());
    }
}

// Upstream: it.each([...])("Writes & Reads Structured Selector for %s", ...)
#[test]
#[ignore = "not implemented yet"]
fn writes_reads_structured_selector_for() {
    let cases: [(&str, serde_json::Value, Vec<u8>); 2] = [
        (
            "whole",
            json!({"indicatorType": StructuredIndicatorType::Whole as u8}),
            vec![StructuredIndicatorType::Whole as u8],
        ),
        (
            "indexes only",
            json!({"indexes": [3, 4, 5, 256]}),
            vec![4, 3, 0, 4, 0, 5, 0, 0, 1],
        ),
    ];

    for (_name, value, written) in cases {
        let mut buffalo = BuffaloZcl::new(vec![0; 50]);
        buffalo.write(
            BuffaloZclDataType::STRUCTURED_SELECTOR as u32,
            &value,
            &json!({}),
        );
        assert_eq!(buffalo.get_position(), written.len());
        assert_eq!(buffalo.get_written(), written);

        buffalo.set_position(0);
        assert_eq!(
            buffalo.read(BuffaloZclDataType::STRUCTURED_SELECTOR as u32, &json!({})),
            value
        );
        assert_eq!(buffalo.get_position(), written.len());
    }
}

// Upstream: it.each([...])("Writes Structured Selector for %s", ...)
#[test]
#[ignore = "not implemented yet"]
fn writes_structured_selector_for() {
    let cases: [(&str, u8); 2] = [
        ("Add", StructuredIndicatorType::WriteAdd as u8),
        ("Remove", StructuredIndicatorType::WriteRemove as u8),
    ];

    for (_name, indicator) in cases {
        let value = json!({"indicatorType": indicator});
        let written = vec![indicator];
        let mut buffalo = BuffaloZcl::new(vec![0; 50]);
        buffalo.write(
            BuffaloZclDataType::STRUCTURED_SELECTOR as u32,
            &value,
            &json!({}),
        );
        assert_eq!(buffalo.get_position(), written.len());
        assert_eq!(buffalo.get_written(), written);
    }
}

// Upstream: it.each([...])("Writes & Reads Tuya Data Point Values List %s", ...)
#[test]
#[ignore = "not implemented yet"]
fn writes_reads_tuya_data_point_values_list() {
    let cases: [(&str, serde_json::Value, Vec<u8>); 2] = [
        (
            "single",
            json!([{"dp": 254, "datatype": 125, "data": [1, 3, 5]}]),
            vec![254, 125, 0, 3, 1, 3, 5],
        ),
        (
            "multiple",
            json!([
                {"dp": 254, "datatype": 125, "data": [1, 3, 5]},
                {"dp": 125, "datatype": 254, "data": [5, 0, 1, 5]},
            ]),
            vec![254, 125, 0, 3, 1, 3, 5, 125, 254, 0, 4, 5, 0, 1, 5],
        ),
    ];

    for (_name, value, written) in cases {
        let mut buffalo = BuffaloZcl::new(vec![0; written.len()]);
        buffalo.write(
            BuffaloZclDataType::LIST_TUYA_DATAPOINT_VALUES as u32,
            &value,
            &json!({}),
        );
        assert_eq!(buffalo.get_position(), written.len());
        assert_eq!(buffalo.get_written(), written);

        buffalo.set_position(0);
        assert_eq!(
            buffalo.read(
                BuffaloZclDataType::LIST_TUYA_DATAPOINT_VALUES as u32,
                &json!({}),
            ),
            value
        );
        assert_eq!(buffalo.get_position(), written.len());
    }
}
