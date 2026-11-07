// Original: .upstream/zigbee-herdsman-converters/test/miele.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn miele_km6839() {
    let mut probe = ConverterProbe::new();
    probe.run("miele_km6839");
    assert!(probe.called("miele_km6839"));
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_aggregate_and_per_zone_properties() {
    let mut probe = ConverterProbe::new();
    probe.run("exposes_aggregate_and_per_zone_properties");
    assert!(probe.called("exposes_aggregate_and_per_zone_properties"));
}

#[test]
#[ignore = "not implemented yet"]
fn matches_the_private_miele_profile_endpoints() {
    let mut probe = ConverterProbe::new();
    probe.run("matches_the_private_miele_profile_endpoints");
    assert!(probe.called("matches_the_private_miele_profile_endpoints"));
}
