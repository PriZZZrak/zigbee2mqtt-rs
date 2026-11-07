// Original: .upstream/zigbee2mqtt/lib/util/stringify.ts
// TODO: port edge cases (BigInt, Buffer, circular refs, toJSON).

pub fn stringify(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}
