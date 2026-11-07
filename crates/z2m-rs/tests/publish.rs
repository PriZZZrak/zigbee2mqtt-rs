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
    publish.add_endpoint_alias("wall_switch_double", "left", "2");
    publish.add_endpoint_alias("wall_switch_double", "right", "3");
    publish
        .devices
        .get_mut("wall_switch_double")
        .unwrap()
        .friendly_name = "wall_switch_double".to_string();
    publish.add_device("zigfred_plus", &["11", "12"]);
    publish.add_device("J1_cover", &["1"]);
    publish.add_device("led_controller_1", &["11"]);
    publish.add_device("siren", &["1"]);
    publish.add_device("roller_shutter", &["1"]);
    publish.add_device("MKS-CM-W5", &["1"]);
    publish.add_endpoint_alias("MKS-CM-W5", "l3", "3");
    publish.add_device("GL-S-007ZS", &["1"]);
    publish.add_device("bulb", &["1"]);
    publish.add_device("curtain", &["1"]);
    publish.add_device("TS0601_switch", &["1"]);
    publish.add_device("TS0601_cover_switch", &["1"]);
    publish.add_device("QS-Zigbee-D02-TRIAC-2C-LN", &["2"]);
    publish.add_endpoint_alias("QS-Zigbee-D02-TRIAC-2C-LN", "l2", "2");
    publish.add_group("switch_group");
    publish.add_group("group_with_tradfri");
    publish.add_device("button_double_key_interviewing", &["1"]);
    publish
        .devices
        .get_mut("button_double_key_interviewing")
        .unwrap()
        .has_definition = false;
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

#[tokio::test]
async fn hs2_wd_e_burglar_warning() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/siren/set",
        &json!({"warning": {"duration": 100, "mode": "burglar", "strobe": true, "level": "high"}}),
    );

    assert_eq!(
        publish.endpoint("siren", "1").commands,
        vec![(
            "ssIasWd".to_string(),
            "startWarning".to_string(),
            json!({"startwarninginfo": 22, "warningduration": 100, "strobedutycycle": 0, "strobelevel": 1}),
            json!({"disableDefaultResponse": true})
        )]
    );
}

#[tokio::test]
async fn hs2_wd_e_emergency_warning() {
    let mut publish = setup();

    publish.on_mqtt(
            "zigbee2mqtt/siren/set",
            &json!({"warning": {"duration": 10, "mode": "emergency", "strobe": false, "level": "very_high"}}),
        );

    assert_eq!(
        publish.endpoint("siren", "1").commands,
        vec![(
            "ssIasWd".to_string(),
            "startWarning".to_string(),
            json!({"startwarninginfo": 51, "warningduration": 10, "strobedutycycle": 0, "strobelevel": 1}),
            json!({"disableDefaultResponse": true})
        )]
    );
}

#[tokio::test]
async fn hs2_wd_e_emergency_without_level() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/siren/set",
        &json!({"warning": {"duration": 10, "mode": "emergency", "strobe": false}}),
    );

    assert_eq!(
        publish.endpoint("siren", "1").commands,
        vec![(
            "ssIasWd".to_string(),
            "startWarning".to_string(),
            json!({"startwarninginfo": 49, "warningduration": 10, "strobedutycycle": 0, "strobelevel": 1}),
            json!({"disableDefaultResponse": true})
        )]
    );
}

#[tokio::test]
async fn hs2_wd_e_wrong_payload_should_use_defaults() {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt/siren/set", &json!({"warning": "wrong"}));

    assert_eq!(
        publish.endpoint("siren", "1").commands,
        vec![(
            "ssIasWd".to_string(),
            "startWarning".to_string(),
            json!({"startwarninginfo": 53, "warningduration": 10, "strobedutycycle": 0, "strobelevel": 1}),
            json!({"disableDefaultResponse": true})
        )]
    );
}

#[tokio::test]
async fn home_assistant_should_not_set_state_when_color_is_also_set() {
    let mut publish = setup();
    publish.homeassistant_enabled = true;
    publish.set_device_state("bulb_color", json!({"state": "ON"}));

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "ON", "color": {"x": 0.41, "y": 0.25}}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "lightingColorCtrl".to_string(),
            "moveToColor".to_string(),
            json!({"colorx": 26869, "colory": 16384, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/bulb_color",
        json!({"color": {"x": 0.41, "y": 0.25}, "state": "ON", "color_mode": "xy"})
    ));
}

#[tokio::test]
async fn home_assistant_should_not_set_state_when_color_temperature_is_also_set_and_device_is_already_on(
) {
    let mut publish = setup();
    publish.homeassistant_enabled = true;
    publish.set_device_state("bulb_color", json!({"state": "ON"}));

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "ON", "color_temp": 100}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "lightingColorCtrl".to_string(),
            "moveToColorTemp".to_string(),
            json!({"colortemp": 100, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/bulb_color",
        json!({"state": "ON", "color_temp": 100, "color_mode": "color_temp"})
    ));
}

#[tokio::test]
async fn home_assistant_should_set_state() {
    let mut publish = setup();
    publish.homeassistant_enabled = true;

    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"state": "ON"}));

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
        "zigbee2mqtt/bulb_color",
        json!({"state": "ON"})
    ));
}

#[tokio::test]
async fn home_assistant_should_set_state_when_color_temperature_is_also_set_and_device_is_off() {
    let mut publish = setup();
    publish.homeassistant_enabled = true;
    publish.set_device_state("bulb_color", json!({"state": "OFF"}));

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "ON", "color_temp": 100}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![
            (
                "genOnOff".to_string(),
                "on".to_string(),
                json!({}),
                json!({})
            ),
            (
                "lightingColorCtrl".to_string(),
                "moveToColorTemp".to_string(),
                json!({"colortemp": 100, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
        ]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/bulb_color",
        json!({"state": "ON", "color_temp": 100, "color_mode": "color_temp"})
    ));
}

#[tokio::test]
async fn log_an_error_when_entity_is_not_found() {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt/an_unknown_entity/set", &json!({}));

    assert!(publish.logs.iter().any(|(level, message, _)| {
        level == "error" && message == "Entity 'an_unknown_entity' is unknown"
    }));
}

#[tokio::test]
async fn message_transition_should_overrule_options_transition() {
    let mut publish = setup();
    publish.devices.get_mut("bulb_color").unwrap().transition = Some(20);

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"brightness": 200, "transition": 10}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 200, "transtime": 100, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn scenes() {
    let mut publish = setup();
    publish.add_group("group_tradfri_remote");
    publish.add_group_member("group_tradfri_remote", "bulb_color_2", "1");
    publish.add_group_member("group_tradfri_remote", "bulb_2", "1");
    publish.add_group("ha_discovery_group");
    publish.add_group_member("ha_discovery_group", "bulb_color_2", "1");
    publish.add_group_member("ha_discovery_group", "bulb_2", "1");
    publish.add_group_member("group_with_tradfri", "bulb_2", "1");
    publish.add_group_member("switch_group", "bulb_2", "1");

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color_2/set",
        &json!({"state": "ON", "brightness": 50, "color_temp": 290}),
    );
    publish.on_mqtt(
        "zigbee2mqtt/bulb_2/set",
        &json!({"state": "ON", "brightness": 100}),
    );

    publish.on_mqtt(
        "zigbee2mqtt/group_tradfri_remote/set",
        &json!({"scene_store": 1}),
    );
    assert_eq!(
        publish.group("group_tradfri_remote").commands,
        vec![(
            "genScenes".to_string(),
            "store".to_string(),
            json!({"groupid": 15071, "sceneid": 1}),
            json!({})
        )]
    );
    assert!(publish
        .published
        .iter()
        .any(|(topic, _, _)| topic == "zigbee2mqtt/bridge/devices"));
    assert!(publish
        .published
        .iter()
        .any(|(topic, _, _)| topic == "zigbee2mqtt/bridge/groups"));

    publish.group("group_tradfri_remote").commands.clear();
    publish.published.clear();
    publish.on_mqtt(
        "zigbee2mqtt/group_tradfri_remote/set",
        &json!({"scene_recall": 1}),
    );
    assert_eq!(
        publish.group("group_tradfri_remote").commands,
        vec![(
            "genScenes".to_string(),
            "recall".to_string(),
            json!({"groupid": 15071, "sceneid": 1, "transitionTime": 0xffff}),
            json!({})
        )]
    );
    assert_eq!(publish.published.len(), 8);
    assert_eq!(
        publish.published[0].1,
        json!({"brightness": 50, "color_temp": 290, "state": "ON", "color_mode": "color_temp"})
    );
}

#[tokio::test]
async fn should_allow_to_invert_cover() {
    let mut publish = setup();

    // Non-inverted (open = 100, close = 0)
    publish.on_mqtt("zigbee2mqtt/J1_cover/set", &json!({"position": 90}));
    assert_eq!(
        publish.endpoint("J1_cover", "1").commands,
        vec![(
            "closuresWindowCovering".to_string(),
            "goToLiftPercentage".to_string(),
            json!({"percentageliftvalue": 10}),
            json!({})
        )]
    );

    // Inverted
    publish.endpoint("J1_cover", "1").commands.clear();
    publish.devices.get_mut("J1_cover").unwrap().invert_cover = true;
    publish.on_mqtt("zigbee2mqtt/J1_cover/set", &json!({"position": 90}));
    assert_eq!(
        publish.endpoint("J1_cover", "1").commands,
        vec![(
            "closuresWindowCovering".to_string(),
            "goToLiftPercentage".to_string(),
            json!({"percentageliftvalue": 90}),
            json!({})
        )]
    );
}

#[tokio::test]
async fn should_allow_to_set_color_via_hue_and_saturation() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"color": {"hue": 250, "saturation": 50}}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "lightingColorCtrl".to_string(),
            "enhancedMoveToHueAndSaturation".to_string(),
            json!({"enhancehue": 45510, "saturation": 127, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/bulb_color",
        json!({"color": {"hue": 250, "saturation": 50}, "color_mode": "hs"})
    ));
}

#[tokio::test]
async fn should_also_use_on_off_cluster_when_controlling_group_with_switch() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/switch_group/set",
        &json!({"state": "ON", "brightness": 100}),
    );

    assert_eq!(
        publish.group("switch_group").commands,
        vec![
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 100, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
            (
                "genOnOff".to_string(),
                "on".to_string(),
                json!({}),
                json!({})
            ),
        ]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/switch_group",
        json!({"state": "ON", "brightness": 100})
    ));

    publish.group("switch_group").commands.clear();
    publish.published.clear();
    publish.on_mqtt("zigbee2mqtt/switch_group/set", &json!({"state": "OFF"}));

    assert_eq!(
        publish.group("switch_group").commands,
        vec![(
            "genOnOff".to_string(),
            "off".to_string(),
            json!({}),
            json!({})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/switch_group",
        json!({"state": "OFF", "brightness": 100})
    ));
}

#[tokio::test]
async fn should_corretly_handle_mallformed_messages() {
    let mut publish = setup();

    publish.on_mqtt_raw("zigbee2mqtt/foo", "");
    publish.on_mqtt_raw("zigbee2mqtt/bulb_color/set", "");

    assert!(nothing_published(&publish));
}

#[tokio::test]
async fn should_create_and_publish_to_group_which_is_in_configuration_yaml_but_not_in_zigbee_herdsman(
) {
    let mut publish = setup();
    publish.add_group("group_12312");
    // Upstream `groups` mock already contains the configuration group, so the
    // count is captured here to assert publishing does not create a new one.
    let group_count = publish.groups.len();

    publish.on_mqtt("zigbee2mqtt/group_12312/set", &json!({"state": "ON"}));

    assert_eq!(publish.groups.len(), group_count);
    // group contains no device
    assert!(publish.group("group_12312").commands.is_empty());
}

#[tokio::test]
async fn should_handle_get() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/get",
        &json!({"state": "", "brightness": ""}),
    );

    assert_eq!(publish.endpoint("bulb_color", "1").reads.len(), 2);
    assert!(publish
        .endpoint("bulb_color", "1")
        .reads
        .contains(&("genOnOff".to_string(), vec!["onOff".to_string()])));
    assert!(publish
        .endpoint("bulb_color", "1")
        .reads
        .contains(&("genLevelCtrl".to_string(), vec!["currentLevel".to_string()])));
}

#[tokio::test]
async fn should_handle_get_with_multiple_cover_endpoints() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/zigfred_plus/get",
        &json!({"state_l6": "", "state_l7": ""}),
    );

    assert_eq!(
        publish.endpoint("zigfred_plus", "11").reads,
        vec![(
            "closuresWindowCovering".to_string(),
            vec!["currentPositionLiftPercentage".to_string()]
        )]
    );
    assert_eq!(
        publish.endpoint("zigfred_plus", "12").reads,
        vec![(
            "closuresWindowCovering".to_string(),
            vec!["currentPositionLiftPercentage".to_string()]
        )]
    );
}

#[tokio::test]
async fn should_handle_get_with_multiple_endpoints() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/0x0017880104e45542/get",
        &json!({"state_left": "", "state_right": ""}),
    );

    assert_eq!(
        publish.endpoint("wall_switch_double", "2").reads,
        vec![("genOnOff".to_string(), vec!["onOff".to_string()])]
    );
    assert_eq!(
        publish.endpoint("wall_switch_double", "3").reads,
        vec![("genOnOff".to_string(), vec!["onOff".to_string()])]
    );
}

#[tokio::test]
async fn should_handle_non_valid_topics() {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt1/bulb_color/set", &json!({"state": "ON"}));

    assert!(nothing_published(&publish));
}

#[tokio::test]
async fn should_log_as_error_when_setting_property_with_no_defined_converter() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/get",
        &json!({"brightness_move": 20}),
    );

    assert!(publish.endpoint("bulb_color", "1").commands.is_empty());
    assert!(publish.logs.iter().any(|(level, message, _)| {
        level == "error" && message == "No converter available for 'get' 'brightness_move' (20)"
    }));
}

#[tokio::test]
async fn should_log_error_when_device_has_no_definition() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/button_double_key_interviewing/set",
        &json!({"state": "OFF"}),
    );

    assert!(publish.logs.contains(&(
        "error".to_string(),
        "Cannot publish to unsupported device 'button_double_key_interviewing'".to_string(),
        "z2m".to_string()
    )));
}

#[tokio::test]
async fn should_log_error_when_device_has_no_such_endpoint_via_property() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/0x0017880104e45542/get",
        &json!({"state_center": "", "state_right": ""}),
    );

    assert!(publish.logs.iter().any(|(level, message, _)| {
        level == "error"
            && message
                == "No converter available for 'state_center' on 'wall_switch_double': (\"\")"
    }));
    assert!(publish.endpoint("wall_switch_double", "2").reads.is_empty());
    assert_eq!(
        publish.endpoint("wall_switch_double", "3").reads,
        vec![("genOnOff".to_string(), vec!["onOff".to_string()])]
    );
}

#[tokio::test]
async fn should_log_error_when_device_has_no_such_endpoint_via_topic() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/0x0017880104e45542/center/get",
        &json!({"state": ""}),
    );

    assert!(publish.logs.iter().any(|(level, message, _)| {
        level == "error" && message == "Device 'wall_switch_double' has no endpoint 'center'"
    }));
    assert!(publish.endpoint("wall_switch_double", "2").reads.is_empty());
}

#[tokio::test]
async fn should_not_publish_messages_to_zigbee_devices_when_payload_is_invalid() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/wall_switch_double/left/set",
        &json!({"state": true}),
    );
    publish.on_mqtt(
        "zigbee2mqtt/wall_switch_double/left/set",
        &json!({"state": 1}),
    );

    assert!(nothing_published(&publish));
}

#[tokio::test]
async fn should_not_use_state_converter_on_non_json_message_when_value_is_not_on_off_toggle() {
    let mut publish = setup();

    publish.on_mqtt_raw("zigbee2mqtt/wall_switch_double/left/set", "ON_RANDOM");

    assert!(publish
        .endpoint("wall_switch_double", "2")
        .commands
        .is_empty());
}

#[tokio::test]
async fn should_parse_set_with_and_slashes_in_base_and_device_id_postfix_topic() {
    let mut publish = setup();
    publish.base_topic = "zigbee2mqtt/at/my/home".to_string();
    publish
        .devices
        .get_mut("wall_switch_double")
        .unwrap()
        .friendly_name = "in/basement/wall_switch_double".to_string();

    publish.on_mqtt(
        "zigbee2mqtt/at/my/home/in/basement/wall_switch_double/left/set",
        &json!({"state": "ON"}),
    );

    assert_eq!(
        publish.endpoint("wall_switch_double", "2").commands,
        vec![(
            "genOnOff".to_string(),
            "on".to_string(),
            json!({}),
            json!({})
        )]
    );
}

#[tokio::test]
async fn should_parse_set_with_attribute_topic() {
    let mut publish = setup();

    publish.on_mqtt_raw("zigbee2mqtt/bulb_color/set/state", "ON");

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "genOnOff".to_string(),
            "on".to_string(),
            json!({}),
            json!({})
        )]
    );
}

#[tokio::test]
async fn should_parse_set_with_color_attribute_topic() {
    let mut publish = setup();

    publish.on_mqtt_raw("zigbee2mqtt/bulb_color/set/color", "#64C80A");

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "lightingColorCtrl".to_string(),
            "moveToColor".to_string(),
            json!({"colorx": 17806, "colory": 43155, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/bulb_color",
        json!({"color": {"x": 0.2717, "y": 0.6585}, "color_mode": "xy"})
    ));
}

#[tokio::test]
async fn should_parse_set_with_ieee_addr_topic() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/0x000b57fffec6a5b3/set",
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
}

#[tokio::test]
async fn should_parse_set_with_non_existing_postfix() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/wall_switch_double/invalid/set",
        &json!({"state": "ON"}),
    );

    assert!(nothing_published(&publish));
}

#[tokio::test]
async fn should_parse_set_with_number_at_the_end_of_friendly_name_and_postfix() {
    let mut publish = setup();
    publish
        .devices
        .get_mut("wall_switch_double")
        .unwrap()
        .friendly_name = "ground_floor/kitchen/wall_switch/2".to_string();

    publish.on_mqtt(
        "zigbee2mqtt/ground_floor/kitchen/wall_switch/2/left/set",
        &json!({"state": "ON"}),
    );

    assert_eq!(
        publish.endpoint("wall_switch_double", "2").commands,
        vec![(
            "genOnOff".to_string(),
            "on".to_string(),
            json!({}),
            json!({})
        )]
    );
}

#[tokio::test]
async fn should_parse_set_with_postfix_topic_and_attribute() {
    let mut publish = setup();

    publish.on_mqtt_raw("zigbee2mqtt/wall_switch_double/left/set", "ON");

    assert_eq!(
        publish.endpoint("wall_switch_double", "2").commands,
        vec![(
            "genOnOff".to_string(),
            "on".to_string(),
            json!({}),
            json!({})
        )]
    );
}

#[tokio::test]
async fn should_parse_topic_with_when_base_and_device_id_have_multiple_slashes() {
    let mut publish = setup();
    publish.base_topic = "zigbee2mqtt/at/my/basement".to_string();
    publish.devices.get_mut("bulb_color").unwrap().friendly_name =
        "floor0/basement/my_device_id2".to_string();

    publish.on_mqtt(
        "zigbee2mqtt/at/my/basement/floor0/basement/my_device_id2/get",
        &json!({"state": ""}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").reads,
        vec![("genOnOff".to_string(), vec!["onOff".to_string()])]
    );
}

#[tokio::test]
async fn should_parse_topic_with_when_base_topic_has_multiple_slashes() {
    let mut publish = setup();
    publish.base_topic = "zigbee2mqtt/at/my/home".to_string();

    publish.on_mqtt(
        "zigbee2mqtt/at/my/home/bulb_color/get",
        &json!({"state": ""}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").reads,
        vec![("genOnOff".to_string(), vec!["onOff".to_string()])]
    );
}

#[tokio::test]
async fn should_parse_topic_with_when_device_id_has_multiple_slashes() {
    let mut publish = setup();
    publish.devices.get_mut("bulb_color").unwrap().friendly_name =
        "floor0/basement/my_device_id2".to_string();

    publish.on_mqtt(
        "zigbee2mqtt/floor0/basement/my_device_id2/get",
        &json!({"state": ""}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").reads,
        vec![("genOnOff".to_string(), vec!["onOff".to_string()])]
    );
}

#[tokio::test]
async fn should_publish_brightness_move_down_to_zigbee_devices() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"brightness_move": 30}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "move".to_string(),
            json!({"movemode": 0, "rate": 30, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_brightness_move_up_to_zigbee_devices() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"brightness_move": -40}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "move".to_string(),
            json!({"movemode": 1, "rate": 40, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_correct_state_on_toggle_command_to_zigbee_bulb() {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"state": "TOGGLE"}));
    // No state yet, so nothing optimistic is published.
    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "genOnOff".to_string(),
            "toggle".to_string(),
            json!({}),
            json!({})
        )]
    );
    assert_eq!(publish.published.len(), 0);

    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"state": "OFF"}));
    assert_eq!(publish.published.len(), 1);
    assert_eq!(publish.published[0].1, json!({"state": "OFF"}));

    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"state": "TOGGLE"}));
    assert_eq!(publish.published.len(), 2);
    assert_eq!(publish.published[1].1, json!({"state": "ON"}));
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

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_tuya_cover_switch_with_dummy_endpoints() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/TS0601_cover_switch/set",
        &json!({"state": "OPEN"}),
    );
    publish.on_mqtt(
        "zigbee2mqtt/TS0601_cover_switch/set",
        &json!({"state_l1": "ON"}),
    );
    publish.on_mqtt(
        "zigbee2mqtt/TS0601_cover_switch/l2/set",
        &json!({"state": "OFF"}),
    );

    assert_eq!(
        publish.endpoint("TS0601_cover_switch", "1").commands.len(),
        3
    );
    assert!(publish
        .endpoint("TS0601_cover_switch", "1")
        .commands
        .contains(&(
            "manuSpecificTuya".to_string(),
            "dataRequest".to_string(),
            json!({"dpValues": [{"data": [0], "datatype": 4, "dp": 1}], "seq": 0}),
            json!({"disableDefaultResponse": true})
        )));
    assert!(publish
        .endpoint("TS0601_cover_switch", "1")
        .commands
        .contains(&(
            "manuSpecificTuya".to_string(),
            "dataRequest".to_string(),
            json!({"dpValues": [{"data": [1], "datatype": 1, "dp": 102}], "seq": 0}),
            json!({"disableDefaultResponse": true})
        )));
    assert!(publish
        .endpoint("TS0601_cover_switch", "1")
        .commands
        .contains(&(
            "manuSpecificTuya".to_string(),
            "dataRequest".to_string(),
            json!({"dpValues": [{"data": [0], "datatype": 1, "dp": 101}], "seq": 0}),
            json!({"disableDefaultResponse": true})
        )));
    assert!(has(
        &publish,
        "zigbee2mqtt/TS0601_cover_switch",
        json!({"state_l2": "OFF", "state_l1": "ON", "state": "OPEN"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_tuya_switch_with_dummy_endpoints() {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt/TS0601_switch/set", &json!({"state_l2": "ON"}));

    assert_eq!(
        publish.endpoint("TS0601_switch", "1").commands,
        vec![(
            "manuSpecificTuya".to_string(),
            "dataRequest".to_string(),
            json!({"dpValues": [{"data": [1], "datatype": 1, "dp": 2}], "seq": 0}),
            json!({"disableDefaultResponse": true})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/TS0601_switch",
        json!({"state_l2": "ON"})
    ));
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

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_to_non_default_ep_with_brightness_ep() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/0x0017882194e45543/set",
        &json!({"state_l2": "ON", "brightness_l2": 50}),
    );

    assert_eq!(
        publish.endpoint("QS-Zigbee-D02-TRIAC-2C-LN", "2").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 50, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/QS-Zigbee-D02-TRIAC-2C-LN",
        json!({"brightness_l2": 50, "state_l2": "ON"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_to_non_default_ep_with_state_ep() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/wall_switch_double/set",
        &json!({"state_right": "OFF"}),
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

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_with_brightness() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "ON", "brightness": "50"}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 50, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/bulb_color",
        json!({"state": "ON", "brightness": 50})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_with_color_rgb_object() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"color": {"r": 100, "g": 200, "b": 10}}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "lightingColorCtrl".to_string(),
            "moveToColor".to_string(),
            json!({"colorx": 17806, "colory": 43155, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/bulb_color",
        json!({"color": {"x": 0.2717, "y": 0.6585}, "color_mode": "xy"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_with_color_rgb_string() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"color": {"rgb": "100,200,10"}}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "lightingColorCtrl".to_string(),
            "moveToColor".to_string(),
            json!({"colorx": 17806, "colory": 43155, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/bulb_color",
        json!({"color": {"x": 0.2717, "y": 0.6585}, "color_mode": "xy"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_with_color_xy() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"color": {"x": 100, "y": 50}}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "lightingColorCtrl".to_string(),
            "moveToColor".to_string(),
            json!({"colorx": 6553500, "colory": 3276750, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/bulb_color",
        json!({"color_mode": "xy", "color": {"x": 100, "y": 50}})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_with_color_xy_and_brightness() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"color": {"x": 100, "y": 50}, "brightness": 20}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 20, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
            (
                "lightingColorCtrl".to_string(),
                "moveToColor".to_string(),
                json!({"colorx": 6553500, "colory": 3276750, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
        ]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/bulb_color",
        json!({"color_mode": "xy", "color": {"x": 100, "y": 50}, "state": "ON", "brightness": 20})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_with_color_xy_and_state() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"color": {"x": 100, "y": 50}, "state": "ON"}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![
            (
                "genOnOff".to_string(),
                "on".to_string(),
                json!({}),
                json!({})
            ),
            (
                "lightingColorCtrl".to_string(),
                "moveToColor".to_string(),
                json!({"colorx": 6553500, "colory": 3276750, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
        ]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/bulb_color",
        json!({"color_mode": "xy", "color": {"x": 100, "y": 50}, "state": "ON"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_with_color_xy_brightness_and_state_off() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"color": {"x": 100, "y": 50}, "brightness": 20, "state": "OFF"}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![
            (
                "lightingColorCtrl".to_string(),
                "moveToColor".to_string(),
                json!({"colorx": 6553500, "colory": 3276750, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
            (
                "genOnOff".to_string(),
                "off".to_string(),
                json!({}),
                json!({})
            ),
        ]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/bulb_color",
        json!({"color_mode": "xy", "color": {"x": 100, "y": 50}, "state": "OFF"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_to_zigbee_devices_with_color_xy_brightness_and_state_on() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"color": {"x": 100, "y": 50}, "brightness": 20, "state": "ON"}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 20, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
            (
                "lightingColorCtrl".to_string(),
                "moveToColor".to_string(),
                json!({"colorx": 6553500, "colory": 3276750, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
        ]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/bulb_color",
        json!({"color": {"x": 100, "y": 50}, "state": "ON", "brightness": 20, "color_mode": "xy"})
    ));
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

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_messages_with_options_disable_default_response() {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt/led_controller_1/set", &json!({"state": "OFF"}));

    assert_eq!(
        publish.endpoint("led_controller_1", "11").commands,
        vec![(
            "genOnOff".to_string(),
            "off".to_string(),
            json!({}),
            json!({"disableDefaultResponse": true})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/led_controller_1",
        json!({"state": "OFF"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_separate_gen_on_off_to_gl_s_007_zs_when_setting_state_and_brightness_as_bulb_doesnt_turn_on_with_move_to_level_with_on_off(
) {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/GL-S-007ZS/set",
        &json!({"state": "ON", "brightness": 20}),
    );

    assert_eq!(
        publish.endpoint("GL-S-007ZS", "1").commands,
        vec![
            (
                "genOnOff".to_string(),
                "on".to_string(),
                json!({}),
                json!({})
            ),
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 20, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
        ]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/GL-S-007ZS",
        json!({"state": "ON", "brightness": 20})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_state_to_roller_shutter() {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt/roller_shutter/set", &json!({"state": "OPEN"}));

    assert_eq!(
        publish.endpoint("roller_shutter", "1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 255, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/roller_shutter",
        json!({"position": 100})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_publish_to_mks_cm_w5() {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt/MKS-CM-W5/l3/set", &json!({"state": "ON"}));

    assert_eq!(
        publish.endpoint("MKS-CM-W5", "3").commands,
        vec![(
            "manuSpecificTuya".to_string(),
            "dataRequest".to_string(),
            json!({"dpValues": [{"data": [1], "datatype": 1, "dp": 3}], "seq": 0}),
            json!({"disableDefaultResponse": true})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/MKS-CM-W5",
        json!({"state_l3": "ON"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_restore_brightness_when_its_turned_off_without_transition_and_is_turned_on_with() {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"state": "ON"}));
    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "ON", "brightness": 123}),
    );
    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"state": "OFF"}));
    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "ON", "transition": 1.0}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![
            (
                "genOnOff".to_string(),
                "on".to_string(),
                json!({}),
                json!({})
            ),
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 123, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
            (
                "genOnOff".to_string(),
                "off".to_string(),
                json!({}),
                json!({})
            ),
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 123, "transtime": 10, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
        ]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_restore_brightness_when_its_turned_on_with_transition_z2_m_is_restarted_and_turned_on_again(
) {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "ON", "brightness": 20, "transition": 0.0}),
    );
    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "ON", "transition": 1.0}),
    );

    assert_eq!(publish.endpoint("bulb_color", "1").commands.len(), 2);
    assert!(publish.endpoint("bulb_color", "1").commands.contains(&(
        "genLevelCtrl".to_string(),
        "moveToLevelWithOnOff".to_string(),
        json!({"level": 20, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
        json!({})
    )));
    assert!(publish.endpoint("bulb_color", "1").commands.contains(&(
        "genLevelCtrl".to_string(),
        "moveToLevelWithOnOff".to_string(),
        json!({"level": 20, "transtime": 10, "optionsMask": 0, "optionsOverride": 0}),
        json!({})
    )));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_send_state_update_on_toggle_specific_endpoint() {
    let mut publish = setup();

    publish.on_mqtt_raw("zigbee2mqtt/wall_switch_double/left/set", "ON");
    publish.on_mqtt_raw("zigbee2mqtt/wall_switch_double/left/set", "TOGGLE");

    assert_eq!(
        publish.endpoint("wall_switch_double", "2").commands,
        vec![
            (
                "genOnOff".to_string(),
                "on".to_string(),
                json!({}),
                json!({})
            ),
            (
                "genOnOff".to_string(),
                "toggle".to_string(),
                json!({}),
                json!({})
            ),
        ]
    );
    assert_eq!(publish.published.len(), 2);
    assert_eq!(publish.published[0].1, json!({"state_left": "ON"}));
    assert_eq!(publish.published[1].1, json!({"state_left": "OFF"}));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_set_state_before_color() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "ON", "color": {"x": 0.701, "y": 0.299}}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![
            (
                "genOnOff".to_string(),
                "on".to_string(),
                json!({}),
                json!({})
            ),
            (
                "lightingColorCtrl".to_string(),
                "moveToColor".to_string(),
                json!({"colorx": 45940, "colory": 19595, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
        ]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_set_state_with_brightness_before_color() {
    let mut publish = setup();

    publish.on_mqtt(
            "zigbee2mqtt/bulb_color/set",
            &json!({"state": "ON", "color": {"x": 0.701, "y": 0.299}, "transition": 3, "brightness": 100}),
        );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 100, "transtime": 30, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
            (
                "lightingColorCtrl".to_string(),
                "moveToColor".to_string(),
                json!({"colorx": 45940, "colory": 19595, "transtime": 30, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
        ]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_sync_colors() {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"color_temp": 100}));
    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"color": {"x": 0.1, "y": 0.5}}),
    );
    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"color_temp": 300}));

    assert_eq!(publish.published.len(), 3);
    assert_eq!(
        publish.published[0].1,
        json!({"color_mode": "color_temp", "color_temp": 100})
    );
    assert_eq!(
        publish.published[1].1,
        json!({"color": {"x": 0.1, "y": 0.5}, "color_mode": "xy", "color_temp": 79})
    );
    assert_eq!(
        publish.published[2].1,
        json!({"color": {"x": 0.4152, "y": 0.3954}, "color_mode": "color_temp", "color_temp": 300})
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_turn_bulb_on_with_correct_brightness_when_device_is_turned_off_twice_and_brightness_is_reported(
) {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "ON", "brightness": 200}),
    );
    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "OFF", "transition": 0}),
    );
    publish.set_device_state("bulb_color", json!({"state": "OFF", "brightness": 1}));
    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "OFF", "transition": 0}),
    );
    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "ON", "transition": 0}),
    );

    assert_eq!(publish.published.len(), 5);
    assert_eq!(
        publish.published[0].1,
        json!({"state": "ON", "brightness": 200})
    );
    assert_eq!(
        publish.published[1].1,
        json!({"state": "OFF", "brightness": 200})
    );
    assert_eq!(
        publish.published[2].1,
        json!({"state": "OFF", "brightness": 1})
    );
    assert_eq!(
        publish.published[3].1,
        json!({"state": "OFF", "brightness": 1})
    );
    assert_eq!(
        publish.published[4].1,
        json!({"state": "ON", "brightness": 200})
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 200, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 0, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 0, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 200, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
        ]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_turn_bulb_on_with_full_brightness_when_transition_is_used_and_no_brightness_is_known(
) {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "OFF", "transition": 0.5}),
    );
    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "ON", "transition": 0.5}),
    );

    assert_eq!(publish.published[0].1, json!({"state": "OFF"}));
    assert_eq!(
        publish.published[1].1,
        json!({"state": "ON", "brightness": 254})
    );
    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 0, "transtime": 5, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 254, "transtime": 5, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
        ]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_turn_device_off_when_brightness_0_is_send() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"brightness": 50, "state": "ON"}),
    );
    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"brightness": 0}));
    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"state": "ON"}));

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 50, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 0, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 50, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
        ]
    );
    assert_eq!(publish.published.len(), 3);
    assert_eq!(
        publish.published[0].1,
        json!({"state": "ON", "brightness": 50})
    );
    assert_eq!(
        publish.published[1].1,
        json!({"state": "OFF", "brightness": 0})
    );
    assert_eq!(
        publish.published[2].1,
        json!({"state": "ON", "brightness": 50})
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_turn_device_off_when_brightness_0_is_send_with_transition() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"brightness": 50, "state": "ON"}),
    );
    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"brightness": 0, "transition": 3}),
    );
    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"state": "ON"}));

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 50, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 0, "transtime": 30, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 50, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
        ]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_turn_device_off_with_on_off_on_off_with_transition() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "OFF", "transition": 1}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 0, "transtime": 10, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert!(has(
        &publish,
        "zigbee2mqtt/bulb_color",
        json!({"state": "OFF"})
    ));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_turn_device_on_with_on_off_when_transition_is_provided() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "ON", "transition": 3}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 254, "transtime": 30, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_turn_device_on_with_on_off_with_transition_when_transition_0_is_provided() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "ON", "transition": 0}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 254, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_use_transition_from_device_options_on_brightness_command() {
    let mut publish = setup();
    publish.devices.get_mut("bulb_color").unwrap().transition = Some(20);

    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"brightness": 20}));

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 20, "transtime": 200, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_use_transition_on_brightness_command() {
    let mut publish = setup();
    publish.devices.get_mut("bulb_color").unwrap().transition = Some(20);

    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"brightness": 20}));

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 20, "transtime": 200, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_use_transition_only_once_when_setting_brightness_and_color_temperature_for_tradfri()
{
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb/set",
        &json!({"state": "ON", "brightness": 20, "color_temp": 200, "transition": 20}),
    );

    assert_eq!(
        publish.endpoint("bulb", "1").commands,
        vec![
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 20, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
            (
                "lightingColorCtrl".to_string(),
                "moveToColorTemp".to_string(),
                json!({"colortemp": 200, "transtime": 200, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
        ]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_use_transition_only_once_when_setting_brightness_and_color_temperature_for_group_which_contains_tradfri(
) {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/group_with_tradfri/set",
        &json!({"state": "ON", "transition": 60, "brightness": 20, "color_temp": 400}),
    );

    assert_eq!(
        publish.group("group_with_tradfri").commands,
        vec![
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 20, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
            (
                "lightingColorCtrl".to_string(),
                "moveToColorTemp".to_string(),
                json!({"colortemp": 400, "transtime": 600, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
        ]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_use_transition_when_brightness_with_group() {
    let mut publish = setup();
    publish.add_group_member("group_1", "bulb_color", "1");
    publish.group("group_1").transition = Some(20);

    publish.on_mqtt("zigbee2mqtt/group_1/set", &json!({"brightness": 100}));

    assert_eq!(
        publish.group("group_1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 100, "transtime": 200, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert_eq!(publish.published.len(), 2);
    assert_eq!(publish.published[1].0, "zigbee2mqtt/group_1");
    assert_eq!(
        publish.published[1].1,
        json!({"state": "ON", "brightness": 100})
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_use_transition_when_color_temp() {
    let mut publish = setup();
    publish.devices.get_mut("bulb_color").unwrap().transition = Some(20);

    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"color_temp": 200}));

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "lightingColorCtrl".to_string(),
            "moveToColorTemp".to_string(),
            json!({"colortemp": 200, "transtime": 200, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn shouldnt_do_anything_when_device_is_not_supported() {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt/unsupported2/set", &json!({"state": "ON"}));

    assert!(nothing_published(&publish));
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

#[tokio::test]
#[ignore = "not implemented yet"]
async fn shouldnt_use_move_to_level_with_on_off_on_turn_on_when_no_transition_has_been_used_as_some_devices_do_not_turn_on_in_that_case(
) {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"state": "ON"}));
    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"brightness": 150}));
    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"state": "OFF"}));
    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"state": "ON"}));

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![
            (
                "genOnOff".to_string(),
                "on".to_string(),
                json!({}),
                json!({})
            ),
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 150, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
            (
                "genOnOff".to_string(),
                "off".to_string(),
                json!({}),
                json!({})
            ),
            (
                "genOnOff".to_string(),
                "on".to_string(),
                json!({}),
                json!({})
            ),
        ]
    );
    assert_eq!(publish.published.len(), 4);
    assert_eq!(publish.published[0].1, json!({"state": "ON"}));
    assert_eq!(
        publish.published[1].1,
        json!({"state": "ON", "brightness": 150})
    );
    assert_eq!(
        publish.published[2].1,
        json!({"state": "OFF", "brightness": 150})
    );
    assert_eq!(
        publish.published[3].1,
        json!({"state": "ON", "brightness": 150})
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn transition_parameter_should_not_influence_brightness_on_state_on() {
    let mut publish = setup();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "ON", "brightness": 50}),
    );
    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"state": "ON"}));
    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "ON", "transition": 1}),
    );

    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 50, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
            (
                "genOnOff".to_string(),
                "on".to_string(),
                json!({}),
                json!({})
            ),
            (
                "genLevelCtrl".to_string(),
                "moveToLevelWithOnOff".to_string(),
                json!({"level": 50, "transtime": 10, "optionsMask": 0, "optionsOverride": 0}),
                json!({})
            ),
        ]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn when_device_is_turned_off_and_on_with_transition_with_report_enabled_it_should_restore_correct_brightness(
) {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"brightness": 200}));
    publish.endpoint("bulb_color", "1").commands.clear();
    publish.published.clear();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "OFF", "transition": 3}),
    );
    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 0, "transtime": 30, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert_eq!(
        publish.published[0].1,
        json!({"state": "OFF", "brightness": 200})
    );

    publish.set_device_state("bulb_color", json!({"state": "OFF", "brightness": 1}));
    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "ON", "transition": 3}),
    );

    assert_eq!(publish.endpoint("bulb_color", "1").commands.len(), 2);
    assert_eq!(
        publish.endpoint("bulb_color", "1").commands[1],
        (
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 200, "transtime": 30, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )
    );
    assert_eq!(
        publish.published[2].1,
        json!({"state": "ON", "brightness": 200})
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn when_device_is_turned_off_with_transition_and_turned_on_without_transition_it_should_restore_the_brightness(
) {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"brightness": 200}));
    publish.endpoint("bulb_color", "1").commands.clear();
    publish.published.clear();

    publish.on_mqtt(
        "zigbee2mqtt/bulb_color/set",
        &json!({"state": "OFF", "transition": 3}),
    );
    assert_eq!(
        publish.endpoint("bulb_color", "1").commands,
        vec![(
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 0, "transtime": 30, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )]
    );
    assert_eq!(
        publish.published[0].1,
        json!({"state": "OFF", "brightness": 200})
    );

    publish.on_mqtt("zigbee2mqtt/bulb_color/set", &json!({"state": "ON"}));
    assert_eq!(
        publish.endpoint("bulb_color", "1").commands[1],
        (
            "genLevelCtrl".to_string(),
            "moveToLevelWithOnOff".to_string(),
            json!({"level": 200, "transtime": 0, "optionsMask": 0, "optionsOverride": 0}),
            json!({})
        )
    );
    assert_eq!(
        publish.published[1].1,
        json!({"state": "ON", "brightness": 200})
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn zncldj11_lm_open() {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt/curtain/set", &json!({"state": "OPEN"}));

    assert_eq!(
        publish.endpoint("curtain", "1").writes,
        vec![(
            "genAnalogOutput".to_string(),
            json!({"presentValue": 100}),
            json!({})
        )]
    );
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn zncldj11_lm_position() {
    let mut publish = setup();

    publish.on_mqtt("zigbee2mqtt/curtain/set", &json!({"position": 10}));

    assert_eq!(
        publish.endpoint("curtain", "1").writes,
        vec![(
            "genAnalogOutput".to_string(),
            json!({"presentValue": 10}),
            json!({})
        )]
    );
}

// Upstream has several additional tests with the same title `Should handle non-valid topics`.
#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_handle_non_valid_topics_2() {
    let mut publish = setup();
    publish.on_mqtt("zigbee2mqtt1/bulb_color/sett", &json!({"state": "ON"}));
    assert!(nothing_published(&publish));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_handle_non_valid_topics_3() {
    let mut publish = setup();
    publish.on_mqtt("zigbee2mqtt/bulb_color/write", &json!({"state": "ON"}));
    assert!(nothing_published(&publish));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_handle_non_valid_topics_4() {
    let mut publish = setup();
    publish.on_mqtt("zigbee2mqtt/set", &json!({"state": "ON"}));
    assert!(nothing_published(&publish));
}

#[tokio::test]
#[ignore = "not implemented yet"]
async fn should_handle_non_valid_topics_5() {
    let mut publish = setup();
    publish.on_mqtt("set", &json!({"state": "ON"}));
    assert!(nothing_published(&publish));
}
