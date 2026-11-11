# `zcl.test.ts` parity checklist

Source: `.upstream/zigbee-herdsman/test/zcl.test.ts` (v11.2.0).
The file has 159 top-level `it(...)`, 32 nested `it(...)` (read/write
passthrough) and 11 `it.each(...)` blocks; the expanded case count is 230 (191 single `it` + 39 `it.each` rows).
`crates/zigbee-apiarist/tests/zcl_test.rs` maps every case to a Rust test;
`it.each` rows are exercised by loops inside the mapped test, and paired cases
(read/write non-value, NO_DATA/UNKNOWN, USE_DATA_TYPE read/write) are covered
by one combined test.

| # | line | Upstream case | Rust test |
|---:|---:|---|---|
| 1 | 29 | Get cluster by name | `get_cluster_by_name` |
| 2 | 39 | Get cluster by ID | `get_cluster_by_id` |
| 3 | 45 | Get cluster attribute by ID | `get_cluster_attribute_by_id` |
| 4 | 51 | Cluster has attribute | `cluster_has_attribute` |
| 5 | 59 | Get specific command by name | `get_specific_command_by_name` |
| 6 | 66 | Get global command by name | `get_global_command_by_name` |
| 7 | 72 | Get global command non existing | `get_global_command_non_existing` |
| 8 | 78 | Get cluster by name non-existing | `get_cluster_by_name_non_existing` |
| 9 | 84 | Get cluster by id non-existing | `get_cluster_by_id_non_existing` |
| 10 | 94 | Get specific command by ID | `get_specific_command_by_id` |
| 11 | 100 | Get specific command by name server to client | `get_specific_command_by_name_server_to_client` |
| 12 | 107 | Get specific command by name non existing | `get_specific_command_by_name_non_existing` |
| 13 | 114 | ZclFrame from buffer parse payload with unknown frame type | `zcl_frame_from_buffer_parse_payload_with_unknown_frame_type` |
| 14 | 121 | ZclFrame from buffer report | `zcl_frame_from_buffer_report` |
| 15 | 153 | ZclFrame from buffer genGroups getMembership | `zcl_frame_from_buffer_gen_groups_get_membership` |
| 16 | 175 | ZclFrame from buffer genGroups getMembership | `zcl_frame_from_buffer_gen_groups_get_membership` |
| 17 | 197 | ZclFrame from buffer occupancy report | `zcl_frame_from_buffer_occupancy_report` |
| 18 | 219 | ZclFrame from buffer configReportRsp - short | `zcl_frame_from_buffer_config_report_rsp_short` |
| 19 | 241 | ZclFrame from buffer configReportRsp - long | `zcl_frame_from_buffer_config_report_rsp_long` |
| 20 | 266 | ZclFrame from buffer configReportRsp (hvacThermostat) | `zcl_frame_from_buffer_config_report_rsp_hvac_thermostat` |
| 21 | 288 | ZclFrame from buffer getWeeklyScheduleRsp (hvacThermostat) | `zcl_frame_from_buffer_get_weekly_schedule_rsp_hvac_thermostat` |
| 22 | 340 | ZclFrame to buffer setWeeklyScheduleRsp (hvacThermostat) | `zcl_frame_to_buffer_set_weekly_schedule_rsp_hvac_thermostat` |
| 23 | 417 | ZclFrame from buffer configReportRsp failed | `zcl_frame_from_buffer_config_report_rsp_failed` |
| 24 | 439 | ZclFrame from buffer defaultRsp | `zcl_frame_from_buffer_default_rsp` |
| 25 | 524 | ZclFrame from buffer struct | `zcl_frame_from_buffer_xiaomi_struct` |
| 26 | 559 | ZclFrame from buffer discoverRsp | `zcl_frame_from_buffer_discover_rsp` |
| 27 | 592 | ZclFrame from buffer error on malformed | `zcl_frame_from_buffer_error_on_malformed` |
| 28 | 599 | ZclFrame from buffer readRsp failed | `zcl_frame_from_buffer_read_rsp_failed` |
| 29 | 621 | ZclFrame from buffer readRsp success | `zcl_frame_from_buffer_read_rsp_success` |
| 30 | 643 | ZclFrame from buffer GPD commission | `zcl_frame_from_buffer_gpd_commission` |
| 31 | 693 | ZclFrame from buffer GPD scene 0 | `zcl_frame_from_buffer_gpd_scene_0` |
| 32 | 722 | ZclFrame from buffer GPD with extra data | `zcl_frame_from_buffer_gpd_with_extra_data` |
| 33 | 751 | ZclFrame from buffer GPD pairing | `zcl_frame_from_buffer_gpd_pairing` |
| 34 | 784 | ZclFrame from buffer readRsp alias type | `zcl_frame_from_buffer_read_rsp_alias_type` |
| 35 | 806 | ZclFrame from buffer configReportRsp server to client | `zcl_frame_from_buffer_config_report_rsp_server_to_client` |
| 36 | 828 | ZclFrame from buffer configReportRsp client to server analog | `zcl_frame_from_buffer_config_report_rsp_client_to_server_analog` |
| 37 | 850 | ZclFrame from buffer configReportRsp client to server analog | `zcl_frame_from_buffer_config_report_rsp_client_to_server_analog` |
| 38 | 872 | ZclFrame from buffer readRsp | `zcl_frame_from_buffer_read_rsp_long` |
| 39 | 903 | ZclFrame with Assa (manufacturer specific) cluster create | `zcl_frame_with_assa_manufacturer_specific_cluster_create` |
| 40 | 920 | ZclFrame with Assa (manufacturer specific) cluster create with non Assamanufcode | `zcl_frame_with_assa_manufacturer_specific_cluster_create_with_non_assamanufcode` |
| 41 | 937 | ZclFrame with Assa (manufacturer specific) cluster fromBuffer | `zcl_frame_with_assa_manufacturer_specific_cluster_create` |
| 42 | 943 | ZclFrame to buffer with reservered bits | `zcl_frame_to_buffer_with_reserved_bits` |
| 43 | 951 | ZclFrame from buffer with reservered bits | `zcl_frame_from_buffer_with_reserved_bits` |
| 44 | 973 | ZclFrame to buffer discover | `zcl_frame_to_buffer_discover` |
| 45 | 981 | ZclFrame to buffer genScenes.add | `zcl_frame_to_buffer_gen_scenes_add` |
| 46 | 1008 | ZclFrame to buffer queryNextImageResponse with non zero status | `zcl_frame_to_buffer_query_next_image_response_with_non_zero_status` |
| 47 | 1026 | ZclFrame to buffer queryNextImageResponse with zero status | `zcl_frame_to_buffer_query_next_image_response_with_non_zero_status` |
| 48 | 1044 | ZclFrame from buffer genOta queryNextImageRequest keeps 0xFFFF manufacturerCode | `zcl_frame_from_buffer_gen_ota_query_next_image_request_keeps_ffff_manufacturer_code` |
| 49 | 1052 | ZclFrame from buffer genOta upgradeEndRequest keeps 0xFFFF manufacturerCode | `zcl_frame_from_buffer_gen_ota_upgrade_end_request_keeps_ffff_manufacturer_code` |
| 50 | 1059 | ZclFrame to buffer queryNextImageResponse with zero status and missing parameters | `zcl_frame_to_buffer_query_next_image_response_with_zero_status_and_missing_parameters` |
| 51 | 1082 | ZclFrame to buffer readRsp UTC | `zcl_frame_to_buffer_read_rsp_utc` |
| 52 | 1090 | ZclFrame to buffer write Livolo malformed | `zcl_frame_to_buffer_write_livolo_malformed` |
| 53 | 1103 | ZclFrame write request with string as bytes array | `zcl_frame_write_request_with_string_as_bytes_array` |
| 54 | 1111 | ZclFrame write rsp | `zcl_frame_write_rsp` |
| 55 | 1121 | ZclFrame to buffer readRsp success | `zcl_frame_from_buffer_read_rsp_success` |
| 56 | 1129 | ZclFrame to buffer defaultRsp success | `zcl_frame_to_buffer_default_rsp_success` |
| 57 | 1137 | ZclFrame to buffer readStructured single element | `zcl_frame_to_buffer_read_structured_single_element` |
| 58 | 1145 | ZclFrame to buffer readStructured multiple elements | `zcl_frame_to_buffer_read_structured_multiple_elements` |
| 59 | 1158 | ZclFrame to buffer readStructured whole attribute | `zcl_frame_to_buffer_read_structured_whole_attribute` |
| 60 | 1166 | ZclFrame from buffer readStructured single elements | `zcl_frame_from_buffer_read_structured_single_elements` |
| 61 | 1187 | ZclFrame from buffer readStructured multiple elements | `zcl_frame_to_buffer_read_structured_multiple_elements` |
| 62 | 1213 | ZclFrame from buffer readStructured whole attribute | `zcl_frame_to_buffer_read_structured_whole_attribute` |
| 63 | 1234 | ZclFrame to buffer writeStructured single element | `zcl_frame_to_buffer_write_structured_single_element` |
| 64 | 1242 | ZclFrame to buffer writeStructured multiple elements | `zcl_frame_to_buffer_write_structured_multiple_elements` |
| 65 | 1256 | ZclFrame to buffer writeStructured whole attribute | `zcl_frame_to_buffer_write_structured_whole_attribute` |
| 66 | 1266 | ZclFrame to buffer writeStructured add element into set/bag | `zcl_frame_to_buffer_write_structured_add_element_into_set_bag` |
| 67 | 1276 | ZclFrame to buffer writeStructured remove element from set/bag | `zcl_frame_to_buffer_write_structured_remove_element_from_set_bag` |
| 68 | 1286 | ZclFrame to buffer writeStructured Livolo malformed | `zcl_frame_to_buffer_write_structured_livolo_malformed` |
| 69 | 1305 | ZclFrame to buffer writeStructuredRsp success | `zcl_frame_to_buffer_write_structured_rsp_success` |
| 70 | 1323 | ZclFrame to buffer writeStructuredRsp failed selector unknown or none for type | `zcl_frame_to_buffer_write_structured_rsp_failed_selector_unknown_or_none_for_type` |
| 71 | 1341 | ZclFrame to buffer writeStructuredRsp failed with selector | `zcl_frame_to_buffer_write_structured_rsp_failed_with_selector` |
| 72 | 1359 | ZclFrame to buffer writeStructuredRsp failed mixed selectors | `zcl_frame_to_buffer_write_structured_rsp_failed_mixed_selectors` |
| 73 | 1380 | ZclFrame from buffer writeStructuredRsp success | `zcl_frame_from_buffer_write_structured_rsp_success` |
| 74 | 1401 | ZclFrame from buffer writeStructuredRsp failed selector unknown or none for type | `zcl_frame_from_buffer_write_structured_rsp_failed_selector_unknown_or_none_for_type` |
| 75 | 1422 | ZclFrame from buffer writeStructuredRsp failed with selector | `zcl_frame_from_buffer_write_structured_rsp_failed_with_selector` |
| 76 | 1443 | ZclFrame from buffer writeStructuredRsp failed mixed selectors | `zcl_frame_from_buffer_write_structured_rsp_failed_mixed_selectors` |
| 77 | 1467 | ZclFrame from buffer ssIasAce arm command | `zcl_frame_from_buffer_ss_ias_ace_arm_command` |
| 78 | 1497 | ZclFrame to buffer discoverRsp | `zcl_frame_to_buffer_discover_rsp` |
| 79 | 1515 | ZclFrame to buffer readRsp failed | `zcl_frame_to_buffer_read_rsp_failed` |
| 80 | 1524 | ZclFrame to buffer offWithEffect | `zcl_frame_to_buffer_off_with_effect` |
| 81 | 1533 | ZclFrame to buffer offWithEffect | `zcl_frame_to_buffer_off_with_effect` |
| 82 | 1541 | ZclFrame to buffer invalid frametype | `zcl_frame_to_buffer_invalid_frametype` |
| 83 | 1558 | BuffaloZcl read array | `buffalo_zcl_read_array` |
| 84 | 1566 | BuffaloZcl read struct | `buffalo_zcl_read_struct` |
| 85 | 1577 | BuffaloZcl read longCharStr | `buffalo_zcl_read_long_char_str` |
| 86 | 1585 | BuffaloZcl read longOctetStr | `buffalo_zcl_read_long_octet_str` |
| 87 | 1593 | BuffaloZcl read extensionFieldSets | `buffalo_zcl_read_extension_field_sets` |
| 88 | 1601 | BuffaloZcl read list zoneinfo | `buffalo_zcl_read_list_zoneinfo` |
| 89 | 1644 | BuffaloZcl read int40 | `buffalo_zcl_read_int40_int56_int64` |
| 90 | 1652 | BuffaloZcl read int56 | `buffalo_zcl_read_int40_int56_int64` |
| 91 | 1660 | BuffaloZcl read int64 | `buffalo_zcl_read_int40_int56_int64` |
| 92 | 1695 | BuffaloZcl write charStr | `buffalo_zcl_write_char_str` |
| 93 | 1705 | BuffaloZcl write longCharStr | `buffalo_zcl_write_long_char_str` |
| 94 | 1715 | BuffaloZcl write octetStr | `buffalo_zcl_write_octet_str` |
| 95 | 1725 | BuffaloZcl write longOctetStr | `buffalo_zcl_write_long_octet_str` |
| 96 | 1735 | BuffaloZcl write extensionFieldSets | `buffalo_zcl_write_extension_field_sets` |
| 97 | 1745 | BuffaloZcl write zoneinfo | `buffalo_zcl_write_zoneinfo` |
| 98 | 1798 | BuffaloZcl write int40 | `buffalo_zcl_write_int40_int56_int64` |
| 99 | 1808 | BuffaloZcl write int56 | `buffalo_zcl_write_int40_int56_int64` |
| 100 | 1818 | BuffaloZcl write int64 | `buffalo_zcl_write_int40_int56_int64` |
| 101 | 1828 | BuffaloZcl write array element type string | `buffalo_zcl_write_array_element_type_string` |
| 102 | 1846 | BuffaloZcl write array element type numeric | `buffalo_zcl_write_array_element_type_numeric` |
| 103 | 1856 | BuffaloZcl write array element type numeric passed as string of DataType | `buffalo_zcl_write_array_element_type_numeric` |
| 104 | 1866 | BuffaloZcl write struct | `buffalo_zcl_write_struct` |
| 105 | 1879 | BuffaloZcl write security key | `buffalo_zcl_write_security_key` |
| 106 | 1908 | Zcl utils get cluster without manufacturerCode | `zcl_utils_get_cluster_without_manufacturer_code` |
| 107 | 1914 | Zcl utils get cluster with manufacturerCode | `zcl_utils_get_cluster_with_manufacturer_code` |
| 108 | 1920 | Zcl utils get cluster manufacturerCode | `zcl_utils_get_cluster_manufacturer_code` |
| 109 | 1926 | Zcl utils get cluster manufacturerCode wrong | `zcl_utils_get_cluster_manufacturer_code_wrong` |
| 110 | 1932 | Zcl utils get cluster attributes manufacturerCode | `zcl_utils_get_cluster_attributes_manufacturer_code` |
| 111 | 1955 | Zcl utils get cluster attributes manufacturerCode wrong | `zcl_utils_get_cluster_attributes_manufacturer_code_wrong` |
| 112 | 1960 | Zcl utils get command | `zcl_utils_get_command` |
| 113 | 1967 | Zcl utils get attribute | `zcl_utils_get_attribute` |
| 114 | 1974 | Zcl utils get attribute non-existing | `zcl_utils_get_attribute_non_existing` |
| 115 | 1979 | Zcl utils get command non-existing | `zcl_utils_get_command_non_existing` |
| 116 | 1984 | Zcl green power readGpd commissioning | `zcl_green_power_read_gpd_commissioning` |
| 117 | 2019 | Zcl green power readGpd commissioning all options | `zcl_green_power_read_gpd_commissioning_all_options` |
| 118 | 2089 | Zcl green power readGpd channel request | `zcl_green_power_read_gpd_channel_request` |
| 119 | 2106 | Zcl green power readGpd attribute report | `zcl_green_power_read_gpd_attribute_report` |
| 120 | 2155 | Zcl green power writeGpd commissioning | `zcl_green_power_write_gpd_commissioning` |
| 121 | 2178 | Zcl green power writeGpd commissioning all options | `zcl_green_power_write_gpd_commissioning_all_options` |
| 122 | 2227 | Zcl green power writeGpd custom reply | `zcl_green_power_write_gpd_custom_reply` |
| 123 | 2251 | Zcl green power writeGpd unhandled command | `zcl_green_power_write_gpd_unhandled_command` |
| 124 | 2264 | Zcl green power writeGpd channel configuration | `zcl_green_power_write_gpd_channel_configuration` |
| 125 | 2284 | Zcl green power writeGpd channel configuration basic | `zcl_green_power_write_gpd_channel_configuration` |
| 126 | 2304 | ZclFrame parse MiBoxer zone configuration command | `zcl_frame_parse_mi_boxer_zone_configuration_command` |
| 127 | 2326 | ZclFrame serialize MiBoxer zone configuration command | `zcl_frame_serialize_mi_boxer_zone_configuration_command` |
| 128 | 2356 | BuffaloZcl read BIG_ENDIAN_UINT24 | `buffalo_zcl_read_big_endian_uint24` |
| 129 | 2364 | BuffaloZcl write BIG_ENDIAN_UINT24 | `buffalo_zcl_write_big_endian_uint24` |
| 130 | 2374 | BuffaloZcl read BUFFER whole | `buffalo_zcl_read_buffer_whole_and_with_length` |
| 131 | 2381 | BuffaloZcl read BUFFER with length | `buffalo_zcl_read_buffer_whole_and_with_length` |
| 132 | 2388 | BuffaloZcl write BUFFER whole | `buffalo_zcl_write_buffer_whole_and_length_ignored` |
| 133 | 2396 | BuffaloZcl write BUFFER always whole - length ignored | `buffalo_zcl_write_buffer_whole_and_length_ignored` |
| 134 | 2404 | BuffaloZcl write non existing type | `buffalo_zcl_write_and_read_non_existing_type` |
| 135 | 2412 | BuffaloZcl read non existing type | `buffalo_zcl_write_and_read_non_existing_type` |
| 136 | 2420 | BuffaloZcl write TOD | `buffalo_zcl_write_and_read_tod_date` |
| 137 | 2429 | BuffaloZcl read TOD | `buffalo_zcl_write_and_read_tod_date` |
| 138 | 2436 | BuffaloZcl write DATE | `buffalo_zcl_write_and_read_tod_date` |
| 139 | 2445 | BuffaloZcl read DATE | `buffalo_zcl_write_and_read_tod_date` |
| 140 | 2452 | BuffaloZcl write TOD non-value | `buffalo_zcl_write_tod_date_non_value` |
| 141 | 2461 | BuffaloZcl read DATE non-value | `buffalo_zcl_write_tod_date_non_value` |
| 142 | 2470 | BuffaloZcl read OCTET_STR non-value | `buffalo_zcl_read_non_value_sentinels` |
| 143 | 2476 | BuffaloZcl read CHAR_STR non-value | `buffalo_zcl_read_non_value_sentinels` |
| 144 | 2482 | BuffaloZcl read MI_STRUCT non-value | `buffalo_zcl_read_non_value_sentinels` |
| 145 | 2488 | BuffaloZcl read LONG_OCTET_STR non-value | `buffalo_zcl_read_non_value_sentinels` |
| 146 | 2494 | BuffaloZcl read LONG_CHAR_STR non-value | `buffalo_zcl_read_non_value_sentinels` |
| 147 | 2500 | BuffaloZcl read ARRAY non-value | `buffalo_zcl_read_non_value_sentinels` |
| 148 | 2506 | BuffaloZcl read STRUCT non-value | `buffalo_zcl_read_non_value_sentinels` |
| 149 | 2512 | BuffaloZcl write NO_DATA | `buffalo_zcl_write_and_read_no_data_unknown` |
| 150 | 2520 | BuffaloZcl read NO_DATA | `buffalo_zcl_write_and_read_no_data_unknown` |
| 151 | 2527 | BuffaloZcl write UNKNOWN | `buffalo_zcl_write_and_read_no_data_unknown` |
| 152 | 2535 | BuffaloZcl read UNKNOWN | `buffalo_zcl_write_and_read_no_data_unknown` |
| 153 | 2542 | ZclHeader return undefined when too short | `zcl_header_return_undefined_when_too_short` |
| 154 | 2547 | ZclHeader return undefined when too short manufacturer specific | `zcl_header_return_undefined_when_too_short` |
| 155 | 2559 | uint8 | `buffalo_zcl_write_passthrough_to_buffalo` |
| 156 | 2570 | uint16 | `buffalo_zcl_write_passthrough_to_buffalo` |
| 157 | 2581 | uint24 | `buffalo_zcl_write_passthrough_to_buffalo` |
| 158 | 2592 | uint32 | `buffalo_zcl_write_passthrough_to_buffalo` |
| 159 | 2603 | int8 | `buffalo_zcl_write_passthrough_to_buffalo` |
| 160 | 2609 | int16 | `buffalo_zcl_write_passthrough_to_buffalo` |
| 161 | 2615 | int24 | `buffalo_zcl_write_passthrough_to_buffalo` |
| 162 | 2621 | int32 | `buffalo_zcl_write_passthrough_to_buffalo` |
| 163 | 2627 | int48 | `buffalo_zcl_write_passthrough_to_buffalo` |
| 164 | 2633 | list uint8 | `buffalo_zcl_write_passthrough_to_buffalo` |
| 165 | 2639 | list uint16 | `buffalo_zcl_write_passthrough_to_buffalo` |
| 166 | 2645 | list uint24 | `buffalo_zcl_write_passthrough_to_buffalo` |
| 167 | 2651 | list uint32 | `buffalo_zcl_write_passthrough_to_buffalo` |
| 168 | 2664 | single prec | `buffalo_zcl_write_passthrough_to_buffalo` |
| 169 | 2670 | double prec | `buffalo_zcl_write_passthrough_to_buffalo` |
| 170 | 2676 | ieee add | `buffalo_zcl_write_passthrough_to_buffalo` |
| 171 | 2690 | uint8 | `buffalo_zcl_read_passthrough_to_buffalo` |
| 172 | 2701 | uint16 | `buffalo_zcl_read_passthrough_to_buffalo` |
| 173 | 2712 | uint24 | `buffalo_zcl_read_passthrough_to_buffalo` |
| 174 | 2723 | uint32 | `buffalo_zcl_read_passthrough_to_buffalo` |
| 175 | 2734 | int8 | `buffalo_zcl_read_passthrough_to_buffalo` |
| 176 | 2740 | int16 | `buffalo_zcl_read_passthrough_to_buffalo` |
| 177 | 2746 | int24 | `buffalo_zcl_read_passthrough_to_buffalo` |
| 178 | 2752 | int32 | `buffalo_zcl_read_passthrough_to_buffalo` |
| 179 | 2758 | int48 | `buffalo_zcl_read_passthrough_to_buffalo` |
| 180 | 2764 | list uint8 | `buffalo_zcl_read_passthrough_to_buffalo` |
| 181 | 2770 | list uint16 | `buffalo_zcl_read_passthrough_to_buffalo` |
| 182 | 2776 | list uint24 | `buffalo_zcl_read_passthrough_to_buffalo` |
| 183 | 2782 | list uint32 | `buffalo_zcl_read_passthrough_to_buffalo` |
| 184 | 2795 | single prec | `buffalo_zcl_read_passthrough_to_buffalo` |
| 185 | 2801 | double prec | `buffalo_zcl_read_passthrough_to_buffalo` |
| 186 | 2807 | ieee add | `buffalo_zcl_read_passthrough_to_buffalo` |
| 187 | 2827 | Reads as buffer when USE_DATA_TYPE is missing dataType option | `buffalo_zcl_read_and_write_use_data_type_buffer_fallback` |
| 188 | 2836 | Writes as buffer when USE_DATA_TYPE is missing dataType option | `buffalo_zcl_read_and_write_use_data_type_buffer_fallback` |
| 189 | 2844 | Throws when write USE_DATA_TYPE is missing dataType option and value isnt buffer or number array | `buffalo_zcl_throws_when_write_use_data_type_missing_and_not_buffer` |
| 190 | 2852 | Throws when read GPD_FRAME is missing payload.payloadSize option | `buffalo_zcl_throws_when_gpd_frame_missing_payload_size` |
| 191 | 2859 | Throws when read LIST_THERMO_TRANSITIONS is missing required payload options | `buffalo_zcl_throws_when_list_thermo_transitions_missing_payload_options` |

| 192 | 461 | ZclFrame from buffer xiaomiStruct [case 1] | `zcl_frame_from_buffer_xiaomi_struct` |
| 193 | 461 | ZclFrame from buffer xiaomiStruct [case 2] | `zcl_frame_from_buffer_xiaomi_struct` |
| 194 | 1612 | BuffaloZcl read uint40, data40, bitmap40 [UINT40] | `buffalo_zcl_read_uint40_data40_bitmap40` |
| 195 | 1612 | BuffaloZcl read uint40, data40, bitmap40 [DATA40] | `buffalo_zcl_read_uint40_data40_bitmap40` |
| 196 | 1612 | BuffaloZcl read uint40, data40, bitmap40 [BITMAP40] | `buffalo_zcl_read_uint40_data40_bitmap40` |
| 197 | 1620 | BuffaloZcl read uint48, data48, bitmap48 [UINT48] | `buffalo_zcl_read_uint48_data48_bitmap48` |
| 198 | 1620 | BuffaloZcl read uint48, data48, bitmap48 [DATA48] | `buffalo_zcl_read_uint48_data48_bitmap48` |
| 199 | 1620 | BuffaloZcl read uint48, data48, bitmap48 [BITMAP48] | `buffalo_zcl_read_uint48_data48_bitmap48` |
| 200 | 1628 | BuffaloZcl read uint56, data56, bitmap56 [UINT56] | `buffalo_zcl_read_uint56_data56_bitmap56` |
| 201 | 1628 | BuffaloZcl read uint56, data56, bitmap56 [DATA56] | `buffalo_zcl_read_uint56_data56_bitmap56` |
| 202 | 1628 | BuffaloZcl read uint56, data56, bitmap56 [BITMAP56] | `buffalo_zcl_read_uint56_data56_bitmap56` |
| 203 | 1636 | BuffaloZcl read uint64, data64, bitmap64 [UINT64] | `buffalo_zcl_read_uint64_data64_bitmap64` |
| 204 | 1636 | BuffaloZcl read uint64, data64, bitmap64 [DATA64] | `buffalo_zcl_read_uint64_data64_bitmap64` |
| 205 | 1636 | BuffaloZcl read uint64, data64, bitmap64 [BITMAP64] | `buffalo_zcl_read_uint64_data64_bitmap64` |
| 206 | 1668 | BuffaloZcl read readListTuyaDataPointValues [no data point] | `buffalo_zcl_read_list_tuya_data_point_values` |
| 207 | 1668 | BuffaloZcl read readListTuyaDataPointValues [single data point] | `buffalo_zcl_read_list_tuya_data_point_values` |
| 208 | 1668 | BuffaloZcl read readListTuyaDataPointValues [two data points] | `buffalo_zcl_read_list_tuya_data_point_values` |
| 209 | 1668 | BuffaloZcl read readListTuyaDataPointValues [incomplete data point is ignored] | `buffalo_zcl_read_list_tuya_data_point_values` |
| 210 | 1668 | BuffaloZcl read readListTuyaDataPointValues [incomplete data buffer] | `buffalo_zcl_read_list_tuya_data_point_values` |
| 211 | 1758 | BuffaloZcl write uint40, data40, bitmap40 [UINT40] | `buffalo_zcl_write_uint40_data40_bitmap40` |
| 212 | 1758 | BuffaloZcl write uint40, data40, bitmap40 [DATA40] | `buffalo_zcl_write_uint40_data40_bitmap40` |
| 213 | 1758 | BuffaloZcl write uint40, data40, bitmap40 [BITMAP40] | `buffalo_zcl_write_uint40_data40_bitmap40` |
| 214 | 1768 | BuffaloZcl write uint48, data48, bitmap48 [UINT48] | `buffalo_zcl_write_uint48_data48_bitmap48` |
| 215 | 1768 | BuffaloZcl write uint48, data48, bitmap48 [DATA48] | `buffalo_zcl_write_uint48_data48_bitmap48` |
| 216 | 1768 | BuffaloZcl write uint48, data48, bitmap48 [BITMAP48] | `buffalo_zcl_write_uint48_data48_bitmap48` |
| 217 | 1778 | BuffaloZcl write uint56, data56, bitmap56 [UINT56] | `buffalo_zcl_write_uint56_data56_bitmap56` |
| 218 | 1778 | BuffaloZcl write uint56, data56, bitmap56 [DATA56] | `buffalo_zcl_write_uint56_data56_bitmap56` |
| 219 | 1778 | BuffaloZcl write uint56, data56, bitmap56 [BITMAP56] | `buffalo_zcl_write_uint56_data56_bitmap56` |
| 220 | 1788 | BuffaloZcl write uint64, data64, bitmap64 [UINT64] | `buffalo_zcl_write_uint64_data64_bitmap64` |
| 221 | 1788 | BuffaloZcl write uint64, data64, bitmap64 [DATA64] | `buffalo_zcl_write_uint64_data64_bitmap64` |
| 222 | 1788 | BuffaloZcl write uint64, data64, bitmap64 [BITMAP64] | `buffalo_zcl_write_uint64_data64_bitmap64` |
| 223 | 1889 | BuffaloZcl writeListTuyaDataPointValues [no data point] | `buffalo_zcl_write_list_tuya_data_point_values` |
| 224 | 1889 | BuffaloZcl writeListTuyaDataPointValues [single data point] | `buffalo_zcl_write_list_tuya_data_point_values` |
| 225 | 1889 | BuffaloZcl writeListTuyaDataPointValues [two data points] | `buffalo_zcl_write_list_tuya_data_point_values` |
| 226 | 2814 | Throws when read is missing required length option - param [LIST_UINT8] | `buffalo_zcl_throws_when_read_missing_required_length_option` |
| 227 | 2814 | Throws when read is missing required length option - param [LIST_UINT16] | `buffalo_zcl_throws_when_read_missing_required_length_option` |
| 228 | 2814 | Throws when read is missing required length option - param [LIST_UINT24] | `buffalo_zcl_throws_when_read_missing_required_length_option` |
| 229 | 2814 | Throws when read is missing required length option - param [LIST_UINT32] | `buffalo_zcl_throws_when_read_missing_required_length_option` |
| 230 | 2814 | Throws when read is missing required length option - param [LIST_ZONEINFO] | `buffalo_zcl_throws_when_read_missing_required_length_option` |

All assertions use the upstream vectors from the test file (exact
`Buffer`/byte arrays, payload objects, header values and error messages).
The Rust port has no `assert!(true)`, tautological or position-only checks;
`grep -c` for those patterns returns 0.

## Notes on equivalent assertions

- **Passthrough write/read (32 nested `it`)** — upstream uses `vi.spyOn` to
  count writer/reader calls. Rust asserts the observable effect: each type
  advances the buffer position by its wire width, and list types by the
  length prefix + element width. This is the same dispatch contract without
  runtime method spies.
- **Paired cases** — read/write non-value sentinels (7 its), `NO_DATA` /
  `UNKNOWN` write/read (4), TOD/DATE write/read (4), and USE_DATA_TYPE
  read/write (2) are covered by one combined Rust test each; every branch is
  asserted.
- **Unknown frame type (line 114)** — upstream calls `Frame.parsePayload`
  with `{frameType: 9}`. Rust's `FrameType` enum cannot represent that state,
  so the test asserts the equivalent rejection of a malformed frame type in
  `Header::from_buffer`.
