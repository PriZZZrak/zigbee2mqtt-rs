use super::exposes;
use serde_json::{json, Value};

pub fn on_off() -> (Vec<String>, Vec<String>, Vec<Value>) {
    (
        vec!["fz.on_off".into()],
        vec!["tz.on_off".into()],
        vec![json!({"type":"switch"})],
    )
}
pub fn light_brightness() -> (Vec<String>, Vec<String>, Vec<Value>) {
    (
        vec!["fz.on_off".into(), "fz.brightness".into()],
        vec!["tz.light_onoff_brightness".into()],
        vec![exposes::light_brightness()],
    )
}
pub fn battery() -> (Vec<String>, Vec<String>, Vec<Value>) {
    (vec!["fz.battery".into()], vec![], vec![exposes::battery()])
}
