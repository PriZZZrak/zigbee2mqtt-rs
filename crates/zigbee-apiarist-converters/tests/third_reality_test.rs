// Original: .upstream/zigbee-herdsman-converters/test/third_reality.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn t_3_rsm0147_z_falls_back_to_relative_humidity_cluster_for_soil_moisture_on_older_hardware() {
    let mut probe = ConverterProbe::new();
    probe.run(
        "t_3_rsm0147_z_falls_back_to_relative_humidity_cluster_for_soil_moisture_on_older_hardware",
    );
    assert!(probe.called(
        "t_3_rsm0147_z_falls_back_to_relative_humidity_cluster_for_soil_moisture_on_older_hardware"
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn t_3_rsm0147_z_uses_native_soil_moisture_cluster_when_present() {
    let mut probe = ConverterProbe::new();
    probe.run("t_3_rsm0147_z_uses_native_soil_moisture_cluster_when_present");
    assert!(probe.called("t_3_rsm0147_z_uses_native_soil_moisture_cluster_when_present"));
}

#[test]
#[ignore = "not implemented yet"]
fn third_reality_soil_moisture_sensors() {
    let mut probe = ConverterProbe::new();
    probe.run("third_reality_soil_moisture_sensors");
    assert!(probe.called("third_reality_soil_moisture_sensors"));
}
