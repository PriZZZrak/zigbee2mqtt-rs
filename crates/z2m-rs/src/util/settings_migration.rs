// Original: .upstream/zigbee2mqtt/lib/util/settingsMigration.ts

use serde_json::{Map, Value};

#[derive(Debug, Clone, Default)]
pub struct MigrationOutcome {
    pub notes: Vec<String>,
    pub errors: Vec<String>,
}

/// Upstream: `migrateIfNecessary()`.
pub fn migrate_if_necessary(
    settings: &mut Value,
    current_version: u32,
    state: Option<&mut Value>,
) -> Result<MigrationOutcome, String> {
    let mut outcome = MigrationOutcome::default();
    let version = settings.get("version").and_then(Value::as_u64).unwrap_or(1) as u32;

    if !matches!(version, 1 | 2 | 3 | 4 | 5) {
        return Err(format!("settings have unsupported version {version}"));
    }

    let mut version = version;
    if version < 2 && current_version >= 2 {
        migrate_to_two(settings, &mut outcome);
        version = 2;
    }
    if version < 3 && current_version >= 3 {
        migrate_to_three(settings, &mut outcome);
        version = 3;
    }
    if version < 4 && current_version >= 4 {
        migrate_to_four(settings, &mut outcome);
        version = 4;
    }
    if version < 5 && current_version >= 5 {
        migrate_to_five(settings, state, &mut outcome);
        version = 5;
    }

    if version > 1 {
        settings["version"] = Value::from(version);
    }
    Ok(outcome)
}

fn get<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn set(value: &mut Value, path: &[&str], new_value: Value) {
    if path.is_empty() {
        *value = new_value;
        return;
    }
    let mut current = value;
    for key in &path[..path.len() - 1] {
        if !current.get(*key).is_some_and(Value::is_object) {
            current[*key] = Value::Object(Map::new());
        }
        current = current.get_mut(*key).unwrap();
    }
    current[path[path.len() - 1]] = new_value;
}

fn remove(value: &mut Value, path: &[&str]) {
    let mut current = value;
    for key in &path[..path.len() - 1] {
        let Some(next) = current.get_mut(*key) else {
            return;
        };
        current = next;
    }
    if let Value::Object(map) = current {
        map.remove(path[path.len() - 1]);
    }
}

fn should_note(value: &Value) -> bool {
    match value {
        Value::Bool(value) => *value,
        Value::Array(items) => !items.is_empty(),
        Value::Null => false,
        _ => true,
    }
}

fn transfer(
    settings: &mut Value,
    from: &[&str],
    to: &[&str],
    note: &str,
    outcome: &mut MigrationOutcome,
) {
    let Some(previous) = get(settings, from).cloned() else {
        return;
    };
    remove(settings, from);

    let destination = get(settings, to).cloned();
    let transferred = destination.is_none() || destination.as_ref().is_some_and(Value::is_null);

    if transferred {
        set(settings, to, previous);
        if should_note(get(settings, to).unwrap_or(&Value::Null)) {
            outcome.notes.push(format!("[TRANSFER] {note}"));
        }
    } else if should_note(&previous) {
        outcome.notes.push(format!("[REMOVAL] {note}"));
    }
}

fn merge_array(
    settings: &mut Value,
    from: &[&str],
    to: &[&str],
    note: &str,
    outcome: &mut MigrationOutcome,
) {
    let Some(Value::Array(previous)) = get(settings, from).cloned() else {
        return;
    };
    remove(settings, from);
    let existing = match get(settings, to).cloned() {
        Some(Value::Array(existing)) => existing,
        _ => Vec::new(),
    };
    let mut merged = previous;
    merged.extend(existing);
    set(settings, to, Value::Array(merged));
    outcome.notes.push(format!("[TRANSFER] {note}"));
}

fn remove_item(settings: &mut Value, path: &[&str], note: &str, outcome: &mut MigrationOutcome) {
    if let Some(value) = get(settings, path).cloned() {
        remove(settings, path);
        if should_note(&value) {
            outcome.notes.push(note.to_string());
        }
    }
}

fn migrate_to_two(settings: &mut Value, outcome: &mut MigrationOutcome) {
    transfer(
        settings,
        &["experimental", "transmit_power"],
        &["advanced", "transmit_power"],
        "Transmit power was moved from experimental.transmit_power to advanced.transmit_power.",
        outcome,
    );
    transfer(
        settings,
        &["experimental", "output"],
        &["advanced", "output"],
        "Output was moved from experimental.output to advanced.output.",
        outcome,
    );
    transfer(
        settings,
        &["advanced", "baudrate"],
        &["serial", "baudrate"],
        "Baudrate was moved from advanced.baudrate to serial.baudrate.",
        outcome,
    );
    transfer(
        settings,
        &["advanced", "rtscts"],
        &["serial", "rtscts"],
        "RTSCTS was moved from advanced.rtscts to serial.rtscts.",
        outcome,
    );
    transfer(
        settings,
        &["advanced", "homeassistant_discovery_topic"],
        &["homeassistant", "discovery_topic"],
        "HA discovery_topic was moved from advanced.homeassistant_discovery_topic to homeassistant.discovery_topic.",
        outcome,
    );
    transfer(
        settings,
        &["advanced", "homeassistant_status_topic"],
        &["homeassistant", "status_topic"],
        "HA status_topic was moved from advanced.homeassistant_status_topic to homeassistant.status_topic.",
        outcome,
    );

    merge_array(
        settings,
        &["ban"],
        &["blocklist"],
        "ban was renamed to passlist.",
        outcome,
    );
    merge_array(
        settings,
        &["whitelist"],
        &["passlist"],
        "whitelist was renamed to passlist.",
        outcome,
    );

    if get(settings, &["advanced", "log_level"]) == Some(&Value::String("warn".to_string())) {
        set(
            settings,
            &["advanced", "log_level"],
            Value::String("warning".to_string()),
        );
        outcome
            .notes
            .push("Log level 'warn' has been renamed to 'warning'.".to_string());
    }

    for (path, note) in [
        (
            &["homeassistant", "legacy_triggers"][..],
            "homeassistant.legacy_triggers",
        ),
        (
            &["homeassistant", "legacy_entity_attributes"][..],
            "homeassistant.legacy_entity_attributes",
        ),
        (
            &["advanced", "homeassistant_legacy_triggers"][..],
            "homeassistant.legacy_triggers",
        ),
        (
            &["advanced", "homeassistant_legacy_entity_attributes"][..],
            "homeassistant.legacy_entity_attributes",
        ),
        (
            &["ota", "ikea_ota_use_test_url"][..],
            "ota.ikea_ota_use_test_url",
        ),
        (
            &["advanced", "ikea_ota_use_test_url"][..],
            "ota.ikea_ota_use_test_url",
        ),
        (&["permit_join"][..], "permit_join"),
        (&["advanced", "legacy_api"][..], "advanced.legacy_api"),
        (
            &["advanced", "legacy_availability_payload"][..],
            "advanced.legacy_availability_payload",
        ),
        (
            &["advanced", "soft_reset_timeout"][..],
            "advanced.soft_reset_timeout",
        ),
        (&["advanced", "report"][..], "advanced.report"),
        (
            &["advanced", "availability_timeout"][..],
            "advanced.availability_timeout",
        ),
        (
            &["advanced", "availability_blocklist"][..],
            "advanced.availability_blocklist",
        ),
        (
            &["advanced", "availability_passlist"][..],
            "advanced.availability_passlist",
        ),
        (
            &["advanced", "availability_blacklist"][..],
            "advanced.availability_blacklist",
        ),
        (
            &["advanced", "availability_whitelist"][..],
            "advanced.availability_whitelist",
        ),
        (&["device_options", "legacy"][..], "device_options.legacy"),
    ] {
        remove_item(settings, path, note, outcome);
    }

    if get(settings, &["experimental"]).is_some() {
        remove(settings, &["experimental"]);
        outcome
            .notes
            .push("The entire experimental section was removed.".to_string());
    }

    let mut retrieve_state_found = false;
    for key in ["devices", "groups"] {
        if let Some(Value::Object(entries)) = settings.get_mut(key) {
            for entry in entries.values_mut() {
                if entry.get("retrieve_state").is_some() {
                    retrieve_state_found = true;
                    if let Value::Object(map) = entry {
                        map.remove("retrieve_state");
                    }
                }
            }
        }
    }
    if retrieve_state_found {
        outcome
            .notes
            .push("(devices|groups).xyz.retrieve_state".to_string());
    }

    let mut group_devices_found = false;
    if let Some(Value::Object(groups)) = settings.get_mut("groups") {
        for entry in groups.values_mut() {
            if entry.get("devices").is_some() {
                group_devices_found = true;
                if let Value::Object(map) = entry {
                    map.remove("devices");
                }
            }
        }
    }
    if group_devices_found {
        outcome.notes.push("groups.xyz.devices".to_string());
    }

    if get(settings, &["external_converters"]).is_some() {
        remove(settings, &["external_converters"]);
        outcome
            .notes
            .push("External converters are now automatically loaded.".to_string());
    }

    settings["version"] = Value::from(2);
    outcome
        .notes
        .push("Migrated settings to version 2".to_string());
}

fn to_object(value: &mut Value, note: &str, outcome: &mut MigrationOutcome) {
    if let Value::Bool(enabled) = value {
        *value = serde_json::json!({"enabled": enabled});
        outcome.notes.push(note.to_string());
    }
}

fn migrate_to_three(settings: &mut Value, outcome: &mut MigrationOutcome) {
    for (key, note) in [
        (
            "homeassistant",
            "[SPECIAL] Property 'homeassistant' is now always an object.",
        ),
        (
            "frontend",
            "[SPECIAL] Property 'frontend' is now always an object.",
        ),
        (
            "availability",
            "[SPECIAL] Property 'availability' is now always an object.",
        ),
    ] {
        if let Some(value) = settings.get_mut(key) {
            to_object(value, note, outcome);
        }
    }
    if let Some(value) = settings.get_mut("availability") {
        if let Value::Object(map) = value {
            if !map.contains_key("enabled") {
                map.insert("enabled".to_string(), Value::Bool(true));
                outcome
                    .notes
                    .push("[SPECIAL] Property 'availability' is now always an object.".to_string());
            }
        }
    }
    settings["version"] = Value::from(3);
    outcome
        .notes
        .push("Migrated settings to version 3".to_string());
}

fn migrate_to_four(settings: &mut Value, outcome: &mut MigrationOutcome) {
    let mut changed = false;
    if let Some(Value::Object(devices)) = settings.get_mut("devices") {
        for device in devices.values_mut() {
            let Some(icon) = device.get("icon").and_then(Value::as_str) else {
                continue;
            };
            if let Some((extension, data)) = crate::util::utils::match_base64_file(icon) {
                let hash = crate::util::utils::md5_hex(data.as_bytes());
                device["icon"] = Value::String(format!("device_icons/{hash}.{extension}"));
                changed = true;
            }
        }
    }
    if changed {
        outcome
            .notes
            .push("[SPECIAL] Device icons are now saved as images.".to_string());
    }
    settings["version"] = Value::from(4);
    outcome
        .notes
        .push("Migrated settings to version 4".to_string());
}

fn migrate_to_five(
    settings: &mut Value,
    state: Option<&mut Value>,
    outcome: &mut MigrationOutcome,
) {
    if let Some(state) = state {
        match state {
            Value::Object(entries) => {
                for entry in entries.values_mut() {
                    if let Value::Object(map) = entry {
                        map.remove("update");
                    }
                }
            }
            _ => {
                outcome
                    .errors
                    .push("Failed to write state: state is not valid JSON".to_string());
            }
        }
    }
    settings["version"] = Value::from(5);
    outcome
        .notes
        .push("Reset all cached OTA states due to internal changes.".to_string());
}
