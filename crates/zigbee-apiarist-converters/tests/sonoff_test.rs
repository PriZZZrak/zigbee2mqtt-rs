// Original: .upstream/zigbee-herdsman-converters/test/sonoff.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn sonoff_snzb_02_dr2() {
    let mut probe = ConverterProbe::new();
    probe.run("sonoff_snzb_02_dr2");
    assert!(probe.called("sonoff_snzb_02_dr2"));
}

#[test]
#[ignore = "not implemented yet"]
fn sonoff_swv() {
    let mut probe = ConverterProbe::new();
    probe.run("sonoff_swv");
    assert!(probe.called("sonoff_swv"));
}

#[test]
#[ignore = "not implemented yet"]
fn sonoff_swv_zfe() {
    let mut probe = ConverterProbe::new();
    probe.run("sonoff_swv_zfe");
    assert!(probe.called("sonoff_swv_zfe"));
}

#[test]
#[ignore = "not implemented yet"]
fn sonoff_trvzb() {
    let mut probe = ConverterProbe::new();
    probe.run("sonoff_trvzb");
    assert!(probe.called("sonoff_trvzb"));
}

#[test]
#[ignore = "not implemented yet"]
fn continues_configuring_when_optional_water_shortage_auto_close_attribute_is_unsupported() {
    let mut probe = ConverterProbe::new();
    probe.run(
        "continues_configuring_when_optional_water_shortage_auto_close_attribute_is_unsupported",
    );
    assert!(probe.called(
        "continues_configuring_when_optional_water_shortage_auto_close_attribute_is_unsupported"
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn disables_the_external_display_via_temperature_sensor_select() {
    let mut probe = ConverterProbe::new();
    probe.run("disables_the_external_display_via_temperature_sensor_select");
    assert!(probe.called("disables_the_external_display_via_temperature_sensor_select"));
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_clear_a_legacy_amount_unit_when_setting_the_same_unit() {
    let mut probe = ConverterProbe::new();
    probe.run("does_not_clear_a_legacy_amount_unit_when_setting_the_same_unit");
    assert!(probe.called("does_not_clear_a_legacy_amount_unit_when_setting_the_same_unit"));
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_clear_a_legacy_amount_unit_when_the_reported_unit_is_unchanged() {
    let mut probe = ConverterProbe::new();
    probe.run("does_not_clear_a_legacy_amount_unit_when_the_reported_unit_is_unchanged");
    assert!(probe.called("does_not_clear_a_legacy_amount_unit_when_the_reported_unit_is_unchanged"));
}

#[test]
#[ignore = "not implemented yet"]
fn enables_the_external_display_via_temperature_sensor_select() {
    let mut probe = ConverterProbe::new();
    probe.run("enables_the_external_display_via_temperature_sensor_select");
    assert!(probe.called("enables_the_external_display_via_temperature_sensor_select"));
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_manual_irrigation_settings_as_scalar_controls() {
    let mut probe = ConverterProbe::new();
    probe.run("exposes_manual_irrigation_settings_as_scalar_controls");
    assert!(probe.called("exposes_manual_irrigation_settings_as_scalar_controls"));
}

#[test]
#[ignore = "not implemented yet"]
fn fills_missing_scalar_settings_with_defaults_on_first_write() {
    let mut probe = ConverterProbe::new();
    probe.run("fills_missing_scalar_settings_with_defaults_on_first_write");
    assert!(probe.called("fills_missing_scalar_settings_with_defaults_on_first_write"));
}

#[test]
#[ignore = "not implemented yet"]
fn from_zigbee() {
    let mut probe = ConverterProbe::new();
    probe.run("from_zigbee");
    assert!(probe.called("from_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn hides_manual_amount_unit_on_firmware_with_unified_water_flow_units() {
    let mut probe = ConverterProbe::new();
    probe.run("hides_manual_amount_unit_on_firmware_with_unified_water_flow_units");
    assert!(probe.called("hides_manual_amount_unit_on_firmware_with_unified_water_flow_units"));
}

#[test]
#[ignore = "not implemented yet"]
fn merges_a_scalar_manual_setting_with_current_scalar_state() {
    let mut probe = ConverterProbe::new();
    probe.run("merges_a_scalar_manual_setting_with_current_scalar_state");
    assert!(probe.called("merges_a_scalar_manual_setting_with_current_scalar_state"));
}

#[test]
#[ignore = "not implemented yet"]
fn multi_day_batch_updates_via_meta_message() {
    let mut probe = ConverterProbe::new();
    probe.run("multi_day_batch_updates_via_meta_message");
    assert!(probe.called("multi_day_batch_updates_via_meta_message"));
}

#[test]
#[ignore = "not implemented yet"]
fn preserves_the_real_liter_amount_when_an_ota_updated_device_first_reports_its_unified_unit() {
    let mut probe = ConverterProbe::new();
    probe.run(
        "preserves_the_real_liter_amount_when_an_ota_updated_device_first_reports_its_unified_unit",
    );
    assert!(probe.called(
        "preserves_the_real_liter_amount_when_an_ota_updated_device_first_reports_its_unified_unit"
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn removes_an_irrigation_plan_through_the_composite_remove_command() {
    let mut probe = ConverterProbe::new();
    probe.run("removes_an_irrigation_plan_through_the_composite_remove_command");
    assert!(probe.called("removes_an_irrigation_plan_through_the_composite_remove_command"));
}

#[test]
#[ignore = "not implemented yet"]
fn sends_irrigation_plan_settings_to_device() {
    let mut probe = ConverterProbe::new();
    probe.run("sends_irrigation_plan_settings_to_device");
    assert!(probe.called("sends_irrigation_plan_settings_to_device"));
}

#[test]
#[ignore = "not implemented yet"]
fn sends_seasonal_watering_adjustment_to_device() {
    let mut probe = ConverterProbe::new();
    probe.run("sends_seasonal_watering_adjustment_to_device");
    assert!(probe.called("sends_seasonal_watering_adjustment_to_device"));
}

#[test]
#[ignore = "not implemented yet"]
fn sends_valve_alarm_settings_to_device() {
    let mut probe = ConverterProbe::new();
    probe.run("sends_valve_alarm_settings_to_device");
    assert!(probe.called("sends_valve_alarm_settings_to_device"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_combine_days_with_identical_schedules_into_a_single_command() {
    let mut probe = ConverterProbe::new();
    probe.run("should_combine_days_with_identical_schedules_into_a_single_command");
    assert!(probe.called("should_combine_days_with_identical_schedules_into_a_single_command"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_handle_all_seven_days_with_same_schedule_in_a_single_command() {
    let mut probe = ConverterProbe::new();
    probe.run("should_handle_all_seven_days_with_same_schedule_in_a_single_command");
    assert!(probe.called("should_handle_all_seven_days_with_same_schedule_in_a_single_command"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_process_multiple_days_with_different_schedules_in_separate_commands() {
    let mut probe = ConverterProbe::new();
    probe.run("should_process_multiple_days_with_different_schedules_in_separate_commands");
    assert!(
        probe.called("should_process_multiple_days_with_different_schedules_in_separate_commands")
    );
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_individual_day_schedules_with_accumulated_composite() {
    let mut probe = ConverterProbe::new();
    probe.run("should_return_individual_day_schedules_with_accumulated_composite");
    assert!(probe.called("should_return_individual_day_schedules_with_accumulated_composite"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_state_when_setting_a_single_day() {
    let mut probe = ConverterProbe::new();
    probe.run("should_return_state_when_setting_a_single_day");
    assert!(probe.called("should_return_state_when_setting_a_single_day"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_send_a_set_weekly_schedule_command_for_each_day() {
    let mut probe = ConverterProbe::new();
    probe.run("should_send_a_set_weekly_schedule_command_for_each_day");
    assert!(probe.called("should_send_a_set_weekly_schedule_command_for_each_day"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_send_set_weekly_schedule_command_if_transitions_are_valid() {
    let mut probe = ConverterProbe::new();
    probe.run("should_send_set_weekly_schedule_command_if_transitions_are_valid");
    assert!(probe.called("should_send_set_weekly_schedule_command_if_transitions_are_valid"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_send_set_weekly_schedule_command_with_transitions_in_ascending_time_order() {
    let mut probe = ConverterProbe::new();
    probe.run("should_send_set_weekly_schedule_command_with_transitions_in_ascending_time_order");
    assert!(probe.called(
        "should_send_set_weekly_schedule_command_with_transitions_in_ascending_time_order"
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn should_set_state() {
    let mut probe = ConverterProbe::new();
    probe.run("should_set_state");
    assert!(probe.called("should_set_state"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_throw_error_if_day_has_more_than_6_transitions() {
    let mut probe = ConverterProbe::new();
    probe.run("should_throw_error_if_day_has_more_than_6_transitions");
    assert!(probe.called("should_throw_error_if_day_has_more_than_6_transitions"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_throw_error_if_first_transition_does_not_start_at_00_00() {
    let mut probe = ConverterProbe::new();
    probe.run("should_throw_error_if_first_transition_does_not_start_at_00_00");
    assert!(probe.called("should_throw_error_if_first_transition_does_not_start_at_00_00"));
}

#[test]
#[ignore = "not implemented yet"]
fn should_validate_schedule_format_for_multi_day_updates() {
    let mut probe = ConverterProbe::new();
    probe.run("should_validate_schedule_format_for_multi_day_updates");
    assert!(probe.called("should_validate_schedule_format_for_multi_day_updates"));
}

#[test]
#[ignore = "not implemented yet"]
fn to_zigbee() {
    let mut probe = ConverterProbe::new();
    probe.run("to_zigbee");
    assert!(probe.called("to_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn uses_the_real_liter_amount_for_every_later_water_flow_unit_change() {
    let mut probe = ConverterProbe::new();
    probe.run("uses_the_real_liter_amount_for_every_later_water_flow_unit_change");
    assert!(probe.called("uses_the_real_liter_amount_for_every_later_water_flow_unit_change"));
}

#[test]
#[ignore = "not implemented yet"]
fn uses_the_unified_water_flow_unit_instead_of_a_stale_legacy_unit() {
    let mut probe = ConverterProbe::new();
    probe.run("uses_the_unified_water_flow_unit_instead_of_a_stale_legacy_unit");
    assert!(probe.called("uses_the_unified_water_flow_unit_instead_of_a_stale_legacy_unit"));
}

#[test]
#[ignore = "not implemented yet"]
fn weekly_schedule() {
    let mut probe = ConverterProbe::new();
    probe.run("weekly_schedule");
    assert!(probe.called("weekly_schedule"));
}

#[test]
#[ignore = "not implemented yet"]
fn when_multiple_command_get_weekly_schedule_rsp_messages_are_received_for_different_days() {
    let mut probe = ConverterProbe::new();
    probe.run(
        "when_multiple_command_get_weekly_schedule_rsp_messages_are_received_for_different_days",
    );
    assert!(probe.called(
        "when_multiple_command_get_weekly_schedule_rsp_messages_are_received_for_different_days"
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn writes_external_humidity_scaled_x100() {
    let mut probe = ConverterProbe::new();
    probe.run("writes_external_humidity_scaled_x100");
    assert!(probe.called("writes_external_humidity_scaled_x100"));
}

#[test]
#[ignore = "not implemented yet"]
fn writes_external_temperature_scaled_x100_signed() {
    let mut probe = ConverterProbe::new();
    probe.run("writes_external_temperature_scaled_x100_signed");
    assert!(probe.called("writes_external_temperature_scaled_x100_signed"));
}
