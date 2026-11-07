// Original: .upstream/zigbee-herdsman-converters/test/nodieby.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn no_dieby_nd_01() {
    let mut probe = ConverterProbe::new();
    probe.run("no_dieby_nd_01");
    assert!(probe.called("no_dieby_nd_01"));
}

#[test]
#[ignore = "not implemented yet"]
fn from_zigbee_messages_received_from_the_device() {
    let mut probe = ConverterProbe::new();
    probe.run("from_zigbee_messages_received_from_the_device");
    assert!(probe.called("from_zigbee_messages_received_from_the_device"));
}

#[test]
#[ignore = "not implemented yet"]
fn maps_an_ias_zone_status_change_to_occupancy() {
    let mut probe = ConverterProbe::new();
    probe.run("maps_an_ias_zone_status_change_to_occupancy");
    assert!(probe.called("maps_an_ias_zone_status_change_to_occupancy"));
}

#[test]
#[ignore = "not implemented yet"]
fn maps_custom_cluster_attribute_reports_to_the_right_exposes_on_ep1() {
    let mut probe = ConverterProbe::new();
    probe.run("maps_custom_cluster_attribute_reports_to_the_right_exposes_on_ep1");
    assert!(probe.called("maps_custom_cluster_attribute_reports_to_the_right_exposes_on_ep1"));
}

#[test]
#[ignore = "not implemented yet"]
fn maps_gen_on_off_reports_to_the_armed_ep1_and_siren_ep2_switches() {
    let mut probe = ConverterProbe::new();
    probe.run("maps_gen_on_off_reports_to_the_armed_ep1_and_siren_ep2_switches");
    assert!(probe.called("maps_gen_on_off_reports_to_the_armed_ep1_and_siren_ep2_switches"));
}

#[test]
#[ignore = "not implemented yet"]
fn reads_ias_zone_status_on_configure_so_occupancy_is_initialised_at_join() {
    let mut probe = ConverterProbe::new();
    probe.run("reads_ias_zone_status_on_configure_so_occupancy_is_initialised_at_join");
    assert!(probe.called("reads_ias_zone_status_on_configure_so_occupancy_is_initialised_at_join"));
}

#[test]
#[ignore = "not implemented yet"]
fn registers_the_custom_cluster_exactly_as_the_firmware_expects_id_type_no_manufacturer_code() {
    let mut probe = ConverterProbe::new();
    probe.run(
        "registers_the_custom_cluster_exactly_as_the_firmware_expects_id_type_no_manufacturer_code",
    );
    assert!(probe.called(
        "registers_the_custom_cluster_exactly_as_the_firmware_expects_id_type_no_manufacturer_code"
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn set_armed_state_toggles_gen_on_off_on_ep1() {
    let mut probe = ConverterProbe::new();
    probe.run("set_armed_state_toggles_gen_on_off_on_ep1");
    assert!(probe.called("set_armed_state_toggles_gen_on_off_on_ep1"));
}

#[test]
#[ignore = "not implemented yet"]
fn set_siren_state_toggles_gen_on_off_on_ep2() {
    let mut probe = ConverterProbe::new();
    probe.run("set_siren_state_toggles_gen_on_off_on_ep2");
    assert!(probe.called("set_siren_state_toggles_gen_on_off_on_ep2"));
}

#[test]
#[ignore = "not implemented yet"]
fn to_zigbee_commands_sent_to_the_device() {
    let mut probe = ConverterProbe::new();
    probe.run("to_zigbee_commands_sent_to_the_device");
    assert!(probe.called("to_zigbee_commands_sent_to_the_device"));
}
