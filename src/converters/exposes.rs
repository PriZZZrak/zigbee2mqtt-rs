use serde_json::{json, Value};

pub fn light_brightness() -> Value {
    json!({"type":"light","features":[{"name":"state","type":"binary"},{"name":"brightness","type":"numeric"}]})
}
pub fn temperature() -> Value {
    json!({"type":"numeric","name":"temperature","unit":"°C"})
}
pub fn humidity() -> Value {
    json!({"type":"numeric","name":"humidity","unit":"%"})
}
pub fn battery() -> Value {
    json!({"type":"numeric","name":"battery","unit":"%"})
}
