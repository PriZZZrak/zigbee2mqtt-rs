// Port of .upstream/zigbee2mqtt/test/settingsMigration.test.ts
//
// Upstream drives the settings module + filesystem. The Rust port passes the
// settings/state documents explicitly and asserts the returned migration notes
// instead of reading `migration-X-to-Y.log`.

use serde_json::{json, Value};
use z2m_rs::util::settings_migration::migrate_if_necessary;

fn before_settings() -> Value {
    json!({
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "mqtt://localhost"},
        "serial": {"port": "/dev/dummy"},
    })
}

#[test]
fn fails_on_unsupported_version() {
    let mut settings = json!({"version": 0, "mqtt": {"server": "mqtt://localhost"}});
    let error = migrate_if_necessary(&mut settings, 5, None).unwrap_err();
    assert!(error.contains("unsupported version 0"), "error={error}");

    let mut settings = json!({"version": 99999, "mqtt": {"server": "mqtt://localhost"}});
    let error = migrate_if_necessary(&mut settings, 5, None).unwrap_err();
    assert!(error.contains("unsupported version 99999"), "error={error}");
}

#[test]
fn no_change_needed_only_add_version() {
    // Upstream: "no change needed - only add version" (Migrates v1 to v2).
    let mut settings = before_settings();
    let before = settings.clone();

    let outcome = migrate_if_necessary(&mut settings, 2, None).unwrap();

    let mut expected = before;
    expected["version"] = json!(2);
    assert_eq!(settings, expected);
    assert!(outcome.notes.iter().any(|note| note.contains("version 2")));
}

#[test]
fn remove_all() {
    // Upstream: "remove all" (Migrates v1 to v2).
    let mut settings = json!({
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "mqtt://localhost"},
        "serial": {"port": "/dev/dummy"},
        "homeassistant": {"legacy_triggers": true, "legacy_entity_attributes": true},
        "ota": {"ikea_ota_use_test_url": true},
        "advanced": {
            "homeassistant_legacy_triggers": true,
            "homeassistant_legacy_entity_attributes": true,
            "ikea_ota_use_test_url": true,
            "legacy_api": true,
            "legacy_availability_payload": true,
            "soft_reset_timeout": 12,
            "report": true,
            "availability_timeout": 65,
            "availability_blocklist": ["abcd", "efgh"],
            "availability_passlist": ["abcd"],
            "availability_blacklist": ["abcd", "efgh"],
            "availability_whitelist": ["abcd", "efgh"],
        },
        "permit_join": true,
        "device_options": {"legacy": true},
        "devices": {
            "0x18fc2600000d7ae2": {"retrieve_state": true},
            "0x000b57fffec6a5b2": {"retrieve_state": true},
        },
        "groups": {
            "15071": {"retrieve_state": true},
            "12": {"devices": ["0x0017880104e45521", "0x0017880104e45524"]},
        },
        "external_converters": ["zyx.js"],
    });

    let outcome = migrate_if_necessary(&mut settings, 2, None).unwrap();

    assert_eq!(settings["version"], json!(2));
    assert_eq!(settings["advanced"], json!({}));
    assert_eq!(settings["device_options"], json!({}));
    assert_eq!(settings["ota"], json!({}));
    assert_eq!(settings["homeassistant"], json!({}));
    assert_eq!(settings["devices"]["0x18fc2600000d7ae2"], json!({}));
    assert_eq!(settings["groups"]["15071"], json!({}));
    assert_eq!(settings["groups"]["12"], json!({}));

    for needle in [
        "homeassistant.legacy_triggers",
        "homeassistant.legacy_entity_attributes",
        "ota.ikea_ota_use_test_url",
        "permit_join",
        "advanced.legacy_api",
        "advanced.legacy_availability_payload",
        "advanced.soft_reset_timeout",
        "advanced.report",
        "advanced.availability_timeout",
        "advanced.availability_blocklist",
        "advanced.availability_passlist",
        "advanced.availability_blacklist",
        "advanced.availability_whitelist",
        "device_options.legacy",
        "(devices|groups).xyz.retrieve_state",
        "groups.xyz.devices",
        "External converters are now automatically loaded",
    ] {
        assert!(
            outcome.notes.iter().any(|note| note.contains(needle)),
            "missing note {needle}"
        );
    }
}

#[test]
fn remove_partial() {
    // Upstream: "remove partial" (Migrates v1 to v2).
    let mut settings = json!({
        "mqtt": {"base_topic": "zigbee2mqtt", "server": "mqtt://localhost"},
        "serial": {"port": "/dev/dummy"},
        "advanced": {
            "homeassistant_legacy_triggers": true,
            "homeassistant_legacy_entity_attributes": true,
            "ikea_ota_use_test_url": true,
            "legacy_api": true,
            "legacy_availability_payload": false,
            "soft_reset_timeout": 16,
            "report": true,
            "availability_timeout": 64,
            "availability_passlist": [],
        },
        "permit_join": true,
        "device_options": {"legacy": true},
        "groups": {"12": {"devices": ["0x0017880104e45521", "0x0017880104e45524"]}},
    });

    let outcome = migrate_if_necessary(&mut settings, 2, None).unwrap();

    assert_eq!(settings["version"], json!(2));
    assert_eq!(settings["advanced"], json!({}));
    assert_eq!(settings["device_options"], json!({}));

    assert!(outcome
        .notes
        .iter()
        .any(|note| note.contains("homeassistant.legacy_triggers")));
    assert!(outcome
        .notes
        .iter()
        .any(|note| note.contains("homeassistant.legacy_entity_attributes")));
    assert!(outcome
        .notes
        .iter()
        .any(|note| note.contains("ota.ikea_ota_use_test_url")));
    assert!(outcome
        .notes
        .iter()
        .any(|note| note.contains("permit_join")));
    assert!(outcome
        .notes
        .iter()
        .any(|note| note.contains("advanced.legacy_api")));
    // was false, no impact
    assert!(!outcome
        .notes
        .iter()
        .any(|note| note.contains("advanced.legacy_availability_payload")));
    assert!(outcome
        .notes
        .iter()
        .any(|note| note.contains("advanced.soft_reset_timeout")));
    assert!(outcome
        .notes
        .iter()
        .any(|note| note.contains("advanced.report")));
    assert!(outcome
        .notes
        .iter()
        .any(|note| note.contains("advanced.availability_timeout")));
    assert!(!outcome
        .notes
        .iter()
        .any(|note| note.contains("advanced.availability_blocklist")));
    // empty array
    assert!(!outcome
        .notes
        .iter()
        .any(|note| note.contains("advanced.availability_passlist")));
    assert!(!outcome
        .notes
        .iter()
        .any(|note| note.contains("advanced.availability_blacklist")));
    assert!(!outcome
        .notes
        .iter()
        .any(|note| note.contains("advanced.availability_whitelist")));
    assert!(outcome
        .notes
        .iter()
        .any(|note| note.contains("device_options.legacy")));
    assert!(!outcome
        .notes
        .iter()
        .any(|note| note.contains("(devices|groups).xyz.retrieve_state")));
    assert!(outcome
        .notes
        .iter()
        .any(|note| note.contains("groups.xyz.devices")));
}

#[test]
fn changes_log_level() {
    // Upstream: "changes log_level" (Migrates v1 to v2).
    let mut settings = before_settings();
    settings["advanced"] = json!({"log_level": "warn"});

    let outcome = migrate_if_necessary(&mut settings, 2, None).unwrap();

    assert_eq!(settings["version"], json!(2));
    assert_eq!(settings["advanced"], json!({"log_level": "warning"}));
    assert!(outcome
        .notes
        .iter()
        .any(|note| note.contains("Log level 'warn' has been renamed to 'warning'.")));
}

#[test]
fn does_not_changes_already_migrated_log_level() {
    // Upstream: "does not changes already migrated log_level" (Migrates v1 to v2).
    let mut settings = before_settings();
    settings["advanced"] = json!({"log_level": "warning"});

    let outcome = migrate_if_necessary(&mut settings, 2, None).unwrap();

    assert_eq!(settings["version"], json!(2));
    assert_eq!(settings["advanced"], json!({"log_level": "warning"}));
    assert!(!outcome
        .notes
        .iter()
        .any(|note| note.contains("Log level 'warn' has been renamed to 'warning'.")));
}

#[test]
fn does_not_changes_other_log_level() {
    // Upstream: "does not changes other log_level" (Migrates v1 to v2).
    let mut settings = before_settings();
    settings["advanced"] = json!({"log_level": "info"});

    let outcome = migrate_if_necessary(&mut settings, 2, None).unwrap();

    assert_eq!(settings["version"], json!(2));
    assert_eq!(settings["advanced"], json!({"log_level": "info"}));
    assert!(!outcome
        .notes
        .iter()
        .any(|note| note.contains("Log level 'warn' has been renamed to 'warning'.")));
}

#[test]
fn transfer_all() {
    // Upstream: "transfer all" (Migrates v1 to v2).
    let mut settings = before_settings();
    settings["advanced"] = json!({
        "homeassistant_discovery_topic": "ha_disc",
        "homeassistant_status_topic": "ha_stat",
        "baudrate": 115200,
        "rtscts": true,
    });
    settings["serial"] = json!({"port": "/dev/dummy", "rtscts": true});
    settings["experimental"] = json!({"transmit_power": 12, "output": "attribute"});
    settings["ban"] = json!(["abcd"]);
    settings["whitelist"] = json!(["efgh"]);

    let outcome = migrate_if_necessary(&mut settings, 2, None).unwrap();

    assert_eq!(
        settings,
        json!({
            "version": 2,
            "mqtt": {"base_topic": "zigbee2mqtt", "server": "mqtt://localhost"},
            "serial": {"port": "/dev/dummy", "baudrate": 115200, "rtscts": true},
            "advanced": {"transmit_power": 12, "output": "attribute"},
            "blocklist": ["abcd"],
            "passlist": ["efgh"],
            "homeassistant": {
                "discovery_topic": "ha_disc",
                "status_topic": "ha_stat",
            },
        })
    );

    for needle in [
        "HA discovery_topic was moved from advanced.homeassistant_discovery_topic to homeassistant.discovery_topic.",
        "HA status_topic was moved from advanced.homeassistant_status_topic to homeassistant.status_topic.",
        "Baudrate was moved from advanced.baudrate to serial.baudrate.",
        "RTSCTS was moved from advanced.rtscts to serial.rtscts.",
        "Transmit power was moved from experimental.transmit_power to advanced.transmit_power.",
        "Output was moved from experimental.output to advanced.output.",
        "ban was renamed to passlist.",
        "whitelist was renamed to passlist.",
        "The entire experimental section was removed.",
    ] {
        assert!(
            outcome.notes.iter().any(|note| note.contains(needle)),
            "missing note {needle}"
        );
    }
}

#[test]
fn transfer_partial() {
    // Upstream: "transfer partial" (Migrates v1 to v2).
    let mut settings = before_settings();
    settings["homeassistant"] = json!({"discovery_topic": "ha_disc_newer"});
    settings["advanced"] = json!({
        "homeassistant_discovery_topic": "ha_disc",
        "baudrate": 115200,
        "rtscts": true,
    });
    settings["serial"] = json!({"port": "/dev/dummy", "rtscts": true});
    settings["ban"] = json!(["abcd"]);
    settings["blocklist"] = json!(["efgh"]);

    let outcome = migrate_if_necessary(&mut settings, 2, None).unwrap();

    assert_eq!(
        settings,
        json!({
            "version": 2,
            "mqtt": {"base_topic": "zigbee2mqtt", "server": "mqtt://localhost"},
            "serial": {"port": "/dev/dummy", "baudrate": 115200, "rtscts": true},
            "advanced": {},
            "homeassistant": {"discovery_topic": "ha_disc_newer"},
            "blocklist": ["abcd", "efgh"],
        })
    );

    assert!(outcome.notes.iter().any(|note| {
        note.contains("[TRANSFER] Baudrate was moved from advanced.baudrate to serial.baudrate.")
    }));
    assert!(outcome.notes.iter().any(|note| {
        note.contains("[REMOVAL] RTSCTS was moved from advanced.rtscts to serial.rtscts.")
    }));
    assert!(outcome
        .notes
        .iter()
        .any(|note| note.contains("[TRANSFER] ban was renamed to passlist.")));
}

#[test]
fn update() {
    // Upstream: "Update" under "Migrates v1 to v3".
    let mut settings = json!({
        "mqtt": {"server": "mqtt://localhost"},
        "homeassistant": false,
        "frontend": true,
        "availability": {"active": {"timeout": 15}},
        "permit_join": true,
        "advanced": {"log_level": "warn"},
        "experimental": {"transmit_power": 12},
    });

    migrate_if_necessary(&mut settings, 3, None).unwrap();

    assert_eq!(
        settings,
        json!({
            "version": 3,
            "mqtt": {"server": "mqtt://localhost"},
            "homeassistant": {"enabled": false},
            "frontend": {"enabled": true},
            "availability": {"enabled": true, "active": {"timeout": 15}},
            "advanced": {"log_level": "warning", "transmit_power": 12},
        })
    );
}

#[test]
fn update_2() {
    // Upstream: "Update" under "Migrates v2 to v3".
    let mut settings = json!({
        "version": 2,
        "mqtt": {"server": "mqtt://localhost"},
        "homeassistant": false,
        "frontend": true,
        "availability": {"active": {"timeout": 15}},
    });

    let outcome = migrate_if_necessary(&mut settings, 3, None).unwrap();

    assert_eq!(
        settings,
        json!({
            "version": 3,
            "mqtt": {"server": "mqtt://localhost"},
            "homeassistant": {"enabled": false},
            "frontend": {"enabled": true},
            "availability": {"enabled": true, "active": {"timeout": 15}},
        })
    );
    for needle in [
        "[SPECIAL] Property 'homeassistant' is now always an object.",
        "[SPECIAL] Property 'frontend' is now always an object.",
        "[SPECIAL] Property 'availability' is now always an object.",
    ] {
        assert!(outcome.notes.iter().any(|note| note.contains(needle)));
    }
}

#[test]
fn update_when_not_set_tests_that_frontend_availability_is_not_added_when_not_set() {
    let mut settings = json!({
        "version": 2,
        "mqtt": {"server": "mqtt://localhost"},
        "homeassistant": false,
    });

    let outcome = migrate_if_necessary(&mut settings, 3, None).unwrap();

    assert_eq!(
        settings,
        json!({
            "version": 3,
            "mqtt": {"server": "mqtt://localhost"},
            "homeassistant": {"enabled": false},
        })
    );
    assert!(outcome
        .notes
        .iter()
        .any(|note| note.contains("[SPECIAL] Property 'homeassistant' is now always an object.")));
    assert!(!outcome
        .notes
        .iter()
        .any(|note| note.contains("[SPECIAL] Property 'frontend' is now always an object.")));
    assert!(!outcome
        .notes
        .iter()
        .any(|note| note.contains("[SPECIAL] Property 'availability' is now always an object.")));
}

#[test]
fn update_3() {
    // Upstream: "Update" under "Migrates v3 to v4".
    let mut settings = json!({
        "version": 3,
        "mqtt": {"server": "mqtt://localhost"},
        "devices": {
            "0x123127fffe8d96bc": {
                "friendly_name": "0x847127fffe8d96bc",
                "icon": "device_icons/08a9016bbc0657cf5f581ae9c19c31a5.png",
            },
            "0x223127fffe8d96bc": {
                "friendly_name": "0x223127fffe8d96bc",
                "icon": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAJ",
            },
            "0x323127fffe8d96bc": {
                "friendly_name": "0x323127fffe8d96bc",
            },
        },
    });

    let outcome = migrate_if_necessary(&mut settings, 4, None).unwrap();

    assert_eq!(settings["version"], json!(4));
    assert_eq!(
        settings["devices"]["0x123127fffe8d96bc"]["icon"],
        json!("device_icons/08a9016bbc0657cf5f581ae9c19c31a5.png")
    );
    assert_eq!(
        settings["devices"]["0x223127fffe8d96bc"]["icon"],
        json!("device_icons/effcad234beeb56ea7c457cf2d36d10b.png")
    );
    assert_eq!(
        settings["devices"]["0x323127fffe8d96bc"],
        json!({"friendly_name": "0x323127fffe8d96bc"})
    );
    assert!(outcome
        .notes
        .iter()
        .any(|note| note.contains("[SPECIAL] Device icons are now saved as images.")));
}

#[test]
fn update_4() {
    // Upstream: "Update" under "Migrates v4 to v5".
    let mut settings = json!({
        "version": 4,
        "mqtt": {"server": "mqtt://localhost"},
    });
    let before_settings = settings.clone();
    let mut state = json!({
        "0x000b57fffec6a5b2": {"state": "ON", "brightness": 50, "color_temp": 370, "linkquality": 99},
        "0x0017880104e45517": {"brightness": 255, "update": {"state": "idle"}},
        "1": {"state": "ON", "update": {"state": "available", "installed_version": 1, "latest_version": 2}},
    });
    let default_state = state.clone();

    assert_eq!(settings, before_settings);
    assert_eq!(state, default_state);

    migrate_if_necessary(&mut settings, 5, Some(&mut state)).unwrap();

    let mut expected_settings = before_settings;
    expected_settings["version"] = json!(5);
    assert_eq!(settings, expected_settings);

    let mut expected_state = default_state;
    expected_state["0x0017880104e45517"]
        .as_object_mut()
        .unwrap()
        .remove("update");
    expected_state["1"]
        .as_object_mut()
        .unwrap()
        .remove("update");
    assert_eq!(state, expected_state);
}

#[test]
fn handles_errors_gracefully() {
    let mut settings = json!({
        "version": 4,
        "mqtt": {"server": "mqtt://localhost"},
    });
    let before_settings = settings.clone();
    // Simulates `state.json` containing invalid JSON.
    let mut state = json!("notjson");

    assert_eq!(settings, before_settings);

    let outcome = migrate_if_necessary(&mut settings, 5, Some(&mut state)).unwrap();

    let mut expected = before_settings;
    expected["version"] = json!(5);
    assert_eq!(settings, expected);
    assert!(outcome
        .errors
        .iter()
        .any(|error| error.contains("Failed to write state")));
    assert!(outcome
        .errors
        .iter()
        .any(|error| error.contains("is not valid JSON")));
}
