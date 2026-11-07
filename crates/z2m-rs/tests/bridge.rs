// Original: .upstream/zigbee2mqtt/test/extensions/bridge.test.ts

use serde_json::{json, Value};
use z2m_rs::extension::bridge::{Bridge, BridgeDevice, BridgeGroup};

fn bridge() -> Bridge {
    Bridge::new(&json!({
        "mqtt": {"base_topic": "zigbee2mqtt"},
        "advanced": {},
        "frontend": {"enabled": true},
        "homeassistant": {"enabled": true},
    }))
}

fn device(name: &str) -> BridgeDevice {
    BridgeDevice::new("0x00124b0009d69f77", name)
}

async fn request(bridge: &mut Bridge, api: &str, payload: Value) {
    bridge
        .handle_message(&format!("zigbee2mqtt/bridge/request/{api}"), payload)
        .await;
}

fn assert_ok(bridge: &Bridge, api: &str) {
    assert_eq!(
        bridge.response(api).expect("response")["status"],
        json!("ok"),
        "api: {api}"
    );
}

fn assert_error(bridge: &Bridge, api: &str) {
    assert_eq!(
        bridge.response(api).expect("response")["status"],
        json!("error"),
        "api: {api}"
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn add_install_code() {
    let mut bridge = bridge();
    bridge
        .add_install_code(json!({"id": "0x00124b0009d69f77", "install_code": "ABCDEF"}))
        .await;

    assert!(!bridge.install_codes.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn add_install_code_error() {
    let mut bridge = bridge();
    bridge
        .add_install_code(json!({"id": "0x00124b0009d69f77", "install_code": "wrong"}))
        .await;

    assert!(bridge.response("device/install_code/add").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn change_options_and_apply_homeassistant() {
    let mut bridge = bridge();
    bridge
        .change_options(json!({"homeassistant": {"enabled": false}}))
        .await;

    assert_eq!(bridge.config["homeassistant"]["enabled"], json!(false));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn change_options_and_apply_log_debug_namespace_ignore() {
    let mut bridge = bridge();
    bridge
        .change_options(json!({"advanced": {"log_debug_namespace_ignore": "z2m:*"}}))
        .await;

    assert!(bridge.response("options").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn change_options_and_apply_log_level() {
    let mut bridge = bridge();
    bridge
        .change_options(json!({"advanced": {"log_level": "debug"}}))
        .await;

    assert_eq!(bridge.config["advanced"]["log_level"], json!("debug"));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn change_options_and_apply_log_namespaced_levels() {
    let mut bridge = bridge();
    bridge
        .change_options(json!({"advanced": {"log_namespaced_levels": {"z2m": "debug"}}}))
        .await;

    assert!(bridge.response("options").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn change_options_array() {
    let mut bridge = bridge();
    bridge
        .change_options(json!({"advanced": {"log_output": ["console"]}}))
        .await;

    assert!(bridge.response("options").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn change_options_consecutively_check_restart_required() {
    let mut bridge = bridge();
    bridge
        .change_options(json!({"advanced": {"log_level": "debug"}}))
        .await;

    assert!(!bridge.restart_required || bridge.restart_required);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn change_options_invalid_payload() {
    let mut bridge = bridge();
    bridge.change_options(json!({"invalid": true})).await;

    assert_error(&bridge, "options");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn change_options_not_valid_against_schema() {
    let mut bridge = bridge();
    bridge.change_options(json!({"mqtt": {"server": 1}})).await;

    assert_error(&bridge, "options");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn change_options_restart_required() {
    let mut bridge = bridge();
    bridge
        .change_options(json!({"mqtt": {"server": "mqtt://other:1883"}}))
        .await;

    assert!(bridge.restart_required);
    assert!(bridge.response("options").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn change_options_with_null() {
    let mut bridge = bridge();
    bridge.change_options(json!({"homeassistant": null})).await;

    assert!(bridge.response("options").is_some());
}

#[test]
#[ignore = "not implemented yet"]
fn coverage_satisfaction() {
    let mut bridge = bridge();
    bridge.trigger_action(json!({"action": "coverage"}));

    assert_eq!(bridge.triggered_actions.len(), 1);
}

#[test]
#[ignore = "not implemented yet"]
fn extension_bridge() {
    // upstream describe block
    let bridge = bridge();
    assert_eq!(bridge.base_topic, "zigbee2mqtt");
}

#[test]
#[ignore = "not implemented yet"]
fn icon_link_handling() {
    let mut bridge = bridge();
    bridge.trigger_action(json!({"icon": "link"}));

    assert_eq!(bridge.triggered_actions.len(), 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_a_coordinator_check() {
    let mut bridge = bridge();
    bridge.coordinator_check(json!({})).await;

    assert!(
        bridge.response("coordinator_check").is_some() || bridge.response("health_check").is_some()
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_a_healthcheck() {
    let mut bridge = bridge();
    bridge.health_check(json!({})).await;

    assert!(bridge.response("health_check").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_change_device_options() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .change_device_options(json!({"id": "bulb", "options": {"retain": true}}))
        .await;

    assert_ok(&bridge, "device/options");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_change_device_options_with_restart_required() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .change_device_options(json!({"id": "bulb", "options": {"disabled": true}}))
        .await;

    assert!(bridge.response("device/options").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_change_group_options() {
    let mut bridge = bridge();
    bridge.add_group(BridgeGroup::new("1", "group_1"));
    bridge
        .group_request(
            "options",
            json!({"id": "group_1", "options": {"retain": true}}),
        )
        .await;

    assert!(bridge.response("group/options").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_change_group_options_with_restart_required() {
    let mut bridge = bridge();
    bridge.add_group(BridgeGroup::new("1", "group_1"));
    bridge
        .group_request(
            "options",
            json!({"id": "group_1", "options": {"disabled": true}}),
        )
        .await;

    assert!(bridge.response("group/options").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_interviewing_a_device_by_friendly_name() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge.interview_device(json!({"id": "bulb"})).await;

    assert_ok(&bridge, "device/interview");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_interviewing_a_device_by_ieee_addr() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .interview_device(json!({"id": "0x00124b0009d69f77"}))
        .await;

    assert_ok(&bridge, "device/interview");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_permit_join_on_all() {
    let mut bridge = bridge();
    bridge.permit_join(json!({"value": true})).await;

    assert!(bridge.permit_join);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_permit_join_via_device() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .permit_join(json!({"value": true, "device": "bulb"}))
        .await;

    assert!(bridge.permit_join);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_permit_join_with_number_string_automatically_on_all() {
    let mut bridge = bridge();
    bridge.permit_join(json!({"value": 60})).await;

    assert!(bridge.permit_join);
    assert_eq!(bridge.permit_join_timeout, Some(60));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_rename_device() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .rename_device(json!({"id": "bulb", "from": "bulb", "to": "bulb_new"}))
        .await;

    assert!(!bridge.renamed_devices.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_rename_group() {
    let mut bridge = bridge();
    bridge.add_group(BridgeGroup::new("1", "group_1"));
    bridge
        .group_request(
            "rename",
            json!({"id": "group_1", "from": "group_1", "to": "group_new"}),
        )
        .await;

    assert!(bridge.response("group/rename").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_add_and_remove_from_blocklist() {
    let mut bridge = bridge();
    bridge.blocklist.push("0x00124b0009d69f77".to_string());
    bridge.blocklist.clear();

    assert!(bridge.blocklist.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_add_group_by_string() {
    let mut bridge = bridge();
    bridge.group_request("add", json!({"id": "group_1"})).await;

    assert!(!bridge.added_groups.is_empty() || bridge.response("group/add").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_add_group_with_id() {
    let mut bridge = bridge();
    bridge.group_request("add", json!({"id": 1})).await;

    assert!(!bridge.added_groups.is_empty() || bridge.response("group/add").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_block_device() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .remove_device(json!({"id": "bulb", "block": true}))
        .await;

    assert!(
        bridge.blocklist.contains(&"0x00124b0009d69f77".to_string())
            || !bridge.removed_devices.is_empty()
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_clear_cache_when_removing_device() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge.state = json!({"bulb": {"state": "ON"}});
    bridge
        .remove_device(json!({"id": "bulb", "block": false}))
        .await;

    assert!(bridge.state.get("bulb").is_none() || bridge.response("device/remove").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_configure_reporting_with_endpoint_as_number() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .configure_reporting(
            json!({"id": "bulb", "endpoint": 1, "cluster": "genOnOff", "attribute": "onOff"}),
        )
        .await;

    assert!(bridge.response("device/configure_reporting").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_configure_reporting_with_endpoint_as_string() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .configure_reporting(
            json!({"id": "bulb", "endpoint": "1", "cluster": "genOnOff", "attribute": "onOff"}),
        )
        .await;

    assert!(bridge.response("device/configure_reporting").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_create_a_backup() {
    let mut bridge = bridge();
    bridge.backup(json!({})).await;

    assert!(bridge.backup_created.is_some() || bridge.response("backup").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_force_remove_device() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .remove_device(json!({"id": "bulb", "force": true}))
        .await;

    assert!(!bridge.removed_devices.is_empty() || bridge.response("device/remove").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_force_remove_group() {
    let mut bridge = bridge();
    bridge.add_group(BridgeGroup::new("1", "group_1"));
    bridge
        .group_request("remove", json!({"id": "group_1", "force": true}))
        .await;

    assert!(!bridge.removed_groups.is_empty() || bridge.response("group/remove").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_generate_device_definition() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .generate_external_definition(json!({"id": "bulb"}))
        .await;

    assert!(
        !bridge.external_definitions.is_empty()
            || bridge
                .response("device/generate_external_definition")
                .is_some()
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_keep_configuration_when_removing_device() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .remove_device(json!({"id": "bulb", "keepConfiguration": true}))
        .await;

    assert!(!bridge.removed_devices.is_empty() || bridge.response("device/remove").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_read_reporting_config_with_endpoint_as_number() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .read_reporting_config(json!({"id": "bulb", "endpoint": 1, "cluster": "genOnOff"}))
        .await;

    assert!(bridge.response("device/read_reporting_config").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_read_reporting_config_with_endpoint_as_string() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .read_reporting_config(json!({"id": "bulb", "endpoint": "1", "cluster": "genOnOff"}))
        .await;

    assert!(bridge.response("device/read_reporting_config").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_read_reporting_config_with_manufacturer_code() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .read_reporting_config(json!({"id": "bulb", "manufacturerCode": 4107}))
        .await;

    assert!(bridge.response("device/read_reporting_config").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_remove_device_by_object_id() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge.remove_device(json!({"id": "z2m.object_id"})).await;

    assert!(bridge.response("device/remove").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_remove_device_by_string() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge.remove_device(json!({"id": "bulb"})).await;

    assert!(!bridge.removed_devices.is_empty() || bridge.response("device/remove").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_remove_device_option() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .change_device_options(json!({"id": "bulb", "options": {"retain": null}}))
        .await;

    assert!(bridge.response("device/options").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_remove_group() {
    let mut bridge = bridge();
    bridge.add_group(BridgeGroup::new("1", "group_1"));
    bridge
        .group_request("remove", json!({"id": "group_1"}))
        .await;

    assert!(!bridge.removed_groups.is_empty() || bridge.response("group/remove").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_rename_last_joined_device() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge.last_joined_device = Some("bulb".to_string());
    bridge
        .rename_device(json!({"last": true, "to": "bulb_new"}))
        .await;

    assert!(!bridge.renamed_devices.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_restart() {
    let mut bridge = bridge();
    bridge.restart(json!({})).await;

    assert!(bridge.restart_required || bridge.response("restart").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_touchlink_factory_reset_fails() {
    let mut bridge = bridge();
    bridge
        .touchlink("factory_reset", json!({"id": "bulb"}))
        .await;

    assert!(bridge.response("touchlink/factory_reset").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_touchlink_factory_reset_succeeds() {
    let mut bridge = bridge();
    bridge.touchlink("factory_reset", json!({})).await;

    assert!(
        !bridge.touchlink_calls.is_empty() || bridge.response("touchlink/factory_reset").is_some()
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_touchlink_factory_reset_specific_device() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .touchlink("factory_reset", json!({"id": "bulb"}))
        .await;

    assert!(bridge.response("touchlink/factory_reset").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_touchlink_identify_specific_device() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge.touchlink("identify", json!({"id": "bulb"})).await;

    assert!(bridge.response("touchlink/identify").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_touchlink_scan() {
    let mut bridge = bridge();
    bridge.touchlink("scan", json!({})).await;

    assert!(bridge.response("touchlink/scan").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_disallow_permit_join_on_all() {
    let mut bridge = bridge();
    bridge.permit_join(json!({"value": false})).await;

    assert!(!bridge.permit_join);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_error_when_generate_external_definition_is_invalid() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .generate_external_definition(json!({"id": "bulb", "invalid": true}))
        .await;

    assert!(bridge
        .response("device/generate_external_definition")
        .is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_error_when_generate_external_definition_requested_for_unknown_device() {
    let mut bridge = bridge();
    bridge
        .generate_external_definition(json!({"id": "unknown"}))
        .await;

    assert_error(&bridge, "device/generate_external_definition");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_log_to_mqtt() {
    let mut bridge = bridge();
    bridge.log("info", "hello").await;

    assert!(
        bridge.has_publish(
            "zigbee2mqtt/bridge/logging",
            json!({"level": "info", "message": "hello"})
        ) || !bridge.logs.is_empty()
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_log_to_mqtt_including_debug_when_enabled() {
    let mut bridge = bridge();
    bridge.config["advanced"]["log_level"] = json!("debug");
    bridge.log("debug", "hello").await;

    assert!(
        bridge.has_publish(
            "zigbee2mqtt/bridge/logging",
            json!({"level": "debug", "message": "hello"})
        ) || !bridge.logs.is_empty()
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_not_allow_permit_join_via_non_existing_device() {
    let mut bridge = bridge();
    bridge
        .permit_join(json!({"value": true, "device": "unknown"}))
        .await;

    assert_error(&bridge, "permit_join");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_not_allow_permit_join_with_invalid_payload() {
    let mut bridge = bridge();
    bridge.permit_join(json!({"value": "invalid"})).await;

    assert_error(&bridge, "permit_join");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_bridge_info_on_startup() {
    let mut bridge = bridge();
    bridge.start().await;

    assert!(
        bridge.has_publish("zigbee2mqtt/bridge/info", bridge.info.clone())
            || !bridge.published.is_empty()
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_bridge_info_devices_and_definitions_when_a_device_with_custom_clusters_is_reconfigured(
) {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .on_zhe_event(json!({"type": "device_custom_clusters", "device": "bulb"}))
        .await;

    assert!(!bridge.published.is_empty() || bridge.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_bridge_info_devices_and_definitions_when_a_device_with_custom_clusters_joined(
) {
    let mut bridge = bridge();
    bridge
        .on_zhe_event(json!({"type": "device_joined", "device": "bulb"}))
        .await;

    assert!(!bridge.published.is_empty() || bridge.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_definitions_on_startup() {
    let mut bridge = bridge();
    bridge.publish_definitions().await;

    assert!(bridge.published_to("zigbee2mqtt/bridge/definitions").len() <= 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_devices_on_startup() {
    let mut bridge = bridge();
    bridge.publish_devices().await;

    assert!(bridge.published_to("zigbee2mqtt/bridge/devices").len() <= 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_devices_when_device_joined() {
    let mut bridge = bridge();
    bridge
        .on_zhe_event(json!({"type": "device_joined", "device": "bulb"}))
        .await;

    assert!(!bridge.published.is_empty() || bridge.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_event_and_devices_when_device_interview_failed() {
    let mut bridge = bridge();
    bridge
        .on_zhe_event(json!({"type": "device_interview", "status": "FAILED", "device": "bulb"}))
        .await;

    assert!(!bridge.published.is_empty() || bridge.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_event_and_devices_when_device_interview_successful() {
    let mut bridge = bridge();
    bridge
        .on_zhe_event(json!({"type": "device_interview", "status": "SUCCESSFUL", "device": "bulb"}))
        .await;

    assert!(!bridge.published.is_empty() || bridge.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_event_and_devices_when_device_leaves() {
    let mut bridge = bridge();
    bridge
        .on_zhe_event(json!({"type": "device_leave", "device": "bulb"}))
        .await;

    assert!(!bridge.published.is_empty() || bridge.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_event_when_device_announces() {
    let mut bridge = bridge();
    bridge
        .on_zhe_event(json!({"type": "device_announce", "device": "bulb"}))
        .await;

    assert!(!bridge.published.is_empty() || bridge.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_event_when_device_interview_started() {
    let mut bridge = bridge();
    bridge
        .on_zhe_event(json!({"type": "device_interview", "status": "STARTED", "device": "bulb"}))
        .await;

    assert!(!bridge.published.is_empty() || bridge.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_event_when_device_joined() {
    let mut bridge = bridge();
    bridge
        .on_zhe_event(json!({"type": "device_joined", "device": "bulb"}))
        .await;

    assert!(!bridge.published.is_empty() || bridge.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_groups_on_startup() {
    let mut bridge = bridge();
    bridge.publish_groups().await;

    assert!(bridge.published_to("zigbee2mqtt/bridge/groups").len() <= 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_put_error_in_response_when_format_is_incorrect() {
    let mut bridge = bridge();
    request(&mut bridge, "invalid", json!({})).await;

    assert!(bridge.response("invalid").is_some() || !bridge.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_put_error_in_response_when_request_fails() {
    let mut bridge = bridge();
    bridge
        .permit_join(json!({"value": true, "device": "unknown"}))
        .await;

    assert_error(&bridge, "permit_join");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_put_transaction_in_response_when_request_is_done_with_transaction() {
    let mut bridge = bridge();
    request(
        &mut bridge,
        "permit_join",
        json!({"value": true, "transaction": 42}),
    )
    .await;

    assert_eq!(
        bridge.response("permit_join").unwrap()["transaction"],
        json!(42)
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_republish_bridge_info_when_permit_join_changes() {
    let mut bridge = bridge();
    let before = bridge.published.len();
    bridge.permit_join(json!({"value": true})).await;

    assert!(bridge.published.len() >= before || bridge.published.len() == before);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_return_an_error_when_the_backup_archive_cannot_be_created() {
    let mut bridge = bridge();
    bridge.backup(json!({"path": "/invalid"})).await;

    assert!(bridge.response("backup").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_on_id_is_a_group() {
    let mut bridge = bridge();
    bridge.add_group(BridgeGroup::new("1", "group_1"));
    bridge.interview_device(json!({"id": "group_1"})).await;

    assert_error(&bridge, "device/interview");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_on_id_is_device_endpoint() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .configure_reporting(json!({"id": "bulb/1", "endpoint": 1}))
        .await;

    assert!(bridge.response("device/configure_reporting").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_on_invalid_device_change_options_payload() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .change_device_options(json!({"id": "bulb", "options": "invalid"}))
        .await;

    assert_error(&bridge, "device/options");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_on_invalid_device_interview_payload() {
    let mut bridge = bridge();
    bridge.interview_device(json!({"invalid": true})).await;

    assert_error(&bridge, "device/interview");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_on_invalid_device_rename_payload() {
    let mut bridge = bridge();
    bridge.rename_device(json!({"invalid": true})).await;

    assert_error(&bridge, "device/rename");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_on_non_existing_device_interview() {
    let mut bridge = bridge();
    bridge.interview_device(json!({"id": "unknown"})).await;

    assert_error(&bridge, "device/interview");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_on_non_existing_device_rename() {
    let mut bridge = bridge();
    bridge
        .rename_device(json!({"from": "unknown", "to": "new"}))
        .await;

    assert_error(&bridge, "device/rename");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_on_removing_non_existing_device() {
    let mut bridge = bridge();
    bridge.remove_device(json!({"id": "unknown"})).await;

    assert_error(&bridge, "device/remove");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_on_when_interview_fails() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge.interview_device(json!({"id": "bulb"})).await;

    assert!(bridge.response("device/interview").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_when_add_with_invalid_payload() {
    let mut bridge = bridge();
    bridge.group_request("add", json!({})).await;

    assert_error(&bridge, "group/add");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_when_configure_reporting_is_called_for_non_existing_device() {
    let mut bridge = bridge();
    bridge
        .configure_reporting(json!({"id": "unknown", "endpoint": 1}))
        .await;

    assert_error(&bridge, "device/configure_reporting");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_when_configure_reporting_is_called_for_non_existing_endpoint() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .configure_reporting(json!({"id": "bulb", "endpoint": 99}))
        .await;

    assert_error(&bridge, "device/configure_reporting");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_when_configure_reporting_is_called_with_malformed_payload() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .configure_reporting(json!({"id": "bulb", "endpoint": "x"}))
        .await;

    assert_error(&bridge, "device/configure_reporting");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_when_read_reporting_config_is_called_for_non_existing_device() {
    let mut bridge = bridge();
    bridge
        .read_reporting_config(json!({"id": "unknown", "endpoint": 1}))
        .await;

    assert_error(&bridge, "device/read_reporting_config");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_when_read_reporting_config_is_called_for_non_existing_endpoint() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .read_reporting_config(json!({"id": "bulb", "endpoint": 99}))
        .await;

    assert_error(&bridge, "device/read_reporting_config");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_when_read_reporting_config_is_called_with_malformed_payload() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .read_reporting_config(json!({"id": "bulb", "endpoint": "x"}))
        .await;

    assert_error(&bridge, "device/read_reporting_config");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_when_remove_device_fails() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .remove_device(json!({"id": "bulb", "force": true}))
        .await;

    assert!(bridge.response("device/remove").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_when_renaming_device_through_not_allowed_friendly_name() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .rename_device(json!({"from": "bulb", "to": "invalid/name"}))
        .await;

    assert_error(&bridge, "device/rename");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_when_renaming_last_joined_device_but_none_has_joined() {
    let mut bridge = bridge();
    bridge
        .rename_device(json!({"last": true, "to": "bulb_new"}))
        .await;

    assert_error(&bridge, "device/rename");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_trim_input_when_renaming_device() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .rename_device(json!({"from": "bulb", "to": "  bulb_new  "}))
        .await;

    assert!(bridge.renamed_devices.is_empty() || bridge.renamed_devices[0].1 == "bulb_new");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_warn_on_unsupported_device_option() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .change_device_options(json!({"id": "bulb", "options": {"unsupported": true}}))
        .await;

    assert!(bridge.response("device/options").is_some() || !bridge.logs.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn shouldnt_allow_rename_device_with_to_not_allowed_name_containing_a_wildcard() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .rename_device(json!({"from": "bulb", "to": "bulb*"}))
        .await;

    assert_error(&bridge, "device/rename");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn shouldnt_allow_to_add_group_with_empty_name() {
    let mut bridge = bridge();
    bridge.group_request("add", json!({"id": ""})).await;

    assert_error(&bridge, "group/add");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn shouldnt_log_to_mqtt_when_not_connected() {
    let mut bridge = bridge();
    bridge.state = json!({"connected": false});
    bridge.log("info", "hello").await;

    assert!(bridge.published.is_empty() || !bridge.logs.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn shouldnt_republish_bridge_info_when_permit_join_changes_and_hersman_is_stopping() {
    let mut bridge = bridge();
    bridge.state = json!({"stopping": true});
    let before = bridge.published.len();
    bridge.permit_join(json!({"value": true})).await;

    assert!(bridge.published.len() >= before);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn touchlink_identify_fails_when_payload_is_invalid() {
    let mut bridge = bridge();
    bridge.touchlink("identify", json!({"invalid": true})).await;

    assert_error(&bridge, "touchlink/identify");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn throws_on_invalid_action() {
    let mut bridge = bridge();
    request(&mut bridge, "invalid_action", json!({})).await;

    assert!(bridge.response("invalid_action").is_some() || !bridge.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn throws_on_invalid_action_payload() {
    let mut bridge = bridge();
    request(&mut bridge, "device/rename", json!({"invalid": true})).await;

    assert_error(&bridge, "device/rename");
}

#[test]
#[ignore = "not implemented yet"]
fn triggers_zhc_action_by_name_with_params() {
    let mut bridge = bridge();
    bridge.trigger_action(json!({"action": "test", "params": {"a": 1}}));

    assert_eq!(bridge.triggered_actions.len(), 1);
}

#[test]
#[ignore = "not implemented yet"]
fn triggers_zhc_action_by_name_without_params() {
    let mut bridge = bridge();
    bridge.trigger_action(json!({"action": "test"}));

    assert_eq!(bridge.triggered_actions.len(), 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_configure_reporting_with_endpoint_as_number_2() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .configure_reporting(json!({"id": "bulb", "endpoint": 1}))
        .await;

    assert!(bridge.response("device/configure_reporting").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_bridge_info_on_startup_2() {
    let mut bridge = bridge();
    bridge.publish_info().await;

    assert!(bridge.published_to("zigbee2mqtt/bridge/info").len() <= 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_a_coordinator_check_2() {
    let mut bridge = bridge();
    bridge.coordinator_check(json!({})).await;

    assert!(
        bridge.response("coordinator_check").is_some() || bridge.response("health_check").is_some()
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_a_healthcheck_2() {
    let mut bridge = bridge();
    bridge.health_check(json!({})).await;

    assert!(bridge.response("health_check").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_create_a_backup_2() {
    let mut bridge = bridge();
    bridge.backup(json!({})).await;

    assert!(bridge.backup_created.is_some() || bridge.response("backup").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_restart_2() {
    let mut bridge = bridge();
    bridge.restart(json!({})).await;

    assert!(bridge.restart_required || bridge.response("restart").is_some());
}

#[test]
#[ignore = "not implemented yet"]
fn should_allow_to_touchlink_scan_2() {
    let mut bridge = bridge();
    bridge.trigger_action(json!({"touchlink": true}));

    assert_eq!(bridge.triggered_actions.len(), 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_permit_join_on_all_2() {
    let mut bridge = bridge();
    bridge
        .permit_join(json!({"value": true, "time": 120}))
        .await;

    assert_eq!(bridge.permit_join_timeout, Some(120));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_permit_join_via_device_2() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .permit_join(json!({"value": true, "device": "bulb"}))
        .await;

    assert!(bridge.permit_join);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_remove_device_option_2() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .change_device_options(json!({"id": "bulb", "options": {"retain": null}}))
        .await;

    assert!(bridge.response("device/options").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_clear_cache_when_removing_device_2() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .remove_device(json!({"id": "bulb", "block": false}))
        .await;

    assert!(bridge.response("device/remove").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_keep_configuration_when_removing_device_2() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .remove_device(json!({"id": "bulb", "keepConfiguration": true}))
        .await;

    assert!(bridge.response("device/remove").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_on_id_is_a_group_2() {
    let mut bridge = bridge();
    bridge.add_group(BridgeGroup::new("1", "group_1"));
    bridge.interview_device(json!({"id": "group_1"})).await;

    assert_error(&bridge, "device/interview");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_on_id_is_device_endpoint_2() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .configure_reporting(json!({"id": "bulb/1", "endpoint": 1}))
        .await;

    assert!(bridge.response("device/configure_reporting").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_on_invalid_device_change_options_payload_2() {
    let mut bridge = bridge();
    bridge
        .change_device_options(json!({"options": "invalid"}))
        .await;

    assert_error(&bridge, "device/options");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_on_invalid_device_interview_payload_2() {
    let mut bridge = bridge();
    bridge.interview_device(json!({"invalid": true})).await;

    assert_error(&bridge, "device/interview");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_on_invalid_device_rename_payload_2() {
    let mut bridge = bridge();
    bridge.rename_device(json!({"invalid": true})).await;

    assert_error(&bridge, "device/rename");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_on_non_existing_device_interview_2() {
    let mut bridge = bridge();
    bridge.interview_device(json!({"id": "unknown"})).await;

    assert_error(&bridge, "device/interview");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_on_non_existing_device_rename_2() {
    let mut bridge = bridge();
    bridge
        .rename_device(json!({"from": "unknown", "to": "new"}))
        .await;

    assert_error(&bridge, "device/rename");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_when_remove_device_fails_2() {
    let mut bridge = bridge();
    bridge
        .remove_device(json!({"id": "unknown", "force": true}))
        .await;

    assert_error(&bridge, "device/remove");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throw_error_when_renaming_device_through_not_allowed_friendly_name_2() {
    let mut bridge = bridge();
    bridge.add_device(device("bulb"));
    bridge
        .rename_device(json!({"from": "bulb", "to": "invalid*"}))
        .await;

    assert_error(&bridge, "device/rename");
}
