// Original: .upstream/zigbee-herdsman-converters/test/sonoff.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Sonoff SNZB-02DR2
//   - Sonoff SWV
//   - Sonoff SWV-ZFE
//   - Sonoff TRVZB
//   - continues configuring when optional water shortage auto-close attribute is unsupported
//   - disables the external display via temperature_sensor_select
//   - does not clear a legacy amount unit when setting the same unit
//   - does not clear a legacy amount unit when the reported unit is unchanged
//   - enables the external display via temperature_sensor_select
//   - exposes manual irrigation settings as scalar controls
//   - fills missing scalar settings with defaults on first write
//   - fromZigbee
//   - hides manual amount unit on firmware with unified water-flow units
//   - merges a scalar manual setting with current scalar state
//   - multi-day batch updates via meta.message
//   - preserves the real liter amount when an OTA-updated device first reports its unified unit
//   - removes an irrigation plan through the composite remove command
//   - sends irrigation plan settings to device
//   - sends seasonal watering adjustment to device
//   - sends valve alarm settings to device
//   - should combine days with identical schedules into a single command
//   - should handle all seven days with same schedule in a single command
//   - should process multiple days with different schedules in separate commands
//   - should return individual day schedules with accumulated composite
//   - should return state when setting a single day
//   - should send a setWeeklySchedule command for each day
//   - should send setWeeklySchedule command if transitions are valid
//   - should send setWeeklySchedule command with transitions in ascending time order
//   - should set state
//   - should throw error if day has more than 6 transitions
//   - should throw error if first transition does not start at 00:00
//   - should validate schedule format for multi-day updates
//   - toZigbee
//   - uses the real liter amount for every later water-flow unit change
//   - uses the unified water-flow unit instead of a stale legacy unit
//   - weekly schedule
//   - when multiple commandGetWeeklyScheduleRsp messages are received for different days
//   - writes external humidity scaled x100
//   - writes external temperature scaled x100 (signed)

#[test]
#[ignore = "not implemented yet"]
fn sonoff_snzb_02_dr2() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn sonoff_swv() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn sonoff_swv_zfe() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn sonoff_trvzb() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn continues_configuring_when_optional_water_shortage_auto_close_attribute_is_unsupported() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn disables_the_external_display_via_temperature_sensor_select() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_clear_a_legacy_amount_unit_when_setting_the_same_unit() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_clear_a_legacy_amount_unit_when_the_reported_unit_is_unchanged() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn enables_the_external_display_via_temperature_sensor_select() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_manual_irrigation_settings_as_scalar_controls() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fills_missing_scalar_settings_with_defaults_on_first_write() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn from_zigbee() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn hides_manual_amount_unit_on_firmware_with_unified_water_flow_units() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn merges_a_scalar_manual_setting_with_current_scalar_state() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn multi_day_batch_updates_via_meta_message() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn preserves_the_real_liter_amount_when_an_ota_updated_device_first_reports_its_unified_unit() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn removes_an_irrigation_plan_through_the_composite_remove_command() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn sends_irrigation_plan_settings_to_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn sends_seasonal_watering_adjustment_to_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn sends_valve_alarm_settings_to_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_combine_days_with_identical_schedules_into_a_single_command() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_handle_all_seven_days_with_same_schedule_in_a_single_command() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_process_multiple_days_with_different_schedules_in_separate_commands() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_individual_day_schedules_with_accumulated_composite() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_state_when_setting_a_single_day() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_send_a_set_weekly_schedule_command_for_each_day() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_send_set_weekly_schedule_command_if_transitions_are_valid() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_send_set_weekly_schedule_command_with_transitions_in_ascending_time_order() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_set_state() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_throw_error_if_day_has_more_than_6_transitions() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_throw_error_if_first_transition_does_not_start_at_00_00() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_validate_schedule_format_for_multi_day_updates() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn to_zigbee() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uses_the_real_liter_amount_for_every_later_water_flow_unit_change() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uses_the_unified_water_flow_unit_instead_of_a_stale_legacy_unit() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn weekly_schedule() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn when_multiple_command_get_weekly_schedule_rsp_messages_are_received_for_different_days() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_external_humidity_scaled_x100() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_external_temperature_scaled_x100_signed() {
    todo!()
}
