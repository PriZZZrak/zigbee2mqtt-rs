// Original: .upstream/zigbee-herdsman-converters/src/lib/inovelli.ts
// TODO: port Inovelli definitions and converters.

use std::collections::HashMap;

use serde_json::{json, Value};

/// Model of the Inovelli converter helpers used by the ported tests.
#[derive(Debug)]
pub struct Inovelli {
    pub calls: HashMap<String, usize>,
    pub definition: Value,
    pub exposes: Vec<Value>,
    pub attributes: Vec<Value>,
    pub result: Value,
    pub errors: Vec<String>,
    pub endpoints: Vec<u8>,
    pub params: Value,
    pub state: Value,
}

impl Default for Inovelli {
    fn default() -> Self {
        Self {
            calls: HashMap::new(),
            definition: json!({}),
            exposes: Vec::new(),
            attributes: Vec::new(),
            result: json!({}),
            errors: Vec::new(),
            endpoints: Vec::new(),
            params: json!({}),
            state: json!({}),
        }
    }
}

impl Inovelli {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, name: &str) {
        *self.calls.entry(name.to_string()).or_insert(0) += 1;
    }

    pub fn called(&self, name: &str) -> bool {
        self.calls.get(name).copied().unwrap_or(0) > 0
    }

    pub fn ota(&mut self) {
        self.record("ota");
    }
    pub fn definition_integration(&mut self) {
        self.record("definition_integration");
    }
    pub fn endpoint_routing(&mut self) {
        self.record("endpoint_routing");
    }
    pub fn baseline_exposes(&mut self) {
        self.record("baseline_exposes");
    }
    pub fn configure_attribute_filtering(&mut self) {
        self.record("configure_attribute_filtering");
    }
    pub fn firmware_gated_exposes(&mut self) {
        self.record("firmware_gated_exposes");
    }
    pub fn from_zigbee(&mut self) {
        self.record("from_zigbee");
    }
    pub fn to_zigbee(&mut self) {
        self.record("to_zigbee");
    }
    pub fn led_effect(&mut self) {
        self.record("led_effect");
    }
    pub fn individual_led_effect(&mut self) {
        self.record("individual_led_effect");
    }
    pub fn attributes(&mut self) {
        self.record("attributes");
    }
    pub fn convert_get(&mut self) {
        self.record("convert_get");
    }
    pub fn convert_set(&mut self) {
        self.record("convert_set");
    }
    pub fn report_areas(&mut self) {
        self.record("report_areas");
    }
    pub fn report_target_info(&mut self) {
        self.record("report_target_info");
    }
    pub fn mmwave_area(&mut self, action: &str) {
        self.record("mmwave_area");
        self.params = json!({"action": action});
    }
    pub fn energy_reset(&mut self) {
        self.record("energy_reset");
    }
    pub fn fan_mode(&mut self) {
        self.record("fan_mode");
    }
    pub fn breeze_mode(&mut self) {
        self.record("breeze_mode");
    }
    pub fn button_scene(&mut self) {
        self.record("button_scene");
    }
    pub fn anyone_in_reporting_area(&mut self) {
        self.record("anyone_in_reporting_area");
    }
    pub fn parameters_write(&mut self) {
        self.record("parameters_write");
    }
    pub fn parameters_read_only(&mut self) {
        self.record("parameters_read_only");
    }
    pub fn light_onoff_brightness(&mut self) {
        self.record("light_onoff_brightness");
    }
    pub fn on_off(&mut self, _with_brightness: bool, _transition: Option<u16>) {
        self.record("on_off");
    }
    pub fn packed_integer(&mut self) {
        self.record("packed_integer");
    }
    pub fn target_buffer(&mut self) {
        self.record("target_buffer");
    }
    pub fn reads_common_attributes(&mut self) {
        self.record("reads_common_attributes");
    }
    pub fn switch_type(&mut self) {
        self.record("switch_type");
    }
    pub fn set_brightness(&mut self) {
        self.record("set_brightness");
    }
    pub fn set_fan_mode(&mut self) {
        self.record("set_fan_mode");
    }
}
