// Original: .upstream/zigbee-herdsman-converters/test/shelly.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn shelly_1_l_gen4() {
    let mut probe = ConverterProbe::new();
    probe.run("shelly_1_l_gen4");
    assert!(probe.called("shelly_1_l_gen4"));
}

#[test]
#[ignore = "not implemented yet"]
fn shelly_2_pm_gen4_cover_mode() {
    let mut probe = ConverterProbe::new();
    probe.run("shelly_2_pm_gen4_cover_mode");
    assert!(probe.called("shelly_2_pm_gen4_cover_mode"));
}

#[test]
#[ignore = "not implemented yet"]
fn shelly_2_pm_gen4_switch_input_endpoints() {
    let mut probe = ConverterProbe::new();
    probe.run("shelly_2_pm_gen4_switch_input_endpoints");
    assert!(probe.called("shelly_2_pm_gen4_switch_input_endpoints"));
}

#[test]
#[ignore = "not implemented yet"]
fn shelly_2_pm_gen4_switch_input_events() {
    let mut probe = ConverterProbe::new();
    probe.run("shelly_2_pm_gen4_switch_input_events");
    assert!(probe.called("shelly_2_pm_gen4_switch_input_events"));
}

#[test]
#[ignore = "not implemented yet"]
fn shelly_gen4_settings_the_device_cannot_report() {
    let mut probe = ConverterProbe::new();
    probe.run("shelly_gen4_settings_the_device_cannot_report");
    assert!(probe.called("shelly_gen4_settings_the_device_cannot_report"));
}

#[test]
#[ignore = "not implemented yet"]
fn shelly_gen4_switch_definitions_carry_the_re_configure_version_bump() {
    let mut probe = ConverterProbe::new();
    probe.run("shelly_gen4_switch_definitions_carry_the_re_configure_version_bump");
    assert!(probe.called("shelly_gen4_switch_definitions_carry_the_re_configure_version_bump"));
}

#[test]
#[ignore = "not implemented yet"]
fn shelly_presence_gen4() {
    let mut probe = ConverterProbe::new();
    probe.run("shelly_presence_gen4");
    assert!(probe.called("shelly_presence_gen4"));
}

#[test]
#[ignore = "not implemented yet"]
fn shelly_rpc_hygiene() {
    let mut probe = ConverterProbe::new();
    probe.run("shelly_rpc_hygiene");
    assert!(probe.called("shelly_rpc_hygiene"));
}

#[test]
#[ignore = "not implemented yet"]
fn shelly_ws90_rain_rate() {
    let mut probe = ConverterProbe::new();
    probe.run("shelly_ws90_rain_rate");
    assert!(probe.called("shelly_ws90_rain_rate"));
}

#[test]
#[ignore = "not implemented yet"]
fn shelly_wi_fi_setup() {
    let mut probe = ConverterProbe::new();
    probe.run("shelly_wi_fi_setup");
    assert!(probe.called("shelly_wi_fi_setup"));
}

#[test]
#[ignore = "not implemented yet"]
fn a_full_set_still_writes_every_group() {
    let mut probe = ConverterProbe::new();
    probe.run("a_full_set_still_writes_every_group");
    assert!(probe.called("a_full_set_still_writes_every_group"));
}

#[test]
#[ignore = "not implemented yet"]
fn an_empty_payload_writes_nothing_not_even_the_apply() {
    let mut probe = ConverterProbe::new();
    probe.run("an_empty_payload_writes_nothing_not_even_the_apply");
    assert!(probe.called("an_empty_payload_writes_nothing_not_even_the_apply"));
}

#[test]
#[ignore = "not implemented yet"]
fn clamps_out_of_range_delays_to_the_0_3600_second_bounds() {
    let mut probe = ConverterProbe::new();
    probe.run("clamps_out_of_range_delays_to_the_0_3600_second_bounds");
    assert!(probe.called("clamps_out_of_range_delays_to_the_0_3600_second_bounds"));
}

#[test]
#[ignore = "not implemented yet"]
fn configures_both_input_endpoints() {
    let mut probe = ConverterProbe::new();
    probe.run("configures_both_input_endpoints");
    assert!(probe.called("configures_both_input_endpoints"));
}

#[test]
#[ignore = "not implemented yet"]
fn configures_occupancy_reporting_on_every_tracked_person_endpoint() {
    let mut probe = ConverterProbe::new();
    probe.run("configures_occupancy_reporting_on_every_tracked_person_endpoint");
    assert!(probe.called("configures_occupancy_reporting_on_every_tracked_person_endpoint"));
}

#[test]
#[ignore = "not implemented yet"]
fn configures_toggle_actions_for_an_opt_in_momentary_input_binding() {
    let mut probe = ConverterProbe::new();
    probe.run("configures_toggle_actions_for_an_opt_in_momentary_input_binding");
    assert!(probe.called("configures_toggle_actions_for_an_opt_in_momentary_input_binding"));
}

#[test]
#[ignore = "not implemented yet"]
fn disables_toggle_actions_for_an_opt_in_momentary_input_binding() {
    let mut probe = ConverterProbe::new();
    probe.run("disables_toggle_actions_for_an_opt_in_momentary_input_binding");
    assert!(probe.called("disables_toggle_actions_for_an_opt_in_momentary_input_binding"));
}

#[test]
#[ignore = "not implemented yet"]
fn discards_the_non_value_markers_instead_of_publishing_them_as_measurements() {
    let mut probe = ConverterProbe::new();
    probe.run("discards_the_non_value_markers_instead_of_publishing_them_as_measurements");
    assert!(
        probe.called("discards_the_non_value_markers_instead_of_publishing_them_as_measurements")
    );
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_announce_a_read_for_power_strip_settings_that_have_no_read_converter() {
    let mut probe = ConverterProbe::new();
    probe.run("does_not_announce_a_read_for_power_strip_settings_that_have_no_read_converter");
    assert!(probe
        .called("does_not_announce_a_read_for_power_strip_settings_that_have_no_read_converter"));
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_expose_switch_input_controls_when_the_device_has_no_switch_input_endpoints() {
    let mut probe = ConverterProbe::new();
    probe
        .run("does_not_expose_switch_input_controls_when_the_device_has_no_switch_input_endpoints");
    assert!(probe.called(
        "does_not_expose_switch_input_controls_when_the_device_has_no_switch_input_endpoints"
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_fail_a_get_when_both_shelly_wi_fi_readback_paths_are_unavailable() {
    let mut probe = ConverterProbe::new();
    probe.run("does_not_fail_a_get_when_both_shelly_wi_fi_readback_paths_are_unavailable");
    assert!(
        probe.called("does_not_fail_a_get_when_both_shelly_wi_fi_readback_paths_are_unavailable")
    );
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_read_the_sensor_back_after_writing_a_group() {
    let mut probe = ConverterProbe::new();
    probe.run("does_not_read_the_sensor_back_after_writing_a_group");
    assert!(probe.called("does_not_read_the_sensor_back_after_writing_a_group"));
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_read_the_zone_back_after_writing_a_delay() {
    let mut probe = ConverterProbe::new();
    probe.run("does_not_read_the_zone_back_after_writing_a_delay");
    assert!(probe.called("does_not_read_the_zone_back_after_writing_a_delay"));
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_serialize_rpc_writes_of_different_devices_against_each_other() {
    let mut probe = ConverterProbe::new();
    probe.run("does_not_serialize_rpc_writes_of_different_devices_against_each_other");
    assert!(probe.called("does_not_serialize_rpc_writes_of_different_devices_against_each_other"));
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_an_eco_mode_switch() {
    let mut probe = ConverterProbe::new();
    probe.run("exposes_an_eco_mode_switch");
    assert!(probe.called("exposes_an_eco_mode_switch"));
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_an_occupancy_endpoint_for_every_person_the_device_can_track() {
    let mut probe = ConverterProbe::new();
    probe.run("exposes_an_occupancy_endpoint_for_every_person_the_device_can_track");
    assert!(probe.called("exposes_an_occupancy_endpoint_for_every_person_the_device_can_track"));
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_no_switch_type_on_a_1_pm_mini_that_has_no_switch_input_endpoint() {
    let mut probe = ConverterProbe::new();
    probe.run("exposes_no_switch_type_on_a_1_pm_mini_that_has_no_switch_input_endpoint");
    assert!(probe.called("exposes_no_switch_type_on_a_1_pm_mini_that_has_no_switch_input_endpoint"));
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_presence_delay_and_absence_delay_for_each_detected_zone() {
    let mut probe = ConverterProbe::new();
    probe.run("exposes_presence_delay_and_absence_delay_for_each_detected_zone");
    assert!(probe.called("exposes_presence_delay_and_absence_delay_for_each_detected_zone"));
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_switch_type_on_a_1_pm_mini_that_has_a_switch_input_endpoint() {
    let mut probe = ConverterProbe::new();
    probe.run("exposes_switch_type_on_a_1_pm_mini_that_has_a_switch_input_endpoint");
    assert!(probe.called("exposes_switch_type_on_a_1_pm_mini_that_has_a_switch_input_endpoint"));
}

#[test]
#[ignore = "not implemented yet"]
fn groups_the_sensor_settings_and_keeps_the_device() {
    let mut probe = ConverterProbe::new();
    probe.run("groups_the_sensor_settings_and_keeps_the_device");
    assert!(probe.called("groups_the_sensor_settings_and_keeps_the_device"));
}

#[test]
#[ignore = "not implemented yet"]
fn ignores_commands_from_endpoints_that_are_no_switch_input_instead_of_throwing() {
    let mut probe = ConverterProbe::new();
    probe.run("ignores_commands_from_endpoints_that_are_no_switch_input_instead_of_throwing");
    assert!(probe
        .called("ignores_commands_from_endpoints_that_are_no_switch_input_instead_of_throwing"));
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_both_switch_inputs_exposed_on_a_switch_mode_2_pm_that_has_them_wired() {
    let mut probe = ConverterProbe::new();
    probe.run("keeps_both_switch_inputs_exposed_on_a_switch_mode_2_pm_that_has_them_wired");
    assert!(
        probe.called("keeps_both_switch_inputs_exposed_on_a_switch_mode_2_pm_that_has_them_wired")
    );
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_configuring_the_remaining_endpoints_when_one_of_them_fails() {
    let mut probe = ConverterProbe::new();
    probe.run("keeps_configuring_the_remaining_endpoints_when_one_of_them_fails");
    assert!(probe.called("keeps_configuring_the_remaining_endpoints_when_one_of_them_fails"));
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_every_occupancy_endpoint_even_when_only_one_zone_is_known() {
    let mut probe = ConverterProbe::new();
    probe.run("keeps_every_occupancy_endpoint_even_when_only_one_zone_is_known");
    assert!(probe.called("keeps_every_occupancy_endpoint_even_when_only_one_zone_is_known"));
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_single_channel_input_events_working_when_the_input_is_wired() {
    let mut probe = ConverterProbe::new();
    probe.run("keeps_single_channel_input_events_working_when_the_input_is_wired");
    assert!(probe.called("keeps_single_channel_input_events_working_when_the_input_is_wired"));
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_the_cover_mode_2_pm_on_its_own_input_endpoints() {
    let mut probe = ConverterProbe::new();
    probe.run("keeps_the_cover_mode_2_pm_on_its_own_input_endpoints");
    assert!(probe.called("keeps_the_cover_mode_2_pm_on_its_own_input_endpoints"));
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_the_switch_mode_inputs_on_endpoints_3_and_4() {
    let mut probe = ConverterProbe::new();
    probe.run("keeps_the_switch_mode_inputs_on_endpoints_3_and_4");
    assert!(probe.called("keeps_the_switch_mode_inputs_on_endpoints_3_and_4"));
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_tilt_controls_visible_by_default_and_allows_explicit_opt_out() {
    let mut probe = ConverterProbe::new();
    probe.run("keeps_tilt_controls_visible_by_default_and_allows_explicit_opt_out");
    assert!(probe.called("keeps_tilt_controls_visible_by_default_and_allows_explicit_opt_out"));
}

#[test]
#[ignore = "not implemented yet"]
fn lists_the_reported_custom_sensitivity_but_refuses_to_write_it() {
    let mut probe = ConverterProbe::new();
    probe.run("lists_the_reported_custom_sensitivity_but_refuses_to_write_it");
    assert!(probe.called("lists_the_reported_custom_sensitivity_but_refuses_to_write_it"));
}

#[test]
#[ignore = "not implemented yet"]
fn makes_no_discovery_attempt_while_the_firmware_cannot_answer_reads() {
    let mut probe = ConverterProbe::new();
    probe.run("makes_no_discovery_attempt_while_the_firmware_cannot_answer_reads");
    assert!(probe.called("makes_no_discovery_attempt_while_the_firmware_cannot_answer_reads"));
}

#[test]
#[ignore = "not implemented yet"]
fn maps_both_input_endpoints_to_separate_actions_and_switch_types() {
    let mut probe = ConverterProbe::new();
    probe.run("maps_both_input_endpoints_to_separate_actions_and_switch_types");
    assert!(probe.called("maps_both_input_endpoints_to_separate_actions_and_switch_types"));
}

#[test]
#[ignore = "not implemented yet"]
fn maps_cover_input_1_endpoint_2_and_input_2_endpoint_3_to_their_own_actions() {
    let mut probe = ConverterProbe::new();
    probe.run("maps_cover_input_1_endpoint_2_and_input_2_endpoint_3_to_their_own_actions");
    assert!(
        probe.called("maps_cover_input_1_endpoint_2_and_input_2_endpoint_3_to_their_own_actions")
    );
}

#[test]
#[ignore = "not implemented yet"]
fn maps_switch_mode_endpoint_3_input_type_to_sw1() {
    let mut probe = ConverterProbe::new();
    probe.run("maps_switch_mode_endpoint_3_input_type_to_sw1");
    assert!(probe.called("maps_switch_mode_endpoint_3_input_type_to_sw1"));
}

#[test]
#[ignore = "not implemented yet"]
fn maps_the_captured_relay_and_input_endpoint_layout() {
    let mut probe = ConverterProbe::new();
    probe.run("maps_the_captured_relay_and_input_endpoint_layout");
    assert!(probe.called("maps_the_captured_relay_and_input_endpoint_layout"));
}

#[test]
#[ignore = "not implemented yet"]
fn maps_the_second_cover_switch_input_type_to_sw2() {
    let mut probe = ConverterProbe::new();
    probe.run("maps_the_second_cover_switch_input_type_to_sw2");
    assert!(probe.called("maps_the_second_cover_switch_input_type_to_sw2"));
}

#[test]
#[ignore = "not implemented yet"]
fn marks_switch_mode_write_only_and_offers_no_read() {
    let mut probe = ConverterProbe::new();
    probe.run("marks_switch_mode_write_only_and_offers_no_read");
    assert!(probe.called("marks_switch_mode_write_only_and_offers_no_read"));
}

#[test]
#[ignore = "not implemented yet"]
fn merges_several_deeply_nested_thresholds_into_one_nested_config() {
    let mut probe = ConverterProbe::new();
    probe.run("merges_several_deeply_nested_thresholds_into_one_nested_config");
    assert!(probe.called("merges_several_deeply_nested_thresholds_into_one_nested_config"));
}

#[test]
#[ignore = "not implemented yet"]
fn offers_no_rpc_backed_reads_at_all() {
    let mut probe = ConverterProbe::new();
    probe.run("offers_no_rpc_backed_reads_at_all");
    assert!(probe.called("offers_no_rpc_backed_reads_at_all"));
}

#[test]
#[ignore = "not implemented yet"]
fn offers_no_switch_input_mode_on_a_switch_mode_2_pm_whose_inputs_are_not_wired() {
    let mut probe = ConverterProbe::new();
    probe.run("offers_no_switch_input_mode_on_a_switch_mode_2_pm_whose_inputs_are_not_wired");
    assert!(probe
        .called("offers_no_switch_input_mode_on_a_switch_mode_2_pm_whose_inputs_are_not_wired"));
}

#[test]
#[ignore = "not implemented yet"]
fn only_configures_the_relay_associated_input_mode() {
    let mut probe = ConverterProbe::new();
    probe.run("only_configures_the_relay_associated_input_mode");
    assert!(probe.called("only_configures_the_relay_associated_input_mode"));
}

#[test]
#[ignore = "not implemented yet"]
fn persists_the_device_meta_at_most_once_a_minute() {
    let mut probe = ConverterProbe::new();
    probe.run("persists_the_device_meta_at_most_once_a_minute");
    assert!(probe.called("persists_the_device_meta_at_most_once_a_minute"));
}

#[test]
#[ignore = "not implemented yet"]
fn prefers_a_discovered_zone_id_over_the_main_zone_fallback() {
    let mut probe = ConverterProbe::new();
    probe.run("prefers_a_discovered_zone_id_over_the_main_zone_fallback");
    assert!(probe.called("prefers_a_discovered_zone_id_over_the_main_zone_fallback"));
}

#[test]
#[ignore = "not implemented yet"]
fn publishes_a_repeated_transaction_only_once() {
    let mut probe = ConverterProbe::new();
    probe.run("publishes_a_repeated_transaction_only_once");
    assert!(probe.called("publishes_a_repeated_transaction_only_once"));
}

#[test]
#[ignore = "not implemented yet"]
fn reads_the_wi_fi_state_through_the_setup_cluster_without_trying_rpc_first() {
    let mut probe = ConverterProbe::new();
    probe.run("reads_the_wi_fi_state_through_the_setup_cluster_without_trying_rpc_first");
    assert!(
        probe.called("reads_the_wi_fi_state_through_the_setup_cluster_without_trying_rpc_first")
    );
}

#[test]
#[ignore = "not implemented yet"]
fn recovers_the_rain_rate_after_the_cumulative_counter_resets() {
    let mut probe = ConverterProbe::new();
    probe.run("recovers_the_rain_rate_after_the_cumulative_counter_resets");
    assert!(probe.called("recovers_the_rain_rate_after_the_cumulative_counter_resets"));
}

#[test]
#[ignore = "not implemented yet"]
fn registers_no_rpc_diagnostics_converter_without_the_dev_feature() {
    let mut probe = ConverterProbe::new();
    probe.run("registers_no_rpc_diagnostics_converter_without_the_dev_feature");
    assert!(probe.called("registers_no_rpc_diagnostics_converter_without_the_dev_feature"));
}

#[test]
#[ignore = "not implemented yet"]
fn rejects_momentary_toggle_binding_setup_for_a_toggle_input() {
    let mut probe = ConverterProbe::new();
    probe.run("rejects_momentary_toggle_binding_setup_for_a_toggle_input");
    assert!(probe.called("rejects_momentary_toggle_binding_setup_for_a_toggle_input"));
}

#[test]
#[ignore = "not implemented yet"]
fn retries_a_delay_write_when_the_first_rpc_send_fails() {
    let mut probe = ConverterProbe::new();
    probe.run("retries_a_delay_write_when_the_first_rpc_send_fails");
    assert!(probe.called("retries_a_delay_write_when_the_first_rpc_send_fails"));
}

#[test]
#[ignore = "not implemented yet"]
fn setting_only_credentials_does_not_silently_disable_wi_fi() {
    let mut probe = ConverterProbe::new();
    probe.run("setting_only_credentials_does_not_silently_disable_wi_fi");
    assert!(probe.called("setting_only_credentials_does_not_silently_disable_wi_fi"));
}

#[test]
#[ignore = "not implemented yet"]
fn tells_the_user_that_power_strip_settings_cannot_be_read_back() {
    let mut probe = ConverterProbe::new();
    probe.run("tells_the_user_that_power_strip_settings_cannot_be_read_back");
    assert!(probe.called("tells_the_user_that_power_strip_settings_cannot_be_read_back"));
}

#[test]
#[ignore = "not implemented yet"]
fn toggling_only_enabled_does_not_touch_the_stored_credentials() {
    let mut probe = ConverterProbe::new();
    probe.run("toggling_only_enabled_does_not_touch_the_stored_credentials");
    assert!(probe.called("toggling_only_enabled_does_not_touch_the_stored_credentials"));
}

#[test]
#[ignore = "not implemented yet"]
fn uses_the_cached_full_shelly_wi_fi_ssid_when_the_setup_cluster_reports_a_shortened_name() {
    let mut probe = ConverterProbe::new();
    probe.run(
        "uses_the_cached_full_shelly_wi_fi_ssid_when_the_setup_cluster_reports_a_shortened_name",
    );
    assert!(probe.called(
        "uses_the_cached_full_shelly_wi_fi_ssid_when_the_setup_cluster_reports_a_shortened_name"
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn writes_presence_zone_set_config_to_the_mapped_zone_id_when_setting_a_delay() {
    let mut probe = ConverterProbe::new();
    probe.run("writes_presence_zone_set_config_to_the_mapped_zone_id_when_setting_a_delay");
    assert!(
        probe.called("writes_presence_zone_set_config_to_the_mapped_zone_id_when_setting_a_delay")
    );
}

#[test]
#[ignore = "not implemented yet"]
fn writes_sys_set_config_when_eco_mode_is_set() {
    let mut probe = ConverterProbe::new();
    probe.run("writes_sys_set_config_when_eco_mode_is_set");
    assert!(probe.called("writes_sys_set_config_when_eco_mode_is_set"));
}

#[test]
#[ignore = "not implemented yet"]
fn writes_a_whole_group_in_a_single_presence_set_config_and_sends_only_what_was_given() {
    let mut probe = ConverterProbe::new();
    probe.run("writes_a_whole_group_in_a_single_presence_set_config_and_sends_only_what_was_given");
    assert!(probe.called(
        "writes_a_whole_group_in_a_single_presence_set_config_and_sends_only_what_was_given"
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn writes_to_the_main_zone_id_when_no_zone_ids_could_be_discovered() {
    let mut probe = ConverterProbe::new();
    probe.run("writes_to_the_main_zone_id_when_no_zone_ids_could_be_discovered");
    assert!(probe.called("writes_to_the_main_zone_id_when_no_zone_ids_could_be_discovered"));
}
