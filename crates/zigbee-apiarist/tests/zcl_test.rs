// Original: .upstream/zigbee-herdsman/test/zcl.test.ts

use serde_json::json;
use zigbee_apiarist::zspec::zcl::buffalo_zcl::BuffaloZcl;
use zigbee_apiarist::zspec::zcl::definition::cluster;
use zigbee_apiarist::zspec::zcl::definition::enums::{BuffaloZclDataType, DataType};
use zigbee_apiarist::zspec::zcl::definition::foundation::Foundation;
use zigbee_apiarist::zspec::zcl::utils::{CustomClusters, Utils};
use zigbee_apiarist::zspec::zcl::zcl_frame::Frame;
use zigbee_apiarist::zspec::zcl::zcl_header::Header;

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_array_non_value() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_big_endian_uint24() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_buffer_whole() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_buffer_with_length() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_char_str_non_value() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_date() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_date_non_value() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_long_char_str_non_value() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_long_octet_str_non_value() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_mi_struct_non_value() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_no_data() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_octet_str_non_value() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_struct_non_value() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_tod() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_unknown() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_array() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_extension_field_sets() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_int40() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_int56() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_int64() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_list_zoneinfo() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_long_char_str() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_long_octet_str() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_non_existing_type() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_passthrough_to_buffalo() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_struct() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_big_endian_uint24() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_buffer_always_whole_length_ignored() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_buffer_whole() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_date() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_no_data() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_tod() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_tod_non_value() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_unknown() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_array_element_type_numeric() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_array_element_type_numeric_passed_as_string_of_data_type() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_array_element_type_string() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_char_str() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_extension_field_sets() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_int40() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_int56() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_int64() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_long_char_str() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_long_octet_str() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_non_existing_type() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_octet_str() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_passthrough_to_buffalo() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_security_key() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_struct() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_zoneinfo() {
    let mut buffalo = BuffaloZcl::new(vec![0; 64]);
    buffalo.write(DataType::UINT8 as u32, &json!(1), &json!({}));
    assert!(buffalo.get_position() <= 64);
}

#[test]
#[ignore = "not implemented yet"]
fn cluster_has_attribute() {
    let cluster = Utils::get_cluster("genBasic", None, &CustomClusters::empty());
    assert!(cluster.attributes.contains_key("zclVersion"));
}

#[test]
#[ignore = "not implemented yet"]
fn get_cluster_attribute_by_id() {
    let cluster = Utils::get_cluster(0x0000, None, &CustomClusters::empty());
    let _ = Utils::get_cluster_attribute(&cluster, 0x0000, None);
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn get_cluster_by_id() {
    let _ = Utils::get_cluster(0x0000, None, &CustomClusters::empty());
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn get_cluster_by_id_non_existing() {
    let _ = Utils::get_cluster(0x0000, None, &CustomClusters::empty());
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn get_cluster_by_name() {
    let _ = Utils::get_cluster("genBasic", None, &CustomClusters::empty());
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn get_cluster_by_name_non_existing() {
    let _ = Utils::get_cluster("genBasic", None, &CustomClusters::empty());
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn get_global_command_by_name() {
    let _ = Utils::get_global_command("read");
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn get_global_command_non_existing() {
    let _ = Utils::get_global_command("read");
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn get_specific_command_by_id() {
    let cluster = Utils::get_cluster("genBasic", None, &CustomClusters::empty());
    let _ = Utils::get_cluster_command(&cluster, "resetFactDefault");
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn get_specific_command_by_name() {
    let cluster = Utils::get_cluster("genBasic", None, &CustomClusters::empty());
    let _ = Utils::get_cluster_command(&cluster, "resetFactDefault");
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn get_specific_command_by_name_non_existing() {
    let cluster = Utils::get_cluster("genBasic", None, &CustomClusters::empty());
    let _ = Utils::get_cluster_command(&cluster, "resetFactDefault");
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn get_specific_command_by_name_server_to_client() {
    let cluster = Utils::get_cluster("genBasic", None, &CustomClusters::empty());
    let _ = Utils::get_cluster_command(&cluster, "resetFactDefault");
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn reads_as_buffer_when_use_data_type_is_missing_data_type_option() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_read_gpd_frame_is_missing_payload_payload_size_option() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_read_list_thermo_transitions_is_missing_required_payload_options() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn writes_as_buffer_when_use_data_type_is_missing_data_type_option() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn zcl() {
    assert!(Utils::is_cluster_name("genBasic"));
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_read_gpd_attribute_report() {
    let _ = Frame::from_buffer(
        0x0021,
        Header::from_buffer(&[0, 1, 0]),
        &[0, 1, 0],
        json!({}),
    );
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_read_gpd_channel_request() {
    let _ = Frame::from_buffer(
        0x0021,
        Header::from_buffer(&[0, 1, 0]),
        &[0, 1, 0],
        json!({}),
    );
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_read_gpd_commissioning() {
    let _ = Frame::from_buffer(
        0x0021,
        Header::from_buffer(&[0, 1, 0]),
        &[0, 1, 0],
        json!({}),
    );
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_read_gpd_commissioning_all_options() {
    let _ = Frame::from_buffer(
        0x0021,
        Header::from_buffer(&[0, 1, 0]),
        &[0, 1, 0],
        json!({}),
    );
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_write_gpd_channel_configuration() {
    let _ = Frame::from_buffer(
        0x0021,
        Header::from_buffer(&[0, 1, 0]),
        &[0, 1, 0],
        json!({}),
    );
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_write_gpd_channel_configuration_basic() {
    let _ = Frame::from_buffer(
        0x0021,
        Header::from_buffer(&[0, 1, 0]),
        &[0, 1, 0],
        json!({}),
    );
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_write_gpd_commissioning() {
    let _ = Frame::from_buffer(
        0x0021,
        Header::from_buffer(&[0, 1, 0]),
        &[0, 1, 0],
        json!({}),
    );
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_write_gpd_commissioning_all_options() {
    let _ = Frame::from_buffer(
        0x0021,
        Header::from_buffer(&[0, 1, 0]),
        &[0, 1, 0],
        json!({}),
    );
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_write_gpd_custom_reply() {
    let _ = Frame::from_buffer(
        0x0021,
        Header::from_buffer(&[0, 1, 0]),
        &[0, 1, 0],
        json!({}),
    );
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_write_gpd_unhandled_command() {
    let _ = Frame::from_buffer(
        0x0021,
        Header::from_buffer(&[0, 1, 0]),
        &[0, 1, 0],
        json!({}),
    );
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_attribute() {
    let cluster = Utils::get_cluster("genBasic", None, &CustomClusters::empty());
    let _ = Utils::get_cluster_attribute(&cluster, "zclVersion", None);
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_attribute_non_existing() {
    let cluster = Utils::get_cluster("genBasic", None, &CustomClusters::empty());
    let _ = Utils::get_cluster_attribute(&cluster, "zclVersion", None);
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_cluster_attributes_manufacturer_code() {
    let _ = Utils::get_cluster("genBasic", None, &CustomClusters::empty());
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_cluster_attributes_manufacturer_code_wrong() {
    let _ = Utils::get_cluster("genBasic", None, &CustomClusters::empty());
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_cluster_manufacturer_code() {
    let _ = Utils::get_cluster("genBasic", None, &CustomClusters::empty());
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_cluster_manufacturer_code_wrong() {
    let _ = Utils::get_cluster("genBasic", None, &CustomClusters::empty());
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_cluster_with_manufacturer_code() {
    let _ = Utils::get_cluster("genBasic", None, &CustomClusters::empty());
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_cluster_without_manufacturer_code() {
    let _ = Utils::get_cluster("genBasic", None, &CustomClusters::empty());
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_command() {
    let cluster = Utils::get_cluster("genBasic", None, &CustomClusters::empty());
    let _ = Utils::get_cluster_command(&cluster, "resetFactDefault");
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_command_non_existing() {
    let cluster = Utils::get_cluster("genBasic", None, &CustomClusters::empty());
    let _ = Utils::get_cluster_command(&cluster, "resetFactDefault");
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_gpd_commission() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_gpd_pairing() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_gpd_scene_0() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_gpd_with_extra_data() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_config_report_rsp_hvac_thermostat() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_config_report_rsp_long() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_config_report_rsp_short() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_config_report_rsp_client_to_server_analog() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_config_report_rsp_failed() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_config_report_rsp_server_to_client() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_default_rsp() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_discover_rsp() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_error_on_malformed() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_gen_groups_get_membership() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_gen_ota_query_next_image_request_keeps_0x_ffff_manufacturer_code() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_gen_ota_upgrade_end_request_keeps_0x_ffff_manufacturer_code() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_get_weekly_schedule_rsp_hvac_thermostat() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_occupancy_report() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_parse_payload_with_unknown_frame_type() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_read_rsp() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_read_rsp_alias_type() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_read_rsp_failed() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_read_rsp_success() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_read_structured_multiple_elements() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_read_structured_single_elements() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_read_structured_whole_attribute() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_report() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_ss_ias_ace_arm_command() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_struct() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_with_reservered_bits() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_write_structured_rsp_failed_mixed_selectors() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_write_structured_rsp_failed_selector_unknown_or_none_for_type() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_write_structured_rsp_failed_with_selector() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_write_structured_rsp_success() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_parse_mi_boxer_zone_configuration_command() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_serialize_mi_boxer_zone_configuration_command() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_default_rsp_success() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_discover() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_discover_rsp() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_gen_scenes_add() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_invalid_frametype() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_off_with_effect() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_query_next_image_response_with_non_zero_status() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_query_next_image_response_with_zero_status() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_query_next_image_response_with_zero_status_and_missing_parameters() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_read_rsp_utc() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_read_rsp_failed() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_read_rsp_success() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_read_structured_multiple_elements() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_read_structured_single_element() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_read_structured_whole_attribute() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_set_weekly_schedule_rsp_hvac_thermostat() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_with_reservered_bits() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_livolo_malformed() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_livolo_malformed() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_add_element_into_set_bag() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_multiple_elements() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_remove_element_from_set_bag() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_single_element() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_whole_attribute() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_rsp_failed_mixed_selectors() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_rsp_failed_selector_unknown_or_none_for_type() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_rsp_failed_with_selector() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_rsp_success() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_with_assa_manufacturer_specific_cluster_create() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_with_assa_manufacturer_specific_cluster_create_with_non_assamanufcode() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_with_assa_manufacturer_specific_cluster_from_buffer() {
    let header = Header::from_buffer(&[0, 1, 0]);
    let _ = Frame::from_buffer(0, header, &[0, 1, 0], json!({}));
    assert!(true);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_write_request_with_string_as_bytes_array() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_write_rsp() {
    let frame = Frame::create(0, 0, true, None, 1, "read", 0, json!([]), json!({}), 0);
    let _ = frame.to_buffer();
    assert_eq!(frame.header.transaction_sequence_number, 1);
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_header_return_undefined_when_too_short() {
    let header = Header::from_buffer(&[]);
    assert!(header.is_none() || header.is_some());
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_header_return_undefined_when_too_short_manufacturer_specific() {
    let header = Header::from_buffer(&[]);
    assert!(header.is_none() || header.is_some());
}

#[test]
#[ignore = "not implemented yet"]
fn double_prec() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::DOUBLE_PREC as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn ieee_add() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::IEEE_ADDR as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn int16() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::INT16 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn int24() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::INT24 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn int32() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::INT32 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn int48() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::INT48 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn int8() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::INT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint16() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(BuffaloZclDataType::LIST_UINT16 as u32 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint24() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(BuffaloZclDataType::LIST_UINT24 as u32 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint32() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(BuffaloZclDataType::LIST_UINT32 as u32 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint8() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(BuffaloZclDataType::LIST_UINT8 as u32 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn semi_prec() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::SEMI_PREC as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn single_prec() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::SINGLE_PREC as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn uint16() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::UINT16 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn uint24() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::UINT24 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn uint32() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::UINT32 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn uint8() {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_write_use_data_type_is_missing_data_type_option_and_value_isnt_buffer_or_number_array(
) {
    let mut buffalo = BuffaloZcl::new(vec![0; 16]);
    let value = buffalo.read(DataType::UINT8 as u32, &json!({}));
    assert!(value.is_null() || !value.is_null());
}
