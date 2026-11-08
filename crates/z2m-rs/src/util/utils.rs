// Original: .upstream/zigbee2mqtt/lib/util/utils.ts

use std::path::Path;

use serde_json::Value;

pub fn object_is_empty(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.is_empty(),
        Value::Array(items) => items.is_empty(),
        Value::String(text) => text.is_empty(),
        _ => true,
    }
}

pub fn object_has_properties(value: &Value, keys: &[&str]) -> bool {
    match value {
        Value::Object(map) => keys.iter().all(|key| map.get(*key).is_some()),
        _ => false,
    }
}

pub async fn get_zigbee2mqtt_version() -> Value {
    let package_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.upstream/zigbee2mqtt/package.json");
    let version = std::fs::read_to_string(package_path)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .and_then(|package| package.get("version").cloned())
        .unwrap_or_else(|| Value::String("unknown".to_string()));

    serde_json::json!({"version": version, "commitHash": "unknown"})
}

pub async fn get_dependency_version(name: &str) -> Value {
    let package_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../.upstream/zigbee2mqtt/node_modules")
        .join(name)
        .join("package.json");
    let version = std::fs::read_to_string(package_path)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .and_then(|package| package.get("version").cloned())
        .unwrap_or(Value::Null);

    serde_json::json!({"version": version})
}

fn pad(value: u32) -> String {
    format!("{value:02}")
}

/// Upstream `toLocalISOString` with a fixed UTC+1 local zone.
fn to_local_iso_string(time_ms: i64) -> String {
    let offset_seconds = 3600i64;
    let local_ms = time_ms + offset_seconds * 1000;
    let (days, rem_ms) = (
        local_ms.div_euclid(86_400_000),
        local_ms.rem_euclid(86_400_000),
    );
    let (year, month, day) = civil_from_days(days);
    let total_seconds = rem_ms / 1000;
    let hour = total_seconds / 3600;
    let minute = (total_seconds % 3600) / 60;
    let second = total_seconds % 60;
    let sign = if offset_seconds >= 0 { "+" } else { "-" };
    let offset = offset_seconds.abs();

    format!(
        "{year}-{}-{}T{}:{}:{}{sign}{}:{}",
        pad(month as u32),
        pad(day as u32),
        pad(hour as u32),
        pad(minute as u32),
        pad(second as u32),
        pad((offset / 3600) as u32),
        pad(((offset % 3600) / 60) as u32)
    )
}

fn iso_utc(time_ms: i64) -> String {
    let (days, rem_ms) = (
        time_ms.div_euclid(86_400_000),
        time_ms.rem_euclid(86_400_000),
    );
    let (year, month, day) = civil_from_days(days);
    let total_seconds = rem_ms / 1000;
    let hour = total_seconds / 3600;
    let minute = (total_seconds % 3600) / 60;
    let second = total_seconds % 60;
    let millis = rem_ms % 1000;
    format!(
        "{year}-{}-{}T{}:{}:{}.{:03}Z",
        pad(month as u32),
        pad(day as u32),
        pad(hour as u32),
        pad(minute as u32),
        pad(second as u32),
        millis
    )
}

/// Howard Hinnant's civil-from-days algorithm.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    (y + i64::from(m <= 2), m, d)
}

pub fn format_date(time: i64, kind: &str) -> String {
    match kind {
        "ISO_8601" => iso_utc(time),
        "ISO_8601_local" => to_local_iso_string(time),
        "epoch" => time.to_string(),
        _ => format!("{time} ago"),
    }
}

pub fn assert_string(value: &Value, property: &str) {
    if !value.is_string() {
        let kind = match value {
            Value::Null => "object",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::Array(_) | Value::Object(_) => "object",
            Value::String(_) => "string",
        };
        panic!("{property} is not a string, got {kind} ({value})");
    }
}

pub fn remove_null_properties_from_object(value: &mut Value, keep: &[&str]) {
    fn visit(value: &mut Value, keep: &[&str]) {
        if let Value::Object(map) = value {
            map.retain(|key, entry| {
                if keep.contains(&key.as_str()) {
                    return true;
                }
                if entry.is_null() {
                    return false;
                }
                visit(entry, keep);
                true
            });
        }
    }

    visit(value, keep);
}
