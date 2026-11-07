// Original: .upstream/zigbee2mqtt/lib/util/objectAssignDeep.ts
// TODO: port remaining non-JSON cases (undefined, functions, symbols).

use serde_json::{Map, Value};

fn is_record(value: &Value) -> bool {
    matches!(value, Value::Object(_))
}

fn forbidden(key: &str) -> bool {
    matches!(key, "__proto__" | "constructor" | "prototype")
}

fn clone_array(input: &[Value]) -> Value {
    Value::Array(
        input
            .iter()
            .map(|value| match value {
                Value::Object(_) => clone_object(value),
                Value::Array(items) => clone_array(items),
                other => other.clone(),
            })
            .collect(),
    )
}

fn clone_object(input: &Value) -> Value {
    let mut output = Map::new();
    if let Value::Object(map) = input {
        for (key, value) in map {
            if forbidden(key) {
                continue;
            }
            output.insert(
                key.clone(),
                match value {
                    Value::Object(_) => clone_object(value),
                    Value::Array(items) => clone_array(items),
                    other => other.clone(),
                },
            );
        }
    }
    Value::Object(output)
}

fn merge_into(target: &mut Map<String, Value>, source: &Map<String, Value>) {
    for (key, value) in source {
        if forbidden(key) {
            continue;
        }

        if is_record(value) {
            let mut merged = match target.get(key) {
                Some(Value::Object(existing)) => existing.clone(),
                _ => Map::new(),
            };
            if let Value::Object(source_object) = value {
                merge_into(&mut merged, source_object);
            }
            target.insert(key.clone(), Value::Object(merged));
        } else if let Value::Array(items) = value {
            target.insert(key.clone(), clone_array(items));
        } else {
            target.insert(key.clone(), value.clone());
        }
    }
}

/// Merge all `sources` into `target` recursively.
///
/// Upstream signature is `objectAssignDeep(target, ...sources)`. Rust has no
/// variadic arguments, so sources are passed as a slice. `target` is mutated
/// and returned.
pub fn object_assign_deep<'a>(target: &'a mut Value, sources: &[Value]) -> &'a mut Value {
    for source in sources {
        let Value::Object(source_object) = source else {
            continue;
        };
        if !target.is_object() {
            *target = Value::Object(Map::new());
        }
        if let Value::Object(target_object) = target {
            merge_into(target_object, source_object);
        }
    }
    target
}
