// Original: .upstream/zigbee2mqtt/lib/util/stringify.ts

use serde_json::{Map, Value};

/// Upstream `sort(array)` — object keys are serialized in ascending order.
fn sorted(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut keys = object.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            let mut output = Map::new();
            for key in keys {
                if let Some(entry) = object.get(&key) {
                    output.insert(key, sorted(entry));
                }
            }
            Value::Object(output)
        }
        Value::Array(items) => Value::Array(items.iter().map(sorted).collect()),
        other => other.clone(),
    }
}

pub fn stringify(value: &Value) -> String {
    serde_json::to_string(&sorted(value)).unwrap_or_else(|_| "null".to_string())
}
