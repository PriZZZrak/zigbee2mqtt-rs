// Original: .upstream/zigbee-herdsman/src/utils/utils.ts

use serde_json::Value;

pub fn assert_string(value: &Value, _property: &str) {
    if !value.is_string() {
        panic!("Input must be a string!");
    }
}

pub fn is_number_array(value: &Value) -> bool {
    matches!(value, Value::Array(items) if items.iter().all(Value::is_number))
}

pub fn is_number_array_of_length(value: &Value, length: usize) -> bool {
    is_number_array(value) && value.as_array().is_some_and(|items| items.len() == length)
}

pub fn is_object_empty(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.is_empty(),
        _ => false,
    }
}
