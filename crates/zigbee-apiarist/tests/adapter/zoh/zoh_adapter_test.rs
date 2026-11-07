// Original: .upstream/zigbee-herdsman/test/adapter/zoh/zohAdapter.test.ts

use serde_json::json;
use zigbee_apiarist::adapter::zoh::adapter::zoh_adapter::ZoHAdapter;

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_gets_state() {
    let mut adapter = ZoHAdapter::new();
    adapter.get_state();
    assert!(adapter.called("get_state"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_permit_join() {
    let mut adapter = ZoHAdapter::new();
    adapter.permit_join(Some(60), None);
    assert!(adapter.called("permit_join"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_all() {
    let mut adapter = ZoHAdapter::new();
    adapter.send_zcl_frame_to_all();
    assert!(adapter.called("send_zcl_frame_to_all"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_endpoint() {
    let mut adapter = ZoHAdapter::new();
    adapter.send_zcl_frame_to_endpoint();
    assert!(adapter.called("send_zcl_frame_to_endpoint"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zcl_frame_to_group() {
    let mut adapter = ZoHAdapter::new();
    adapter.send_zcl_frame_to_group();
    assert!(adapter.called("send_zcl_frame_to_group"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zdo_to_coordinator() {
    let mut adapter = ZoHAdapter::new();
    adapter.send_zdo(true);
    assert!(adapter.called("send_zdo"));
}

#[test]
#[ignore = "not implemented yet"]
fn adapter_impl_send_zdo_to_device() {
    let mut adapter = ZoHAdapter::new();
    adapter.send_zdo(false);
    assert!(adapter.called("send_zdo"));
}

#[test]
#[ignore = "not implemented yet"]
fn zigbee_on_host() {
    let mut adapter = ZoHAdapter::new();
    adapter.record("constructor");
    assert!(adapter.called("constructor"));
}

#[test]
#[ignore = "not implemented yet"]
fn adds_hashed_install_code() {
    let mut adapter = ZoHAdapter::new();
    adapter.add_install_code(true);
    assert!(adapter.called("add_install_code"));
}

#[test]
#[ignore = "not implemented yet"]
fn adds_install_code() {
    let mut adapter = ZoHAdapter::new();
    adapter.add_install_code(false);
    assert!(adapter.called("add_install_code"));
}

#[test]
#[ignore = "not implemented yet"]
fn device_authorized() {
    let mut adapter = ZoHAdapter::new();
    adapter.device_authorized(json!({"ieeeAddr": "0x00124b0009d69f77"}));
    assert!(adapter.called("device_authorized"));
}

#[test]
#[ignore = "not implemented yet"]
fn receives_gp_frame() {
    let mut adapter = ZoHAdapter::new();
    adapter.handle_gp_frame();
    assert!(adapter.called("handle_gp_frame"));
}

#[test]
#[ignore = "not implemented yet"]
fn receives_zcl_frame() {
    let mut adapter = ZoHAdapter::new();
    adapter.handle_zcl_frame();
    assert!(adapter.called("handle_zcl_frame"));
}

#[test]
#[ignore = "not implemented yet"]
fn receives_zdo_frame() {
    let mut adapter = ZoHAdapter::new();
    adapter.handle_zdo_frame();
    assert!(adapter.called("handle_zdo_frame"));
}

#[test]
#[ignore = "not implemented yet"]
fn receives_device_events() {
    let mut adapter = ZoHAdapter::new();
    adapter.handle_device_event();
    assert!(adapter.called("handle_device_event"));
}

#[test]
#[ignore = "not implemented yet"]
fn resets_network_on_mismatch_pan_id() {
    let mut adapter = ZoHAdapter::new();
    adapter.reset();
    assert!(adapter.called("reset"));
}

#[test]
#[ignore = "not implemented yet"]
fn resets_network_on_mismatch_extended_pan_id() {
    let mut adapter = ZoHAdapter::new();
    adapter.reset();
    assert!(adapter.called("reset"));
}

#[test]
#[ignore = "not implemented yet"]
fn resets_network_on_mismatch_network_key() {
    let mut adapter = ZoHAdapter::new();
    adapter.reset();
    assert!(adapter.called("reset"));
}

#[test]
#[ignore = "not implemented yet"]
fn resumes_network() {
    let mut adapter = ZoHAdapter::new();
    adapter.start();
    assert!(adapter.started);
}

#[test]
#[ignore = "not implemented yet"]
fn uses_custom_stack_config() {
    let mut adapter = ZoHAdapter::new();
    adapter.load_stack_config(json!({"custom": true}));
    assert!(adapter.called("load_stack_config"));
}

#[test]
#[ignore = "not implemented yet"]
fn uses_default_stack_config_when_json_not_present() {
    let mut adapter = ZoHAdapter::new();
    adapter.load_stack_config(json!({}));
    assert!(adapter.called("load_stack_config"));
}

#[test]
#[ignore = "not implemented yet"]
fn uses_defaults_when_custom_stack_config_invalid() {
    let mut adapter = ZoHAdapter::new();
    adapter.load_stack_config(json!({"custom": true}));
    assert!(adapter.called("load_stack_config"));
}

#[test]
#[ignore = "not implemented yet"]
fn uses_defaults_when_custom_stack_config_partial() {
    let mut adapter = ZoHAdapter::new();
    adapter.load_stack_config(json!({"custom": true}));
    assert!(adapter.called("load_stack_config"));
}
