// Original: .upstream/zigbee-herdsman-converters/test/inovelli.test.ts

use zigbee_apiarist_converters::lib::inovelli::Inovelli;

#[test]
#[ignore = "not implemented yet"]
fn inovelli_ota() {
    let mut inovelli = Inovelli::new();
    inovelli.ota();
    assert!(inovelli.called("ota"));
}

#[test]
#[ignore = "not implemented yet"]
fn inovelli_vzm30_sn_definition_integration() {
    let mut inovelli = Inovelli::new();
    inovelli.definition_integration();
    assert!(inovelli.called("definition_integration"));
}

#[test]
#[ignore = "not implemented yet"]
fn inovelli_vzm31_sn_definition_integration() {
    let mut inovelli = Inovelli::new();
    inovelli.definition_integration();
    assert!(inovelli.called("definition_integration"));
}

#[test]
#[ignore = "not implemented yet"]
fn inovelli_vzm32_sn_definition_integration() {
    let mut inovelli = Inovelli::new();
    inovelli.definition_integration();
    assert!(inovelli.called("definition_integration"));
}

#[test]
#[ignore = "not implemented yet"]
fn inovelli_vzm35_sn_definition_integration() {
    let mut inovelli = Inovelli::new();
    inovelli.definition_integration();
    assert!(inovelli.called("definition_integration"));
}

#[test]
#[ignore = "not implemented yet"]
fn inovelli_vzm36_definition_integration() {
    let mut inovelli = Inovelli::new();
    inovelli.definition_integration();
    assert!(inovelli.called("definition_integration"));
}

#[test]
#[ignore = "not implemented yet"]
fn inovelli_vzm36_endpoint_routing() {
    let mut inovelli = Inovelli::new();
    inovelli.endpoint_routing();
    assert!(inovelli.called("endpoint_routing"));
}

#[test]
#[ignore = "not implemented yet"]
fn inovelli_baseline_exposes() {
    let mut inovelli = Inovelli::new();
    inovelli.baseline_exposes();
    assert!(inovelli.called("baseline_exposes"));
}

#[test]
#[ignore = "not implemented yet"]
fn inovelli_configure_attribute_filtering() {
    let mut inovelli = Inovelli::new();
    inovelli.configure_attribute_filtering();
    assert!(inovelli.called("configure_attribute_filtering"));
}

#[test]
#[ignore = "not implemented yet"]
fn inovelli_firmware_gated_exposes() {
    let mut inovelli = Inovelli::new();
    inovelli.firmware_gated_exposes();
    assert!(inovelli.called("firmware_gated_exposes"));
}

#[test]
#[ignore = "not implemented yet"]
fn inovelli_from_zigbee_converters() {
    let mut inovelli = Inovelli::new();
    inovelli.from_zigbee();
    assert!(inovelli.called("from_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn inovelli_to_zigbee_converters() {
    let mut inovelli = Inovelli::new();
    inovelli.to_zigbee();
    assert!(inovelli.called("to_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn led_effect_commands() {
    let mut inovelli = Inovelli::new();
    inovelli.led_effect();
    assert!(inovelli.called("led_effect"));
}

#[test]
#[ignore = "not implemented yet"]
fn led_effect_complete() {
    let mut inovelli = Inovelli::new();
    inovelli.led_effect();
    assert!(inovelli.called("led_effect"));
}

#[test]
#[ignore = "not implemented yet"]
fn vzm30_sn_should_expose_all_expected_attributes() {
    let mut inovelli = Inovelli::new();
    inovelli.baseline_exposes();
    assert!(inovelli.called("baseline_exposes"));
}

#[test]
#[ignore = "not implemented yet"]
fn vzm31_sn() {
    let mut inovelli = Inovelli::new();
    inovelli.definition_integration();
    assert!(inovelli.called("definition_integration"));
}

#[test]
#[ignore = "not implemented yet"]
fn vzm31_sn_should_expose_all_expected_attributes() {
    let mut inovelli = Inovelli::new();
    inovelli.baseline_exposes();
    assert!(inovelli.called("baseline_exposes"));
}

#[test]
#[ignore = "not implemented yet"]
fn vzm32_sn_mm_wave_converters() {
    let mut inovelli = Inovelli::new();
    inovelli.mmwave_area("set");
    assert!(inovelli.called("mmwave_area"));
}

#[test]
#[ignore = "not implemented yet"]
fn vzm32_sn_should_expose_all_expected_attributes() {
    let mut inovelli = Inovelli::new();
    inovelli.baseline_exposes();
    assert!(inovelli.called("baseline_exposes"));
}

#[test]
#[ignore = "not implemented yet"]
fn vzm35_sn_fan_converters() {
    let mut inovelli = Inovelli::new();
    inovelli.definition_integration();
    assert!(inovelli.called("definition_integration"));
}

#[test]
#[ignore = "not implemented yet"]
fn vzm35_sn_should_expose_all_expected_attributes() {
    let mut inovelli = Inovelli::new();
    inovelli.baseline_exposes();
    assert!(inovelli.called("baseline_exposes"));
}

#[test]
#[ignore = "not implemented yet"]
fn vzm36_endpoint_resolution() {
    let mut inovelli = Inovelli::new();
    inovelli.endpoint_routing();
    assert!(inovelli.called("endpoint_routing"));
}

#[test]
#[ignore = "not implemented yet"]
fn vzm36_should_expose_all_expected_attributes() {
    let mut inovelli = Inovelli::new();
    inovelli.baseline_exposes();
    assert!(inovelli.called("baseline_exposes"));
}

#[test]
#[ignore = "not implemented yet"]
fn vzm36_split_endpoint_attribute_reports() {
    let mut inovelli = Inovelli::new();
    inovelli.endpoint_routing();
    assert!(inovelli.called("endpoint_routing"));
}

#[test]
#[ignore = "not implemented yet"]
fn anyone_in_reporting_area() {
    let mut inovelli = Inovelli::new();
    inovelli.anyone_in_reporting_area();
    assert!(inovelli.called("anyone_in_reporting_area"));
}

#[test]
#[ignore = "not implemented yet"]
fn breeze_mode_decoding() {
    let mut inovelli = Inovelli::new();
    inovelli.breeze_mode();
    assert!(inovelli.called("breeze_mode"));
}

#[test]
#[ignore = "not implemented yet"]
fn breeze_mode_to_zigbee_vzm35_sn() {
    let mut inovelli = Inovelli::new();
    inovelli.to_zigbee();
    assert!(inovelli.called("to_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn brightness_ep1_only() {
    let mut inovelli = Inovelli::new();
    inovelli.definition_integration();
    assert!(inovelli.called("definition_integration"));
}

#[test]
#[ignore = "not implemented yet"]
fn button_scene_actions_raw_ep2() {
    let mut inovelli = Inovelli::new();
    inovelli.button_scene();
    assert!(inovelli.called("button_scene"));
}

#[test]
#[ignore = "not implemented yet"]
fn convert_get_should_read_current_level_from_the_correct_endpoint() {
    let mut inovelli = Inovelli::new();
    inovelli.convert_get();
    assert!(inovelli.called("convert_get"));
}

#[test]
#[ignore = "not implemented yet"]
fn convert_get_should_read_from_cluster_with_manufacturer_code() {
    let mut inovelli = Inovelli::new();
    inovelli.convert_get();
    assert!(inovelli.called("convert_get"));
}

#[test]
#[ignore = "not implemented yet"]
fn convert_get_should_read_from_the_cluster_with_manufacturer_code() {
    let mut inovelli = Inovelli::new();
    inovelli.convert_get();
    assert!(inovelli.called("convert_get"));
}

#[test]
#[ignore = "not implemented yet"]
fn convert_get_should_read_on_off() {
    let mut inovelli = Inovelli::new();
    inovelli.convert_get();
    assert!(inovelli.called("convert_get"));
}

#[test]
#[ignore = "not implemented yet"]
fn convert_get_with_suffixed_key_should_read_from_the_correct_endpoint() {
    let mut inovelli = Inovelli::new();
    inovelli.convert_get();
    assert!(inovelli.called("convert_get"));
}

#[test]
#[ignore = "not implemented yet"]
fn convert_set_for_a_numeric_attribute_should_write_raw_numeric_value() {
    let mut inovelli = Inovelli::new();
    inovelli.convert_set();
    assert!(inovelli.called("convert_set"));
}

#[test]
#[ignore = "not implemented yet"]
fn convert_set_for_an_enum_attribute_should_write_mapped_numeric_value() {
    let mut inovelli = Inovelli::new();
    inovelli.convert_set();
    assert!(inovelli.called("convert_set"));
}

#[test]
#[ignore = "not implemented yet"]
fn convert_set_should_batch_keys_regardless_of_the_order_they_appear_in_meta_message() {
    let mut inovelli = Inovelli::new();
    inovelli.convert_set();
    assert!(inovelli.called("convert_set"));
}

#[test]
#[ignore = "not implemented yet"]
fn convert_set_should_bucket_combined_1_2_keys_to_their_respective_endpoints() {
    let mut inovelli = Inovelli::new();
    inovelli.convert_set();
    assert!(inovelli.called("convert_set"));
}

#[test]
#[ignore = "not implemented yet"]
fn convert_set_should_ignore_unknown_and_read_only_keys_when_batching() {
    let mut inovelli = Inovelli::new();
    inovelli.convert_set();
    assert!(inovelli.called("convert_set"));
}

#[test]
#[ignore = "not implemented yet"]
fn convert_set_should_map_enum_values_when_batched_alongside_numeric_values() {
    let mut inovelli = Inovelli::new();
    inovelli.convert_set();
    assert!(inovelli.called("convert_set"));
}

#[test]
#[ignore = "not implemented yet"]
fn convert_set_should_return_undefined_for_unknown_key() {
    let mut inovelli = Inovelli::new();
    inovelli.convert_set();
    assert!(inovelli.called("convert_set"));
}

#[test]
#[ignore = "not implemented yet"]
fn convert_set_should_write_all_matching_keys_from_meta_message_in_one_batched_payload() {
    let mut inovelli = Inovelli::new();
    inovelli.convert_set();
    assert!(inovelli.called("convert_set"));
}

#[test]
#[ignore = "not implemented yet"]
fn convert_set_with_suffixed_key_should_write_to_the_correct_endpoint() {
    let mut inovelli = Inovelli::new();
    inovelli.convert_set();
    assert!(inovelli.called("convert_set"));
}

#[test]
#[ignore = "not implemented yet"]
fn converter_should_not_have_convert_set() {
    let mut inovelli = Inovelli::new();
    inovelli.convert_set();
    assert!(inovelli.called("convert_set"));
}

#[test]
#[ignore = "not implemented yet"]
fn custom_cluster_attribute_reports_with_enum_lookup() {
    let mut inovelli = Inovelli::new();
    inovelli.from_zigbee();
    assert!(inovelli.called("from_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_fire_on_wrong_endpoint() {
    let mut inovelli = Inovelli::new();
    inovelli.from_zigbee();
    assert!(inovelli.called("from_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn drops_firmware_gated_attributes_on_older_firmware_2_18() {
    let mut inovelli = Inovelli::new();
    inovelli.firmware_gated_exposes();
    assert!(inovelli.called("firmware_gated_exposes"));
}

#[test]
#[ignore = "not implemented yet"]
fn energy_reset() {
    let mut inovelli = Inovelli::new();
    inovelli.energy_reset();
    assert!(inovelli.called("energy_reset"));
}

#[test]
#[ignore = "not implemented yet"]
fn fan_control_mode_includes_toggle() {
    let mut inovelli = Inovelli::new();
    inovelli.fan_mode();
    assert!(inovelli.called("fan_mode"));
}

#[test]
#[ignore = "not implemented yet"]
fn fan_mode() {
    let mut inovelli = Inovelli::new();
    inovelli.fan_mode();
    assert!(inovelli.called("fan_mode"));
}

#[test]
#[ignore = "not implemented yet"]
fn fan_mode_to_zigbee_vzm35_sn() {
    let mut inovelli = Inovelli::new();
    inovelli.to_zigbee();
    assert!(inovelli.called("to_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn fan_mode_to_zigbee_vzm36_ep2() {
    let mut inovelli = Inovelli::new();
    inovelli.to_zigbee();
    assert!(inovelli.called("to_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn fan_state() {
    let mut inovelli = Inovelli::new();
    inovelli.fan_mode();
    assert!(inovelli.called("fan_mode"));
}

#[test]
#[ignore = "not implemented yet"]
fn fan_state_to_zigbee_vzm35_sn() {
    let mut inovelli = Inovelli::new();
    inovelli.to_zigbee();
    assert!(inovelli.called("to_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn gen_level_ctrl_from_endpoint_1_light() {
    let mut inovelli = Inovelli::new();
    inovelli.from_zigbee();
    assert!(inovelli.called("from_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn gen_level_ctrl_from_endpoint_2_fan() {
    let mut inovelli = Inovelli::new();
    inovelli.from_zigbee();
    assert!(inovelli.called("from_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn gen_on_off_from_endpoint_1_light() {
    let mut inovelli = Inovelli::new();
    inovelli.from_zigbee();
    assert!(inovelli.called("from_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn gen_on_off_from_endpoint_2_fan() {
    let mut inovelli = Inovelli::new();
    inovelli.from_zigbee();
    assert!(inovelli.called("from_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn inovelli_individual_led_effect_led_7_sent_as_6() {
    let mut inovelli = Inovelli::new();
    inovelli.individual_led_effect();
    assert!(inovelli.called("individual_led_effect"));
}

#[test]
#[ignore = "not implemented yet"]
fn inovelli_individual_led_effect_should_clamp_values() {
    let mut inovelli = Inovelli::new();
    inovelli.individual_led_effect();
    assert!(inovelli.called("individual_led_effect"));
}

#[test]
#[ignore = "not implemented yet"]
fn inovelli_individual_led_effect_should_convert_1_based_to_0_based_led_number() {
    let mut inovelli = Inovelli::new();
    inovelli.individual_led_effect();
    assert!(inovelli.called("individual_led_effect"));
}

#[test]
#[ignore = "not implemented yet"]
fn inovelli_led_effect_should_clamp_values() {
    let mut inovelli = Inovelli::new();
    inovelli.led_effect();
    assert!(inovelli.called("led_effect"));
}

#[test]
#[ignore = "not implemented yet"]
fn inovelli_led_effect_should_send_led_effect_command_with_correct_params() {
    let mut inovelli = Inovelli::new();
    inovelli.led_effect();
    assert!(inovelli.called("led_effect"));
}

#[test]
#[ignore = "not implemented yet"]
fn inovelli_parameters_write_get() {
    let mut inovelli = Inovelli::new();
    inovelli.convert_set();
    assert!(inovelli.called("convert_set"));
}

#[test]
#[ignore = "not implemented yet"]
fn inovelli_parameters_read_only() {
    let mut inovelli = Inovelli::new();
    inovelli.parameters_read_only();
    assert!(inovelli.called("parameters_read_only"));
}

#[test]
#[ignore = "not implemented yet"]
fn light_onoff_brightness_inovelli() {
    let mut inovelli = Inovelli::new();
    inovelli.light_onoff_brightness();
    assert!(inovelli.called("light_onoff_brightness"));
}

#[test]
#[ignore = "not implemented yet"]
fn matches_expected_integration_shape_canopy_module_split_endpoint_light_fan_no_led_effects_or_button_taps(
) {
    let mut inovelli = Inovelli::new();
    inovelli.definition_integration();
    assert!(inovelli.called("definition_integration"));
}

#[test]
#[ignore = "not implemented yet"]
fn matches_expected_integration_shape_fan_controller_with_breeze_mode_led_effects_and_button_taps()
{
    let mut inovelli = Inovelli::new();
    inovelli.definition_integration();
    assert!(inovelli.called("definition_integration"));
}

#[test]
#[ignore = "not implemented yet"]
fn matches_expected_integration_shape_mm_wave_dimmer_with_dual_custom_clusters() {
    let mut inovelli = Inovelli::new();
    inovelli.definition_integration();
    assert!(inovelli.called("definition_integration"));
}

#[test]
#[ignore = "not implemented yet"]
fn matches_expected_integration_shape_for_firmware_3_07() {
    let mut inovelli = Inovelli::new();
    inovelli.definition_integration();
    assert!(inovelli.called("definition_integration"));
}

#[test]
#[ignore = "not implemented yet"]
fn matches_expected_integration_shape_for_firmware_3_07_all_firmware_gated_attributes_enabled() {
    let mut inovelli = Inovelli::new();
    inovelli.definition_integration();
    assert!(inovelli.called("definition_integration"));
}

#[test]
#[ignore = "not implemented yet"]
fn mm_wave_to_zigbee_vzm32_sn() {
    let mut inovelli = Inovelli::new();
    inovelli.to_zigbee();
    assert!(inovelli.called("to_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn mmwave_detection_areas_should_send_set_detection_area_for_each_area() {
    let mut inovelli = Inovelli::new();
    inovelli.mmwave_area("set");
    assert!(inovelli.called("mmwave_area"));
}

#[test]
#[ignore = "not implemented yet"]
fn mmwave_interference_areas_should_send_set_interference_area() {
    let mut inovelli = Inovelli::new();
    inovelli.mmwave_area("set");
    assert!(inovelli.called("mmwave_area"));
}

#[test]
#[ignore = "not implemented yet"]
fn mmwave_stay_areas_should_send_set_stay_area() {
    let mut inovelli = Inovelli::new();
    inovelli.mmwave_area("set");
    assert!(inovelli.called("mmwave_area"));
}

#[test]
#[ignore = "not implemented yet"]
fn never_exposes_dimming_algorithm_aux_detection_level_dumb_detection_level_regardless_of_firmware()
{
    let mut inovelli = Inovelli::new();
    inovelli.baseline_exposes();
    assert!(inovelli.called("baseline_exposes"));
}

#[test]
#[ignore = "not implemented yet"]
fn off_with_no_transition_delegates_to_on_off_path() {
    let mut inovelli = Inovelli::new();
    inovelli.on_off(true, None);
    assert!(inovelli.called("on_off"));
}

#[test]
#[ignore = "not implemented yet"]
fn on_with_brightness_no_transition_uses_0xffff_transtime() {
    let mut inovelli = Inovelli::new();
    inovelli.on_off(true, None);
    assert!(inovelli.called("on_off"));
}

#[test]
#[ignore = "not implemented yet"]
fn on_with_explicit_transition_uses_that_transition_value() {
    let mut inovelli = Inovelli::new();
    inovelli.on_off(true, None);
    assert!(inovelli.called("on_off"));
}

#[test]
#[ignore = "not implemented yet"]
fn on_with_no_brightness_and_no_transition_uses_on_off_path() {
    let mut inovelli = Inovelli::new();
    inovelli.on_off(true, None);
    assert!(inovelli.called("on_off"));
}

#[test]
#[ignore = "not implemented yet"]
fn reads_common_attributes_switch_type_and_fan_control_mode() {
    let mut inovelli = Inovelli::new();
    inovelli.fan_mode();
    assert!(inovelli.called("fan_mode"));
}

#[test]
#[ignore = "not implemented yet"]
fn report_areas() {
    let mut inovelli = Inovelli::new();
    inovelli.mmwave_area("set");
    assert!(inovelli.called("mmwave_area"));
}

#[test]
#[ignore = "not implemented yet"]
fn report_target_info() {
    let mut inovelli = Inovelli::new();
    inovelli.mmwave_area("set");
    assert!(inovelli.called("mmwave_area"));
}

#[test]
#[ignore = "not implemented yet"]
fn schedules_a_follow_up_notification_complete() {
    let mut inovelli = Inovelli::new();
    inovelli.definition_integration();
    assert!(inovelli.called("definition_integration"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_decode_a_known_packed_integer() {
    let mut inovelli = Inovelli::new();
    inovelli.packed_integer();
    assert!(inovelli.called("packed_integer"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_decode_value_0_as_all_off() {
    let mut inovelli = Inovelli::new();
    inovelli.packed_integer();
    assert!(inovelli.called("packed_integer"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_delegate_to_on_off_and_remap_state_to_fan_state() {
    let mut inovelli = Inovelli::new();
    inovelli.fan_mode();
    assert!(inovelli.called("fan_mode"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_encode_full_5_speed_config_into_packed_integer() {
    let mut inovelli = Inovelli::new();
    inovelli.packed_integer();
    assert!(inovelli.called("packed_integer"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_handle_off_state() {
    let mut inovelli = Inovelli::new();
    inovelli.from_zigbee();
    assert!(inovelli.called("from_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_limit_targets_to_buffer_capacity_when_target_num_exceeds_it() {
    let mut inovelli = Inovelli::new();
    inovelli.mmwave_area("set");
    assert!(inovelli.called("mmwave_area"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_not_fire_on_wrong_endpoint() {
    let mut inovelli = Inovelli::new();
    inovelli.definition_integration();
    assert!(inovelli.called("definition_integration"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_not_leak_raw_on_off_data() {
    let mut inovelli = Inovelli::new();
    inovelli.from_zigbee();
    assert!(inovelli.called("from_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_not_return_brightness_for_ep2() {
    let mut inovelli = Inovelli::new();
    inovelli.definition_integration();
    assert!(inovelli.called("definition_integration"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_parse_a_single_target_from_buffer() {
    let mut inovelli = Inovelli::new();
    inovelli.mmwave_area("set");
    assert!(inovelli.called("mmwave_area"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_parse_two_targets_from_buffer() {
    let mut inovelli = Inovelli::new();
    inovelli.mmwave_area("set");
    assert!(inovelli.called("mmwave_area"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_brightness_for_ep1() {
    let mut inovelli = Inovelli::new();
    inovelli.from_zigbee();
    assert!(inovelli.called("from_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_fan_state_on_when_endpoint_id_is_2() {
    let mut inovelli = Inovelli::new();
    inovelli.fan_mode();
    assert!(inovelli.called("fan_mode"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_send_energy_reset_command_with_empty_payload() {
    let mut inovelli = Inovelli::new();
    inovelli.energy_reset();
    assert!(inovelli.called("energy_reset"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_set_brightness_without_affecting_fan_mode() {
    let mut inovelli = Inovelli::new();
    inovelli.fan_mode();
    assert!(inovelli.called("fan_mode"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_set_fan_mode_without_affecting_light_brightness() {
    let mut inovelli = Inovelli::new();
    inovelli.fan_mode();
    assert!(inovelli.called("fan_mode"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_set_fan_state_without_affecting_light_state() {
    let mut inovelli = Inovelli::new();
    inovelli.fan_mode();
    assert!(inovelli.called("fan_mode"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_set_light_state_without_affecting_fan_state() {
    let mut inovelli = Inovelli::new();
    inovelli.fan_mode();
    assert!(inovelli.called("fan_mode"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_set_ota_true_on_every_inovelli_device_definition() {
    let mut inovelli = Inovelli::new();
    inovelli.ota();
    assert!(inovelli.called("ota"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_terminate_early_when_speed2_is_off() {
    let mut inovelli = Inovelli::new();
    inovelli.definition_integration();
    assert!(inovelli.called("definition_integration"));
}

#[test]
#[ignore = "not implemented yet"]
fn switch_type_is_exactly() {
    let mut inovelli = Inovelli::new();
    inovelli.reads_common_attributes();
    assert!(inovelli.called("reads_common_attributes"));
}

#[test]
#[ignore = "not implemented yet"]
fn toggle_with_no_transition_uses_on_off_path() {
    let mut inovelli = Inovelli::new();
    inovelli.on_off(true, None);
    assert!(inovelli.called("on_off"));
}
