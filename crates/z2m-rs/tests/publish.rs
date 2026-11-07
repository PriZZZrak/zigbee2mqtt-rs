// Original: .upstream/zigbee2mqtt/test/extensions/publish.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Extension: Publish
//   - HS2WD-E burglar warning
//   - HS2WD-E emergency warning
//   - HS2WD-E emergency without level
//   - HS2WD-E wrong payload (should use defaults)
//   - Home Assistant: should not set state when color is also set
//   - Home Assistant: should not set state when color temperature is also set and device is already on
//   - Home Assistant: should set state
//   - Home Assistant: should set state when color temperature is also set and device is off
//   - Log an error when entity is not found
//   - Message transition should overrule options transition
//   - Scenes
//   - Should allow to invert cover
//   - Should allow to set color via hue and saturation
//   - Should also use on/off cluster when controlling group with switch
//   - Should corretly handle mallformed messages
//   - Should create and publish to group which is in configuration.yaml but not in zigbee-herdsman
//   - Should handle get
//   - Should handle get with multiple cover endpoints
//   - Should handle get with multiple endpoints
//   - Should handle non-valid topics
//   - Should log as error when setting property with no defined converter
//   - Should log error when device has no definition
//   - Should log error when device has no such endpoint (via property)
//   - Should log error when device has no such endpoint (via topic)
//   - Should not publish messages to zigbee devices when payload is invalid
//   - Should not use state converter on non-json message when value is not on/off/toggle
//   - Should parse set with and slashes in base and deviceID postfix topic
//   - Should parse set with attribute topic
//   - Should parse set with color attribute topic
//   - Should parse set with ieeeAddr topic
//   - Should parse set with non-existing postfix
//   - Should parse set with number at the end of friendly_name and postfix
//   - Should parse set with postfix topic and attribute
//   - Should parse topic with when base and deviceID have multiple slashes
//   - Should parse topic with when base topic has multiple slashes
//   - Should parse topic with when deviceID has multiple slashes
//   - Should publish brightness_move down to zigbee devices
//   - Should publish brightness_move up to zigbee devices
//   - Should publish correct state on toggle command to zigbee bulb
//   - Should publish messages groups
//   - Should publish messages to Tuya cover switch with dummy endpoints
//   - Should publish messages to Tuya switch with dummy endpoints
//   - Should publish messages to group with just 1 Hue Twilight in int (convers have an `endpoint` on it)
//   - Should publish messages to groups color
//   - Should publish messages to groups color temperature
//   - Should publish messages to groups when converter is not in the default list but device in it supports it
//   - Should publish messages to groups with brightness_percent
//   - Should publish messages to groups with off and brightness
//   - Should publish messages to groups with on and brightness
//   - Should publish messages to zigbee devices
//   - Should publish messages to zigbee devices to non default-ep with brightness_[EP]
//   - Should publish messages to zigbee devices to non default-ep with state_[EP]
//   - Should publish messages to zigbee devices when brightness is in %
//   - Should publish messages to zigbee devices when brightness is in number
//   - Should publish messages to zigbee devices when there is a get converter but no set
//   - Should publish messages to zigbee devices when there is no converters
//   - Should publish messages to zigbee devices with brightness
//   - Should publish messages to zigbee devices with color rgb object
//   - Should publish messages to zigbee devices with color rgb string
//   - Should publish messages to zigbee devices with color xy
//   - Should publish messages to zigbee devices with color xy and brightness
//   - Should publish messages to zigbee devices with color xy and state
//   - Should publish messages to zigbee devices with color xy, brightness and state off
//   - Should publish messages to zigbee devices with color xy, brightness and state on
//   - Should publish messages to zigbee devices with color_temp
//   - Should publish messages to zigbee devices with color_temp in %
//   - Should publish messages to zigbee devices with complicated topic
//   - Should publish messages to zigbee devices with endpoint ID
//   - Should publish messages to zigbee devices with non-default ep
//   - Should publish messages to zigbee devices with non-default ep and postfix
//   - Should publish messages with options disableDefaultResponse
//   - Should publish separate genOnOff to GL-S-007ZS when setting state and brightness as bulb doesnt turn on with moveToLevelWithOnOff
//   - Should publish state to roller shutter
//   - Should publish to MKS-CM-W5
//   - Should restore brightness when its turned off without transition and is turned on with
//   - Should restore brightness when its turned on with transition, Z2M is restarted and turned on again
//   - Should send state update on toggle specific endpoint
//   - Should set state before color
//   - Should set state with brightness before color
//   - Should sync colors
//   - Should turn bulb on with correct brightness when device is turned off twice and brightness is reported
//   - Should turn bulb on with full brightness when transition is used and no brightness is known
//   - Should turn device off when brightness 0 is send
//   - Should turn device off when brightness 0 is send with transition
//   - Should turn device off with onOff on off with transition
//   - Should turn device on with on/off when transition is provided
//   - Should turn device on with on/off with transition when transition 0 is provided
//   - Should use transition from device_options on brightness command
//   - Should use transition on brightness command
//   - Should use transition only once when setting brightness and color temperature for TRADFRI
//   - Should use transition only once when setting brightness and color temperature for group which contains TRADFRI
//   - Should use transition when brightness with group
//   - Should use transition when color temp
//   - Shouldnt do anything when device is not supported
//   - Shouldnt publish new brightness state when filtered_optimistic is used
//   - Shouldnt publish new state when optimistic = false
//   - Shouldnt publish new state when optimistic = false for group
//   - Shouldnt use moveToLevelWithOnOff on turn on when no transition has been used as some devices do not turn on in that case
//   - Transition parameter should not influence brightness on state ON
//   - When device is turned off and on with transition with report enabled it should restore correct brightness
//   - When device is turned off with transition and turned on WITHOUT transition it should restore the brightness
//   - ZNCLDJ11LM open
//   - ZNCLDJ11LM position

use serde_json::json;
use z2m_rs::extension::publish::Publish;

fn setup() -> Publish {
    let mut publish = Publish::new();
    publish.add_device("bulb_color", &["1"]);
    publish.add_device("thermostat", &["1"]);
    publish
        .devices
        .get_mut("thermostat")
        .unwrap()
        .has_set_converter = false;
    publish.add_device("wall_switch", &["1", "2"]);
    publish.add_device("wall_switch_double", &["1", "2", "3"]);
    publish.add_group("group_1");
    publish.add_group("group_2");
    publish.add_group("thermostat_group");
    publish.add_group("hue_twilight_group");
    publish
}

fn opts() -> serde_json::Value {
    json!({"qos": 0, "retain": false})
}

fn has(publish: &Publish, topic: &str, payload: serde_json::Value) -> bool {
    publish
        .published
        .contains(&(topic.to_string(), payload, opts()))
}

fn nothing_published(publish: &Publish) -> bool {
    publish.published.is_empty()
        && publish.devices.values().all(|device| {
            device
                .endpoints
                .values()
                .all(|endpoint| endpoint.commands.is_empty())
        })
}

#[test]
#[ignore = "not implemented yet"]
fn hs2_wd_e_burglar_warning() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn hs2_wd_e_emergency_warning() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn hs2_wd_e_emergency_without_level() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn hs2_wd_e_wrong_payload_should_use_defaults() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn home_assistant_should_not_set_state_when_color_is_also_set() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn home_assistant_should_not_set_state_when_color_temperature_is_also_set_and_device_is_already_on()
{
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn home_assistant_should_set_state() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn home_assistant_should_set_state_when_color_temperature_is_also_set_and_device_is_off() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn log_an_error_when_entity_is_not_found() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn message_transition_should_overrule_options_transition() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn scenes() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_allow_to_invert_cover() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_allow_to_set_color_via_hue_and_saturation() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_also_use_on_off_cluster_when_controlling_group_with_switch() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_corretly_handle_mallformed_messages() {
    let mut publish = setup();

    publish.on_mqtt_raw("zigbee2mqtt/foo", "");
    publish.on_mqtt_raw("zigbee2mqtt/bulb_color/set", "");

    assert!(nothing_published(&publish));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_create_and_publish_to_group_which_is_in_configuration_yaml_but_not_in_zigbee_herdsman(
) {
    let mut publish = setup();
    publish.add_group("group_12312");
    assert_eq!(publish.groups.len(), 5);

    publish.on_mqtt("zigbee2mqtt/group_12312/set", &json!({"state": "ON"}));

    assert_eq!(publish.groups.len(), 5);
    // group contains no device
    assert!(publish.group("group_12312").commands.is_empty());
}

#[test]
#[ignore = "not implemented yet"]
fn should_handle_get() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_handle_get_with_multiple_cover_endpoints() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_handle_get_with_multiple_endpoints() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_handle_non_valid_topics() {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt1/bulb_color/set", &json!({"state": "ON"}));

    assert!(nothing_published(&publish));
}

#[test]
#[ignore = "not implemented yet"]
fn should_log_as_error_when_setting_property_with_no_defined_converter() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_log_error_when_device_has_no_definition() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_log_error_when_device_has_no_such_endpoint_via_property() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_log_error_when_device_has_no_such_endpoint_via_topic() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_not_publish_messages_to_zigbee_devices_when_payload_is_invalid() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_not_use_state_converter_on_non_json_message_when_value_is_not_on_off_toggle() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_parse_set_with_and_slashes_in_base_and_device_id_postfix_topic() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_parse_set_with_attribute_topic() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_parse_set_with_color_attribute_topic() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_parse_set_with_ieee_addr_topic() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_parse_set_with_non_existing_postfix() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_parse_set_with_number_at_the_end_of_friendly_name_and_postfix() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_parse_set_with_postfix_topic_and_attribute() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_parse_topic_with_when_base_and_device_id_have_multiple_slashes() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_parse_topic_with_when_base_topic_has_multiple_slashes() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_parse_topic_with_when_device_id_has_multiple_slashes() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_brightness_move_down_to_zigbee_devices() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_brightness_move_up_to_zigbee_devices() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_correct_state_on_toggle_command_to_zigbee_bulb() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_groups() {
    let mut publish = setup();
    publish.add_group_member("group_1", "bulb_color", "1");

    publish.on_mqtt("zigbee2mqtt/group_1/set", &json!({"state": "ON"}));

    assert_eq!(
        publish.group("group_1").commands,
        vec![(
            "genOnOff".to_string(),
            "on".to_string(),
            json!({}),
            json!({})
        )]
    );
    assert_eq!(publish.published.len(), 2);
    assert_eq!(publish.published[1].0, "zigbee2mqtt/group_1");
    assert_eq!(publish.published[1].1, json!({"state": "ON"}));
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_messages_to_tuya_cover_switch_with_dummy_endpoints() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_messages_to_tuya_switch_with_dummy_endpoints() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_group_with_just_1_hue_twilight_in_int_convers_have_an_endpoint_on_it(
) {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/hue_twilight_group/set",
        &json!({"state": "ON"}),
    );

    assert_eq!(
        publish.group("hue_twilight_group").commands,
        vec![(
            "genOnOff".to_string(),
            "on".to_string(),
            json!({}),
            json!({})
        )]
    );
    assert_eq!(publish.published.len(), 2);
    assert_eq!(publish.published[1].0, "zigbee2mqtt/hue_twilight_group");
    assert_eq!(publish.published[1].1, json!({"state": "ON"}));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_groups_color() {
    let mut publish = setup();
    publish.add_group_member("group_1", "bulb_color", "1");

    publish.on_mqtt(
        "zigbee2mqtt/group_1/set",
        &json!({"color": {"x": 0.37, "y": 0.28}}),
    );

    assert_eq!(
        publish.group("group_1").commands,
        vec![(
            "lightingColorCtrl".to_string(),
            "moveToColor".to_string(),
            json!({"colorx": 24248, "colory": 18350, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert_eq!(publish.published.len(), 2);
    assert_eq!(publish.published[1].0, "zigbee2mqtt/group_1");
    assert_eq!(
        publish.published[1].1,
        json!({"color": {"x": 0.37, "y": 0.28}, "color_mode": "xy"})
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_groups_color_temperature() {
    let mut publish = setup();
    publish.add_group_member("group_1", "bulb_color", "1");

    publish.on_mqtt("zigbee2mqtt/group_1/set", &json!({"color_temp": 100}));

    assert_eq!(
        publish.group("group_1").commands,
        vec![(
            "lightingColorCtrl".to_string(),
            "moveToColorTemp".to_string(),
            json!({"colortemp": 100, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert_eq!(publish.published.len(), 2);
    assert_eq!(publish.published[1].0, "zigbee2mqtt/group_1");
    assert_eq!(
        publish.published[1].1,
        json!({"color_temp": 100, "color_mode": "color_temp"})
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_groups_when_converter_is_not_in_the_default_list_but_device_in_it_supports_it(
) {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/thermostat_group/set",
        &json!({"child_lock": "LOCK"}),
    );

    assert_eq!(
        publish.group("thermostat_group").commands,
        vec![(
            "manuSpecificTuya".to_string(),
            "dataRequest".to_string(),
            json!({"dpValues": [{"data": [1], "datatype": 1, "dp": 7}], "seq": 0}),
            json!({"disableDefaultResponse": true})
        )]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_groups_with_brightness_percent() {
    let mut publish = setup();
    publish.add_group_member("group_1", "bulb_color", "1");

    publish.on_mqtt(
        "zigbee2mqtt/group_1/set",
        &json!({"brightness_percent": 50}),
    );

    assert_eq!(
        publish.group("group_1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 128, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert_eq!(publish.published.len(), 2);
    assert_eq!(publish.published[1].0, "zigbee2mqtt/group_1");
    assert_eq!(
        publish.published[1].1,
        json!({"state": "ON", "brightness": 128})
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_groups_with_off_and_brightness() {
    let mut publish = setup();
    publish.add_group_member("group_1", "bulb_color", "1");

    publish.on_mqtt(
        "zigbee2mqtt/group_1/set",
        &json!({"state": "OFF", "brightness": 50}),
    );

    assert_eq!(
        publish.group("group_1").commands,
        vec![(
            "genOnOff".to_string(),
            "off".to_string(),
            json!({}),
            json!({})
        )]
    );
    assert_eq!(publish.published.len(), 2);
    assert_eq!(publish.published[1].0, "zigbee2mqtt/group_1");
    assert_eq!(publish.published[1].1, json!({"state": "OFF"}));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_groups_with_on_and_brightness() {
    let mut publish = setup();
    publish.add_group_member("group_1", "bulb_color", "1");

    publish.on_mqtt(
        "zigbee2mqtt/group_1/set",
        &json!({"state": "ON", "brightness": 50}),
    );

    assert_eq!(
        publish.group("group_1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 50, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert_eq!(publish.published.len(), 2);
    assert_eq!(publish.published[1].0, "zigbee2mqtt/group_1");
    assert_eq!(
        publish.published[1].1,
        json!({"state": "ON", "brightness": 50})
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices() {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"brightness": "200"}));

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 200, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert_eq!(publish.published.len(), 1);
    assert!(has(
        &publish,
        "zigbee2mqtt/bulb_color",
        json!({"brightness": 200, "state": "ON"})
    ));
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_messages_to_zigbee_devices_to_non_default_ep_with_brightness_ep() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_messages_to_zigbee_devices_to_non_default_ep_with_state_ep() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_when_brightness_is_in() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"brightness_percent": "92"}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 235, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/bulb_color",
        json!({"state": "ON", "brightness": 235})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_when_brightness_is_in_number() {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"brightness": 230}));

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 230, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/bulb_color",
        json!({"state": "ON", "brightness": 230})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_when_there_is_a_get_converter_but_no_set() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/thermostat/set",
        &json!({"relay_status_log_rsp": "200"}),
    );

    assert!(nothing_published(&publish));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_when_there_is_no_converters() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"brightness_no": "200"}),
    );

    assert!(nothing_published(&publish));
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_messages_to_zigbee_devices_with_brightness() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_messages_to_zigbee_devices_with_color_rgb_object() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_messages_to_zigbee_devices_with_color_rgb_string() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_messages_to_zigbee_devices_with_color_xy() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_messages_to_zigbee_devices_with_color_xy_and_brightness() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_messages_to_zigbee_devices_with_color_xy_and_state() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_messages_to_zigbee_devices_with_color_xy_brightness_and_state_off() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_messages_to_zigbee_devices_with_color_xy_brightness_and_state_on() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_with_color_temp() {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"color_temp": "222"}));

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "lightingColorCtrl".to_string(),
            "moveToColorTemp".to_string(),
            json!({"colortemp": 222, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/bulb_color",
        json!({"color_mode": "color_temp", "color_temp": 222})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_with_color_temp_in() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"color_temp_percent": 100}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "lightingColorCtrl".to_string(),
            "moveToColorTemp".to_string(),
            json!({"colortemp": 500, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/bulb_color",
        json!({"color_mode": "color_temp", "color_temp": 500})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_with_complicated_topic() {
    let mut publish = setup();
    publish.devices.get_mut("bulb_color").unwrap().friendly_name =
        "wohnzimmer.light.wall.right".to_string();

    publish.on_mqtt(
        "zigbee2mqtt/wohnzimmer.light.wall.right/set",
        &json!({"state": "ON"}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "genOnOff".to_string(),
            "on".to_string(),
            json!({}),
            json!({})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/wohnzimmer.light.wall.right",
        json!({"state": "ON"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_with_endpoint_id() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/wall_switch_double/3/set",
        &json!({"state": "OFF"}),
    );

    assert_eq!(
        publish.endpoint("wall_switch_double", "3").commands,
        vec![(
            "genOnOff".to_string(),
            "off".to_string(),
            json!({}),
            json!({})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/wall_switch_double",
        json!({"state_right": "OFF"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_with_non_default_ep() {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt/wall_switch/set", &json!({"state": "OFF"}));

    assert_eq!(
        publish.endpoint("wall_switch", "2").commands,
        vec![(
            "genOnOff".to_string(),
            "off".to_string(),
            json!({}),
            json!({})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/wall_switch",
        json!({"state": "OFF"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_with_non_default_ep_and_postfix() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/wall_switch_double/right/set",
        &json!({"state": "OFF"}),
    );

    assert_eq!(
        publish.endpoint("wall_switch_double", "3").commands,
        vec![(
            "genOnOff".to_string(),
            "off".to_string(),
            json!({}),
            json!({})
        )]
    );
    assert_eq!(publish.published.len(), 2);
    assert_eq!(publish.published[1].0, "zigbee2mqtt/wall_switch_double");
    assert_eq!(publish.published[1].1, json!({"state_right": "OFF"}));
    assert_eq!(publish.published[1].2, opts());
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_messages_with_options_disable_default_response() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_separate_gen_on_off_to_gl_s_007_zs_when_setting_state_and_brightness_as_bulb_doesnt_turn_on_with_move_to_level_with_on_off(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_state_to_roller_shutter() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_publish_to_mks_cm_w5() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_restore_brightness_when_its_turned_off_without_transition_and_is_turned_on_with() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_restore_brightness_when_its_turned_on_with_transition_z2_m_is_restarted_and_turned_on_again(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_send_state_update_on_toggle_specific_endpoint() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_set_state_before_color() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_set_state_with_brightness_before_color() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_sync_colors() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_turn_bulb_on_with_correct_brightness_when_device_is_turned_off_twice_and_brightness_is_reported(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_turn_bulb_on_with_full_brightness_when_transition_is_used_and_no_brightness_is_known() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_turn_device_off_when_brightness_0_is_send() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_turn_device_off_when_brightness_0_is_send_with_transition() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_turn_device_off_with_on_off_on_off_with_transition() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_turn_device_on_with_on_off_when_transition_is_provided() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_turn_device_on_with_on_off_with_transition_when_transition_0_is_provided() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_use_transition_from_device_options_on_brightness_command() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_use_transition_on_brightness_command() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_use_transition_only_once_when_setting_brightness_and_color_temperature_for_tradfri() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_use_transition_only_once_when_setting_brightness_and_color_temperature_for_group_which_contains_tradfri(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_use_transition_when_brightness_with_group() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_use_transition_when_color_temp() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn shouldnt_do_anything_when_device_is_not_supported() {
    todo!()
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn shouldnt_publish_new_brightness_state_when_filtered_optimistic_is_used() {
    let mut publish = setup();
    publish
        .devices
        .get_mut("bulb_color")
        .unwrap()
        .filtered_optimistic = vec!["brightness".to_string()];

    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"brightness": "200"}));

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 200, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert_eq!(publish.published.len(), 1);
    assert_eq!(publish.published[0].1, json!({"state": "ON"}));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn shouldnt_publish_new_state_when_optimistic_false() {
    let mut publish = setup();
    publish.devices.get_mut("bulb_color").unwrap().optimistic = false;

    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"brightness": "200"}));

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 200, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert_eq!(publish.published.len(), 0);
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn shouldnt_publish_new_state_when_optimistic_false_for_group() {
    let mut publish = setup();
    publish.group("group_2").optimistic = false;

    publish.on_mqtt("zigbee2mqtt/group_2/set", &json!({"brightness": "200"}));

    assert_eq!(publish.published.len(), 0);
}

#[test]
#[ignore = "not implemented yet"]
fn shouldnt_use_move_to_level_with_on_off_on_turn_on_when_no_transition_has_been_used_as_some_devices_do_not_turn_on_in_that_case(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn transition_parameter_should_not_influence_brightness_on_state_on() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn when_device_is_turned_off_and_on_with_transition_with_report_enabled_it_should_restore_correct_brightness(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn when_device_is_turned_off_with_transition_and_turned_on_without_transition_it_should_restore_the_brightness(
) {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zncldj11_lm_open() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn zncldj11_lm_position() {
    todo!()
}
