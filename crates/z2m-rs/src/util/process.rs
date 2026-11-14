// Original: .upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters/dist/index.js
// Upstream revision: zigbee-herdsman-converters 26.125.0 (zigbee2mqtt 2.14.3).

use serde_json::{Map, Value};

/// Upstream `postProcessConvertedFromZigbeeMessage` (device resolved from the
/// static model cache because this wrapper has no device handle).
pub fn post_process(
    definition: &zigbee_converters::Definition,
    payload: &mut Map<String, Value>,
    options: &Value,
) -> Vec<String> {
    zigbee_converters::utils::post_process_converted_from_zigbee_message(
        definition,
        payload,
        options,
        &Value::Null,
    )
}
