// Original: .upstream/zigbee-herdsman-converters/test/lumi.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Feeder schedule
//   - PS-S04D battery
//   - RTCZCGQ11LM configured regions
//   - SJCGQ12LM IAS enrollment
//   - SP-EUC01 event mode
//   - Schedule 0 days
//   - Schedule 1 day
//   - Too small frame
//   - UT-A01E NTC sensor
//   - WP-P09D optional exposes
//   - WXKG01LM hold length
//   - ZNCLBL01LM terminal position readback
//   - ZNYB01LM bathroom heater
//   - converts empty string to empty schedule object
//   - converts human-readable string pattern to schedule object
//   - converts schedule object to buffer
//   - converts schedule object to human-readable string pattern
//   - converts undefined value to empty schedule object
//   - decodes battery percentage (tag 24) and voltage (tag 23) from the 0x00F7 struct
//   - decodes heartbeat in normal operation
//   - decodes heartbeat in setup mode
//   - decodes packed state from the Lumi heartbeat
//   - decodes private configuration attributes
//   - decodes setup mode
//   - decodes the values reported before and after a power cycle
//   - decodes user preset
//   - decodes valve alarm
//   - drops only the exposes that are turned off on lumiElectricityMeter
//   - drops only the exposes that are turned off on lumiOnOff
//   - enables event mode during configure
//   - encodes the beta in the 2 least significant bytes of a float32 close to the beta
//   - enrolls the device when it reports zoneState 0
//   - fails if any individual duration is less than 1 hour
//   - fails if minimum total duration is more than 24 hours
//   - fails on empty events
//   - fails on insufficient number of events
//   - fails on invalid event temperature value
//   - fails on invalid event time type
//   - fails when the device stays unenrolled
//   - handles invalid time pattern
//   - is used by the W500
//   - keeps device_temperature and power_outage_count on lumiOnOff by default
//   - keeps energy, voltage and current on lumiElectricityMeter by default
//   - leaves a short press without a duration at all
//   - leaves an already enrolled device alone
//   - lib/lumi
//   - lumiChildLock
//   - preserves fan speed when changing swing mode
//   - preserves mute end time when changing mute start time
//   - preserves swing mode when changing fan speed
//   - preserves the night-light schedule when toggling it
//   - reads current temperature from the packed state
//   - reads final position only when changing to stopped
//   - reads private state during configuration
//   - reads schedule object from buffer
//   - records when the 0x00F7 struct was last received, keeping the battery poll dormant while fresh
//   - rejects invalid mute time
//   - rejects invalid values
//   - reports the hold length under action_duration, which is not part of the device state
//   - returns configured_regions after a region delete
//   - returns configured_regions after a region upsert
//   - still publishes the full lumi_specific payload when an expose is turned off
//   - trv
//   - uses the standard LOCK/UNLOCK values on attribute 0x0285
//   - writes a custom sensor
//   - writes target temperature using the Aqara packed command
//   - writes the resistance and beta of a preset

#[test]
#[ignore = "not implemented yet"]
fn feeder_schedule() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ps_s04_d_battery() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn rtczcgq11_lm_configured_regions() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn sjcgq12_lm_ias_enrollment() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn sp_euc01_event_mode() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn schedule_0_days() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn schedule_1_day() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn too_small_frame() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ut_a01_e_ntc_sensor() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn wp_p09_d_optional_exposes() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn wxkg01_lm_hold_length() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znclbl01_lm_terminal_position_readback() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn znyb01_lm_bathroom_heater() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn converts_empty_string_to_empty_schedule_object() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn converts_human_readable_string_pattern_to_schedule_object() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn converts_schedule_object_to_buffer() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn converts_schedule_object_to_human_readable_string_pattern() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn converts_undefined_value_to_empty_schedule_object() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn decodes_battery_percentage_tag_24_and_voltage_tag_23_from_the_0x00_f7_struct() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn decodes_heartbeat_in_normal_operation() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn decodes_heartbeat_in_setup_mode() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn decodes_packed_state_from_the_lumi_heartbeat() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn decodes_private_configuration_attributes() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn decodes_setup_mode() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn decodes_the_values_reported_before_and_after_a_power_cycle() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn decodes_user_preset() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn decodes_valve_alarm() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn drops_only_the_exposes_that_are_turned_off_on_lumi_electricity_meter() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn drops_only_the_exposes_that_are_turned_off_on_lumi_on_off() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn enables_event_mode_during_configure() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn encodes_the_beta_in_the_2_least_significant_bytes_of_a_float32_close_to_the_beta() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn enrolls_the_device_when_it_reports_zone_state_0() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_if_any_individual_duration_is_less_than_1_hour() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_if_minimum_total_duration_is_more_than_24_hours() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_on_empty_events() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_on_insufficient_number_of_events() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_on_invalid_event_temperature_value() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_on_invalid_event_time_type() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn fails_when_the_device_stays_unenrolled() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_invalid_time_pattern() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn is_used_by_the_w500() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_device_temperature_and_power_outage_count_on_lumi_on_off_by_default() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_energy_voltage_and_current_on_lumi_electricity_meter_by_default() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn leaves_a_short_press_without_a_duration_at_all() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn leaves_an_already_enrolled_device_alone() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn lib_lumi() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn lumi_child_lock() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn preserves_fan_speed_when_changing_swing_mode() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn preserves_mute_end_time_when_changing_mute_start_time() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn preserves_swing_mode_when_changing_fan_speed() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn preserves_the_night_light_schedule_when_toggling_it() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_current_temperature_from_the_packed_state() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_final_position_only_when_changing_to_stopped() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_private_state_during_configuration() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reads_schedule_object_from_buffer() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn records_when_the_0x00_f7_struct_was_last_received_keeping_the_battery_poll_dormant_while_fresh()
{
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn rejects_invalid_mute_time() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn rejects_invalid_values() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn reports_the_hold_length_under_action_duration_which_is_not_part_of_the_device_state() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn returns_configured_regions_after_a_region_delete() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn returns_configured_regions_after_a_region_upsert() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn still_publishes_the_full_lumi_specific_payload_when_an_expose_is_turned_off() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn trv() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn uses_the_standard_lock_unlock_values_on_attribute_0x0285() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_a_custom_sensor() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_target_temperature_using_the_aqara_packed_command() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn writes_the_resistance_and_beta_of_a_preset() {
    todo!()
}
