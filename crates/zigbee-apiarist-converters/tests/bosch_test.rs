// Original: .upstream/zigbee-herdsman-converters/test/bosch.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn bosch_room_thermostat_ii_230_v_home_assistant_climate_modes() {
    let mut probe = ConverterProbe::new();
    probe.run("bosch_room_thermostat_ii_230_v_home_assistant_climate_modes");
    assert!(probe.called("bosch_room_thermostat_ii_230_v_home_assistant_climate_modes"));
}

#[test]
#[ignore = "not implemented yet"]
fn bosch_thermostat_relay_state_extension() {
    let mut probe = ConverterProbe::new();
    probe.run("bosch_thermostat_relay_state_extension");
    assert!(probe.called("bosch_thermostat_relay_state_extension"));
}

#[test]
#[ignore = "not implemented yet"]
fn bosch_thermostats() {
    let mut probe = ConverterProbe::new();
    probe.run("bosch_thermostats");
    assert!(probe.called("bosch_thermostats"));
}

#[test]
#[ignore = "not implemented yet"]
fn can_expose_cool_only_home_assistant_climate_discovery() {
    let mut probe = ConverterProbe::new();
    probe.run("can_expose_cool_only_home_assistant_climate_discovery");
    assert!(probe.called("can_expose_cool_only_home_assistant_climate_discovery"));
}

#[test]
#[ignore = "not implemented yet"]
fn can_expose_heat_and_cool_home_assistant_climate_discovery() {
    let mut probe = ConverterProbe::new();
    probe.run("can_expose_heat_and_cool_home_assistant_climate_discovery");
    assert!(probe.called("can_expose_heat_and_cool_home_assistant_climate_discovery"));
}

#[test]
#[ignore = "not implemented yet"]
fn defaults_to_heat_only_home_assistant_climate_discovery() {
    let mut probe = ConverterProbe::new();
    probe.run("defaults_to_heat_only_home_assistant_climate_discovery");
    assert!(probe.called("defaults_to_heat_only_home_assistant_climate_discovery"));
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_expose_or_configure_the_relay_when_the_device_does_not_support_gen_on_off() {
    let mut probe = ConverterProbe::new();
    probe.run("does_not_expose_or_configure_the_relay_when_the_device_does_not_support_gen_on_off");
    assert!(probe.called(
        "does_not_expose_or_configure_the_relay_when_the_device_does_not_support_gen_on_off"
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_expose_weekly_schedule_controls_for_bth_rm() {
    let mut probe = ConverterProbe::new();
    probe.run("does_not_expose_weekly_schedule_controls_for_bth_rm");
    assert!(probe.called("does_not_expose_weekly_schedule_controls_for_bth_rm"));
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_and_configures_the_relay_when_the_device_supports_gen_on_off() {
    let mut probe = ConverterProbe::new();
    probe.run("exposes_and_configures_the_relay_when_the_device_supports_gen_on_off");
    assert!(probe.called("exposes_and_configures_the_relay_when_the_device_supports_gen_on_off"));
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_standard_weekly_schedule_controls_for_bth_rm230_z() {
    let mut probe = ConverterProbe::new();
    probe.run("exposes_standard_weekly_schedule_controls_for_bth_rm230_z");
    assert!(probe.called("exposes_standard_weekly_schedule_controls_for_bth_rm230_z"));
}
