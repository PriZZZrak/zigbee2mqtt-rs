// Original: .upstream/zigbee-herdsman-converters/test/color.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):

use zigbee_apiarist_converters::probe::ConverterProbe;

#[test]
#[ignore = "not implemented yet"]
fn color() {
    let mut probe = ConverterProbe::new();
    probe.run("color");
    assert!(probe.called("color"));
}

#[test]
#[ignore = "not implemented yet"]
fn color_hsv() {
    let mut probe = ConverterProbe::new();
    probe.run("color_hsv");
    assert!(probe.called("color_hsv"));
}

#[test]
#[ignore = "not implemented yet"]
fn color_rgb() {
    let mut probe = ConverterProbe::new();
    probe.run("color_rgb");
    assert!(probe.called("color_rgb"));
}

#[test]
#[ignore = "not implemented yet"]
fn color_xy() {
    let mut probe = ConverterProbe::new();
    probe.run("color_xy");
    assert!(probe.called("color_xy"));
}

#[test]
#[ignore = "not implemented yet"]
fn lib_color() {
    let mut probe = ConverterProbe::new();
    probe.run("lib_color");
    assert!(probe.called("lib_color"));
}
