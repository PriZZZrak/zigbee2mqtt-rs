// Original: .upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters/dist/lib/utils.js
// Upstream revision: zigbee-herdsman-converters 26.125.0 (zigbee2mqtt 2.14.3).

use serde_json::{json, Map, Value};

use zigbee_converters::{access, Definition, Expose};
use z2m_rs::util::process::post_process;

#[test]
fn applies_calibration_offset_and_precision() {
    let mut payload = Map::from_iter([("temperature".to_string(), json!(21.234))]);
    let definition = Definition::new("TEST", "Acme", "Test")
        .with_exposes(vec![Expose::numeric("temperature", access::STATE)]);
    let errors = post_process(
        &definition,
        &mut payload,
        &json!({"temperature_calibration": 1, "temperature_precision": 1}),
    );
    assert!(errors.is_empty());
    assert_eq!(payload["temperature"], json!(22.2));
}

#[test]
fn default_precision_is_applied() {
    let mut payload = Map::from_iter([("temperature".to_string(), json!(21.234))]);
    let definition = Definition::new("TEST", "Acme", "Test")
        .with_exposes(vec![Expose::numeric("temperature", access::STATE)]);
    let errors = post_process(&definition, &mut payload, &json!({}));
    assert!(errors.is_empty());
    assert_eq!(payload["temperature"], json!(21.23));
}

#[test]
fn percentual_calibration_for_power() {
    let mut payload = Map::from_iter([("power".to_string(), json!(100.0))]);
    let definition = Definition::new("TEST", "Acme", "Test")
        .with_exposes(vec![Expose::numeric("power", access::STATE)]);
    let errors = post_process(&definition, &mut payload, &json!({"power_calibration": 10}));
    assert!(errors.is_empty());
    assert_eq!(payload["power"], json!(110.0));
}

#[test]
fn invalid_calibration_reports_error() {
    let mut payload = Map::from_iter([("temperature".to_string(), json!(20.0))]);
    let definition = Definition::new("TEST", "Acme", "Test")
        .with_exposes(vec![Expose::numeric("temperature", access::STATE)]);
    let errors = post_process(
        &definition,
        &mut payload,
        &json!({"temperature_calibration": "abc"}),
    );
    assert_eq!(errors.len(), 1);
    assert!(errors[0].starts_with("Failed to apply calibration to 'temperature'"));
}

#[test]
fn ignores_non_numeric_and_unknown_properties() {
    let mut payload = Map::from_iter([
        ("temperature".to_string(), Value::String("21".to_string())),
        ("custom".to_string(), json!(5)),
    ]);
    let definition = Definition::new("TEST", "Acme", "Test")
        .with_exposes(vec![Expose::numeric("temperature", access::STATE)]);
    let errors = post_process(&definition, &mut payload, &json!({}));
    assert!(errors.is_empty());
    assert_eq!(payload["temperature"], json!("21"));
    assert_eq!(payload["custom"], json!(5));
}
