#![allow(unused)]
// Original: .upstream/zigbee2mqtt/lib/util/data.ts
// TODO: port data path resolution.

use std::path::PathBuf;

/// Resolve the data directory.
pub fn get_path() -> PathBuf {
    todo!()
}

/// Resolve `file` against the data directory.
pub fn join_path(file: &str) -> PathBuf {
    todo!()
}

/// Re-read `ZIGBEE2MQTT_DATA` (test hook).
pub fn _test_reload() {
    todo!()
}
