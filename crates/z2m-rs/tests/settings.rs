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
use std::collections::BTreeMap;

use z2m_rs::util::settings::{
    Settings, CONFIGURATION_FILE, DEVICES_FILE, DEVICES_FILE2, GROUPS_FILE, SECRET_FILE,
};

fn minimal_config() -> serde_json::Value {
    json!({
        "homeassistant": {"enabled": true},
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "localhost"},
    })
}

#[test]
fn combine_everything_groups_and_devices_from_separate_file() {
    let mut settings = Settings::with_files(BTreeMap::from([
        (
            CONFIGURATION_FILE.to_string(),
            json!({"devices": "devices.yaml", "groups": "groups.yaml"}),
        ),
        (
            GROUPS_FILE.to_string(),
            json!({"1": {"friendly_name": "123"}}),
        ),
    ]));
    let expected_configuration = json!({"devices": "devices.yaml", "groups": "groups.yaml"});

    assert_eq!(
        settings.read_file(CONFIGURATION_FILE),
        expected_configuration
    );

    settings.add_device("0x1234");

    assert_eq!(
        settings.read_file(CONFIGURATION_FILE),
        expected_configuration
    );
    assert_eq!(
        settings.read_file(DEVICES_FILE),
        json!({"0x1234": {"friendly_name": "0x1234"}})
    );
    assert_eq!(
        settings.get_group("1"),
        Some(json!({"ID": 1, "friendly_name": "123"}))
    );
    assert_eq!(
        settings.read_file(CONFIGURATION_FILE),
        expected_configuration
    );
    assert_eq!(
        settings.get_device("0x1234"),
        Some(json!({"ID": "0x1234", "friendly_name": "0x1234"}))
    );
}

#[test]
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
fn ensures_configuration_example_yaml_is_never_out_of_sync_with_settings() {
    let example = json!({
        "version": 5,
        "homeassistant": {"enabled": false},
        "frontend": {"enabled": true},
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "mqtt://localhost"},
        "advanced": {
            "network_key": "GENERATE",
            "pan_id": "GENERATE",
            "ext_pan_id": "GENERATE",
        },
    });
    let settings = Settings::with_files(BTreeMap::from([(
        "configuration.example.yaml".to_string(),
        example.clone(),
    )]));

    assert_eq!(settings.read_file("configuration.example.yaml"), example);
}

#[test]
fn frontend_config() {
    let mut settings = Settings::with_config(json!({"frontend": {"enabled": true}}));
    settings.re_read();

    assert_eq!(
        settings.get()["frontend"],
        json!({
            "enabled": true,
            "package": "zigbee2mqtt-windfront",
            "port": 8080,
            "base_url": "/",
        })
    );
}

#[test]
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
fn should_add_and_remove_devices_to_a_separate_file_if_devices_yaml_doesnt_exist() {
    let mut settings = Settings::with_files(BTreeMap::from([(
        CONFIGURATION_FILE.to_string(),
        json!({"devices": "devices.yaml"}),
    )]));

    settings.add_device("0x1234");
    assert_eq!(
        settings.read_file(CONFIGURATION_FILE),
        json!({"devices": "devices.yaml"})
    );

    settings.remove_device("0x1234").unwrap();
    assert_eq!(
        settings.read_file(CONFIGURATION_FILE),
        json!({"devices": "devices.yaml"})
    );
    assert_eq!(settings.read_file(DEVICES_FILE), json!({}));
}

#[test]
fn should_add_devices() {
    let mut settings = Settings::from_config(json!({}));
    settings.add_device("0x12345678");

    assert_eq!(
        settings.configuration,
        json!({
            "devices": {
                "0x12345678": {"friendly_name": "0x12345678"},
            },
        })
    );
}

#[test]
fn should_add_devices_even_when_devices_exist_empty() {
    let mut settings = Settings::from_config(json!({"devices": []}));
    settings.add_device("0x12345678");

    assert_eq!(
        settings.configuration,
        json!({
            "devices": {
                "0x12345678": {"friendly_name": "0x12345678"},
            },
        })
    );
}

#[test]
fn should_add_devices_for_first_file_when_using_2_separates_file() {
    let mut settings = Settings::with_files(BTreeMap::from([
        (
            CONFIGURATION_FILE.to_string(),
            json!({"devices": ["devices.yaml", "devices2.yaml"]}),
        ),
        (
            DEVICES_FILE.to_string(),
            json!({"0x12345678": {"friendly_name": "0x12345678", "retain": false}}),
        ),
        (
            DEVICES_FILE2.to_string(),
            json!({"0x87654321": {"friendly_name": "0x87654321", "retain": false}}),
        ),
    ]));

    settings.add_device("0x1234");

    assert_eq!(
        settings.read_file(CONFIGURATION_FILE),
        json!({"devices": ["devices.yaml", "devices2.yaml"]})
    );
    assert_eq!(
        settings.read_file(DEVICES_FILE),
        json!({
            "0x12345678": {"friendly_name": "0x12345678", "retain": false},
            "0x1234": {"friendly_name": "0x1234"},
        })
    );
    assert_eq!(
        settings.read_file(DEVICES_FILE2),
        json!({"0x87654321": {"friendly_name": "0x87654321", "retain": false}})
    );
}

#[test]
fn should_add_devices_for_first_file_when_using_2_separates_file_and_the_second_file_is_empty() {
    let mut settings = Settings::with_files(BTreeMap::from([
        (
            CONFIGURATION_FILE.to_string(),
            json!({"devices": ["devices.yaml", "devices2.yaml"]}),
        ),
        (
            DEVICES_FILE.to_string(),
            json!({"0x12345678": {"friendly_name": "0x12345678", "retain": false}}),
        ),
        (DEVICES_FILE2.to_string(), json!({})),
    ]));

    settings.add_device("0x1234");

    assert_eq!(
        settings.read_file(CONFIGURATION_FILE),
        json!({"devices": ["devices.yaml", "devices2.yaml"]})
    );
    assert_eq!(
        settings.read_file(DEVICES_FILE),
        json!({
            "0x12345678": {"friendly_name": "0x12345678", "retain": false},
            "0x1234": {"friendly_name": "0x1234"},
        })
    );
    assert_eq!(settings.read_file(DEVICES_FILE2), json!({}));
}

#[test]
fn should_add_devices_to_a_separate_file() {
    let mut settings = Settings::with_files(BTreeMap::from([
        (
            CONFIGURATION_FILE.to_string(),
            json!({"devices": "devices.yaml"}),
        ),
        (
            DEVICES_FILE.to_string(),
            json!({"0x12345678": {"friendly_name": "0x12345678", "retain": false}}),
        ),
    ]));

    settings.add_device("0x1234");

    assert_eq!(
        settings.read_file(CONFIGURATION_FILE),
        json!({"devices": "devices.yaml"})
    );
    assert_eq!(
        settings.read_file(DEVICES_FILE),
        json!({
            "0x12345678": {"friendly_name": "0x12345678", "retain": false},
            "0x1234": {"friendly_name": "0x1234"},
        })
    );
}

#[test]
fn should_add_devices_to_a_separate_file_if_devices_yaml_doesnt_exist() {
    let mut settings = Settings::with_files(BTreeMap::from([(
        CONFIGURATION_FILE.to_string(),
        json!({"devices": "devices.yaml"}),
    )]));

    settings.add_device("0x1234");

    assert_eq!(
        settings.read_file(CONFIGURATION_FILE),
        json!({"devices": "devices.yaml"})
    );
    assert_eq!(
        settings.read_file(DEVICES_FILE),
        json!({"0x1234": {"friendly_name": "0x1234"}})
    );
}

#[test]
fn should_add_devices_to_blocklist() {
    let mut settings = Settings::new(json!({}));

    settings.block_device("0x123");
    assert_eq!(settings.get()["blocklist"], json!(["0x123"]));

    settings.block_device("0x1234");
    assert_eq!(settings.get()["blocklist"], json!(["0x123", "0x1234"]));
}

#[test]
fn should_add_groups() {
    let mut settings = Settings::from_config(json!({}));

    settings.add_group("test123", None);
    assert_eq!(
        settings.get()["groups"],
        json!({"1": {"friendly_name": "test123"}})
    );

    settings.add_group("test456", Some(""));
    assert_eq!(
        settings.get()["groups"],
        json!({
            "1": {"friendly_name": "test123"},
            "2": {"friendly_name": "test456"},
        })
    );
}

#[test]
fn should_add_groups_with_specific_id() {
    let mut settings = Settings::from_config(json!({}));

    settings.add_group("test123", Some("123"));
    assert_eq!(
        settings.get()["groups"],
        json!({"123": {"friendly_name": "test123"}})
    );
}

#[test]
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
fn should_allow_username_without_password() {
    let settings = Settings::with_config(json!({
        "mqtt": {"server": "my.mqtt.server", "user": "myusername"},
    }));

    assert_eq!(
        settings.get()["mqtt"],
        json!({
            "base_topic": "zigbee2mqtt",
            "include_device_information": false,
            "maximum_packet_size": 1048576,
            "keepalive": 60,
            "reject_unauthorized": true,
            "version": 4,
            "force_disable_retain": false,
            "server": "my.mqtt.server",
            "user": "myusername",
        })
    );
}

#[test]
fn should_apply_home_assistant_environment_variables() {
    let mut settings = Settings::with_config(json!({}));
    settings.env = BTreeMap::from([
            ("ZIGBEE2MQTT_CONFIG_FRONTEND_ENABLED".to_string(), "true".to_string()),
            ("ZIGBEE2MQTT_CONFIG_FRONTEND_PORT".to_string(), "8099".to_string()),
            ("ZIGBEE2MQTT_CONFIG_HOMEASSISTANT_ENABLED".to_string(), "true".to_string()),
            (
                "ZIGBEE2MQTT_CONFIG_SERIAL_PORT".to_string(),
                "/dev/serial/by-id/usb-ITead_Sonoff_Zigbee_3.0_USB_Dongle_Plus_48be7a7468d8ed11bfea786ff2c613ac-if00-port0".to_string(),
            ),
            ("ZIGBEE2MQTT_CONFIG_MQTT_SERVER".to_string(), "mqtt://core-mosquitto:1883".to_string()),
            ("ZIGBEE2MQTT_CONFIG_MQTT_USER".to_string(), "addons".to_string()),
            (
                "ZIGBEE2MQTT_CONFIG_MQTT_PASSWORD".to_string(),
                "7RbR4NBSskS8TKG4ugzdRilXw6TPQ4NKFs259j2DxeultOFtl5JwciNZFd6feT5o".to_string(),
            ),
        ]);

    for _ in 0..3 {
        assert!(settings.write());
        assert_eq!(settings.validate(), Vec::<String>::new());

        assert_eq!(settings.get()["frontend"]["enabled"], json!(true));
        assert_eq!(settings.get()["frontend"]["port"], json!(8099));
        assert_eq!(settings.get()["homeassistant"]["enabled"], json!(true));
        assert_eq!(
                settings.get()["serial"]["port"],
                json!("/dev/serial/by-id/usb-ITead_Sonoff_Zigbee_3.0_USB_Dongle_Plus_48be7a7468d8ed11bfea786ff2c613ac-if00-port0")
            );
        assert_eq!(
            settings.get()["mqtt"]["server"],
            json!("mqtt://core-mosquitto:1883")
        );
        assert_eq!(settings.get()["mqtt"]["user"], json!("addons"));
        assert_eq!(
            settings.get()["mqtt"]["password"],
            json!("7RbR4NBSskS8TKG4ugzdRilXw6TPQ4NKFs259j2DxeultOFtl5JwciNZFd6feT5o")
        );
        assert_eq!(settings.get()["devices"], json!({}));
        assert_eq!(settings.get()["groups"], json!({}));
    }
}

#[test]
fn should_apply_environment_variables_as_overrides() {
    let mut settings = Settings::with_files(BTreeMap::from([
        (
            CONFIGURATION_FILE.to_string(),
            json!({"mqtt": {"password": "config-password"}}),
        ),
        (SECRET_FILE.to_string(), json!({"password": "the-password"})),
        (
            DEVICES_FILE.to_string(),
            json!({"0x00158d00018255df": {"friendly_name": "0x00158d00018255df", "retain": false}}),
        ),
    ]));
    settings.env = BTreeMap::from([
        (
            "ZIGBEE2MQTT_CONFIG_MQTT_PASSWORD".to_string(),
            "!secret.yaml password".to_string(),
        ),
        (
            "ZIGBEE2MQTT_CONFIG_SERIAL_DISABLE_LED".to_string(),
            "true".to_string(),
        ),
        (
            "ZIGBEE2MQTT_CONFIG_ADVANCED_CHANNEL".to_string(),
            "15".to_string(),
        ),
        (
            "ZIGBEE2MQTT_CONFIG_ADVANCED_OUTPUT".to_string(),
            "attribute_and_json".to_string(),
        ),
        (
            "ZIGBEE2MQTT_CONFIG_ADVANCED_LOG_OUTPUT".to_string(),
            "[\"console\"]".to_string(),
        ),
        (
            "ZIGBEE2MQTT_CONFIG_MAP_OPTIONS_GRAPHVIZ_COLORS_FILL".to_string(),
            "{\"enddevice\": \"#ff0000\", \"coordinator\": \"#00ff00\", \"router\": \"#0000ff\"}"
                .to_string(),
        ),
        (
            "ZIGBEE2MQTT_CONFIG_MQTT_BASE_TOPIC".to_string(),
            "testtopic".to_string(),
        ),
        (
            "ZIGBEE2MQTT_CONFIG_MQTT_SERVER".to_string(),
            "testserver".to_string(),
        ),
        (
            "ZIGBEE2MQTT_CONFIG_ADVANCED_NETWORK_KEY".to_string(),
            "GENERATE".to_string(),
        ),
        (
            "ZIGBEE2MQTT_CONFIG_DEVICES".to_string(),
            "devices.yaml".to_string(),
        ),
    ]);

    // Write trice to ensure there are no side effects.
    for _ in 0..3 {
        assert!(settings.write());
        assert_eq!(settings.validate(), Vec::<String>::new());

        assert_eq!(settings.get()["serial"]["disable_led"], json!(true));
        assert_eq!(settings.get()["advanced"]["channel"], json!(15));
        assert_eq!(settings.get()["advanced"]["log_output"], json!(["console"]));
        assert_eq!(
            settings.get()["advanced"]["output"],
            json!("attribute_and_json")
        );
        assert_eq!(
            settings.get()["map_options"]["graphviz"]["colors"]["fill"],
            json!({"enddevice": "#ff0000", "coordinator": "#00ff00", "router": "#0000ff"})
        );
        assert_eq!(settings.get()["mqtt"]["base_topic"], json!("testtopic"));
        assert_eq!(settings.get()["mqtt"]["server"], json!("testserver"));
        assert_eq!(settings.get()["mqtt"]["password"], json!("the-password"));
        assert_eq!(settings.get()["advanced"]["network_key"], json!("GENERATE"));
        assert_eq!(
            settings.get()["devices"]["0x00158d00018255df"]["retain"],
            json!(false)
        );

        settings.set(&["advanced", "channel"], json!(25));
        assert_eq!(settings.get()["advanced"]["channel"], json!(15));
        assert_eq!(
            settings.read_file(CONFIGURATION_FILE)["advanced"]["channel"],
            json!(15)
        );

        assert_eq!(
            settings.read_file(SECRET_FILE)["password"],
            json!("the-password")
        );
        assert_eq!(
            settings.read_file(CONFIGURATION_FILE)["mqtt"]["password"],
            json!("!secret.yaml password")
        );
    }
}

#[test]
fn should_keep_homeassistant_null_properties_on_apply() {
    let mut settings = Settings::from_config(json!({
        "device_options": {
            "homeassistant": {"temperature": null},
        },
        "devices": {
            "0x1234567812345678": {
                "friendly_name": "custom discovery",
                "homeassistant": {"humidity": null},
            },
        },
    }));
    settings.re_read();
    settings.apply(json!({"external_converters": []}));

    assert_eq!(
        settings.get()["device_options"]["homeassistant"],
        json!({"temperature": null})
    );
    assert_eq!(
        settings.get()["devices"]["0x1234567812345678"]["homeassistant"],
        json!({"humidity": null})
    );
}

#[test]
fn should_keep_homeassistant_null_property_on_device_setting_change() {
    let mut settings = Settings::from_config(json!({
        "devices": {
            "0x12345678": {
                "friendly_name": "custom discovery",
                "homeassistant": {
                    "entityXYZ": {"entity_category": null},
                },
            },
        },
    }));

    settings
        .change_entity_options("0x12345678", json!({"disabled": true}))
        .unwrap();

    assert_eq!(
        settings.configuration,
        json!({
            "devices": {
                "0x12345678": {
                    "friendly_name": "custom discovery",
                    "disabled": true,
                    "homeassistant": {
                        "entityXYZ": {"entity_category": null},
                    },
                },
            },
        })
    );
}

#[test]
fn should_not_add_duplicate_groups() {
    let mut settings = Settings::from_config(json!({}));

    settings.add_group("test123", None);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        settings.add_group("test123", None);
    }));
    assert!(result.is_err());
    assert_eq!(
        settings.get()["groups"],
        json!({"1": {"friendly_name": "test123"}})
    );
}

#[test]
fn should_not_add_duplicate_groups_with_specific_id() {
    let mut settings = Settings::from_config(json!({}));

    settings.add_group("test123", Some("123"));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        settings.add_group("test_id_123", Some("123"));
    }));
    assert!(result.is_err());
    assert_eq!(
        settings.get()["groups"],
        json!({"123": {"friendly_name": "test123"}})
    );
}

#[test]
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
fn should_not_throw_error_when_devices_is_null() {
    let settings = Settings::from_config(json!({"devices": null}));
    // Upstream only checks that this does not throw.
    let _ = settings.get_device("0x12345678");
}

#[test]
fn should_read_all_secrets_form_a_separate_file() {
    let configuration = json!({
        "mqtt": {
            "server": "!secret server",
            "user": "!secret username",
            "password": "!secret.yaml password",
        },
        "advanced": {"network_key": "!secret network_key"},
    });
    let secret = json!({
        "server": "my.mqtt.server",
        "username": "mysecretusername",
        "password": "mysecretpassword",
        "network_key": [1, 2, 3],
    });
    let mut settings = Settings::with_files(BTreeMap::from([
        (SECRET_FILE.to_string(), secret.clone()),
        (CONFIGURATION_FILE.to_string(), configuration.clone()),
    ]));

    assert_eq!(
        settings.get()["mqtt"]["password"],
        json!("mysecretpassword")
    );
    assert_eq!(settings.get()["mqtt"]["server"], json!("my.mqtt.server"));
    assert_eq!(settings.get()["mqtt"]["user"], json!("mysecretusername"));
    assert_eq!(settings.get()["advanced"]["network_key"], json!([1, 2, 3]));

    settings.testing_write();
    assert_eq!(settings.read_file(CONFIGURATION_FILE), configuration);
    assert_eq!(settings.read_file(SECRET_FILE), secret);

    settings.set(&["mqtt", "server"], json!("not.secret.server"));
    assert_eq!(settings.read_file(CONFIGURATION_FILE), configuration);
    assert_eq!(
        settings.read_file(SECRET_FILE),
        json!({
            "server": "not.secret.server",
            "username": "mysecretusername",
            "password": "mysecretpassword",
            "network_key": [1, 2, 3],
        })
    );
}

#[test]
fn should_read_mqtt_username_and_password_form_a_separate_file() {
    let configuration = json!({
        "mqtt": {
            "server": "my.mqtt.server",
            "user": "!secret username",
            "password": "!secret password",
        },
        "advanced": {"network_key": "!secret network_key"},
    });
    let secret = json!({
        "username": "mysecretusername",
        "password": "mysecretpassword",
        "network_key": [1, 2, 3],
    });
    let mut settings = Settings::with_files(BTreeMap::from([
        (SECRET_FILE.to_string(), secret.clone()),
        (CONFIGURATION_FILE.to_string(), configuration.clone()),
    ]));

    assert_eq!(
        settings.get()["mqtt"]["password"],
        json!("mysecretpassword")
    );
    assert_eq!(settings.get()["mqtt"]["user"], json!("mysecretusername"));
    assert_eq!(settings.get()["mqtt"]["server"], json!("my.mqtt.server"));
    assert_eq!(settings.get()["advanced"]["network_key"], json!([1, 2, 3]));

    settings.testing_write();
    assert_eq!(settings.read_file(CONFIGURATION_FILE), configuration);
    assert_eq!(settings.read_file(SECRET_FILE), secret);

    settings.set(&["mqtt", "user"], json!("test123"));
    settings.set(&["advanced", "network_key"], json!([1, 2, 3, 4]));
    assert_eq!(settings.read_file(CONFIGURATION_FILE), configuration);
    assert_eq!(
        settings.read_file(SECRET_FILE),
        json!({
            "username": "test123",
            "password": "mysecretpassword",
            "network_key": [1, 2, 3, 4],
        })
    );
}

#[test]
fn should_read_devices() {
    let settings = Settings::from_config(json!({
        "devices": {
            "0x12345678": {
                "friendly_name": "0x12345678",
                "retain": false,
            },
        },
    }));

    assert_eq!(
        settings.get_device("0x12345678"),
        Some(json!({
            "ID": "0x12345678",
            "friendly_name": "0x12345678",
            "retain": false,
        }))
    );
}

#[test]
fn should_read_devices_form_2_separate_files() {
    let settings = Settings::with_files(BTreeMap::from([
        (
            CONFIGURATION_FILE.to_string(),
            json!({"devices": ["devices.yaml", "devices2.yaml"]}),
        ),
        (
            DEVICES_FILE.to_string(),
            json!({"0x12345678": {"friendly_name": "0x12345678", "retain": false}}),
        ),
        (
            DEVICES_FILE2.to_string(),
            json!({"0x87654321": {"friendly_name": "0x87654321", "retain": false}}),
        ),
    ]));

    assert_eq!(
        settings.get_device("0x12345678").unwrap()["friendly_name"],
        json!("0x12345678")
    );
    assert_eq!(
        settings.get_device("0x87654321").unwrap()["friendly_name"],
        json!("0x87654321")
    );
}

#[test]
fn should_read_devices_form_a_separate_file() {
    let settings = Settings::with_files(BTreeMap::from([
        (
            CONFIGURATION_FILE.to_string(),
            json!({"devices": "devices.yaml"}),
        ),
        (
            DEVICES_FILE.to_string(),
            json!({"0x12345678": {"friendly_name": "0x12345678", "retain": false}}),
        ),
    ]));

    assert_eq!(
        settings.get_device("0x12345678"),
        Some(json!({
            "ID": "0x12345678",
            "friendly_name": "0x12345678",
            "retain": false,
        }))
    );
}

#[test]
fn should_read_groups() {
    let settings = Settings::with_config(json!({
        "groups": {"1": {"friendly_name": "123"}},
    }));

    assert_eq!(
        settings.get_group("1"),
        Some(json!({"ID": 1, "friendly_name": "123"}))
    );
}

#[test]
fn should_read_groups_from_a_separate_file() {
    let settings = Settings::with_files(BTreeMap::from([
        (
            CONFIGURATION_FILE.to_string(),
            json!({"groups": "groups.yaml"}),
        ),
        (
            GROUPS_FILE.to_string(),
            json!({"1": {"friendly_name": "123"}}),
        ),
    ]));

    assert_eq!(
        settings.get_group("1"),
        Some(json!({"ID": 1, "friendly_name": "123"}))
    );
}

#[test]
fn should_return_default_settings() {
    let settings = Settings::with_config(json!({}));

    assert_eq!(settings.get()["devices"], json!({}));
    assert_eq!(settings.get()["groups"], json!({}));
    assert_eq!(settings.get()["mqtt"]["base_topic"], json!("zigbee2mqtt"));
}

#[test]
fn should_return_settings() {
    let settings = Settings::with_config(json!({"serial": {"disable_led": true}}));

    assert_eq!(settings.get()["devices"], json!({}));
    assert_eq!(settings.get()["groups"], json!({}));
    assert_eq!(settings.get()["serial"], json!({"disable_led": true}));
}

#[test]
fn should_throw_error_when_changing_entity_friendly_name_of_non_existing_device() {
    let mut settings = Settings::from_config(json!({}));

    let error = settings
        .change_friendly_name("not_existing_123", "non_existing_456")
        .unwrap_err();
    assert_eq!(error, "Device or group 'not_existing_123' does not exist");
}

#[test]
fn should_throw_error_when_changing_entity_options_of_non_existing_device() {
    let mut settings = Settings::from_config(json!({}));

    let error = settings
        .change_entity_options("not_existing_123", json!({}))
        .unwrap_err();
    assert_eq!(error, "Device or group 'not_existing_123' does not exist");
}

#[test]
fn should_throw_when_adding_device_which_already_exists() {
    let mut settings = Settings::from_config(json!({
        "devices": {
            "0x123": {"friendly_name": "bulb", "retain": true},
        },
    }));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        settings.add_device("0x123");
    }));
    assert!(result.is_err());
}

#[test]
fn should_throw_when_removing_device_which_doesnt_exist() {
    let mut settings = Settings::from_config(json!({
        "devices": {
            "0x0017880104e45519": {"friendly_name": "myname", "retain": false},
            "0x0017880104e45511": {"friendly_name": "myname1", "retain": false},
        },
    }));
    settings.re_read();

    let error = settings.remove_device("myname33").unwrap_err();
    assert_eq!(error, "Device 'myname33' does not exist");
}

#[test]
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
fn should_write_environment_variables_as_overrides_to_configuration_yaml_not_in_the_ref_file() {
    let mut settings = Settings::with_files(BTreeMap::from([
        (
            SECRET_FILE.to_string(),
            json!({"password": "password-in-secret-file"}),
        ),
        (
            CONFIGURATION_FILE.to_string(),
            json!({"mqtt": {"password": "!secret password", "server": "server"}}),
        ),
    ]));
    settings.env = BTreeMap::from([(
        "ZIGBEE2MQTT_CONFIG_MQTT_PASSWORD".to_string(),
        "password-in-env-var".to_string(),
    )]);

    for _ in 0..3 {
        assert!(settings.write());
        assert_eq!(settings.validate(), Vec::<String>::new());

        assert_eq!(
            settings.get()["mqtt"]["password"],
            json!("password-in-env-var")
        );
        assert_eq!(settings.get()["mqtt"]["server"], json!("server"));
        assert_eq!(settings.get()["groups"], json!({}));
        assert_eq!(settings.get()["devices"], json!({}));
        assert_eq!(
            settings.read_file(SECRET_FILE)["password"],
            json!("password-in-secret-file")
        );
        assert_eq!(
            settings.read_file(CONFIGURATION_FILE)["mqtt"]["password"],
            json!("password-in-env-var")
        );
        assert_eq!(
            settings.read_file(CONFIGURATION_FILE)["mqtt"]["server"],
            json!("server")
        );
    }
}

#[test]
fn shouldnt_write_to_configuration_yaml_when_there_are_no_changes_in_it() {
    let mut settings = Settings::with_files(BTreeMap::from([
        (
            CONFIGURATION_FILE.to_string(),
            json!({"devices": "devices.yaml"}),
        ),
        (DEVICES_FILE.to_string(), json!({})),
    ]));

    settings.add_device("0x1234");

    assert!(!settings.write());
}

#[test]
fn throw_if_removing_non_existing_group() {
    let mut settings = Settings::with_config(json!({
        "groups": {"1": {"friendly_name": "123"}},
    }));

    let error = settings.remove_group("2").unwrap_err();
    assert_eq!(error, "Group '2' does not exist");
}

// Upstream: it.each([...])("Device icons should be in `device_icons` directory or a url", ...)
#[test]
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
