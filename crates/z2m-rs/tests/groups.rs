// Original: .upstream/zigbee2mqtt/test/extensions/groups.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Add to group via MQTT
//   - Add to group via MQTT fails
//   - Add to group via MQTT with postfix
//   - Add to group via MQTT with postfix shouldnt add it twice
//   - Add to group with slashes via MQTT
//   - Error when add/remove with invalid payload
//   - Error when adding a non-existing device
//   - Error when adding a non-existing endpoint
//   - Error when adding to non-existing group
//   - Error when invalid payload
//   - Extension: Groups
//   - Remove from group all
//   - Remove from group via MQTT
//   - Remove from group via MQTT fails
//   - Remove from group via MQTT keeping device reporting
//   - Remove from group via MQTT with postfix variant 1
//   - Remove from group via MQTT with postfix variant 2
//   - Remove from group via MQTT with postfix variant 3
//   - Should not publish state change off if any lights within are still on when changed via device
//   - Should not publish state change off if any lights within are still on when changed via device with non default-ep
//   - Should not publish state change off if any lights within are still on when changed via shared group
//   - Should not publish state change off if any lights within with non default-ep are still on when changed via device
//   - Should not publish state change when group changes state and device is disabled
//   - Should not republish identical optimistic group states
//   - Should only include relevant properties when publishing member states
//   - Should only update group state with changed properties
//   - Should publish group state change when a device in it changes state
//   - Should publish state change for group when members state change
//   - Should publish state change of all members when a group changes its state
//   - Should publish state change of all members when a group changes its state, filtered
//   - Should publish state change of another group with shared device when a group changes its state
//   - Should publish state change off even when missing current state
//   - Should publish state change off if all lights within turn off
//   - Should publish state change off if all lights within turn off with non default-ep
//   - Should publish state change off if all lights within turn off with non default-ep, but device state does not use them
//   - Should publish state change off if any lights within are still on when changed via device when off_state: last_member_state is used
//   - Should publish state of device with endpoint name
//   - Should publish state of group when specific state of specific endpoint is changed
//   - Shouldnt publish group state change when a group is not optimistic

use serde_json::json;
use z2m_rs::extension::groups::{GroupConfig, GroupState, Groups};

fn setup() -> Groups {
    let mut groups = Groups::new();
    groups.add_group("group_1");
    groups.add_device("bulb_color", &[("default", "1")]);
    groups.add_device(
        "wall_switch_double",
        &[("left", "1"), ("right", "3"), ("1", "1"), ("3", "3")],
    );
    groups.add_device(
        "0x0017880104e45542",
        &[("left", "1"), ("right", "3"), ("1", "1"), ("3", "3")],
    );
    groups
}

const ADD: &str = "zigbee2mqtt/bridge/request/group/members/add";
const REMOVE: &str = "zigbee2mqtt/bridge/request/group/members/remove";
const REMOVE_ALL: &str = "zigbee2mqtt/bridge/request/group/members/remove_all";
const GROUPS_TOPIC: &str = "zigbee2mqtt/bridge/groups";

fn response(topic: &str) -> String {
    format!(
        "zigbee2mqtt/bridge/response/group/members/{}",
        topic.rsplit('/').next().unwrap()
    )
}

fn has_response(groups: &Groups, topic: &str, payload: serde_json::Value) -> bool {
    groups
        .published
        .contains(&(response(topic), payload, json!({})))
}

fn has_groups_publish(groups: &Groups) -> bool {
    groups
        .published
        .iter()
        .any(|(topic, _, _)| topic == GROUPS_TOPIC)
}

#[tokio::test]
async fn add_to_group_via_mqtt() {
    let mut groups = setup();
    groups.published.clear();

    groups.on_message(
        ADD,
        &json!({"transaction": "123", "group": "group_1", "device": "bulb_color"}),
    );

    assert_eq!(
        groups.groups["group_1"].members,
        vec!["bulb_color:1".to_string()]
    );
    assert!(has_groups_publish(&groups));
    assert!(has_response(
        &groups,
        ADD,
        json!({
            "data": {"device": "bulb_color", "endpoint": "default", "group": "group_1"},
            "transaction": "123",
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn add_to_group_via_mqtt_fails() {
    let mut groups = setup();
    groups.fail_add = Some("timeout".to_string());
    groups.published.clear();

    groups.on_message(ADD, &json!({"group": "group_1", "device": "bulb_color"}));

    assert!(groups.groups["group_1"].members.is_empty());
    assert!(!has_groups_publish(&groups));
    assert!(has_response(
        &groups,
        ADD,
        json!({"data": {}, "status": "error", "error": "Failed to add to group (timeout)"})
    ));
}

#[tokio::test]
async fn add_to_group_via_mqtt_with_postfix() {
    let mut groups = setup();
    groups.published.clear();

    groups.on_message(
        ADD,
        &json!({"group": "group_1", "device": "wall_switch_double", "endpoint": "right"}),
    );

    assert_eq!(
        groups.groups["group_1"].members,
        vec!["wall_switch_double:3".to_string()]
    );
    assert!(has_groups_publish(&groups));
    assert!(has_response(
        &groups,
        ADD,
        json!({
            "data": {"device": "wall_switch_double", "endpoint": "right", "group": "group_1"},
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn add_to_group_via_mqtt_with_postfix_shouldnt_add_it_twice() {
    let mut groups = setup();
    groups.published.clear();

    groups.on_message(
        ADD,
        &json!({"group": "group_1", "device": "wall_switch_double", "endpoint": "right"}),
    );
    groups.on_message(
        ADD,
        &json!({"group": "group_1", "device": "wall_switch_double", "endpoint": "right"}),
    );

    assert_eq!(
        groups.groups["group_1"].members,
        vec!["wall_switch_double:3".to_string()]
    );
}

#[tokio::test]
async fn add_to_group_with_slashes_via_mqtt() {
    let mut groups = setup();
    groups.add_group("group/with/slashes");
    groups.published.clear();

    groups.on_message(
        ADD,
        &json!({"group": "group/with/slashes", "device": "bulb_color"}),
    );

    assert_eq!(
        groups.groups["group/with/slashes"].members,
        vec!["bulb_color:1".to_string()]
    );
    assert!(has_groups_publish(&groups));
    assert!(has_response(
        &groups,
        ADD,
        json!({
            "data": {"device": "bulb_color", "endpoint": "default", "group": "group/with/slashes"},
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn error_when_add_remove_with_invalid_payload() {
    let mut groups = setup();
    groups.published.clear();

    groups.on_message(ADD, &json!({"groupz": "group_1", "device": "bulb_color"}));

    assert!(!has_groups_publish(&groups));
    assert!(has_response(
        &groups,
        ADD,
        json!({"data": {}, "status": "error", "error": "Invalid payload"})
    ));
}

#[tokio::test]
async fn error_when_adding_a_non_existing_device() {
    let mut groups = setup();
    groups.published.clear();

    groups.on_message(
        ADD,
        &json!({"group": "group_1", "device": "bulb_color_not_existing"}),
    );

    assert!(!has_groups_publish(&groups));
    assert!(has_response(
        &groups,
        ADD,
        json!({"data": {}, "status": "error", "error": "Device 'bulb_color_not_existing' does not exist"})
    ));
}

#[tokio::test]
async fn error_when_adding_a_non_existing_endpoint() {
    let mut groups = setup();
    groups.published.clear();

    groups.on_message(
        ADD,
        &json!({"group": "group_1", "device": "bulb_color", "endpoint": "not_existing_endpoint"}),
    );

    assert!(!has_groups_publish(&groups));
    assert!(has_response(
        &groups,
        ADD,
        json!({"data": {}, "status": "error", "error": "Device 'bulb_color' does not have endpoint 'not_existing_endpoint'"})
    ));
}

#[tokio::test]
async fn error_when_adding_to_non_existing_group() {
    let mut groups = Groups::new();
    groups.add_device("bulb_color", &[("default", "1")]);
    groups.published.clear();

    groups.on_message(
        REMOVE,
        &json!({"group": "group_1_not_existing", "device": "bulb_color"}),
    );

    assert!(!has_groups_publish(&groups));
    assert!(has_response(
        &groups,
        REMOVE,
        json!({"data": {}, "status": "error", "error": "Group 'group_1_not_existing' does not exist"})
    ));
}

#[tokio::test]
async fn error_when_invalid_payload() {
    let mut groups = setup();
    groups.published.clear();

    groups.on_message(ADD, &json!({"group": "group_1", "devicez": "bulb_color"}));

    assert!(!has_groups_publish(&groups));
    assert!(has_response(
        &groups,
        ADD,
        json!({"data": {}, "status": "error", "error": "Invalid payload"})
    ));
}

#[tokio::test]
async fn remove_from_group_all() {
    let mut groups = setup();
    groups
        .member("group_1")
        .push("0x0017880104e45542:3".to_string());
    groups.published.clear();

    groups.on_message(
        REMOVE_ALL,
        &json!({"device": "0x0017880104e45542", "endpoint": "right"}),
    );

    assert!(groups.groups["group_1"].members.is_empty());
    assert!(has_groups_publish(&groups));
    assert!(has_response(
        &groups,
        REMOVE_ALL,
        json!({"data": {"device": "0x0017880104e45542", "endpoint": "right"}, "status": "ok"})
    ));
}

#[tokio::test]
async fn remove_from_group_via_mqtt() {
    let mut groups = setup();
    groups.member("group_1").push("bulb_color:1".to_string());
    groups.published.clear();

    groups.on_message(REMOVE, &json!({"group": "group_1", "device": "bulb_color"}));

    assert!(groups.groups["group_1"].members.is_empty());
    assert!(has_groups_publish(&groups));
    assert!(has_response(
        &groups,
        REMOVE,
        json!({
            "data": {"device": "bulb_color", "endpoint": "default", "group": "group_1"},
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn remove_from_group_via_mqtt_fails() {
    let mut groups = setup();
    groups.member("group_1").push("bulb_color:1".to_string());
    groups.fail_remove = Some("timeout".to_string());
    groups.published.clear();

    groups.on_message(REMOVE, &json!({"group": "group_1", "device": "bulb_color"}));

    assert_eq!(groups.groups["group_1"].members.len(), 1);
    assert!(!has_groups_publish(&groups));
    assert!(has_response(
        &groups,
        REMOVE,
        json!({"data": {}, "status": "error", "error": "Failed to remove from group (timeout)"})
    ));
}

#[tokio::test]
async fn remove_from_group_via_mqtt_keeping_device_reporting() {
    let mut groups = setup();
    groups.member("group_1").push("bulb_color:1".to_string());
    groups.published.clear();

    groups.on_message(
        REMOVE,
        &json!({"group": "group_1", "device": "bulb_color", "skip_disable_reporting": true}),
    );

    assert!(groups.groups["group_1"].members.is_empty());
    assert!(has_groups_publish(&groups));
    assert!(has_response(
        &groups,
        REMOVE,
        json!({
            "data": {"device": "bulb_color", "endpoint": "default", "group": "group_1"},
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn remove_from_group_via_mqtt_with_postfix_variant_1() {
    let mut groups = setup();
    groups
        .member("group_1")
        .push("0x0017880104e45542:3".to_string());
    groups.published.clear();

    groups.on_message(
        REMOVE,
        &json!({"group": "group_1", "device": "0x0017880104e45542", "endpoint": "3"}),
    );

    assert!(groups.groups["group_1"].members.is_empty());
    assert!(has_groups_publish(&groups));
    assert!(has_response(
        &groups,
        REMOVE,
        json!({
            "data": {"device": "0x0017880104e45542", "endpoint": "3", "group": "group_1"},
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn remove_from_group_via_mqtt_with_postfix_variant_2() {
    let mut groups = setup();
    groups
        .member("group_1")
        .push("0x0017880104e45542:3".to_string());
    groups.published.clear();

    groups.on_message(
        REMOVE,
        &json!({"group": "group_1", "device": "wall_switch_double", "endpoint": "3"}),
    );

    assert!(groups.groups["group_1"].members.is_empty());
    assert!(has_groups_publish(&groups));
    assert!(has_response(
        &groups,
        REMOVE,
        json!({
            "data": {"device": "wall_switch_double", "endpoint": "3", "group": "group_1"},
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn remove_from_group_via_mqtt_with_postfix_variant_3() {
    let mut groups = setup();
    groups
        .member("group_1")
        .push("0x0017880104e45542:3".to_string());
    groups.published.clear();

    groups.on_message(
        REMOVE,
        &json!({"group": "group_1", "device": "0x0017880104e45542", "endpoint": "right"}),
    );

    assert!(groups.groups["group_1"].members.is_empty());
    assert!(has_groups_publish(&groups));
    assert!(has_response(
        &groups,
        REMOVE,
        json!({
            "data": {"device": "0x0017880104e45542", "endpoint": "right", "group": "group_1"},
            "status": "ok",
        })
    ));
}

#[tokio::test]
async fn should_not_publish_state_change_off_if_any_lights_within_are_still_on_when_changed_via_device(
) {
    let mut state = GroupState::new();
    state.add_group("group_1");
    state.add_member("group_1", "bulb_color", "1");
    state.add_member("group_1", "bulb", "1");

    state.on_group_set("group_1", &json!({"state": "ON"}));
    state.published.clear();

    state.on_device_set("bulb_color", &json!({"state": "OFF"}));

    assert_eq!(state.published.len(), 1);
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb_color".to_string(),
        json!({"state": "OFF"}),
        json!({"retain": false, "qos": 0})
    )));
}

#[tokio::test]
async fn should_not_publish_state_change_off_if_any_lights_within_are_still_on_when_changed_via_device_with_non_default_ep(
) {
    let mut state = GroupState::new();
    state.add_group("group_1");
    state.add_member("group_1", "bulb_color", "1");
    state.add_member("group_1", "QBKG03LM", "2");
    state.add_member("group_1", "QBKG03LM", "3");

    state.on_group_set("group_1", &json!({"state": "ON"}));
    state.published.clear();

    state.on_device_set("wall_switch_double", &json!({"state_left": "OFF"}));

    assert_eq!(state.published.len(), 1);
    assert!(state.published.contains(&(
        "zigbee2mqtt/wall_switch_double".to_string(),
        json!({"state_left": "OFF", "state_right": "ON"}),
        json!({"retain": false, "qos": 0})
    )));
}

#[tokio::test]
async fn should_not_publish_state_change_off_if_any_lights_within_are_still_on_when_changed_via_shared_group(
) {
    let mut state = GroupState::new();
    state.add_group("group_1");
    state.add_group("group_2");
    state.add_member("group_1", "bulb_color", "1");
    state.add_member("group_1", "bulb", "1");
    state.add_member("group_2", "bulb_color", "1");

    state.on_group_set("group_1", &json!({"state": "ON"}));
    state.published.clear();

    state.on_group_set("group_2", &json!({"state": "OFF"}));

    assert_eq!(state.published.len(), 2);
    assert!(state.published.contains(&(
        "zigbee2mqtt/group_2".to_string(),
        json!({"state": "OFF"}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb_color".to_string(),
        json!({"state": "OFF"}),
        json!({"retain": false, "qos": 0})
    )));
}

#[tokio::test]
async fn should_not_publish_state_change_off_if_any_lights_within_with_non_default_ep_are_still_on_when_changed_via_device(
) {
    let mut state = GroupState::new();
    state.add_group("group_1");
    state.add_member("group_1", "bulb_color", "1");
    state.add_member("group_1", "QBKG03LM", "2");

    state.on_group_set("group_1", &json!({"state": "ON"}));
    state.published.clear();

    state.on_device_set("bulb_color", &json!({"state": "OFF"}));

    assert_eq!(state.published.len(), 1);
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb_color".to_string(),
        json!({"state": "OFF"}),
        json!({"retain": false, "qos": 0})
    )));
}

#[tokio::test]
async fn should_not_publish_state_change_when_group_changes_state_and_device_is_disabled() {
    let mut state = GroupState::new();
    state.add_group("group_1");
    state.add_member("group_1", "bulb_color", "1");
    state.disable("bulb_color");

    state.on_group_set("group_1", &json!({"state": "ON"}));

    assert_eq!(state.published.len(), 1);
    assert!(state.published.contains(&(
        "zigbee2mqtt/group_1".to_string(),
        json!({"state": "ON"}),
        json!({"retain": false, "qos": 0})
    )));
}

#[tokio::test]
async fn should_not_republish_identical_optimistic_group_states() {
    let mut state = GroupState::new();
    for group in [
        "group_tradfri_remote",
        "group_with_tradfri",
        "ha_discovery_group",
        "switch_group",
    ] {
        state.add_group(group);
    }
    state.add_member("group_tradfri_remote", "bulb_2", "1");
    state.add_member("group_tradfri_remote", "bulb_color_2", "1");
    state.add_member("group_with_tradfri", "bulb_2", "1");
    state.add_member("ha_discovery_group", "bulb_2", "1");
    state.add_member("ha_discovery_group", "bulb_color_2", "1");
    state.add_member("switch_group", "bulb_2", "1");

    state.on_attribute_report("bulb_2", "1", &json!({"onOff": 1}));
    state.on_attribute_report("bulb_color_2", "1", &json!({"onOff": 1}));

    assert_eq!(state.published.len(), 6);
    for topic in [
        "zigbee2mqtt/group_tradfri_remote",
        "zigbee2mqtt/bulb_2",
        "zigbee2mqtt/bulb_color_2",
        "zigbee2mqtt/group_with_tradfri",
        "zigbee2mqtt/ha_discovery_group",
        "zigbee2mqtt/switch_group",
    ] {
        assert!(state.published.contains(&(
            topic.to_string(),
            json!({"state": "ON"}),
            json!({"retain": false, "qos": 0})
        )));
    }
}

#[tokio::test]
async fn should_only_include_relevant_properties_when_publishing_member_states() {
    let mut state = GroupState::new();
    state.add_group("group_1");
    state.add_member("group_1", "bulb_color", "1");
    state.add_member("group_1", "bulb", "1");
    state.set_retain("bulb", true);

    state.on_group_set("group_1", &json!({"color_temp": 50}));

    assert_eq!(state.published.len(), 3);
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb_color".to_string(),
        json!({"color_mode": "color_temp", "color_temp": 50}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/group_1".to_string(),
        json!({"color_mode": "color_temp", "color_temp": 50}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb".to_string(),
        json!({"color_mode": "color_temp", "color_temp": 50}),
        json!({"retain": true, "qos": 0})
    )));

    state.published.clear();
    state.on_group_set("group_1", &json!({"color": {"x": 0.5, "y": 0.3}}));

    assert_eq!(state.published.len(), 3);
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb_color".to_string(),
        json!({"color": {"x": 0.5, "y": 0.3}, "color_mode": "xy", "color_temp": 548}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/group_1".to_string(),
        json!({"color": {"x": 0.5, "y": 0.3}, "color_mode": "xy", "color_temp": 548}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb".to_string(),
        json!({"color_mode": "color_temp", "color_temp": 548}),
        json!({"retain": true, "qos": 0})
    )));
}

#[tokio::test]
async fn should_only_update_group_state_with_changed_properties() {
    let mut state = GroupState::new();
    state.add_group("group_1");
    state.add_member("group_1", "bulb_color", "1");
    state.add_member("group_1", "bulb", "1");
    state.set_retain("bulb", true);

    state.on_device_set("bulb_color", &json!({"state": "OFF", "color_temp": 200}));
    state.on_device_set("bulb", &json!({"state": "ON", "color_temp": 250}));
    state.published.clear();

    state.on_group_set("group_1", &json!({"color_temp": 300}));

    assert_eq!(state.published.len(), 3);
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb_color".to_string(),
        json!({"color_mode": "color_temp", "color_temp": 300, "state": "OFF"}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb".to_string(),
        json!({"color_mode": "color_temp", "color_temp": 300, "state": "ON"}),
        json!({"retain": true, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/group_1".to_string(),
        json!({"color_mode": "color_temp", "color_temp": 300, "state": "ON"}),
        json!({"retain": false, "qos": 0})
    )));
}

#[tokio::test]
async fn should_publish_group_state_change_when_a_device_in_it_changes_state() {
    let mut state = GroupState::new();
    state.add_group("group_1");
    state.add_member("group_1", "bulb_color", "1");

    state.on_attribute_report("bulb_color", "1", &json!({"onOff": 1}));

    assert_eq!(state.published.len(), 2);
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb_color".to_string(),
        json!({"state": "ON"}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/group_1".to_string(),
        json!({"state": "ON"}),
        json!({"retain": false, "qos": 0})
    )));
}

#[tokio::test]
async fn should_publish_state_change_for_group_when_members_state_change() {
    let mut state = GroupState::new();
    state.add_group("group_1");
    state.add_member("group_1", "bulb_color", "1");

    state.on_device_set("bulb_color", &json!({"state": "ON"}));
    assert_eq!(state.published.len(), 2);
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb_color".to_string(),
        json!({"state": "ON"}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/group_1".to_string(),
        json!({"state": "ON"}),
        json!({"retain": false, "qos": 0})
    )));

    state.published.clear();
    state.on_group_set("group_1", &json!({"state": "OFF"}));
    assert_eq!(state.published.len(), 2);
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb_color".to_string(),
        json!({"state": "OFF"}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/group_1".to_string(),
        json!({"state": "OFF"}),
        json!({"retain": false, "qos": 0})
    )));

    state.published.clear();
    state.on_device_set("bulb_color", &json!({"state": "ON"}));
    assert_eq!(state.published.len(), 2);
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb_color".to_string(),
        json!({"state": "ON"}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/group_1".to_string(),
        json!({"state": "ON"}),
        json!({"retain": false, "qos": 0})
    )));
}

#[tokio::test]
async fn should_publish_state_change_of_all_members_when_a_group_changes_its_state() {
    let mut state = GroupState::new();
    state.add_group("group_1");
    state.add_member("group_1", "bulb_color", "1");

    state.on_group_set("group_1", &json!({"state": "ON"}));

    assert_eq!(state.published.len(), 2);
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb_color".to_string(),
        json!({"state": "ON"}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/group_1".to_string(),
        json!({"state": "ON"}),
        json!({"retain": false, "qos": 0})
    )));
}

#[tokio::test]
async fn should_publish_state_change_of_all_members_when_a_group_changes_its_state_filtered() {
    let mut state = GroupState::new();
    state.add_group("group_1");
    state.add_member("group_1", "bulb_color", "1");
    state.set_group_config(
        "group_1",
        GroupConfig {
            optimistic: true,
            filtered_attributes: vec!["brightness".to_string()],
            off_state: None,
        },
    );

    state.on_group_set("group_1", &json!({"state": "ON", "brightness": 100}));

    assert_eq!(state.published.len(), 2);
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb_color".to_string(),
        json!({"state": "ON", "brightness": 100}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/group_1".to_string(),
        json!({"state": "ON"}),
        json!({"retain": false, "qos": 0})
    )));
}

#[tokio::test]
async fn should_publish_state_change_of_another_group_with_shared_device_when_a_group_changes_its_state(
) {
    let mut state = GroupState::new();
    state.add_group("group_1");
    state.add_group("group_2");
    state.add_member("group_1", "bulb_color", "1");
    state.add_member("group_2", "bulb_color", "1");

    state.on_group_set("group_1", &json!({"state": "ON"}));

    assert_eq!(state.published.len(), 3);
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb_color".to_string(),
        json!({"state": "ON"}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/group_1".to_string(),
        json!({"state": "ON"}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/group_2".to_string(),
        json!({"state": "ON"}),
        json!({"retain": false, "qos": 0})
    )));
}

#[tokio::test]
async fn should_publish_state_change_off_even_when_missing_current_state() {
    let mut state = GroupState::new();
    state.add_group("group_1");
    state.add_member("group_1", "bulb_color", "1");
    state.add_member("group_1", "bulb", "1");

    state.on_group_set("group_1", &json!({"state": "ON"}));
    state.published.clear();
    state.group_states.clear();

    state.on_device_set("bulb_color", &json!({"state": "OFF"}));

    assert_eq!(state.published.len(), 2);
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb_color".to_string(),
        json!({"state": "OFF"}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/group_1".to_string(),
        json!({"state": "OFF"}),
        json!({"retain": false, "qos": 0})
    )));
}

#[tokio::test]
async fn should_publish_state_change_off_if_all_lights_within_turn_off() {
    let mut state = GroupState::new();
    state.add_group("group_1");
    state.add_member("group_1", "bulb_color", "1");
    state.add_member("group_1", "bulb", "1");
    state.set_retain("bulb", true);

    state.on_group_set("group_1", &json!({"state": "ON"}));

    state.published.clear();
    state.on_device_set("bulb_color", &json!({"state": "OFF"}));
    state.on_device_set("bulb", &json!({"state": "OFF"}));

    assert_eq!(state.published.len(), 3);
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb_color".to_string(),
        json!({"state": "OFF"}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb".to_string(),
        json!({"state": "OFF"}),
        json!({"retain": true, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/group_1".to_string(),
        json!({"state": "OFF"}),
        json!({"retain": false, "qos": 0})
    )));
}

#[tokio::test]
async fn should_publish_state_change_off_if_all_lights_within_turn_off_with_non_default_ep() {
    let mut state = GroupState::new();
    state.add_group("group_1");
    state.add_member("group_1", "bulb_color", "1");
    state.add_member("group_1", "QBKG03LM", "2");

    state.on_group_set("group_1", &json!({"state": "ON"}));
    state.published.clear();

    state.on_device_set("bulb_color", &json!({"state": "OFF"}));
    state.on_device_set("wall_switch_double", &json!({"state_left": "OFF"}));

    assert_eq!(state.published.len(), 3);
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb_color".to_string(),
        json!({"state": "OFF"}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/wall_switch_double".to_string(),
        json!({"state_left": "OFF"}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/group_1".to_string(),
        json!({"state": "OFF"}),
        json!({"retain": false, "qos": 0})
    )));
}

#[tokio::test]
async fn should_publish_state_change_off_if_all_lights_within_turn_off_with_non_default_ep_but_device_state_does_not_use_them(
) {
    let mut state = GroupState::new();
    state.add_group("group_1");
    state.add_member("group_1", "bulb_color", "1");
    state.add_member("group_1", "InovelliVZM31SN", "2");

    state.on_group_set("group_1", &json!({"state": "ON"}));
    state.published.clear();

    state.on_device_set("bulb_color", &json!({"state": "OFF"}));
    state.on_device_set("wall_switch_double", &json!({"state_left": "OFF"}));

    assert_eq!(state.published.len(), 3);
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb_color".to_string(),
        json!({"state": "OFF"}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/wall_switch_double".to_string(),
        json!({"state_left": "OFF"}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/group_1".to_string(),
        json!({"state": "OFF"}),
        json!({"retain": false, "qos": 0})
    )));
}

#[tokio::test]
async fn should_publish_state_change_off_if_any_lights_within_are_still_on_when_changed_via_device_when_off_state_last_member_state_is_used(
) {
    let mut state = GroupState::new();
    state.add_group("group_1");
    state.add_member("group_1", "bulb_color", "1");
    state.add_member("group_1", "bulb", "1");
    state.set_group_config(
        "group_1",
        GroupConfig {
            optimistic: true,
            filtered_attributes: Vec::new(),
            off_state: Some("last_member_state".to_string()),
        },
    );

    state.on_group_set("group_1", &json!({"state": "ON"}));
    state.published.clear();

    state.on_device_set("bulb_color", &json!({"state": "OFF"}));

    assert_eq!(state.published.len(), 2);
    assert_eq!(
        state.published[0],
        (
            "zigbee2mqtt/group_1".to_string(),
            json!({"state": "OFF"}),
            json!({"retain": false, "qos": 0})
        )
    );
    assert_eq!(
        state.published[1],
        (
            "zigbee2mqtt/bulb_color".to_string(),
            json!({"state": "OFF"}),
            json!({"retain": false, "qos": 0})
        )
    );
}

#[tokio::test]
async fn should_publish_state_of_device_with_endpoint_name() {
    let mut state = GroupState::new();
    state.add_group("gledopto_group");
    state.add_member("gledopto_group", "GLEDOPTO_2ID", "11");

    state.on_group_set("gledopto_group", &json!({"state": "ON"}));

    assert_eq!(state.published.len(), 2);
    assert!(state.published.contains(&(
        "zigbee2mqtt/GLEDOPTO_2ID".to_string(),
        json!({"state_cct": "ON"}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/gledopto_group".to_string(),
        json!({"state": "ON"}),
        json!({"retain": false, "qos": 0})
    )));
    assert_eq!(state.command_calls["gledopto_group"], 1);
}

#[tokio::test]
async fn should_publish_state_of_group_when_specific_state_of_specific_endpoint_is_changed() {
    let mut state = GroupState::new();
    state.add_group("gledopto_group");
    state.add_member("gledopto_group", "GLEDOPTO_2ID", "11");

    state.on_device_set("GLEDOPTO_2ID", &json!({"state_cct": "ON"}));

    assert_eq!(state.published.len(), 2);
    assert!(state.published.contains(&(
        "zigbee2mqtt/GLEDOPTO_2ID".to_string(),
        json!({"state_cct": "ON"}),
        json!({"retain": false, "qos": 0})
    )));
    assert!(state.published.contains(&(
        "zigbee2mqtt/gledopto_group".to_string(),
        json!({"state": "ON"}),
        json!({"retain": false, "qos": 0})
    )));
    assert_eq!(
        state
            .command_calls
            .get("gledopto_group")
            .copied()
            .unwrap_or(0),
        0
    );
}

#[tokio::test]
async fn shouldnt_publish_group_state_change_when_a_group_is_not_optimistic() {
    let mut state = GroupState::new();
    state.add_group("group_1");
    state.add_member("group_1", "bulb_color", "1");
    state.set_group_config(
        "group_1",
        GroupConfig {
            optimistic: false,
            filtered_attributes: Vec::new(),
            off_state: None,
        },
    );

    state.on_attribute_report("bulb_color", "1", &json!({"onOff": 1}));

    assert_eq!(state.published.len(), 1);
    assert!(state.published.contains(&(
        "zigbee2mqtt/bulb_color".to_string(),
        json!({"state": "ON"}),
        json!({"retain": false, "qos": 0})
    )));
}
