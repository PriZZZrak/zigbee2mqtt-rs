// Original: .upstream/zigbee-herdsman-converters/test/checks.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn check_definitions() {
    let mut probe = ConverterProbe::new();
    probe.run("check_definitions");
    assert!(probe.called("check_definitions"));
}

#[test]
#[ignore = "not implemented yet"]
fn check_if_all_exposes_have_a_color_temp_range() {
    let mut probe = ConverterProbe::new();
    probe.run("check_if_all_exposes_have_a_color_temp_range");
    assert!(probe.called("check_if_all_exposes_have_a_color_temp_range"));
}

#[test]
#[ignore = "not implemented yet"]
fn definition_version() {
    let mut probe = ConverterProbe::new();
    probe.run("definition_version");
    assert!(probe.called("definition_version"));
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_access_matches_to_zigbee() {
    let mut probe = ConverterProbe::new();
    probe.run("exposes_access_matches_to_zigbee");
    assert!(probe.called("exposes_access_matches_to_zigbee"));
}

#[test]
#[ignore = "not implemented yet"]
fn exposes_properties_are_unique() {
    let mut probe = ConverterProbe::new();
    probe.run("exposes_properties_are_unique");
    assert!(probe.called("exposes_properties_are_unique"));
}

#[test]
#[ignore = "not implemented yet"]
fn model_should_be_unique() {
    let mut probe = ConverterProbe::new();
    probe.run("model_should_be_unique");
    assert!(probe.called("model_should_be_unique"));
}

#[test]
#[ignore = "not implemented yet"]
fn number_exposes_with_set_access_should_have_a_range() {
    let mut probe = ConverterProbe::new();
    probe.run("number_exposes_with_set_access_should_have_a_range");
    assert!(probe.called("number_exposes_with_set_access_should_have_a_range"));
}

#[test]
#[ignore = "not implemented yet"]
fn vendor_name_casing_should_be_consistent() {
    let mut probe = ConverterProbe::new();
    probe.run("vendor_name_casing_should_be_consistent");
    assert!(probe.called("vendor_name_casing_should_be_consistent"));
}

#[test]
#[ignore = "not implemented yet"]
fn respect_snake_case_naming_conventions_for_exposes_and_options() {
    let mut probe = ConverterProbe::new();
    probe.run("respect_snake_case_naming_conventions_for_exposes_and_options");
    assert!(probe.called("respect_snake_case_naming_conventions_for_exposes_and_options"));
}
