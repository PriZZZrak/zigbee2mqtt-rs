// Original: .upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters/dist/lib/utils.js
// Upstream revision: zigbee-herdsman-converters 26.125.0 (zigbee2mqtt 2.14.3).

use serde_json::{Map, Value};

use crate::definition::Definition;

/// Upstream `calibrateAndPrecisionRoundOptionsDefaultPrecision`.
pub const DEFAULT_PRECISION: [(&str, u32); 24] = [
    ("ac_frequency", 2),
    ("temperature", 2),
    ("humidity", 2),
    ("pressure", 1),
    ("pm25", 0),
    ("power", 2),
    ("current", 2),
    ("current_phase_b", 2),
    ("current_phase_c", 2),
    ("current_neutral", 2),
    ("voltage", 2),
    ("voltage_phase_b", 2),
    ("voltage_phase_c", 2),
    ("power_phase_b", 2),
    ("power_phase_c", 2),
    ("energy", 2),
    ("device_temperature", 0),
    ("soil_moisture", 2),
    ("co2", 0),
    ("illuminance", 0),
    ("voc", 0),
    ("formaldehyd", 0),
    ("co", 0),
    ("voltage_phase_a", 2),
];

fn is_percentual(name: &str) -> bool {
    name.starts_with("current")
        || name.starts_with("energy")
        || name.starts_with("voltage")
        || name.starts_with("power")
        || name.starts_with("illuminance")
}

fn to_number(value: &Value, key: &str) -> Result<f64, String> {
    match value {
        Value::Number(number) => number
            .as_f64()
            .ok_or_else(|| format!("'{key}' is not a number")),
        Value::String(text) if !text.is_empty() => text
            .parse::<f64>()
            .map_err(|_| format!("'{key}' is not a number")),
        _ => Ok(0.0),
    }
}

fn precision_round(number: f64, precision: u32) -> f64 {
    let factor = 10f64.powi(precision as i32);
    (number * factor).round() / factor
}

/// Upstream `calibrateAndPrecisionRoundOptions(number, options, type)`.
pub fn calibrate_and_precision_round_options(
    number: f64,
    options: &Value,
    name: &str,
) -> Result<f64, String> {
    let calibration_key = format!("{name}_calibration");
    let calibration_value = options.get(&calibration_key).filter(|value| !value.is_null());
    let mut offset = to_number(
        calibration_value.unwrap_or(&Value::from(0)),
        &calibration_key,
    )?;
    if is_percentual(name) {
        offset = (number * offset) / 100.0;
    }
    let value = number + offset;

    let precision_key = format!("{name}_precision");
    let precision_value = options.get(&precision_key).filter(|value| !value.is_null());
    let default = DEFAULT_PRECISION
        .iter()
        .find(|(entry, _)| *entry == name)
        .map(|(_, precision)| *precision)
        .unwrap_or(0);
    let precision = match precision_value {
        Some(value) => to_number(value, &precision_key)? as i64,
        None => default as i64,
    };
    Ok(precision_round(value, precision.max(0) as u32))
}

/// Upstream `postProcessConvertedFromZigbeeMessage`.
pub fn post_process_converted_from_zigbee_message(
    definition: &Definition,
    payload: &mut Map<String, Value>,
    options: &Value,
    device: &Value,
) -> Vec<String> {
    let exposes = definition.exposes.resolve(device, options);
    let mut errors = Vec::new();
    for (key, value) in payload.iter_mut() {
        let expose = exposes.iter().find(|expose| expose.property.as_deref() == Some(key));
        let Some(name) = expose.and_then(|expose| expose.name.clone()) else {
            continue;
        };
        if !DEFAULT_PRECISION.iter().any(|(entry, _)| *entry == name) || value == &Value::String(String::new()) {
            continue;
        }
        let Value::Number(number) = value else {
            continue;
        };
        let number = number.as_f64().unwrap_or(0.0);
        match calibrate_and_precision_round_options(number, options, &name) {
            Ok(calibrated) => *value = Value::from(calibrated),
            Err(error) => errors.push(format!("Failed to apply calibration to '{name}': {error}")),
        }
    }
    errors
}
