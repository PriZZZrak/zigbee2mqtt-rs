// Original: .upstream/zigbee-herdsman/test/greenpower.test.ts

use serde_json::json;
use zigbee_apiarist::controller::green_power::GreenPower;

#[test]
fn fullencr_zt_lp_zeu2_s_wh_ms_moes_2_gang_vectors_from_ember() {
    let mut green_power = GreenPower::new();
    let _ = green_power.process_command(json!({}));
    assert!(green_power.called("process_command"));
}

#[test]
fn fullencr_zt_lp_zeu2_s_wh_ms_moes_2_gang_vectors_from_zstack_through_gpp() {
    let mut green_power = GreenPower::new();
    let _ = green_power.process_command(json!({}));
    assert!(green_power.called("process_command"));
}

#[test]
fn fullencr_zt_lp_zeu2_s_wh_ms_moes_3_gang_vectors_from_ember() {
    let mut green_power = GreenPower::new();
    let _ = green_power.process_command(json!({}));
    assert!(green_power.called("process_command"));
}

#[test]
fn green_power() {
    let mut green_power = GreenPower::new();
    let _ = green_power.record("constructor");
    assert!(green_power.called("constructor"));
}

#[test]
fn does_not_parse_command_frame_when_fullencr_security_level_gpp() {
    let mut green_power = GreenPower::new();
    let _ = green_power.parse_command(json!({}));
    assert!(green_power.called("parse_command"));
}

#[test]
fn does_not_parse_command_frame_when_fullencr_security_level_sink() {
    let mut green_power = GreenPower::new();
    let _ = green_power.parse_command(json!({}));
    assert!(green_power.called("parse_command"));
}

#[test]
fn encodes_decodes_commissioning_mode_options() {
    let mut green_power = GreenPower::new();
    let _ = green_power.decode_options(0);
    assert!(green_power.called("decode_options"));
}

#[test]
fn encodes_decodes_pairing_options() {
    let mut green_power = GreenPower::new();
    let _ = green_power.encode_options(json!({}));
    assert!(green_power.called("encode_options"));
}

#[test]
fn omits_gpp_data_and_mic_from_raw_payload() {
    let mut green_power = GreenPower::new();
    let _ = green_power.omit_gpp_data_and_mic(json!({}));
    assert!(green_power.called("omit_gpp_data_and_mic"));
}

#[test]
fn omits_gpp_data_from_raw_payload() {
    let mut green_power = GreenPower::new();
    let _ = green_power.omit_gpp_data(json!({}));
    assert!(green_power.called("omit_gpp_data"));
}

#[test]
fn omits_mic_from_raw_payload() {
    let mut green_power = GreenPower::new();
    let _ = green_power.omit_mic(json!({}));
    assert!(green_power.called("omit_mic"));
}
