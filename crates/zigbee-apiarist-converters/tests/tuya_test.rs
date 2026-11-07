// Original: .upstream/zigbee-herdsman-converters/test/tuya.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn ts0001_manufacturer_dependent_settings() {
    let mut probe = ConverterProbe::new();
    probe.run("ts0001_manufacturer_dependent_settings");
    assert!(probe.called("ts0001_manufacturer_dependent_settings"));
}

#[test]
#[ignore = "not implemented yet"]
fn ts004_f_button_configure_zg_101_zl() {
    let mut probe = ConverterProbe::new();
    probe.run("ts004_f_button_configure_zg_101_zl");
    assert!(probe.called("ts004_f_button_configure_zg_101_zl"));
}

#[test]
#[ignore = "not implemented yet"]
fn ts004_f_knob_configure_zg_101_zd() {
    let mut probe = ConverterProbe::new();
    probe.run("ts004_f_knob_configure_zg_101_zd");
    assert!(probe.called("ts004_f_knob_configure_zg_101_zd"));
}

#[test]
#[ignore = "not implemented yet"]
fn ts130_f_manufacturer_dependent_switch_type() {
    let mut probe = ConverterProbe::new();
    probe.run("ts130_f_manufacturer_dependent_switch_type");
    assert!(probe.called("ts130_f_manufacturer_dependent_switch_type"));
}

#[test]
#[ignore = "not implemented yet"]
fn closures_window_covering_custom_cluster_tuya_covers() {
    let mut probe = ConverterProbe::new();
    probe.run("closures_window_covering_custom_cluster_tuya_covers");
    assert!(probe.called("closures_window_covering_custom_cluster_tuya_covers"));
}

#[test]
#[ignore = "not implemented yet"]
fn corrects_a_nous_b4_z_stale_start_position_after_an_optimistic_position_update() {
    let mut probe = ConverterProbe::new();
    probe.run("corrects_a_nous_b4_z_stale_start_position_after_an_optimistic_position_update");
    assert!(probe
        .called("corrects_a_nous_b4_z_stale_start_position_after_an_optimistic_position_update"));
}

#[test]
#[ignore = "not implemented yet"]
fn declares_moes_calibration_time_as_uint16_so_value_10_fits() {
    let mut probe = ConverterProbe::new();
    probe.run("declares_moes_calibration_time_as_uint16_so_value_10_fits");
    assert!(probe.called("declares_moes_calibration_time_as_uint16_so_value_10_fits"));
}

#[test]
#[ignore = "not implemented yet"]
fn decodes_readings_below_the_16_bit_boundary() {
    let mut probe = ConverterProbe::new();
    probe.run("decodes_readings_below_the_16_bit_boundary");
    assert!(probe.called("decodes_readings_below_the_16_bit_boundary"));
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_correct_a_nous_b4_z_stop_report_before_the_target_was_acknowledged() {
    let mut probe = ConverterProbe::new();
    probe.run("does_not_correct_a_nous_b4_z_stop_report_before_the_target_was_acknowledged");
    assert!(
        probe.called("does_not_correct_a_nous_b4_z_stop_report_before_the_target_was_acknowledged")
    );
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_replace_an_acknowledged_nous_b4_z_target_with_a_stale_moving_report() {
    let mut probe = ConverterProbe::new();
    probe.run("does_not_replace_an_acknowledged_nous_b4_z_target_with_a_stale_moving_report");
    assert!(probe
        .called("does_not_replace_an_acknowledged_nous_b4_z_target_with_a_stale_moving_report"));
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_turn_a_large_positive_power_into_a_negative_one() {
    let mut probe = ConverterProbe::new();
    probe.run("does_not_turn_a_large_positive_power_into_a_negative_one");
    assert!(probe.called("does_not_turn_a_large_positive_power_into_a_negative_one"));
}

#[test]
#[ignore = "not implemented yet"]
fn dp_thzbsettings() {
    let mut probe = ConverterProbe::new();
    probe.run("dp_thzbsettings");
    assert!(probe.called("dp_thzbsettings"));
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_power_on_behavior_for_manufacturers_using_the_manu_specific_tuya3_attribute() {
    let mut probe = ConverterProbe::new();
    probe
        .run("exposes_power_on_behavior_for_manufacturers_using_the_manu_specific_tuya3_attribute");
    assert!(probe.called(
        "exposes_power_on_behavior_for_manufacturers_using_the_manu_specific_tuya3_attribute"
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_the_settings_tz3000_p26flek3_reports_on_its_private_clusters() {
    let mut probe = ConverterProbe::new();
    probe.run("exposes_the_settings_tz3000_p26flek3_reports_on_its_private_clusters");
    assert!(probe.called("exposes_the_settings_tz3000_p26flek3_reports_on_its_private_clusters"));
}

#[test]
#[ignore = "not implemented yet"]
fn ignores_unsupported_attribute_on_the_magic_packet_and_tuya_operation_mode_write() {
    let mut probe = ConverterProbe::new();
    probe.run("ignores_unsupported_attribute_on_the_magic_packet_and_tuya_operation_mode_write");
    assert!(probe
        .called("ignores_unsupported_attribute_on_the_magic_packet_and_tuya_operation_mode_write"));
}

#[test]
#[ignore = "not implemented yet"]
fn ignores_unsupported_attribute_on_the_tuya_operation_mode_write() {
    let mut probe = ConverterProbe::new();
    probe.run("ignores_unsupported_attribute_on_the_tuya_operation_mode_write");
    assert!(probe.called("ignores_unsupported_attribute_on_the_tuya_operation_mode_write"));
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_current_consistent_with_power_and_voltage_under_high_load() {
    let mut probe = ConverterProbe::new();
    probe.run("keeps_current_consistent_with_power_and_voltage_under_high_load");
    assert!(probe.called("keeps_current_consistent_with_power_and_voltage_under_high_load"));
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_large_positive_readings_positive() {
    let mut probe = ConverterProbe::new();
    probe.run("keeps_large_positive_readings_positive");
    assert!(probe.called("keeps_large_positive_readings_positive"));
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_other_ts0001_manufacturers_on_on_off_only() {
    let mut probe = ConverterProbe::new();
    probe.run("keeps_other_ts0001_manufacturers_on_on_off_only");
    assert!(probe.called("keeps_other_ts0001_manufacturers_on_on_off_only"));
}

#[test]
#[ignore = "not implemented yet"]
fn leaves_the_generic_ts130_f_switch_type_unchanged() {
    let mut probe = ConverterProbe::new();
    probe.run("leaves_the_generic_ts130_f_switch_type_unchanged");
    assert!(probe.called("leaves_the_generic_ts130_f_switch_type_unchanged"));
}

#[test]
#[ignore = "not implemented yet"]
fn leaves_the_settings_of_tz3000_bzzgvet0_unchanged() {
    let mut probe = ConverterProbe::new();
    probe.run("leaves_the_settings_of_tz3000_bzzgvet0_unchanged");
    assert!(probe.called("leaves_the_settings_of_tz3000_bzzgvet0_unchanged"));
}

#[test]
#[ignore = "not implemented yet"]
fn lets_power_on_behavior_2_win_the_shared_power_on_behavior_key() {
    let mut probe = ConverterProbe::new();
    probe.run("lets_power_on_behavior_2_win_the_shared_power_on_behavior_key");
    assert!(probe.called("lets_power_on_behavior_2_win_the_shared_power_on_behavior_key"));
}

#[test]
#[ignore = "not implemented yet"]
fn lib_tuya() {
    let mut probe = ConverterProbe::new();
    probe.run("lib_tuya");
    assert!(probe.called("lib_tuya"));
}

#[test]
#[ignore = "not implemented yet"]
fn never_reports_a_negative_reading_as_a_multi_megawatt_positive_one() {
    let mut probe = ConverterProbe::new();
    probe.run("never_reports_a_negative_reading_as_a_multi_megawatt_positive_one");
    assert!(probe.called("never_reports_a_negative_reading_as_a_multi_megawatt_positive_one"));
}

#[test]
#[ignore = "not implemented yet"]
fn phase_variant2_with_phase() {
    let mut probe = ConverterProbe::new();
    probe.run("phase_variant2_with_phase");
    assert!(probe.called("phase_variant2_with_phase"));
}

#[test]
#[ignore = "not implemented yet"]
fn still_exposes_power_outage_memory_for_the_legacy_manufacturers_on_the_same_definition() {
    let mut probe = ConverterProbe::new();
    probe.run(
        "still_exposes_power_outage_memory_for_the_legacy_manufacturers_on_the_same_definition",
    );
    assert!(probe.called(
        "still_exposes_power_outage_memory_for_the_legacy_manufacturers_on_the_same_definition"
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn still_fails_on_other_errors() {
    let mut probe = ConverterProbe::new();
    probe.run("still_fails_on_other_errors");
    assert!(probe.called("still_fails_on_other_errors"));
}

#[test]
#[ignore = "not implemented yet"]
fn suffixes_every_key_with_the_phase() {
    let mut probe = ConverterProbe::new();
    probe.run("suffixes_every_key_with_the_phase");
    assert!(probe.called("suffixes_every_key_with_the_phase"));
}

#[test]
#[ignore = "not implemented yet"]
fn tuya_on_off_power_on_behaviour_selection() {
    let mut probe = ConverterProbe::new();
    probe.run("tuya_on_off_power_on_behaviour_selection");
    assert!(probe.called("tuya_on_off_power_on_behaviour_selection"));
}

#[test]
#[ignore = "not implemented yet"]
fn tuya_weather_forecast() {
    let mut probe = ConverterProbe::new();
    probe.run("tuya_weather_forecast");
    assert!(probe.called("tuya_weather_forecast"));
}

#[test]
#[ignore = "not implemented yet"]
fn uses_forecast_fields_1_through_3_and_includes_their_humidity_in_the_payload() {
    let mut probe = ConverterProbe::new();
    probe.run("uses_forecast_fields_1_through_3_and_includes_their_humidity_in_the_payload");
    assert!(
        probe.called("uses_forecast_fields_1_through_3_and_includes_their_humidity_in_the_payload")
    );
}

#[test]
#[ignore = "not implemented yet"]
fn uses_the_manu_specific_tuya3_wall_switch_selector_for_the_moes_zm_108_m() {
    let mut probe = ConverterProbe::new();
    probe.run("uses_the_manu_specific_tuya3_wall_switch_selector_for_the_moes_zm_108_m");
    assert!(probe.called("uses_the_manu_specific_tuya3_wall_switch_selector_for_the_moes_zm_108_m"));
}

#[test]
#[ignore = "not implemented yet"]
fn writes_backlight_mode_to_tuya_backlight_switch_and_indicator_mode_to_tuya_backlight_mode() {
    let mut probe = ConverterProbe::new();
    probe.run(
        "writes_backlight_mode_to_tuya_backlight_switch_and_indicator_mode_to_tuya_backlight_mode",
    );
    assert!(probe.called(
        "writes_backlight_mode_to_tuya_backlight_switch_and_indicator_mode_to_tuya_backlight_mode"
    ));
}
