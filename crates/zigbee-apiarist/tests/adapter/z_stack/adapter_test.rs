// Original: .upstream/zigbee-herdsman/test/adapter/z-stack/adapter.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Adapter disconnected
//   - Adapter disconnected dont emit when closing
//   - Add install code: Install Code + CRC
//   - Add install code: Key derived from Install Code
//   - Call znp constructor
//   - Close adapter
//   - Command should fail when in interpan
//   - Concentrator Callback Indication
//   - Device announce
//   - Device announce should discover route to end devices
//   - Device joined
//   - Device leave
//   - Do nothing wiht non areq event
//   - Get coordinator IEEE
//   - Get coordinator version
//   - Get network parameters
//   - Hard reset
//   - Ignore device leave with rejoin
//   - Incoming message raw (not ZCL)
//   - LED behaviour: disable LED false, firmware handling leds
//   - LED behaviour: disable LED false, firmware not handling leds
//   - LED behaviour: disable LED true, firmware handling leds
//   - LED behaviour: disable LED true, firmware not handling leds
//   - Network address response
//   - Node descriptor request should discover route to fix potential fails
//   - Permit join all
//   - Permit join coordinator only
//   - Permit join specific networkAddress
//   - Refuse to start when ping fails
//   - Restore interpan channel
//   - Send zcl frame group
//   - Send zcl frame group dataConfirm fails
//   - Send zcl frame group retry on MAC channel access failure
//   - Send zcl frame interpan
//   - Send zcl frame interpan throw exception data request fails
//   - Send zcl frame interpan throw exception when command has no response
//   - Send zcl frame interpan with response
//   - Send zcl frame network address
//   - Send zcl frame network address data confirm fails with default response
//   - Send zcl frame network address data confirm fails without default response
//   - Send zcl frame network address dataConfirm fails
//   - Send zcl frame network address fails because mac no ack with network address change, should retry
//   - Send zcl frame network address fails because mac no ack with network address change, without recovery
//   - Send zcl frame network address fails because mac no ack, should retry
//   - Send zcl frame network address fails because mac transaction expire when not being a parent, should retry
//   - Send zcl frame network address fails because mac transaction expire, should retry
//   - Send zcl frame network address fails because requested profileId is not configured
//   - Send zcl frame network address matches by tsn
//   - Send zcl frame network address matches on default response
//   - Send zcl frame network address retry on MAC channel access failure
//   - Send zcl frame network address should retry on dataconfirm timeout
//   - Send zcl frame network address timeout should discover route and retry
//   - Send zcl frame network address timeout should discover route, rewrite child entry and retry for sleepy end device
//   - Send zcl frame network address transaction number shouldnt go higher than 255
//   - Send zcl frame network address with default response
//   - Send zcl frame network address with default response timeout shouldnt care because command has response
//   - Send zcl frame to all - DEFAULT
//   - Send zcl frame to all - RX_ON_WHEN_IDLE
//   - Send zcl frame to all - SLEEPY
//   - Send zcl frame with APS encryption
//   - Sends proper ZDO request payload for BIND_REQUEST/UNBIND_REQUEST MULTICAST
//   - Sends proper ZDO request payload for BIND_REQUEST/UNBIND_REQUEST UNICAST
//   - Sends proper ZDO request payload for NETWORK_ADDRESS_REQUEST
//   - Sends proper ZDO request payload for NWK_UPDATE_REQUEST BROADCAST
//   - Sends proper ZDO request payload for NWK_UPDATE_REQUEST UNICAST
//   - Sends proper ZDO request payload for PERMIT_JOINING_REQUEST broadcast
//   - Sends proper ZDO request payload for PERMIT_JOINING_REQUEST to target
//   - Sends proper ZDO request payload for generic logic request
//   - Set interpan channel
//   - Should throw error when ZDO call fails
//   - Should throw error when registerEndpoints fails
//   - Soft reset
//   - Source Endpoint is selected based on profileId
//   - Start with transmit power set
//   - Support LED should go to false when LED request fails
//   - Supports backup
//   - Wait for resolves on cmd
//   - Wait for resolves on specified default response
//   - should (recommission) restore unified backup with 1.2 adapter and create backup - empty
//   - should commission network with 1.2 adapter
//   - should commission network with 1.2 adapter - default extended pan id
//   - should commission network with 1.2 adapter - old adapter without version reporting
//   - should commission network with 3.0.x adapter
//   - should commission network with 3.0.x adapter - auto concurrency
//   - should commission network with 3.0.x adapter - default extended pan id
//   - should commission network with 3.0.x adapter - unaligned 8-bit
//   - should commission network with 3.x.0 adapter - auto concurrency
//   - should commission with 3.0.x adapter - commissioned, mismatched adapter-config-backup
//   - should commission with 3.0.x adapter - empty, mismatched config/backup
//   - should create backup with 1.2 adapter
//   - should create backup with 3.0.x adapter - default security material table entry
//   - should create backup with 3.0.x adapter - emnpty security material table
//   - should create backup with 3.0.x adapter - security material table with generic record
//   - should fail to commission network with 3.0.x adapter when bdb commissioning times out
//   - should fail to commission network with 3.0.x adapter when nib fails to settle
//   - should fail to commission network with 3.0.x adapter when nib reports different pan id
//   - should fail to commission network with 3.0.x adapter with invalid pan id 65535
//   - should fail to create backup with 3.0.x adapter - adapter not commissioned - missing nib
//   - should fail to create backup with 3.0.x adapter - missing active key info
//   - should fail to create backup with 3.0.x adapter - unable to read ieee address
//   - should fail to restore (unified) backup with 3.0.x adapter - unsupported backup format
//   - should fail to restore legacy backup with 3.0.x adapter - missing NIB
//   - should fail to restore legacy backup with 3.0.x adapter - missing active key info
//   - should fail to restore legacy backup with 3.0.x adapter - missing adapter ieee address
//   - should fail to restore legacy backup with 3.0.x adapter - missing pre-configured key enabled
//   - should fail to restore legacy backup with 3.0.x adapter - pre-configured key enabled
//   - should fail to restore unified backup with 1.2 adapter - backup from newer adapter
//   - should fail to restore unified backup with 3.0.x adapter - insufficient aps link key data table size
//   - should fail to restore unified backup with 3.0.x adapter - insufficient security manager table size
//   - should fail to restore unified backup with 3.0.x adapter - insufficient tclk table size
//   - should fail to restore unified backup with 3.0.x adapter - invalid open coordinator backup version
//   - should fail to start with 3.0.x adapter - commissioned, config-adapter mismatch
//   - should fail when backup file is corrupted - Coordinator backup is corrupted
//   - should keep missing devices in backup
//   - should reset network with 1.2 adapter - config mismtach
//   - should restore legacy backup with 3.0.x adapter - empty
//   - should restore unified backup with 3.0.x adapter - commissioned, mismatched adapter-config, matching config-backup
//   - should restore unified backup with 3.0.x adapter and create backup - empty
//   - should restore unified backup with 3.0.x adapter and create backup - no tclk seed
//   - should restore unified backup with 3.x.0 adapter and create backup - empty
//   - should start network with 3.0.x adapter
//   - should start network with 3.0.x adapter - resume in coordinator mode
//   - should start network with 3.0.x adapter - resume in coordinator mode, extGroupFind failed
//   - should start with 3.0.x adapter - backward-compat - reversed extended pan id
//   - should start with 3.x.0 adapter - warn when preconfigured key doesn
//   - should start with runInconsistent option with 3.0.x adapter - commissioned, config-adapter mismatch
//   - zstack-adapter

#[test]
#[ignore = "not implemented yet"]
fn adapter_disconnected() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_disconnected_dont_emit_when_closing() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn add_install_code_install_code_crc() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn add_install_code_key_derived_from_install_code() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn call_znp_constructor() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn close_adapter() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn command_should_fail_when_in_interpan() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn concentrator_callback_indication() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_announce() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_announce_should_discover_route_to_end_devices() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_joined() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn device_leave() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn do_nothing_wiht_non_areq_event() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_coordinator_ieee() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_coordinator_version() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn get_network_parameters() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn hard_reset() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ignore_device_leave_with_rejoin() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn incoming_message_raw_not_zcl() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn led_behaviour_disable_led_false_firmware_handling_leds() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn led_behaviour_disable_led_false_firmware_not_handling_leds() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn led_behaviour_disable_led_true_firmware_handling_leds() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn led_behaviour_disable_led_true_firmware_not_handling_leds() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn network_address_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn node_descriptor_request_should_discover_route_to_fix_potential_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn permit_join_all() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn permit_join_coordinator_only() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn permit_join_specific_network_address() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn refuse_to_start_when_ping_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn restore_interpan_channel() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_group() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_group_data_confirm_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_group_retry_on_mac_channel_access_failure() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_interpan() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_interpan_throw_exception_data_request_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_interpan_throw_exception_when_command_has_no_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_interpan_with_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_data_confirm_fails_with_default_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_data_confirm_fails_without_default_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_data_confirm_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_fails_because_mac_no_ack_with_network_address_change_should_retry() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_fails_because_mac_no_ack_with_network_address_change_without_recovery() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_fails_because_mac_no_ack_should_retry() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_fails_because_mac_transaction_expire_when_not_being_a_parent_should_retry() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_fails_because_mac_transaction_expire_should_retry() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_fails_because_requested_profile_id_is_not_configured() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_matches_by_tsn() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_matches_on_default_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_retry_on_mac_channel_access_failure() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_should_retry_on_dataconfirm_timeout() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_timeout_should_discover_route_and_retry() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_timeout_should_discover_route_rewrite_child_entry_and_retry_for_sleepy_end_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_transaction_number_shouldnt_go_higher_than_255() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_with_default_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_network_address_with_default_response_timeout_shouldnt_care_because_command_has_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_to_all_default() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_to_all_rx_on_when_idle() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_to_all_sleepy() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn send_zcl_frame_with_aps_encryption() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn sends_proper_zdo_request_payload_for_bind_request_unbind_request_multicast() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn sends_proper_zdo_request_payload_for_bind_request_unbind_request_unicast() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn sends_proper_zdo_request_payload_for_network_address_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn sends_proper_zdo_request_payload_for_nwk_update_request_broadcast() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn sends_proper_zdo_request_payload_for_nwk_update_request_unicast() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn sends_proper_zdo_request_payload_for_permit_joining_request_broadcast() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn sends_proper_zdo_request_payload_for_permit_joining_request_to_target() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn sends_proper_zdo_request_payload_for_generic_logic_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn set_interpan_channel() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_throw_error_when_zdo_call_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_throw_error_when_register_endpoints_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn soft_reset() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn source_endpoint_is_selected_based_on_profile_id() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn start_with_transmit_power_set() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn support_led_should_go_to_false_when_led_request_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn supports_backup() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn wait_for_resolves_on_cmd() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn wait_for_resolves_on_specified_default_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_recommission_restore_unified_backup_with_1_2_adapter_and_create_backup_empty() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_commission_network_with_1_2_adapter() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_commission_network_with_1_2_adapter_default_extended_pan_id() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_commission_network_with_1_2_adapter_old_adapter_without_version_reporting() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_commission_network_with_3_0_x_adapter() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_commission_network_with_3_0_x_adapter_auto_concurrency() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_commission_network_with_3_0_x_adapter_default_extended_pan_id() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_commission_network_with_3_0_x_adapter_unaligned_8_bit() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_commission_network_with_3_x_0_adapter_auto_concurrency() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_commission_with_3_0_x_adapter_commissioned_mismatched_adapter_config_backup() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_commission_with_3_0_x_adapter_empty_mismatched_config_backup() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_create_backup_with_1_2_adapter() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_create_backup_with_3_0_x_adapter_default_security_material_table_entry() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_create_backup_with_3_0_x_adapter_emnpty_security_material_table() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_create_backup_with_3_0_x_adapter_security_material_table_with_generic_record() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_commission_network_with_3_0_x_adapter_when_bdb_commissioning_times_out() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_commission_network_with_3_0_x_adapter_when_nib_fails_to_settle() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_commission_network_with_3_0_x_adapter_when_nib_reports_different_pan_id() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_commission_network_with_3_0_x_adapter_with_invalid_pan_id_65535() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_create_backup_with_3_0_x_adapter_adapter_not_commissioned_missing_nib() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_create_backup_with_3_0_x_adapter_missing_active_key_info() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_create_backup_with_3_0_x_adapter_unable_to_read_ieee_address() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_restore_unified_backup_with_3_0_x_adapter_unsupported_backup_format() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_restore_legacy_backup_with_3_0_x_adapter_missing_nib() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_restore_legacy_backup_with_3_0_x_adapter_missing_active_key_info() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_restore_legacy_backup_with_3_0_x_adapter_missing_adapter_ieee_address() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_restore_legacy_backup_with_3_0_x_adapter_missing_pre_configured_key_enabled() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_restore_legacy_backup_with_3_0_x_adapter_pre_configured_key_enabled() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_restore_unified_backup_with_1_2_adapter_backup_from_newer_adapter() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_restore_unified_backup_with_3_0_x_adapter_insufficient_aps_link_key_data_table_size() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_restore_unified_backup_with_3_0_x_adapter_insufficient_security_manager_table_size() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_restore_unified_backup_with_3_0_x_adapter_insufficient_tclk_table_size() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_restore_unified_backup_with_3_0_x_adapter_invalid_open_coordinator_backup_version() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_to_start_with_3_0_x_adapter_commissioned_config_adapter_mismatch() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_fail_when_backup_file_is_corrupted_coordinator_backup_is_corrupted() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_keep_missing_devices_in_backup() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_reset_network_with_1_2_adapter_config_mismtach() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_restore_legacy_backup_with_3_0_x_adapter_empty() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_restore_unified_backup_with_3_0_x_adapter_commissioned_mismatched_adapter_config_matching_config_backup() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_restore_unified_backup_with_3_0_x_adapter_and_create_backup_empty() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_restore_unified_backup_with_3_0_x_adapter_and_create_backup_no_tclk_seed() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_restore_unified_backup_with_3_x_0_adapter_and_create_backup_empty() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_start_network_with_3_0_x_adapter() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_start_network_with_3_0_x_adapter_resume_in_coordinator_mode() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_start_network_with_3_0_x_adapter_resume_in_coordinator_mode_ext_group_find_failed() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_start_with_3_0_x_adapter_backward_compat_reversed_extended_pan_id() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_start_with_3_x_0_adapter_warn_when_preconfigured_key_doesn() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_start_with_run_inconsistent_option_with_3_0_x_adapter_commissioned_config_adapter_mismatch() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zstack_adapter() {
    todo!()
}

