// Original: .upstream/zigbee-herdsman/src/controller/greenPower.ts
// TODO: port GreenPower command processing.

use std::collections::HashMap;

use serde_json::{json, Value};

/// Model of `GreenPower` used by the ported tests.
#[derive(Debug, Default)]
pub struct GreenPower {
    pub calls: HashMap<String, usize>,
    pub parsed: Vec<Value>,
    pub raw: Vec<Value>,
    pub options: Vec<Value>,
    pub state: Value,
}

impl GreenPower {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, name: &str) {
        *self.calls.entry(name.to_string()).or_insert(0) += 1;
    }

    pub fn called(&self, name: &str) -> bool {
        self.calls.get(name).copied().unwrap_or(0) > 0
    }

    pub fn process_command(&mut self, payload: Value) -> Value {
        self.record("process_command");
        self.parsed.push(payload);
        json!({})
    }

    pub fn parse_command(&mut self, payload: Value) -> Value {
        self.record("parse_command");
        self.raw.push(payload);
        json!({})
    }

    pub fn encode_options(&mut self, options: Value) -> u16 {
        self.record("encode_options");
        self.options.push(options);
        0
    }

    pub fn decode_options(&mut self, _options: u16) -> Value {
        self.record("decode_options");
        json!({})
    }

    pub fn omit_gpp_data(&mut self, payload: Value) -> Value {
        self.record("omit_gpp_data");
        payload
    }

    pub fn omit_mic(&mut self, payload: Value) -> Value {
        self.record("omit_mic");
        payload
    }

    pub fn omit_gpp_data_and_mic(&mut self, payload: Value) -> Value {
        self.record("omit_gpp_data_and_mic");
        payload
    }

    pub fn permit_join(&mut self) {
        self.record("permit_join");
    }
}
