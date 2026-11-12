# `controller.test.ts` parity checklist

Source: `.upstream/zigbee-herdsman/test/controller.test.ts` (v11.2.0), 311 `it` cases.
Status is `PORTED` when a Rust test (`tests/controller_suite.rs` or a unit
test under `src/controller/`) asserts the upstream behaviour; remaining rows
are explicitly `PENDING`.

| # | line | Upstream case | Rust test | Status |
|---:|---:|---|---|---|
| 1 | 609 | Call controller constructor options mixed with default options | `call_controller_constructor_options_mixed_with_default_options` | PORTED |
| 2 | 625 | Call controller constructor error on invalid channel | `call_controller_constructor_error_on_invalid_channel` | PORTED |
| 3 | 632 | Call controller constructor error when network key too small | `call_controller_constructor_error_when_network_key_too_small` | PORTED |
| 4 | 640 | Call controller constructor error when extendedPanID is too long | `call_controller_constructor_error_when_extended_pan_id_is_too_long` | PORTED |
| 5 | 648 | Call controller constructor error with invalid panID | `call_controller_constructor_error_with_invalid_pan_id` | PORTED |
| 6 | 661 | Controller stop, should create backup | `controller_stop_should_create_backup` | PORTED |
| 7 | 678 | controller should backup at defined interval | `controller_should_backup_at_defined_interval` | PORTED |
| 8 | 687 | controller should not crash on failed interval backup | `controller_should_not_crash_on_failed_interval_backup` | PORTED |
| 9 | 696 | Syncs runtime lookups | — | PENDING |
| 10 | 756 | Controller start | `controller_start` | PORTED |
| 11 | 820 | aborts start on signal | — | PENDING |
| 12 | 847 | Controller update ieeeAddr if changed | `controller_update_ieee_addr_if_changed` | PORTED |
| 13 | 856 | Touchlink factory reset first | `touchlink_factory_reset_first` | PORTED |
| 14 | 961 | Touchlink scan | `touchlink_scan` | PORTED |
| 15 | 1029 | Touchlink lock | — | PENDING |
| 16 | 1050 | Touchlink factory reset | `touchlink_factory_reset_and_identify` | PORTED |
| 17 | 1123 | Touchlink identify | `touchlink_factory_reset_and_identify` | PORTED |
| 18 | 1180 | Controller should ignore touchlink messages | `controller_should_ignore_touchlink_messages` | PORTED |
| 19 | 1223 | Device should update properties when reported | `device_should_update_properties_when_reported` | PORTED |
| 20 | 1242 | Change channel on start | `change_channel_on_start` | PORTED |
| 21 | 1263 | Change channel on start when nwkUpdateID is 0xff | `change_channel_on_start` | PORTED |
| 22 | 1282 | Does not change channel on start if not changed | `does_not_change_channel_on_start_if_not_changed` | PORTED |
| 23 | 1291 | Get coordinator version | `get_coordinator_version_and_network_parameters` | PORTED |
| 24 | 1296 | Get network parameters | `get_coordinator_version_and_network_parameters` | PORTED |
| 25 | 1304 | Iterates over all devices | `iterates_over_all_devices` | PORTED |
| 26 | 1318 | Iterates over devices with predicate | `iterates_over_devices_with_predicate` | PORTED |
| 27 | 1332 | Iterates over all groups | `iterates_over_all_groups_and_with_predicate` | PORTED |
| 28 | 1348 | Iterates over groups with predicate | `iterates_over_all_groups_and_with_predicate` | PORTED |
| 29 | 1364 | Join a device | `join_a_device` | PORTED |
| 30 | 1448 | Join a device and explictly accept it | `join_a_device_and_explicitly_accept_it` | PORTED |
| 31 | 1530 | Join a device and explictly refuses it | `join_a_device_and_explicitly_refuses_it` | PORTED |
| 32 | 1547 | Join a device and explictly refuses it but LEAVE request fails | `join_a_device_and_explicitly_refuses_it_but_leave_fails` | PORTED |
| 33 | 1566 | Set device powersource by string | `set_device_power_source_by_string` | PORTED |
| 34 | 1574 | Get device should return same instance | `get_device_should_return_same_instance` | PORTED |
| 35 | 1580 | Device announce event | `device_announce_event` | PORTED |
| 36 | 1597 | Skip Device announce event from unknown device | `skip_device_announce_event_from_unknown_device` | PORTED |
| 37 | 1609 | Device announce event should update network address when different | `device_announce_event_should_update_network_address_when_different` | PORTED |
| 38 | 1627 | Network address event should update network address when different | `network_address_event_should_update_network_address_when_different` | PORTED |
| 39 | 1644 | Network address event shouldnt update network address when the same | `network_address_event_should_not_update_network_address_when_same` | PORTED |
| 40 | 1658 | Network address event from unknown device | `zdo_network_address_response_from_unknown_device` | PORTED |
| 41 | 1669 | Network address event should update the last seen value | `network_address_event_should_update_last_seen` | PORTED |
| 42 | 1684 | IEEE address event should update network address when different | `ieee_address_event_should_update_network_address_when_different` | PORTED |
| 43 | 1701 | IEEE address event shouldnt update network address when the same | `ieee_address_event_should_not_update_network_address_when_same` | PORTED |
| 44 | 1715 | IEEE address event from unknown device | `ieee_address_event_from_unknown_device` | PORTED |
| 45 | 1726 | IEEE address event should update the last seen value | `ieee_address_event_should_update_last_seen` | PORTED |
| 46 | 1741 | ZDO response for NETWORK_ADDRESS_RESPONSE should update network address when different | `zdo_network_address_response_updates_network_address_when_different` | PORTED |
| 47 | 1755 | ZDO response for NETWORK_ADDRESS_RESPONSE shouldnt update network address when the same | `zdo_network_address_response_should_not_update_when_same` | PORTED |
| 48 | 1766 | ZDO response for NETWORK_ADDRESS_RESPONSE from unknown device | `zdo_network_address_response_from_unknown_device` | PORTED |
| 49 | 1774 | ZDO response for NETWORK_ADDRESS_RESPONSE should update the last seen value | `zdo_network_address_response_updates_last_seen` | PORTED |
| 50 | 1786 | ZDO response for END_DEVICE_ANNOUNCE should bubble up event | `zdo_end_device_announce_should_bubble_event` | PORTED |
| 51 | 1813 | ZDO response for END_DEVICE_ANNOUNCE should update network address when different | `zdo_end_device_announce_updates_network_address_when_different` | PORTED |
| 52 | 1840 | ZDO response for END_DEVICE_ANNOUNCE from unknown device | `zdo_end_device_announce_from_unknown_device` | PORTED |
| 53 | 1862 | ZDO response for cluster ID with no extra processing | `zdo_cluster_with_no_extra_processing` | PORTED |
| 54 | 1867 | Emit lastSeenChanged event even when no message is emitted from it | `emit_last_seen_changed_even_without_message` | PORTED |
| 55 | 1890 | Device leave event and remove from database | `device_leave_event_and_remove_from_database` | PORTED |
| 56 | 1905 | Device leave event with only nwk addr and remove from database | `device_leave_event_with_only_network_address` | PORTED |
| 57 | 1921 | Start with reset should clear database | `start_with_reset_should_clear_database` | PORTED |
| 58 | 1947 | Existing database.tmp should not be overwritten | — | PENDING |
| 59 | 1966 | Should create backup of databse before clearing when datbaseBackupPath is provided | `should_create_database_backup_before_clearing` | PORTED |
| 60 | 1980 | Add install code 18 byte | `add_install_code_18_byte` | PORTED |
| 61 | 1992 | Add install code 16 byte - missing CRC is appended | `add_install_code_16_byte_missing_crc_is_appended` | PORTED |
| 62 | 2010 | Add install code widely adopted format | `add_install_code_widely_adopted_format` | PORTED |
| 63 | 2063 | Add install code Aqara | `add_install_code_aqara_hue_and_pipe_formats` | PORTED |
| 64 | 2075 | Add install code Hue | `add_install_code_aqara_hue_and_pipe_formats` | PORTED |
| 65 | 2087 | Add install code pipe | `add_install_code_aqara_hue_and_pipe_formats` | PORTED |
| 66 | 2099 | Add install code pipe-enclosed | `add_install_code_aqara_hue_and_pipe_formats` | PORTED |
| 67 | 2111 | Add install code invalid | `add_install_code_invalid_and_unknown_format` | PORTED |
| 68 | 2121 | Add install code unknown format | `add_install_code_invalid_and_unknown_format` | PORTED |
| 69 | 2133 | Controller permit joining all, disabled automatically | `controller_permit_joining_all_disabled_automatically` | PORTED |
| 70 | 2183 | Controller permit joining all, disabled manually | `controller_permit_joining_all_disabled_manually` | PORTED |
| 71 | 2244 | Controller permit joining through specific device | `controller_permit_joining_through_specific_device` | PORTED |
| 72 | 2271 | Controller permit joining for specific time | `controller_permit_joining_for_specific_time` | PORTED |
| 73 | 2296 | Controller permit joining for too long time throws | `controller_permit_joining_for_too_long_time_throws` | PORTED |
| 74 | 2304 | Shouldnt create backup when adapter doesnt support it | `shouldnt_create_backup_when_adapter_doesnt_support_it` | PORTED |
| 75 | 2312 | Soft reset | `soft_and_hard_reset` | PORTED |
| 76 | 2319 | Hard reset | `soft_and_hard_reset` | PORTED |
| 77 | 2326 | Adapter disconnected event | `adapter_disconnected_event` | PORTED |
| 78 | 2350 | handles disconnected event early after adapter.start | `adapter_disconnected_event` | PORTED |
| 79 | 2375 | Device joins another time with different network address | `device_joins_another_time_with_different_network_address` | PORTED |
| 80 | 2391 | Device joins and interview succeeds | `device_joins_and_interview_succeeds` | PORTED |
| 81 | 2403 | Device joins and interview fails | `device_joins_and_interview_fails` | PORTED |
| 82 | 2416 | Device joins with endpoints [4,1], should read modelID from 1 | `device_joins_with_endpoints_4_and_1_reads_model_id_from_1` | PORTED |
| 83 | 2429 | Device joins with endpoints [2,1], as 2 is the only endpoint supporting genBasic it should read modelID from that | `device_joins_with_endpoints_2_and_1_reads_model_id_from_2` | PORTED |
| 84 | 2442 | Device joins and interview iAs enrollment succeeds | `device_joins_and_interview_ias_enrollment_succeeds` | PORTED |
| 85 | 2499 | Device joins and interview iAs enrollment fails | `device_joins_and_interview_ias_enrollment_fails` | PORTED |
| 86 | 2513 | Device joins and interview iAs enrollment throws | `device_joins_and_interview_ias_enrollment_throws` | PORTED |
| 87 | 2527 | Device joins, shouldnt enroll when already enrolled | `device_joins_shouldnt_enroll_when_already_enrolled` | PORTED |
| 88 | 2541 | Receive zclData occupancy report | `receive_zcldata_occupancy_report` | PORTED |
| 89 | 2585 | Receive raw data | `receive_raw_data_and_unknown_cluster` | PORTED |
| 90 | 2621 | Receive raw data from unknown cluster | `receive_raw_data_and_unknown_cluster` | PORTED |
| 91 | 2873 | Receive zclData from unkonwn device shouldnt emit anything | `receive_zcldata_from_unknown_device_should_emit_nothing` | PORTED |
| 92 | 2892 | Receive zclData with device deleted during processing (prevent race condition) | — | PENDING |
| 93 | 2937 | Receive readResponse from unknown endpoint | `receive_read_response_from_unknown_endpoint` | PORTED |
| 94 | 2981 | Receive cluster command | `receive_cluster_command` | PORTED |
| 95 | 3035 | Receive cluster command from unknown cluster | `receive_cluster_command_from_unknown_cluster_emits_nothing` | PORTED |
| 96 | 3070 | Receive zclData send default response | `send_default_response_when_requested` | PORTED |
| 97 | 3118 | Receive zclData dont send default resopnse with skipDefaultResponse | `do_not_send_default_response_when_disabled_broadcast_or_skip` | PORTED |
| 98 | 3158 | Receive zclData dont send default resopnse when broadcast | `do_not_send_default_response_when_disabled_broadcast_or_skip` | PORTED |
| 99 | 3195 | Receive zclData send default response fails should NOT attempt route discover when adapter does not support it | — | PENDING |
| 100 | 3235 | Respond to genTime read | `respond_to_gen_time_read_and_failure` | PORTED |
| 101 | 3340 | Allow to override read response through `device.customReadResponse | `allow_override_read_response` | PORTED |
| 102 | 3368 | Respond to read of attribute | `respond_to_read_of_attribute` | PORTED |
| 103 | 3404 | Sends read response for unsupported attribute | `sends_read_response_for_unsupported_attribute` | PORTED |
| 104 | 3430 | Respond to genTime read fails | `respond_to_gen_time_read_and_failure` | PORTED |
| 105 | 3450 | Tuya end devices joins, stops responding after 1 requests, should read modelID and manufacturerName immediately on second pair | `tuya_end_device_second_pair_reads_model_and_manufacturer` | PORTED |
| 106 | 3494 | Xiaomi WXCJKG11LM join (get simple descriptor for endpoint 2 fails) | `xiaomi_wxcjkg11lm_join_simple_descriptor_for_endpoint_2_fails` | PORTED |
| 107 | 3508 | Gledopto GL-C-007/GL-C-008 join (all endpoints support genBasic but only 12 responds) | `gledopto_join_only_endpoint_12_responds` | PORTED |
| 108 | 3522 | Control4 device join (genBasic reads fail, interview completes via quirk) | `control4_device_join_completes_via_quirk` | PORTED |
| 109 | 3537 | Xiaomi end device joins (node descriptor fails) | `xiaomi_end_device_joins_node_descriptor_fails` | PORTED |
| 110 | 3598 | Xiaomi end device joins (node descriptor succeeds, but active endpoint response fails) | `xiaomi_end_device_joins_active_endpoints_fails` | PORTED |
| 111 | 3659 | Should use cached node descriptor when device is re-interviewed, but retrieve it when ignoreCache=true | — | PENDING |
| 112 | 3679 | Should remove disappeared endpoints on updateActiveEndpoints | — | PENDING |
| 113 | 3691 | Receive zclData report from unkown attribute | `receive_report_from_unknown_attribute` | PORTED |
| 114 | 3748 | Should allow to specify custom attributes for existing cluster | `custom_attributes_for_existing_cluster` | PORTED |
| 115 | 3786 | Should allow to specify custom cluster | `custom_cluster_and_override` | PORTED |
| 116 | 3814 | Should allow to specify custom cluster as override for Zcl cluster | `custom_cluster_and_override` | PORTED |
| 117 | 3842 | merges added custom clusters definitions | `merges_added_custom_clusters_definitions` | PORTED |
| 118 | 4068 | Send zcl command to all no options | `send_zcl_command_to_all_no_options_and_manufacturer` | PORTED |
| 119 | 4091 | Send zcl command to all with manufacturer option | `send_zcl_command_to_all_no_options_and_manufacturer` | PORTED |
| 120 | 4126 | Should roll-over transaction ID | `should_roll_over_transaction_id` | PORTED |
| 121 | 4148 | Throw error when creating endpoint which already exists | `throw_error_creating_duplicate_entities` | PORTED |
| 122 | 4161 | Throw error when device with IEEE address already exists | `throw_error_when_device_with_ieee_address_already_exists` | PORTED |
| 123 | 4173 | Should allow to set type | `should_allow_to_set_type` | PORTED |
| 124 | 4180 | Return device from databse when not in lookup | — | PENDING |
| 125 | 4187 | Throw error when interviewing and calling interview again | — | PENDING |
| 126 | 4206 | Removes device from network and database but keep in cache | `remove_device_from_network_and_database_variants` | PORTED |
| 127 | 4225 | Removes device from network and clears cache | `remove_device_clear_cache_variants` | PORTED |
| 128 | 4286 | Removes device from database but keep in cache | `remove_device_from_network_and_database_variants` | PORTED |
| 129 | 4302 | Removes device from database and clears cache | `remove_device_clear_cache_variants` | PORTED |
| 130 | 4315 | Removes group from network and database | `remove_group_from_network_and_database` | PORTED |
| 131 | 4339 | Removes every member when removing a group from the network | `remove_group_from_network_and_database` | PORTED |
| 132 | 4363 | Removes group from database | `remove_group_from_network_and_database` | PORTED |
| 133 | 4376 | Device lqi | `device_lqi_routing_and_binding_table` | PORTED |
| 134 | 4387 | Device routing table | `device_lqi_routing_and_binding_table` | PORTED |
| 135 | 4398 | Device binding table | `device_lqi_routing_and_binding_table` | PORTED |
| 136 | 4422 | Device updates endpoint bindings from binding table | `device_updates_endpoint_bindings_from_binding_table` | PORTED |
| 137 | 4485 | Device updates existing endpoint bindings from binding table | `device_updates_existing_endpoint_bindings_from_binding_table` | PORTED |
| 138 | 4552 | Device clears all bindings | `device_clears_all_and_selective_bindings` | PORTED |
| 139 | 4568 | Device selectively clears target bindings | `device_clears_all_and_selective_bindings` | PORTED |
| 140 | 4584 | Device selectively clears multi-target bindings | `device_clears_all_and_selective_bindings` | PORTED |
| 141 | 4600 | Device ping | `device_ping` | PORTED |
| 142 | 4620 | Poll control supported | `poll_control_supported` | PORTED |
| 143 | 4739 | Poll control unsupported | `poll_control_unsupported` | PORTED |
| 144 | 4748 | Endpoint get id | `endpoint_get_id_and_device_type` | PORTED |
| 145 | 4755 | Endpoint get id by endpoint device type | `endpoint_get_id_by_endpoint_device_type` | PORTED |
| 146 | 4763 | Endpoint bind | `endpoint_bind_add_binding_and_unbind` | PORTED |
| 147 | 4783 | Endpoint addBinding | `endpoint_bind_add_binding_and_unbind` | PORTED |
| 148 | 4798 | Endpoint get binds non-existing device | `endpoint_get_binds_non_existing_device` | PORTED |
| 149 | 4813 | Group bind | — | PENDING |
| 150 | 4835 | Group addBinding | — | PENDING |
| 151 | 4845 | Group bind by number (should create group) | — | PENDING |
| 152 | 4868 | Group addBinding by number (should create group) | — | PENDING |
| 153 | 4879 | Endpoint unbind | `endpoint_bind_add_binding_and_unbind` | PORTED |
| 154 | 4898 | Group unbind | — | PENDING |
| 155 | 4923 | Group unbind by number | — | PENDING |
| 156 | 4948 | Group unbind by number, non-existing group and non-existing bind with force=true | — | PENDING |
| 157 | 4970 | Endpoint configure reporting | `endpoint_configure_reporting_and_read_config` | PORTED |
| 158 | 5006 | handles configure reporting with non-Analog data type | `configure_reporting_non_analog_and_disable_response` | PORTED |
| 159 | 5041 | throws when trying to configure reporting on endpoint with bad attribute | `endpoint_configure_reporting_bad_attribute` | PORTED |
| 160 | 5071 | Should replace legacy configured reportings without manufacturerCode | — | PENDING |
| 161 | 5095 | Endpoint configure reporting for manufacturer specific attribute | `configure_reporting_manufacturer_specific_attribute` | PORTED |
| 162 | 5148 | Endpoint configure reporting for manufacturer specific attribute from definition | — | PENDING |
| 163 | 5196 | Save endpoint configure reporting | `endpoint_configure_reporting_and_read_config` | PORTED |
| 164 | 5264 | Endpoint configure reporting fails when status code is not 0 | `endpoint_configure_reporting_fails_on_non_zero_status` | PORTED |
| 165 | 5291 | Endpoint configure reporting fails when status code is not 0 default rsp | — | PENDING |
| 166 | 5319 | Endpoint configure reporting with disable response | `configure_reporting_non_analog_and_disable_response` | PORTED |
| 167 | 5341 | Endpoint read reporting config | `endpoint_configure_reporting_and_read_config` | PORTED |
| 168 | 5500 | Endpoint throws without response to read reporting config | `read_reporting_config_no_response_errors` | PORTED |
| 169 | 5510 | Endpoint ignores unknown attributes for read reporting config | `read_reporting_config_ignores_unknown_attributes` | PORTED |
| 170 | 5567 | Endpoint manuf-spec read reporting config | `read_reporting_config_manufacturer_specific` | PORTED |
| 171 | 5710 | Return group from databse when not in lookup | — | PENDING |
| 172 | 5717 | Throw error when creating group already exists | `throw_error_creating_duplicate_entities` | PORTED |
| 173 | 5729 | Add endpoint to group | `endpoint_add_and_remove_from_group` | PORTED |
| 174 | 5769 | Adds endpoint to group when Add Group response reports duplicate | `endpoint_add_to_group_when_response_reports_duplicate` | PORTED |
| 175 | 5796 | Does not add endpoint to group when Add Group response reports failure | `endpoint_add_to_group_failure_does_not_add` | PORTED |
| 176 | 5821 | Remove endpoint from group | `endpoint_add_and_remove_from_group` | PORTED |
| 177 | 5840 | Does not remove endpoint from group when Remove Group response reports failure | `endpoint_remove_from_group_failure_keeps_member` | PORTED |
| 178 | 5866 | Removes endpoint from group database when Remove Group response reports NOT_FOUND | `endpoint_remove_from_group_database_when_not_found` | PORTED |
| 179 | 5893 | Remove endpoint from group by number | — | PENDING |
| 180 | 5909 | Try to get deleted device from endpoint | — | PENDING |
| 181 | 5925 | Command response | `command_response` | PORTED |
| 182 | 5934 | Group command | `group_command` | PORTED |
| 183 | 5956 | Endpoint command with options | `endpoint_command_with_options` | PORTED |
| 184 | 5983 | Endpoint command with duplicate cluster ID | `endpoint_command_with_duplicate_cluster_id` | PORTED |
| 185 | 6005 | Endpoint command with duplicate identifier | `endpoint_command_with_duplicate_identifier` | PORTED |
| 186 | 6019 | Endpoint commandResponse | `endpoint_command_response` | PORTED |
| 187 | 6046 | Device without meta should set meta to {} | `device_without_meta_sets_empty` | PORTED |
| 188 | 6125 | Read from group | `read_from_group_and_failure` | PORTED |
| 189 | 6139 | Read from group ignores unknown attributes | `read_from_group_and_failure` | PORTED |
| 190 | 6154 | Read from group fails | `read_from_group_and_failure` | PORTED |
| 191 | 6167 | Write to group | `write_to_group_and_failure` | PORTED |
| 192 | 6194 | Write to group with unknown attribute should fail | `write_to_group_and_failure` | PORTED |
| 193 | 6206 | Write to group fails | `write_to_group_and_failure` | PORTED |
| 194 | 6219 | Write to endpoint custom attributes | `write_to_endpoint_custom_attributes` | PORTED |
| 195 | 6250 | Write to endpoint custom attributes without specifying manufacturerCode | `write_to_endpoint_custom_attributes_variants` | PORTED |
| 196 | 6283 | WriteUndiv to endpoint custom attributes without response | `write_to_endpoint_custom_attributes_variants` | PORTED |
| 197 | 6321 | Write to endpoint with unknown string attribute | `write_to_endpoint_with_unknown_string_attribute_fails` | PORTED |
| 198 | 6337 | Write response to endpoint with non ZCL attribute | `write_response_to_endpoint_variants` | PORTED |
| 199 | 6367 | Write response to endpoint with unknown string attribute | `write_response_to_endpoint_variants` | PORTED |
| 200 | 6383 | Write response to endpoint throw when transaction sequence number provided through options | `write_response_to_endpoint_variants` | PORTED |
| 201 | 6399 | Write response to endpoint with no status attribute specified | `write_response_to_endpoint_variants` | PORTED |
| 202 | 6415 | Write response to endpoint with ZCL attribute | `write_response_to_endpoint_variants` | PORTED |
| 203 | 6445 | WriteResponse error | — | PENDING |
| 204 | 6464 | Write response to endpoint with options | `write_response_to_endpoint_variants` | PORTED |
| 205 | 6494 | Read from endpoint with string | `read_from_endpoint_variants` | PORTED |
| 206 | 6512 | Read from endpoint with string ignores unknown | `read_from_endpoint_variants` | PORTED |
| 207 | 6531 | Read from endpoint with custom attribute | `read_from_endpoint_custom_attribute` | PORTED |
| 208 | 6563 | Read from endpoint unknown attribute with options | `read_from_endpoint_variants` | PORTED |
| 209 | 6581 | Read response to endpoint with non ZCL attribute | `read_response_to_endpoint_variants` | PORTED |
| 210 | 6611 | Read response to endpoint with unknown string attribute | `read_response_to_endpoint_variants` | PORTED |
| 211 | 6627 | Read response to endpoint throw when transaction sequence number provided through options | `read_response_to_endpoint_variants` | PORTED |
| 212 | 6643 | Configure reporting endpoint custom attributes | `configure_reporting_custom_attribute_by_id` | PORTED |
| 213 | 6692 | Remove endpoint from all groups | `remove_endpoint_from_all_groups` | PORTED |
| 214 | 6720 | Load database | — | PENDING |
| 215 | 7060 | Load database: interviewState migration and reset IN_PROGRESS | `load_database_interview_state_migration` | PORTED |
| 216 | 7087 | Shouldnt load device from group databaseentry | — | PENDING |
| 217 | 7094 | Should throw datbase basic crud errors | `database_basic_crud_errors` | PORTED |
| 218 | 7113 | Should save received attributes | `should_save_received_attributes` | PORTED |
| 219 | 7149 | Emit read from device | `emit_read_from_device` | PORTED |
| 220 | 7191 | Emit write from device | `emit_write_from_device` | PORTED |
| 221 | 7235 | Endpoint command error | — | PENDING |
| 222 | 7254 | Endpoint commandResponse error | — | PENDING |
| 223 | 7273 | Endpoint commandResponse error when transactionSequenceNumber provided through options | — | PENDING |
| 224 | 7288 | ConfigureReporting error | — | PENDING |
| 225 | 7309 | DefaultResponse error | — | PENDING |
| 226 | 7328 | DefaultResponse error when transactionSequenceNumber provided through options | — | PENDING |
| 227 | 7343 | Skip unbind if not bound | — | PENDING |
| 228 | 7354 | Handle unbind with number not matching any group | — | PENDING |
| 229 | 7367 | Unbind against unbound cluster | — | PENDING |
| 230 | 7395 | Unbind error | — | PENDING |
| 231 | 7422 | Bind error | — | PENDING |
| 232 | 7448 | ReadResponse error | — | PENDING |
| 233 | 7467 | Read error | — | PENDING |
| 234 | 7486 | Read with disable response | — | PENDING |
| 235 | 7496 | Write error | — | PENDING |
| 236 | 7515 | Write with disable response | — | PENDING |
| 237 | 7525 | Group command error | — | PENDING |
| 238 | 7538 | Write structured | — | PENDING |
| 239 | 7548 | Write structured with disable response | — | PENDING |
| 240 | 7558 | Write structured error | — | PENDING |
| 241 | 7577 | Write with custom payload | — | PENDING |
| 242 | 7631 | Green power | — | PENDING |
| 243 | 7875 | Should handle green power responses gracefully | — | PENDING |
| 244 | 7916 | Should handle green power broadcast commissioning | — | PENDING |
| 245 | 7954 | Should handle green power unicast commissioning success | — | PENDING |
| 246 | 8010 | Should handle green power unicast commissioning failure | — | PENDING |
| 247 | 8065 | Should handle green power commissioning frame with IEEE addressing gracefully | — | PENDING |
| 248 | 8091 | Should handle green power commissioning frame with switch info | — | PENDING |
| 249 | 8120 | Should ignore invalid green power frame | — | PENDING |
| 250 | 8146 | Green power channel request | — | PENDING |
| 251 | 8199 | Green power rxOnCap | — | PENDING |
| 252 | 8304 | Green power unicast | — | PENDING |
| 253 | 8624 | Get input/ouptut clusters | — | PENDING |
| 254 | 8642 | Report to endpoint custom attributes | `write_to_endpoint_custom_attributes` | PORTED |
| 255 | 8673 | Report to endpoint with unknown string attribute | `write_to_endpoint_with_unknown_string_attribute_fails` | PORTED |
| 256 | 8689 | Report error | — | PENDING |
| 257 | 8708 | Write to device with pendingRequestTimeout > 0 | — | PENDING |
| 258 | 8754 | Write to device with pendingRequestTimeout > 0, override default sendPolicy | — | PENDING |
| 259 | 8781 | Write to device with pendingRequestTimeout > 0, error | — | PENDING |
| 260 | 8855 | Write to device with pendingRequestTimeout > 0, replace queued messages | — | PENDING |
| 261 | 9035 | Write to device with pendingRequestTimeout > 0, discard messages after expiration | — | PENDING |
| 262 | 9099 | Implicit checkin while send already in progress | — | PENDING |
| 263 | 9139 | Write to device with pendingRequestTimeout > 0, send bulk messages | — | PENDING |
| 264 | 9220 | Send to device with pendingRequestTimeout > 0, retry on timeout error | — | PENDING |
| 265 | 9269 | Send to device with pendingRequestTimeout > 0, fails on non-timeout error | — | PENDING |
| 266 | 9342 | Handle retransmitted Xiaomi messages | — | PENDING |
| 267 | 9379 | Shouldnt throw error on coordinatorCheck when adapter doesnt support backups | — | PENDING |
| 268 | 9388 | Should do a coordinator check | — | PENDING |
| 269 | 9667 | zclCommand | — | PENDING |
| 270 | 9677 | zclCommand with cluster/command objects | — | PENDING |
| 271 | 9689 | zclCommand with error | — | PENDING |
| 272 | 9709 | Interview on coordinator | — | PENDING |
| 273 | 9721 | Device node descriptor fails | — | PENDING |
| 274 | 9731 | Device active endpoints fails | — | PENDING |
| 275 | 9741 | Endpoint simple descriptor fails | — | PENDING |
| 276 | 9752 | Node Descriptor on R21 device | — | PENDING |
| 277 | 9762 | Node Descriptor on R pre-21 device | — | PENDING |
| 278 | 9772 | Device requests network address - unchanged | — | PENDING |
| 279 | 9809 | Device requests network address - changed | — | PENDING |
| 280 | 9849 | Device remove from network fails | — | PENDING |
| 281 | 9862 | Device LQI table fails | — | PENDING |
| 282 | 9875 | Device routing table fails | — | PENDING |
| 283 | 9888 | Device binding table fails | — | PENDING |
| 284 | 9901 | Device clear all bindings fails | — | PENDING |
| 285 | 9914 | Device LQI table with more than 1 request | — | PENDING |
| 286 | 9951 | Device routing table with more than 1 request | — | PENDING |
| 287 | 9988 | Device binding table with more than 1 request | — | PENDING |
| 288 | 10067 | Touchlink stops during stop | — | PENDING |
| 289 | 10075 | Touchlink fails to stop during stop | — | PENDING |
| 290 | 10083 | Adapter permitJoin fails during stop | — | PENDING |
| 291 | 10091 | Adapter stop fails after adapter disconnected | — | PENDING |
| 292 | 10099 | Device network address changed while Z2M was offline, received no notification on start | — | PENDING |
| 293 | 10149 | Device network address changed while Z2M was running, received no notification | — | PENDING |
| 294 | 10224 | Device network address changed while Z2M was offline - fails to retrieve new one | — | PENDING |
| 295 | 10268 | Device network address changed while Z2M was offline, no duplicate triggering of IEEE request | — | PENDING |
| 296 | 10326 | Device network address changed while Z2M was offline, no spamming of IEEE request when device doesnt respond | — | PENDING |
| 297 | 10368 | does not try to identify unknown device when deleted | — | PENDING |
| 298 | 10414 | reads/writes to group with custom cluster when common to all members | — | PENDING |
| 299 | 10540 | does not read/write to group with non-common custom clusters | — | PENDING |
| 300 | 10572 | sends & receives command to group with custom cluster when common to all members | — | PENDING |
| 301 | 10704 | sends one group command with an explicit custom cluster supported by one member | — | PENDING |
| 302 | 10760 | Updates a device genBasic properties | — | PENDING |
| 303 | 10808 | triggers sendZdo on sendRaw | — | PENDING |
| 304 | 10825 | triggers sendZclFrameInterPANToIeeeAddr on sendRaw | — | PENDING |
| 305 | 10857 | triggers sendZclFrameInterPANBroadcast on sendRaw | — | PENDING |
| 306 | 10886 | triggers sendZclFrameToGroup on sendRaw | — | PENDING |
| 307 | 10920 | triggers sendZclFrameToAll on sendRaw | — | PENDING |
| 308 | 10953 | triggers sendZclFrameToEndpoint on sendRaw | — | PENDING |
| 309 | 10997 | triggers sendZclFrameToEndpoint with custom cluster on sendRaw | — | PENDING |
| 310 | 11060 | ignores invalid attributes when processing ZCL attribute | — | PENDING |
| 311 | 11097 | ensures manufacturer code is always unique in a single call | — | PENDING |

Ported: **196 / 311**.
