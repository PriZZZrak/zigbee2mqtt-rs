// Original: .upstream/zigbee-herdsman-converters/test/onesti.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn onesti_products_as_locks() {
    let mut probe = ConverterProbe::new();
    probe.run("onesti_products_as_locks");
    assert!(probe.called("onesti_products_as_locks"));
}

#[test]
#[ignore = "not implemented yet"]
fn pin_code_format() {
    let mut probe = ConverterProbe::new();
    probe.run("pin_code_format");
    assert!(probe.called("pin_code_format"));
}

#[test]
#[ignore = "not implemented yet"]
fn accepts_an_array_of_bytes_as_well_as_a_buffer() {
    let mut probe = ConverterProbe::new();
    probe.run("accepts_an_array_of_bytes_as_well_as_a_buffer");
    assert!(probe.called("accepts_an_array_of_bytes_as_well_as_a_buffer"));
}

#[test]
#[ignore = "not implemented yet"]
fn decodes_a_pin_sent_as_ascii_digits() {
    let mut probe = ConverterProbe::new();
    probe.run("decodes_a_pin_sent_as_ascii_digits");
    assert!(probe.called("decodes_a_pin_sent_as_ascii_digits"));
}

#[test]
#[ignore = "not implemented yet"]
fn decodes_a_pin_sent_as_packed_bcd() {
    let mut probe = ConverterProbe::new();
    probe.run("decodes_a_pin_sent_as_packed_bcd");
    assert!(probe.called("decodes_a_pin_sent_as_packed_bcd"));
}

#[test]
#[ignore = "not implemented yet"]
fn decodes_a_keypad_unlock_with_a_user_slot() {
    let mut probe = ConverterProbe::new();
    probe.run("decodes_a_keypad_unlock_with_a_user_slot");
    assert!(probe.called("decodes_a_keypad_unlock_with_a_user_slot"));
}

#[test]
#[ignore = "not implemented yet"]
fn decodes_an_auto_relock() {
    let mut probe = ConverterProbe::new();
    probe.run("decodes_an_auto_relock");
    assert!(probe.called("decodes_an_auto_relock"));
}

#[test]
#[ignore = "not implemented yet"]
fn decodes_source_0x05_as_unattributed_instead_of_unknown() {
    let mut probe = ConverterProbe::new();
    probe.run("decodes_source_0x05_as_unattributed_instead_of_unknown");
    assert!(probe.called("decodes_source_0x05_as_unattributed_instead_of_unknown"));
}

#[test]
#[ignore = "not implemented yet"]
fn definitions() {
    let mut probe = ConverterProbe::new();
    probe.run("definitions");
    assert!(probe.called("definitions"));
}

#[test]
#[ignore = "not implemented yet"]
fn falls_back_to_hex_when_the_bytes_are_neither_ascii_digits_nor_bcd() {
    let mut probe = ConverterProbe::new();
    probe.run("falls_back_to_hex_when_the_bytes_are_neither_ascii_digits_nor_bcd");
    assert!(probe.called("falls_back_to_hex_when_the_bytes_are_neither_ascii_digits_nor_bcd"));
}

#[test]
#[ignore = "not implemented yet"]
fn ignores_trailing_nul_padding_which_trim_does_not_remove() {
    let mut probe = ConverterProbe::new();
    probe.run("ignores_trailing_nul_padding_which_trim_does_not_remove");
    assert!(probe.called("ignores_trailing_nul_padding_which_trim_does_not_remove"));
}

#[test]
#[ignore = "not implemented yet"]
fn keeps_min_and_max_pin_length_apart() {
    let mut probe = ConverterProbe::new();
    probe.run("keeps_min_and_max_pin_length_apart");
    assert!(probe.called("keeps_min_and_max_pin_length_apart"));
}

#[test]
#[ignore = "not implemented yet"]
fn last_action_source_and_user() {
    let mut probe = ConverterProbe::new();
    probe.run("last_action_source_and_user");
    assert!(probe.called("last_action_source_and_user"));
}

#[test]
#[ignore = "not implemented yet"]
fn lock_capabilities() {
    let mut probe = ConverterProbe::new();
    probe.run("lock_capabilities");
    assert!(probe.called("lock_capabilities"));
}

#[test]
#[ignore = "not implemented yet"]
fn publishes_nothing_when_the_message_holds_nothing_it_handles() {
    let mut probe = ConverterProbe::new();
    probe.run("publishes_nothing_when_the_message_holds_nothing_it_handles");
    assert!(probe.called("publishes_nothing_when_the_message_holds_nothing_it_handles"));
}

#[test]
#[ignore = "not implemented yet"]
fn publishes_the_auto_relock_time() {
    let mut probe = ConverterProbe::new();
    probe.run("publishes_the_auto_relock_time");
    assert!(probe.called("publishes_the_auto_relock_time"));
}

#[test]
#[ignore = "not implemented yet"]
fn publishes_the_capability_attributes_zigbee_herdsman_keys_by_name() {
    let mut probe = ConverterProbe::new();
    probe.run("publishes_the_capability_attributes_zigbee_herdsman_keys_by_name");
    assert!(probe.called("publishes_the_capability_attributes_zigbee_herdsman_keys_by_name"));
}

#[test]
#[ignore = "not implemented yet"]
fn reads_the_capabilities_of_a_nimly_lock_on_configure() {
    let mut probe = ConverterProbe::new();
    probe.run("reads_the_capabilities_of_a_nimly_lock_on_configure");
    assert!(probe.called("reads_the_capabilities_of_a_nimly_lock_on_configure"));
}

#[test]
#[ignore = "not implemented yet"]
fn reads_the_user_slot_as_16_bits() {
    let mut probe = ConverterProbe::new();
    probe.run("reads_the_user_slot_as_16_bits");
    assert!(probe.called("reads_the_user_slot_as_16_bits"));
}

#[test]
#[ignore = "not implemented yet"]
fn reads_two_bcd_bytes_as_four_digits_not_as_two_ascii_digits() {
    let mut probe = ConverterProbe::new();
    probe.run("reads_two_bcd_bytes_as_four_digits_not_as_two_ascii_digits");
    assert!(probe.called("reads_two_bcd_bytes_as_four_digits_not_as_two_ascii_digits"));
}

#[test]
#[ignore = "not implemented yet"]
fn reports_an_unmapped_source_as_unknown() {
    let mut probe = ConverterProbe::new();
    probe.run("reports_an_unmapped_source_as_unknown");
    assert!(probe.called("reports_an_unmapped_source_as_unknown"));
}

#[test]
#[ignore = "not implemented yet"]
fn uses_the_minimum_pin_length_the_lock_reported_to_tell_the_formats_apart() {
    let mut probe = ConverterProbe::new();
    probe.run("uses_the_minimum_pin_length_the_lock_reported_to_tell_the_formats_apart");
    assert!(probe.called("uses_the_minimum_pin_length_the_lock_reported_to_tell_the_formats_apart"));
}
