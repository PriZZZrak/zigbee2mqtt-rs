# `adapter.test.ts` parity checklist

Source: `.upstream/zigbee-herdsman/test/adapter/z-stack/adapter.test.ts` (v11.2.0).
The upstream file has 124 plain `it(...)` cases and three `it.each(...)` blocks
with two rows each = **130 test cases**. `crates/zigbee-apiarist/tests/adapter_zstack_suite.rs`
contains one named test per case.

| # | Upstream case | Rust test | Assertion |
|---:|---|---|---|
| 1 | should commission network with 3.0.x adapter | `should_commission_network_with_3_0_x_adapter` | exact |
| 2 | should commission network with 3.0.x adapter - auto concurrency | `should_commission_network_with_3_0_x_adapter_auto_concurrency` | exact |
| 3 | should commission network with 3.x.0 adapter - auto concurrency | `should_commission_network_with_3_x_0_adapter_auto_concurrency` | exact |
| 4 | should commission network with 3.0.x adapter - unaligned 8-bit | `should_commission_network_with_3_0_x_adapter_unaligned_8_bit` | exact |
| 5 | should commission network with 3.0.x adapter - default extended pan id | `should_commission_network_with_3_0_x_adapter_default_extended_pan_id` | exact |
| 6 | should commission with 3.0.x adapter - empty, mismatched config/backup | `should_commission_with_3_0_x_adapter_empty_mismatched_config_and_backup` | exact |
| 7 | should commission with 3.0.x adapter - commissioned, mismatched adapter-config-backup | `should_commission_with_3_0_x_adapter_commissioned_mismatched_adapter_config_backup` | exact |
| 8 | should fail to commission network with 3.0.x adapter with invalid pan id 65535 | `should_fail_to_commission_network_with_3_0_x_adapter_with_invalid_pan_id_65535` | exact |
| 9 | should fail to commission network with 3.0.x adapter when bdb commissioning times out | `should_fail_to_commission_when_bdb_commissioning_times_out` | exact |
| 10 | should fail to commission network with 3.0.x adapter when nib fails to settle | `should_fail_to_commission_when_nib_fails_to_settle` | exact |
| 11 | should fail to commission network with 3.0.x adapter when nib reports different pan id | `should_fail_to_commission_network_with_3_0_x_adapter_when_nib_reports_different_pan_id` | exact |
| 12 | should start network with 3.0.x adapter | `should_start_network_with_3_0_x_adapter` | exact |
| 13 | should restore unified backup with 3.0.x adapter and create backup - empty | `should_restore_unified_backup_with_3_0_x_adapter_and_create_backup_empty` | exact |
| 14 | should restore unified backup with 3.0.x adapter and create backup - no tclk seed | `should_restore_unified_backup_with_3_0_x_adapter_and_create_backup_no_tclk_seed` | exact |
| 15 | should restore unified backup with 3.x.0 adapter and create backup - empty | `should_restore_unified_backup_with_3_x_0_adapter_and_create_backup_empty` | exact |
| 16 | should (recommission) restore unified backup with 1.2 adapter and create backup - empty | `should_recommission_restore_unified_backup_with_1_2_adapter_and_create_backup_empty` | exact |
| 17 | should create backup with 3.0.x adapter - default security material table entry | `should_create_backup_with_3_0_x_adapter_default_security_material_table_entry` | exact |
| 18 | should create backup with 3.0.x adapter - emnpty security material table | `should_create_backup_with_3_0_x_adapter_empty_security_material_table` | exact |
| 19 | should create backup with 3.0.x adapter - security material table with generic record | `should_create_backup_with_3_0_x_adapter_security_material_table_with_generic_record` | exact |
| 20 | should create backup with 1.2 adapter | `should_create_backup_with_1_2_adapter` | exact |
| 21 | should keep missing devices in backup | `should_keep_missing_devices_in_backup` | exact |
| 22 | should fail when backup file is corrupted - Coordinator backup is corrupted | `should_fail_when_backup_file_is_corrupted` | exact |
| 23 | should fail to restore unified backup with 3.0.x adapter - invalid open coordinator backup version | `should_fail_to_restore_unified_backup_with_invalid_backup_version` | exact |
| 24 | should fail to restore (unified) backup with 3.0.x adapter - unsupported backup format | `should_fail_to_restore_unified_backup_with_unsupported_backup_format` | exact |
| 25 | should fail to restore unified backup with 3.0.x adapter - insufficient tclk table size | `should_fail_to_restore_unified_backup_with_insufficient_tclk_table_size` | exact |
| 26 | should fail to restore unified backup with 3.0.x adapter - insufficient aps link key data table size | `should_fail_to_restore_unified_backup_with_insufficient_aps_link_key_data_table_size` | exact |
| 27 | should fail to restore unified backup with 3.0.x adapter - insufficient security manager table size | `should_fail_to_restore_unified_backup_with_insufficient_security_manager_table_size` | exact |
| 28 | should fail to restore unified backup with 1.2 adapter - backup from newer adapter | `should_fail_to_restore_unified_backup_with_1_2_adapter_backup_from_newer_adapter` | exact |
| 29 | should fail to create backup with 3.0.x adapter - unable to read ieee address | `should_fail_to_create_backup_when_unable_to_read_ieee_address` | exact |
| 30 | should fail to create backup with 3.0.x adapter - adapter not commissioned - missing nib | `should_fail_to_create_backup_when_adapter_not_commissioned_missing_nib` | exact |
| 31 | should fail to create backup with 3.0.x adapter - missing active key info | `should_fail_to_create_backup_when_missing_active_key_info` | exact |
| 32 | should restore legacy backup with 3.0.x adapter - empty | `should_restore_legacy_backup_with_3_0_x_adapter_empty` | exact |
| 33 | should fail to restore legacy backup with 3.0.x adapter - missing NIB | `should_fail_to_restore_legacy_backup_missing_nib` | exact |
| 34 | should fail to restore legacy backup with 3.0.x adapter - missing active key info | `should_fail_to_restore_legacy_backup_missing_active_key_info` | exact |
| 35 | should fail to restore legacy backup with 3.0.x adapter - missing pre-configured key enabled | `should_fail_to_restore_legacy_backup_missing_preconfigured_key_enabled` | exact |
| 36 | should fail to restore legacy backup with 3.0.x adapter - pre-configured key enabled | `should_fail_to_restore_legacy_backup_preconfigured_key_enabled_missing_material_table` | exact |
| 37 | should fail to restore legacy backup with 3.0.x adapter - missing adapter ieee address | `should_fail_to_restore_legacy_backup_missing_adapter_ieee_address` | exact |
| 38 | should fail to start with 3.0.x adapter - commissioned, config-adapter mismatch | `should_fail_to_start_with_commissioned_config_adapter_mismatch` | exact |
| 39 | should start with runInconsistent option with 3.0.x adapter - commissioned, config-adapter mismatch | `should_start_with_run_inconsistent_option_on_config_adapter_mismatch` | exact |
| 40 | should start with 3.0.x adapter - backward-compat - reversed extended pan id | `should_start_with_backward_compat_reversed_extended_pan_id` | exact |
| 41 | should start with 3.x.0 adapter - warn when preconfigured key doesn't match network key | `should_start_with_3_x_0_adapter_warn_when_preconfigured_key_does_not_match` | exact |
| 42 | should restore unified backup with 3.0.x adapter - commissioned, mismatched adapter-config, matching config-backup | `should_restore_unified_backup_with_mismatched_adapter_config_but_matching_config_backup` | exact |
| 43 | should start network with 3.0.x adapter - resume in coordinator mode | `should_start_network_with_3_0_x_adapter_resume_in_coordinator_mode` | exact |
| 44 | should start network with 3.0.x adapter - resume in coordinator mode, extGroupFind failed | `should_start_network_with_3_0_x_adapter_resume_in_coordinator_mode_ext_group_find_failed` | exact |
| 45 | should commission network with 1.2 adapter | `should_commission_network_with_1_2_adapter` | exact |
| 46 | should commission network with 1.2 adapter - default extended pan id | `should_commission_network_with_1_2_adapter_default_extended_pan_id` | exact |
| 47 | should commission network with 1.2 adapter - old adapter without version reporting | `should_commission_network_with_1_2_adapter_old_adapter_without_version_reporting` | exact |
| 48 | should reset network with 1.2 adapter - config mismtach | `should_reset_network_with_1_2_adapter_config_mismatch` | exact |
| 49 | Add install code: Install Code + CRC | `add_install_code_with_crc` | exact |
| 50 | Add install code: Key derived from Install Code | `add_install_code_key_derived_from_install_code` | exact |
| 51 | LED behaviour: disable LED true, firmware not handling leds | `led_behaviour_disable_led_true_firmware_not_handling_leds` | exact |
| 52 | LED behaviour: disable LED false, firmware not handling leds | `led_behaviour_disable_led_false_firmware_not_handling_leds` | exact |
| 53 | LED behaviour: disable LED true, firmware handling leds | `led_behaviour_disable_led_true_firmware_handling_leds` | exact |
| 54 | LED behaviour: disable LED false, firmware handling leds | `led_behaviour_disable_led_false_firmware_handling_leds` | exact |
| 55 | Call znp constructor | `call_znp_constructor` | exact |
| 56 | Close adapter | `close_adapter` | exact |
| 57 | Get coordinator IEEE | `get_coordinator_ieee` | exact |
| 58 | Permit join all | `permit_join_all` | exact |
| 59 | Permit join coordinator only | `permit_join_coordinator_only` | exact |
| 60 | Permit join specific networkAddress | `permit_join_specific_network_address` | exact |
| 61 | Get coordinator version | `get_coordinator_version` | exact |
| 62 | Soft reset | `soft_reset_uses_reset_type_one` | exact |
| 63 | Hard reset | `hard_reset_uses_reset_type_zero` | exact |
| 64 | Start with transmit power set | `start_with_transmit_power_set` | exact |
| 65 | Support LED should go to false when LED request fails | `support_led_should_go_to_false_when_led_request_fails` | exact |
| 66 | Send zcl frame network address | `send_zcl_frame_network_address` | exact |
| 67 | Send zcl frame with APS encryption | `send_zcl_frame_with_aps_encryption` | exact |
| 68 | Send zcl frame network address retry on MAC channel access failure | `send_zcl_frame_network_address_retry_on_mac_channel_access_failure` | exact |
| 69 | Send zcl frame network address dataConfirm fails | `send_zcl_frame_network_address_data_confirm_fails` | exact |
| 70 | Send zcl frame network address with default response | `send_zcl_frame_network_address_with_default_response` | exact |
| 71 | Send zcl frame network address fails because mac transaction expire, should retry | `send_zcl_frame_network_address_fails_because_mac_transaction_expire_should_retry` | exact |
| 72 | Send zcl frame network address fails because mac transaction expire when not being a parent, should retry | `send_zcl_frame_network_address_fails_because_mac_transaction_expire_when_not_parent_should_retry` | exact |
| 73 | Send zcl frame network address fails because mac no ack, should retry | `send_zcl_frame_network_address_fails_because_mac_no_ack_should_retry` | exact |
| 74 | Send zcl frame network address fails because mac no ack with network address change, should retry | `send_zcl_frame_network_address_with_network_address_change_should_retry` | exact |
| 75 | Send zcl frame network address fails because mac no ack with network address change, without recovery | `send_zcl_frame_network_address_with_network_address_change_without_recovery` | exact |
| 76 | Send zcl frame network address fails because requested profileId is not configured | `send_zcl_frame_network_address_fails_because_requested_profile_is_not_configured` | exact |
| 77 | Source Endpoint is selected based on profileId | `source_endpoint_is_selected_based_on_profile_id` | exact |
| 78 | Send zcl frame network address should retry on dataconfirm timeout | `send_zcl_frame_network_address_should_retry_on_dataconfirm_timeout` | exact |
| 79 | Send zcl frame group | `send_zcl_frame_group` | exact |
| 80 | Send zcl frame group retry on MAC channel access failure | `send_zcl_frame_group_retry_on_mac_channel_access_failure` | exact |
| 81 | Send zcl frame to all - DEFAULT | `send_zcl_frame_to_all_default` | exact |
| 82 | Send zcl frame to all - RX_ON_WHEN_IDLE | `send_zcl_frame_to_all_rx_on_when_idle` | exact |
| 83 | Send zcl frame to all - SLEEPY | `send_zcl_frame_to_all_sleepy` | exact |
| 84 | Send zcl frame network address transaction number shouldnt go higher than 255 | `send_zcl_frame_network_address_transaction_number_should_not_go_higher_than_255` | exact |
| 85 | Send zcl frame group dataConfirm fails | `send_zcl_frame_group_data_confirm_fails` | exact |
| 86 | Send zcl frame network address matches by tsn | `send_zcl_frame_network_address_matches_by_tsn` | exact |
| 87 | Send zcl frame network address matches on default response | `send_zcl_frame_network_address_matches_on_default_response` | exact |
| 88 | Send zcl frame network address data confirm fails with default response | `send_zcl_frame_network_address_data_confirm_fails_with_default_response` | exact |
| 89 | Send zcl frame network address data confirm fails without default response | `send_zcl_frame_network_address_data_confirm_fails_without_default_response` | exact |
| 90 | Send zcl frame network address timeout should discover route and retry | `send_zcl_frame_network_address_timeout_should_discover_route_and_retry` | exact |
| 91 | Send zcl frame network address timeout should discover route, rewrite child entry and retry for sleepy end device | `send_zcl_frame_network_address_timeout_should_rewrite_child_entry_and_retry` | exact |
| 92 | Send zcl frame network address with default response timeout shouldnt care because command has response | `send_zcl_frame_network_address_with_default_response_timeout_shouldnt_care_because_command_has_response` | exact |
| 93 | Supports backup | `supports_backup` | exact |
| 94 | Incoming message raw (not ZCL) | `incoming_message_raw_not_zcl` | exact |
| 95 | Device joined | `device_joined_event` | exact |
| 96 | Device announce | `device_announce_event` | exact |
| 97 | Device announce should discover route to end devices | `device_announce_should_discover_route_to_end_devices` | exact |
| 98 | Network address response | `network_address_response_event` | exact |
| 99 | Concentrator Callback Indication | `concentrator_callback_indication` | exact |
| 100 | Device leave | `device_leave_event` | exact |
| 101 | Ignore device leave with rejoin | `ignore_device_leave_with_rejoin` | exact |
| 102 | Do nothing wiht non areq event | `do_nothing_with_non_areq_event` | exact |
| 103 | Get network parameters | `get_network_parameters` | exact |
| 104 | Set interpan channel | `set_interpan_channel` | exact |
| 105 | Restore interpan channel | `restore_interpan_channel` | exact |
| 106 | Send zcl frame interpan | `send_zcl_frame_interpan_to_ieee_address` | exact |
| 107 | Send zcl frame interpan with response | `send_zcl_frame_interpan_with_response` | exact |
| 108 | Send zcl frame interpan throw exception when command has no response | `send_zcl_frame_interpan_throws_when_command_has_no_response` | exact |
| 109 | Send zcl frame interpan throw exception data request fails | `send_zcl_frame_interpan_throws_when_data_request_fails` | exact |
| 110 | Refuse to start when ping fails | `refuse_to_start_when_ping_fails` | exact |
| 111 | Wait for resolves on cmd | `wait_for_resolves_on_cmd` | exact |
| 112 | Wait for resolves on specified default response | `wait_for_resolves_on_specified_default_response` | exact |
| 113 | Command should fail when in interpan | `command_should_fail_when_in_interpan` | exact |
| 114 | Sends proper ZDO request payload for PERMIT_JOINING_REQUEST to target | `sends_proper_zdo_request_payload_for_permit_joining_request_to_target` | exact |
| 115 | Sends proper ZDO request payload for PERMIT_JOINING_REQUEST broadcast | `sends_proper_zdo_request_payload_for_permit_joining_request_broadcast` | exact |
| 116 | Sends proper ZDO request payload for NWK_UPDATE_REQUEST UNICAST | `sends_proper_zdo_request_payload_for_nwk_update_request_unicast` | exact |
| 117 | Sends proper ZDO request payload for NWK_UPDATE_REQUEST BROADCAST | `sends_proper_zdo_request_payload_for_nwk_update_request_broadcast` | exact |
| 118 | Sends proper ZDO request payload for BIND_REQUEST/UNBIND_REQUEST UNICAST | `sends_proper_zdo_request_payload_for_bind_request_unbind_request_unicast` | exact |
| 119 | Sends proper ZDO request payload for BIND_REQUEST/UNBIND_REQUEST MULTICAST | `sends_proper_zdo_request_payload_for_bind_request_unbind_request_multicast` | exact |
| 120 | Sends proper ZDO request payload for NETWORK_ADDRESS_REQUEST | `sends_proper_zdo_request_payload_for_network_address_request` | exact |
| 121 | Sends proper ZDO request payload for generic logic request | `sends_proper_zdo_request_payload_for_generic_logic_request` | exact |
| 122 | Node descriptor request should discover route to fix potential fails | `node_descriptor_request_should_discover_route_to_fix_potential_fails` | exact |
| 123 | Should throw error when ZDO call fails | `should_throw_error_when_zdo_call_fails` | exact |
| 124 | Should throw error when registerEndpoints fails | `should_throw_error_when_register_endpoints_fails` | exact |
| 125 | Incoming message extended (inter PAN) - 16/64-bit address [srcAddrMode=2] | `incoming_message_extended_interpan_16_bit_address` | exact |
| 126 | Incoming message extended (inter PAN) - 16/64-bit address [srcAddrMode=3] | `incoming_message_extended_interpan_64_bit_address` | exact |
| 127 | Incoming message extended (huge data byte count) - 16/64-bit address [srcAddrMode=2] | `incoming_message_extended_huge_data_16_bit_address` | exact |
| 128 | Incoming message extended (huge data byte count) - 16/64-bit address [srcAddrMode=3] | `incoming_message_extended_huge_data_64_bit_address` | exact |
| 129 | Incoming message extended (huge data byte count) with dataRetrieve error [mockErrType=1] | `incoming_message_extended_huge_data_retrieve_error_status` | exact |
| 130 | Incoming message extended (huge data byte count) with dataRetrieve error [mockErrType=2] | `incoming_message_extended_huge_data_retrieve_length_mismatch` | exact |

## Notes on the two wire-level literals

- Upstream `Should throw error when ZDO call fails` rejects from its `Znp`
  mock with the literal `Failed`. On the wire the equivalent failure is an
  SRSP status error; the Rust test asserts the complete actual message
  `--> 'SREQ: ZDO - SIMPLE_DESCRIPTOR_REQUEST - d2047b0000' failed with status '(0x83: UNKNOWN)'`.
- Upstream `Refuse to start when ping fails` throws `Couldnt lock port` from
  the mock. The Rust test asserts the complete wire error
  `Failed to connect to the adapter (SRSP - SYS - ping after 6000ms)`.

All other error strings are asserted with `assert_eq!` against the exact
upstream text: DataConfirmError names (e.g.
`Data request failed with error: 'NWK_UNSUPPORTED_ATTRIBUTE' (0xc9)`),
`Data request failed with code '99'`, `Status 'NOT_ACTIVE'`,
`dataRetrieve [timestamp: 286331153, index: 0, chunkSize: 240] error status: 1`
and `dataRetrieve length mismatch [240 requested, 241 returned`.
