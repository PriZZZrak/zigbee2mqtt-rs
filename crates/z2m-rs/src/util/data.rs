// Original: .upstream/zigbee2mqtt/lib/util/data.ts

use std::path::PathBuf;

/// Resolve the data directory.
pub fn get_path() -> PathBuf {
    match std::env::var_os("ZIGBEE2MQTT_DATA") {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        _ => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data"),
    }
}

/// Resolve `file` against the data directory.
pub fn join_path(file: &str) -> PathBuf {
    let path = PathBuf::from(file);
    if path.is_absolute() {
        path
    } else if file.is_empty() {
        get_path()
    } else {
        get_path().join(path)
    }
}

/// Re-read `ZIGBEE2MQTT_DATA` (test hook; resolution is already dynamic).
pub fn _test_reload() {}
