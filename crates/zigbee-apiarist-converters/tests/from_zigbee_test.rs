// Original: .upstream/zigbee-herdsman-converters/test/fromZigbee.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn device_specifying_voltage_to_percentage_ignores_reported_percentage() {
    let mut probe = ConverterProbe::new();
    probe.run("device_specifying_voltage_to_percentage_ignores_reported_percentage");
    assert!(probe.called("device_specifying_voltage_to_percentage_ignores_reported_percentage"));
}

#[test]
#[ignore = "not implemented yet"]
fn device_uses_reported_percentage() {
    let mut probe = ConverterProbe::new();
    probe.run("device_uses_reported_percentage");
    assert!(probe.called("device_uses_reported_percentage"));
}

#[test]
#[ignore = "not implemented yet"]
fn message_with_no_properties_does_not_error_converting_battery_percentages() {
    let mut probe = ConverterProbe::new();
    probe.run("message_with_no_properties_does_not_error_converting_battery_percentages");
    assert!(
        probe.called("message_with_no_properties_does_not_error_converting_battery_percentages")
    );
}

#[test]
#[ignore = "not implemented yet"]
fn xhs2_ue_uses_voltage_curve_3_v_2100_instead_of_stale_reported_battery_percentage() {
    let mut probe = ConverterProbe::new();
    probe.run("xhs2_ue_uses_voltage_curve_3_v_2100_instead_of_stale_reported_battery_percentage");
    assert!(probe.called(
        "xhs2_ue_uses_voltage_curve_3_v_2100_instead_of_stale_reported_battery_percentage"
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn zhemi101_converts_electricity_instantaneous_demand_to_w() {
    let mut probe = ConverterProbe::new();
    probe.run("zhemi101_converts_electricity_instantaneous_demand_to_w");
    assert!(probe.called("zhemi101_converts_electricity_instantaneous_demand_to_w"));
}

#[test]
#[ignore = "not implemented yet"]
fn zhemi101_keeps_gas_and_water_instantaneous_demand_in_m3_h() {
    let mut probe = ConverterProbe::new();
    probe.run("zhemi101_keeps_gas_and_water_instantaneous_demand_in_m3_h");
    assert!(probe.called("zhemi101_keeps_gas_and_water_instantaneous_demand_in_m3_h"));
}

#[test]
#[ignore = "not implemented yet"]
fn zhemi101_republishes_missing_metering_metadata_from_stale_null_state() {
    let mut probe = ConverterProbe::new();
    probe.run("zhemi101_republishes_missing_metering_metadata_from_stale_null_state");
    assert!(probe.called("zhemi101_republishes_missing_metering_metadata_from_stale_null_state"));
}

#[test]
#[ignore = "not implemented yet"]
fn command_step_color_temperature() {
    let mut probe = ConverterProbe::new();
    probe.run("command_step_color_temperature");
    assert!(probe.called("command_step_color_temperature"));
}

#[test]
#[ignore = "not implemented yet"]
fn converters_from_zigbee() {
    let mut probe = ConverterProbe::new();
    probe.run("converters_from_zigbee");
    assert!(probe.called("converters_from_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn converts_a_fully_closed_lift_percentage_using_the_default_convention() {
    let mut probe = ConverterProbe::new();
    probe.run("converts_a_fully_closed_lift_percentage_using_the_default_convention");
    assert!(probe.called("converts_a_fully_closed_lift_percentage_using_the_default_convention"));
}

#[test]
#[ignore = "not implemented yet"]
fn converts_a_fully_open_lift_percentage_using_the_default_non_inverted_convention() {
    let mut probe = ConverterProbe::new();
    probe.run("converts_a_fully_open_lift_percentage_using_the_default_non_inverted_convention");
    assert!(probe
        .called("converts_a_fully_open_lift_percentage_using_the_default_non_inverted_convention"));
}

#[test]
#[ignore = "not implemented yet"]
fn cover_position_tilt() {
    let mut probe = ConverterProbe::new();
    probe.run("cover_position_tilt");
    assert!(probe.called("cover_position_tilt"));
}

#[test]
#[ignore = "not implemented yet"]
fn decodes_cover_mode_bits() {
    let mut probe = ConverterProbe::new();
    probe.run("decodes_cover_mode_bits");
    assert!(probe.called("decodes_cover_mode_bits"));
}

#[test]
#[ignore = "not implemented yet"]
fn derives_state_from_tilt_when_cover_state_from_tilt_is_set() {
    let mut probe = ConverterProbe::new();
    probe.run("derives_state_from_tilt_when_cover_state_from_tilt_is_set");
    assert!(probe.called("derives_state_from_tilt_when_cover_state_from_tilt_is_set"));
}

#[test]
#[ignore = "not implemented yet"]
fn device_level_cover_inverted_meta_alone_preserves_pre_existing_behaviour() {
    let mut probe = ConverterProbe::new();
    probe.run("device_level_cover_inverted_meta_alone_preserves_pre_existing_behaviour");
    assert!(probe.called("device_level_cover_inverted_meta_alone_preserves_pre_existing_behaviour"));
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_include_action_transition_time_when_transtime_is_undefined() {
    let mut probe = ConverterProbe::new();
    probe.run("does_not_include_action_transition_time_when_transtime_is_undefined");
    assert!(probe.called("does_not_include_action_transition_time_when_transtime_is_undefined"));
}

#[test]
#[ignore = "not implemented yet"]
fn emits_negative_action_color_temperature_delta_when_stepping_down() {
    let mut probe = ConverterProbe::new();
    probe.run("emits_negative_action_color_temperature_delta_when_stepping_down");
    assert!(probe.called("emits_negative_action_color_temperature_delta_when_stepping_down"));
}

#[test]
#[ignore = "not implemented yet"]
fn emits_positive_action_color_temperature_delta_when_stepping_up() {
    let mut probe = ConverterProbe::new();
    probe.run("emits_positive_action_color_temperature_delta_when_stepping_up");
    assert!(probe.called("emits_positive_action_color_temperature_delta_when_stepping_up"));
}

#[test]
#[ignore = "not implemented yet"]
fn invert_cover_flips_both_position_and_state_together() {
    let mut probe = ConverterProbe::new();
    probe.run("invert_cover_flips_both_position_and_state_together");
    assert!(probe.called("invert_cover_flips_both_position_and_state_together"));
}

#[test]
#[ignore = "not implemented yet"]
fn invert_cover_flips_tilt_derived_state_too() {
    let mut probe = ConverterProbe::new();
    probe.run("invert_cover_flips_tilt_derived_state_too");
    assert!(probe.called("invert_cover_flips_tilt_derived_state_too"));
}

#[test]
#[ignore = "not implemented yet"]
fn invert_cover_on_a_cover_inverted_device_cancels_back_to_the_non_inverted_combination() {
    let mut probe = ConverterProbe::new();
    probe.run(
        "invert_cover_on_a_cover_inverted_device_cancels_back_to_the_non_inverted_combination",
    );
    assert!(probe.called(
        "invert_cover_on_a_cover_inverted_device_cancels_back_to_the_non_inverted_combination"
    ));
}
