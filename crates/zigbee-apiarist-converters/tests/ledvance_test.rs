// Original: .upstream/zigbee-herdsman-converters/test/ledvance.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn silences_all_six_divisor_multiplier_constants() {
    let mut probe = ConverterProbe::new();
    probe.run("silences_all_six_divisor_multiplier_constants");
    assert!(probe.called("silences_all_six_divisor_multiplier_constants"));
}

#[test]
#[ignore = "not implemented yet"]
fn still_configures_the_measurements_when_silencing_fails() {
    let mut probe = ConverterProbe::new();
    probe.run("still_configures_the_measurements_when_silencing_fails");
    assert!(probe.called("still_configures_the_measurements_when_silencing_fails"));
}
