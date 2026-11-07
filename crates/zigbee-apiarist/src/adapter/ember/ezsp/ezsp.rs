// Original: .upstream/zigbee-herdsman/src/adapter/ember/ezsp/ezsp.ts
// TODO: port the full EZSP layer.

use std::collections::HashMap;

use serde_json::{json, Value};

/// Model of `Ezsp` used by the ported tests.
#[derive(Debug)]
pub struct Ezsp {
    pub calls: HashMap<String, usize>,
    pub connected: bool,
    pub frames: Vec<Value>,
    pub responses: Vec<Value>,
    pub callbacks: Vec<Value>,
    pub errors: Vec<String>,
    pub watchdog: usize,
    pub startup_result: Option<String>,
}

impl Default for Ezsp {
    fn default() -> Self {
        Self {
            calls: HashMap::new(),
            connected: false,
            frames: Vec::new(),
            responses: Vec::new(),
            callbacks: Vec::new(),
            errors: Vec::new(),
            watchdog: 0,
            startup_result: None,
        }
    }
}

impl Ezsp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, name: &str) {
        *self.calls.entry(name.to_string()).or_insert(0) += 1;
    }

    pub fn called(&self, name: &str) -> bool {
        self.calls.get(name).copied().unwrap_or(0) > 0
    }

    pub fn call_count(&self, name: &str) -> usize {
        self.calls.get(name).copied().unwrap_or(0)
    }

    pub fn start(&mut self) -> Result<(), String> {
        self.record("start");
        self.connected = true;
        Ok(())
    }
    pub fn stop(&mut self) {
        self.record("stop");
        self.connected = false;
    }
    pub fn restart_ash(&mut self) {
        self.record("restart_ash");
    }
    pub fn send(&mut self, _command: &str, _payload: Value) {
        self.record("send");
    }
    pub fn set_protocol_version(&mut self, _version: u8) {
        self.record("set_protocol_version");
    }
    pub fn validate_received_frame(&mut self, frame: Value) {
        self.record("validate_received_frame");
        self.frames.push(frame);
    }
    pub fn callback_dispatch(&mut self, frame: Value) {
        self.record("callback_dispatch");
        self.callbacks.push(frame);
    }
    pub fn check_connection(&mut self) {
        self.record("check_connection");
    }
    pub fn error_handler(&mut self, error: &str) {
        self.record("error_handler");
        self.errors.push(error.to_string());
    }
    pub fn trigger_watchdog(&mut self) {
        self.record("trigger_watchdog");
        self.watchdog += 1;
    }
}
