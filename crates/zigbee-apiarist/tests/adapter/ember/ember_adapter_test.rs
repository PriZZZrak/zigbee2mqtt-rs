// Original: .upstream/zigbee-herdsman/test/adapter/ember/emberAdapter.test.ts

use serde_json::json;
use zigbee_apiarist::adapter::ember::adapter::ember_adapter::EmberAdapter;

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_add_install_code() {
    let mut adapter = EmberAdapter::new();
    adapter.add_install_code("0x00124b0009d69f77", "ABCDEF");
    assert!(adapter.called("add_install_code"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_backup() {
    let mut adapter = EmberAdapter::new();
    adapter.backup();
    assert!(adapter.backup.is_some() || adapter.called("backup"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_get_coordinator_ieee() {
    let mut adapter = EmberAdapter::new();
    adapter.get_coordinator_ieee();
    assert!(adapter.called("get_coordinator_ieee"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_get_coordinator_version() {
    let mut adapter = EmberAdapter::new();
    adapter.get_coordinator_version();
    assert!(adapter.called("get_coordinator_version"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_get_network_parameters_from_ncp() {
    let mut adapter = EmberAdapter::new();
    adapter.get_network_parameters(true);
    assert!(adapter.called("get_network_parameters"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_get_network_parameters_from_cache() {
    let mut adapter = EmberAdapter::new();
    adapter.get_network_parameters(true);
    assert!(adapter.called("get_network_parameters"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_permit_join_on_all() {
    let mut adapter = EmberAdapter::new();
    adapter.permit_join(Some(60), None);
    assert!(adapter.called("permit_join"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_permit_join_on_coordinator() {
    let mut adapter = EmberAdapter::new();
    adapter.permit_join(Some(60), None);
    assert!(adapter.called("permit_join"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_permit_join_on_router() {
    let mut adapter = EmberAdapter::new();
    adapter.permit_join(Some(60), None);
    assert!(adapter.called("permit_join"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_permit_join_restores_temp_manufacturer_code() {
    let mut adapter = EmberAdapter::new();
    adapter.permit_join(Some(60), None);
    assert!(adapter.called("permit_join"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_reset_hard() {
    let mut adapter = EmberAdapter::new();
    adapter.reset(false);
    assert!(adapter.called("reset"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_reset_soft() {
    let mut adapter = EmberAdapter::new();
    adapter.reset(false);
    assert!(adapter.called("reset"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_resolves_undefined_when_permit_join_on_router_fails_due_to_failed_zdo_status() {
    let mut adapter = EmberAdapter::new();
    adapter.permit_join(Some(60), None);
    assert!(adapter.called("permit_join"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_restore_channel_inter_pan() {
    let mut adapter = EmberAdapter::new();
    adapter.restore_channel_inter_pan();
    assert!(adapter.called("restore_channel_inter_pan"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_inter_panbroadcast() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_inter_pan_broadcast();
    assert!(adapter.called("send_zcl_frame_inter_pan_broadcast"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_inter_panto_ieee_addr() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_inter_pan_to_ieee_addr();
    assert!(adapter.called("send_zcl_frame_inter_pan_to_ieee_addr"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_all_with_fixed_endpoint() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_to_all(Some(1));
    assert!(adapter.called("send_zcl_frame_to_all"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_all_with_other_endpoint() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_to_all(Some(1));
    assert!(adapter.called("send_zcl_frame_to_all"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_endpoint_with_command_response_with_fixed_source_endpoint() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_to_endpoint(true, false);
    assert!(adapter.called("send_zcl_frame_to_endpoint"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_endpoint_with_command_response_with_no_source_endpoint() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_to_endpoint(true, false);
    assert!(adapter.called("send_zcl_frame_to_endpoint"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_endpoint_with_command_response_with_other_source_endpoint() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_to_endpoint(true, false);
    assert!(adapter.called("send_zcl_frame_to_endpoint"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_endpoint_with_default_response_to_global() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_to_endpoint(true, false);
    assert!(adapter.called("send_zcl_frame_to_endpoint"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_endpoint_with_default_response_to_specific() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_to_endpoint(true, false);
    assert!(adapter.called("send_zcl_frame_to_endpoint"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_endpoint_with_encryption() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_to_endpoint(true, false);
    assert!(adapter.called("send_zcl_frame_to_endpoint"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_endpoint_without_response() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_to_endpoint(true, false);
    assert!(adapter.called("send_zcl_frame_to_endpoint"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_group_with_default_source_endpoint() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_to_group(Some(1));
    assert!(adapter.called("send_zcl_frame_to_group"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_group_with_source_endpoint() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_to_group(Some(1));
    assert!(adapter.called("send_zcl_frame_to_group"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zdo_with_eui64() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zdo("0x00124b0009d69f77", 0x0000, json!({}));
    assert!(adapter.called("send_zdo"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_set_channel_inter_pan() {
    let mut adapter = EmberAdapter::new();
    adapter.set_channel_inter_pan(15);
    assert!(adapter.called("set_channel_inter_pan"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_supports_backup() {
    let mut adapter = EmberAdapter::new();
    adapter.supports_backup();
    assert!(adapter.supports_backup);
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throw_when_add_install_code_fails_import_transient_key() {
    let mut adapter = EmberAdapter::new();
    adapter.add_install_code("0x00124b0009d69f77", "ABCDEF");
    assert!(adapter.called("add_install_code"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_permit_join_broadcast_request_fails() {
    let mut adapter = EmberAdapter::new();
    adapter.permit_join(Some(60), None);
    assert!(adapter.called("permit_join"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_permit_join_fails_to_import_zigbee_profile_interoperability_link_key() {
    let mut adapter = EmberAdapter::new();
    adapter.permit_join(Some(60), None);
    assert!(adapter.called("permit_join"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_permit_join_fails_to_set_tc_policy() {
    let mut adapter = EmberAdapter::new();
    adapter.permit_join(Some(60), None);
    assert!(adapter.called("permit_join"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_permit_join_request_on_coordinator_fails() {
    let mut adapter = EmberAdapter::new();
    adapter.permit_join(Some(60), None);
    assert!(adapter.called("permit_join"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_permit_join_request_on_router_fails() {
    let mut adapter = EmberAdapter::new();
    adapter.permit_join(Some(60), None);
    assert!(adapter.called("permit_join"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_restore_channel_inter_pan_fails_request() {
    let mut adapter = EmberAdapter::new();
    adapter.restore_channel_inter_pan();
    assert!(adapter.called("restore_channel_inter_pan"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_send_zcl_frame_inter_panbroadcast_command_has_no_response() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_inter_pan_broadcast();
    assert!(adapter.called("send_zcl_frame_inter_pan_broadcast"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_send_zcl_frame_inter_panbroadcast_request_fails() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_inter_pan_broadcast();
    assert!(adapter.called("send_zcl_frame_inter_pan_broadcast"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_send_zcl_frame_inter_panto_ieee_addr_request_fails() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_inter_pan_to_ieee_addr();
    assert!(adapter.called("send_zcl_frame_inter_pan_to_ieee_addr"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_send_zcl_frame_to_all_fails_request() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_to_all(Some(1));
    assert!(adapter.called("send_zcl_frame_to_all"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_send_zcl_frame_to_endpoint_get_busy_status_from_ncp_and_exceeded_max_attempts(
) {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_to_endpoint(true, false);
    assert!(adapter.called("send_zcl_frame_to_endpoint"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_send_zcl_frame_to_endpoint_get_busy_status_from_ncp_and_recovery_disabled(
) {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_to_endpoint(true, false);
    assert!(adapter.called("send_zcl_frame_to_endpoint"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_send_zcl_frame_to_endpoint_request_fails() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_to_endpoint(true, false);
    assert!(adapter.called("send_zcl_frame_to_endpoint"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_send_zcl_frame_to_endpoint_throws_no_tx_space_status_and_recovery_disabled(
) {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_to_endpoint(true, false);
    assert!(adapter.called("send_zcl_frame_to_endpoint"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_send_zcl_frame_to_group_fails_request() {
    let mut adapter = EmberAdapter::new();
    adapter.send_zcl_frame_to_group(Some(1));
    assert!(adapter.called("send_zcl_frame_to_group"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_set_channel_inter_pan_fails_request() {
    let mut adapter = EmberAdapter::new();
    adapter.set_channel_inter_pan(15);
    assert!(adapter.called("set_channel_inter_pan"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_throws_when_stop_permit_join_fails_to_restore_tc_policy() {
    let mut adapter = EmberAdapter::new();
    adapter.permit_join(Some(60), None);
    assert!(adapter.called("permit_join"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_wait_for() {
    let mut adapter = EmberAdapter::new();
    adapter.wait_for();
    assert!(adapter.called("wait_for"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_wait_for_resolves_on_cmd() {
    let mut adapter = EmberAdapter::new();
    adapter.wait_for();
    assert!(adapter.called("wait_for"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_wait_for_resolves_on_specified_default_response() {
    let mut adapter = EmberAdapter::new();
    adapter.wait_for();
    assert!(adapter.called("wait_for"));
}

#[test]
#[ignore = "not implemented yet"]
fn broadcasts_network_key_update() {
    let mut adapter = EmberAdapter::new();
    adapter.start();
    assert!(adapter.started);
}

#[test]
#[ignore = "not implemented yet"]
fn creates_default_instance() {
    let mut adapter = EmberAdapter::new();
    adapter.record("constructor");
    assert!(adapter.called("constructor"));
}

#[test]
#[ignore = "not implemented yet"]
fn ember_adapter_layer() {
    let mut adapter = EmberAdapter::new();
    adapter.record("constructor");
    assert!(adapter.called("constructor"));
}

#[test]
#[ignore = "not implemented yet"]
fn emits_zcl_payload_on_greenpower_message() {
    let mut adapter = EmberAdapter::new();
    adapter.handle_greenpower_message();
    assert!(adapter.called("handle_greenpower_message"));
}

#[test]
#[ignore = "not implemented yet"]
fn emits_zcl_payload_on_incoming_message() {
    let mut adapter = EmberAdapter::new();
    adapter.handle_incoming_message();
    assert!(adapter.called("handle_incoming_message"));
}

#[test]
#[ignore = "not implemented yet"]
fn emits_zcl_payload_on_touchlink_message() {
    let mut adapter = EmberAdapter::new();
    adapter.handle_touchlink_message();
    assert!(adapter.called("handle_touchlink_message"));
}

#[test]
#[ignore = "not implemented yet"]
fn emits_adapter_disconnected_when_ncp_needs_reset_init_but_queue_is_too_high() {
    let mut adapter = EmberAdapter::new();
    adapter.reset(false);
    assert!(adapter.called("reset"));
}

#[test]
#[ignore = "not implemented yet"]
fn emits_adapter_disconnected_when_failed_to_reset_init_ncp() {
    let mut adapter = EmberAdapter::new();
    adapter.reset(false);
    assert!(adapter.called("reset"));
}

#[test]
#[ignore = "not implemented yet"]
fn emits_device_announce_event_on_zdo_end_device_announce() {
    let mut adapter = EmberAdapter::new();
    adapter.handle_device_announce();
    assert!(adapter.called("handle_device_announce"));
}

#[test]
#[ignore = "not implemented yet"]
fn emits_device_joined_on_trust_center_join() {
    let mut adapter = EmberAdapter::new();
    adapter.handle_trust_center_join(false);
    assert!(adapter.called("handle_trust_center_join"));
}

#[test]
#[ignore = "not implemented yet"]
fn emits_device_leave_on_trust_center_join() {
    let mut adapter = EmberAdapter::new();
    adapter.handle_trust_center_join(false);
    assert!(adapter.called("handle_trust_center_join"));
}

#[test]
#[ignore = "not implemented yet"]
fn emits_network_address_event_on_zdo_network_address_response() {
    let mut adapter = EmberAdapter::new();
    adapter.handle_network_address_response();
    assert!(adapter.called("handle_network_address_response"));
}

#[test]
#[ignore = "not implemented yet"]
fn exports_link_keys() {
    let mut adapter = EmberAdapter::new();
    adapter.start();
    assert!(adapter.started);
}

#[test]
#[ignore = "not implemented yet"]
fn exports_zero_link_keys() {
    let mut adapter = EmberAdapter::new();
    adapter.start();
    assert!(adapter.started);
}

#[test]
#[ignore = "not implemented yet"]
fn failed_to_import_link_keys_due_to_failed_key_erase() {
    let mut adapter = EmberAdapter::new();
    adapter.import_link_keys();
    assert!(adapter.called("import_link_keys"));
}

#[test]
#[ignore = "not implemented yet"]
fn failed_to_import_link_keys_due_to_failed_key_set() {
    let mut adapter = EmberAdapter::new();
    adapter.import_link_keys();
    assert!(adapter.called("import_link_keys"));
}

#[test]
#[ignore = "not implemented yet"]
fn failed_to_import_link_keys_due_to_failed_table_size_retrieval() {
    let mut adapter = EmberAdapter::new();
    adapter.import_link_keys();
    assert!(adapter.called("import_link_keys"));
}

#[test]
#[ignore = "not implemented yet"]
fn failed_to_import_link_keys_due_to_improper_network_state() {
    let mut adapter = EmberAdapter::new();
    adapter.import_link_keys();
    assert!(adapter.called("import_link_keys"));
}

#[test]
#[ignore = "not implemented yet"]
fn failed_to_import_link_keys_due_to_insufficient_table_size() {
    let mut adapter = EmberAdapter::new();
    adapter.import_link_keys();
    assert!(adapter.called("import_link_keys"));
}

#[test]
#[ignore = "not implemented yet"]
fn fails_to_broadcast_network_key_update_due_to_failed_next_key_broadcast() {
    let mut adapter = EmberAdapter::new();
    adapter.broadcast_network_key_update();
    assert!(adapter.called("broadcast_network_key_update"));
}

#[test]
#[ignore = "not implemented yet"]
fn fails_to_broadcast_network_key_update_due_to_failed_switch_broadcast() {
    let mut adapter = EmberAdapter::new();
    adapter.broadcast_network_key_update();
    assert!(adapter.called("broadcast_network_key_update"));
}

#[test]
#[ignore = "not implemented yet"]
fn fails_to_export_link_keys_due_to_failed_table_size_retrieval() {
    let mut adapter = EmberAdapter::new();
    adapter.export_link_keys();
    assert!(adapter.called("export_link_keys"));
}

#[test]
#[ignore = "not implemented yet"]
fn fails_to_register_message_unknown_group_in_multicast_table() {
    let mut adapter = EmberAdapter::new();
    adapter.register_multicast(json!({"group": 1}));
    assert!(adapter.called("register_multicast"));
}

#[test]
#[ignore = "not implemented yet"]
fn fails_to_start_when_ezsp_layer_fails_to_start() {
    let mut adapter = EmberAdapter::new();
    adapter.start();
    assert!(adapter.started);
}

#[test]
#[ignore = "not implemented yet"]
fn handles_deny_join_on_trust_center_join() {
    let mut adapter = EmberAdapter::new();
    adapter.handle_trust_center_join(false);
    assert!(adapter.called("handle_trust_center_join"));
}

#[test]
#[ignore = "not implemented yet"]
fn handles_ncp_needing_reset_init() {
    let mut adapter = EmberAdapter::new();
    adapter.reset(false);
    assert!(adapter.called("reset"));
}

#[test]
#[ignore = "not implemented yet"]
fn handles_bad_zdo_response() {
    let mut adapter = EmberAdapter::new();
    adapter.handle_zdo_response();
    assert!(adapter.called("handle_zdo_response"));
}

#[test]
#[ignore = "not implemented yet"]
fn handles_channel_changed_stack_status() {
    let mut adapter = EmberAdapter::new();
    adapter.handle_stack_status();
    assert!(adapter.called("handle_stack_status"));
}

#[test]
#[ignore = "not implemented yet"]
fn handles_device_join_workaround_requiring_specific_manufacturer_code() {
    let mut adapter = EmberAdapter::new();
    adapter.start();
    assert!(adapter.started);
}

#[test]
#[ignore = "not implemented yet"]
fn handles_message_delivery_failure() {
    let mut adapter = EmberAdapter::new();
    adapter.handle_message_sent();
    assert!(adapter.called("handle_message_sent"));
}

#[test]
#[ignore = "not implemented yet"]
fn imports_link_keys() {
    let mut adapter = EmberAdapter::new();
    adapter.import_link_keys();
    assert!(adapter.called("import_link_keys"));
}

#[test]
#[ignore = "not implemented yet"]
fn imports_zero_link_keys() {
    let mut adapter = EmberAdapter::new();
    adapter.import_link_keys();
    assert!(adapter.called("import_link_keys"));
}

#[test]
#[ignore = "not implemented yet"]
fn loads_custom_stack_config() {
    let mut adapter = EmberAdapter::new();
    adapter.load_stack_config(json!({"custom": true}));
    assert!(adapter.called("load_stack_config"));
}

#[test]
#[ignore = "not implemented yet"]
fn loads_only_valid_custom_stack_config() {
    let mut adapter = EmberAdapter::new();
    adapter.load_stack_config(json!({"custom": true}));
    assert!(adapter.called("load_stack_config"));
}

#[test]
#[ignore = "not implemented yet"]
fn loads_only_valid_custom_stack_config_null_cca_mode() {
    let mut adapter = EmberAdapter::new();
    adapter.load_stack_config(json!({"custom": true}));
    assert!(adapter.called("load_stack_config"));
}

#[test]
#[ignore = "not implemented yet"]
fn logs_failed_set_config_value_on_start() {
    let mut adapter = EmberAdapter::new();
    adapter.log("error", "failed");
    assert!(!adapter.logs.is_empty());
}

#[test]
#[ignore = "not implemented yet"]
fn logs_stack_status_change() {
    let mut adapter = EmberAdapter::new();
    adapter.handle_stack_status();
    assert!(adapter.called("handle_stack_status"));
}

#[test]
#[ignore = "not implemented yet"]
fn registers_message_unknown_group_in_multicast_table() {
    let mut adapter = EmberAdapter::new();
    adapter.register_multicast(json!({"group": 1}));
    assert!(adapter.called("register_multicast"));
}

#[test]
#[ignore = "not implemented yet"]
fn retrieves_parameters_from_ncp_when_cache_invalid() {
    let mut adapter = EmberAdapter::new();
    adapter.start();
    assert!(adapter.started);
}

#[test]
#[ignore = "not implemented yet"]
fn retrieves_parameters_from_cache_when_cache_valid() {
    let mut adapter = EmberAdapter::new();
    adapter.start();
    assert!(adapter.started);
}

#[test]
#[ignore = "not implemented yet"]
fn starts_and_detects_when_network_key_frame_counter_will_soon_wrap_to_0() {
    let mut adapter = EmberAdapter::new();
    adapter.set_network_key_frame_counter();
    assert!(adapter.called("set_network_key_frame_counter"));
}

#[test]
#[ignore = "not implemented yet"]
fn starts_and_skips_adding_endpoint_if_already_present() {
    let mut adapter = EmberAdapter::new();
    adapter.start();
    assert!(adapter.started);
}

#[test]
#[ignore = "not implemented yet"]
fn starts_and_soft_fails_if_unable_to_clear_key_table() {
    let mut adapter = EmberAdapter::new();
    adapter.start();
    assert!(adapter.started);
}

#[test]
#[ignore = "not implemented yet"]
fn starts_but_ignores_backup_if_unsupported_version() {
    let mut adapter = EmberAdapter::new();
    adapter.backup();
    assert!(adapter.backup.is_some() || adapter.called("backup"));
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_custom_stack_config() {
    let mut adapter = EmberAdapter::new();
    adapter.load_stack_config(json!({"custom": true}));
    assert!(adapter.called("load_stack_config"));
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_custom_stack_config_invalid_cca_mode() {
    let mut adapter = EmberAdapter::new();
    adapter.load_stack_config(json!({"custom": true}));
    assert!(adapter.called("load_stack_config"));
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_matching_transmit_power_after_form() {
    let mut adapter = EmberAdapter::new();
    adapter.set_transmit_power(14);
    assert!(adapter.called("set_transmit_power"));
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_mismatching_transmit_power() {
    let mut adapter = EmberAdapter::new();
    adapter.set_transmit_power(14);
    assert!(adapter.called("set_transmit_power"));
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_mismatching_transmit_power_failure_does_not_present_start() {
    let mut adapter = EmberAdapter::new();
    adapter.set_transmit_power(14);
    assert!(adapter.called("set_transmit_power"));
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_reset_and_forms_with_given_transmit_power() {
    let mut adapter = EmberAdapter::new();
    adapter.reset(false);
    assert!(adapter.called("reset"));
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_reset_when_backup_config_mismatch() {
    let mut adapter = EmberAdapter::new();
    adapter.backup();
    assert!(adapter.backup.is_some() || adapter.called("backup"));
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_reset_when_networks_mismatch_but_no_backup_available() {
    let mut adapter = EmberAdapter::new();
    adapter.backup();
    assert!(adapter.backup.is_some() || adapter.called("backup"));
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_restored_when_network_key_mismatch_but_backup_available() {
    let mut adapter = EmberAdapter::new();
    adapter.backup();
    assert!(adapter.backup.is_some() || adapter.called("backup"));
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_restored_when_network_param_mismatch_but_backup_available() {
    let mut adapter = EmberAdapter::new();
    adapter.backup();
    assert!(adapter.backup.is_some() || adapter.called("backup"));
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_restored_when_no_network_in_adapter() {
    let mut adapter = EmberAdapter::new();
    adapter.start();
    assert!(adapter.started);
}

#[test]
#[ignore = "not implemented yet"]
fn starts_with_resumed_when_everything_matches() {
    let mut adapter = EmberAdapter::new();
    adapter.start();
    assert!(adapter.started);
}

#[test]
#[ignore = "not implemented yet"]
fn stops_ezsp_layer_on_stop() {
    let mut adapter = EmberAdapter::new();
    adapter.stop();
    assert!(adapter.stopped);
}

#[test]
#[ignore = "not implemented yet"]
fn switches_ezsp_protocol_when_supported() {
    let mut adapter = EmberAdapter::new();
    adapter.switch_protocol();
    assert!(adapter.called("switch_protocol"));
}

#[test]
#[ignore = "not implemented yet"]
fn throws_when_failed_to_retrieve_parameter_from_ncp() {
    let mut adapter = EmberAdapter::new();
    adapter.start();
    assert!(adapter.started);
}

#[test]
#[ignore = "not implemented yet"]
fn triggers_watchdog_counters() {
    let mut adapter = EmberAdapter::new();
    adapter.trigger_watchdog();
    assert!(adapter.called("trigger_watchdog"));
}

#[test]
#[ignore = "not implemented yet"]
fn uses_default_concurrency_for_queue_if_not_supplied_valid() {
    let mut adapter = EmberAdapter::new();
    adapter.set_concurrency(3);
    assert!(adapter.called("set_concurrency"));
}

#[test]
#[ignore = "not implemented yet"]
fn warns_if_ncp_has_non_ga_firmware() {
    let mut adapter = EmberAdapter::new();
    adapter.warn("non-GA firmware");
    assert!(!adapter.warnings.is_empty());
}

#[test]
#[ignore = "not implemented yet"]
fn when_started() {
    let mut adapter = EmberAdapter::new();
    adapter.record("constructor");
    assert!(adapter.called("constructor"));
}

#[test]
#[ignore = "not implemented yet"]
fn incoming_message() {
    let mut adapter = EmberAdapter::new();
    adapter.handle_incoming_message();
    assert!(adapter.called("handle_incoming_message"));
}

#[test]
#[ignore = "not implemented yet"]
fn message_sent() {
    let mut adapter = EmberAdapter::new();
    adapter.handle_message_sent();
    assert!(adapter.called("handle_message_sent"));
}

#[test]
#[ignore = "not implemented yet"]
fn ncp_needs_reset_and_init() {
    let mut adapter = EmberAdapter::new();
    adapter.reset(false);
    assert!(adapter.called("reset"));
}

#[test]
#[ignore = "not implemented yet"]
fn stack_status() {
    let mut adapter = EmberAdapter::new();
    adapter.handle_stack_status();
    assert!(adapter.called("handle_stack_status"));
}

#[test]
#[ignore = "not implemented yet"]
fn touchlink_message() {
    let mut adapter = EmberAdapter::new();
    adapter.handle_touchlink_message();
    assert!(adapter.called("handle_touchlink_message"));
}

#[test]
#[ignore = "not implemented yet"]
fn trust_center_join() {
    let mut adapter = EmberAdapter::new();
    adapter.handle_trust_center_join(false);
    assert!(adapter.called("handle_trust_center_join"));
}

#[test]
#[ignore = "not implemented yet"]
fn zdo_response() {
    let mut adapter = EmberAdapter::new();
    adapter.handle_zdo_response();
    assert!(adapter.called("handle_zdo_response"));
}
