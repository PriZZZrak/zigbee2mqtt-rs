// Original: .upstream/zigbee-herdsman/src/adapter/adapterDiscovery.ts
// TODO: port discovery logic.

use crate::adapter::adapter::AdapterKind;
use crate::adapter::serial_port::SerialPortInfo;

/// Upstream `findAllDevices()` result entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredAdapter {
    pub name: String,
    pub path: String,
    pub adapter: Option<AdapterKind>,
    /// Upstream camelCase: `baudRate`.
    pub baud_rate: Option<u32>,
    pub rtscts: Option<bool>,
}

/// One mocked mDNS service (upstream `bonjour-service` `Service`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdnsService {
    pub name: Option<String>,
    /// Upstream `txt.radio_type`.
    pub radio_type: Option<String>,
    pub host: String,
    pub port: String,
    pub addresses: Vec<String>,
}

impl MdnsService {
    pub fn new(radio_type: &str) -> Self {
        Self {
            name: Some("Mock Adapter".to_string()),
            radio_type: Some(radio_type.to_string()),
            host: "mock_adapter.local".to_string(),
            port: "1122".to_string(),
            addresses: vec!["192.168.1.123".to_string()],
        }
    }
}

/// Test/dev injection point for `SerialPort.list()` + `bonjour-service`.
#[derive(Debug, Clone)]
pub struct DiscoveryContext {
    /// Upstream `node:os.platform()`.
    pub platform: String,
    /// Upstream `SerialPort.list()` result.
    pub serial_ports: Result<Vec<SerialPortInfo>, String>,
    /// Services returned by the mDNS browser, in discovery order.
    pub mdns_services: Vec<MdnsService>,
    /// Upstream `new Bonjour(options, errorCb)` error callback.
    pub mdns_start_error: Option<String>,
    /// Upstream `mockBonjourDestroy` call count.
    pub bonjour_destroyed: usize,
}

impl Default for DiscoveryContext {
    fn default() -> Self {
        Self {
            platform: "linux".to_string(),
            serial_ports: Ok(Vec::new()),
            mdns_services: vec![
                MdnsService::new("zstack"),
                MdnsService::new("ezsp"), // expected as `ember` Adapter
                MdnsService::new("znp"),  // expected as `zstack` Adapter
            ],
            mdns_start_error: None,
            bonjour_destroyed: 0,
        }
    }
}

/// Upstream `findAllDevices()`.
pub fn find_all_devices(_context: &mut DiscoveryContext) -> Vec<DiscoveredAdapter> {
    todo!()
}

pub fn discover_adapter() {
    todo!()
}
pub fn find_all_devices_deprecated() {
    todo!()
}
pub fn find_mdns_adapter() {
    todo!()
}
pub fn find_tcp_adapter() {
    todo!()
}
pub fn find_usb_adapter() {
    todo!()
}
pub fn find_usb_adapter_best_match() {
    todo!()
}
pub fn match_usb_adapter() {
    todo!()
}
