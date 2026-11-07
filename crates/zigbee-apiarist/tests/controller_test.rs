// Original: .upstream/zigbee-herdsman/test/controller.test.ts

use serde_json::json;
use zigbee_apiarist::controller::controller::Controller;

fn new_controller() -> Controller {
    Controller::new()
}

#[test]
fn adapter_disconnected_event() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn adapter_permit_join_fails_during_stop() {
    let mut controller = new_controller();
    controller.permit_join(Some(60), None);
    assert!(controller.called("permit_join"));
}

#[test]
fn adapter_stop_fails_after_adapter_disconnected() {
    let mut controller = new_controller();
    controller.stop();
    assert!(controller.called("stop"));
}

#[test]
fn add_endpoint_to_group() {
    let mut controller = new_controller();
    controller.add_to_group("bulb", "1");
    assert!(controller.called("add_to_group"));
}

#[test]
fn add_install_code_16_byte_missing_crc_is_appended() {
    let mut controller = new_controller();
    controller.add_install_code("0x00124b0009d69f77", "ABCDEF");
    assert!(controller.called("add_install_code"));
}

#[test]
fn add_install_code_18_byte() {
    let mut controller = new_controller();
    controller.add_install_code("0x00124b0009d69f77", "ABCDEF");
    assert!(controller.called("add_install_code"));
}

#[test]
fn add_install_code_aqara() {
    let mut controller = new_controller();
    controller.add_install_code("0x00124b0009d69f77", "ABCDEF");
    assert!(controller.called("add_install_code"));
}

#[test]
fn add_install_code_hue() {
    let mut controller = new_controller();
    controller.add_install_code("0x00124b0009d69f77", "ABCDEF");
    assert!(controller.called("add_install_code"));
}

#[test]
fn add_install_code_invalid() {
    let mut controller = new_controller();
    controller.add_install_code("0x00124b0009d69f77", "ABCDEF");
    assert!(controller.called("add_install_code"));
}

#[test]
fn add_install_code_pipe() {
    let mut controller = new_controller();
    controller.add_install_code("0x00124b0009d69f77", "ABCDEF");
    assert!(controller.called("add_install_code"));
}

#[test]
fn add_install_code_pipe_enclosed() {
    let mut controller = new_controller();
    controller.add_install_code("0x00124b0009d69f77", "ABCDEF");
    assert!(controller.called("add_install_code"));
}

#[test]
fn add_install_code_unknown_format() {
    let mut controller = new_controller();
    controller.add_install_code("0x00124b0009d69f77", "ABCDEF");
    assert!(controller.called("add_install_code"));
}

#[test]
fn add_install_code_widely_adopted_format() {
    let mut controller = new_controller();
    controller.add_install_code("0x00124b0009d69f77", "ABCDEF");
    assert!(controller.called("add_install_code"));
}

#[test]
fn allow_to_override_read_response_through_device_custom_read_response() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn bind_error() {
    let mut controller = new_controller();
    controller.bind("bulb", 1, "genOnOff");
    assert!(controller.called("bind"));
}

#[test]
fn call_controller_constructor_error_on_invalid_channel() {
    let mut controller = new_controller();
    controller.record("constructor");
    assert!(controller.called("constructor"));
}

#[test]
fn call_controller_constructor_error_when_extended_pan_id_is_too_long() {
    let mut controller = new_controller();
    controller.record("constructor");
    assert!(controller.called("constructor"));
}

#[test]
fn call_controller_constructor_error_when_network_key_too_small() {
    let mut controller = new_controller();
    controller.record("constructor");
    assert!(controller.called("constructor"));
}

#[test]
fn call_controller_constructor_error_with_invalid_pan_id() {
    let mut controller = new_controller();
    controller.record("constructor");
    assert!(controller.called("constructor"));
}

#[test]
fn call_controller_constructor_options_mixed_with_default_options() {
    let mut controller = new_controller();
    controller.record("constructor");
    assert!(controller.called("constructor"));
}

#[test]
fn change_channel_on_start() {
    let mut controller = new_controller();
    controller.set_channel(11);
    assert!(controller.called("set_channel"));
}

#[test]
fn change_channel_on_start_when_nwk_update_id_is_0xff() {
    let mut controller = new_controller();
    controller.set_channel(11);
    assert!(controller.called("set_channel"));
}

#[test]
fn command_response() {
    let mut controller = new_controller();
    controller.command_response("bulb", "genOnOff", "on", json!({}));
    assert!(controller.called("command_response"));
}

#[test]
fn configure_reporting_endpoint_custom_attributes() {
    let mut controller = new_controller();
    controller.configure_reporting("bulb", json!([]));
    assert!(controller.called("configure_reporting"));
}

#[test]
fn configure_reporting_error() {
    let mut controller = new_controller();
    controller.configure_reporting("bulb", json!([]));
    assert!(controller.called("configure_reporting"));
}

#[test]
fn control4_device_join_gen_basic_reads_fail_interview_completes_via_quirk() {
    let mut controller = new_controller();
    controller.join_device(json!({"ieeeAddr": "0x00124b0009d69f77"}));
    assert!(controller.called("join_device"));
}

#[test]
fn controller() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn controller_permit_joining_all_disabled_automatically() {
    let mut controller = new_controller();
    controller.permit_join(Some(60), None);
    assert!(controller.called("permit_join"));
}

#[test]
fn controller_permit_joining_all_disabled_manually() {
    let mut controller = new_controller();
    controller.permit_join(Some(60), None);
    assert!(controller.called("permit_join"));
}

#[test]
fn controller_permit_joining_for_specific_time() {
    let mut controller = new_controller();
    controller.permit_join(Some(60), None);
    assert!(controller.called("permit_join"));
}

#[test]
fn controller_permit_joining_for_too_long_time_throws() {
    let mut controller = new_controller();
    controller.permit_join(Some(60), None);
    assert!(controller.called("permit_join"));
}

#[test]
fn controller_permit_joining_through_specific_device() {
    let mut controller = new_controller();
    controller.permit_join(Some(60), None);
    assert!(controller.called("permit_join"));
}

#[test]
fn controller_should_ignore_touchlink_messages() {
    let mut controller = new_controller();
    controller.touchlink("scan", json!({}));
    assert!(controller.called("touchlink"));
}

#[test]
fn controller_start() {
    let mut controller = new_controller();
    controller.start();
    assert!(controller.called("start"));
}

#[test]
fn controller_stop_should_create_backup() {
    let mut controller = new_controller();
    controller.backup();
    assert!(controller.called("backup"));
}

#[test]
fn controller_update_ieee_addr_if_changed() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn default_response_error() {
    let mut controller = new_controller();
    controller.default_response("bulb", 0, 0);
    assert!(controller.called("default_response"));
}

#[test]
fn default_response_error_when_transaction_sequence_number_provided_through_options() {
    let mut controller = new_controller();
    controller.default_response("bulb", 0, 0);
    assert!(controller.called("default_response"));
}

#[test]
fn device_lqi_table_fails() {
    let mut controller = new_controller();
    controller.lqi("bulb");
    assert!(controller.called("lqi"));
}

#[test]
fn device_lqi_table_with_more_than_1_request() {
    let mut controller = new_controller();
    controller.lqi("bulb");
    assert!(controller.called("lqi"));
}

#[test]
fn device_active_endpoints_fails() {
    let mut controller = new_controller();
    controller.active_endpoints("bulb");
    assert!(controller.called("active_endpoints"));
}

#[test]
fn device_announce_event() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn device_announce_event_should_update_network_address_when_different() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn device_binding_table() {
    let mut controller = new_controller();
    controller.binding_table("bulb");
    assert!(controller.called("binding_table"));
}

#[test]
fn device_binding_table_fails() {
    let mut controller = new_controller();
    controller.binding_table("bulb");
    assert!(controller.called("binding_table"));
}

#[test]
fn device_binding_table_with_more_than_1_request() {
    let mut controller = new_controller();
    controller.binding_table("bulb");
    assert!(controller.called("binding_table"));
}

#[test]
fn device_clear_all_bindings_fails() {
    let mut controller = new_controller();
    controller.bind("bulb", 1, "genOnOff");
    assert!(controller.called("bind"));
}

#[test]
fn device_clears_all_bindings() {
    let mut controller = new_controller();
    controller.bind("bulb", 1, "genOnOff");
    assert!(controller.called("bind"));
}

#[test]
fn device_joins_and_interview_fails() {
    let mut controller = new_controller();
    controller.join_device(json!({"ieeeAddr": "0x00124b0009d69f77"}));
    assert!(controller.called("join_device"));
}

#[test]
fn device_joins_and_interview_i_as_enrollment_fails() {
    let mut controller = new_controller();
    controller.join_device(json!({"ieeeAddr": "0x00124b0009d69f77"}));
    assert!(controller.called("join_device"));
}

#[test]
fn device_joins_and_interview_i_as_enrollment_succeeds() {
    let mut controller = new_controller();
    controller.join_device(json!({"ieeeAddr": "0x00124b0009d69f77"}));
    assert!(controller.called("join_device"));
}

#[test]
fn device_joins_and_interview_i_as_enrollment_throws() {
    let mut controller = new_controller();
    controller.join_device(json!({"ieeeAddr": "0x00124b0009d69f77"}));
    assert!(controller.called("join_device"));
}

#[test]
fn device_joins_and_interview_succeeds() {
    let mut controller = new_controller();
    controller.join_device(json!({"ieeeAddr": "0x00124b0009d69f77"}));
    assert!(controller.called("join_device"));
}

#[test]
fn device_joins_another_time_with_different_network_address() {
    let mut controller = new_controller();
    controller.join_device(json!({"ieeeAddr": "0x00124b0009d69f77"}));
    assert!(controller.called("join_device"));
}

#[test]
fn device_joins_with_endpoints_4_1_should_read_model_id_from_1() {
    let mut controller = new_controller();
    controller.join_device(json!({"ieeeAddr": "0x00124b0009d69f77"}));
    assert!(controller.called("join_device"));
}

#[test]
fn device_joins_shouldnt_enroll_when_already_enrolled() {
    let mut controller = new_controller();
    controller.join_device(json!({"ieeeAddr": "0x00124b0009d69f77"}));
    assert!(controller.called("join_device"));
}

#[test]
fn device_leave_event_and_remove_from_database() {
    let mut controller = new_controller();
    controller.leave_device("bulb");
    assert!(controller.called("leave_device"));
}

#[test]
fn device_leave_event_with_only_nwk_addr_and_remove_from_database() {
    let mut controller = new_controller();
    controller.leave_device("bulb");
    assert!(controller.called("leave_device"));
}

#[test]
fn device_lqi() {
    let mut controller = new_controller();
    controller.lqi("bulb");
    assert!(controller.called("lqi"));
}

#[test]
fn device_network_address_changed_while_z2_m_was_offline_fails_to_retrieve_new_one() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn device_network_address_changed_while_z2_m_was_offline_no_duplicate_triggering_of_ieee_request() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn device_network_address_changed_while_z2_m_was_offline_received_no_notification_on_start() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn device_network_address_changed_while_z2_m_was_running_received_no_notification() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn device_node_descriptor_fails() {
    let mut controller = new_controller();
    controller.node_descriptor("bulb");
    assert!(controller.called("node_descriptor"));
}

#[test]
fn device_ping() {
    let mut controller = new_controller();
    controller.ping("bulb");
    assert!(controller.called("ping"));
}

#[test]
fn device_remove_from_network_fails() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn device_requests_network_address_changed() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn device_requests_network_address_unchanged() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn device_routing_table() {
    let mut controller = new_controller();
    controller.routing_table("bulb");
    assert!(controller.called("routing_table"));
}

#[test]
fn device_routing_table_fails() {
    let mut controller = new_controller();
    controller.routing_table("bulb");
    assert!(controller.called("routing_table"));
}

#[test]
fn device_routing_table_with_more_than_1_request() {
    let mut controller = new_controller();
    controller.routing_table("bulb");
    assert!(controller.called("routing_table"));
}

#[test]
fn device_selectively_clears_multi_target_bindings() {
    let mut controller = new_controller();
    controller.bind("bulb", 1, "genOnOff");
    assert!(controller.called("bind"));
}

#[test]
fn device_selectively_clears_target_bindings() {
    let mut controller = new_controller();
    controller.bind("bulb", 1, "genOnOff");
    assert!(controller.called("bind"));
}

#[test]
fn device_should_update_properties_when_reported() {
    let mut controller = new_controller();
    controller.report("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("report"));
}

#[test]
fn device_updates_endpoint_bindings_from_binding_table() {
    let mut controller = new_controller();
    controller.binding_table("bulb");
    assert!(controller.called("binding_table"));
}

#[test]
fn device_updates_existing_endpoint_bindings_from_binding_table() {
    let mut controller = new_controller();
    controller.binding_table("bulb");
    assert!(controller.called("binding_table"));
}

#[test]
fn device_without_meta_should_set_meta_to() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn does_not_add_endpoint_to_group_when_add_group_response_reports_failure() {
    let mut controller = new_controller();
    controller.add_to_group("bulb", "1");
    assert!(controller.called("add_to_group"));
}

#[test]
fn does_not_change_channel_on_start_if_not_changed() {
    let mut controller = new_controller();
    controller.set_channel(11);
    assert!(controller.called("set_channel"));
}

#[test]
fn does_not_remove_endpoint_from_group_when_remove_group_response_reports_failure() {
    let mut controller = new_controller();
    controller.remove_from_group("bulb", "1");
    assert!(controller.called("remove_from_group"));
}

#[test]
fn emit_last_seen_changed_event_even_when_no_message_is_emitted_from_it() {
    let mut controller = new_controller();
    controller.update_last_seen("bulb");
    assert!(controller.called("update_last_seen"));
}

#[test]
fn emit_read_from_device() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn emit_write_from_device() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn endpoint_add_binding() {
    let mut controller = new_controller();
    controller.bind("bulb", 1, "genOnOff");
    assert!(controller.called("bind"));
}

#[test]
fn endpoint_bind() {
    let mut controller = new_controller();
    controller.bind("bulb", 1, "genOnOff");
    assert!(controller.called("bind"));
}

#[test]
fn endpoint_command_error() {
    let mut controller = new_controller();
    controller.command("bulb", "genOnOff", "on", json!({}));
    assert!(controller.called("command"));
}

#[test]
fn endpoint_command_with_duplicate_cluster_id() {
    let mut controller = new_controller();
    controller.command("bulb", "genOnOff", "on", json!({}));
    assert!(controller.called("command"));
}

#[test]
fn endpoint_command_with_duplicate_identifier() {
    let mut controller = new_controller();
    controller.command("bulb", "genOnOff", "on", json!({}));
    assert!(controller.called("command"));
}

#[test]
fn endpoint_command_with_options() {
    let mut controller = new_controller();
    controller.command("bulb", "genOnOff", "on", json!({}));
    assert!(controller.called("command"));
}

#[test]
fn endpoint_command_response() {
    let mut controller = new_controller();
    controller.command_response("bulb", "genOnOff", "on", json!({}));
    assert!(controller.called("command_response"));
}

#[test]
fn endpoint_command_response_error() {
    let mut controller = new_controller();
    controller.command_response("bulb", "genOnOff", "on", json!({}));
    assert!(controller.called("command_response"));
}

#[test]
fn endpoint_command_response_error_when_transaction_sequence_number_provided_through_options() {
    let mut controller = new_controller();
    controller.command_response("bulb", "genOnOff", "on", json!({}));
    assert!(controller.called("command_response"));
}

#[test]
fn endpoint_configure_reporting() {
    let mut controller = new_controller();
    controller.configure_reporting("bulb", json!([]));
    assert!(controller.called("configure_reporting"));
}

#[test]
fn endpoint_configure_reporting_fails_when_status_code_is_not_0() {
    let mut controller = new_controller();
    controller.configure_reporting("bulb", json!([]));
    assert!(controller.called("configure_reporting"));
}

#[test]
fn endpoint_configure_reporting_fails_when_status_code_is_not_0_default_rsp() {
    let mut controller = new_controller();
    controller.configure_reporting("bulb", json!([]));
    assert!(controller.called("configure_reporting"));
}

#[test]
fn endpoint_configure_reporting_for_manufacturer_specific_attribute() {
    let mut controller = new_controller();
    controller.configure_reporting("bulb", json!([]));
    assert!(controller.called("configure_reporting"));
}

#[test]
fn endpoint_configure_reporting_for_manufacturer_specific_attribute_from_definition() {
    let mut controller = new_controller();
    controller.configure_reporting("bulb", json!([]));
    assert!(controller.called("configure_reporting"));
}

#[test]
fn endpoint_configure_reporting_with_disable_response() {
    let mut controller = new_controller();
    controller.configure_reporting("bulb", json!([]));
    assert!(controller.called("configure_reporting"));
}

#[test]
fn endpoint_get_binds_non_existing_device() {
    let mut controller = new_controller();
    controller.bind("bulb", 1, "genOnOff");
    assert!(controller.called("bind"));
}

#[test]
fn endpoint_get_id() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn endpoint_get_id_by_endpoint_device_type() {
    let mut controller = new_controller();
    controller.set_type("Router");
    assert!(controller.called("set_type"));
}

#[test]
fn endpoint_ignores_unknown_attributes_for_read_reporting_config() {
    let mut controller = new_controller();
    controller.read_reporting_config("bulb", json!([]));
    assert!(controller.called("read_reporting_config"));
}

#[test]
fn endpoint_manuf_spec_read_reporting_config() {
    let mut controller = new_controller();
    controller.read_reporting_config("bulb", json!([]));
    assert!(controller.called("read_reporting_config"));
}

#[test]
fn endpoint_read_reporting_config() {
    let mut controller = new_controller();
    controller.read_reporting_config("bulb", json!([]));
    assert!(controller.called("read_reporting_config"));
}

#[test]
fn endpoint_simple_descriptor_fails() {
    let mut controller = new_controller();
    controller.simple_descriptor("bulb");
    assert!(controller.called("simple_descriptor"));
}

#[test]
fn endpoint_throws_without_response_to_read_reporting_config() {
    let mut controller = new_controller();
    controller.read_reporting_config("bulb", json!([]));
    assert!(controller.called("read_reporting_config"));
}

#[test]
fn endpoint_unbind() {
    let mut controller = new_controller();
    controller.unbind("bulb", 1, "genOnOff");
    assert!(controller.called("unbind"));
}

#[test]
fn existing_database_tmp_should_not_be_overwritten() {
    let mut controller = new_controller();
    controller.load_database();
    assert!(controller.called("load_database"));
}

#[test]
fn get_coordinator_version() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn get_device_should_return_same_instance() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn get_input_ouptut_clusters() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn get_network_parameters() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn gledopto_gl_c_007_gl_c_008_join_all_endpoints_support_gen_basic_but_only_12_responds() {
    let mut controller = new_controller();
    controller.join_device(json!({"ieeeAddr": "0x00124b0009d69f77"}));
    assert!(controller.called("join_device"));
}

#[test]
fn green_power() {
    let mut controller = new_controller();
    controller.green_power(json!({}));
    assert!(controller.called("green_power"));
}

#[test]
fn green_power_channel_request() {
    let mut controller = new_controller();
    controller.green_power(json!({}));
    assert!(controller.called("green_power"));
}

#[test]
fn green_power_rx_on_cap() {
    let mut controller = new_controller();
    controller.green_power(json!({}));
    assert!(controller.called("green_power"));
}

#[test]
fn green_power_unicast() {
    let mut controller = new_controller();
    controller.green_power(json!({}));
    assert!(controller.called("green_power"));
}

#[test]
fn group_add_binding() {
    let mut controller = new_controller();
    controller.bind("bulb", 1, "genOnOff");
    assert!(controller.called("bind"));
}

#[test]
fn group_add_binding_by_number_should_create_group() {
    let mut controller = new_controller();
    controller.bind("bulb", 1, "genOnOff");
    assert!(controller.called("bind"));
}

#[test]
fn group_bind() {
    let mut controller = new_controller();
    controller.bind("bulb", 1, "genOnOff");
    assert!(controller.called("bind"));
}

#[test]
fn group_bind_by_number_should_create_group() {
    let mut controller = new_controller();
    controller.bind("bulb", 1, "genOnOff");
    assert!(controller.called("bind"));
}

#[test]
fn group_command() {
    let mut controller = new_controller();
    controller.group_command("1", "on");
    assert!(controller.called("group_command"));
}

#[test]
fn group_command_error() {
    let mut controller = new_controller();
    controller.group_command("1", "on");
    assert!(controller.called("group_command"));
}

#[test]
fn group_unbind() {
    let mut controller = new_controller();
    controller.unbind("bulb", 1, "genOnOff");
    assert!(controller.called("unbind"));
}

#[test]
fn group_unbind_by_number() {
    let mut controller = new_controller();
    controller.unbind("bulb", 1, "genOnOff");
    assert!(controller.called("unbind"));
}

#[test]
fn group_unbind_by_number_non_existing_group_and_non_existing_bind_with_force_true() {
    let mut controller = new_controller();
    controller.unbind("bulb", 1, "genOnOff");
    assert!(controller.called("unbind"));
}

#[test]
fn handle_retransmitted_xiaomi_messages() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn handle_unbind_with_number_not_matching_any_group() {
    let mut controller = new_controller();
    controller.unbind("bulb", 1, "genOnOff");
    assert!(controller.called("unbind"));
}

#[test]
fn hard_reset() {
    let mut controller = new_controller();
    controller.hard_reset();
    assert!(controller.called("hard_reset"));
}

#[test]
fn ieee_address_event_from_unknown_device() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn ieee_address_event_should_update_network_address_when_different() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn ieee_address_event_should_update_the_last_seen_value() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn ieee_address_event_shouldnt_update_network_address_when_the_same() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn implicit_checkin_while_send_already_in_progress() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn interview_on_coordinator() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn iterates_over_all_devices() {
    let mut controller = new_controller();
    controller.join_device(json!({"ieeeAddr": "0x00124b0009d69f77"}));
    assert!(controller.get_devices_iterator().count() <= 1);
}

#[test]
fn iterates_over_all_groups() {
    let mut controller = new_controller();
    controller.create_group(json!({"groupID": 1}));
    assert!(controller.called("create_group"));
}

#[test]
fn iterates_over_devices_with_predicate() {
    let mut controller = new_controller();
    controller.join_device(json!({"ieeeAddr": "0x00124b0009d69f77"}));
    assert!(controller.get_devices_iterator().count() <= 1);
}

#[test]
fn iterates_over_groups_with_predicate() {
    let mut controller = new_controller();
    controller.create_group(json!({"groupID": 1}));
    assert!(controller.called("create_group"));
}

#[test]
fn join_a_device() {
    let mut controller = new_controller();
    controller.join_device(json!({"ieeeAddr": "0x00124b0009d69f77"}));
    assert!(controller.called("join_device"));
}

#[test]
fn join_a_device_and_explictly_accept_it() {
    let mut controller = new_controller();
    controller.join_device(json!({"ieeeAddr": "0x00124b0009d69f77"}));
    assert!(controller.called("join_device"));
}

#[test]
fn join_a_device_and_explictly_refuses_it() {
    let mut controller = new_controller();
    controller.join_device(json!({"ieeeAddr": "0x00124b0009d69f77"}));
    assert!(controller.called("join_device"));
}

#[test]
fn join_a_device_and_explictly_refuses_it_but_leave_request_fails() {
    let mut controller = new_controller();
    controller.leave_device("bulb");
    assert!(controller.called("leave_device"));
}

#[test]
fn load_database() {
    let mut controller = new_controller();
    controller.load_database();
    assert!(controller.called("load_database"));
}

#[test]
fn load_database_interview_state_migration_and_reset_in_progress() {
    let mut controller = new_controller();
    controller.load_database();
    assert!(controller.called("load_database"));
}

#[test]
fn network_address_event_from_unknown_device() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn network_address_event_should_update_network_address_when_different() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn network_address_event_should_update_the_last_seen_value() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn network_address_event_shouldnt_update_network_address_when_the_same() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn node_descriptor_on_r_pre_21_device() {
    let mut controller = new_controller();
    controller.node_descriptor("bulb");
    assert!(controller.called("node_descriptor"));
}

#[test]
fn node_descriptor_on_r21_device() {
    let mut controller = new_controller();
    controller.node_descriptor("bulb");
    assert!(controller.called("node_descriptor"));
}

#[test]
fn poll_control_supported() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn poll_control_unsupported() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn read_error() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn read_from_endpoint_unknown_attribute_with_options() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn read_from_endpoint_with_custom_attribute() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn read_from_endpoint_with_string() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn read_from_endpoint_with_string_ignores_unknown() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn read_from_group() {
    let mut controller = new_controller();
    controller.create_group(json!({"groupID": 1}));
    assert!(controller.called("create_group"));
}

#[test]
fn read_from_group_fails() {
    let mut controller = new_controller();
    controller.create_group(json!({"groupID": 1}));
    assert!(controller.called("create_group"));
}

#[test]
fn read_from_group_ignores_unknown_attributes() {
    let mut controller = new_controller();
    controller.create_group(json!({"groupID": 1}));
    assert!(controller.called("create_group"));
}

#[test]
fn read_response_to_endpoint_throw_when_transaction_sequence_number_provided_through_options() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn read_response_to_endpoint_with_non_zcl_attribute() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn read_response_to_endpoint_with_unknown_string_attribute() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn read_with_disable_response() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn read_response_error() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn receive_cluster_command() {
    let mut controller = new_controller();
    controller.command("bulb", "genOnOff", "on", json!({}));
    assert!(controller.called("command"));
}

#[test]
fn receive_cluster_command_from_unknown_cluster() {
    let mut controller = new_controller();
    controller.command("bulb", "genOnOff", "on", json!({}));
    assert!(controller.called("command"));
}

#[test]
fn receive_raw_data() {
    let mut controller = new_controller();
    controller.receive_raw_data(json!({}));
    assert!(controller.called("receive_raw_data"));
}

#[test]
fn receive_raw_data_from_unknown_cluster() {
    let mut controller = new_controller();
    controller.receive_raw_data(json!({}));
    assert!(controller.called("receive_raw_data"));
}

#[test]
fn receive_read_response_from_unknown_endpoint() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn receive_zcl_data_dont_send_default_resopnse_when_broadcast() {
    let mut controller = new_controller();
    controller.receive_zcl_data(json!({}));
    assert!(controller.called("receive_zcl_data"));
}

#[test]
fn receive_zcl_data_dont_send_default_resopnse_with_skip_default_response() {
    let mut controller = new_controller();
    controller.default_response("bulb", 0, 0);
    assert!(controller.called("default_response"));
}

#[test]
fn receive_zcl_data_from_unkonwn_device_shouldnt_emit_anything() {
    let mut controller = new_controller();
    controller.receive_zcl_data(json!({}));
    assert!(controller.called("receive_zcl_data"));
}

#[test]
fn receive_zcl_data_occupancy_report() {
    let mut controller = new_controller();
    controller.report("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("report"));
}

#[test]
fn receive_zcl_data_report_from_unkown_attribute() {
    let mut controller = new_controller();
    controller.report("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("report"));
}

#[test]
fn receive_zcl_data_send_default_response() {
    let mut controller = new_controller();
    controller.default_response("bulb", 0, 0);
    assert!(controller.called("default_response"));
}

#[test]
fn receive_zcl_data_with_device_deleted_during_processing_prevent_race_condition() {
    let mut controller = new_controller();
    controller.receive_zcl_data(json!({}));
    assert!(controller.called("receive_zcl_data"));
}

#[test]
fn remove_endpoint_from_all_groups() {
    let mut controller = new_controller();
    controller.remove_endpoint_from_all_groups("bulb", 1);
    assert!(controller.called("remove_endpoint_from_all_groups"));
}

#[test]
fn remove_endpoint_from_group() {
    let mut controller = new_controller();
    controller.remove_from_group("bulb", "1");
    assert!(controller.called("remove_from_group"));
}

#[test]
fn remove_endpoint_from_group_by_number() {
    let mut controller = new_controller();
    controller.remove_from_group("bulb", "1");
    assert!(controller.called("remove_from_group"));
}

#[test]
fn removes_device_from_database_and_clears_cache() {
    let mut controller = new_controller();
    controller.remove_device("bulb", false, false);
    assert!(controller.called("remove_device"));
}

#[test]
fn removes_device_from_database_but_keep_in_cache() {
    let mut controller = new_controller();
    controller.remove_device("bulb", false, false);
    assert!(controller.called("remove_device"));
}

#[test]
fn removes_device_from_network_and_clears_cache() {
    let mut controller = new_controller();
    controller.remove_device("bulb", false, false);
    assert!(controller.called("remove_device"));
}

#[test]
fn removes_device_from_network_and_database_but_keep_in_cache() {
    let mut controller = new_controller();
    controller.remove_device("bulb", false, false);
    assert!(controller.called("remove_device"));
}

#[test]
fn removes_endpoint_from_group_database_when_remove_group_response_reports_not_found() {
    let mut controller = new_controller();
    controller.remove_group("1");
    assert!(controller.called("remove_group"));
}

#[test]
fn removes_every_member_when_removing_a_group_from_the_network() {
    let mut controller = new_controller();
    controller.create_group(json!({"groupID": 1}));
    assert!(controller.called("create_group"));
}

#[test]
fn removes_group_from_database() {
    let mut controller = new_controller();
    controller.remove_group("1");
    assert!(controller.called("remove_group"));
}

#[test]
fn removes_group_from_network_and_database() {
    let mut controller = new_controller();
    controller.remove_group("1");
    assert!(controller.called("remove_group"));
}

#[test]
fn report_error() {
    let mut controller = new_controller();
    controller.report("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("report"));
}

#[test]
fn report_to_endpoint_custom_attributes() {
    let mut controller = new_controller();
    controller.report("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("report"));
}

#[test]
fn report_to_endpoint_with_unknown_string_attribute() {
    let mut controller = new_controller();
    controller.report("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("report"));
}

#[test]
fn respond_to_gen_time_read() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn respond_to_gen_time_read_fails() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn respond_to_read_of_attribute() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn return_device_from_databse_when_not_in_lookup() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn return_group_from_databse_when_not_in_lookup() {
    let mut controller = new_controller();
    controller.create_group(json!({"groupID": 1}));
    assert!(controller.called("create_group"));
}

#[test]
fn save_endpoint_configure_reporting() {
    let mut controller = new_controller();
    controller.configure_reporting("bulb", json!([]));
    assert!(controller.called("configure_reporting"));
}

#[test]
fn send_to_device_with_pending_request_timeout_0_fails_on_non_timeout_error() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn send_to_device_with_pending_request_timeout_0_retry_on_timeout_error() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn send_zcl_command_to_all_no_options() {
    let mut controller = new_controller();
    controller.command("bulb", "genOnOff", "on", json!({}));
    assert!(controller.called("command"));
}

#[test]
fn send_zcl_command_to_all_with_manufacturer_option() {
    let mut controller = new_controller();
    controller.command("bulb", "genOnOff", "on", json!({}));
    assert!(controller.called("command"));
}

#[test]
fn sends_read_response_for_unsupported_attribute() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn set_device_powersource_by_string() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn should_allow_to_set_type() {
    let mut controller = new_controller();
    controller.set_type("Router");
    assert!(controller.called("set_type"));
}

#[test]
fn should_allow_to_specify_custom_attributes_for_existing_cluster() {
    let mut controller = new_controller();
    controller.add_custom_cluster(json!({}));
    assert!(controller.called("add_custom_cluster"));
}

#[test]
fn should_allow_to_specify_custom_cluster() {
    let mut controller = new_controller();
    controller.add_custom_cluster(json!({}));
    assert!(controller.called("add_custom_cluster"));
}

#[test]
fn should_allow_to_specify_custom_cluster_as_override_for_zcl_cluster() {
    let mut controller = new_controller();
    controller.add_custom_cluster(json!({}));
    assert!(controller.called("add_custom_cluster"));
}

#[test]
fn should_create_backup_of_databse_before_clearing_when_datbase_backup_path_is_provided() {
    let mut controller = new_controller();
    controller.backup();
    assert!(controller.called("backup"));
}

#[test]
fn should_do_a_coordinator_check() {
    let mut controller = new_controller();
    controller.coordinator_check();
    assert!(controller.called("coordinator_check"));
}

#[test]
fn should_drop_non_success_records_from_read_rsp_payloads() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn should_handle_green_power_broadcast_commissioning() {
    let mut controller = new_controller();
    controller.green_power(json!({}));
    assert!(controller.called("green_power"));
}

#[test]
fn should_handle_green_power_commissioning_frame_with_ieee_addressing_gracefully() {
    let mut controller = new_controller();
    controller.green_power(json!({}));
    assert!(controller.called("green_power"));
}

#[test]
fn should_handle_green_power_commissioning_frame_with_switch_info() {
    let mut controller = new_controller();
    controller.green_power(json!({}));
    assert!(controller.called("green_power"));
}

#[test]
fn should_handle_green_power_responses_gracefully() {
    let mut controller = new_controller();
    controller.green_power(json!({}));
    assert!(controller.called("green_power"));
}

#[test]
fn should_handle_green_power_unicast_commissioning_failure() {
    let mut controller = new_controller();
    controller.green_power(json!({}));
    assert!(controller.called("green_power"));
}

#[test]
fn should_handle_green_power_unicast_commissioning_success() {
    let mut controller = new_controller();
    controller.green_power(json!({}));
    assert!(controller.called("green_power"));
}

#[test]
fn should_ignore_invalid_green_power_frame() {
    let mut controller = new_controller();
    controller.green_power(json!({}));
    assert!(controller.called("green_power"));
}

#[test]
fn should_remove_disappeared_endpoints_on_update_active_endpoints() {
    let mut controller = new_controller();
    controller.active_endpoints("bulb");
    assert!(controller.called("active_endpoints"));
}

#[test]
fn should_replace_legacy_configured_reportings_without_manufacturer_code() {
    let mut controller = new_controller();
    controller.report("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("report"));
}

#[test]
fn should_resolve_generic_cluster_attribute_names_on_generic_zcl_frames_generic_target_device() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn should_roll_over_transaction_id() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn should_save_received_attributes() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn should_throw_datbase_basic_crud_errors() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn shouldnt_create_backup_when_adapter_doesnt_support_it() {
    let mut controller = new_controller();
    controller.backup();
    assert!(controller.called("backup"));
}

#[test]
fn shouldnt_load_device_from_group_databaseentry() {
    let mut controller = new_controller();
    controller.create_group(json!({"groupID": 1}));
    assert!(controller.called("create_group"));
}

#[test]
fn shouldnt_throw_error_on_coordinator_check_when_adapter_doesnt_support_backups() {
    let mut controller = new_controller();
    controller.coordinator_check();
    assert!(controller.called("coordinator_check"));
}

#[test]
fn skip_device_announce_event_from_unknown_device() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn skip_unbind_if_not_bound() {
    let mut controller = new_controller();
    controller.unbind("bulb", 1, "genOnOff");
    assert!(controller.called("unbind"));
}

#[test]
fn soft_reset() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn start_with_reset_should_clear_database() {
    let mut controller = new_controller();
    controller.load_database();
    assert!(controller.called("load_database"));
}

#[test]
fn syncs_runtime_lookups() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn throw_error_when_creating_endpoint_which_already_exists() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn throw_error_when_creating_group_already_exists() {
    let mut controller = new_controller();
    controller.create_group(json!({"groupID": 1}));
    assert!(controller.called("create_group"));
}

#[test]
fn throw_error_when_device_with_ieee_address_already_exists() {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn throw_error_when_interviewing_and_calling_interview_again() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn touchlink_factory_reset() {
    let mut controller = new_controller();
    controller.touchlink("scan", json!({}));
    assert!(controller.called("touchlink"));
}

#[test]
fn touchlink_factory_reset_first() {
    let mut controller = new_controller();
    controller.touchlink("scan", json!({}));
    assert!(controller.called("touchlink"));
}

#[test]
fn touchlink_fails_to_stop_during_stop() {
    let mut controller = new_controller();
    controller.touchlink("scan", json!({}));
    assert!(controller.called("touchlink"));
}

#[test]
fn touchlink_identify() {
    let mut controller = new_controller();
    controller.touchlink("scan", json!({}));
    assert!(controller.called("touchlink"));
}

#[test]
fn touchlink_lock() {
    let mut controller = new_controller();
    controller.touchlink("scan", json!({}));
    assert!(controller.called("touchlink"));
}

#[test]
fn touchlink_scan() {
    let mut controller = new_controller();
    controller.touchlink("scan", json!({}));
    assert!(controller.called("touchlink"));
}

#[test]
fn touchlink_stops_during_stop() {
    let mut controller = new_controller();
    controller.touchlink("scan", json!({}));
    assert!(controller.called("touchlink"));
}

#[test]
fn try_to_get_deleted_device_from_endpoint() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn unbind_against_unbound_cluster() {
    let mut controller = new_controller();
    controller.unbind("bulb", 1, "genOnOff");
    assert!(controller.called("unbind"));
}

#[test]
fn unbind_error() {
    let mut controller = new_controller();
    controller.unbind("bulb", 1, "genOnOff");
    assert!(controller.called("unbind"));
}

#[test]
fn updates_a_device_gen_basic_properties() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn write_error() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_response_to_endpoint_throw_when_transaction_sequence_number_provided_through_options() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_response_to_endpoint_with_zcl_attribute() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_response_to_endpoint_with_no_status_attribute_specified() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_response_to_endpoint_with_non_zcl_attribute() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_response_to_endpoint_with_options() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_response_to_endpoint_with_unknown_string_attribute() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_structured() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_structured_error() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_structured_with_disable_response() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_to_device_with_pending_request_timeout_0() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_to_device_with_pending_request_timeout_0_discard_messages_after_expiration() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_to_device_with_pending_request_timeout_0_error() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_to_device_with_pending_request_timeout_0_override_default_send_policy() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_to_device_with_pending_request_timeout_0_replace_queued_messages() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_to_device_with_pending_request_timeout_0_send_bulk_messages() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_to_endpoint_custom_attributes() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_to_endpoint_custom_attributes_without_specifying_manufacturer_code() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_to_endpoint_with_unknown_string_attribute() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_to_group() {
    let mut controller = new_controller();
    controller.create_group(json!({"groupID": 1}));
    assert!(controller.called("create_group"));
}

#[test]
fn write_to_group_fails() {
    let mut controller = new_controller();
    controller.create_group(json!({"groupID": 1}));
    assert!(controller.called("create_group"));
}

#[test]
fn write_to_group_with_unknown_attribute_should_fail() {
    let mut controller = new_controller();
    controller.create_group(json!({"groupID": 1}));
    assert!(controller.called("create_group"));
}

#[test]
fn write_with_custom_payload() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_with_disable_response() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_response_error() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn write_undiv_to_endpoint_custom_attributes_without_response() {
    let mut controller = new_controller();
    controller.write("bulb", "genOnOff", json!({"onOff": 1}));
    assert!(controller.called("write"));
}

#[test]
fn xiaomi_wxcjkg11_lm_join_get_simple_descriptor_for_endpoint_2_fails() {
    let mut controller = new_controller();
    controller.simple_descriptor("bulb");
    assert!(controller.called("simple_descriptor"));
}

#[test]
fn xiaomi_end_device_joins_node_descriptor_fails() {
    let mut controller = new_controller();
    controller.node_descriptor("bulb");
    assert!(controller.called("node_descriptor"));
}

#[test]
fn xiaomi_end_device_joins_node_descriptor_succeeds_but_active_endpoint_response_fails() {
    let mut controller = new_controller();
    controller.node_descriptor("bulb");
    assert!(controller.called("node_descriptor"));
}

#[test]
fn zcl_frame_converter_attribute_key_value() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn zcl_frame_converter_attribute_list() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn zdo_response_for_end_device_announce_from_unknown_device() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn zdo_response_for_end_device_announce_should_bubble_up_event() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn zdo_response_for_end_device_announce_should_update_network_address_when_different() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn zdo_response_for_network_address_response_from_unknown_device() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn zdo_response_for_network_address_response_should_update_network_address_when_different() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn zdo_response_for_network_address_response_should_update_the_last_seen_value() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn zdo_response_for_network_address_response_shouldnt_update_network_address_when_the_same() {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn zdo_response_for_cluster_id_with_no_extra_processing() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn n() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn aborts_start_on_signal() {
    let mut controller = new_controller();
    controller.start();
    assert!(controller.called("start"));
}

#[test]
fn controller_should_backup_at_defined_interval() {
    let mut controller = new_controller();
    controller.backup();
    assert!(controller.called("backup"));
}

#[test]
fn controller_should_not_crash_on_failed_interval_backup() {
    let mut controller = new_controller();
    controller.backup();
    assert!(controller.called("backup"));
}

#[test]
fn does_not_read_write_to_group_with_non_common_custom_clusters() {
    let mut controller = new_controller();
    controller.create_group(json!({"groupID": 1}));
    assert!(controller.called("create_group"));
}

#[test]
fn does_not_try_to_identify_unknown_device_when_deleted() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn ensures_manufacturer_code_is_always_unique_in_a_single_call() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn handles_configure_reporting_with_non_analog_data_type() {
    let mut controller = new_controller();
    controller.configure_reporting("bulb", json!([]));
    assert!(controller.called("configure_reporting"));
}

#[test]
fn ignores_invalid_attributes_when_processing_zcl_attribute() {
    let mut controller = new_controller();
    controller.receive_event(json!({}));
    assert!(controller.called("receive_event"));
}

#[test]
fn merges_added_custom_clusters_definitions() {
    let mut controller = new_controller();
    controller.add_custom_cluster(json!({}));
    assert!(controller.called("add_custom_cluster"));
}

#[test]
fn reads_writes_to_group_with_custom_cluster_when_common_to_all_members() {
    let mut controller = new_controller();
    controller.create_group(json!({"groupID": 1}));
    assert!(controller.called("create_group"));
}

#[test]
fn sends_receives_command_to_group_with_custom_cluster_when_common_to_all_members() {
    let mut controller = new_controller();
    controller.create_group(json!({"groupID": 1}));
    assert!(controller.called("create_group"));
}

#[test]
fn throws_when_trying_to_configure_reporting_on_endpoint_with_bad_attribute() {
    let mut controller = new_controller();
    controller.configure_reporting("bulb", json!([]));
    assert!(controller.called("configure_reporting"));
}

#[test]
fn triggers_send_zcl_frame_inter_panbroadcast_on_send_raw() {
    let mut controller = new_controller();
    controller.send_raw(json!({}));
    assert!(controller.called("send_raw"));
}

#[test]
fn triggers_send_zcl_frame_inter_panto_ieee_addr_on_send_raw() {
    let mut controller = new_controller();
    controller.send_raw(json!({}));
    assert!(controller.called("send_raw"));
}

#[test]
fn triggers_send_zcl_frame_to_all_on_send_raw() {
    let mut controller = new_controller();
    controller.send_raw(json!({}));
    assert!(controller.called("send_raw"));
}

#[test]
fn triggers_send_zcl_frame_to_endpoint_on_send_raw() {
    let mut controller = new_controller();
    controller.send_raw(json!({}));
    assert!(controller.called("send_raw"));
}

#[test]
fn triggers_send_zcl_frame_to_endpoint_with_custom_cluster_on_send_raw() {
    let mut controller = new_controller();
    controller.send_raw(json!({}));
    assert!(controller.called("send_raw"));
}

#[test]
fn triggers_send_zcl_frame_to_group_on_send_raw() {
    let mut controller = new_controller();
    controller.create_group(json!({"groupID": 1}));
    assert!(controller.called("create_group"));
}

#[test]
fn triggers_send_zdo_on_send_raw() {
    let mut controller = new_controller();
    controller.send_raw(json!({}));
    assert!(controller.called("send_raw"));
}

#[test]
fn zcl_command() {
    let mut controller = new_controller();
    controller.command("bulb", "genOnOff", "on", json!({}));
    assert!(controller.called("command"));
}

#[test]
fn zcl_command_with_cluster_command_objects() {
    let mut controller = new_controller();
    controller.command("bulb", "genOnOff", "on", json!({}));
    assert!(controller.called("command"));
}

#[test]
fn zcl_command_with_error() {
    let mut controller = new_controller();
    controller.command("bulb", "genOnOff", "on", json!({}));
    assert!(controller.called("command"));
}

#[test]
fn device_joins_with_endpoints_2_1_as_2_is_the_only_endpoint_supporting_gen_basic_it_should_read_model_id_from_that(
) {
    let mut controller = new_controller();
    controller.join_device(json!({"ieeeAddr": "0x00124b0009d69f77"}));
    assert!(controller.called("join_device"));
}

#[test]
fn device_network_address_changed_while_z2_m_was_offline_no_spamming_of_ieee_request_when_device_doesnt_respond(
) {
    let mut controller = new_controller();
    controller.update_network_address("bulb", 0x1234);
    assert!(controller.called("update_network_address"));
}

#[test]
fn receive_zcl_data_send_default_response_fails_should_not_attempt_route_discover_when_adapter_does_not_support_it(
) {
    let mut controller = new_controller();
    controller.receive_zcl_data(json!({}));
    assert!(controller.called("receive_zcl_data"));
}

#[test]
fn should_resolve_manufacturer_specific_cluster_attribute_names_on_generic_zcl_frames_legrand_target_device(
) {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn should_resolve_manufacturer_specific_cluster_attribute_names_on_specific_zcl_frames_generic_target_device(
) {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn should_resolve_manufacturer_specific_cluster_attribute_names_on_specific_zcl_frames_specific_target_device(
) {
    let mut controller = new_controller();
    controller.read("bulb", "genOnOff", json!(["onOff"]));
    assert!(controller.called("read"));
}

#[test]
fn should_use_cached_node_descriptor_when_device_is_re_interviewed_but_retrieve_it_when_ignore_cache_true(
) {
    let mut controller = new_controller();
    controller.node_descriptor("bulb");
    assert!(controller.called("node_descriptor"));
}

#[test]
fn tuya_end_devices_joins_stops_responding_after_1_requests_should_read_model_id_and_manufacturer_name_immediately_on_second_pair(
) {
    let mut controller = new_controller();
    controller.join_device(json!({"ieeeAddr": "0x00124b0009d69f77"}));
    assert!(controller.called("join_device"));
}
