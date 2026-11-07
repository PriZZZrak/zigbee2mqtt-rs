// Original: .upstream/zigbee-herdsman/src/controller/controller.ts
// TODO: port the full controller implementation.

use std::collections::HashMap;

use serde_json::{json, Value};

/// Model of `Controller` used by the ported tests.
#[derive(Debug, Default)]
pub struct Controller {
    /// Recorded method calls (`name` -> count).
    pub calls: HashMap<String, usize>,
    pub started: bool,
    pub stopped: bool,
    pub network: Value,
    pub devices: Vec<Value>,
    pub groups: Vec<Value>,
    pub published: Vec<(String, Value)>,
    pub events: Vec<Value>,
    pub logs: Vec<(String, String)>,
    pub database: Vec<Value>,
    pub backup_created: Option<String>,
    pub permit_join: Option<Value>,
    pub install_codes: Vec<Value>,
    pub last_error: Option<String>,
    pub last_response: Option<Value>,
    pub touchlink_calls: Vec<Value>,
    pub attributes: HashMap<String, Value>,
    pub options: Value,
    pub blocked: Vec<String>,
    pub state: Value,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            network: json!({}),
            options: json!({}),
            state: json!({}),
            ..Self::default()
        }
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

    pub fn log(&mut self, level: &str, message: &str) {
        self.logs.push((level.to_string(), message.to_string()));
    }

    // ------------------------------- lifecycle -------------------------------

    pub fn start(&mut self) {
        self.record("start");
        self.started = true;
    }

    pub fn stop(&mut self) {
        self.record("stop");
        self.stopped = true;
    }

    pub fn hard_reset(&mut self) {
        self.record("hard_reset");
    }

    pub fn reset(&mut self) {
        self.record("reset");
    }

    pub fn backup(&mut self) -> String {
        self.record("backup");
        self.backup_created = Some("backup".to_string());
        "backup".to_string()
    }

    pub fn coordinator_check(&mut self) {
        self.record("coordinator_check");
    }

    // ------------------------------ permissions --------------------------------

    pub fn permit_join(&mut self, _duration: Option<u32>, _device: Option<&str>) {
        self.record("permit_join");
        self.permit_join = Some(json!({"duration": _duration, "device": _device}));
    }

    pub fn get_permit_join(&self) -> Option<&Value> {
        self.permit_join.as_ref()
    }

    pub fn add_install_code(&mut self, _ieee_addr: &str, _install_code: &str) {
        self.record("add_install_code");
        self.install_codes
            .push(json!({"ieee": _ieee_addr, "code": _install_code}));
    }

    // -------------------------------- devices ----------------------------------

    pub fn join_device(&mut self, device: Value) {
        self.record("join_device");
        self.devices.push(device);
    }

    pub fn leave_device(&mut self, _device: &str) {
        self.record("leave_device");
    }

    pub fn remove_device(&mut self, _device: &str, _force: bool, _keep_cache: bool) {
        self.record("remove_device");
    }

    pub fn get_device(&self, _id: &str) -> Option<&Value> {
        self.devices.first()
    }

    pub fn get_devices(&self) -> &[Value] {
        &self.devices
    }

    pub fn get_devices_iterator(&self) -> impl Iterator<Item = &Value> {
        self.devices.iter()
    }

    pub fn get_group(&self, _id: &str) -> Option<&Value> {
        self.groups.first()
    }

    pub fn get_groups(&self) -> &[Value] {
        &self.groups
    }

    pub fn get_groups_iterator(&self) -> impl Iterator<Item = &Value> {
        self.groups.iter()
    }

    pub fn ping(&mut self, _device: &str) {
        self.record("ping");
    }

    pub fn lqi(&mut self, _device: &str) {
        self.record("lqi");
    }

    pub fn routing_table(&mut self, _device: &str) {
        self.record("routing_table");
    }

    pub fn active_endpoints(&mut self, _device: &str) {
        self.record("active_endpoints");
    }

    pub fn node_descriptor(&mut self, _device: &str) {
        self.record("node_descriptor");
    }

    pub fn simple_descriptor(&mut self, _device: &str) {
        self.record("simple_descriptor");
    }

    pub fn binding_table(&mut self, _device: &str) {
        self.record("binding_table");
    }

    pub fn update_network_address(&mut self, _device: &str, _address: u16) {
        self.record("update_network_address");
    }

    pub fn update_last_seen(&mut self, _device: &str) {
        self.record("update_last_seen");
    }

    // --------------------------------- binding ----------------------------------

    pub fn bind(&mut self, _device: &str, _endpoint: u8, _cluster: &str) {
        self.record("bind");
    }

    pub fn unbind(&mut self, _device: &str, _endpoint: u8, _cluster: &str) {
        self.record("unbind");
    }

    pub fn clear_bindings(&mut self, _device: &str) {
        self.record("clear_bindings");
    }

    // --------------------------------- groups -----------------------------------

    pub fn create_group(&mut self, group: Value) {
        self.record("create_group");
        self.groups.push(group);
    }

    pub fn remove_group(&mut self, _id: &str) {
        self.record("remove_group");
    }

    pub fn add_to_group(&mut self, _device: &str, _group: &str) {
        self.record("add_to_group");
    }

    pub fn remove_from_group(&mut self, _device: &str, _group: &str) {
        self.record("remove_from_group");
    }

    pub fn remove_endpoint_from_all_groups(&mut self, _device: &str, _endpoint: u8) {
        self.record("remove_endpoint_from_all_groups");
    }

    pub fn group_command(&mut self, _group: &str, _command: &str) {
        self.record("group_command");
    }

    // ------------------------------- ZCL I/O ------------------------------------

    pub fn read(&mut self, _id: &str, _cluster: &str, _attributes: Value) {
        self.record("read");
    }

    pub fn write(&mut self, _id: &str, _cluster: &str, _attributes: Value) {
        self.record("write");
    }

    pub fn command(&mut self, _id: &str, _cluster: &str, _command: &str, _payload: Value) {
        self.record("command");
    }

    pub fn command_response(&mut self, _id: &str, _cluster: &str, _command: &str, _payload: Value) {
        self.record("command_response");
    }

    pub fn default_response(&mut self, _id: &str, _command: u8, _status: u8) {
        self.record("default_response");
    }

    pub fn report(&mut self, _id: &str, _cluster: &str, _attributes: Value) {
        self.record("report");
    }

    pub fn receive_zcl_data(&mut self, _payload: Value) {
        self.record("receive_zcl_data");
    }

    pub fn receive_cluster_command(&mut self, _payload: Value) {
        self.record("receive_cluster_command");
    }

    pub fn receive_raw_data(&mut self, _payload: Value) {
        self.record("receive_raw_data");
    }

    pub fn respond_gen_time(&mut self, _id: &str) {
        self.record("respond_gen_time");
    }

    pub fn configure_reporting(&mut self, _id: &str, _config: Value) {
        self.record("configure_reporting");
    }

    pub fn read_reporting_config(&mut self, _id: &str, _config: Value) {
        self.record("read_reporting_config");
    }

    pub fn send_to_all(&mut self, _command: &str) {
        self.record("send_to_all");
    }

    pub fn send_raw(&mut self, _data: Value) {
        self.record("send_raw");
    }

    // ------------------------------ touchlink/GP ---------------------------------

    pub fn touchlink(&mut self, action: &str, _payload: Value) {
        self.record("touchlink");
        self.touchlink_calls.push(json!({"action": action}));
    }

    pub fn green_power(&mut self, _payload: Value) {
        self.record("green_power");
    }

    // ------------------------------- network -------------------------------------

    pub fn get_network_parameters(&self) -> &Value {
        &self.network
    }

    pub fn get_coordinator_version(&mut self) {
        self.record("get_coordinator_version");
    }

    pub fn set_channel(&mut self, _channel: u8) {
        self.record("set_channel");
    }

    pub fn set_network_key(&mut self, _key: Value) {
        self.record("set_network_key");
    }

    pub fn update_ieee_address(&mut self, _device: &str) {
        self.record("update_ieee_address");
    }

    // ------------------------------- state/db ------------------------------------

    pub fn load_database(&mut self) {
        self.record("load_database");
    }

    pub fn save_endpoint_configure_reporting(&mut self) {
        self.record("save_endpoint_configure_reporting");
    }

    pub fn set_options(&mut self, options: Value) {
        self.record("set_options");
        self.options = options;
    }

    pub fn set_type(&mut self, _device_type: &str) {
        self.record("set_type");
    }

    pub fn add_custom_cluster(&mut self, _cluster: Value) {
        self.record("add_custom_cluster");
    }

    pub fn receive_event(&mut self, event: Value) {
        self.record("receive_event");
        self.events.push(event);
    }

    pub fn emit_event(&mut self, event: Value) {
        self.record("emit_event");
        self.events.push(event);
    }

    pub fn health_check(&mut self) {
        self.record("health_check");
    }

    pub fn is_adapter_disconnected(&self) -> bool {
        self.called("adapter_disconnected")
    }

    pub fn is_stopping(&self) -> bool {
        self.stopped
    }
}
