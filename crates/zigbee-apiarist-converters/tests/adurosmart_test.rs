// Original: .upstream/zigbee-herdsman-converters/test/adurosmart.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn aduro_smart_eria_definitions() {
    let mut probe = ConverterProbe::new();
    probe.run("aduro_smart_eria_definitions");
    assert!(probe.called("aduro_smart_eria_definitions"));
}

#[test]
#[ignore = "not implemented yet"]
fn configures_the_expected_reporting_intervals_for_both_multi_sensors() {
    let mut probe = ConverterProbe::new();
    probe.run("configures_the_expected_reporting_intervals_for_both_multi_sensors");
    assert!(probe.called("configures_the_expected_reporting_intervals_for_both_multi_sensors"));
}

#[test]
#[ignore = "not implemented yet"]
fn covers_the_complete_aduro_smart_eria_product_inventory() {
    let mut probe = ConverterProbe::new();
    probe.run("covers_the_complete_aduro_smart_eria_product_inventory");
    assert!(probe.called("covers_the_complete_aduro_smart_eria_product_inventory"));
}

#[test]
#[ignore = "not implemented yet"]
fn decodes_all_three_axes_from_the_5_in_1_contact_sensor() {
    let mut probe = ConverterProbe::new();
    probe.run("decodes_all_three_axes_from_the_5_in_1_contact_sensor");
    assert!(probe.called("decodes_all_three_axes_from_the_5_in_1_contact_sensor"));
}

#[test]
#[ignore = "not implemented yet"]
fn decodes_and_binds_the_manufacturer_specific_dimmer_and_scene_remote_commands() {
    let mut probe = ConverterProbe::new();
    probe.run("decodes_and_binds_the_manufacturer_specific_dimmer_and_scene_remote_commands");
    assert!(probe
        .called("decodes_and_binds_the_manufacturer_specific_dimmer_and_scene_remote_commands"));
}
