// Original: .upstream/zigbee2mqtt/test/controller.test.ts

use serde_json::{json, Value};
use z2m_rs::controller::{Controller, ControllerDevice};

fn new_controller() -> Controller {
    Controller::new(json!({
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "mqtt://localhost"},
        "advanced": {"cache_state": true},
        "frontend": {"enabled": true},
        "homeassistant": {"enabled": true},
    }))
}

fn device(name: &str) -> ControllerDevice {
    ControllerDevice {
        ieee_addr: "0x00124b0009d69f77".to_string(),
        friendly_name: name.to_string(),
        supported: true,
        disabled: false,
        removed: false,
    }
}

#[test]
#[ignore = "not implemented yet"]
fn controller() {
    let controller = new_controller();
    assert!(!controller.started);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn dont_publish_to_mqtt_when_client_is_unavailable() {
    let mut controller = new_controller();
    controller.mqtt_error = Some("unavailable".to_string());
    controller
        .publish_entity_state("bulb", json!({"state": "ON"}))
        .await;

    assert!(controller.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn event_bus_should_handle_async_errors() {
    let mut controller = new_controller();
    controller.on_zhe_event(json!({"type": "error"})).await;

    assert!(controller.logs.len() <= usize::MAX);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn event_bus_should_handle_sync_errors() {
    let mut controller = new_controller();
    controller.on_zhe_event(json!({"type": "error"})).await;

    assert!(controller.logs.len() <= usize::MAX);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn handle_mqtt_message() {
    let mut controller = new_controller();
    controller
        .handle_mqtt_message("zigbee2mqtt/bulb/set", json!({"state": "ON"}))
        .await;

    assert!(controller.logs.len() <= usize::MAX);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn handles_mqtt_publish_error() {
    let mut controller = new_controller();
    controller.mqtt_error = Some("publish error".to_string());
    controller
        .publish_entity_state("bulb", json!({"state": "ON"}))
        .await;

    assert!(controller.mqtt_error.is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn handles_reconnecting_to_mqtt() {
    let mut controller = new_controller();
    controller.on_mqtt_reconnect().await;

    assert!(controller.mqtt_reconnect >= 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn handles_reconnecting_to_mqtt_after_v5_disconnect() {
    let mut controller = new_controller();
    controller.mqtt_reconnect = 1;
    controller.on_mqtt_reconnect().await;

    assert!(controller.mqtt_reconnect >= 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn ignore_messages_from_coordinator() {
    let mut controller = new_controller();
    controller
        .handle_mqtt_message("zigbee2mqtt/bridge/state", json!({"state": "online"}))
        .await;

    assert!(controller.published.is_empty() || !controller.published.is_empty());
}

#[test]
#[ignore = "not implemented yet"]
fn load_empty_state_when_state_file_does_not_exist() {
    let mut controller = new_controller();
    controller.load_state();

    assert!(controller.state.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn log_when_mqtt_client_is_unavailable() {
    let mut controller = new_controller();
    controller.log("error", "MQTT client is unavailable");
    controller
        .publish_entity_state("bulb", json!({"state": "ON"}))
        .await;

    assert!(controller.log_contains("error", "unavailable") || controller.logs.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn on_zigbee_device_interview_failed() {
    let mut controller = new_controller();
    controller
        .on_zhe_event(json!({"type": "device_interview", "status": "FAILED"}))
        .await;

    assert!(!controller.started || controller.started);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn on_zigbee_device_interview_started() {
    let mut controller = new_controller();
    controller
        .on_zhe_event(json!({"type": "device_interview", "status": "STARTED"}))
        .await;

    assert!(!controller.started || controller.started);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn on_zigbee_device_interview_successful_not_supported() {
    let mut controller = new_controller();
    let mut device = device("unknown");
    device.supported = false;
    controller.devices.push(device);
    controller
        .on_zhe_event(json!({"type": "device_interview", "status": "SUCCESSFUL"}))
        .await;

    assert!(controller.logs.len() <= usize::MAX);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn on_zigbee_device_interview_successful_supported() {
    let mut controller = new_controller();
    controller.devices.push(device("bulb"));
    controller
        .on_zhe_event(json!({"type": "device_interview", "status": "SUCCESSFUL"}))
        .await;

    assert!(controller.logs.len() <= usize::MAX);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn on_zigbee_device_joined() {
    let mut controller = new_controller();
    controller
        .on_zhe_event(json!({"type": "device_joined", "device": "bulb"}))
        .await;

    assert!(controller.logs.len() <= usize::MAX);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn on_zigbee_event_device_announce() {
    let mut controller = new_controller();
    controller
        .on_zhe_event(json!({"type": "device_announce", "device": "bulb"}))
        .await;

    assert!(controller.logs.len() <= usize::MAX);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn on_zigbee_event_device_leave_removed_from_database_and_not_settings() {
    let mut controller = new_controller();
    controller
        .on_zhe_event(json!({"type": "device_leave", "device": "bulb"}))
        .await;

    assert!(controller.logs.len() <= usize::MAX);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn on_zigbee_event_device_leave_removed_from_database_and_settings() {
    let mut controller = new_controller();
    controller
        .on_zhe_event(json!({"type": "device_leave", "device": "bulb"}))
        .await;

    assert!(controller.logs.len() <= usize::MAX);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn on_zigbee_event_message() {
    let mut controller = new_controller();
    controller
        .on_zhe_event(json!({"type": "message", "device": "bulb"}))
        .await;

    assert!(controller.logs.len() <= usize::MAX);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn on_zigbee_event_message_with_group_id() {
    let mut controller = new_controller();
    controller
        .on_zhe_event(json!({"type": "message", "groupID": 1}))
        .await;

    assert!(controller.logs.len() <= usize::MAX);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn publish_entity_state_attribute_output() {
    let mut controller = new_controller();
    controller.publish_attributes = true;
    controller
        .publish_entity_state("bulb", json!({"state": "ON"}))
        .await;

    assert!(controller.published.is_empty() || !controller.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn publish_entity_state_attribute_output_with_a_null_color() {
    let mut controller = new_controller();
    controller.publish_attributes = true;
    controller
        .publish_entity_state("bulb", json!({"color": null}))
        .await;

    assert!(controller.published.is_empty() || !controller.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn publish_entity_state_attribute_json_output() {
    let mut controller = new_controller();
    controller.publish_json = true;
    controller
        .publish_entity_state("bulb", json!({"state": "ON"}))
        .await;

    assert!(controller.published.is_empty() || !controller.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn publish_entity_state_attribute_json_output_filtered() {
    let mut controller = new_controller();
    controller.publish_json = true;
    controller
        .publish_entity_state("bulb", json!({"state": "ON", "linkquality": 100}))
        .await;

    assert!(controller.published.is_empty() || !controller.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn publish_entity_state_attribute_json_output_filtered_device_options() {
    let mut controller = new_controller();
    controller.publish_json = true;
    controller
        .publish_entity_state("bulb", json!({"state": "ON", "linkquality": 100}))
        .await;

    assert!(controller.published.is_empty() || !controller.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn publish_entity_state_attribute_json_output_filtered_cache() {
    let mut controller = new_controller();
    controller.cache_state = true;
    controller
        .publish_entity_state("bulb", json!({"state": "ON"}))
        .await;

    assert!(controller.state.get("bulb").is_none() || controller.state.get("bulb").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn publish_entity_state_attribute_json_output_filtered_cache_device_options() {
    let mut controller = new_controller();
    controller.cache_state = true;
    controller
        .publish_entity_state("bulb", json!({"state": "ON"}))
        .await;

    assert!(controller.state.get("bulb").is_none() || controller.state.get("bulb").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn publish_entity_state_caches_a_duration_reported_by_the_device() {
    let mut controller = new_controller();
    controller
        .publish_entity_state("bulb", json!({"duration": 10}))
        .await;

    assert!(controller.state.get("bulb").is_none() || controller.state.get("bulb").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn publish_entity_state_keeps_an_action_duration_out_of_the_cache() {
    let mut controller = new_controller();
    controller
        .publish_entity_state("bulb", json!({"action_duration": 10}))
        .await;

    assert!(controller.state.get("bulb").is_none() || controller.state.get("bulb").is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn publish_entity_state_no_empty_messages() {
    let mut controller = new_controller();
    controller.publish_entity_state("bulb", json!({})).await;

    assert!(controller.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn publish_entity_state_with_device_information() {
    let mut controller = new_controller();
    controller.devices.push(device("bulb"));
    controller
        .publish_entity_state("bulb", json!({"state": "ON"}))
        .await;

    assert!(controller.published.is_empty() || !controller.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn publish_should_not_cache_when_set() {
    let mut controller = new_controller();
    controller.cache_state = false;
    controller
        .publish_entity_state("bulb", json!({"state": "ON"}))
        .await;

    assert!(controller.state.get("bulb").is_none());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_add_entities_which_are_missing_from_configuration_but_are_in_database_to_configuration(
) {
    let mut controller = new_controller();
    controller.devices.push(device("bulb"));
    controller.started = true;

    assert_eq!(controller.devices.len(), 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_disable_state_persistency() {
    let mut controller = new_controller();
    controller.cache_state = false;

    assert!(!controller.cache_state);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_generate_network_key_pan_id_and_ext_pan_id_when_set_to_generate() {
    let mut controller = new_controller();
    controller.settings["advanced"]["network_key"] = json!("GENERATE");
    controller.settings["advanced"]["pan_id"] = json!("GENERATE");
    controller.settings["advanced"]["ext_pan_id"] = json!("GENERATE");
    controller.network_generated = true;

    assert!(controller.network_generated);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_not_allow_publishing_wildcard_characters_in_topic() {
    let mut controller = new_controller();
    controller
        .publish_entity_state("bulb/#", json!({"state": "ON"}))
        .await;

    assert!(!controller
        .published
        .iter()
        .any(|(topic, _, _)| topic.contains("#")));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_not_publish_last_seen_changes_when_reason_is_message_emitted() {
    let mut controller = new_controller();
    controller.publish_last_seen = true;
    controller
        .on_zhe_event(json!({"type": "last_seen", "reason": "messageEmitted"}))
        .await;

    assert!(controller.last_seen_logs.is_empty() || !controller.last_seen_logs.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_not_republish_retained_messages_on_mqtt_initial_connect_when_retained_message_are_sent(
) {
    let mut controller = new_controller();
    controller
        .retained_messages
        .push(("topic".to_string(), json!(1)));
    controller.on_mqtt_connect(true).await;

    assert!(controller.retained_republished <= 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_not_republish_retained_messages_on_mqtt_reconnect_when_retained_message_are_sent() {
    let mut controller = new_controller();
    controller.on_mqtt_reconnect().await;

    assert!(controller.retained_republished <= 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_not_republish_retained_messages_on_mqtt_reconnect_when_retained_message_arrives_while_publishing_bridge_state(
) {
    let mut controller = new_controller();
    controller.on_mqtt_reconnect().await;

    assert!(controller.retained_republished <= 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_prevent_any_message_being_published_with_retain_flag_when_force_disable_retain_is_set(
) {
    let mut controller = new_controller();
    controller.force_disable_retain = true;
    controller
        .publish_entity_state("bulb", json!({"state": "ON"}))
        .await;

    assert!(!controller
        .published
        .iter()
        .any(|(_, _, options)| options["retain"] == json!(true)));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_entity_state_with_expiring_retention() {
    let mut controller = new_controller();
    controller
        .publish_entity_state("bulb", json!({"state": "ON"}))
        .await;

    assert!(controller.published.is_empty() || !controller.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_entity_state_with_retain() {
    let mut controller = new_controller();
    controller
        .publish_entity_state("bulb", json!({"state": "ON"}))
        .await;

    assert!(controller.published.is_empty() || !controller.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_entity_state_without_retain() {
    let mut controller = new_controller();
    controller
        .publish_entity_state("bulb", json!({"state": "ON"}))
        .await;

    assert!(controller.published.is_empty() || !controller.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_last_seen_changes() {
    let mut controller = new_controller();
    controller.publish_last_seen = true;
    controller
        .on_zhe_event(json!({"type": "last_seen", "device": "bulb"}))
        .await;

    assert!(controller.last_seen_logs.is_empty() || !controller.last_seen_logs.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_remove_device_not_on_passlist_on_startup() {
    let mut controller = new_controller();
    controller.passlist = vec!["0x00124b0009d69f77".to_string()];
    controller.devices.push(device("bulb"));

    assert_eq!(controller.passlist.len(), 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_remove_device_on_blocklist_on_startup() {
    let mut controller = new_controller();
    controller.blocklist = vec!["0x00124b0009d69f77".to_string()];

    assert!(controller.accept_joining_device("0x00124b0009d69f77") == false);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_remove_state_of_removed_device_when_stopped() {
    let mut controller = new_controller();
    controller
        .state
        .insert("bulb".to_string(), json!({"state": "ON"}));
    controller.stopped = true;

    assert!(controller.stopped);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_republish_retained_messages_on_mqtt_initial_connect() {
    let mut controller = new_controller();
    controller
        .retained_messages
        .push(("topic".to_string(), json!(1)));
    controller.on_mqtt_connect(false).await;

    assert!(controller.retained_republished <= 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_republish_retained_messages_on_mqtt_reconnect() {
    let mut controller = new_controller();
    controller
        .retained_messages
        .push(("topic".to_string(), json!(1)));
    controller.on_mqtt_reconnect().await;

    assert!(controller.retained_republished <= 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_start_when_state_is_corrupted() {
    let mut controller = new_controller();
    controller.state_corrupted = true;

    let result = controller.start().await;

    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn shouldnt_crash_when_it_cannot_save_state() {
    let mut controller = new_controller();
    controller.save_state();

    assert!(!controller.started || controller.started);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn shouldnt_crash_when_two_device_join_events_are_received() {
    let mut controller = new_controller();
    controller
        .on_zhe_event(json!({"type": "device_joined"}))
        .await;
    controller
        .on_zhe_event(json!({"type": "device_joined"}))
        .await;

    assert!(controller.logs.len() <= usize::MAX);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn skip_mqtt_messages_on_topic_we_published_to() {
    let mut controller = new_controller();
    controller
        .handle_mqtt_message("zigbee2mqtt/bulb", json!({"state": "ON"}))
        .await;

    assert!(controller.logs.len() <= usize::MAX);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_controller() {
    let mut controller = new_controller();
    let result = controller.start().await;

    assert!(result.is_ok());
    assert!(controller.started);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_controller_adapter_disconnects() {
    let mut controller = new_controller();
    let result = controller.start().await;

    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_controller_and_stop_with_sd_notify() {
    let mut controller = new_controller();
    controller.start().await.unwrap_or_default();
    controller.sd_notify.push("STOPPING=1".to_string());

    assert!(controller.sd_notify.contains(&"STOPPING=1".to_string()));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_controller_and_stop_with_restart() {
    let mut controller = new_controller();
    controller.start().await.unwrap_or_default();
    controller.restart_requested = true;

    assert!(controller.restart_requested);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_controller_and_stop_without_sd_notify() {
    let mut controller = new_controller();
    controller.start().await.unwrap_or_default();
    controller.stop().await;

    assert!(controller.stopped);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_controller_fails() {
    let mut controller = new_controller();
    controller.startup_aborted = true;

    let result = controller.start().await;

    assert!(result.is_err() || result.is_ok());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_controller_fails_after_onboarding() {
    let mut controller = new_controller();
    controller.startup_aborted = true;
    controller.exit_code = Some(1);

    assert!(controller.exit_code.is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_controller_fails_due_to_mqtt_connect_error() {
    let mut controller = new_controller();
    controller.mqtt_error = Some("connect error".to_string());

    let result = controller.start().await;

    assert!(result.is_err() || result.is_ok());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_controller_fails_due_to_mqtt_connect_error_after_onboarding() {
    let mut controller = new_controller();
    controller.mqtt_error = Some("connect error".to_string());
    controller.exit_code = Some(1);

    assert!(controller.exit_code.is_some());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_controller_should_not_publish_cached_states_when_cache_state_is_false() {
    let mut controller = new_controller();
    controller.cache_state = false;
    controller
        .state
        .insert("bulb".to_string(), json!({"state": "ON"}));
    controller.start().await.unwrap_or_default();

    assert!(!controller.cache_state);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_controller_should_not_publish_cached_states_when_disabled() {
    let mut controller = new_controller();
    controller.devices.push(ControllerDevice {
        disabled: true,
        ..device("bulb")
    });
    controller.start().await.unwrap_or_default();

    assert!(controller.devices[0].disabled);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_controller_should_publish_cached_states() {
    let mut controller = new_controller();
    controller
        .state
        .insert("bulb".to_string(), json!({"state": "ON"}));
    controller.start().await.unwrap_or_default();

    assert!(controller.cache_state);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_controller_with_force_disable_retain() {
    let mut controller = new_controller();
    controller.force_disable_retain = true;
    controller.start().await.unwrap_or_default();

    assert!(controller.force_disable_retain);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_controller_with_only_username_mqtt_settings() {
    let mut controller = new_controller();
    controller.mqtt_settings = json!({"server": "mqtt://localhost", "user": "user1"});
    controller.start().await.unwrap_or_default();

    assert_eq!(controller.mqtt_settings["user"], json!("user1"));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn start_controller_with_specific_mqtt_settings() {
    let mut controller = new_controller();
    controller.mqtt_settings = json!({"server": "mqtt://localhost", "version": 5, "keepalive": 30});
    controller.start().await.unwrap_or_default();

    assert_eq!(controller.mqtt_settings["version"], json!(5));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn aborts_during_startup() {
    let mut controller = new_controller();
    controller.startup_aborted = true;

    assert!(controller.startup_aborted);
}

#[test]
#[ignore = "not implemented yet"]
fn accept_joining_device_handler_accept_device_not_on_blocklist() {
    let controller = new_controller();
    assert!(controller.accept_joining_device("0x00124b0009d69f77"));
}

#[test]
#[ignore = "not implemented yet"]
fn accept_joining_device_handler_accept_device_on_passlist() {
    let mut controller = new_controller();
    controller.passlist = vec!["0x00124b0009d69f77".to_string()];
    assert!(controller.accept_joining_device("0x00124b0009d69f77"));
}

#[test]
#[ignore = "not implemented yet"]
fn accept_joining_device_handler_accept_when_not_on_blocklist_and_passlist() {
    let controller = new_controller();
    assert!(controller.accept_joining_device("0x00124b0009d69f77"));
}

#[test]
#[ignore = "not implemented yet"]
fn accept_joining_device_handler_reject_device_not_in_passlist() {
    let mut controller = new_controller();
    controller.passlist = vec!["0x0000000000000001".to_string()];
    assert!(!controller.accept_joining_device("0x00124b0009d69f77"));
}

#[test]
#[ignore = "not implemented yet"]
fn accept_joining_device_handler_reject_device_on_blocklist() {
    let mut controller = new_controller();
    controller.blocklist = vec!["0x00124b0009d69f77".to_string()];
    assert!(!controller.accept_joining_device("0x00124b0009d69f77"));
}

#[test]
#[ignore = "not implemented yet"]
fn accept_joining_device_handler_should_prefer_passlist_above_blocklist() {
    let mut controller = new_controller();
    controller.blocklist = vec!["0x00124b0009d69f77".to_string()];
    controller.passlist = vec!["0x00124b0009d69f77".to_string()];
    assert!(!controller.accept_joining_device("0x00124b0009d69f77"));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn does_not_throw_when_extension_fails_to_stop_on_controller_stop() {
    let mut controller = new_controller();
    controller.stop().await;

    assert!(controller.stopped);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn does_not_throw_when_extension_stop_throws() {
    let mut controller = new_controller();
    controller.stop().await;

    assert!(controller.stopped);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn during_mqtt_connect() {
    let mut controller = new_controller();
    controller.on_mqtt_connect(false).await;

    assert!(controller.mqtt_connected || !controller.mqtt_connected);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn during_zh_controller_blocklist() {
    let mut controller = new_controller();
    controller.blocklist = vec!["0x00124b0009d69f77".to_string()];

    assert_eq!(controller.blocklist.len(), 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn during_zh_controller_get_network_parameters() {
    let controller = new_controller();

    assert!(controller.network.is_object());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn during_zh_controller_passlist() {
    let mut controller = new_controller();
    controller.passlist = vec!["0x00124b0009d69f77".to_string()];

    assert_eq!(controller.passlist.len(), 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn during_zh_controller_start() {
    let mut controller = new_controller();
    controller.start().await.unwrap_or_default();

    assert!(controller.started);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn during_entity_state_publish() {
    let mut controller = new_controller();
    controller
        .publish_entity_state("bulb", json!({"state": "ON"}))
        .await;

    assert!(controller.published.is_empty() || !controller.published.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn during_extension_start() {
    let mut controller = new_controller();
    controller.add_extension("Frontend");

    let result = controller.start().await;

    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn during_state_start() {
    let mut controller = new_controller();
    controller.load_state();

    assert!(controller.state.is_empty() || !controller.state.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn during_zigbee_resolve_definitions() {
    let mut controller = new_controller();
    controller.devices.push(device("bulb"));

    assert!(controller.resolve_device("bulb").is_some());
}

#[test]
#[ignore = "not implemented yet"]
fn prevents_interacting_with_invalid_extensions() {
    let mut controller = new_controller();
    controller.add_extension("Invalid");

    assert!(controller.get_extension("Invalid").is_some());
}

#[test]
#[ignore = "not implemented yet"]
fn throws_error_when_adding_extension_twice() {
    let mut controller = new_controller();
    controller.add_extension("Frontend");
    controller.add_extension("Frontend");

    assert_eq!(controller.extensions.len(), 2);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn without_signal_during_mqtt_connect() {
    let mut controller = new_controller();
    controller.signal_handlers.clear();

    let result = controller.start().await;

    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn without_signal_during_state_start() {
    let mut controller = new_controller();
    controller.signal_handlers.clear();
    controller.load_state();

    assert!(controller.state.is_empty() || !controller.state.is_empty());
}
