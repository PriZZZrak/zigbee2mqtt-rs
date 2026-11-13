// Original: .upstream/zigbee2mqtt/lib/util/utils.ts

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use base64::Engine as _;
use serde_json::Value;

/// Upstream `DEFAULT_BIND_GROUP_ID` (`utils.ts:15`).
pub const DEFAULT_BIND_GROUP_ID: u16 = 901;

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

/// Upstream `filterProperties(filter, data)` (`utils.ts:318-326`).
pub fn filter_properties(filter: Option<&[String]>, data: &mut Value) {
    let Some(filter) = filter else {
        return;
    };
    let Value::Object(map) = data else {
        return;
    };

    map.retain(|key, _| !filter.iter().any(|pattern| matches_pattern(pattern, key)));
}

fn matches_pattern(pattern: &str, text: &str) -> bool {
    fn compiled(pattern: &str) -> regex::Regex {
        regex::Regex::new(&format!("^{pattern}$"))
            .unwrap_or_else(|error| panic!("invalid filterProperties pattern '{pattern}': {error}"))
    }

    static CACHE: OnceLock<Mutex<std::collections::HashMap<String, regex::Regex>>> =
        OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    let mut cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    cache
        .entry(pattern.to_string())
        .or_insert_with(|| compiled(pattern))
        .is_match(text)
}

/// Upstream `isDebouncedOrThrottled(device)` (`utils.ts:293`).
pub fn is_debounced_or_throttled(entity: &dyn crate::model::entity::Entity) -> bool {
    let options = entity.options();
    let truthy = |key: &str| match options.get(key) {
        Some(Value::Bool(value)) => *value,
        Some(Value::Number(value)) => value.as_f64().is_some_and(|value| value != 0.0),
        Some(Value::String(value)) => !value.is_empty(),
        Some(Value::Array(items)) => !items.is_empty(),
        Some(Value::Object(map)) => !map.is_empty(),
        _ => false,
    };
    truthy("debounce") || truthy("throttle")
}

/// Upstream `publishLastSeen(data, settings, allowMessageEmitted, publishEntityState)`
/// (`utils.ts:295-316`).
pub async fn publish_last_seen(
    data: &Value,
    device: std::sync::Arc<dyn crate::model::entity::Entity>,
    settings: &Value,
    allow_message_emitted: bool,
    publish: &crate::model::entity::PublishEntityState,
) {
    let reason = data.get("reason").and_then(Value::as_str).unwrap_or("");
    let last_seen = settings
        .get("advanced")
        .and_then(|advanced| advanced.get("last_seen"))
        .and_then(Value::as_str)
        .unwrap_or("disable");

    let allow = (reason != "messageEmitted"
        || (reason == "messageEmitted" && allow_message_emitted))
        && !(reason == "messageNonEmitted" && is_debounced_or_throttled(device.as_ref()));

    if !last_seen.is_empty() && last_seen != "disable" && allow {
        publish(
            device,
            serde_json::json!({}),
            Some("lastSeenChanged".to_string()),
        )
        .await;
    }
}

/// Upstream `parseJSON(json, fallback)` (`utils.ts`).
pub fn parse_json(text: &str, fallback: &str) -> Value {
    serde_json::from_str(text).unwrap_or_else(|_| Value::String(fallback.to_string()))
}

/// Upstream `getResponse(request, data, error?)` (`utils.ts:117-145`).
pub fn get_response(request: &Value, data: Value, error: Option<String>) -> Value {
    match error {
        Some(error) => {
            let mut response = serde_json::json!({
                "data": {},
                "status": "error",
                "error": error,
            });
            if request.get("transaction").is_some() {
                response["transaction"] = request["transaction"].clone();
            }
            response
        }
        None => {
            let mut response = serde_json::json!({
                "data": data,
                "status": "ok",
            });
            if request.get("transaction").is_some() {
                response["transaction"] = request["transaction"].clone();
            }
            response
        }
    }
}

/// Upstream `seconds/minutes/hours` (`utils.ts:287-289`).
pub fn seconds(value: u64) -> u64 {
    value.saturating_mul(1000)
}
pub fn minutes(value: u64) -> u64 {
    seconds(value).saturating_mul(60)
}
pub fn hours(value: u64) -> u64 {
    minutes(value).saturating_mul(60)
}

/// Upstream `utils.sleep(seconds)`.
pub async fn sleep(seconds: u64) {
    tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;
}

/// Upstream `deviceNotCoordinator(device)` (`utils.ts:381-383`).
pub fn device_not_coordinator(
    device: &std::sync::Arc<zigbee_apiarist::controller::model::device::Device>,
) -> bool {
    device.r#type() != zigbee_apiarist::controller::database::DeviceType::Coordinator
}

/// Upstream `isAvailabilityEnabledForEntity(entity, settings)` (`utils.ts:249-269`).
pub fn is_availability_enabled_for_entity(
    entity: &dyn crate::model::entity::Entity,
    settings: &Value,
) -> bool {
    if entity.is_device()
        && entity
            .options()
            .get("disabled")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        return false;
    }

    if entity.is_group() {
        return entity
            .members_devices()
            .iter()
            .all(|member| is_availability_enabled_for_entity(member.as_ref(), settings));
    }

    if let Some(availability) = entity.options().get("availability") {
        if !availability.is_null() {
            return match availability {
                Value::Bool(value) => *value,
                Value::Object(_) => true,
                _ => false,
            };
        }
    }

    settings
        .get("availability")
        .and_then(|availability| availability.get("enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// Deterministic-enough `Math.random()` replacement for jitter.
pub fn random_f64() -> f64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos())
        .unwrap_or(0);
    (nanos % 1_000_000) as f64 / 1_000_000.0
}

/// Upstream `capitalize(s)` (`utils.ts:33`).
pub fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Upstream `getScenes(endpoint|group)` (`utils.ts:362-379`).
fn scenes_from_endpoints(
    endpoints: &[std::sync::Arc<zigbee_apiarist::controller::model::endpoint::Endpoint>],
    group_id: u64,
) -> Vec<Value> {
    let mut scenes: std::collections::BTreeMap<u64, Value> = std::collections::BTreeMap::new();
    for endpoint in endpoints {
        let meta = endpoint.meta.lock().unwrap();
        let Some(Value::Object(table)) = meta.get("scenes") else {
            continue;
        };
        for (key, data) in table {
            let mut parts = key.split('_');
            let scene_id = parts
                .next()
                .and_then(|part| part.parse::<u64>().ok())
                .unwrap_or(0);
            let scene_group = parts
                .next()
                .and_then(|part| part.parse::<u64>().ok())
                .unwrap_or(0);
            if scene_group != group_id {
                continue;
            }
            let name = data
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| format!("Scene {scene_id}"));
            scenes.insert(scene_id, serde_json::json!({"id": scene_id, "name": name}));
        }
    }
    scenes.into_values().collect()
}

pub fn get_scenes_endpoint(
    endpoint: &std::sync::Arc<zigbee_apiarist::controller::model::endpoint::Endpoint>,
) -> Vec<Value> {
    scenes_from_endpoints(std::slice::from_ref(endpoint), 0)
}

pub fn get_scenes_group(
    group: &std::sync::Arc<zigbee_apiarist::controller::model::group::Group>,
) -> Vec<Value> {
    scenes_from_endpoints(&group.members(), group.group_id as u64)
}

/// Upstream `getAllFiles(path)` (`utils.ts:203-218`).
pub fn get_all_files(path: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut result = Vec::new();
    let Ok(entries) = std::fs::read_dir(path) else {
        return result;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let file_path = entry.path();
        if file_type.is_file() {
            result.push(file_path);
        } else {
            result.extend(get_all_files(&file_path));
        }
    }
    result
}

/// Upstream `sanitizeImageParameter` (`utils.ts`).
pub fn sanitize_image_parameter(parameter: &str) -> String {
    parameter
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric()
                || matches!(character, '-' | '_' | ' ' | '.' | '/' | ':')
            {
                character
            } else {
                '-'
            }
        })
        .collect()
}

/// Upstream `matchBase64File(value)` (`utils.ts:385-393`).
pub fn match_base64_file(value: &str) -> Option<(String, String)> {
    let rest = value.strip_prefix("data:image/")?;
    let (extension, data) = rest.split_once(";base64,")?;
    if extension.is_empty() || data.is_empty() {
        return None;
    }
    Some((extension.to_string(), data.to_string()))
}

/// Upstream `saveBase64DeviceIcon(match)` (`utils.ts:396-403`).
pub fn save_base64_device_icon(extension: &str, data: &str) -> Result<String, String> {
    use md5::{Digest, Md5};

    let digest = Md5::digest(data.as_bytes());
    let hash: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    let file_settings = format!("device_icons/{hash}.{extension}");
    let path = crate::util::data::join_path(&file_settings);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|error| error.to_string())?;
    std::fs::write(path, bytes).map_err(|error| error.to_string())?;
    Ok(file_settings)
}
