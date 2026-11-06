//! Persistent entity state cache (`state.json`).
//!
//! Upstream `lib/state.ts` restores cached entity state on startup and
//! republishes it when `advanced.cache_state_send_on_startup` is set, so a
//! sleepy device (e.g. a Tuya sensor reporting temperature every 30-60 min)
//! does not appear empty after a restart or a migration from the original
//! Zigbee2MQTT.
//!
//! Keys are IEEE addresses (`0x…`), values are the last published state
//! objects.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Map, Value};

use crate::devices::DeviceRegistry;

/// Load `state.json`. A missing/corrupt file yields `None` rather than
/// aborting startup; the bridge then starts with empty state and fills it from
/// device reports.
#[must_use]
pub fn load(path: &Path) -> Option<BTreeMap<String, Map<String, Value>>> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Snapshot every device's current state keyed by IEEE address.
#[must_use]
pub fn snapshot(devices: &DeviceRegistry) -> Value {
    let mut out = Map::new();
    for device in devices.all_devices() {
        out.insert(
            device.ieee_addr.as_hex(),
            Value::Object(device.state.clone()),
        );
    }
    Value::Object(out)
}

/// Write the cache atomically (`state.json.tmp` + rename).
pub fn save(path: &Path, value: &Value) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec(value)?)?;
    std::fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_round_trip() {
        let dir = std::env::temp_dir().join(format!(
            "z2m-state-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        let value = serde_json::json!({
            "0x00124b0024c2d400": {
                "temperature": 26.4,
                "humidity": 45,
                "battery": 45,
                "last_seen": "2026-11-05T08:16:47.701Z"
            }
        });
        save(&path, &value).unwrap();
        let loaded = load(&path).expect("state file");
        assert_eq!(loaded.len(), 1);
        assert_eq!(
            loaded["0x00124b0024c2d400"]["temperature"],
            serde_json::json!(26.4)
        );
        assert_eq!(
            loaded["0x00124b0024c2d400"]["humidity"],
            serde_json::json!(45)
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_state_file_is_not_fatal() {
        assert!(load(Path::new("/nonexistent/z2m/state.json")).is_none());
    }
}
