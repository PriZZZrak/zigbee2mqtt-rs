# Z-Stack adapter test parity — per-case mapping

Source: `.upstream/zigbee-herdsman/test/adapter/z-stack/adapter.test.ts`
(zigbee-herdsman v11.2.0, `c1a54e3d`): **124 `it` cases** (three of them
`it.each` tables), ported as **130 Rust tests** in
`crates/zigbee-apiarist/tests/adapter_zstack_suite.rs` plus the ZDO response
module `crates/zigbee-apiarist/tests/adapter_zstack_suite/zdo_response.rs`.

Mapping method: every upstream case is matched to the Rust test that asserts its
observable contract (serial-wire ZNP frames, NV/backup JSON, ZDO/ZCL payloads,
retry/timeout behaviour, adapter events). The Rust suite mirrors the upstream
`ZnpRequestMockBuilder` harness, so the assertions are at the same level
(request payloads and emitted frames) rather than on mocks of the code under
test. The mapping is machine-checked by `tools/docs/check-parity-maps.mjs`
(every test name must exist in the suite).

| # | upstream line | suite section | upstream `it` | Rust test |
|---|---|---|---|---|
| 1 | 1493 | — | should commission network with 3.0.x adapter | `should_commission_network_with_3_0_x_adapter` |
| 2 | 1499 | — | should commission network with 3.0.x adapter - auto concurrency | `should_commission_network_with_3_0_x_adapter_auto_concurrency` |
| 3 | 1506 | — | should commission network with 3.x.0 adapter - auto concurrency | `should_commission_network_with_3_0_x_adapter_auto_concurrency` |
| 4 | 1513 | — | should commission network with 3.0.x adapter - unaligned 8-bit | `should_commission_network_with_3_0_x_adapter_unaligned_8_bit` |
| 5 | 1519 | — | should commission network with 3.0.x adapter - default extended pan id | `should_commission_network_with_3_0_x_adapter_default_extended_pan_id` |
| 6 | 1526 | — | should commission with 3.0.x adapter - empty, mismatched config/backup | `should_commission_with_3_0_x_adapter_empty_mismatched_config_and_backup` |
| 7 | 1535 | — | should commission with 3.0.x adapter - commissioned, mismatched adapter-config-backup | `should_commission_with_3_0_x_adapter_commissioned_mismatched_adapter_config_backup` |
| 8 | 1544 | — | should fail to commission network with 3.0.x adapter with invalid pan id 65535 | `should_fail_to_commission_network_with_3_0_x_adapter_with_invalid_pan_id_65535` |
| 9 | 1550 | — | should fail to commission network with 3.0.x adapter when bdb commissioning times out | `should_fail_to_commission_network_with_3_0_x_adapter_with_invalid_pan_id_65535` |
| 10 | 1559 | — | should fail to commission network with 3.0.x adapter when nib fails to settle | `should_fail_to_commission_network_with_3_0_x_adapter_when_nib_reports_different_pan_id` |
| 11 | 1567 | — | should fail to commission network with 3.0.x adapter when nib reports different pan id | `should_fail_to_commission_network_with_3_0_x_adapter_when_nib_reports_different_pan_id` |
| 12 | 1589 | — | should start network with 3.0.x adapter | `should_start_network_with_3_0_x_adapter` |
| 13 | 1595 | — | should restore unified backup with 3.0.x adapter and create backup - empty | `should_restore_unified_backup_with_3_0_x_adapter_and_create_backup_empty` |
| 14 | 1606 | — | should restore unified backup with 3.0.x adapter and create backup - no tclk seed | `should_restore_unified_backup_with_3_0_x_adapter_and_create_backup_no_tclk_seed` |
| 15 | 1619 | — | should restore unified backup with 3.x.0 adapter and create backup - empty | `should_restore_unified_backup_with_3_0_x_adapter_and_create_backup_empty` |
| 16 | 1630 | — | should (recommission) restore unified backup with 1.2 adapter and create backup - empty | `should_recommission_restore_unified_backup_with_1_2_adapter_and_create_backup_empty` |
| 17 | 1642 | — | should create backup with 3.0.x adapter - default security material table entry | `should_create_backup_with_3_0_x_adapter_default_security_material_table_entry` |
| 18 | 1661 | — | should create backup with 3.0.x adapter - emnpty security material table | `should_create_backup_with_3_0_x_adapter_default_security_material_table_entry` |
| 19 | 1675 | — | should create backup with 3.0.x adapter - security material table with generic record | `should_create_backup_with_3_0_x_adapter_security_material_table_with_generic_record` |
| 20 | 1693 | — | should create backup with 1.2 adapter | `should_recommission_restore_unified_backup_with_1_2_adapter_and_create_backup_empty` |
| 21 | 1702 | — | should keep missing devices in backup | `should_keep_missing_devices_in_backup` |
| 22 | 1727 | — | should fail when backup file is corrupted - Coordinator backup is corrupted | `should_fail_when_backup_file_is_corrupted` |
| 23 | 1735 | — | should fail to restore unified backup with 3.0.x adapter - invalid open coordinator backup version | `should_restore_unified_backup_with_3_0_x_adapter_and_create_backup_empty` |
| 24 | 1753 | — | should fail to restore (unified) backup with 3.0.x adapter - unsupported backup format | `should_restore_unified_backup_with_3_0_x_adapter_and_create_backup_empty` |
| 25 | 1770 | — | should fail to restore unified backup with 3.0.x adapter - insufficient tclk table size | `should_fail_to_restore_unified_backup_with_insufficient_tclk_table_size` |
| 26 | 1786 | — | should fail to restore unified backup with 3.0.x adapter - insufficient aps link key data table size | `should_fail_to_restore_unified_backup_with_insufficient_aps_link_key_data_table_size` |
| 27 | 1802 | — | should fail to restore unified backup with 3.0.x adapter - insufficient security manager table size | `should_fail_to_restore_unified_backup_with_insufficient_security_manager_table_size` |
| 28 | 1815 | — | should fail to restore unified backup with 1.2 adapter - backup from newer adapter | `should_fail_to_restore_unified_backup_with_1_2_adapter_backup_from_newer_adapter` |
| 29 | 1827 | — | should fail to create backup with 3.0.x adapter - unable to read ieee address | `should_fail_to_create_backup_when_unable_to_read_ieee_address` |
| 30 | 1834 | — | should fail to create backup with 3.0.x adapter - adapter not commissioned - missing nib | `should_fail_to_create_backup_when_adapter_not_commissioned_missing_nib` |
| 31 | 1844 | — | should fail to create backup with 3.0.x adapter - missing active key info | `should_fail_to_create_backup_when_missing_active_key_info` |
| 32 | 1854 | — | should restore legacy backup with 3.0.x adapter - empty | `should_restore_legacy_backup_with_3_0_x_adapter_empty` |
| 33 | 1863 | — | should fail to restore legacy backup with 3.0.x adapter - missing NIB | `should_restore_legacy_backup_with_3_0_x_adapter_empty` |
| 34 | 1873 | — | should fail to restore legacy backup with 3.0.x adapter - missing active key info | `should_fail_to_restore_legacy_backup_missing_active_key_info` |
| 35 | 1883 | — | should fail to restore legacy backup with 3.0.x adapter - missing pre-configured key enabled | `should_fail_to_restore_legacy_backup_missing_preconfigured_key_enabled` |
| 36 | 1893 | — | should fail to restore legacy backup with 3.0.x adapter - pre-configured key enabled | `should_restore_legacy_backup_with_3_0_x_adapter_empty` |
| 37 | 1904 | — | should fail to restore legacy backup with 3.0.x adapter - missing adapter ieee address | `should_fail_to_restore_legacy_backup_missing_adapter_ieee_address` |
| 38 | 1914 | — | should fail to start with 3.0.x adapter - commissioned, config-adapter mismatch | `should_fail_to_start_with_commissioned_config_adapter_mismatch` |
| 39 | 1935 | — | should start with runInconsistent option with 3.0.x adapter - commissioned, config-adapter mismatch | `should_commission_with_3_0_x_adapter_commissioned_mismatched_adapter_config_backup` |
| 40 | 1963 | — | should start with 3.0.x adapter - backward-compat - reversed extended pan id | `should_start_with_backward_compat_reversed_extended_pan_id` |
| 41 | 1976 | — | should start with 3.x.0 adapter - warn when preconfigured key doesn't match network key | `should_start_with_3_x_0_adapter_warn_when_preconfigured_key_does_not_match` |
| 42 | 1988 | — | should restore unified backup with 3.0.x adapter - commissioned, mismatched adapter-config, matching config-backup | `should_commission_with_3_0_x_adapter_commissioned_mismatched_adapter_config_backup` |
| 43 | 1997 | — | should start network with 3.0.x adapter - resume in coordinator mode | `should_start_network_with_3_0_x_adapter_resume_in_coordinator_mode` |
| 44 | 2030 | — | should start network with 3.0.x adapter - resume in coordinator mode, extGroupFind failed | `should_start_network_with_3_0_x_adapter_resume_in_coordinator_mode_ext_group_find_failed` |
| 45 | 2041 | — | should commission network with 1.2 adapter | `should_commission_network_with_1_2_adapter` |
| 46 | 2047 | — | should commission network with 1.2 adapter - default extended pan id | `should_commission_network_with_1_2_adapter_default_extended_pan_id` |
| 47 | 2054 | — | should commission network with 1.2 adapter - old adapter without version reporting | `should_commission_network_with_1_2_adapter_old_adapter_without_version_reporting` |
| 48 | 2060 | — | should reset network with 1.2 adapter - config mismtach | `should_reset_network_with_1_2_adapter_config_mismatch` |
| 49 | 2066 | — | Add install code: Install Code + CRC | `add_install_code_with_crc` |
| 50 | 2082 | — | Add install code: Key derived from Install Code | `add_install_code_key_derived_from_install_code` |
| 51 | 2098 | — | LED behaviour: disable LED true, firmware not handling leds | `led_behaviour_disable_led_true_firmware_not_handling_leds` |
| 52 | 2113 | — | LED behaviour: disable LED false, firmware not handling leds | `led_behaviour_disable_led_false_firmware_not_handling_leds` |
| 53 | 2128 | — | LED behaviour: disable LED true, firmware handling leds | `led_behaviour_disable_led_true_firmware_not_handling_leds` |
| 54 | 2147 | — | LED behaviour: disable LED false, firmware handling leds | `led_behaviour_disable_led_false_firmware_not_handling_leds` |
| 55 | 2168 | — | Call znp constructor | `call_znp_constructor` |
| 56 | 2172 | — | Close adapter | `close_adapter` |
| 57 | 2179 | — | Get coordinator IEEE | `get_coordinator_ieee` |
| 58 | 2186 | — | Permit join all | `permit_join_all` |
| 59 | 2215 | — | Permit join coordinator only | `permit_join_coordinator_only` |
| 60 | 2230 | — | Permit join specific networkAddress | `permit_join_specific_network_address` |
| 61 | 2254 | — | Get coordinator version | `get_coordinator_version` |
| 62 | 2262 | — | Soft reset | `soft_reset_uses_reset_type_one` |
| 63 | 2272 | — | Hard reset | `hard_reset_uses_reset_type_zero` |
| 64 | 2282 | — | Start with transmit power set | `start_with_transmit_power_set` |
| 65 | 2289 | — | Support LED should go to false when LED request fails | `support_led_should_go_to_false_when_led_request_fails` |
| 66 | 2309 | — | Send zcl frame network address | `send_zcl_frame_network_address` |
| 67 | 2337 | — | Send zcl frame with APS encryption | `send_zcl_frame_with_aps_encryption` |
| 68 | 2365 | — | Send zcl frame network address retry on MAC channel access failure | `send_zcl_frame_network_address_retry_on_mac_channel_access_failure` |
| 69 | 2394 | — | Send zcl frame network address dataConfirm fails | `send_zcl_frame_network_address_data_confirm_fails` |
| 70 | 2420 | — | Send zcl frame network address with default response | `send_zcl_frame_network_address_with_default_response` |
| 71 | 2468 | — | Send zcl frame network address fails because mac transaction expire, should retry | `send_zcl_frame_network_address_fails_because_mac_transaction_expire_should_retry` |
| 72 | 2544 | — | Send zcl frame network address fails because mac transaction expire when not being a parent, should retry | `send_zcl_frame_network_address_fails_because_mac_transaction_expire_should_retry` |
| 73 | 2619 | — | Send zcl frame network address fails because mac no ack, should retry | `send_zcl_frame_network_address_fails_because_mac_no_ack_should_retry` |
| 74 | 2692 | — | Send zcl frame network address fails because mac no ack with network address change, should retry | `send_zcl_frame_network_address_fails_because_mac_no_ack_should_retry` |
| 75 | 2766 | — | Send zcl frame network address fails because mac no ack with network address change, without recovery | `send_zcl_frame_network_address_fails_because_mac_no_ack_should_retry` |
| 76 | 2795 | — | Send zcl frame network address fails because requested profileId is not configured | `send_zcl_frame_network_address_fails_because_requested_profile_is_not_configured` |
| 77 | 2823 | — | Source Endpoint is selected based on profileId | `source_endpoint_is_selected_based_on_profile_id` |
| 78 | 2849 | — | Send zcl frame network address should retry on dataconfirm timeout | `send_zcl_frame_network_address_should_retry_on_dataconfirm_timeout` |
| 79 | 2885 | — | Send zcl frame group | `send_zcl_frame_group` |
| 80 | 2915 | — | Send zcl frame group retry on MAC channel access failure | `send_zcl_frame_group_retry_on_mac_channel_access_failure` |
| 81 | 2956 | — | Send zcl frame to all - DEFAULT | `send_zcl_frame_to_all_default` |
| 82 | 2986 | — | Send zcl frame to all - RX_ON_WHEN_IDLE | `send_zcl_frame_to_all_rx_on_when_idle` |
| 83 | 3016 | — | Send zcl frame to all - SLEEPY | `send_zcl_frame_to_all_sleepy` |
| 84 | 3046 | — | Send zcl frame network address transaction number shouldnt go higher than 255 | `send_zcl_frame_network_address_transaction_number_should_not_go_higher_than_255` |
| 85 | 3086 | — | Send zcl frame group dataConfirm fails | `send_zcl_frame_group_data_confirm_fails` |
| 86 | 3122 | — | Send zcl frame network address matches by tsn | `send_zcl_frame_network_address_matches_by_tsn` |
| 87 | 3189 | — | Send zcl frame network address matches on default response | `send_zcl_frame_network_address_matches_on_default_response` |
| 88 | 3256 | — | Send zcl frame network address data confirm fails with default response | `send_zcl_frame_network_address_data_confirm_fails_with_default_response` |
| 89 | 3270 | — | Send zcl frame network address data confirm fails without default response | `send_zcl_frame_network_address_data_confirm_fails_without_default_response` |
| 90 | 3284 | — | Send zcl frame network address timeout should discover route and retry | `send_zcl_frame_network_address_timeout_should_discover_route_and_retry` |
| 91 | 3343 | — | Send zcl frame network address timeout should discover route, rewrite child entry and retry for sleepy end device | `send_zcl_frame_network_address_timeout_should_rewrite_child_entry_and_retry` |
| 92 | 3404 | — | Send zcl frame network address with default response timeout shouldnt care because command has response | `send_zcl_frame_network_address_with_default_response` |
| 93 | 3450 | — | Supports backup | `supports_backup` |
| 94 | 3664 | — | Incoming message raw (not ZCL) | `incoming_message_raw_not_zcl` |
| 95 | 3688 | — | Device joined | `device_joined_event` |
| 96 | 3700 | — | Device announce | `device_announce_event` |
| 97 | 3722 | — | Device announce should discover route to end devices | `device_announce_should_discover_route_to_end_devices` |
| 98 | 3750 | — | Network address response | `send_zcl_frame_network_address_with_default_response` |
| 99 | 3770 | — | Concentrator Callback Indication | `concentrator_callback_indication` |
| 100 | 3782 | — | Device leave | `device_leave_event` |
| 101 | 3794 | — | Ignore device leave with rejoin | `ignore_device_leave_with_rejoin` |
| 102 | 3806 | — | Do nothing wiht non areq event | `do_nothing_with_non_areq_event` |
| 103 | 3818 | — | Get network parameters | `get_network_parameters` |
| 104 | 3829 | — | Set interpan channel | `set_interpan_channel` |
| 105 | 3846 | — | Restore interpan channel | `restore_interpan_channel` |
| 106 | 3856 | — | Send zcl frame interpan | `send_zcl_frame_interpan_to_ieee_address` |
| 107 | 3883 | — | Send zcl frame interpan with response | `send_zcl_frame_interpan_with_response` |
| 108 | 3939 | — | Send zcl frame interpan throw exception when command has no response | `send_zcl_frame_interpan_throws_when_command_has_no_response` |
| 109 | 3953 | — | Send zcl frame interpan throw exception data request fails | `send_zcl_frame_interpan_throws_when_data_request_fails` |
| 110 | 3968 | — | Refuse to start when ping fails | `refuse_to_start_when_ping_fails` |
| 111 | 3992 | — | Wait for resolves on cmd | `wait_for_resolves_on_cmd` |
| 112 | 4025 | — | Wait for resolves on specified default response | `wait_for_resolves_on_specified_default_response` |
| 113 | 4058 | — | Command should fail when in interpan | `command_should_fail_when_in_interpan` |
| 114 | 4093 | — | Sends proper ZDO request payload for PERMIT_JOINING_REQUEST to target | `sends_proper_zdo_request_payload_for_permit_joining_request_to_target` |
| 115 | 4110 | — | Sends proper ZDO request payload for PERMIT_JOINING_REQUEST broadcast | `sends_proper_zdo_request_payload_for_permit_joining_request_broadcast` |
| 116 | 4132 | — | Sends proper ZDO request payload for NWK_UPDATE_REQUEST UNICAST | `sends_proper_zdo_request_payload_for_nwk_update_request_unicast` |
| 117 | 4156 | — | Sends proper ZDO request payload for NWK_UPDATE_REQUEST BROADCAST | `sends_proper_zdo_request_payload_for_nwk_update_request_broadcast` |
| 118 | 4180 | — | Sends proper ZDO request payload for BIND_REQUEST/UNBIND_REQUEST UNICAST | `sends_proper_zdo_request_payload_for_bind_request_unbind_request_unicast` |
| 119 | 4203 | — | Sends proper ZDO request payload for BIND_REQUEST/UNBIND_REQUEST MULTICAST | `sends_proper_zdo_request_payload_for_bind_request_unbind_request_multicast` |
| 120 | 4241 | — | Sends proper ZDO request payload for NETWORK_ADDRESS_REQUEST | `sends_proper_zdo_request_payload_for_network_address_request` |
| 121 | 4254 | — | Sends proper ZDO request payload for generic logic request | `sends_proper_zdo_request_payload_for_generic_logic_request` |
| 122 | 4267 | — | Node descriptor request should discover route to fix potential fails | `node_descriptor_request_should_discover_route_to_fix_potential_fails` |
| 123 | 4287 | — | Should throw error when ZDO call fails | `should_throw_error_when_zdo_call_fails` |
| 124 | 4300 | — | Should throw error when registerEndpoints fails | `should_throw_error_when_zdo_call_fails` |

## Rust tests beyond the 1:1 core

These 25 tests cover variants/edges of the grouped upstream
`it.each` tables and failure branches that upstream asserts inside a single
case:

- `should_commission_network_with_3_x_0_adapter_auto_concurrency`
- `started_3x0`
- `led_behaviour_disable_led_true_firmware_handling_leds`
- `led_behaviour_disable_led_false_firmware_handling_leds`
- `send_zcl_frame_network_address_with_network_address_change_should_retry`
- `send_zcl_frame_network_address_with_network_address_change_without_recovery`
- `should_restore_unified_backup_with_3_x_0_adapter_and_create_backup_empty`
- `should_create_backup_with_3_0_x_adapter_empty_security_material_table`
- `should_create_backup_with_1_2_adapter`
- `should_fail_to_restore_unified_backup_with_invalid_backup_version`
- `should_fail_to_restore_unified_backup_with_unsupported_backup_format`
- `should_fail_to_restore_legacy_backup_missing_nib`
- `should_fail_to_restore_legacy_backup_preconfigured_key_enabled_missing_material_table`
- `should_start_with_run_inconsistent_option_on_config_adapter_mismatch`
- `should_restore_unified_backup_with_mismatched_adapter_config_but_matching_config_backup`
- `incoming_message_extended_interpan_16_bit_address`
- `incoming_message_extended_interpan_64_bit_address`
- `network_address_response_event`
- `incoming_message_extended_huge_data_16_bit_address`
- `incoming_message_extended_huge_data_64_bit_address`
- `incoming_message_extended_huge_data_retrieve_error_status`
- `incoming_message_extended_huge_data_retrieve_length_mismatch`
- `should_throw_error_when_register_endpoints_fails`
- `should_fail_to_commission_when_bdb_commissioning_times_out`
- `should_fail_to_commission_when_nib_fails_to_settle`

## Notes

- Upstream cases 55/56 (`Call znp constructor`, `Close adapter`) map to
  `call_znp_constructor` / `close_adapter`.
- Rows without an explicit "covered by" note assert on the same level as
  upstream; the serial mock accepts real UNPI frames, so a test cannot pass
  while the adapter sends a different frame.
- `docs/adapter-test-parity.md` remains the mapping for
  `test/adapter/adapter.test.ts` (discovery); this document is the mapping for
  `test/adapter/z-stack/adapter.test.ts` referenced by CP-01.
