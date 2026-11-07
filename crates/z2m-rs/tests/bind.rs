// Original: .upstream/zigbee2mqtt/test/extensions/bind.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Error bind fails when source device
//   - Error bind fails when source device does not exist
//   - Error bind fails when target device
//   - Error bind fails when target device or group does not exist
//   - Extension: Bind
//   - Filters out reporting setup based on bind status
//   - Filters out unsupported clusters for reporting setup
//   - Should allow to bind to coordinator by ieeeAddr
//   - Should bind from non default endpoint IDs
//   - Should bind from non default endpoint names
//   - Should bind only specified clusters
//   - Should bind server clusters to client clusters
//   - Should bind to default endpoint returned by endpoints()
//   - Should bind to device and configure reporting
//   - Should bind to group by number
//   - Should bind to groups
//   - Should log error when there is nothing to bind
//   - Should log when bind fails
//   - Should poll bounded Hue bulb when receiving message from Hue dimmer
//   - Should poll bounded Hue bulb when receiving message from scene controller
//   - Should poll grouped Hue bulb when receiving message from TRADFRI remote
//   - Should throw error on invalid payload
//   - Should unbind
//   - Should unbind coordinator
//   - Should unbind from default_bind_group
//   - Should unbind from group
//   - Should unbind from group with skip_disable_reporting=false
//   - Should unbind from group with skip_disable_reporting=true
//   - clears all bindings
//   - clears targeted bindings
//   - throw on invalid clears bindings payload
//   - throw on invalid clears bindings target

use serde_json::json;
use z2m_rs::extension::bind::Bind;

fn setup() -> Bind {
    let mut bind = Bind::new();
    bind.add_device("remote", "0x0017880104e45517", &["default", "1", "2"]);
    bind.add_device("bulb_color", "0x000b57fffec6a5b3", &["default", "1"]);
    bind.add_device("bulb", "0x000b57fffec6a5b2", &["default", "1"]);
    bind.add_group("group_1");
    bind
}

fn has(bind: &Bind, topic: &str, payload: serde_json::Value) -> bool {
    bind.published
        .contains(&(topic.to_string(), payload, json!({})))
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn error_bind_fails_when_source_device() {
    let mut bind = setup();
    bind.published.clear();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"from": "remote", "from_endpoint": "not_existing_endpoint", "to": "bulb_color"}),
    );

    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({
            "data": {},
            "status": "error",
            "error": "Source device 'remote' does not have endpoint 'not_existing_endpoint'",
        })
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn error_bind_fails_when_source_device_does_not_exist() {
    let mut bind = setup();
    bind.published.clear();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"from": "remote_not_existing", "to": "bulb_color"}),
    );

    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({
            "data": {},
            "status": "error",
            "error": "Source device 'remote_not_existing' does not exist",
        })
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn error_bind_fails_when_target_device() {
    let mut bind = setup();
    bind.published.clear();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"from": "remote", "to": "bulb_color", "to_endpoint": "not_existing_endpoint"}),
    );

    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({
            "data": {},
            "status": "error",
            "error": "Target device 'bulb_color' does not have endpoint 'not_existing_endpoint'",
        })
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn error_bind_fails_when_target_device_or_group_does_not_exist() {
    let mut bind = setup();
    bind.published.clear();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"from": "remote", "to": "bulb_color_not_existing"}),
    );

    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({
            "data": {},
            "status": "error",
            "error": "Target device or group 'bulb_color_not_existing' does not exist",
        })
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn filters_out_reporting_setup_based_on_bind_status() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn filters_out_unsupported_clusters_for_reporting_setup() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_allow_to_bind_to_coordinator_by_ieee_addr() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_bind_from_non_default_endpoint_ids() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_bind_from_non_default_endpoint_names() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_bind_only_specified_clusters() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_bind_server_clusters_to_client_clusters() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_bind_to_default_endpoint_returned_by_endpoints() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_bind_to_device_and_configure_reporting() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_bind_to_group_by_number() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_bind_to_groups() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_log_error_when_there_is_nothing_to_bind() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_log_when_bind_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_poll_bounded_hue_bulb_when_receiving_message_from_hue_dimmer() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_poll_bounded_hue_bulb_when_receiving_message_from_scene_controller() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_poll_grouped_hue_bulb_when_receiving_message_from_tradfri_remote() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_on_invalid_payload() {
    let mut bind = setup();
    bind.published.clear();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/bind",
        &json!({"fromz": "remote", "to": "bulb_color"}),
    );

    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/bind",
        json!({"data": {}, "status": "error", "error": "Invalid payload"})
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn should_unbind() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_unbind_coordinator() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_unbind_from_default_bind_group() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_unbind_from_group() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_unbind_from_group_with_skip_disable_reporting_false() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_unbind_from_group_with_skip_disable_reporting_true() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn clears_all_bindings() {
    let mut bind = setup();
    bind.published.clear();
    bind.devices_changed = 0;

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/binds/clear",
        &json!({"transaction": "1234", "target": "remote"}),
    );

    assert_eq!(bind.clear_bindings_calls.get("remote").copied(), Some(1));
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/binds/clear",
        json!({
            "transaction": "1234",
            "data": {
                "target": "remote",
                "ieee_list": ["0xffffffffffffffff"],
            },
            "status": "ok",
        })
    ));
    assert!(bind
        .published
        .iter()
        .any(|(topic, _, _)| topic == "zigbee2mqtt/bridge/devices"));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn clears_targeted_bindings() {
    let mut bind = setup();
    bind.published.clear();
    bind.devices_changed = 0;

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/binds/clear",
        &json!({
            "transaction": "1234",
            "target": "remote",
            "ieee_list": ["0x000b57fffec6a5b3"],
        }),
    );

    assert_eq!(bind.clear_bindings_calls.get("remote").copied(), Some(1));
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/binds/clear",
        json!({
            "transaction": "1234",
            "data": {
                "target": "remote",
                "ieee_list": ["0x000b57fffec6a5b3"],
            },
            "status": "ok",
        })
    ));
    assert!(bind
        .published
        .iter()
        .any(|(topic, _, _)| topic == "zigbee2mqtt/bridge/devices"));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn throw_on_invalid_clears_bindings_payload() {
    let mut bind = setup();
    bind.published.clear();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/binds/clear",
        &json!({"targetz": "remote"}),
    );

    assert!(bind.clear_bindings_calls.is_empty());
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/binds/clear",
        json!({"data": {}, "status": "error", "error": "Invalid payload"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn throw_on_invalid_clears_bindings_target() {
    let mut bind = setup();
    bind.published.clear();

    bind.on_message(
        "zigbee2mqtt/bridge/request/device/binds/clear",
        &json!({"target": "remotez"}),
    );

    assert!(bind.clear_bindings_calls.is_empty());
    assert!(has(
        &bind,
        "zigbee2mqtt/bridge/response/device/binds/clear",
        json!({"data": {}, "status": "error", "error": "Invalid target"})
    ));
}
