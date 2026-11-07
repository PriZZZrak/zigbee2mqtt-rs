// Original: .upstream/zigbee-herdsman-converters/test/schneider_electric.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn schneider_electric_s520567() {
    let mut probe = ConverterProbe::new();
    probe.run("schneider_electric_s520567");
    assert!(probe.called("schneider_electric_s520567"));
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_derive_state_from_a_tilt_report() {
    let mut probe = ConverterProbe::new();
    probe.run("does_not_derive_state_from_a_tilt_report");
    assert!(probe.called("does_not_derive_state_from_a_tilt_report"));
}

#[test]
#[ignore = "not implemented yet"]
fn reports_state_alongside_position_for_a_lift_percentage_report() {
    let mut probe = ConverterProbe::new();
    probe.run("reports_state_alongside_position_for_a_lift_percentage_report");
    assert!(probe.called("reports_state_alongside_position_for_a_lift_percentage_report"));
}

#[test]
#[ignore = "not implemented yet"]
fn reports_state_on_a_read_response_as_well_as_an_attribute_report() {
    let mut probe = ConverterProbe::new();
    probe.run("reports_state_on_a_read_response_as_well_as_an_attribute_report");
    assert!(probe.called("reports_state_on_a_read_response_as_well_as_an_attribute_report"));
}
