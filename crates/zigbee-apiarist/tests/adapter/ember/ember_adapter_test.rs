// Original: .upstream/zigbee-herdsman/test/adapter/ember/emberAdapter.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Adapter impl: addInstallCode
//   - Adapter impl: backup
//   - Adapter impl: getCoordinatorIEEE
//   - Adapter impl: getCoordinatorVersion
//   - Adapter impl: getNetworkParameters from NCP
//   - Adapter impl: getNetworkParameters from cache
//   - Adapter impl: permitJoin on all
//   - Adapter impl: permitJoin on coordinator
//   - Adapter impl: permitJoin on router
//   - Adapter impl: permitJoin restores temp manufacturer code
//   - Adapter impl: reset hard
//   - Adapter impl: reset soft
//   - Adapter impl: resolves undefined when permitJoin on router fails due to failed ZDO status
//   - Adapter impl: restoreChannelInterPAN
//   - Adapter impl: sendZclFrameInterPANBroadcast
//   - Adapter impl: sendZclFrameInterPANToIeeeAddr
//   - Adapter impl: sendZclFrameToAll with fixed endpoint
//   - Adapter impl: sendZclFrameToAll with other endpoint
//   - Adapter impl: sendZclFrameToEndpoint with command response with fixed source endpoint
//   - Adapter impl: sendZclFrameToEndpoint with command response with no source endpoint
//   - Adapter impl: sendZclFrameToEndpoint with command response with other source endpoint
//   - Adapter impl: sendZclFrameToEndpoint with default response to global
//   - Adapter impl: sendZclFrameToEndpoint with default response to specific
//   - Adapter impl: sendZclFrameToEndpoint with encryption
//   - Adapter impl: sendZclFrameToEndpoint without response
//   - Adapter impl: sendZclFrameToGroup with default source endpoint
//   - Adapter impl: sendZclFrameToGroup with source endpoint
//   - Adapter impl: sendZdo with EUI64
//   - Adapter impl: setChannelInterPAN
//   - Adapter impl: supportsBackup
//   - Adapter impl: throw when addInstallCode fails import transient key
//   - Adapter impl: throws when permitJoin broadcast request fails
//   - Adapter impl: throws when permitJoin fails to import ZIGBEE_PROFILE_INTEROPERABILITY_LINK_KEY
//   - Adapter impl: throws when permitJoin fails to set TC policy
//   - Adapter impl: throws when permitJoin request on coordinator fails
//   - Adapter impl: throws when permitJoin request on router fails
//   - Adapter impl: throws when restoreChannelInterPAN fails request
//   - Adapter impl: throws when sendZclFrameInterPANBroadcast command has no response
//   - Adapter impl: throws when sendZclFrameInterPANBroadcast request fails
//   - Adapter impl: throws when sendZclFrameInterPANToIeeeAddr request fails
//   - Adapter impl: throws when sendZclFrameToAll fails request
//   - Adapter impl: throws when sendZclFrameToEndpoint get BUSY status from NCP and exceeded max attempts
//   - Adapter impl: throws when sendZclFrameToEndpoint get BUSY status from NCP and recovery disabled
//   - Adapter impl: throws when sendZclFrameToEndpoint request fails
//   - Adapter impl: throws when sendZclFrameToEndpoint throws NO_TX_SPACE status and recovery disabled
//   - Adapter impl: throws when sendZclFrameToGroup fails request
//   - Adapter impl: throws when setChannelInterPAN fails request
//   - Adapter impl: throws when stop permitJoin fails to restore TC policy
//   - Adapter impl: waitFor
//   - Adapter impl: waitFor resolves on cmd
//   - Adapter impl: waitFor resolves on specified default response
//   - Broadcasts network key update
//   - Creates default instance
//   - Ember Adapter Layer
//   - Emits ZCL payload on greenpower message
//   - Emits ZCL payload on incoming message
//   - Emits ZCL payload on touchlink message
//   - Emits adapter disconnected when NCP needs reset & init but queue is too high
//   - Emits adapter disconnected when failed to reset & init NCP
//   - Emits device announce event on ZDO END_DEVICE_ANNOUNCE
//   - Emits device joined on trust center join
//   - Emits device leave on trust center join
//   - Emits network address event on ZDO NETWORK_ADDRESS_RESPONSE
//   - Exports link keys
//   - Exports zero link keys
//   - Failed to import link keys due to failed key erase
//   - Failed to import link keys due to failed key set
//   - Failed to import link keys due to failed table size retrieval
//   - Failed to import link keys due to improper network state
//   - Failed to import link keys due to insufficient table size
//   - Fails to broadcast network key update due to failed next key broadcast
//   - Fails to broadcast network key update due to failed switch broadcast
//   - Fails to export link keys due to failed table size retrieval
//   - Fails to register message unknown group in multicast table
//   - Fails to start when EZSP layer fails to start
//   - Handles DENY_JOIN on trust center join
//   - Handles NCP needing reset & init
//   - Handles bad ZDO response
//   - Handles channel changed stack status
//   - Handles device join workaround requiring specific manufacturer code
//   - Handles message delivery failure
//   - Imports link keys
//   - Imports zero link keys
//   - Loads custom stack config
//   - Loads only valid custom stack config
//   - Loads only valid custom stack config - null CCA_MODE
//   - Logs failed set config value on start
//   - Logs stack status change
//   - Registers message unknown group in multicast table
//   - Retrieves parameters from NCP when cache invalid
//   - Retrieves parameters from cache when cache valid
//   - Starts and detects when network key frame counter will soon wrap to 0
//   - Starts and skips adding endpoint if already present
//   - Starts and soft-fails if unable to clear key table
//   - Starts but ignores backup if unsupported version
//   - Starts with custom stack config
//   - Starts with custom stack config invalid CCA_MODE
//   - Starts with matching transmit power after form
//   - Starts with mismatching transmit power
//   - Starts with mismatching transmit power, failure does not present start
//   - Starts with reset and forms with given transmit power
//   - Starts with reset when backup/config mismatch
//   - Starts with reset when networks mismatch but no backup available
//   - Starts with restored when network key mismatch but backup available
//   - Starts with restored when network param mismatch but backup available
//   - Starts with restored when no network in adapter
//   - Starts with resumed when everything matches
//   - Stops Ezsp layer on stop
//   - Switches EZSP protocol when supported
//   - Throws when failed to retrieve parameter from NCP
//   - Triggers watchdog counters
//   - Uses default concurrency for queue if not supplied/valid
//   - Warns if NCP has non-GA firmware
//   - When started
//   - incomingMessage
//   - messageSent
//   - ncpNeedsResetAndInit
//   - stackStatus
//   - touchlinkMessage
//   - trustCenterJoin
//   - zdoResponse

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_add_install_code() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_backup() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_get_coordinator_ieee() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_get_coordinator_version() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_get_network_parameters_from_ncp() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_get_network_parameters_from_cache() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_permit_join_on_all() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_permit_join_on_coordinator() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_permit_join_on_router() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_permit_join_restores_temp_manufacturer_code() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_reset_hard() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_reset_soft() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_resolves_undefined_when_permit_join_on_router_fails_due_to_failed_zdo_status() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_restore_channel_inter_pan() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_inter_panbroadcast() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_inter_panto_ieee_addr() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_all_with_fixed_endpoint() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_all_with_other_endpoint() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_endpoint_with_command_response_with_fixed_source_endpoint() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_endpoint_with_command_response_with_no_source_endpoint() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_endpoint_with_command_response_with_other_source_endpoint() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_endpoint_with_default_response_to_global() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_endpoint_with_default_response_to_specific() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_endpoint_with_encryption() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_endpoint_without_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_group_with_default_source_endpoint() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_group_with_source_endpoint() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zdo_with_eui64() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_set_channel_inter_pan() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_supports_backup() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throw_when_add_install_code_fails_import_transient_key() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_permit_join_broadcast_request_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_permit_join_fails_to_import_zigbee_profile_interoperability_link_key() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_permit_join_fails_to_set_tc_policy() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_permit_join_request_on_coordinator_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_permit_join_request_on_router_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_restore_channel_inter_pan_fails_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_send_zcl_frame_inter_panbroadcast_command_has_no_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_send_zcl_frame_inter_panbroadcast_request_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_send_zcl_frame_inter_panto_ieee_addr_request_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_send_zcl_frame_to_all_fails_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_send_zcl_frame_to_endpoint_get_busy_status_from_ncp_and_exceeded_max_attempts(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_send_zcl_frame_to_endpoint_get_busy_status_from_ncp_and_recovery_disabled(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_send_zcl_frame_to_endpoint_request_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_send_zcl_frame_to_endpoint_throws_no_tx_space_status_and_recovery_disabled(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_send_zcl_frame_to_group_fails_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_set_channel_inter_pan_fails_request() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_stop_permit_join_fails_to_restore_tc_policy() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_wait_for() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_wait_for_resolves_on_cmd() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_wait_for_resolves_on_specified_default_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn broadcasts_network_key_update() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn creates_default_instance() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ember_adapter_layer() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn emits_zcl_payload_on_greenpower_message() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn emits_zcl_payload_on_incoming_message() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn emits_zcl_payload_on_touchlink_message() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn emits_adapter_disconnected_when_ncp_needs_reset_init_but_queue_is_too_high() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn emits_adapter_disconnected_when_failed_to_reset_init_ncp() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn emits_device_announce_event_on_zdo_end_device_announce() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn emits_device_joined_on_trust_center_join() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn emits_device_leave_on_trust_center_join() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn emits_network_address_event_on_zdo_network_address_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn exports_link_keys() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn exports_zero_link_keys() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn failed_to_import_link_keys_due_to_failed_key_erase() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn failed_to_import_link_keys_due_to_failed_key_set() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn failed_to_import_link_keys_due_to_failed_table_size_retrieval() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn failed_to_import_link_keys_due_to_improper_network_state() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn failed_to_import_link_keys_due_to_insufficient_table_size() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_to_broadcast_network_key_update_due_to_failed_next_key_broadcast() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_to_broadcast_network_key_update_due_to_failed_switch_broadcast() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_to_export_link_keys_due_to_failed_table_size_retrieval() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_to_register_message_unknown_group_in_multicast_table() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_to_start_when_ezsp_layer_fails_to_start() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_deny_join_on_trust_center_join() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_ncp_needing_reset_init() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_bad_zdo_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_channel_changed_stack_status() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_device_join_workaround_requiring_specific_manufacturer_code() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_message_delivery_failure() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn imports_link_keys() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn imports_zero_link_keys() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn loads_custom_stack_config() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn loads_only_valid_custom_stack_config() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn loads_only_valid_custom_stack_config_null_cca_mode() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn logs_failed_set_config_value_on_start() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn logs_stack_status_change() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn registers_message_unknown_group_in_multicast_table() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn retrieves_parameters_from_ncp_when_cache_invalid() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn retrieves_parameters_from_cache_when_cache_valid() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn starts_and_detects_when_network_key_frame_counter_will_soon_wrap_to_0() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn starts_and_skips_adding_endpoint_if_already_present() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn starts_and_soft_fails_if_unable_to_clear_key_table() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn starts_but_ignores_backup_if_unsupported_version() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_custom_stack_config() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_custom_stack_config_invalid_cca_mode() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_matching_transmit_power_after_form() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_mismatching_transmit_power() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_mismatching_transmit_power_failure_does_not_present_start() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_reset_and_forms_with_given_transmit_power() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_reset_when_backup_config_mismatch() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_reset_when_networks_mismatch_but_no_backup_available() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_restored_when_network_key_mismatch_but_backup_available() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_restored_when_network_param_mismatch_but_backup_available() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_restored_when_no_network_in_adapter() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_resumed_when_everything_matches() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn stops_ezsp_layer_on_stop() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn switches_ezsp_protocol_when_supported() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_failed_to_retrieve_parameter_from_ncp() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn triggers_watchdog_counters() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uses_default_concurrency_for_queue_if_not_supplied_valid() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn warns_if_ncp_has_non_ga_firmware() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn when_started() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn incoming_message() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn message_sent() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ncp_needs_reset_and_init() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn stack_status() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn touchlink_message() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn trust_center_join() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zdo_response() {
    todo!()
}
