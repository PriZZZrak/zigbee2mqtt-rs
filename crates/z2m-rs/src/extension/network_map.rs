// Original: .upstream/zigbee2mqtt/lib/extension/networkMap.ts

use serde_json::{json, Value};
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
    pub async fn handle_request(&mut self, payload: Value) {
        let request_type = payload.as_str().map(str::to_string).or_else(|| {
            payload
                .get("type")
                .and_then(Value::as_str)
                .map(str::to_string)
        });

        let Some(request_type) = request_type else {
            self.published.push((
                "zigbee2mqtt/bridge/response/networkmap".to_string(),
                json!({
                    "data": {},
                    "status": "error",
                    "error": "Type 'undefined' not supported, allowed are: raw,graphviz,plantuml",
                }),
                json!({}),
            ));
            return;
        };

        let (links, nodes) = self.build_map();

        let result = match request_type.as_str() {
            "raw" => {
                let value = json!({
                    "links": links.iter().map(link_to_json).collect::<Vec<_>>(),
                    "nodes": nodes.iter().map(node_to_json).collect::<Vec<_>>(),
                });
                json!({"data": {"routes": true, "type": "raw", "value": value}, "status": "ok"})
            }
            "graphviz" => {
                let value = self.graphviz(&links, &nodes);
                json!({"data": {"routes": true, "type": "graphviz", "value": value}, "status": "ok"})
            }
            "plantuml" => {
                let value = self.plantuml(&links, &nodes);
                json!({"data": {"routes": true, "type": "plantuml", "value": value}, "status": "ok"})
            }
            other => json!({
                "data": {},
                "status": "error",
                "error": format!("Type '{other}' not supported, allowed are: raw,graphviz,plantuml"),
            }),
        };

        self.published.push((
            "zigbee2mqtt/bridge/response/networkmap".to_string(),
            result,
            json!({}),
        ));
    }

    fn device_by_addr(&self, ieee: &str, nwk: u16) -> Option<&NetworkMapDevice> {
        self.devices
            .iter()
            .find(|device| device.ieee_addr == ieee)
            .or_else(|| {
                if ieee == "0x0000000000000000" {
                    self.devices
                        .iter()
                        .find(|device| device.network_address == nwk)
                } else {
                    None
                }
            })
    }

    fn is_disabled(&self, device: &NetworkMapDevice) -> bool {
        self.disabled.iter().any(|ieee| ieee == &device.ieee_addr)
    }

    fn build_map(&mut self) -> (Vec<Link>, Vec<NetworkMapDevice>) {
        let nodes: Vec<NetworkMapDevice> = self
            .devices
            .iter()
            .filter(|device| !self.is_disabled(device))
            .cloned()
            .collect();

        let mut links = Vec::new();
        for polled in self.devices.clone() {
            if self.is_disabled(&polled) {
                continue;
            }

            let Some(entries) = self.lqi.get(&polled.ieee_addr).cloned() else {
                continue;
            };

            for entry in entries {
                if entry.relationship == 4 {
                    continue;
                }
                let Some(source) = self
                    .device_by_addr(&entry.eui64, entry.nwk_address)
                    .cloned()
                else {
                    continue;
                };
                if self.is_disabled(&source) {
                    // Links still reported by the parent are kept below; only
                    // the disabled device itself is not polled.
                }

                let routes = self
                    .routing_table
                    .get(&polled.ieee_addr)
                    .map(|routes| {
                        routes
                            .iter()
                            .filter(|route| route.next_hop_address == source.network_address)
                            .cloned()
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                links.push(Link {
                    source_ieee: source.ieee_addr.clone(),
                    source_nwk: source.network_address,
                    target_ieee: polled.ieee_addr.clone(),
                    target_nwk: polled.network_address,
                    depth: entry.depth,
                    lqi: entry.lqi,
                    relationship: entry.relationship,
                    routes,
                });
            }
        }

        (links, nodes)
    }

    fn relative_time(&self, device: &NetworkMapDevice) -> String {
        if device.device_type == "Coordinator" {
            return "0 seconds ago".to_string();
        }
        match device.last_seen {
            Some(last_seen) => {
                let seconds = self.now.saturating_sub(last_seen) / 1000;
                format!("{seconds} seconds ago")
            }
            None => "unknown".to_string(),
        }
    }

    fn definition_string(device: &NetworkMapDevice) -> Option<String> {
        let definition = device.definition.as_ref()?;
        let vendor = definition
            .get("vendor")
            .and_then(Value::as_str)
            .unwrap_or("");
        let description = definition
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("");
        let model = definition
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("");
        Some(format!("{vendor} {description} ({model})"))
    }

    fn graphviz(&self, links: &[Link], nodes: &[NetworkMapDevice]) -> String {
        let mut lines = vec![
            "digraph G {".to_string(),
            "    node[shape=record];".to_string(),
        ];

        for device in nodes {
            let (style, fill, font) = match device.device_type.as_str() {
                "Coordinator" => ("bold, filled", "#e04e5d", "#ffffff"),
                "EndDevice" => ("rounded, dashed, filled", "#fff8ce", "#000000"),
                _ => ("rounded, filled", "#4ea3e0", "#ffffff"),
            };
            let failed = if device.failed.is_empty() {
                String::new()
            } else {
                format!("failed: {}", device.failed.join(","))
            };
            let definition = Self::definition_string(device).unwrap_or_else(|| {
                if device.device_type == "Coordinator" {
                    String::new()
                } else {
                    String::new()
                }
            });
            let label = if device.device_type == "Coordinator" {
                format!(
                    "{{{0}|{1} (0x{2:04x})|{3}}}",
                    device.friendly_name,
                    device.ieee_addr,
                    device.network_address,
                    self.relative_time(device)
                )
            } else {
                format!(
                    "{{{0}|{1} (0x{2:04x}){3}|{4}|{5}}}",
                    device.friendly_name,
                    device.ieee_addr,
                    device.network_address,
                    failed,
                    definition,
                    self.relative_time(device)
                )
                .replace('"', r#"\""#)
            };
            lines.push(format!(
                "    \"{0}\" [style=\"{1}\", fillcolor=\"{2}\", fontcolor=\"{3}\", label=\"{4}\"];",
                device.ieee_addr, style, fill, font, label
            ));

            for link in links
                .iter()
                .filter(|link| link.source_ieee == device.ieee_addr)
            {
                let (width, weight, color) = if !link.routes.is_empty() {
                    (2.0, 1, "#009900")
                } else if device.device_type == "EndDevice" {
                    (1.0, 0, "#994444")
                } else {
                    (0.5, 0, "#994444")
                };
                let label = if let Some(route) = link.routes.first() {
                    format!("{} (routes: 0x{:04x})", link.lqi, route.destination_address)
                } else {
                    link.lqi.to_string()
                };
                lines.push(format!(
                    "    \"{0}\" -> \"{1}\" [penwidth={2}, weight={3}, color=\"{4}\", label=\"{5}\"]",
                    link.source_ieee, link.target_ieee, width, weight, color, label
                ));
            }
        }

        lines.push("}".to_string());
        lines.join("\n")
    }

    fn plantuml(&self, links: &[Link], nodes: &[NetworkMapDevice]) -> String {
        let mut sorted = nodes.to_vec();
        sorted.sort_by(|a, b| {
            a.friendly_name
                .to_lowercase()
                .cmp(&b.friendly_name.to_lowercase())
        });

        let mut output = String::from("' paste into: https://www.planttext.com/\n\n@startuml\n");
        for (index, device) in sorted.iter().enumerate() {
            if index > 0 {
                output.push('\n');
            }
            let failed = if device.failed.is_empty() {
                String::new()
            } else {
                format!(" failed: {}", device.failed.join(","))
            };
            output.push_str(&format!(
                "card {} [\n{}\n---\n{} (0x{:04x}){}\n",
                device.ieee_addr,
                device.friendly_name,
                device.ieee_addr,
                device.network_address,
                failed
            ));
            if let Some(definition) = Self::definition_string(device) {
                output.push_str(&format!("---\n{}\n", definition));
            }
            output.push_str(&format!("---\n{}\n]\n", self.relative_time(device)));
        }

        if !links.is_empty() {
            output.push('\n');
        }
        for link in links {
            output.push_str(&format!(
                "{} --> {}: {}\n",
                link.source_ieee, link.target_ieee, link.lqi
            ));
        }
        output.push_str("\n@enduml");
        output
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

#[derive(Debug, Clone)]
struct Link {
    source_ieee: String,
    source_nwk: u16,
    target_ieee: String,
    target_nwk: u16,
    depth: u8,
    lqi: u8,
    relationship: u8,
    routes: Vec<NetworkMapRoute>,
}

fn link_to_json(link: &Link) -> Value {
    json!({
        "deviceType": 0,
        "rxOnWhenIdle": 0,
        "permitJoining": 0,
        "depth": link.depth,
        "linkquality": link.lqi,
        "lqi": link.lqi,
        "relationship": link.relationship,
        "routes": link.routes.iter().map(|route| json!({
            "destinationAddress": route.destination_address,
            "nextHopAddress": route.next_hop_address,
            "status": route.status,
            "memoryConstrained": 0,
            "manyToOne": 0,
            "routeRecordRequired": 0,
            "reserved1": 0,
        })).collect::<Vec<_>>(),
        "source": {"ieeeAddr": link.source_ieee, "networkAddress": link.source_nwk},
        "sourceIeeeAddr": link.source_ieee,
        "sourceNwkAddr": link.source_nwk,
        "target": {"ieeeAddr": link.target_ieee, "networkAddress": link.target_nwk},
        "targetIeeeAddr": link.target_ieee,
    })
}

fn node_to_json(device: &NetworkMapDevice) -> Value {
    let mut node = json!({
        "failed": device.failed,
        "friendlyName": device.friendly_name,
        "ieeeAddr": device.ieee_addr,
        "lastSeen": device.last_seen,
        "networkAddress": device.network_address,
        "type": device.device_type,
    });
    let object = node.as_object_mut().unwrap();
    if let Some(definition) = &device.definition {
        object.insert("definition".to_string(), definition.clone());
    }
    if let Some(model_id) = &device.model_id {
        object.insert("modelID".to_string(), json!(model_id));
    }
    if let Some(manufacturer_name) = &device.manufacturer_name {
        object.insert("manufacturerName".to_string(), json!(manufacturer_name));
    }
    node
}
