// Original: .upstream/zigbee2mqtt/test/settings.test.ts
// Upstream test titles (ported as #[test] #[ignore] stubs):
//   - Combine everything! groups and devices from separate file :)
//   - Configuration friendly name cannot be empty
//   - Configuration friendly name cannot contain control char
//   - Configuration friendly name cannot end with /
//   - Configuration shouldnt be valid when duplicate friendly_name are used
//   - Configuration shouldnt be valid when friendly_name contains a MQTT wildcard
//   - Configuration shouldnt be valid when friendly_name ends with /DIGIT
//   - Configuration shouldnt be valid when invalid QOS value is used
//   - Ensures configuration.example.yaml is never out of sync with settings
//   - Frontend config
//   - Settings
//   - Should add and remove devices to a separate file if devices.yaml doesnt exist
//   - Should add devices
//   - Should add devices even when devices exist empty
//   - Should add devices for first file when using 2 separates file
//   - Should add devices for first file when using 2 separates file and the second file is empty
//   - Should add devices to a separate file
//   - Should add devices to a separate file if devices.yaml doesnt exist
//   - Should add devices to blocklist
//   - Should add groups
//   - Should add groups with specific ID
//   - Should allow retention configuration with MQTT v5
//   - Should allow username without password
//   - Should apply Home Assistant environment variables
//   - Should apply environment variables as overrides
//   - Should keep homeassistant null properties on apply
//   - Should keep homeassistant null property on device setting change
//   - Should not add duplicate groups
//   - Should not add duplicate groups with specific ID
//   - Should not allow any string values for ext_pan_id
//   - Should not allow any string values for network_key
//   - Should not allow any string values for pan_id
//   - Should not allow retention configuration without MQTT v5
//   - Should not throw error when devices is null
//   - Should read ALL secrets form a separate file
//   - Should read MQTT username and password form a separate file
//   - Should read devices
//   - Should read devices form 2 separate files
//   - Should read devices form a separate file
//   - Should read groups
//   - Should read groups from a separate file
//   - Should return default settings
//   - Should return settings
//   - Should throw error when changing entity friendlyName of non-existing device
//   - Should throw error when changing entity options of non-existing device
//   - Should throw when adding device which already exists
//   - Should throw when removing device which doesnt exist
//   - Should validate if settings do not conform to scheme
//   - Should write environment variables as overrides to configuration.yaml, not in the ref file
//   - Shouldnt write to configuration.yaml when there are no changes in it
//   - Throw if removing non-existing group

use serde_json::json;
use z2m_rs::util::settings::Settings;

fn minimal_config() -> serde_json::Value {
    json!({
        "homeassistant": {"enabled": true},
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "localhost"},
    })
}

#[test]
#[ignore = "not implemented yet"]
fn combine_everything_groups_and_devices_from_separate_file() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn configuration_friendly_name_cannot_be_empty() {
    let mut settings = Settings::new(json!({
        "homeassistant": {"enabled": true},
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "localhost"},
        "devices": {"0x0017880104e45519": {"friendly_name": "", "retain": false}},
    }));
    settings.re_read();

    let error = "friendly_name must be at least 1 char long";
    assert!(settings.validate().contains(&error.to_string()));
}

#[test]
#[ignore = "not implemented yet"]
fn configuration_friendly_name_cannot_contain_control_char() {
    let mut settings = Settings::new(json!({
        "homeassistant": {"enabled": true},
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "localhost"},
        "devices": {"0x0017880104e45519": {"friendly_name": "blaa/blaa\u{009f}", "retain": false}},
    }));
    settings.re_read();

    let error = "friendly_name is not allowed to contain control char";
    assert!(settings.validate().contains(&error.to_string()));
}

#[test]
#[ignore = "not implemented yet"]
fn configuration_friendly_name_cannot_end_with() {
    let mut settings = Settings::new(json!({
        "homeassistant": {"enabled": true},
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "localhost"},
        "devices": {"0x0017880104e45519": {"friendly_name": "blaa/", "retain": false}},
    }));
    settings.re_read();

    let error = "friendly_name is not allowed to end or start with /";
    assert!(settings.validate().contains(&error.to_string()));
}

#[test]
#[ignore = "not implemented yet"]
fn configuration_shouldnt_be_valid_when_duplicate_friendly_name_are_used() {
    let mut settings = Settings::new(json!({
        "homeassistant": {"enabled": true},
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "localhost"},
        "devices": {"0x0017880104e45519": {"friendly_name": "myname", "retain": false}},
        "groups": {"1": {"friendly_name": "myname", "retain": false}},
    }));
    settings.re_read();

    let error = "Duplicate friendly_name 'myname' found";
    assert!(settings.validate().contains(&error.to_string()));
}

#[test]
#[ignore = "not implemented yet"]
fn configuration_shouldnt_be_valid_when_friendly_name_contains_a_mqtt_wildcard() {
    let mut settings = Settings::new(json!({
        "homeassistant": {"enabled": true},
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "localhost"},
        "devices": {"0x0017880104e45519": {"friendly_name": "myname#", "retain": false}},
    }));
    settings.re_read();

    let error = "MQTT wildcard (+ and #) not allowed in friendly_name ('myname#')";
    assert!(settings.validate().contains(&error.to_string()));
}

#[test]
#[ignore = "not implemented yet"]
fn configuration_shouldnt_be_valid_when_friendly_name_ends_with_digit() {
    let mut settings = Settings::new(json!({
        "homeassistant": {"enabled": true},
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "localhost"},
        "devices": {"0x0017880104e45519": {"friendly_name": "myname/123", "retain": false}},
    }));
    settings.re_read();

    let error = "Friendly name cannot end with a \"/DIGIT\" ('myname/123')";
    assert!(settings.validate().contains(&error.to_string()));
}

#[test]
#[ignore = "not implemented yet"]
fn configuration_shouldnt_be_valid_when_invalid_qos_value_is_used() {
    let mut settings = Settings::new(json!({
        "homeassistant": {"enabled": true},
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "localhost"},
        "devices": {"0x0017880104e45519": {"friendly_name": "myname", "retain": false, "qos": 3}},
    }));
    settings.re_read();

    let error = "devices/0x0017880104e45519/qos must be equal to one of the allowed values";
    assert!(settings.validate().contains(&error.to_string()));
}

#[test]
#[ignore = "not implemented yet"]
fn ensures_configuration_example_yaml_is_never_out_of_sync_with_settings() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn frontend_config() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn settings() {
    let mut settings = Settings::new(json!({
        "devices": {
            "0x0017880104e45519": {"friendly_name": "myname", "retain": false},
            "0x0017880104e45511": {"friendly_name": "myname1", "retain": false},
        },
    }));
    settings.re_read();

    let error = settings
        .change_friendly_name("myname1", "myname")
        .unwrap_err();
    assert_eq!(error, "friendly_name 'myname' is already in use");
}

#[test]
#[ignore = "not implemented yet"]
fn should_add_and_remove_devices_to_a_separate_file_if_devices_yaml_doesnt_exist() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_add_devices() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_add_devices_even_when_devices_exist_empty() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_add_devices_for_first_file_when_using_2_separates_file() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_add_devices_for_first_file_when_using_2_separates_file_and_the_second_file_is_empty() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_add_devices_to_a_separate_file() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_add_devices_to_a_separate_file_if_devices_yaml_doesnt_exist() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_add_devices_to_blocklist() {
    let mut settings = Settings::new(json!({}));

    settings.block_device("0x123");
    assert_eq!(settings.get()["blocklist"], json!(["0x123"]));

    settings.block_device("0x1234");
    assert_eq!(settings.get()["blocklist"], json!(["0x123", "0x1234"]));
}

#[test]
#[ignore = "not implemented yet"]
fn should_add_groups() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_add_groups_with_specific_id() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_allow_retention_configuration_with_mqtt_v5() {
    let mut settings = Settings::new(json!({
        "homeassistant": {"enabled": true},
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "localhost", "version": 5},
        "devices": {"0x0017880104e45519": {"friendly_name": "tain", "retention": 900}},
    }));
    settings.re_read();

    assert_eq!(settings.validate(), Vec::<String>::new());
}

#[test]
#[ignore = "not implemented yet"]
fn should_allow_username_without_password() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_apply_home_assistant_environment_variables() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_apply_environment_variables_as_overrides() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_keep_homeassistant_null_properties_on_apply() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_keep_homeassistant_null_property_on_device_setting_change() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_not_add_duplicate_groups() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_not_add_duplicate_groups_with_specific_id() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_not_allow_any_string_values_for_ext_pan_id() {
    let mut settings = Settings::new(json!({
        "homeassistant": {"enabled": true},
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "localhost"},
        "advanced": {"ext_pan_id": "NOT_GENERATE"},
    }));
    settings.re_read();

    let error = "advanced.ext_pan_id: should be array or 'GENERATE' (is 'NOT_GENERATE')";
    assert!(settings.validate().contains(&error.to_string()));
}

#[test]
#[ignore = "not implemented yet"]
fn should_not_allow_any_string_values_for_network_key() {
    let mut settings = Settings::new(json!({
        "homeassistant": {"enabled": true},
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "localhost"},
        "advanced": {"network_key": "NOT_GENERATE"},
    }));
    settings.re_read();

    let error = "advanced.network_key: should be array or 'GENERATE' (is 'NOT_GENERATE')";
    assert!(settings.validate().contains(&error.to_string()));
}

#[test]
#[ignore = "not implemented yet"]
fn should_not_allow_any_string_values_for_pan_id() {
    let mut settings = Settings::new(json!({
        "homeassistant": {"enabled": true},
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "localhost"},
        "advanced": {"pan_id": "NOT_GENERATE"},
    }));
    settings.re_read();

    let error = "advanced.pan_id: should be number or 'GENERATE' (is 'NOT_GENERATE')";
    assert!(settings.validate().contains(&error.to_string()));
}

#[test]
#[ignore = "not implemented yet"]
fn should_not_allow_retention_configuration_without_mqtt_v5() {
    let mut settings = Settings::new(json!({
        "homeassistant": {"enabled": true},
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "localhost"},
        "devices": {"0x0017880104e45519": {"friendly_name": "tain", "retention": 900}},
    }));
    settings.re_read();

    let error = "MQTT retention requires protocol version 5";
    assert!(settings.validate().contains(&error.to_string()));
}

#[test]
#[ignore = "not implemented yet"]
fn should_not_throw_error_when_devices_is_null() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_read_all_secrets_form_a_separate_file() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_read_mqtt_username_and_password_form_a_separate_file() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_read_devices() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_read_devices_form_2_separate_files() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_read_devices_form_a_separate_file() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_read_groups() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_read_groups_from_a_separate_file() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_default_settings() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_return_settings() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_throw_error_when_changing_entity_friendly_name_of_non_existing_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_throw_error_when_changing_entity_options_of_non_existing_device() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_throw_when_adding_device_which_already_exists() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_throw_when_removing_device_which_doesnt_exist() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn should_validate_if_settings_do_not_conform_to_scheme() {
    let mut settings = Settings::new(json!({
        "homeassistant": {"enabled": true},
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "localhost"},
        "advanced": null,
    }));
    settings.re_read();

    let error = "advanced must be object";
    assert!(settings.validate().contains(&error.to_string()));
}

#[test]
#[ignore = "not implemented yet"]
fn should_write_environment_variables_as_overrides_to_configuration_yaml_not_in_the_ref_file() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn shouldnt_write_to_configuration_yaml_when_there_are_no_changes_in_it() {
    todo!()
}

#[test]
#[ignore = "not implemented yet"]
fn throw_if_removing_non_existing_group() {
    todo!()
}

// Upstream: it.each([...])("Device icons should be in `device_icons` directory or a url", ...)
#[test]
#[ignore = "not implemented yet"]
fn device_icons_should_be_in_device_icons_directory_or_a_url() {
    let cases: [(&str, Option<&str>); 4] = [
        (
            "bla.png",
            Some("Device icon of 'myname' should start with 'device_icons/', got 'bla.png'"),
        ),
        ("device_icons/bla.png", None),
        ("https://www.example.org/my-device.png", None),
        ("http://www.example.org/my-device.png", None),
    ];

    for (icon, error) in cases {
        let mut settings = Settings::new(json!({
            "homeassistant": {"enabled": true},
            "mqtt": {"base_topic": "zigbee2mqtt", "server": "localhost"},
            "devices": {"0x0017880104e45519": {"friendly_name": "myname", "icon": icon}},
        }));
        settings.re_read();

        if let Some(error) = error {
            assert!(settings.validate().contains(&error.to_string()));
        } else {
            assert_eq!(settings.validate(), Vec::<String>::new());
        }
    }
}
