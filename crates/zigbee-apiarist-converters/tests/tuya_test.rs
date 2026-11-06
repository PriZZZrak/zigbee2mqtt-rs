// Original: .upstream/zigbee-herdsman-converters/test/tuya.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - TS0001 manufacturer-dependent settings
//   - TS004F button configure (ZG-101ZL)
//   - TS004F knob configure (ZG-101ZD)
//   - TS130F manufacturer-dependent switch type
//   - closuresWindowCovering custom cluster (Tuya covers)
//   - corrects a Nous B4Z stale start position after an optimistic position update
//   - declares moesCalibrationTime as UINT16 so value*10 fits
//   - decodes readings below the 16 bit boundary
//   - does not correct a Nous B4Z STOP report before the target was acknowledged
//   - does not replace an acknowledged Nous B4Z target with a stale moving report
//   - does not turn a large positive power into a negative one
//   - dpTHZBSettings
//   - exposes power_on_behavior for manufacturers using the manuSpecificTuya3 attribute
//   - exposes the settings _TZ3000_p26flek3 reports on its private clusters
//   - ignores UNSUPPORTED_ATTRIBUTE on the magic packet and tuyaOperationMode write
//   - ignores UNSUPPORTED_ATTRIBUTE on the tuyaOperationMode write
//   - keeps current consistent with power and voltage under high load
//   - keeps large positive readings positive
//   - keeps other TS0001 manufacturers on on/off only
//   - leaves the generic TS130F switch type unchanged
//   - leaves the settings of _TZ3000_bzzgvet0 unchanged
//   - lets power_on_behavior_2 win the shared power_on_behavior key
//   - lib/tuya
//   - never reports a negative reading as a multi megawatt positive one
//   - phaseVariant2WithPhase
//   - still exposes power_outage_memory for the legacy manufacturers on the same definition
//   - still fails on other errors
//   - suffixes every key with the phase
//   - tuyaOnOff power-on behaviour selection
//   - tuyaWeatherForecast
//   - uses forecast fields 1 through 3 and includes their humidity in the payload
//   - uses the manuSpecificTuya3 wall-switch selector for the Moes ZM-108-M
//   - writes backlight_mode to tuyaBacklightSwitch and indicator_mode to tuyaBacklightMode

#[test]
#[ignore = "not implemented yet"]
fn ts0001_manufacturer_dependent_settings() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ts004_f_button_configure_zg_101_zl() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ts004_f_knob_configure_zg_101_zd() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ts130_f_manufacturer_dependent_switch_type() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn closures_window_covering_custom_cluster_tuya_covers() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn corrects_a_nous_b4_z_stale_start_position_after_an_optimistic_position_update() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn declares_moes_calibration_time_as_uint16_so_value_10_fits() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn decodes_readings_below_the_16_bit_boundary() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_correct_a_nous_b4_z_stop_report_before_the_target_was_acknowledged() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_replace_an_acknowledged_nous_b4_z_target_with_a_stale_moving_report() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_turn_a_large_positive_power_into_a_negative_one() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn dp_thzbsettings() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_power_on_behavior_for_manufacturers_using_the_manu_specific_tuya3_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_the_settings_tz3000_p26flek3_reports_on_its_private_clusters() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ignores_unsupported_attribute_on_the_magic_packet_and_tuya_operation_mode_write() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ignores_unsupported_attribute_on_the_tuya_operation_mode_write() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_current_consistent_with_power_and_voltage_under_high_load() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_large_positive_readings_positive() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_other_ts0001_manufacturers_on_on_off_only() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn leaves_the_generic_ts130_f_switch_type_unchanged() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn leaves_the_settings_of_tz3000_bzzgvet0_unchanged() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn lets_power_on_behavior_2_win_the_shared_power_on_behavior_key() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn lib_tuya() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn never_reports_a_negative_reading_as_a_multi_megawatt_positive_one() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn phase_variant2_with_phase() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn still_exposes_power_outage_memory_for_the_legacy_manufacturers_on_the_same_definition() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn still_fails_on_other_errors() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn suffixes_every_key_with_the_phase() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn tuya_on_off_power_on_behaviour_selection() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn tuya_weather_forecast() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uses_forecast_fields_1_through_3_and_includes_their_humidity_in_the_payload() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uses_the_manu_specific_tuya3_wall_switch_selector_for_the_moes_zm_108_m() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_backlight_mode_to_tuya_backlight_switch_and_indicator_mode_to_tuya_backlight_mode() {
    todo!()
}
