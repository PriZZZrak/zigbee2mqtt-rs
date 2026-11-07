// Original: .upstream/zigbee-herdsman-converters/test/checkDefinition.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn check_definition() {
    let mut probe = ConverterProbe::new();
    probe.run("check_definition");
    assert!(probe.called("check_definition"));
}

#[test]
#[ignore = "not implemented yet"]
fn im6001_mpp01() {
    let mut probe = ConverterProbe::new();
    probe.run("im6001_mpp01");
    assert!(probe.called("im6001_mpp01"));
}
