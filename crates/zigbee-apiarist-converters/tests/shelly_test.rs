// Original: .upstream/zigbee-herdsman-converters/test/shelly.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Shelly 1L Gen4
//   - Shelly 2PM Gen4 cover mode
//   - Shelly 2PM Gen4 switch input endpoints
//   - Shelly 2PM Gen4 switch input events
//   - Shelly Gen4 settings the device cannot report
//   - Shelly Gen4 switch definitions carry the re-configure version bump
//   - Shelly Presence Gen4
//   - Shelly RPC hygiene
//   - Shelly WS90 rain rate
//   - Shelly Wi-Fi setup
//   - a full set still writes every group
//   - an empty payload writes nothing, not even the apply
//   - clamps out-of-range delays to the 0-3600 second bounds
//   - configures both input endpoints
//   - configures occupancy reporting on every tracked-person endpoint
//   - configures toggle actions for an opt-in momentary input binding
//   - disables toggle actions for an opt-in momentary input binding
//   - discards the non-value markers instead of publishing them as measurements
//   - does not announce a read for power strip settings that have no read converter
//   - does not expose switch input controls when the device has no switch input endpoints
//   - does not fail a get when both Shelly Wi-Fi readback paths are unavailable
//   - does not read the sensor back after writing a group
//   - does not read the zone back after writing a delay
//   - does not serialize RPC writes of different devices against each other
//   - exposes an eco_mode switch
//   - exposes an occupancy endpoint for every person the device can track
//   - exposes no switch_type on a 1PM Mini that has no switch input endpoint
//   - exposes presence_delay and absence_delay for each detected zone
//   - exposes switch_type on a 1PM Mini that has a switch input endpoint
//   - groups the sensor settings and keeps the device
//   - ignores commands from endpoints that are no switch input instead of throwing
//   - keeps both switch inputs exposed on a switch-mode 2PM that has them wired
//   - keeps configuring the remaining endpoints when one of them fails
//   - keeps every occupancy endpoint even when only one zone is known
//   - keeps single-channel input events working when the input is wired
//   - keeps the cover-mode 2PM on its own input endpoints
//   - keeps the switch-mode inputs on endpoints 3 and 4
//   - keeps tilt controls visible by default and allows explicit opt-out
//   - lists the reported custom sensitivity but refuses to write it
//   - makes no discovery attempt while the firmware cannot answer reads
//   - maps both input endpoints to separate actions and switch types
//   - maps cover input 1 (endpoint 2) and input 2 (endpoint 3) to their own actions
//   - maps switch-mode endpoint 3 input type to sw1
//   - maps the captured relay and input endpoint layout
//   - maps the second cover switch input type to sw2
//   - marks switch_mode write-only and offers no read
//   - merges several deeply nested thresholds into one nested config
//   - offers no RPC-backed reads at all
//   - offers no switch input mode on a switch-mode 2PM whose inputs are not wired
//   - only configures the relay-associated input mode
//   - persists the device meta at most once a minute
//   - prefers a discovered zone id over the main zone fallback
//   - publishes a repeated transaction only once
//   - reads the Wi-Fi state through the setup cluster without trying RPC first
//   - recovers the rain rate after the cumulative counter resets
//   - registers no RPC diagnostics converter without the Dev feature
//   - rejects momentary toggle binding setup for a toggle input
//   - retries a delay write when the first RPC send fails
//   - setting only credentials does not silently disable Wi-Fi
//   - tells the user that power strip settings cannot be read back
//   - toggling only enabled does not touch the stored credentials
//   - uses the cached full Shelly Wi-Fi SSID when the setup cluster reports a shortened name
//   - writes PresenceZone.SetConfig to the mapped zone id when setting a delay
//   - writes Sys.SetConfig when eco_mode is set
//   - writes a whole group in a single Presence.SetConfig and sends only what was given
//   - writes to the main zone id when no zone ids could be discovered

#[test]
#[ignore = "not implemented yet"]
fn shelly_1_l_gen4() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn shelly_2_pm_gen4_cover_mode() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn shelly_2_pm_gen4_switch_input_endpoints() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn shelly_2_pm_gen4_switch_input_events() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn shelly_gen4_settings_the_device_cannot_report() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn shelly_gen4_switch_definitions_carry_the_re_configure_version_bump() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn shelly_presence_gen4() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn shelly_rpc_hygiene() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn shelly_ws90_rain_rate() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn shelly_wi_fi_setup() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn a_full_set_still_writes_every_group() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn an_empty_payload_writes_nothing_not_even_the_apply() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn clamps_out_of_range_delays_to_the_0_3600_second_bounds() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn configures_both_input_endpoints() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn configures_occupancy_reporting_on_every_tracked_person_endpoint() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn configures_toggle_actions_for_an_opt_in_momentary_input_binding() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn disables_toggle_actions_for_an_opt_in_momentary_input_binding() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn discards_the_non_value_markers_instead_of_publishing_them_as_measurements() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_announce_a_read_for_power_strip_settings_that_have_no_read_converter() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_expose_switch_input_controls_when_the_device_has_no_switch_input_endpoints() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_fail_a_get_when_both_shelly_wi_fi_readback_paths_are_unavailable() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_read_the_sensor_back_after_writing_a_group() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_read_the_zone_back_after_writing_a_delay() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_serialize_rpc_writes_of_different_devices_against_each_other() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_an_eco_mode_switch() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_an_occupancy_endpoint_for_every_person_the_device_can_track() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_no_switch_type_on_a_1_pm_mini_that_has_no_switch_input_endpoint() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_presence_delay_and_absence_delay_for_each_detected_zone() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_switch_type_on_a_1_pm_mini_that_has_a_switch_input_endpoint() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn groups_the_sensor_settings_and_keeps_the_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ignores_commands_from_endpoints_that_are_no_switch_input_instead_of_throwing() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_both_switch_inputs_exposed_on_a_switch_mode_2_pm_that_has_them_wired() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_configuring_the_remaining_endpoints_when_one_of_them_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_every_occupancy_endpoint_even_when_only_one_zone_is_known() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_single_channel_input_events_working_when_the_input_is_wired() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_the_cover_mode_2_pm_on_its_own_input_endpoints() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_the_switch_mode_inputs_on_endpoints_3_and_4() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_tilt_controls_visible_by_default_and_allows_explicit_opt_out() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn lists_the_reported_custom_sensitivity_but_refuses_to_write_it() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn makes_no_discovery_attempt_while_the_firmware_cannot_answer_reads() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn maps_both_input_endpoints_to_separate_actions_and_switch_types() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn maps_cover_input_1_endpoint_2_and_input_2_endpoint_3_to_their_own_actions() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn maps_switch_mode_endpoint_3_input_type_to_sw1() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn maps_the_captured_relay_and_input_endpoint_layout() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn maps_the_second_cover_switch_input_type_to_sw2() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn marks_switch_mode_write_only_and_offers_no_read() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn merges_several_deeply_nested_thresholds_into_one_nested_config() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn offers_no_rpc_backed_reads_at_all() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn offers_no_switch_input_mode_on_a_switch_mode_2_pm_whose_inputs_are_not_wired() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn only_configures_the_relay_associated_input_mode() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn persists_the_device_meta_at_most_once_a_minute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn prefers_a_discovered_zone_id_over_the_main_zone_fallback() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn publishes_a_repeated_transaction_only_once() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_the_wi_fi_state_through_the_setup_cluster_without_trying_rpc_first() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn recovers_the_rain_rate_after_the_cumulative_counter_resets() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn registers_no_rpc_diagnostics_converter_without_the_dev_feature() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn rejects_momentary_toggle_binding_setup_for_a_toggle_input() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn retries_a_delay_write_when_the_first_rpc_send_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn setting_only_credentials_does_not_silently_disable_wi_fi() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn tells_the_user_that_power_strip_settings_cannot_be_read_back() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn toggling_only_enabled_does_not_touch_the_stored_credentials() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uses_the_cached_full_shelly_wi_fi_ssid_when_the_setup_cluster_reports_a_shortened_name() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_presence_zone_set_config_to_the_mapped_zone_id_when_setting_a_delay() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_sys_set_config_when_eco_mode_is_set() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_a_whole_group_in_a_single_presence_set_config_and_sends_only_what_was_given() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_to_the_main_zone_id_when_no_zone_ids_could_be_discovered() {
    todo!()
}
