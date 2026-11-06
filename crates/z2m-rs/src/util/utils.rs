#![allow(unused)]
// Original: .upstream/zigbee2mqtt/lib/util/utils.ts
// TODO: port utils.

use std::path::Path;

pub fn object_is_empty(value: &serde_json::Value) -> bool {
    todo!()
}

pub fn object_has_properties(value: &serde_json::Value, keys: &[&str]) -> bool {
    todo!()
}

pub async fn get_zigbee2mqtt_version() -> serde_json::Value {
    todo!()
}

pub async fn get_dependency_version(name: &str) -> serde_json::Value {
    todo!()
}

pub fn format_date(time: i64, kind: &str) -> String {
    todo!()
}

pub fn assert_string(value: &serde_json::Value, property: &str) {
    todo!()
}

pub fn remove_null_properties_from_object(value: &mut serde_json::Value, keep: &[&str]) {
    todo!()
}
