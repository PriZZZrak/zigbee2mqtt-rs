// Original: .upstream/zigbee-herdsman-converters/test/batteryState.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn battery_state_exposes() {
    let mut probe = ConverterProbe::new();
    probe.run("battery_state_exposes");
    assert!(probe.called("battery_state_exposes"));
}

#[test]
#[ignore = "not implemented yet"]
fn marks_direct_tuya_battery_state_exposes_as_diagnostic() {
    let mut probe = ConverterProbe::new();
    probe.run("marks_direct_tuya_battery_state_exposes_as_diagnostic");
    assert!(probe.called("marks_direct_tuya_battery_state_exposes_as_diagnostic"));
}

#[test]
#[ignore = "not implemented yet"]
fn marks_modern_enum_lookup_battery_state_exposes_as_diagnostic() {
    let mut probe = ConverterProbe::new();
    probe.run("marks_modern_enum_lookup_battery_state_exposes_as_diagnostic");
    assert!(probe.called("marks_modern_enum_lookup_battery_state_exposes_as_diagnostic"));
}

#[test]
#[ignore = "not implemented yet"]
fn marks_the_shared_tuya_battery_state_expose_as_diagnostic() {
    let mut probe = ConverterProbe::new();
    probe.run("marks_the_shared_tuya_battery_state_expose_as_diagnostic");
    assert!(probe.called("marks_the_shared_tuya_battery_state_expose_as_diagnostic"));
}
