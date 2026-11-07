// Original: .upstream/zigbee-herdsman-converters/test/plugwise.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn plugwise_170_01_emma() {
    let mut probe = ConverterProbe::new();
    probe.run("plugwise_170_01_emma");
    assert!(probe.called("plugwise_170_01_emma"));
}

#[test]
#[ignore = "not implemented yet"]
fn battery_type_plugwise_gen_power_cfg_mfg_attribute() {
    let mut probe = ConverterProbe::new();
    probe.run("battery_type_plugwise_gen_power_cfg_mfg_attribute");
    assert!(probe.called("battery_type_plugwise_gen_power_cfg_mfg_attribute"));
}

#[test]
#[ignore = "not implemented yet"]
fn convert_get_read_button_reads_gen_basic_product_code() {
    let mut probe = ConverterProbe::new();
    probe.run("convert_get_read_button_reads_gen_basic_product_code");
    assert!(probe.called("convert_get_read_button_reads_gen_basic_product_code"));
}

#[test]
#[ignore = "not implemented yet"]
fn convert_get_reads_battery_type_with_the_plugwise_manufacturer_code() {
    let mut probe = ConverterProbe::new();
    probe.run("convert_get_reads_battery_type_with_the_plugwise_manufacturer_code");
    assert!(probe.called("convert_get_reads_battery_type_with_the_plugwise_manufacturer_code"));
}

#[test]
#[ignore = "not implemented yet"]
fn from_zigbee_ignores_messages_without_a_product_code_attribute() {
    let mut probe = ConverterProbe::new();
    probe.run("from_zigbee_ignores_messages_without_a_product_code_attribute");
    assert!(probe.called("from_zigbee_ignores_messages_without_a_product_code_attribute"));
}

#[test]
#[ignore = "not implemented yet"]
fn manufacturer_specific_open_therm_attributes() {
    let mut probe = ConverterProbe::new();
    probe.run("manufacturer_specific_open_therm_attributes");
    assert!(probe.called("manufacturer_specific_open_therm_attributes"));
}

#[test]
#[ignore = "not implemented yet"]
fn product_variant_gen_basic_product_code_0x000_a() {
    let mut probe = ConverterProbe::new();
    probe.run("product_variant_gen_basic_product_code_0x000_a");
    assert!(probe.called("product_variant_gen_basic_product_code_0x000_a"));
}

#[test]
#[ignore = "not implemented yet"]
fn registers_a_gen_basic_from_zigbee_converter_for_product_variant() {
    let mut probe = ConverterProbe::new();
    probe.run("registers_a_gen_basic_from_zigbee_converter_for_product_variant");
    assert!(probe.called("registers_a_gen_basic_from_zigbee_converter_for_product_variant"));
}
