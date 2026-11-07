// Original: .upstream/zigbee-herdsman-converters/test/stuckatprototype.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn base_matches_the_same_definition_hides_pro_only_values_and_configure_does_not_throw() {
    let mut probe = ConverterProbe::new();
    probe
        .run("base_matches_the_same_definition_hides_pro_only_values_and_configure_does_not_throw");
    assert!(probe.called(
        "base_matches_the_same_definition_hides_pro_only_values_and_configure_does_not_throw"
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn pro_exposes_co2_illuminance_and_configures_their_reporting() {
    let mut probe = ConverterProbe::new();
    probe.run("pro_exposes_co2_illuminance_and_configures_their_reporting");
    assert!(probe.called("pro_exposes_co2_illuminance_and_configures_their_reporting"));
}

#[test]
#[ignore = "not implemented yet"]
fn stuck_at_prototype_air_cube() {
    let mut probe = ConverterProbe::new();
    probe.run("stuck_at_prototype_air_cube");
    assert!(probe.called("stuck_at_prototype_air_cube"));
}
