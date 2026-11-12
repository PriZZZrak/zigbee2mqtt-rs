// Original: .upstream/zigbee2mqtt/lib/util/settings.ts:20-27
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// Differential fixtures for the Ajv replacement in
// `z2m_rs::util::settings_schema`. Expected messages were captured from
// Ajv 8.20.0 with the same `settings.schema.json` (`allErrors: true`).

use serde_json::{json, Value};

use z2m_rs::util::settings_schema::{restart_required, validate};

fn errors(instance: Value) -> Vec<String> {
    validate(&instance)
        .iter()
        .map(|error| error.render())
        .collect()
}

fn keywords(instance: Value) -> Vec<&'static str> {
    validate(&instance)
        .iter()
        .map(|error| error.keyword)
        .collect()
}

#[test]
fn schema_copy_matches_the_pinned_upstream_file() {
    let upstream = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.upstream/zigbee2mqtt/lib/util/settings.schema.json");
    if !upstream.exists() {
        return;
    }

    let upstream: Value =
        serde_json::from_str(&std::fs::read_to_string(&upstream).unwrap()).unwrap();
    assert_eq!(z2m_rs::util::settings_schema::schema(), &upstream);
}

#[test]
fn missing_required_property() {
    assert_eq!(
        errors(json!({})),
        vec![" must have required property 'mqtt'".to_string()]
    );
    assert_eq!(keywords(json!({})), vec!["required"]);
}

#[test]
fn non_required_errors_skip_required_keyword() {
    let filtered: Vec<String> = validate(&json!({}))
        .into_iter()
        .filter(|error| error.keyword != "required")
        .map(|error| error.render())
        .collect();
    assert!(filtered.is_empty());
}

#[test]
fn type_and_enum_are_both_reported_like_ajv() {
    // Ajv 8.20: [["type","/advanced/log_level"],["enum","/advanced/log_level"]]
    assert_eq!(
        errors(json!({"mqtt": {"server": "localhost"}, "advanced": {"log_level": 123}})),
        vec![
            "advanced/log_level must be string".to_string(),
            "advanced/log_level must be equal to one of the allowed values".to_string(),
        ]
    );
    assert_eq!(
        keywords(json!({"mqtt": {"server": "localhost"}, "advanced": {"log_level": 123}})),
        vec!["type", "enum"]
    );
}

#[test]
fn union_type_message_joins_types_with_a_comma() {
    assert_eq!(
        errors(json!({"mqtt": {"server": "localhost"}, "serial": {"port": 123}})),
        vec!["serial/port must be string,null".to_string()]
    );
}

#[test]
fn nested_property_type_error() {
    assert_eq!(
        errors(json!({"mqtt": {"server": 123}})),
        vec!["mqtt/server must be string".to_string()]
    );
    assert_eq!(
        errors(json!({"mqtt": {"server": "localhost"}, "frontend": {"port": "8080"}})),
        vec!["frontend/port must be number".to_string()]
    );
}

#[test]
fn object_type_error() {
    assert_eq!(
        errors(json!({"mqtt": {"server": "localhost"}, "advanced": null})),
        vec!["advanced must be object".to_string()]
    );
    assert_eq!(
        errors(json!({"mqtt": {"server": "localhost"}, "blocklist": "0x123"})),
        vec!["blocklist must be array".to_string()]
    );
}

#[test]
fn array_item_errors_use_the_index() {
    assert_eq!(
        errors(json!({"mqtt": {"server": "localhost"}, "advanced": {"log_output": ["file", "x"]}})),
        vec!["advanced/log_output/1 must be equal to one of the allowed values".to_string()]
    );
}

#[test]
fn numeric_bounds() {
    assert_eq!(
        errors(json!({"mqtt": {"server": "localhost"}, "advanced": {"channel": 0}})),
        vec!["advanced/channel must be >= 11".to_string()]
    );
    assert_eq!(
        errors(json!({"mqtt": {"server": "localhost"}, "advanced": {"channel": 100}})),
        vec!["advanced/channel must be <= 26".to_string()]
    );
}

#[test]
fn one_of_reports_branch_errors_and_the_one_of_error() {
    assert_eq!(
        errors(json!({"mqtt": {"server": "localhost"}, "advanced": {"network_key": 123}})),
        vec![
            "advanced/network_key must be string".to_string(),
            "advanced/network_key must be array".to_string(),
            "advanced/network_key must match exactly one schema in oneOf".to_string(),
        ]
    );
}

#[test]
fn one_of_accepts_the_generate_string() {
    assert!(
        errors(json!({"mqtt": {"server": "localhost"}, "advanced": {"pan_id": "GENERATE"}}))
            .is_empty()
    );
}

#[test]
fn property_names_errors_match_ajv_order() {
    assert_eq!(
        errors(json!({
            "mqtt": {"server": "localhost"},
            "devices": {"bad": {"friendly_name": "a", "qos": 3}},
        })),
        vec![
            "devices must match pattern \"^0x[\\\\d\\\\w]{16}$\"".to_string(),
            "devices property name must be valid".to_string(),
            "devices/bad/qos must be equal to one of the allowed values".to_string(),
        ]
    );
}

#[test]
fn valid_settings_have_no_errors() {
    assert!(errors(json!({
        "mqtt": {"server": "localhost"},
        "devices": {"0x0017880104e45519": {"friendly_name": "a", "retention": 900}},
    }))
    .is_empty());
}

#[test]
fn restart_required_tracks_present_requires_restart_properties() {
    // From `test/extensions/bridge.test.ts` "Change options ..." cases.
    assert!(restart_required(
        &json!({"serial": {"port": "/dev/newport"}})
    ));
    assert!(restart_required(
        &json!({"homeassistant": {"enabled": true}})
    ));
    assert!(restart_required(&json!({"health": {"interval": 11}})));
    assert!(restart_required(&json!({
        "advanced": {"ext_pan_id": [220, 221, 221, 221, 221, 221, 221, 221]},
    })));
    assert!(restart_required(&json!({"blocklist": []})));

    // Present but null still triggers the keyword (Ajv validates the schema
    // keyword value `true`, not the instance value).
    assert!(restart_required(
        &json!({"serial": {"disable_led": false, "port": null}})
    ));
}

#[test]
fn restart_required_is_false_for_options_applied_on_the_fly() {
    assert!(!restart_required(
        &json!({"advanced": {"log_level": "debug"}})
    ));
    assert!(!restart_required(&json!({
        "advanced": {"log_debug_namespace_ignore": "^zh:controller"},
    })));
    assert!(!restart_required(&json!({
        "advanced": {"log_namespaced_levels": {"z2m:mqtt": "debug"}},
    })));
    assert!(!restart_required(
        &json!({"health": {"reset_on_check": true}})
    ));
    assert!(!restart_required(&json!({"external_converters": []})));
    assert!(!restart_required(&json!({})));
}

#[test]
fn restart_required_applies_to_device_and_group_options() {
    assert!(restart_required(&json!({
        "devices": {"0x0017880104e45519": {"disabled": true}},
    })));
    assert!(restart_required(&json!({
        "devices": {"0x0017880104e45519": {"throttle": 1}},
    })));
    assert!(restart_required(
        &json!({"groups": {"1": {"off_state": "all_members_off"}}})
    ));
    assert!(!restart_required(&json!({
        "devices": {"0x0017880104e45519": {"retain": true}},
    })));
}
