// Original: .upstream/zigbee2mqtt/lib/extension/networkMap.ts

use serde_json::Value;
use std::collections::HashMap;

/// A device as seen by the network map extension (upstream `Device`).
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkMapDevice {
    pub ieee_addr: String,
    pub network_address: u16,
    /// `Coordinator` / `Router` / `EndDevice`.
    pub device_type: String,
    pub friendly_name: String,
    pub definition: Option<Value>,
    /// Upstream camelCase: `modelID`.
    pub model_id: Option<String>,
    /// Upstream camelCase: `manufacturerName`.
    pub manufacturer_name: Option<String>,
    /// Upstream camelCase: `lastSeen`.
    pub last_seen: Option<u64>,
    /// Upstream: links for which `lqi` / `routingTable` failed.
    pub failed: Vec<String>,
}

/// LQI table entry returned by `device.lqi()` (upstream `LQI`).
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkMapLqi {
    pub eui64: String,
    /// Upstream camelCase: `nwkAddress`.
    pub nwk_address: u16,
    pub relationship: u8,
    pub depth: u8,
    pub lqi: u8,
}

/// One entry of `device.routingTable()` (upstream `RoutingTable`).
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkMapRoute {
    /// Upstream camelCase: `destinationAddress`.
    pub destination_address: u16,
    /// Upstream camelCase: `nextHopAddress`.
    pub next_hop_address: u16,
    pub status: String,
}

/// Model of `lib/extension/networkMap.ts` used by the ported tests.
#[derive(Debug, Default)]
pub struct NetworkMap {
    pub devices: Vec<NetworkMapDevice>,
    /// Upstream camelCase: `lqi` responses keyed by `ieeeAddr`.
    pub lqi: HashMap<String, Vec<NetworkMapLqi>>,
    /// Upstream camelCase: `routingTable` responses keyed by `ieeeAddr`.
    pub routing_table: HashMap<String, Vec<NetworkMapRoute>>,
    /// Number of initial attempts that reject before succeeding.
    pub failing_attempts: HashMap<String, usize>,
    /// Devices excluded from the map (upstream `disabled` setting).
    pub disabled: Vec<String>,
    /// Upstream `mockMQTTPublishAsync` calls: `(topic, payload, options)`.
    pub published: Vec<(String, Value, Value)>,
    /// Upstream `mockSleep`/retry bookkeeping.
    pub retries: usize,
    /// Upstream `Date.now()` during the test.
    pub now: u64,
}

impl NetworkMap {
    pub fn new() -> Self {
        Self {
            now: 10_000,
            ..Self::default()
        }
    }

    pub fn add_device(&mut self, device: NetworkMapDevice) {
        self.devices.push(device);
    }

    pub fn set_lqi(&mut self, ieee_addr: &str, entries: Vec<NetworkMapLqi>) {
        self.lqi.insert(ieee_addr.to_string(), entries);
    }

    pub fn set_routing_table(&mut self, ieee_addr: &str, entries: Vec<NetworkMapRoute>) {
        self.routing_table.insert(ieee_addr.to_string(), entries);
    }

    pub fn set_disabled(&mut self, ieee_addr: &str, disabled: bool) {
        if disabled {
            if !self.disabled.iter().any(|ieee| ieee == ieee_addr) {
                self.disabled.push(ieee_addr.to_string());
            }
        } else {
            self.disabled.retain(|ieee| ieee != ieee_addr);
        }
    }

    /// Upstream `settings.set(["devices", ieeeAddr, "disabled"], true)`.
    pub fn disable_device(&mut self, ieee_addr: &str) {
        self.set_disabled(ieee_addr, true);
    }

    /// Upstream `mockMQTTEvents.message("zigbee2mqtt/bridge/request/networkmap", ...)`.
    pub async fn handle_request(&mut self, _payload: Value) {
        todo!()
    }

    /// Upstream `devices[ieeeAddr].lqi()` mock failing on the first attempt.
    pub fn fail_attempts(&mut self, ieee_addr: &str, attempts: usize) {
        self.failing_attempts
            .insert(ieee_addr.to_string(), attempts);
    }
}

impl NetworkMapDevice {
    pub fn coordinator(ieee_addr: &str, network_address: u16) -> Self {
        Self {
            ieee_addr: ieee_addr.to_string(),
            network_address,
            device_type: "Coordinator".to_string(),
            friendly_name: "Coordinator".to_string(),
            definition: None,
            model_id: None,
            manufacturer_name: None,
            last_seen: Some(1000),
            failed: Vec::new(),
        }
    }
}
