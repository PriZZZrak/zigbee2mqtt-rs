// Original: .upstream/zigbee-herdsman/src/adapter/zoh/adapter/zohAdapter.ts
// TODO: port the full Zigbee-on-Host adapter.

use std::collections::HashMap;

use serde_json::{json, Value};

/// Model of `ZoHAdapter` used by the ported tests.
#[derive(Debug)]
pub struct ZoHAdapter {
    pub calls: HashMap<String, usize>,
    pub started: bool,
    pub stopped: bool,
    pub network: Value,
    pub stack_config: Value,
    pub events: Vec<Value>,
    pub install_codes: Vec<Value>,
    pub authorized: Vec<Value>,
    pub state: Value,
    pub errors: Vec<String>,
    pub result: Option<String>,
}

impl Default for ZoHAdapter {
    fn default() -> Self {
        Self {
            calls: HashMap::new(),
            started: false,
            stopped: false,
            network: json!({}),
            stack_config: json!({}),
            events: Vec::new(),
            install_codes: Vec::new(),
            authorized: Vec::new(),
            state: json!({}),
            errors: Vec::new(),
            result: None,
        }
    }
}

impl ZoHAdapter {
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

    pub fn get_state(&mut self) -> Value {
        self.record("get_state");
        self.state.clone()
    }
    pub fn permit_join(&mut self, _duration: Option<u32>, _device: Option<&str>) {
        self.record("permit_join");
    }
    pub fn send_zcl_frame_to_all(&mut self) {
        self.record("send_zcl_frame_to_all");
    }
    pub fn send_zcl_frame_to_endpoint(&mut self) {
        self.record("send_zcl_frame_to_endpoint");
    }
    pub fn send_zcl_frame_to_group(&mut self) {
        self.record("send_zcl_frame_to_group");
    }
    pub fn send_zdo(&mut self, _coordinator: bool) {
        self.record("send_zdo");
    }
    pub fn add_install_code(&mut self, _hashed: bool) {
        self.record("add_install_code");
        self.install_codes.push(json!({"hashed": _hashed}));
    }
    pub fn device_authorized(&mut self, device: Value) {
        self.record("device_authorized");
        self.authorized.push(device);
    }
    pub fn handle_gp_frame(&mut self) {
        self.record("handle_gp_frame");
    }
    pub fn handle_zcl_frame(&mut self) {
        self.record("handle_zcl_frame");
    }
    pub fn handle_zdo_frame(&mut self) {
        self.record("handle_zdo_frame");
    }
    pub fn handle_device_event(&mut self) {
        self.record("handle_device_event");
    }
    pub fn start(&mut self) -> Result<(), String> {
        self.record("start");
        self.started = true;
        Ok(())
    }
    pub fn stop(&mut self) {
        self.record("stop");
        self.stopped = true;
    }
    pub fn reset(&mut self) {
        self.record("reset");
    }
    pub fn load_stack_config(&mut self, config: Value) {
        self.record("load_stack_config");
        self.stack_config = config;
    }
    pub fn log(&mut self, level: &str, message: &str) {
        self.errors.push(format!("{level}: {message}"));
    }
}
