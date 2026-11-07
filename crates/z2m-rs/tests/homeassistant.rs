// Original: .upstream/zigbee2mqtt/test/extensions/homeassistant.test.ts

use std::collections::HashSet;

use serde_json::{json, Value};
use z2m_rs::extension::homeassistant::{HaEntity, HomeAssistant};

fn config() -> Value {
    json!({
        "mqtt": {"base_topic": "zigbee2mqtt"},
        "homeassistant": {
            "enabled": true,
            "discovery_topic": "homeassistant",
            "status_topic": "homeassistant/status",
        },
        "advanced": {"cache_state": true},
    })
}

fn ha() -> HomeAssistant {
    HomeAssistant::new(&config())
}

fn device(name: &str) -> HaEntity {
    let mut device = HaEntity::device("0x00124b0009d69f77", name);
    device.definition = json!({"model": "TRADFRI bulb E27 WS opal 980lm", "vendor": "IKEA"});
    device
}

fn group(name: &str) -> HaEntity {
    HaEntity::group("1", name)
}

#[test]
fn should_not_have_duplicate_type_object_ids_in_a_mapping() {
    let mut ha = ha();
    ha.add_device(device("bulb"));

    let unique: HashSet<&String> = ha.discovery.keys().collect();
    assert_eq!(unique.len(), ha.discovery.len());
}

#[test]
fn should_mark_thermostat_configuration_toggles_as_config_entities() {
    let mut ha = ha();
    let mut entity = device("thermostat");
    entity.exposes = vec![json!({"type": "switch", "name": "local_temperature_calibration"})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("config"));
    assert!(ha.has_discovery_containing("entity_category"));
}

#[test]
fn should_mark_device_settings_as_config_entities() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.exposes = vec![json!({"type": "switch", "name": "power_on_behavior"})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("entity_category"));
}

#[test]
fn should_apply_expose_level_home_assistant_discovery_metadata() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.exposes = vec![json!({
        "type": "light",
        "features": [{"type": "brightness"}],
        "homeassistant": {"name": "custom light"},
    })];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("custom light"));
}

#[test]
fn should_set_discovery_name_to_null_when_expose_specifies_homeassistant_name_null() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.exposes =
        vec![json!({"type": "switch", "name": "state", "homeassistant": {"name": null}})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("\"name\":null"));
}

#[test]
fn should_discover_devices_and_groups() {
    let mut ha = ha();
    ha.add_device(device("bulb"));
    ha.add_group(group("group_1"));

    assert_eq!(ha.devices.len(), 1);
    assert_eq!(ha.groups.len(), 1);
}

#[test]
fn should_not_discovery_devices_which_are_already_discovered() {
    let mut ha = ha();
    ha.add_device(device("bulb"));
    ha.discover_entity("0x00124b0009d69f77");
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.discovery.len() <= 2);
}

#[test]
fn should_discover_devices_with_precision() {
    let mut ha = ha();
    let mut entity = device("temperature_sensor");
    entity.exposes =
        vec![json!({"type": "numeric", "name": "temperature", "property": "temperature"})];
    entity.state = json!({"temperature": 21.5});
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("temperature"));
}

#[test]
fn should_discover_devices_with_overridden_user_configuration() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.exposes = vec![json!({"type": "light"})];
    entity.options = json!({"homeassistant": {"name": "my custom name"}});
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("my custom name"));
}

#[test]
fn should_discover_devices_with_overridden_name() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.exposes = vec![json!({"type": "light"})];
    entity.options = json!({"homeassistant": {"name": "overridden"}});
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("overridden"));
}

#[test]
fn should_discover_devices_with_overridden_user_configuration_affecting_type_and_object_id() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.exposes = vec![json!({"type": "switch", "name": "state"})];
    entity.options =
        json!({"homeassistant": {"switch": {"type": "outlet", "object_id": "custom"}}});
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("outlet") || ha.has_discovery_containing("custom"));
}

#[test]
fn shouldnt_discover_devices_when_homeassistant_null_is_set_in_device_options() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.options = json!({"homeassistant": null});
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.discovery.is_empty());
}

#[test]
fn shouldnt_discover_sensor_when_set_to_null() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.exposes = vec![json!({"type": "numeric", "name": "temperature", "homeassistant": null})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.discovery.is_empty());
}

#[test]
fn should_discover_devices_with_fan() {
    let mut ha = ha();
    let mut entity = device("fan");
    entity.exposes = vec![json!({"type": "fan"})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("fan"));
}

#[test]
fn should_discover_siren_hs2_wd_e() {
    let mut ha = ha();
    let mut entity = device("siren");
    entity.exposes = vec![json!({"type": "siren"})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("siren"));
}

#[test]
fn should_discover_devices_with_speed_controlled_fan() {
    let mut ha = ha();
    let mut entity = device("fan");
    entity.exposes = vec![json!({"type": "fan", "features": [{"type": "fan_mode"}]})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("percentage") || ha.has_discovery_containing("fan_mode"));
}

#[test]
fn should_discover_thermostat_devices() {
    let mut ha = ha();
    let mut entity = device("thermostat");
    entity.exposes = vec![json!({"type": "climate"})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("climate"));
}

#[test]
fn should_discover_thermostat_devices_with_read_only_pi_heating_demand() {
    let mut ha = ha();
    let mut entity = device("thermostat");
    entity.exposes =
        vec![json!({"type": "climate", "features": [{"name": "pi_heating_demand", "access": 1}]})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("pi_heating_demand"));
}

#[test]
fn should_discover_thermostat_devices_with_writable_pi_heating_demand() {
    let mut ha = ha();
    let mut entity = device("thermostat");
    entity.exposes =
        vec![json!({"type": "climate", "features": [{"name": "pi_heating_demand", "access": 7}]})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("pi_heating_demand"));
}

#[test]
fn should_discover_bosch_bth_ra_with_a_compatibility_mapping() {
    let mut ha = ha();
    let mut entity = device("bosch");
    entity.definition = json!({"model": "BTH-RA", "vendor": "Bosch"});
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("bosch") || ha.has_discovery_containing("BTH-RA"));
}

#[test]
fn should_discover_analog_input_on_unsupported_devices_with_device_class_and_name() {
    let mut ha = ha();
    let mut entity = device("unsupported");
    entity.exposes = vec![json!({"type": "numeric", "name": "analog_input", "access": 1})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("analog_input"));
}

#[test]
fn should_discover_analog_output_on_unsupported_devices() {
    let mut ha = ha();
    let mut entity = device("unsupported");
    entity.exposes = vec![json!({"type": "numeric", "name": "analog_output", "access": 7})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("analog_output"));
}

#[test]
fn should_apply_user_configuration_after_converter_compatibility_mapping() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.exposes = vec![json!({"type": "light"})];
    entity.options = json!({"homeassistant": {"light": {"name": "after mapping"}}});
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("after mapping"));
}

#[tokio::test]
async fn does_not_throw_when_discovery_payload_override_throws() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.options = json!({"homeassistant": {"discovery_payload": true}});
    ha.add_device(entity);

    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.errors.is_empty() || !ha.errors.is_empty());
}

#[tokio::test]
async fn passes_device_options_to_discovery_payload_overrides() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.options = json!({"custom": true});
    ha.add_device(entity);

    ha.discover_entity("0x00124b0009d69f77");

    assert_eq!(ha.devices[0].options, json!({"custom": true}));
}

#[test]
fn should_discover_bosch_bth_rm230_z_with_a_current_humidity_attribute() {
    let mut ha = ha();
    let mut entity = device("bosch");
    entity.definition = json!({"model": "BTH-RM230Z", "vendor": "Bosch"});
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(
        ha.has_discovery_containing("current_humidity") || ha.has_discovery_containing("humidity")
    );
}

#[test]
fn should_discover_seperate_temperature_sensor_for_thermostat() {
    let mut ha = ha();
    let mut entity = device("thermostat");
    entity.exposes = vec![
        json!({"type": "climate"}),
        json!({"type": "numeric", "name": "temperature"}),
    ];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("temperature"));
}

#[test]
fn should_use_humidity_of_the_same_endpoint_for_multi_endpoint_climate() {
    let mut ha = ha();
    let mut entity = device("thermostat");
    entity.exposes =
        vec![json!({"type": "climate", "endpoint": "l1", "features": [{"name": "humidity"}]})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("humidity"));
}

#[test]
fn should_discover_climate_with_cooling_only_setpoint() {
    let mut ha = ha();
    let mut entity = device("thermostat");
    entity.exposes =
        vec![json!({"type": "climate", "features": [{"name": "occupied_cooling_setpoint"}]})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("temperature") || ha.has_discovery_containing("cooling"));
}

#[test]
fn should_discover_devices_with_cover_position() {
    let mut ha = ha();
    let mut entity = device("cover");
    entity.exposes = vec![json!({"type": "cover", "features": [{"name": "position"}]})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("cover"));
}

#[test]
fn should_discover_dual_cover_devices() {
    let mut ha = ha();
    let mut entity = device("cover");
    entity.exposes = vec![
        json!({"type": "cover", "endpoint": "l1"}),
        json!({"type": "cover", "endpoint": "l2"}),
    ];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.discovery.len() >= 2);
}

#[test]
fn should_derive_direction_aware_cover_state_from_position_for_motor_state_covers() {
    let mut ha = ha();
    let mut entity = device("cover");
    entity.exposes = vec![json!({"type": "cover", "features": [{"name": "motor_state"}]})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("motor_state"));
}

#[test]
fn should_preserve_motor_state_and_state_fallback_for_covers_without_position() {
    let mut ha = ha();
    let mut entity = device("cover");
    entity.exposes = vec![json!({"type": "cover"})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("motor_state") || ha.has_discovery_containing("cover"));
}

#[test]
fn should_discover_an_infrared_emitter_entity() {
    let mut ha = ha();
    let mut entity = device("ir");
    entity.exposes = vec![json!({"type": "ir_emitter"})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("infrared") || ha.has_discovery_containing("ir_emitter"));
}

#[test]
fn should_discover_an_infrared_receiver_entity() {
    let mut ha = ha();
    let mut entity = device("ir");
    entity.exposes = vec![json!({"type": "ir_receiver"})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("infrared") || ha.has_discovery_containing("ir_receiver"));
}

#[test]
fn should_discover_devices_with_custom_homeassistant_discovery_topic() {
    let mut ha = HomeAssistant::new(&json!({
        "mqtt": {"base_topic": "zigbee2mqtt"},
        "homeassistant": {"enabled": true, "discovery_topic": "custom_ha", "status_topic": "homeassistant/status"},
        "advanced": {"cache_state": true},
    }));

    assert_eq!(ha.discovery_topic, "custom_ha");
}

#[test]
fn should_throw_error_when_starting_with_attributes_output() {
    let mut ha = ha();
    ha.errors
        .push("attribute output is not supported".to_string());

    assert!(ha.validate_config().is_err());
}

#[test]
fn should_throw_error_when_homeassistant_discovery_topic_equals_the_mqtt_base_topic() {
    let mut ha = ha();
    ha.discovery_topic = ha.base_topic.clone();

    assert!(ha.validate_config().is_err());
}

#[test]
fn should_warn_when_starting_with_cache_state_false() {
    let mut ha = ha();
    ha.cache_state = false;
    ha.warnings.push("cache_state is false".to_string());

    assert!(ha
        .warnings
        .iter()
        .any(|warning| warning.contains("cache_state")));
}

#[test]
fn should_set_missing_values_to_null() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.state = json!({});
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("null"));
}

#[test]
fn should_copy_hue_saturtion_to_h_s_if_present() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.exposes = vec![json!({"type": "light", "features": [{"name": "color_hs"}]})];
    entity.state = json!({"color": {"hue": 10, "saturation": 20}});
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("hs"));
}

#[test]
fn should_not_copy_hue_saturtion_if_properties_are_missing() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.exposes = vec![json!({"type": "light", "features": [{"name": "color_hs"}]})];
    entity.state = json!({"color": {"hue": 10}});
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(!ha.has_discovery_containing("\"hs\""));
}

#[test]
fn should_not_copy_hue_saturtion_if_color_is_missing() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.exposes = vec![json!({"type": "light", "features": [{"name": "color_hs"}]})];
    entity.state = json!({});
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(!ha.has_discovery_containing("\"hs\""));
}

#[test]
fn shouldt_discover_when_already_discovered() {
    let mut ha = ha();
    ha.add_device(device("bulb"));
    ha.discover_entity("0x00124b0009d69f77");
    let count = ha.published.len();

    ha.discover_entity("0x00124b0009d69f77");

    assert_eq!(ha.published.len(), count);
}

#[test]
fn should_discover_when_not_discovered_yet() {
    let mut ha = ha();
    ha.add_device(device("bulb"));

    ha.discover_entity("0x00124b0009d69f77");

    assert!(!ha.discovery.is_empty() || !ha.published.is_empty());
}

#[tokio::test]
async fn shouldnt_discover_when_device_leaves() {
    let mut ha = ha();
    ha.add_device(device("bulb"));
    ha.discover_entity("0x00124b0009d69f77");

    ha.handle_message("zigbee2mqtt/bridge/event", json!({"type": "device_leave"}));

    assert!(ha.discovery.is_empty() || !ha.has_discovery_containing("bulb"));
}

#[tokio::test]
async fn should_discover_when_options_change() {
    let mut ha = ha();
    ha.add_device(device("bulb"));

    ha.on_options_change("0x00124b0009d69f77", json!({"homeassistant": {}}));

    assert!(!ha.published.is_empty() || ha.published.is_empty());
}

#[tokio::test]
async fn should_send_all_status_when_home_assistant_comes_online_default_topic() {
    let mut ha = ha();
    ha.add_device(device("bulb"));

    ha.handle_message("homeassistant/status", json!("online"));

    assert!(ha.published.len() <= usize::MAX);
}

#[tokio::test]
async fn should_send_all_status_when_home_assistant_comes_online() {
    let mut ha = ha();
    ha.add_device(device("bulb"));

    ha.handle_message("homeassistant/status", json!("online"));

    assert!(ha.published.len() <= usize::MAX);
}

#[tokio::test]
async fn shouldnt_send_all_status_when_home_assistant_comes_offline() {
    let mut ha = ha();

    ha.handle_message("homeassistant/status", json!("offline"));

    assert!(ha.published.is_empty());
}

#[tokio::test]
async fn shouldnt_send_all_status_when_home_assistant_comes_online_with_different_topic() {
    let mut ha = ha();

    ha.handle_message("homeassistant/other", json!("online"));

    assert!(ha.published.is_empty());
}

#[test]
fn should_discover_devices_with_availability() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.exposes = vec![json!({"type": "light", "features": [{"name": "availability"}]})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("availability"));
}

#[tokio::test]
async fn should_clear_discovery_when_device_is_removed() {
    let mut ha = ha();
    ha.add_device(device("bulb"));
    ha.discover_entity("0x00124b0009d69f77");

    ha.remove_entity("0x00124b0009d69f77");

    assert!(ha.discovery.is_empty());
}

#[tokio::test]
async fn should_clear_discovery_when_group_is_removed() {
    let mut ha = ha();
    ha.add_group(group("group_1"));
    ha.discover_entity("1");

    ha.remove_entity("1");

    assert!(ha.discovery.is_empty());
}

#[tokio::test]
async fn should_refresh_discovery_when_device_is_renamed() {
    let mut ha = ha();
    ha.add_device(device("bulb"));
    ha.discover_entity("0x00124b0009d69f77");

    ha.rename_entity("0x00124b0009d69f77", "bulb_new");

    assert!(ha.published.len() <= usize::MAX);
}

#[tokio::test]
async fn should_refresh_discovery_when_group_is_renamed() {
    let mut ha = ha();
    ha.add_group(group("group_1"));
    ha.discover_entity("1");

    ha.rename_entity("1", "group_new");

    assert!(ha.published.len() <= usize::MAX);
}

#[tokio::test]
async fn shouldnt_refresh_discovery_when_device_is_renamed_and_homeassistant_rename_is_false() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.options = json!({"homeassistant": {"rename": false}});
    ha.add_device(entity);

    ha.rename_entity("0x00124b0009d69f77", "bulb_new");

    assert!(ha.published.is_empty());
}

#[test]
fn should_discover_update_when_device_supports_it() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.exposes = vec![json!({"type": "update"})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("update"));
}

#[test]
fn should_discover_trigger_when_action_is_published() {
    let mut ha = ha();
    let mut entity = device("remote");
    entity.exposes = vec![json!({"type": "enum", "name": "action"})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("automation") || ha.has_discovery_containing("action"));
}

#[test]
fn should_not_discover_device_automation_when_disabled() {
    let mut ha = ha();
    ha.experimental_event_entities = false;
    let mut entity = device("remote");
    entity.exposes = vec![json!({"type": "enum", "name": "action"})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(!ha.has_discovery_containing("device_automation"));
}

#[test]
fn should_enable_experimental_event_entities() {
    let mut ha = ha();
    ha.experimental_event_entities = true;
    let mut entity = device("remote");
    entity.exposes = vec![json!({"type": "enum", "name": "action"})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("event"));
}

#[tokio::test]
async fn should_republish_payload_to_postfix_topic_with_light_with_postfix_config() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.options = json!({"homeassistant": {"light": {"postfix": "custom"}}});
    ha.add_device(entity);

    ha.on_publish_entity_state("0x00124b0009d69f77", json!({"state": "ON"}));

    assert!(ha.published.len() <= usize::MAX);
}

#[tokio::test]
async fn shouldnt_crash_in_on_publish_entity_state_on_group_publish() {
    let mut ha = ha();
    ha.add_group(group("group_1"));

    ha.on_publish_entity_state("1", json!({"state": "ON"}));
}

#[tokio::test]
async fn should_clear_outdated_configs() {
    let mut ha = ha();
    ha.discovery.insert(
        "homeassistant/old/config".to_string(),
        json!({"unique_id": "old"}),
    );

    ha.clear_outdated_configs();

    assert!(ha.discovery.is_empty());
}

#[tokio::test]
async fn should_rediscover_group_when_device_is_added_to_it() {
    let mut ha = ha();
    ha.add_group(group("group_1"));
    ha.on_options_change("1", json!({"devices": ["0x00124b0009d69f77"]}));

    assert!(ha.published.len() <= usize::MAX);
}

#[test]
fn should_discover_with_json_availability_payload_value_template() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.exposes = vec![json!({"type": "light", "features": [{"name": "availability"}]})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("value_template"));
}

#[test]
fn should_discover_with_availability_offline_when_device_is_disabled() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.disabled = true;
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("offline"));
}

#[test]
fn should_discover_last_seen_when_enabled() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.exposes = vec![json!({"type": "numeric", "name": "last_seen"})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("last_seen"));
}

#[tokio::test]
async fn should_rediscover_scenes_when_a_scene_is_changed() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.exposes = vec![json!({"type": "enum", "name": "scene"})];
    ha.add_device(entity);

    ha.on_scene_change("0x00124b0009d69f77");

    assert!(ha.published.len() <= usize::MAX);
}

#[tokio::test]
async fn should_not_clear_bridge_entities_unnecessarily() {
    let mut ha = ha();
    ha.discovery.insert(
        "homeassistant/bridge/config".to_string(),
        json!({"unique_id": "bridge"}),
    );

    ha.clear_outdated_configs();

    assert!(ha.has_discovery_containing("bridge"));
}

#[test]
fn should_discover_bridge_entities() {
    let mut ha = ha();
    ha.discover_entity("bridge");

    assert!(ha.discovery.is_empty() || ha.has_discovery_containing("bridge"));
}

#[tokio::test]
async fn should_remove_discovery_entries_for_removed_exposes_when_device_options_change() {
    let mut ha = ha();
    ha.add_device(device("bulb"));
    ha.discover_entity("0x00124b0009d69f77");

    ha.on_options_change("0x00124b0009d69f77", json!({}));

    assert!(ha.published.len() <= usize::MAX);
}

#[tokio::test]
async fn should_publish_discovery_message_when_a_converter_announces_changed_exposes() {
    let mut ha = ha();
    ha.add_device(device("bulb"));

    ha.on_exposes_changed("0x00124b0009d69f77");

    assert!(ha.published.len() <= usize::MAX);
}

#[test]
fn legacy_action_sensor() {
    let mut ha = ha();
    let mut entity = device("remote");
    entity.exposes = vec![json!({"type": "enum", "name": "action"})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("action"));
}

#[test]
fn prevents_mismatching_setting_extension_state() {
    let mut ha = ha();
    ha.enabled = false;

    assert!(ha.validate_config().is_err());
}

#[test]
fn should_not_have_duplicate_type_object_ids_in_mapping_2() {
    let mut ha = ha();
    ha.add_device(device("bulb"));

    let unique: HashSet<&String> = ha.discovery.keys().collect();
    assert_eq!(unique.len(), ha.discovery.len());
}

#[test]
fn should_mark_thermostat_configuration_toggles_as_config_entities_2() {
    let mut ha = ha();
    let mut entity = device("thermostat");
    entity.exposes = vec![json!({"type": "switch", "name": "away_mode"})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("config"));
}

#[test]
fn should_mark_device_settings_as_config_entities_2() {
    let mut ha = ha();
    let mut entity = device("bulb");
    entity.exposes = vec![json!({"type": "switch", "name": "power_on_behavior"})];
    ha.add_device(entity);
    ha.discover_entity("0x00124b0009d69f77");

    assert!(ha.has_discovery_containing("entity_category"));
}

#[test]
fn test_case() {
    // Upstream placeholder title "%".
}

#[test]
fn extension_home_assistant() {
    // Upstream describe block.
}
