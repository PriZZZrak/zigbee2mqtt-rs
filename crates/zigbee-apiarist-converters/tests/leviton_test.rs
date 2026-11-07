// Original: .upstream/zigbee-herdsman-converters/test/leviton.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn leviton_dg3_hl_1_bw() {
    let mut probe = ConverterProbe::new();
    probe.run("leviton_dg3_hl_1_bw");
    assert!(probe.called("leviton_dg3_hl_1_bw"));
}

#[test]
#[ignore = "not implemented yet"]
fn allows_the_tested_power_on_level_value_255() {
    let mut probe = ConverterProbe::new();
    probe.run("allows_the_tested_power_on_level_value_255");
    assert!(probe.called("allows_the_tested_power_on_level_value_255"));
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_only_the_supported_level_and_ballast_configuration() {
    let mut probe = ConverterProbe::new();
    probe.run("exposes_only_the_supported_level_and_ballast_configuration");
    assert!(probe.called("exposes_only_the_supported_level_and_ballast_configuration"));
}
