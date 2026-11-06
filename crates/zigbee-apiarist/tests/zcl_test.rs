// Original: .upstream/zigbee-herdsman/test/zcl.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - BuffaloZcl read ARRAY non-value
//   - BuffaloZcl read BIG_ENDIAN_UINT24
//   - BuffaloZcl read BUFFER whole
//   - BuffaloZcl read BUFFER with length
//   - BuffaloZcl read CHAR_STR non-value
//   - BuffaloZcl read DATE
//   - BuffaloZcl read DATE non-value
//   - BuffaloZcl read LONG_CHAR_STR non-value
//   - BuffaloZcl read LONG_OCTET_STR non-value
//   - BuffaloZcl read MI_STRUCT non-value
//   - BuffaloZcl read NO_DATA
//   - BuffaloZcl read OCTET_STR non-value
//   - BuffaloZcl read STRUCT non-value
//   - BuffaloZcl read TOD
//   - BuffaloZcl read UNKNOWN
//   - BuffaloZcl read array
//   - BuffaloZcl read extensionFieldSets
//   - BuffaloZcl read int40
//   - BuffaloZcl read int56
//   - BuffaloZcl read int64
//   - BuffaloZcl read list zoneinfo
//   - BuffaloZcl read longCharStr
//   - BuffaloZcl read longOctetStr
//   - BuffaloZcl read non existing type
//   - BuffaloZcl read passthrough to Buffalo
//   - BuffaloZcl read struct
//   - BuffaloZcl write BIG_ENDIAN_UINT24
//   - BuffaloZcl write BUFFER always whole - length ignored
//   - BuffaloZcl write BUFFER whole
//   - BuffaloZcl write DATE
//   - BuffaloZcl write NO_DATA
//   - BuffaloZcl write TOD
//   - BuffaloZcl write TOD non-value
//   - BuffaloZcl write UNKNOWN
//   - BuffaloZcl write array element type numeric
//   - BuffaloZcl write array element type numeric passed as string of DataType
//   - BuffaloZcl write array element type string
//   - BuffaloZcl write charStr
//   - BuffaloZcl write extensionFieldSets
//   - BuffaloZcl write int40
//   - BuffaloZcl write int56
//   - BuffaloZcl write int64
//   - BuffaloZcl write longCharStr
//   - BuffaloZcl write longOctetStr
//   - BuffaloZcl write non existing type
//   - BuffaloZcl write octetStr
//   - BuffaloZcl write passthrough to Buffalo
//   - BuffaloZcl write security key
//   - BuffaloZcl write struct
//   - BuffaloZcl write zoneinfo
//   - Cluster has attribute
//   - Get cluster attribute by ID
//   - Get cluster by ID
//   - Get cluster by id non-existing
//   - Get cluster by name
//   - Get cluster by name non-existing
//   - Get global command by name
//   - Get global command non existing
//   - Get specific command by ID
//   - Get specific command by name
//   - Get specific command by name non existing
//   - Get specific command by name server to client
//   - Reads as buffer when USE_DATA_TYPE is missing dataType option
//   - Throws when read GPD_FRAME is missing payload.payloadSize option
//   - Throws when read LIST_THERMO_TRANSITIONS is missing required payload options
//   - Throws when write USE_DATA_TYPE is missing dataType option and value isnt buffer or number array
//   - Writes as buffer when USE_DATA_TYPE is missing dataType option
//   - Zcl
//   - Zcl green power readGpd attribute report
//   - Zcl green power readGpd channel request
//   - Zcl green power readGpd commissioning
//   - Zcl green power readGpd commissioning all options
//   - Zcl green power writeGpd channel configuration
//   - Zcl green power writeGpd channel configuration basic
//   - Zcl green power writeGpd commissioning
//   - Zcl green power writeGpd commissioning all options
//   - Zcl green power writeGpd custom reply
//   - Zcl green power writeGpd unhandled command
//   - Zcl utils get attribute
//   - Zcl utils get attribute non-existing
//   - Zcl utils get cluster attributes manufacturerCode
//   - Zcl utils get cluster attributes manufacturerCode wrong
//   - Zcl utils get cluster manufacturerCode
//   - Zcl utils get cluster manufacturerCode wrong
//   - Zcl utils get cluster with manufacturerCode
//   - Zcl utils get cluster without manufacturerCode
//   - Zcl utils get command
//   - Zcl utils get command non-existing
//   - ZclFrame from buffer GPD commission
//   - ZclFrame from buffer GPD pairing
//   - ZclFrame from buffer GPD scene 0
//   - ZclFrame from buffer GPD with extra data
//   - ZclFrame from buffer configReportRsp (hvacThermostat)
//   - ZclFrame from buffer configReportRsp - long
//   - ZclFrame from buffer configReportRsp - short
//   - ZclFrame from buffer configReportRsp client to server analog
//   - ZclFrame from buffer configReportRsp failed
//   - ZclFrame from buffer configReportRsp server to client
//   - ZclFrame from buffer defaultRsp
//   - ZclFrame from buffer discoverRsp
//   - ZclFrame from buffer error on malformed
//   - ZclFrame from buffer genGroups getMembership
//   - ZclFrame from buffer genOta queryNextImageRequest keeps 0xFFFF manufacturerCode
//   - ZclFrame from buffer genOta upgradeEndRequest keeps 0xFFFF manufacturerCode
//   - ZclFrame from buffer getWeeklyScheduleRsp (hvacThermostat)
//   - ZclFrame from buffer occupancy report
//   - ZclFrame from buffer parse payload with unknown frame type
//   - ZclFrame from buffer readRsp
//   - ZclFrame from buffer readRsp alias type
//   - ZclFrame from buffer readRsp failed
//   - ZclFrame from buffer readRsp success
//   - ZclFrame from buffer readStructured multiple elements
//   - ZclFrame from buffer readStructured single elements
//   - ZclFrame from buffer readStructured whole attribute
//   - ZclFrame from buffer report
//   - ZclFrame from buffer ssIasAce arm command
//   - ZclFrame from buffer struct
//   - ZclFrame from buffer with reservered bits
//   - ZclFrame from buffer writeStructuredRsp failed mixed selectors
//   - ZclFrame from buffer writeStructuredRsp failed selector unknown or none for type
//   - ZclFrame from buffer writeStructuredRsp failed with selector
//   - ZclFrame from buffer writeStructuredRsp success
//   - ZclFrame parse MiBoxer zone configuration command
//   - ZclFrame serialize MiBoxer zone configuration command
//   - ZclFrame to buffer defaultRsp success
//   - ZclFrame to buffer discover
//   - ZclFrame to buffer discoverRsp
//   - ZclFrame to buffer genScenes.add
//   - ZclFrame to buffer invalid frametype
//   - ZclFrame to buffer offWithEffect
//   - ZclFrame to buffer queryNextImageResponse with non zero status
//   - ZclFrame to buffer queryNextImageResponse with zero status
//   - ZclFrame to buffer queryNextImageResponse with zero status and missing parameters
//   - ZclFrame to buffer readRsp UTC
//   - ZclFrame to buffer readRsp failed
//   - ZclFrame to buffer readRsp success
//   - ZclFrame to buffer readStructured multiple elements
//   - ZclFrame to buffer readStructured single element
//   - ZclFrame to buffer readStructured whole attribute
//   - ZclFrame to buffer setWeeklyScheduleRsp (hvacThermostat)
//   - ZclFrame to buffer with reservered bits
//   - ZclFrame to buffer write Livolo malformed
//   - ZclFrame to buffer writeStructured Livolo malformed
//   - ZclFrame to buffer writeStructured add element into set/bag
//   - ZclFrame to buffer writeStructured multiple elements
//   - ZclFrame to buffer writeStructured remove element from set/bag
//   - ZclFrame to buffer writeStructured single element
//   - ZclFrame to buffer writeStructured whole attribute
//   - ZclFrame to buffer writeStructuredRsp failed mixed selectors
//   - ZclFrame to buffer writeStructuredRsp failed selector unknown or none for type
//   - ZclFrame to buffer writeStructuredRsp failed with selector
//   - ZclFrame to buffer writeStructuredRsp success
//   - ZclFrame with Assa (manufacturer specific) cluster create
//   - ZclFrame with Assa (manufacturer specific) cluster create with non Assamanufcode
//   - ZclFrame with Assa (manufacturer specific) cluster fromBuffer
//   - ZclFrame write request with string as bytes array
//   - ZclFrame write rsp
//   - ZclHeader return undefined when too short
//   - ZclHeader return undefined when too short manufacturer specific
//   - double prec
//   - ieee add
//   - int16
//   - int24
//   - int32
//   - int48
//   - int8
//   - list uint16
//   - list uint24
//   - list uint32
//   - list uint8
//   - semi prec
//   - single prec
//   - uint16
//   - uint24
//   - uint32
//   - uint8

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_array_non_value() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_big_endian_uint24() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_buffer_whole() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_buffer_with_length() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_char_str_non_value() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_date() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_date_non_value() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_long_char_str_non_value() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_long_octet_str_non_value() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_mi_struct_non_value() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_no_data() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_octet_str_non_value() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_struct_non_value() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_tod() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_unknown() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_array() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_extension_field_sets() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_int40() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_int56() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_int64() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_list_zoneinfo() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_long_char_str() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_long_octet_str() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_non_existing_type() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_passthrough_to_buffalo() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_read_struct() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_big_endian_uint24() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_buffer_always_whole_length_ignored() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_buffer_whole() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_date() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_no_data() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_tod() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_tod_non_value() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_unknown() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_array_element_type_numeric() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_array_element_type_numeric_passed_as_string_of_data_type() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_array_element_type_string() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_char_str() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_extension_field_sets() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_int40() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_int56() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_int64() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_long_char_str() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_long_octet_str() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_non_existing_type() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_octet_str() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_passthrough_to_buffalo() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_security_key() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_struct() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn buffalo_zcl_write_zoneinfo() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn cluster_has_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_cluster_attribute_by_id() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_cluster_by_id() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_cluster_by_id_non_existing() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_cluster_by_name() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_cluster_by_name_non_existing() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_global_command_by_name() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_global_command_non_existing() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_specific_command_by_id() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_specific_command_by_name() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_specific_command_by_name_non_existing() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_specific_command_by_name_server_to_client() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_as_buffer_when_use_data_type_is_missing_data_type_option() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_read_gpd_frame_is_missing_payload_payload_size_option() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_read_list_thermo_transitions_is_missing_required_payload_options() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_write_use_data_type_is_missing_data_type_option_and_value_isnt_buffer_or_number_array(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_as_buffer_when_use_data_type_is_missing_data_type_option() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_read_gpd_attribute_report() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_read_gpd_channel_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_read_gpd_commissioning() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_read_gpd_commissioning_all_options() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_write_gpd_channel_configuration() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_write_gpd_channel_configuration_basic() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_write_gpd_commissioning() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_write_gpd_commissioning_all_options() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_write_gpd_custom_reply() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_green_power_write_gpd_unhandled_command() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_attribute_non_existing() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_cluster_attributes_manufacturer_code() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_cluster_attributes_manufacturer_code_wrong() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_cluster_manufacturer_code() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_cluster_manufacturer_code_wrong() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_cluster_with_manufacturer_code() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_cluster_without_manufacturer_code() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_command() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_utils_get_command_non_existing() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_gpd_commission() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_gpd_pairing() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_gpd_scene_0() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_gpd_with_extra_data() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_config_report_rsp_hvac_thermostat() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_config_report_rsp_long() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_config_report_rsp_short() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_config_report_rsp_client_to_server_analog() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_config_report_rsp_failed() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_config_report_rsp_server_to_client() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_default_rsp() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_discover_rsp() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_error_on_malformed() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_gen_groups_get_membership() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_gen_ota_query_next_image_request_keeps_0x_ffff_manufacturer_code() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_gen_ota_upgrade_end_request_keeps_0x_ffff_manufacturer_code() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_get_weekly_schedule_rsp_hvac_thermostat() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_occupancy_report() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_parse_payload_with_unknown_frame_type() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_read_rsp() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_read_rsp_alias_type() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_read_rsp_failed() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_read_rsp_success() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_read_structured_multiple_elements() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_read_structured_single_elements() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_read_structured_whole_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_report() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_ss_ias_ace_arm_command() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_struct() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_with_reservered_bits() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_write_structured_rsp_failed_mixed_selectors() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_write_structured_rsp_failed_selector_unknown_or_none_for_type() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_write_structured_rsp_failed_with_selector() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_from_buffer_write_structured_rsp_success() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_parse_mi_boxer_zone_configuration_command() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_serialize_mi_boxer_zone_configuration_command() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_default_rsp_success() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_discover() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_discover_rsp() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_gen_scenes_add() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_invalid_frametype() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_off_with_effect() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_query_next_image_response_with_non_zero_status() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_query_next_image_response_with_zero_status() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_query_next_image_response_with_zero_status_and_missing_parameters() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_read_rsp_utc() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_read_rsp_failed() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_read_rsp_success() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_read_structured_multiple_elements() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_read_structured_single_element() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_read_structured_whole_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_set_weekly_schedule_rsp_hvac_thermostat() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_with_reservered_bits() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_livolo_malformed() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_livolo_malformed() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_add_element_into_set_bag() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_multiple_elements() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_remove_element_from_set_bag() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_single_element() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_whole_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_rsp_failed_mixed_selectors() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_rsp_failed_selector_unknown_or_none_for_type() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_rsp_failed_with_selector() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_to_buffer_write_structured_rsp_success() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_with_assa_manufacturer_specific_cluster_create() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_with_assa_manufacturer_specific_cluster_create_with_non_assamanufcode() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_with_assa_manufacturer_specific_cluster_from_buffer() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_write_request_with_string_as_bytes_array() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_frame_write_rsp() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_header_return_undefined_when_too_short() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zcl_header_return_undefined_when_too_short_manufacturer_specific() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn double_prec() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ieee_add() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn int16() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn int24() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn int32() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn int48() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn int8() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint16() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint24() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint32() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn list_uint8() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn semi_prec() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn single_prec() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uint16() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uint24() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uint32() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uint8() {
    todo!()
}
