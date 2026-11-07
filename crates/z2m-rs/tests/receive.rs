// Original: .upstream/zigbee2mqtt/test/extensions/receive.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Extension: Receive
//   - Should NOT publish old messages from State cache during debouncing
//   - Should add elapsed
//   - Should allow to disable the legacy integration
//   - Should allow to invert cover
//   - Should debounce and retain messages when set via device_options
//   - Should debounce messages
//   - Should debounce messages only with the same payload values for provided debounce_ignore keys
//   - Should emit DevicesChanged event when a converter announces changed exposes
//   - Should handle a command
//   - Should handle a zigbee message
//   - Should handle a zigbee message which uses ep (left)
//   - Should handle a zigbee message which uses ep (right)
//   - Should handle a zigbee message with 0 precision
//   - Should handle a zigbee message with 1 precision
//   - Should handle a zigbee message with 1 precision when set via device_options
//   - Should handle a zigbee message with 2 precision when overrides device_options
//   - Should handle a zigbee message with default precision
//   - Should handle a zigbee message with voltage 2990
//   - Should handle messages from Xiaomi router devices
//   - Should log when message is from supported device but has no converters
//   - Should not bypass the debounce when a message produces no payload
//   - Should not handle messages from coordinator
//   - Should not handle messages from still interviewing devices with unknown definition
//   - Should publish 1 message when converted twice
//   - Should publish last_seen ISO_8601
//   - Should publish last_seen ISO_8601_local
//   - Should publish last_seen epoch
//   - Should report correct energy and power values for different versions of SP600
//   - Should throttle multiple messages from spamming devices
//   - Shouldnt republish old state

use serde_json::json;
use z2m_rs::extension::receive::Receive;

fn setup() -> Receive {
    let mut receive = Receive::new();
    receive.add_device("button", 0, false);
    receive.add_device("button_double_key", 0, false);
    receive.add_device("weather_sensor", 1, false);
    receive.add_device("J1_cover", 0, false);
    receive.add_device("occupancy_sensor", 0, false);
    receive.add_device("power_plug", 0, false);
    receive.add_device("bulb", 0, false);
    receive.add_device("ikea_onoff", 0, false);
    receive.add_device("SP600_NEW", 0, false);
    receive.add_device("SP600_OLD", 0, false);
    receive.add_device("spammer1", 0, true);
    receive.add_device("BMCT-SLZ", 0, false);
    receive
}

fn opts(qos: u8, retain: bool) -> serde_json::Value {
    json!({"retain": retain, "qos": qos})
}

fn has(receive: &Receive, topic: &str, payload: serde_json::Value, qos: u8, retain: bool) -> bool {
    receive
        .published
        .contains(&(topic.to_string(), payload, opts(qos, retain)))
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_not_publish_old_messages_from_state_cache_during_debouncing() {
    let mut receive = setup();
    receive.set_debounce(0.1);

    receive.on_message(
        "weather_sensor",
        "1",
        "msTemperatureMeasurement",
        &json!({"measuredValue": 8}),
        10,
    );
    receive.on_message(
        "weather_sensor",
        "1",
        "msRelativeHumidity",
        &json!({"measuredValue": 1}),
        10,
    );
    receive.on_message(
        "weather_sensor",
        "1",
        "msPressureMeasurement",
        &json!({"measuredValue": 2}),
        10,
    );

    receive.advance_time(50);
    assert_eq!(receive.published.len(), 0);
    receive.flush_debounce("weather_sensor");

    assert_eq!(receive.published.len(), 2);
    assert_eq!(
        receive.published[0].1,
        json!({"temperature": 0.08, "humidity": 0.01, "pressure": 2})
    );
    assert_eq!(receive.published[1].0, "zigbee2mqtt/bridge/health");

    receive.on_message(
        "weather_sensor",
        "1",
        "msTemperatureMeasurement",
        &json!({"measuredValue": 9}),
        10,
    );
    receive.publish_entity_state("weather_sensor", &json!({}));
    receive.advance_time(0);

    assert_eq!(receive.published.len(), 5);
    assert_eq!(receive.published[1].0, "zigbee2mqtt/bridge/health");
    assert_eq!(
        receive.published[2].1,
        json!({"temperature": 0.09, "humidity": 0.01, "pressure": 2})
    );
    assert_eq!(
        receive.published[3].1,
        json!({"temperature": 0.09, "humidity": 0.01, "pressure": 2})
    );
    assert_eq!(receive.published[4].0, "zigbee2mqtt/bridge/health");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_add_elapsed() {
    let mut receive = setup();
    receive.elapsed_enabled = true;
    receive.advance_time(150);

    receive.on_message("ikea_onoff", "1", "genLevelCtrl", &json!({}), 0);
    receive.advance_time(50);
    receive.on_message("ikea_onoff", "1", "genLevelCtrl", &json!({}), 0);

    assert_eq!(receive.published.len(), 2);
    assert!(has(
        &receive,
        "zigbee2mqtt/ikea_onoff",
        json!({"action": "brightness_stop"}),
        0,
        false
    ));
    assert_eq!(receive.published[1].1["action"], json!("brightness_stop"));
    assert_eq!(receive.published[1].1["elapsed"], json!(50));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_disable_the_legacy_integration() {
    let mut receive = setup();
    receive.device_mut("button").legacy = false;

    receive.on_message("button", "1", "genOnOff", &json!({"onOff": 1}), 10);

    assert_eq!(receive.published.len(), 1);
    assert!(has(
        &receive,
        "zigbee2mqtt/button",
        json!({"action": "single"}),
        0,
        false
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_allow_to_invert_cover() {
    let mut receive = setup();

    // Non-inverted (open = 100, close = 0)
    receive.on_message(
        "J1_cover",
        "1",
        "closuresWindowCovering",
        &json!({"currentPositionLiftPercentage": 90, "currentPositionTiltPercentage": 80}),
        10,
    );
    assert_eq!(receive.published.len(), 1);
    assert!(has(
        &receive,
        "zigbee2mqtt/J1_cover",
        json!({"position": 10, "tilt": 20, "state": "OPEN"}),
        0,
        false
    ));

    // Inverted
    receive.published.clear();
    receive.device_mut("J1_cover").invert_cover = true;
    receive.on_message(
        "J1_cover",
        "1",
        "closuresWindowCovering",
        &json!({"currentPositionLiftPercentage": 90, "currentPositionTiltPercentage": 80}),
        10,
    );
    assert_eq!(receive.published.len(), 1);
    assert!(has(
        &receive,
        "zigbee2mqtt/J1_cover",
        json!({"position": 90, "tilt": 80, "state": "OPEN"}),
        0,
        false
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_debounce_and_retain_messages_when_set_via_device_options() {
    let mut receive = setup();
    receive.set_debounce(0.1);
    receive.device_mut("weather_sensor").retain = true;

    receive.on_message(
        "weather_sensor",
        "1",
        "msTemperatureMeasurement",
        &json!({"measuredValue": 8}),
        10,
    );
    receive.on_message(
        "weather_sensor",
        "1",
        "msRelativeHumidity",
        &json!({"measuredValue": 1}),
        10,
    );
    receive.on_message(
        "weather_sensor",
        "1",
        "msPressureMeasurement",
        &json!({"measuredValue": 2}),
        10,
    );

    receive.advance_time(50);
    assert_eq!(receive.published.len(), 0);

    receive.flush_debounce("weather_sensor");
    assert_eq!(receive.published.len(), 2);
    assert!(has(
        &receive,
        "zigbee2mqtt/weather_sensor",
        json!({"temperature": 0.08, "humidity": 0.01, "pressure": 2}),
        1,
        true
    ));
    assert_eq!(receive.published[1].0, "zigbee2mqtt/bridge/health");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_debounce_messages() {
    let mut receive = setup();
    receive.set_debounce(0.1);

    receive.on_message(
        "weather_sensor",
        "1",
        "msTemperatureMeasurement",
        &json!({"measuredValue": 8}),
        10,
    );
    receive.on_message(
        "weather_sensor",
        "1",
        "msRelativeHumidity",
        &json!({"measuredValue": 1}),
        10,
    );
    receive.on_message(
        "weather_sensor",
        "1",
        "msPressureMeasurement",
        &json!({"measuredValue": 2}),
        10,
    );

    receive.advance_time(50);
    assert_eq!(receive.published.len(), 0);

    receive.flush_debounce("weather_sensor");
    assert_eq!(receive.published.len(), 2);
    assert!(has(
        &receive,
        "zigbee2mqtt/weather_sensor",
        json!({"temperature": 0.08, "humidity": 0.01, "pressure": 2}),
        1,
        false
    ));
    assert_eq!(receive.published[1].0, "zigbee2mqtt/bridge/health");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_debounce_messages_only_with_the_same_payload_values_for_provided_debounce_ignore_keys(
) {
    let mut receive = setup();
    receive.set_debounce(0.1);
    receive.debounce_ignore = vec!["temperature".to_string()];

    receive.on_message(
        "weather_sensor",
        "1",
        "msTemperatureMeasurement",
        &json!({"measuredValue": 8}),
        13,
    );
    receive.on_message(
        "weather_sensor",
        "1",
        "msPressureMeasurement",
        &json!({"measuredValue": 2}),
        13,
    );
    receive.on_message(
        "weather_sensor",
        "1",
        "msTemperatureMeasurement",
        &json!({"measuredValue": 7}),
        13,
    );
    receive.on_message(
        "weather_sensor",
        "1",
        "msRelativeHumidity",
        &json!({"measuredValue": 3}),
        13,
    );

    receive.advance_time(50);
    assert_eq!(receive.published.len(), 1);
    assert_eq!(
        receive.published[0].1,
        json!({"temperature": 0.08, "pressure": 2})
    );

    receive.flush_debounce("weather_sensor");
    assert_eq!(receive.published.len(), 3);
    assert_eq!(
        receive.published[1].1,
        json!({"temperature": 0.07, "pressure": 2, "humidity": 0.03})
    );
    assert_eq!(receive.published[2].0, "zigbee2mqtt/bridge/health");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_emit_devices_changed_event_when_a_converter_announces_changed_exposes() {
    let mut receive = setup();

    receive.on_message(
        "BMCT-SLZ",
        "1",
        "boschEnergyDevice",
        &json!({"deviceMode": 0}),
        10,
    );

    assert_eq!(receive.published[0].0, "zigbee2mqtt/bridge/devices");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_handle_a_command() {
    let mut receive = setup();

    receive.on_message("ikea_onoff", "1", "genLevelCtrl", &json!({}), 10);

    assert_eq!(receive.published.len(), 1);
    assert!(has(
        &receive,
        "zigbee2mqtt/ikea_onoff",
        json!({"action": "brightness_stop"}),
        0,
        false
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_handle_a_zigbee_message() {
    let mut receive = setup();
    receive.device_mut("button").legacy = true;

    receive.on_message("button", "1", "genOnOff", &json!({"onOff": 1}), 10);

    assert_eq!(receive.published.len(), 1);
    assert!(has(
        &receive,
        "zigbee2mqtt/button",
        json!({"action": "single", "linkquality": 10}),
        0,
        false
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_handle_a_zigbee_message_which_uses_ep_left() {
    let mut receive = setup();

    receive.on_message(
        "button_double_key",
        "1",
        "genOnOff",
        &json!({"onOff": 1}),
        10,
    );

    assert_eq!(receive.published.len(), 1);
    assert!(has(
        &receive,
        "zigbee2mqtt/button_double_key",
        json!({"action": "single_left"}),
        0,
        false
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_handle_a_zigbee_message_which_uses_ep_right() {
    let mut receive = setup();

    receive.on_message(
        "button_double_key",
        "2",
        "genOnOff",
        &json!({"onOff": 1}),
        10,
    );

    assert_eq!(receive.published.len(), 1);
    assert!(has(
        &receive,
        "zigbee2mqtt/button_double_key",
        json!({"action": "single_right"}),
        0,
        false
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_handle_a_zigbee_message_with_0_precision() {
    let mut receive = setup();
    receive.device_mut("weather_sensor").precision = Some(0);

    receive.on_message(
        "weather_sensor",
        "1",
        "msTemperatureMeasurement",
        &json!({"measuredValue": -85}),
        10,
    );

    assert_eq!(receive.published.len(), 1);
    assert!(has(
        &receive,
        "zigbee2mqtt/weather_sensor",
        json!({"temperature": -1}),
        1,
        false
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_handle_a_zigbee_message_with_1_precision() {
    let mut receive = setup();
    receive.device_mut("weather_sensor").precision = Some(1);

    receive.on_message(
        "weather_sensor",
        "1",
        "msTemperatureMeasurement",
        &json!({"measuredValue": -85}),
        10,
    );

    assert_eq!(receive.published.len(), 1);
    assert!(has(
        &receive,
        "zigbee2mqtt/weather_sensor",
        json!({"temperature": -0.8}),
        1,
        false
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_handle_a_zigbee_message_with_1_precision_when_set_via_device_options() {
    let mut receive = setup();
    receive.device_mut("weather_sensor").precision = Some(1);

    receive.on_message(
        "weather_sensor",
        "1",
        "msTemperatureMeasurement",
        &json!({"measuredValue": -85}),
        10,
    );

    assert_eq!(receive.published.len(), 1);
    assert!(has(
        &receive,
        "zigbee2mqtt/weather_sensor",
        json!({"temperature": -0.8}),
        1,
        false
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_handle_a_zigbee_message_with_2_precision_when_overrides_device_options() {
    let mut receive = setup();
    receive.device_mut("weather_sensor").precision = Some(0);

    receive.on_message(
        "weather_sensor",
        "1",
        "msTemperatureMeasurement",
        &json!({"measuredValue": -85}),
        10,
    );

    assert_eq!(receive.published.len(), 1);
    assert!(has(
        &receive,
        "zigbee2mqtt/weather_sensor",
        json!({"temperature": -1}),
        1,
        false
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_handle_a_zigbee_message_with_default_precision() {
    let mut receive = setup();

    receive.on_message(
        "weather_sensor",
        "1",
        "msTemperatureMeasurement",
        &json!({"measuredValue": -85}),
        10,
    );

    assert_eq!(receive.published.len(), 1);
    assert!(has(
        &receive,
        "zigbee2mqtt/weather_sensor",
        json!({"temperature": -0.85}),
        1,
        false
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_handle_a_zigbee_message_with_voltage_2990() {
    let mut receive = setup();

    receive.on_message(
        "button_double_key",
        "1",
        "genBasic",
        &json!({"65281": {"1": 2990}}),
        10,
    );

    assert_eq!(receive.published.len(), 1);
    assert!(has(
        &receive,
        "zigbee2mqtt/button_double_key",
        json!({"battery": 93, "voltage": 2990}),
        0,
        false
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_handle_messages_from_xiaomi_router_devices() {
    let mut receive = setup();
    receive.device_mut("power_plug").linked_group = Some("switch_group".to_string());

    receive.on_message("power_plug", "1", "genOnOff", &json!({"onOff": 1}), 20);

    assert_eq!(receive.published.len(), 2);
    assert!(has(
        &receive,
        "zigbee2mqtt/power_plug",
        json!({"state": "ON"}),
        0,
        false
    ));
    assert!(has(
        &receive,
        "zigbee2mqtt/switch_group",
        json!({"state": "ON"}),
        0,
        false
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_log_when_message_is_from_supported_device_but_has_no_converters() {
    let mut receive = setup();
    receive.add_device("ZNCZ02LM", 0, false);
    receive.device_mut("ZNCZ02LM").has_converters = false;

    receive.on_message(
        "ZNCZ02LM",
        "1",
        "genBinaryOutput",
        &json!({"inactiveText": "hello"}),
        20,
    );

    assert_eq!(receive.published.len(), 0);
    assert!(receive.logs.iter().any(|(level, message)| {
            level == "debug"
                && message.contains(
                    "No converter available for 'ZNCZ02LM' with cluster 'genBinaryOutput' and type 'attributeReport' and data '{\"inactiveText\":\"hello\"}'",
                )
        }));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_not_bypass_the_debounce_when_a_message_produces_no_payload() {
    let mut receive = setup();
    receive.set_debounce(0.1);
    receive.last_seen = Some("ISO_8601".to_string());

    // Attribute report without measuredValue: the lumi_temperature converter returns nothing.
    receive.on_message(
        "weather_sensor",
        "1",
        "msTemperatureMeasurement",
        &json!({}),
        10,
    );
    receive.advance_time(50);
    assert_eq!(receive.published.len(), 0);

    receive.flush_debounce("weather_sensor");
    assert_eq!(receive.published.len(), 2);
    assert_eq!(receive.published[0].0, "zigbee2mqtt/weather_sensor");
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_not_handle_messages_from_coordinator() {
    let mut receive = setup();
    receive.add_device("coordinator", 0, false);
    receive.device_mut("coordinator").is_coordinator = true;

    receive.on_message("coordinator", "1", "genOnOff", &json!({"onOff": 1}), 10);

    assert_eq!(receive.published.len(), 0);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_not_handle_messages_from_still_interviewing_devices_with_unknown_definition() {
    let mut receive = setup();
    receive.device_mut("bulb").interviewing = true;
    receive.device_mut("bulb").has_converters = false;

    receive.on_message("bulb", "1", "genOnOff", &json!({"onOff": 1}), 10);

    assert_eq!(receive.published.len(), 0);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_1_message_when_converted_twice() {
    let mut receive = setup();

    receive.on_message(
        "occupancy_sensor",
        "1",
        "genBasic",
        &json!({"65281": {"1": 3045, "3": 19, "5": 35, "6": [0, 3], "11": 381, "100": 0}}),
        10,
    );

    assert_eq!(receive.published.len(), 1);
    assert!(has(
        &receive,
        "zigbee2mqtt/occupancy_sensor",
        json!({
            "battery": 100,
            "illuminance": 381,
            "voltage": 3045,
            "device_temperature": 19,
            "power_outage_count": 34,
        }),
        0,
        false
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_last_seen_iso_8601() {
    let mut receive = setup();
    receive.last_seen = Some("ISO_8601".to_string());

    receive.on_message(
        "button_double_key",
        "1",
        "genOnOff",
        &json!({"onOff": 1}),
        10,
    );

    assert_eq!(receive.published.len(), 1);
    assert_eq!(receive.published[0].0, "zigbee2mqtt/button_double_key");
    assert!(receive.published[0].1["last_seen"].is_string());
    assert_eq!(receive.published[0].2, opts(0, false));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_last_seen_iso_8601_local() {
    let mut receive = setup();
    receive.last_seen = Some("ISO_8601_local".to_string());

    receive.on_message(
        "button_double_key",
        "1",
        "genOnOff",
        &json!({"onOff": 1}),
        10,
    );

    assert_eq!(receive.published.len(), 1);
    assert_eq!(receive.published[0].0, "zigbee2mqtt/button_double_key");
    assert!(receive.published[0].1["last_seen"].is_string());
    assert_eq!(receive.published[0].2, opts(0, false));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_last_seen_epoch() {
    let mut receive = setup();
    receive.last_seen = Some("epoch".to_string());

    receive.on_message(
        "button_double_key",
        "1",
        "genOnOff",
        &json!({"onOff": 1}),
        10,
    );

    assert_eq!(receive.published.len(), 1);
    assert_eq!(receive.published[0].0, "zigbee2mqtt/button_double_key");
    assert!(receive.published[0].1["last_seen"].is_number());
    assert_eq!(receive.published[0].2, opts(0, false));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_report_correct_energy_and_power_values_for_different_versions_of_sp600() {
    let mut receive = setup();
    let data = json!({"instantaneousDemand": 496, "currentSummDelivered": 6648});

    receive.on_message("SP600_NEW", "1", "seMetering", &data, 10);
    assert_eq!(receive.published.len(), 1);
    assert!(has(
        &receive,
        "zigbee2mqtt/SP600_NEW",
        json!({"energy": 0.66, "power": 49.6}),
        0,
        false
    ));

    receive.published.clear();
    receive.on_message("SP600_OLD", "1", "seMetering", &data, 10);
    assert_eq!(receive.published.len(), 1);
    assert!(has(
        &receive,
        "zigbee2mqtt/SP600_OLD",
        json!({"energy": 6.65, "power": 496}),
        0,
        false
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_throttle_multiple_messages_from_spamming_devices() {
    let mut receive = setup();
    receive.set_throttle(1.0);
    receive.device_mut("spammer1").retain = true;

    for value in [1, 2, 3] {
        receive.on_message(
            "spammer1",
            "1",
            "msTemperatureMeasurement",
            &json!({"measuredValue": value}),
            10,
        );
    }

    assert_eq!(receive.published.len(), 1);
    assert!(has(
        &receive,
        "zigbee2mqtt/spammer1",
        json!({"temperature": 0.01}),
        0,
        true
    ));

    receive.advance_time(2000);
    assert_eq!(receive.published.len(), 2);
    assert!(has(
        &receive,
        "zigbee2mqtt/spammer1",
        json!({"temperature": 0.03}),
        0,
        true
    ));

    receive.on_message(
        "spammer1",
        "1",
        "msTemperatureMeasurement",
        &json!({"measuredValue": 4}),
        10,
    );
    assert_eq!(receive.published.len(), 3);
    assert!(has(
        &receive,
        "zigbee2mqtt/spammer1",
        json!({"temperature": 0.04}),
        0,
        true
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn shouldnt_republish_old_state() {
    let mut receive = setup();
    receive.set_debounce(0.1);

    receive.on_message("bulb", "1", "genOnOff", &json!({"onOff": 0}), 10);
    receive.on_mqtt_set("bulb", &json!({"state": "ON"}));
    receive.flush_debounce("bulb");

    assert_eq!(receive.published.len(), 3);
    assert_eq!(receive.published[0].1, json!({"state": "ON"}));
    assert_eq!(receive.published[1].1, json!({"state": "ON"}));
    assert_eq!(receive.published[2].0, "zigbee2mqtt/bridge/health");
}

// Upstream has a second test with the same title `Should publish 1 message when converted twice`.
#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_1_message_when_converted_twice_2() {
    let mut receive = setup();

    receive.on_message(
        "occupancy_sensor",
        "1",
        "genBasic",
        &json!({"9999": {"1": 3045}}),
        10,
    );

    assert_eq!(receive.published.len(), 0);
}
