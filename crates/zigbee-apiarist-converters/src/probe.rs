// Shared scaffolding for ported converter tests.
// TODO: replace with real converter invocations as implementations land.

use std::collections::HashSet;

use serde_json::{json, Value};

/// Minimal recorded-invocation probe used by the ported converter tests.
#[derive(Debug)]
pub struct ConverterProbe {
    pub calls: HashSet<String>,
    pub input: Value,
    pub output: Value,
    pub errors: Vec<String>,
}

impl Default for ConverterProbe {
    fn default() -> Self {
        Self {
            calls: HashSet::new(),
            input: json!({}),
            output: json!({}),
            errors: Vec::new(),
        }
    }
}

impl ConverterProbe {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(&mut self, name: &str) {
        self.calls.insert(name.to_string());
    }

    pub fn called(&self, name: &str) -> bool {
        self.calls.contains(name)
    }

    pub fn set_input(&mut self, input: Value) {
        self.input = input;
    }

    pub fn output(&self) -> &Value {
        &self.output
    }
}
