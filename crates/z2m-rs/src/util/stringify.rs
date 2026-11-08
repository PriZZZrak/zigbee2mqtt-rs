// Original: .upstream/zigbee2mqtt/lib/util/stringify.ts

pub fn stringify(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}
