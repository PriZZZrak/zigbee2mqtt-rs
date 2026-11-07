// Original: .upstream/zigbee-herdsman-converters/src/lib/generateDefinition.ts
// TODO: port definition generation.

use std::collections::HashMap;

use serde_json::{json, Value};

/// Model of `generateDefinition` used by the ported tests.
#[derive(Debug, Default)]
pub struct GenerateDefinition {
    pub calls: HashMap<String, usize>,
    pub last_result: Value,
    pub errors: Vec<String>,
}

impl GenerateDefinition {
    pub fn new() -> Self {
        Self {
            last_result: json!({}),
            ..Self::default()
        }
    }

    pub fn record(&mut self, name: &str) {
        *self.calls.entry(name.to_string()).or_insert(0) += 1;
    }

    pub fn called(&self, name: &str) -> bool {
        self.calls.get(name).copied().unwrap_or(0) > 0
    }

    pub fn generate_definition(&mut self, exposes: Value) -> Value {
        self.record("generate_definition");
        self.last_result = json!({"exposes": exposes});
        self.last_result.clone()
    }

    pub fn generate_green_power_definition(&mut self, exposes: Value) -> Value {
        self.record("generate_green_power_definition");
        self.last_result = json!({"exposes": exposes});
        self.last_result.clone()
    }
}
