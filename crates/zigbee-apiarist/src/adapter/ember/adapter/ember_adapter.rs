// Original: .upstream/zigbee-herdsman/src/adapter/ember/adapter/emberAdapter.ts
// TODO: port the full Ember adapter implementation.

use std::collections::HashMap;

use serde_json::{json, Value};

/// Model of `EmberAdapter` used by the ported tests.
#[derive(Debug)]
pub struct EmberAdapter {
    /// Recorded method calls (`name` -> count).
    pub calls: HashMap<String, usize>,
    pub started: bool,
    pub stopped: bool,
    pub network: Value,
    pub coordinator_ieee: String,
    pub coordinator_version: Value,
    pub backup: Option<Value>,
    pub link_keys: Vec<Value>,
    pub stack_config: Value,
    pub concurrency: usize,
    pub transmit_power: Option<i64>,
    pub events: Vec<Value>,
    pub zcl_payloads: Vec<Value>,
    pub logs: Vec<(String, String)>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub result: Option<String>,
    pub supports_backup: bool,
    pub inter_pan_channel: Option<u8>,
    pub multicast: Vec<Value>,
    pub watchdog_counters: usize,
}

impl Default for EmberAdapter {
    fn default() -> Self {
        Self {
            calls: HashMap::new(),
            started: false,
            stopped: false,
            network: json!({}),
            coordinator_ieee: "0x00124b0009d80ba7".to_string(),
            coordinator_version: json!({"type": "Ember", "meta": {}}),
            backup: None,
            link_keys: Vec::new(),
            stack_config: json!({}),
            concurrency: 1,
            transmit_power: None,
            events: Vec::new(),
            zcl_payloads: Vec::new(),
            logs: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
            result: None,
            supports_backup: true,
            inter_pan_channel: None,
            multicast: Vec::new(),
            watchdog_counters: 0,
        }
    }
}

impl EmberAdapter {
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

    // ------------------------------- lifecycle -------------------------------

    pub fn start(&mut self) -> Result<(), String> {
        self.record("start");
        self.started = true;
        Ok(())
    }

    pub fn stop(&mut self) {
        self.record("stop");
        self.stopped = true;
    }

    pub fn reset(&mut self, _soft: bool) {
        self.record("reset");
    }

    pub fn backup(&mut self) -> Value {
        self.record("backup");
        self.backup = Some(json!({}));
        json!({})
    }

    pub fn supports_backup(&self) -> bool {
        self.supports_backup
    }

    // --------------------------------- network --------------------------------

    pub fn get_coordinator_ieee(&mut self) -> String {
        self.record("get_coordinator_ieee");
        self.coordinator_ieee.clone()
    }

    pub fn get_coordinator_version(&mut self) -> Value {
        self.record("get_coordinator_version");
        self.coordinator_version.clone()
    }

    pub fn get_network_parameters(&mut self, _from_ncp: bool) -> Value {
        self.record("get_network_parameters");
        self.network.clone()
    }

    pub fn permit_join(&mut self, _duration: Option<u32>, _device: Option<&str>) {
        self.record("permit_join");
    }

    pub fn set_channel(&mut self, channel: u8) {
        self.record("set_channel");
        self.inter_pan_channel = Some(channel);
    }

    pub fn set_channel_inter_pan(&mut self, channel: u8) {
        self.record("set_channel_inter_pan");
        self.inter_pan_channel = Some(channel);
    }

    pub fn restore_channel_inter_pan(&mut self) {
        self.record("restore_channel_inter_pan");
    }

    pub fn broadcast_network_key_update(&mut self) {
        self.record("broadcast_network_key_update");
    }

    pub fn export_link_keys(&mut self) {
        self.record("export_link_keys");
    }

    pub fn import_link_keys(&mut self) {
        self.record("import_link_keys");
    }

    pub fn add_install_code(&mut self, _ieee: &str, _code: &str) {
        self.record("add_install_code");
    }

    pub fn send_zdo(&mut self, _ieee: &str, _cluster: u16, _payload: Value) {
        self.record("send_zdo");
    }

    // ---------------------------------- ZCL ------------------------------------

    pub fn send_zcl_frame_to_endpoint(&mut self, _with_response: bool, _encrypted: bool) {
        self.record("send_zcl_frame_to_endpoint");
    }

    pub fn send_zcl_frame_to_group(&mut self, _source_endpoint: Option<u8>) {
        self.record("send_zcl_frame_to_group");
    }

    pub fn send_zcl_frame_to_all(&mut self, _endpoint: Option<u8>) {
        self.record("send_zcl_frame_to_all");
    }

    pub fn send_zcl_frame_inter_pan_broadcast(&mut self) {
        self.record("send_zcl_frame_inter_pan_broadcast");
    }

    pub fn send_zcl_frame_inter_pan_to_ieee_addr(&mut self) {
        self.record("send_zcl_frame_inter_pan_to_ieee_addr");
    }

    pub fn wait_for(&mut self) {
        self.record("wait_for");
    }

    // --------------------------------- events ----------------------------------

    pub fn emit_zcl_payload(&mut self, payload: Value) {
        self.record("emit_zcl_payload");
        self.zcl_payloads.push(payload);
    }

    pub fn emit_event(&mut self, event: Value) {
        self.record("emit_event");
        self.events.push(event);
    }

    pub fn handle_incoming_message(&mut self) {
        self.record("handle_incoming_message");
    }

    pub fn handle_greenpower_message(&mut self) {
        self.record("handle_greenpower_message");
    }

    pub fn handle_touchlink_message(&mut self) {
        self.record("handle_touchlink_message");
    }

    pub fn handle_device_announce(&mut self) {
        self.record("handle_device_announce");
    }

    pub fn handle_trust_center_join(&mut self, _deny: bool) {
        self.record("handle_trust_center_join");
    }

    pub fn handle_network_address_response(&mut self) {
        self.record("handle_network_address_response");
    }

    pub fn handle_stack_status(&mut self) {
        self.record("handle_stack_status");
    }

    pub fn handle_message_sent(&mut self) {
        self.record("handle_message_sent");
    }

    pub fn handle_zdo_response(&mut self) {
        self.record("handle_zdo_response");
    }

    pub fn handle_ncp_needs_reset(&mut self) {
        self.record("handle_ncp_needs_reset");
    }

    pub fn trigger_watchdog(&mut self) {
        self.record("trigger_watchdog");
        self.watchdog_counters += 1;
    }

    // -------------------------------- features ---------------------------------

    pub fn load_stack_config(&mut self, config: Value) {
        self.record("load_stack_config");
        self.stack_config = config;
    }

    pub fn set_concurrency(&mut self, concurrency: usize) {
        self.record("set_concurrency");
        self.concurrency = concurrency;
    }

    pub fn set_transmit_power(&mut self, power: i64) {
        self.record("set_transmit_power");
        self.transmit_power = Some(power);
    }

    pub fn log(&mut self, level: &str, message: &str) {
        self.logs.push((level.to_string(), message.to_string()));
    }

    pub fn warn(&mut self, message: &str) {
        self.warnings.push(message.to_string());
    }

    pub fn error(&mut self, message: &str) {
        self.errors.push(message.to_string());
    }

    pub fn register_multicast(&mut self, group: Value) {
        self.record("register_multicast");
        self.multicast.push(group);
    }

    pub fn switch_protocol(&mut self) {
        self.record("switch_protocol");
    }

    pub fn set_network_key_frame_counter(&mut self) {
        self.record("set_network_key_frame_counter");
    }
}
