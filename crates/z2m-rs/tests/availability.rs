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

fn setup_with_jitter_devices() -> Availability {
    let mut availability = setup();
    for device in [
        "TS0601_thermostat",
        "bulb_2",
        "ZNCZ02LM",
        "GLEDOPTO_2ID",
        "QBKG03LM",
        "hue_twilight",
    ] {
        availability.add_device(device, false);
    }
    availability
}

fn total_pings(availability: &Availability, devices: &[&str]) -> usize {
    devices
        .iter()
        .map(|device| pings(availability, device).len())
        .sum()
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

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_prevent_instance_restart() {
    let mut availability = setup();
    availability.start();
    availability.stop();

    let result = availability.start_checked().await;
    assert!(result.is_err());
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

#[tokio::test]
#[ignore = "not implemented yet"]
async fn allows_to_disable_backoff() {
    let mut availability = setup();
    availability.set_max_jitter(0);
    availability.set_backoff_enabled(false);
    availability.start();
    availability.on_last_seen_changed("bulb_color");
    availability.set_fail_first_ping("bulb_color");

    availability.advance_seconds(10 * 60 + 30); // 10:30
    assert_eq!(pings(&availability, "bulb_color"), vec![true, false]);

    availability.advance_seconds(9 * 60 + 30); // 20:00
    assert_eq!(pings(&availability, "bulb_color").len(), 2);

    availability.advance_seconds(30); // 20:30
    assert_eq!(pings(&availability, "bulb_color").len(), 3);

    availability.advance_seconds(9 * 60 + 30); // 30:00
    assert_eq!(pings(&availability, "bulb_color").len(), 3);

    availability.advance_seconds(30); // 30:30
    assert_eq!(pings(&availability, "bulb_color").len(), 4);

    availability.advance_seconds(9 * 60 + 30); // 40:00
    assert_eq!(pings(&availability, "bulb_color").len(), 4);

    availability.advance_seconds(30); // 40:30
    assert_eq!(pings(&availability, "bulb_color").len(), 5);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn clamps_the_ping_delay_to_the_maximum_supported_timeout() {
    let mut availability = setup();
    // ~27.8 days
    availability.set_timeout("bulb_color", 40000);
    availability.set_device_max_jitter("bulb_color", 0);
    availability.start();

    // unclamped, the delay collapses to 1ms, so pings would already be looping by now
    availability.advance_seconds(1);

    assert!(pings(&availability, "bulb_color").is_empty());
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn does_not_trigger_backoff_on_ping_success() {
    let mut availability = setup();
    availability.set_max_jitter(0);
    availability.start();
    availability.on_last_seen_changed("bulb_color");

    availability.advance_seconds(10 * 60 + 30); // 10:30
    assert_eq!(pings(&availability, "bulb_color"), vec![true]);

    availability.advance_seconds(10 * 60 + 30); // 21:00
    assert_eq!(pings(&availability, "bulb_color").len(), 2);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn jitters_pings() {
    let mut availability = setup_with_jitter_devices();
    availability.set_device_max_jitter("hue_twilight", 1000);
    availability.set_device_backoff_enabled("GLEDOPTO_2ID", false);
    availability.set_device_pause_on_backoff_gt("QBKG03LM", 1.5);
    availability.start();

    let devices = [
        "bulb_color",
        "TS0601_thermostat",
        "bulb_2",
        "ZNCZ02LM",
        "GLEDOPTO_2ID",
        "QBKG03LM",
        "hue_twilight",
    ];

    availability.advance_minutes(10);
    assert_eq!(total_pings(&availability, &devices), 0);

    availability.advance_seconds(2); // 10:02
    assert_eq!(pings(&availability, "hue_twilight").len(), 1); // max_jitter: 1000

    availability.advance_seconds(8); // 10:10
    let called = total_pings(&availability, &devices);
    assert!(called < 7);

    availability.advance_seconds(20); // 10:20
    let called = total_pings(&availability, &devices);

    availability.advance_seconds(35); // 10:35
    assert_eq!(total_pings(&availability, &devices), 7);
    let _ = called;

    for device in devices {
        assert_eq!(pings(&availability, device).len(), 1);
    }
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn pauses_when_backoff_reaches_configured_value_and_unpauses_on_last_seen() {
    let mut availability = setup_with_jitter_devices();
    availability.set_max_jitter(0);
    availability.set_pause_on_backoff_gt(3.0);
    availability.set_device_pause_on_backoff_gt("QBKG03LM", 1.5);
    availability.start();
    availability.on_last_seen_changed("bulb_color");
    availability.on_last_seen_changed("QBKG03LM");

    availability.set_fail_first_ping("bulb_color");
    availability.set_fail_first_ping("QBKG03LM");

    availability.advance_seconds(10 * 60 + 30); // 10:30
    assert_eq!(pings(&availability, "bulb_color"), vec![true, false]);
    assert_eq!(pings(&availability, "QBKG03LM"), vec![true, false]);

    availability.advance_seconds(14 * 60 + 30); // 25:00
    assert_eq!(pings(&availability, "bulb_color").len(), 2);
    assert_eq!(pings(&availability, "QBKG03LM").len(), 2);

    availability.advance_seconds(30); // 25:30
    assert_eq!(pings(&availability, "bulb_color").len(), 3);
    assert_eq!(pings(&availability, "QBKG03LM").len(), 3);

    availability.advance_seconds(29 * 60 + 30); // 55:00
    assert_eq!(pings(&availability, "bulb_color").len(), 3);
    assert_eq!(pings(&availability, "QBKG03LM").len(), 3);

    availability.advance_seconds(30); // 55:30
    assert_eq!(pings(&availability, "bulb_color").len(), 4);
    assert_eq!(pings(&availability, "QBKG03LM").len(), 3);

    availability.advance_seconds(100 * 60);
    assert_eq!(pings(&availability, "bulb_color").len(), 4);
    assert_eq!(pings(&availability, "QBKG03LM").len(), 3);

    availability.advance_seconds(100 * 60);
    assert_eq!(pings(&availability, "bulb_color").len(), 4);
    assert_eq!(pings(&availability, "QBKG03LM").len(), 3);

    availability.on_last_seen_changed("bulb_color");
    availability.on_last_seen_changed("QBKG03LM");

    availability.advance_seconds(10 * 60 + 30);
    assert_eq!(pings(&availability, "bulb_color").len(), 5);
    assert_eq!(pings(&availability, "QBKG03LM").len(), 4);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn resets_backoff_on_last_seen() {
    let mut availability = setup();
    availability.set_max_jitter(0);
    availability.start();
    availability.on_last_seen_changed("bulb_color");

    availability.set_fail_first_ping("bulb_color");

    availability.advance_seconds(10 * 60 + 30); // 10:30
    assert_eq!(pings(&availability, "bulb_color"), vec![true, false]);

    availability.on_last_seen_changed("bulb_color");

    availability.advance_seconds(10 * 60 + 30); // 21:00
    assert_eq!(pings(&availability, "bulb_color").len(), 3);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn triggers_backoff_on_successive_ping_failures() {
    let mut availability = setup_with_jitter_devices();
    availability.set_max_jitter(0);
    availability.set_device_backoff_enabled("GLEDOPTO_2ID", false);
    availability.start();
    availability.on_last_seen_changed("bulb_color");
    availability.on_last_seen_changed("GLEDOPTO_2ID");

    availability.set_fail_first_ping("bulb_color");
    availability.set_fail_first_ping("GLEDOPTO_2ID");

    availability.advance_seconds(10 * 60 + 30); // 10:30
    assert_eq!(pings(&availability, "bulb_color"), vec![true, false]);
    assert_eq!(pings(&availability, "GLEDOPTO_2ID"), vec![true, false]);

    availability.advance_seconds(14 * 60 + 30); // 25:00
    assert_eq!(pings(&availability, "bulb_color").len(), 2);
    assert_eq!(pings(&availability, "GLEDOPTO_2ID").len(), 3);

    availability.advance_seconds(30); // 25:30
    assert_eq!(pings(&availability, "bulb_color").len(), 3);
    assert_eq!(pings(&availability, "GLEDOPTO_2ID").len(), 3);

    availability.advance_seconds(29 * 60 + 30); // 55:00
    assert_eq!(pings(&availability, "bulb_color").len(), 3);
    assert_eq!(pings(&availability, "GLEDOPTO_2ID").len(), 6);

    availability.advance_seconds(30); // 55:30
    assert_eq!(pings(&availability, "bulb_color").len(), 4);
    assert_eq!(pings(&availability, "GLEDOPTO_2ID").len(), 6);

    availability.advance_seconds(59 * 60 + 30); // 115:00
    assert_eq!(pings(&availability, "bulb_color").len(), 4);
    assert_eq!(pings(&availability, "GLEDOPTO_2ID").len(), 12);

    availability.advance_seconds(30); // 115:30
    assert_eq!(pings(&availability, "bulb_color").len(), 5);
    assert_eq!(pings(&availability, "GLEDOPTO_2ID").len(), 12);

    availability.advance_seconds(10 * 60 + 30); // 126:00
    assert_eq!(pings(&availability, "bulb_color").len(), 6);
    assert_eq!(pings(&availability, "GLEDOPTO_2ID").len(), 13);
}
