// Original: .upstream/zigbee2mqtt/test/controller.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Controller
//   - Dont publish to mqtt when client is unavailable
//   - EventBus should handle async errors
//   - EventBus should handle sync errors
//   - Handle mqtt message
//   - Handles MQTT publish error
//   - Handles reconnecting to MQTT
//   - Handles reconnecting to MQTT after v5+ DISCONNECT
//   - Ignore messages from coordinator
//   - Load empty state when state file does not exist
//   - Log when MQTT client is unavailable
//   - On zigbee deviceInterview failed
//   - On zigbee deviceInterview started
//   - On zigbee deviceInterview successful not supported
//   - On zigbee deviceInterview successful supported
//   - On zigbee deviceJoined
//   - On zigbee event device announce
//   - On zigbee event device leave (removed from database and NOT settings)
//   - On zigbee event device leave (removed from database and settings)
//   - On zigbee event message
//   - On zigbee event message with group ID
//   - Publish entity state attribute output
//   - Publish entity state attribute output with a null color
//   - Publish entity state attribute_json output
//   - Publish entity state attribute_json output filtered
//   - Publish entity state attribute_json output filtered (device_options)
//   - Publish entity state attribute_json output filtered cache
//   - Publish entity state attribute_json output filtered cache (device_options)
//   - Publish entity state caches a duration reported by the device
//   - Publish entity state keeps an action_duration out of the cache
//   - Publish entity state no empty messages
//   - Publish entity state with device information
//   - Publish should not cache when set
//   - Should add entities which are missing from configuration but are in database to configuration
//   - Should allow to disable state persistency
//   - Should generate network_key, pan_id and ext_pan_id when set to GENERATE
//   - Should not allow publishing wildcard characters in topic
//   - Should not publish last seen changes when reason is messageEmitted
//   - Should not republish retained messages on MQTT initial connect when retained message are sent
//   - Should not republish retained messages on MQTT reconnect when retained message are sent
//   - Should not republish retained messages on MQTT reconnect when retained message arrives while publishing bridge state
//   - Should prevent any message being published with retain flag when force_disable_retain is set
//   - Should publish entity state with expiring retention
//   - Should publish entity state with retain
//   - Should publish entity state without retain
//   - Should publish last seen changes
//   - Should remove device not on passlist on startup
//   - Should remove device on blocklist on startup
//   - Should remove state of removed device when stopped
//   - Should republish retained messages on MQTT initial connect
//   - Should republish retained messages on MQTT reconnect
//   - Should start when state is corrupted
//   - Shouldnt crash when it cannot save state
//   - Shouldnt crash when two device join events are received
//   - Skip MQTT messages on topic we published to
//   - Start controller
//   - Start controller adapter disconnects
//   - Start controller and stop with SdNotify
//   - Start controller and stop with restart
//   - Start controller and stop without SdNotify
//   - Start controller fails
//   - Start controller fails after onboarding
//   - Start controller fails due to MQTT connect error
//   - Start controller fails due to MQTT connect error after onboarding
//   - Start controller should not publish cached states when cache_state is false
//   - Start controller should not publish cached states when disabled
//   - Start controller should publish cached states
//   - Start controller with force_disable_retain
//   - Start controller with only username MQTT settings
//   - Start controller with specific MQTT settings
//   - aborts during startup
//   - acceptJoiningDeviceHandler accept device not on blocklist
//   - acceptJoiningDeviceHandler accept device on passlist
//   - acceptJoiningDeviceHandler accept when not on blocklist and passlist
//   - acceptJoiningDeviceHandler reject device not in passlist
//   - acceptJoiningDeviceHandler reject device on blocklist
//   - acceptJoiningDeviceHandler should prefer passlist above blocklist
//   - does not throw when extension fails to stop on controller stop
//   - does not throw when extension stop throws
//   - during MQTT connect
//   - during ZH Controller blocklist
//   - during ZH Controller getNetworkParameters
//   - during ZH Controller passlist
//   - during ZH Controller start
//   - during entity state publish
//   - during extension start
//   - during state start
//   - during zigbee resolve definitions
//   - prevents interacting with invalid extensions
//   - throws error when adding extension twice
//   - without signal during MQTT connect
//   - without signal during state start

#[test]
#[ignore = "not implemented yet"]
fn controller() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn dont_publish_to_mqtt_when_client_is_unavailable() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn event_bus_should_handle_async_errors() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn event_bus_should_handle_sync_errors() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handle_mqtt_message() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_mqtt_publish_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_reconnecting_to_mqtt() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn handles_reconnecting_to_mqtt_after_v5_disconnect() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn ignore_messages_from_coordinator() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn load_empty_state_when_state_file_does_not_exist() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn log_when_mqtt_client_is_unavailable() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn on_zigbee_device_interview_failed() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn on_zigbee_device_interview_started() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn on_zigbee_device_interview_successful_not_supported() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn on_zigbee_device_interview_successful_supported() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn on_zigbee_device_joined() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn on_zigbee_event_device_announce() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn on_zigbee_event_device_leave_removed_from_database_and_not_settings() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn on_zigbee_event_device_leave_removed_from_database_and_settings() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn on_zigbee_event_message() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn on_zigbee_event_message_with_group_id() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn publish_entity_state_attribute_output() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn publish_entity_state_attribute_output_with_a_null_color() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn publish_entity_state_attribute_json_output() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn publish_entity_state_attribute_json_output_filtered() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn publish_entity_state_attribute_json_output_filtered_device_options() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn publish_entity_state_attribute_json_output_filtered_cache() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn publish_entity_state_attribute_json_output_filtered_cache_device_options() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn publish_entity_state_caches_a_duration_reported_by_the_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn publish_entity_state_keeps_an_action_duration_out_of_the_cache() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn publish_entity_state_no_empty_messages() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn publish_entity_state_with_device_information() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn publish_should_not_cache_when_set() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_add_entities_which_are_missing_from_configuration_but_are_in_database_to_configuration() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_allow_to_disable_state_persistency() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_generate_network_key_pan_id_and_ext_pan_id_when_set_to_generate() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_not_allow_publishing_wildcard_characters_in_topic() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_not_publish_last_seen_changes_when_reason_is_message_emitted() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_not_republish_retained_messages_on_mqtt_initial_connect_when_retained_message_are_sent() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_not_republish_retained_messages_on_mqtt_reconnect_when_retained_message_are_sent() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_not_republish_retained_messages_on_mqtt_reconnect_when_retained_message_arrives_while_publishing_bridge_state(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_prevent_any_message_being_published_with_retain_flag_when_force_disable_retain_is_set() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_entity_state_with_expiring_retention() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_entity_state_with_retain() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_entity_state_without_retain() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_last_seen_changes() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_remove_device_not_on_passlist_on_startup() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_remove_device_on_blocklist_on_startup() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_remove_state_of_removed_device_when_stopped() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_republish_retained_messages_on_mqtt_initial_connect() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_republish_retained_messages_on_mqtt_reconnect() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_start_when_state_is_corrupted() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn shouldnt_crash_when_it_cannot_save_state() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn shouldnt_crash_when_two_device_join_events_are_received() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn skip_mqtt_messages_on_topic_we_published_to() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn start_controller() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn start_controller_adapter_disconnects() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn start_controller_and_stop_with_sd_notify() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn start_controller_and_stop_with_restart() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn start_controller_and_stop_without_sd_notify() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn start_controller_fails() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn start_controller_fails_after_onboarding() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn start_controller_fails_due_to_mqtt_connect_error() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn start_controller_fails_due_to_mqtt_connect_error_after_onboarding() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn start_controller_should_not_publish_cached_states_when_cache_state_is_false() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn start_controller_should_not_publish_cached_states_when_disabled() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn start_controller_should_publish_cached_states() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn start_controller_with_force_disable_retain() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn start_controller_with_only_username_mqtt_settings() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn start_controller_with_specific_mqtt_settings() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn aborts_during_startup() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn accept_joining_device_handler_accept_device_not_on_blocklist() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn accept_joining_device_handler_accept_device_on_passlist() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn accept_joining_device_handler_accept_when_not_on_blocklist_and_passlist() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn accept_joining_device_handler_reject_device_not_in_passlist() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn accept_joining_device_handler_reject_device_on_blocklist() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn accept_joining_device_handler_should_prefer_passlist_above_blocklist() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_throw_when_extension_fails_to_stop_on_controller_stop() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_throw_when_extension_stop_throws() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn during_mqtt_connect() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn during_zh_controller_blocklist() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn during_zh_controller_get_network_parameters() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn during_zh_controller_passlist() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn during_zh_controller_start() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn during_entity_state_publish() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn during_extension_start() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn during_state_start() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn during_zigbee_resolve_definitions() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn prevents_interacting_with_invalid_extensions() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throws_error_when_adding_extension_twice() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn without_signal_during_mqtt_connect() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn without_signal_during_state_start() {
    todo!()
}
