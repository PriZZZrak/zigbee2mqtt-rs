// Original: .upstream/zigbee-herdsman-converters/test/findByDevice.bench.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn find_by_device() {
    let mut probe = ConverterProbe::new();
    probe.run("find_by_device");
    assert!(probe.called("find_by_device"));
}
