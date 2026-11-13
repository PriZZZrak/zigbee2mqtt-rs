// Original: .upstream/zigbee2mqtt/lib/extension/networkMap.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::sync::Arc;

use serde_json::{json, Value};

use crate::extension::extension::{Extension, ExtensionContext, ExtensionFuture};
use crate::model::entity::Entity;
use crate::util::stringify::stringify;
use crate::util::utils;

const SUPPORTED_FORMATS: [&str; 3] = ["raw", "graphviz", "plantuml"];

pub struct NetworkMap {
    context: ExtensionContext,
    topic: String,
}

impl NetworkMap {
    pub fn new(context: ExtensionContext) -> Arc<Self> {
        let topic = format!(
            "{}/bridge/request/networkmap",
            context.settings_value()["mqtt"]["base_topic"]
                .as_str()
                .unwrap_or("zigbee2mqtt")
        );
        Arc::new(Self { context, topic })
    }

    /// Upstream `onMQTTMessage` (`networkMap.ts:24-59`).
    async fn on_mqtt_message(self: &Arc<Self>, data: Value) {
        if data.get("topic").and_then(Value::as_str) != Some(self.topic.as_str()) {
            return;
        }
        let text = data.get("message").and_then(Value::as_str).unwrap_or("");
        let message = utils::parse_json(text, text);

        let response = (|| -> Result<Value, String> {
            let r#type = if message.is_object() {
                message.get("type").cloned().unwrap_or(Value::Null)
            } else {
                message.clone()
            };
            let type_name = r#type.as_str().unwrap_or("").to_string();
            if !SUPPORTED_FORMATS.contains(&type_name.as_str()) {
                return Err(format!(
                    "Type '{type_name}' not supported, allowed are: {}",
                    SUPPORTED_FORMATS.join(",")
                ));
            }
            let routes = message
                .get("routes")
                .and_then(Value::as_bool)
                .unwrap_or(false);

            // `networkScan` is async; the scan itself is executed by
            // `network_scan` from the async caller below.
            let _ = routes;
            Ok(json!({"type": type_name, "routes": routes}))
        })();

        let Ok(mut prepared) = response else {
            let error = response.unwrap_err();
            self.context
                .mqtt
                .lock()
                .await
                .publish(
                    "bridge/response/networkmap",
                    stringify(&utils::get_response(&message, json!({}), Some(error))),
                    Default::default(),
                )
                .await;
            return;
        };

        let routes = prepared
            .get("routes")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let topology = self.network_scan(routes).await;
        let type_name = prepared["type"].as_str().unwrap_or("raw").to_string();
        let value = match type_name.as_str() {
            "graphviz" => Value::String(self.graphviz(&topology)),
            "plantuml" => Value::String(self.plantuml(&topology)),
            _ => self.raw(&topology),
        };
        prepared["value"] = value;

        self.context
            .mqtt
            .lock()
            .await
            .publish(
                "bridge/response/networkmap",
                stringify(&utils::get_response(&message, prepared, None)),
                Default::default(),
            )
            .await;
    }

    /// Upstream `raw` (`networkMap.ts:61-63`).
    pub fn raw(&self, topology: &Value) -> Value {
        topology.clone()
    }

    /// Upstream `graphviz` (`networkMap.ts:65-134`).
    pub fn graphviz(&self, topology: &Value) -> String {
        let settings = self.context.settings_value();
        let colors = &settings["map_options"]["graphviz"]["colors"];
        let text_color =
            |kind: &str, color: &str| colors["fill"][kind].as_str().unwrap_or(color).to_string();
        let font_color = |kind: &str| {
            colors["font"][kind]
                .as_str()
                .unwrap_or("#ffffff")
                .to_string()
        };

        let mut text = String::from("digraph G {\nnode[shape=record];\n");
        let nodes = topology["nodes"].as_array().cloned().unwrap_or_default();
        let links = topology["links"].as_array().cloned().unwrap_or_default();

        for node in &nodes {
            let mut labels = Vec::new();
            labels.push(
                node["friendlyName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            );
            let failed = node["failed"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            labels.push(format!(
                "{} ({}){}",
                node["ieeeAddr"].as_str().unwrap_or_default(),
                utils::to_network_address_hex(node["networkAddress"].as_u64().unwrap_or(0)),
                if failed.is_empty() {
                    String::new()
                } else {
                    format!("failed: {failed}")
                }
            ));

            if node["type"].as_str() != Some("Coordinator") {
                if let Some(definition) = node.get("definition").filter(|d| d.is_object()) {
                    labels.push(format!(
                        "{} {} ({})",
                        definition["vendor"].as_str().unwrap_or_default(),
                        definition["description"].as_str().unwrap_or_default(),
                        definition["model"].as_str().unwrap_or_default(),
                    ));
                }
            }

            let date = if node["type"].as_str() == Some("Coordinator") {
                Some(zigbee_apiarist::utils::datetime::now_ms() as i64)
            } else {
                node["lastSeen"].as_i64()
            };
            labels.push(
                date.map(utils::format_date_relative)
                    .unwrap_or_else(|| "unknown".to_string()),
            );

            let escaped: Vec<String> = labels
                .iter()
                .map(|label| label.replace('\\', "\\\\").replace('"', "\\\""))
                .collect();

            let style = match node["type"].as_str() {
                Some("Coordinator") => format!(
                    "style=\"bold, filled\", fillcolor=\"{}\", fontcolor=\"{}\"",
                    text_color("coordinator", "#e04e5d"),
                    font_color("coordinator")
                ),
                Some("Router") => format!(
                    "style=\"rounded, filled\", fillcolor=\"{}\", fontcolor=\"{}\"",
                    text_color("router", "#4ea3e0"),
                    font_color("router")
                ),
                _ => format!(
                    "style=\"rounded, dashed, filled\", fillcolor=\"{}\", fontcolor=\"{}\"",
                    text_color("enddevice", "#fff8ce"),
                    font_color("enddevice")
                ),
            };

            text.push_str(&format!(
                "  \"{}\" [{style}, label=\"{{{}}}\"];\n",
                node["ieeeAddr"].as_str().unwrap_or_default(),
                escaped.join("|")
            ));

            for link in &links {
                if link["source"]["ieeeAddr"].as_str() != node["ieeeAddr"].as_str() {
                    continue;
                }
                let routes = link["routes"].as_array().cloned().unwrap_or_default();
                let line_style = if node["type"].as_str() == Some("EndDevice") {
                    "penwidth=1, "
                } else if routes.is_empty() {
                    "penwidth=0.5, "
                } else {
                    "penwidth=2, "
                };
                let line_weight = if routes.is_empty() {
                    format!(
                        "weight=0, color=\"{}\", ",
                        colors["line"]["inactive"].as_str().unwrap_or("#994444")
                    )
                } else {
                    format!(
                        "weight=1, color=\"{}\", ",
                        colors["line"]["active"].as_str().unwrap_or("#009900")
                    )
                };
                let route_text: Vec<String> = routes
                    .iter()
                    .map(|route| {
                        utils::to_network_address_hex(
                            route["destinationAddress"].as_u64().unwrap_or(0),
                        )
                    })
                    .collect();
                let line_labels = if routes.is_empty() {
                    format!("label=\"{}\"", link["linkquality"])
                } else {
                    format!(
                        "label=\"{} (routes: {})\"",
                        link["linkquality"],
                        route_text.join(",")
                    )
                };
                text.push_str(&format!(
                    "  \"{}\" -> \"{}\" [{}]\n",
                    node["ieeeAddr"].as_str().unwrap_or_default(),
                    link["target"]["ieeeAddr"].as_str().unwrap_or_default(),
                    format!("{line_style}{line_weight}{line_labels}")
                ));
            }
        }

        text.push('}');
        text.replace('\u{0}', "")
    }

    /// Upstream `plantuml` (`networkMap.ts:136-187`).
    pub fn plantuml(&self, topology: &Value) -> String {
        let mut text = vec![
            "' paste into: https://www.planttext.com/".to_string(),
            String::new(),
            "@startuml".to_string(),
        ];

        let mut nodes = topology["nodes"].as_array().cloned().unwrap_or_default();
        nodes.sort_by_key(|node| {
            node["friendlyName"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        });

        for node in &nodes {
            text.push(format!(
                "card {} [",
                node["ieeeAddr"].as_str().unwrap_or_default()
            ));
            text.push(
                node["friendlyName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            );
            text.push("---".to_string());
            let failed = node["failed"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            text.push(format!(
                "{} ({}){}",
                node["ieeeAddr"].as_str().unwrap_or_default(),
                utils::to_network_address_hex(node["networkAddress"].as_u64().unwrap_or(0)),
                if failed.is_empty() {
                    String::new()
                } else {
                    format!(" failed: {failed}")
                }
            ));

            if node["type"].as_str() != Some("Coordinator") {
                text.push("---".to_string());
                if let Some(definition) = node.get("definition").filter(|d| d.is_object()) {
                    text.push(format!(
                        "{} {} ({})",
                        definition["vendor"].as_str().unwrap_or_default(),
                        definition["description"].as_str().unwrap_or_default(),
                        definition["model"].as_str().unwrap_or_default()
                    ));
                }
            }

            let date = if node["type"].as_str() == Some("Coordinator") {
                Some(zigbee_apiarist::utils::datetime::now_ms() as i64)
            } else {
                node["lastSeen"].as_i64()
            };
            text.push("---".to_string());
            text.push(
                date.map(utils::format_date_relative)
                    .unwrap_or_else(|| "unknown".to_string()),
            );
            text.push("]".to_string());
            text.push(String::new());
        }

        for link in topology["links"].as_array().cloned().unwrap_or_default() {
            text.push(format!(
                "{} --> {}: {}",
                link["sourceIeeeAddr"].as_str().unwrap_or_default(),
                link["targetIeeeAddr"].as_str().unwrap_or_default(),
                link["lqi"]
            ));
        }

        text.push(String::new());
        text.push("@enduml".to_string());
        text.join("\n")
    }

    /// Upstream `networkScan` (`networkMap.ts:189-337`).
    pub async fn network_scan(self: &Arc<Self>, include_routes: bool) -> Value {
        self.context.logger.lock().unwrap().info(
            &format!("Starting network scan (includeRoutes '{include_routes}')"),
            "z2m",
        );

        let devices: Vec<Arc<crate::model::Device>> =
            self.context.zigbee.devices_iterator(|device| {
                device.r#type() != zigbee_apiarist::controller::database::DeviceType::GreenPower
                    && device.r#type()
                        != zigbee_apiarist::controller::database::DeviceType::EndDevice
            });
        let mut lqis: Vec<(Arc<crate::model::Device>, Vec<Value>)> = Vec::new();
        let mut routing_tables: Vec<(Arc<crate::model::Device>, Vec<Value>)> = Vec::new();
        let mut failed: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        for device in devices {
            if device
                .options()
                .get("disabled")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                continue;
            }
            let mut device_failures = Vec::new();
            utils::sleep(1).await;

            let lqi = match device.zh.lqi().await {
                Ok(table) => Some(table),
                Err(_) => {
                    utils::sleep(5).await;
                    device.zh.lqi().await.ok()
                }
            };
            match lqi {
                Some(table) => {
                    lqis.push((device.clone(), table));
                    self.context
                        .logger
                        .lock()
                        .unwrap()
                        .debug(&format!("LQI succeeded for '{}'", device.name()), "z2m");
                }
                None => {
                    device_failures.push("lqi".to_string());
                    self.context.logger.lock().unwrap().error(
                        &format!("Failed to execute LQI for '{}'", device.name()),
                        "z2m",
                    );
                }
            }

            if include_routes {
                let routes = match device.zh.routing_table().await {
                    Ok(table) => Some(table),
                    Err(_) => {
                        utils::sleep(5).await;
                        device.zh.routing_table().await.ok()
                    }
                };
                match routes {
                    Some(table) => {
                        routing_tables.push((device.clone(), table));
                        self.context.logger.lock().unwrap().debug(
                            &format!("Routing table succeeded for '{}'", device.name()),
                            "z2m",
                        );
                    }
                    None => {
                        device_failures.push("routingTable".to_string());
                        self.context.logger.lock().unwrap().error(
                            &format!("Failed to execute routing table for '{}'", device.name()),
                            "z2m",
                        );
                    }
                }
            }

            failed.insert(device.ieee_addr(), device_failures);
        }

        self.context
            .logger
            .lock()
            .unwrap()
            .info("Network scan finished", "z2m");

        let mut nodes = Vec::new();
        let all_devices: Vec<Arc<crate::model::Device>> =
            self.context.zigbee.devices_iterator(|device| {
                device.r#type() != zigbee_apiarist::controller::database::DeviceType::GreenPower
            });
        for device in all_devices {
            if device
                .options()
                .get("disabled")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                continue;
            }

            let definition = device.definition().map(|definition| {
                let supports: std::collections::BTreeSet<String> = device
                    .exposes()
                    .unwrap_or_default()
                    .iter()
                    .map(|expose| {
                        expose.name.clone().unwrap_or_else(|| {
                            format!(
                                "{} ({})",
                                expose.r#type,
                                expose
                                    .features
                                    .iter()
                                    .filter_map(|feature| feature.name.clone())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        })
                    })
                    .collect();
                json!({
                    "model": definition.model,
                    "vendor": definition.vendor,
                    "description": definition.description,
                    "supports": supports.into_iter().collect::<Vec<_>>().join(", "),
                })
            });

            nodes.push(json!({
                "ieeeAddr": device.ieee_addr(),
                "friendlyName": device.name(),
                "type": device.zh.r#type().as_str(),
                "networkAddress": device.zh.network_address(),
                "manufacturerName": device.zh.manufacturer_name(),
                "modelID": device.zh.model_id(),
                "failed": failed.get(&device.ieee_addr()).cloned().unwrap_or_default(),
                "lastSeen": device.zh.last_seen(),
                "definition": definition,
            }));
        }

        let mut links = Vec::new();
        for (device, table) in lqis {
            for neighbor in table {
                if neighbor["relationship"].as_u64().unwrap_or(0) > 3 {
                    continue;
                }

                let neighbor_eui64 = if neighbor["eui64"].as_str() == Some("0x0000000000000000") {
                    self.context
                        .zigbee
                        .device_by_network_address(
                            neighbor["nwkAddress"].as_u64().unwrap_or(0) as u16
                        )
                        .map(|device| device.ieee_addr())
                        .unwrap_or_else(|| {
                            neighbor["eui64"].as_str().unwrap_or_default().to_string()
                        })
                } else {
                    neighbor["eui64"].as_str().unwrap_or_default().to_string()
                };

                let mut routes = Vec::new();
                if let Some((_, routing_table)) = routing_tables
                    .iter()
                    .find(|(candidate, _)| candidate.ieee_addr() == device.ieee_addr())
                {
                    for entry in routing_table {
                        if entry["nextHopAddress"].as_u64() == neighbor["nwkAddress"].as_u64() {
                            routes.push(entry.clone());
                        }
                    }
                }

                links.push(json!({
                    "source": {
                        "ieeeAddr": neighbor_eui64,
                        "networkAddress": neighbor["nwkAddress"],
                    },
                    "target": {
                        "ieeeAddr": device.ieee_addr(),
                        "networkAddress": device.zh.network_address(),
                    },
                    "deviceType": neighbor["deviceType"],
                    "rxOnWhenIdle": neighbor["rxOnWhenIdle"],
                    "relationship": neighbor["relationship"],
                    "permitJoining": neighbor["permitJoining"],
                    "depth": neighbor["depth"],
                    "lqi": neighbor["lqi"],
                    "routes": routes,
                    "sourceIeeeAddr": neighbor_eui64,
                    "targetIeeeAddr": device.ieee_addr(),
                    "sourceNwkAddr": neighbor["nwkAddress"],
                    "linkquality": neighbor["lqi"],
                }));
            }
        }

        json!({"nodes": nodes, "links": links})
    }
}

impl Extension for NetworkMap {
    fn name(&self) -> &'static str {
        "NetworkMap"
    }

    fn context(&self) -> &ExtensionContext {
        &self.context
    }

    fn start(self: Arc<Self>) -> ExtensionFuture {
        Box::pin(async move {
            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_mqtt_message(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move { this.on_mqtt_message(data).await });
                }),
            );
            Ok(())
        })
    }
}
