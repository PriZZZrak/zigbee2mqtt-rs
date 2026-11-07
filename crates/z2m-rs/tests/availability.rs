// Original: .upstream/zigbee2mqtt/test/extensions/availability.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Extension: Availability
//   - Should allow to be disabled
//   - Should allow to change availability timeout via avaiability options
//   - Should allow to change availability timeout via device options
//   - Should allow to enable availability for just one device
//   - Should clear the ping queue on stop
//   - Should immediately mark device as online when it lastSeen changes
//   - Should not ping disabled devices
//   - Should not ping on startup for available or disabled devices
//   - Should ping again when first ping fails
//   - Should ping on startup for enabled and unavailable devices
//   - Should prevent instance restart
//   - Should publish availability for groups
//   - Should publish availability on startup for device where it is enabled for
//   - Should publish availability payload in JSON format
//   - Should publish offline for active device when not seen for 10 minutes
//   - Should publish offline for passive device when not seen for 25 hours
//   - Should republish availability when device is renamed
//   - Should reset ping timer when device last seen changes for active device
//   - Should reset ping timer when device last seen changes for passive device
//   - Should retrieve device state when it reconnects
//   - Should stop pinging device when it is removed
//   - Should stop pinging device when it leaves
//   - Shouldnt do anything for a device when availability: false is set for device
//   - allows to disable backoff
//   - clamps the ping delay to the maximum supported timeout
//   - does not trigger backoff on ping success
//   - jitters pings
//   - pauses when backoff reaches configured value and unpauses on last seen
//   - resets backoff on last seen
//   - triggers backoff on successive ping failures

use serde_json::json;
use z2m_rs::extension::availability::Availability;

fn setup() -> Availability {
    let mut availability = Availability::new();
    availability.add_device("bulb_color", false);
    availability.add_device("remote", true);
    availability.add_device("bulb_color_2", false);
    availability.set_availability("bulb_color_2", false);
    availability.add_group("group_tradfri_remote", &["bulb_color_2"]);
    availability
}

fn opts() -> serde_json::Value {
    json!({"retain": true, "qos": 1})
}

fn has(availability: &Availability, topic: &str, payload: serde_json::Value) -> bool {
    availability
        .published
        .contains(&(topic.to_string(), payload, opts()))
}

fn pings(availability: &Availability, device: &str) -> Vec<bool> {
    availability.devices[device].ping_calls.clone()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_be_disabled() {
    let mut availability = setup();
    availability.set_enabled(false);
    availability.start();
    availability
        .devices
        .get_mut("bulb_color")
        .unwrap()
        .ping_calls
        .clear();

    availability.advance_minutes(12);

    assert!(pings(&availability, "bulb_color").is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_change_availability_timeout_via_avaiability_options() {
    let mut availability = setup();
    availability.set_active_timeout(30);
    availability.start();
    availability
        .devices
        .get_mut("bulb_color")
        .unwrap()
        .ping_calls
        .clear();

    availability.advance_minutes(25);
    assert!(pings(&availability, "bulb_color").is_empty());

    availability.advance_minutes(7);
    assert_eq!(pings(&availability, "bulb_color"), vec![true]);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_change_availability_timeout_via_device_options() {
    let mut availability = setup();
    availability.set_timeout("bulb_color", 40);
    availability.start();
    availability
        .devices
        .get_mut("bulb_color")
        .unwrap()
        .ping_calls
        .clear();

    availability.advance_minutes(25);
    assert!(pings(&availability, "bulb_color").is_empty());

    availability.advance_minutes(17);
    assert_eq!(pings(&availability, "bulb_color"), vec![true]);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_enable_availability_for_just_one_device() {
    let mut availability = setup();
    availability.set_enabled(false);
    availability.set_availability("bulb_color", true);
    availability.start();
    availability
        .devices
        .get_mut("bulb_color")
        .unwrap()
        .ping_calls
        .clear();

    availability.advance_minutes(11);

    assert_eq!(pings(&availability, "bulb_color"), vec![true]);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_clear_the_ping_queue_on_stop() {
    let mut availability = setup();
    availability.start();
    availability.add_to_ping_queue("bulb_color");
    availability.add_to_ping_queue("bulb_color_2");

    availability.stop();
    availability.advance_minutes(1);

    assert!(availability.ping_queue.is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_immediately_mark_device_as_online_when_it_last_seen_changes() {
    let mut availability = setup();
    availability.start();
    availability.published.clear();

    availability.advance_minutes(15);
    assert!(has(
        &availability,
        "zigbee2mqtt/bulb_color/availability",
        json!({"state": "offline"})
    ));

    availability.set_last_seen_minutes_ago("bulb_color", 0);
    availability.on_last_seen_changed("bulb_color");

    assert!(has(
        &availability,
        "zigbee2mqtt/bulb_color/availability",
        json!({"state": "online"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_not_ping_disabled_devices() {
    let mut availability = setup();
    availability.set_disabled("bulb_color");
    availability.start();
    availability
        .devices
        .get_mut("bulb_color")
        .unwrap()
        .ping_calls
        .clear();

    availability.advance_minutes(15);

    assert!(pings(&availability, "bulb_color").is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_not_ping_on_startup_for_available_or_disabled_devices() {
    let mut availability = setup();
    availability.set_availability("bulb_color_2", true);
    availability.set_disabled("bulb_color_2");
    availability.set_last_seen_minutes_ago("bulb_color", 0);
    availability.set_last_seen_minutes_ago("bulb_color_2", 20);
    availability.start();

    availability.advance_minutes(1);

    assert!(pings(&availability, "bulb_color").is_empty()); // enabled/available
    assert!(pings(&availability, "bulb_color_2").is_empty()); // disabled/unavailable
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_ping_again_when_first_ping_fails() {
    let mut availability = setup();
    availability.start();
    availability.published.clear();
    availability.on_last_seen_changed("bulb_color");
    availability.set_fail_first_ping("bulb_color");

    availability.advance_minutes(15);

    assert_eq!(pings(&availability, "bulb_color"), vec![true, false]);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_ping_on_startup_for_enabled_and_unavailable_devices() {
    let mut availability = setup();
    availability.set_availability("bulb_color_2", true);
    availability.set_last_seen_minutes_ago("bulb_color", 20);
    availability.set_last_seen_minutes_ago("bulb_color_2", 0);
    availability.start();

    availability.advance_minutes(1);

    assert_eq!(pings(&availability, "bulb_color"), vec![true]); // enabled/unavailable
    assert!(pings(&availability, "bulb_color_2").is_empty()); // enabled/available
}

#[test]
#[ignore = "not implemented yet"]
fn should_prevent_instance_restart() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_availability_for_groups() {
    let mut availability = setup();
    availability.set_availability("bulb_color_2", true);
    availability.start();
    availability
        .devices
        .get_mut("bulb_color_2")
        .unwrap()
        .ping_calls
        .clear();

    assert!(has(
        &availability,
        "zigbee2mqtt/group_tradfri_remote/availability",
        json!({"state": "online"})
    ));

    availability.published.clear();
    availability.advance_minutes(12);
    assert!(has(
        &availability,
        "zigbee2mqtt/group_tradfri_remote/availability",
        json!({"state": "offline"})
    ));

    availability.published.clear();
    availability.set_last_seen_minutes_ago("bulb_color_2", 0);
    availability.on_last_seen_changed("bulb_color_2");
    assert!(has(
        &availability,
        "zigbee2mqtt/group_tradfri_remote/availability",
        json!({"state": "online"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_availability_on_startup_for_device_where_it_is_enabled_for() {
    let mut availability = setup();
    availability.start();

    assert!(has(
        &availability,
        "zigbee2mqtt/bulb_color/availability",
        json!({"state": "online"})
    ));
    assert!(has(
        &availability,
        "zigbee2mqtt/remote/availability",
        json!({"state": "online"})
    ));
    assert!(!has(
        &availability,
        "zigbee2mqtt/bulb_color_2/availability",
        json!({"state": "online"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_availability_payload_in_json_format() {
    let mut availability = setup();
    availability.start();
    availability
        .devices
        .get_mut("remote")
        .unwrap()
        .ping_calls
        .clear();
    availability.published.clear();

    availability.advance_hours(26);

    assert!(pings(&availability, "remote").is_empty());
    assert!(has(
        &availability,
        "zigbee2mqtt/remote/availability",
        json!({"state": "offline"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_offline_for_active_device_when_not_seen_for_10_minutes() {
    let mut availability = setup();
    availability.start();
    availability.published.clear();

    availability.advance_minutes(5);
    assert!(pings(&availability, "bulb_color").is_empty());

    availability.advance_minutes(7);
    assert_eq!(pings(&availability, "bulb_color"), vec![true]);
    assert!(has(
        &availability,
        "zigbee2mqtt/bulb_color/availability",
        json!({"state": "offline"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_offline_for_passive_device_when_not_seen_for_25_hours() {
    let mut availability = setup();
    availability.start();
    availability.published.clear();

    availability.advance_hours(26);

    assert!(pings(&availability, "remote").is_empty());
    assert!(has(
        &availability,
        "zigbee2mqtt/remote/availability",
        json!({"state": "offline"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_republish_availability_when_device_is_renamed() {
    let mut availability = setup();
    availability.start();
    availability.published.clear();

    availability.on_rename("bulb_color", "bulb_new_name");

    assert!(availability.published.contains(&(
        "zigbee2mqtt/bulb_color/availability".to_string(),
        json!(""),
        opts()
    )));
    assert!(has(
        &availability,
        "zigbee2mqtt/bulb_new_name/availability",
        json!({"state": "online"})
    ));

    availability.advance_hours(12);
    assert!(has(
        &availability,
        "zigbee2mqtt/bulb_new_name/availability",
        json!({"state": "offline"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_reset_ping_timer_when_device_last_seen_changes_for_active_device() {
    let mut availability = setup();
    availability.start();
    availability.published.clear();

    availability.advance_minutes(6);
    assert!(pings(&availability, "bulb_color").is_empty());

    availability.on_last_seen_changed("bulb_color");
    assert!(has(
        &availability,
        "zigbee2mqtt/bulb_color/availability",
        json!({"state": "offline"})
    ));

    availability.advance_minutes(7);
    assert!(pings(&availability, "bulb_color").is_empty());

    availability.advance_minutes(10);
    assert_eq!(pings(&availability, "bulb_color"), vec![true]);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_reset_ping_timer_when_device_last_seen_changes_for_passive_device() {
    let mut availability = setup();
    availability.start();
    availability.published.clear();

    availability.advance_hours(24);
    assert!(pings(&availability, "remote").is_empty());

    availability.on_last_seen_changed("remote");
    assert!(has(
        &availability,
        "zigbee2mqtt/remote/availability",
        json!({"state": "offline"})
    ));

    availability.advance_hours(25);
    assert!(pings(&availability, "remote").is_empty());

    availability.advance_hours(3);
    assert!(pings(&availability, "remote").is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_retrieve_device_state_when_it_reconnects() {
    let mut availability = setup();
    availability.start();

    availability.on_device_announce("bulb_color");
    availability.advance_seconds(1);
    availability.on_device_announce("bulb_color");
    assert_eq!(availability.endpoint_reads, 0);

    availability.advance_seconds(2);
    assert_eq!(availability.endpoint_reads, 1);

    availability.endpoint_reads = 0;
    availability.on_device_announce("bulb_color");
    availability.advance_seconds(3);
    assert_eq!(availability.endpoint_reads, 1);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_stop_pinging_device_when_it_is_removed() {
    let mut availability = setup();
    availability.start();

    availability.advance_minutes(9);
    assert!(pings(&availability, "bulb_color").is_empty());

    availability.on_device_remove("bulb_color");

    availability.advance_minutes(3);
    assert!(pings(&availability, "bulb_color").is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_stop_pinging_device_when_it_leaves() {
    let mut availability = setup();
    availability.start();

    availability.advance_minutes(9);
    assert!(pings(&availability, "bulb_color").is_empty());

    availability.on_device_leave("bulb_color");

    availability.advance_minutes(3);
    assert!(pings(&availability, "bulb_color").is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn shouldnt_do_anything_for_a_device_when_availability_false_is_set_for_device() {
    let mut availability = setup();
    availability.start();

    availability.on_last_seen_changed("bulb_color_2");
    availability.advance_minutes(12);

    assert!(pings(&availability, "bulb_color_2").is_empty());
}

#[test]
#[ignore = "not implemented yet"]
fn allows_to_disable_backoff() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn clamps_the_ping_delay_to_the_maximum_supported_timeout() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn does_not_trigger_backoff_on_ping_success() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn jitters_pings() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn pauses_when_backoff_reaches_configured_value_and_unpauses_on_last_seen() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn resets_backoff_on_last_seen() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn triggers_backoff_on_successive_ping_failures() {
    todo!()
}
