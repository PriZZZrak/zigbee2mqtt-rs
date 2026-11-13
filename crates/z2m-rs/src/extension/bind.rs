// Original: .upstream/zigbee2mqtt/lib/extension/bind.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};
use zigbee_apiarist::controller::database::DeviceType;
use zigbee_apiarist::controller::model::endpoint::{BindTarget, ConfigureReportingItem, Endpoint};
use zigbee_apiarist::controller::model::group::Group as ZhGroup;

use crate::extension::extension::{Extension, ExtensionContext, ExtensionFuture};
use crate::model::entity::Entity;
use crate::util::stringify::stringify;
use crate::util::utils;

/// Upstream `ALL_CLUSTER_CANDIDATES` (`bind.ts:16-28`).
const ALL_CLUSTER_CANDIDATES: [&str; 11] = [
    "genScenes",
    "genOnOff",
    "genLevelCtrl",
    "lightingColorCtrl",
    "closuresWindowCovering",
    "hvacThermostat",
    "msIlluminanceMeasurement",
    "msTemperatureMeasurement",
    "msRelativeHumidity",
    "msSoilMoisture",
    "msCO2",
];

/// Upstream `DEFAULT_BIND_GROUP` (`bind.ts:31`).
const DEFAULT_BIND_GROUP_ID: u16 = 901;
const DEFAULT_BIND_GROUP_NAME: &str = "default_bind_group";

/// Upstream manufacturer codes in `POLL_ON_MESSAGE` (`bind.ts:95-134`).
const POLL_MANUFACTURER_IDS: [u16; 6] = [4107, 4116, 4398, 4417, 4635, 4687];
const POLL_MANUFACTURER_NAMES: [&str; 2] = ["GLEDOPTO", "Trust International B.V.\u{0}"];

struct PollRule {
    cluster: &'static str,
    r#type: &'static str,
    data: Value,
    read_cluster: &'static str,
    attributes: &'static [&'static str],
    color_attributes: bool,
}

fn poll_rules() -> Vec<PollRule> {
    let mut rules = Vec::new();
    let philips_buttons = |button: u64| json!({"button": button});
    for (cluster, r#type, data) in [
        (
            "manuSpecificPhilips",
            "commandHueNotification",
            philips_buttons(2),
        ),
        (
            "manuSpecificPhilips",
            "commandHueNotification",
            philips_buttons(3),
        ),
        ("genLevelCtrl", "commandStep", json!({})),
        ("genLevelCtrl", "commandStepWithOnOff", json!({})),
        ("genLevelCtrl", "commandStop", json!({})),
        ("genLevelCtrl", "commandMoveWithOnOff", json!({})),
        ("genLevelCtrl", "commandStopWithOnOff", json!({})),
        ("genLevelCtrl", "commandMove", json!({})),
        ("genLevelCtrl", "commandMoveToLevelWithOnOff", json!({})),
        ("genScenes", "commandRecall", json!({})),
    ] {
        rules.push(PollRule {
            cluster,
            r#type,
            data,
            read_cluster: "genLevelCtrl",
            attributes: &["currentLevel"],
            color_attributes: false,
        });
    }
    for (cluster, r#type, data) in [
        ("genLevelCtrl", "commandStepWithOnOff", json!({})),
        ("genLevelCtrl", "commandMoveWithOnOff", json!({})),
        ("genLevelCtrl", "commandStopWithOnOff", json!({})),
        ("genLevelCtrl", "commandMoveToLevelWithOnOff", json!({})),
        ("genOnOff", "commandOn", json!({})),
        ("genOnOff", "commandOff", json!({})),
        ("genOnOff", "commandOffWithEffect", json!({})),
        ("genOnOff", "commandToggle", json!({})),
        ("genScenes", "commandRecall", json!({})),
        (
            "manuSpecificPhilips",
            "commandHueNotification",
            philips_buttons(1),
        ),
        (
            "manuSpecificPhilips",
            "commandHueNotification",
            philips_buttons(4),
        ),
    ] {
        rules.push(PollRule {
            cluster,
            r#type,
            data,
            read_cluster: "genOnOff",
            attributes: &["onOff"],
            color_attributes: false,
        });
    }
    rules.push(PollRule {
        cluster: "genScenes",
        r#type: "commandRecall",
        data: json!({}),
        read_cluster: "lightingColorCtrl",
        attributes: &[],
        color_attributes: true,
    });
    rules
}

/// Upstream `REPORT_CLUSTERS` configuration.
struct ReportConfig {
    cluster: &'static str,
    attribute: &'static str,
    minimum: u32,
    maximum: u32,
    change: f64,
    requires: Option<&'static str>,
}

fn report_configs() -> Vec<ReportConfig> {
    let default = |attribute| ReportConfig {
        cluster: "",
        attribute,
        minimum: 5,
        maximum: 3600,
        change: 1.0,
        requires: None,
    };
    vec![
        ReportConfig {
            cluster: "genOnOff",
            attribute: "onOff",
            minimum: 0,
            maximum: 3600,
            change: 0.0,
            requires: None,
        },
        ReportConfig {
            cluster: "genLevelCtrl",
            ..default("currentLevel")
        },
        ReportConfig {
            cluster: "lightingColorCtrl",
            requires: Some("colorTemperature"),
            ..default("colorTemperature")
        },
        ReportConfig {
            cluster: "lightingColorCtrl",
            requires: Some("colorXY"),
            ..default("currentX")
        },
        ReportConfig {
            cluster: "lightingColorCtrl",
            requires: Some("colorXY"),
            ..default("currentY")
        },
        ReportConfig {
            cluster: "closuresWindowCovering",
            ..default("currentPositionLiftPercentage")
        },
        ReportConfig {
            cluster: "closuresWindowCovering",
            ..default("currentPositionTiltPercentage")
        },
    ]
}

struct ParsedMessage {
    r#type: String,
    source_key: Option<String>,
    source_endpoint_key: Option<String>,
    target_key: Option<String>,
    target_endpoint_key: Option<Value>,
    clusters: Option<Vec<String>>,
    skip_disable_reporting: bool,
    resolved_source: Option<Arc<crate::model::Device>>,
    resolved_target_device: Option<Arc<crate::model::Device>>,
    resolved_target_group: Option<Arc<crate::model::Group>>,
    default_group: bool,
    resolved_source_endpoint: Option<Arc<Endpoint>>,
    resolved_bind_target: Option<BindTarget>,
    raw: Value,
}

pub struct Bind {
    context: ExtensionContext,
    topic_regex: regex::Regex,
    poll_debouncers: Mutex<std::collections::HashMap<String, u64>>,
}

impl Bind {
    pub fn new(context: ExtensionContext) -> Arc<Self> {
        let base = context.settings_value()["mqtt"]["base_topic"]
            .as_str()
            .unwrap_or("zigbee2mqtt")
            .to_string();
        Arc::new(Self {
            context,
            topic_regex: regex::Regex::new(&format!(
                "^{}/bridge/request/device/(bind|unbind|binds/clear)",
                regex::escape(&base)
            ))
            .expect("valid bind topic regex"),
            poll_debouncers: Mutex::new(std::collections::HashMap::new()),
        })
    }

    /// Upstream `parseMQTTMessage` (`bind.ts:201-280`).
    fn parse(&self, data: &Value) -> Option<(Value, Result<ParsedMessage, String>)> {
        let topic = data.get("topic").and_then(Value::as_str)?;
        self.topic_regex.captures(topic)?;
        let r#type = if topic.ends_with("unbind") {
            "unbind".to_string()
        } else if topic.ends_with("binds/clear") {
            "binds/clear".to_string()
        } else {
            "bind".to_string()
        };
        let text = data.get("message").and_then(Value::as_str).unwrap_or("");
        let raw = utils::parse_json(text, text);

        if r#type == "binds/clear" {
            return Some((
                raw.clone(),
                Ok(ParsedMessage {
                    r#type,
                    source_key: None,
                    source_endpoint_key: None,
                    target_key: None,
                    target_endpoint_key: None,
                    clusters: None,
                    skip_disable_reporting: false,
                    resolved_source: None,
                    resolved_target_device: None,
                    resolved_target_group: None,
                    default_group: false,
                    resolved_source_endpoint: None,
                    resolved_bind_target: None,
                    raw,
                }),
            ));
        }

        Some((
            raw.clone(),
            (|| -> Result<ParsedMessage, String> {
                if !raw.is_object() || raw.get("from").is_none() || raw.get("to").is_none() {
                    return Err("Invalid payload".to_string());
                }
                let source_key = raw["from"]
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| raw["from"].to_string());
                let source_endpoint_key = raw
                    .get("from_endpoint")
                    .and_then(Value::as_str)
                    .unwrap_or("default")
                    .to_string();
                let target_key = raw["to"]
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| raw["to"].to_string());
                let target_endpoint_key = raw.get("to_endpoint").cloned();
                let clusters = raw.get("clusters").and_then(Value::as_array).map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                });
                let skip_disable_reporting = raw
                    .get("skip_disable_reporting")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);

                let source_entity = self.context.zigbee.resolve_entity(&source_key);
                let resolved_source = source_entity.and_then(|entity| {
                    entity
                        .zh_device()
                        .and_then(|zh| self.context.zigbee.resolve_device(&zh.ieee_addr()))
                });
                if resolved_source.is_none() {
                    return Err(format!("Source device '{}' does not exist", source_key));
                }
                let resolved_source = resolved_source.unwrap();

                let default_group = target_key == DEFAULT_BIND_GROUP_NAME
                    || target_key == DEFAULT_BIND_GROUP_ID.to_string();
                let (resolved_target_device, resolved_target_group, resolved_bind_target) =
                    if default_group {
                        (None, None, Some(BindTarget::GroupId(DEFAULT_BIND_GROUP_ID)))
                    } else {
                        let entity = self.context.zigbee.resolve_entity(&target_key);
                        let Some(entity) = entity else {
                            return Err(format!(
                                "Target device or group '{}' does not exist",
                                target_key
                            ));
                        };
                        if entity.is_device() {
                            let device = entity
                                .zh_device()
                                .and_then(|zh| self.context.zigbee.resolve_device(&zh.ieee_addr()));
                            let Some(device) = device else {
                                return Err(format!(
                                    "Target device or group '{}' does not exist",
                                    target_key
                                ));
                            };
                            let endpoint = device
                                .endpoint(target_endpoint_key.as_ref().and_then(Value::as_str));
                            let Some(endpoint) = endpoint else {
                                return Err(format!(
                                    "Target device '{}' does not have endpoint '{}'",
                                    device.name(),
                                    target_endpoint_key
                                        .as_ref()
                                        .and_then(Value::as_str)
                                        .unwrap_or("default")
                                ));
                            };
                            (Some(device), None, Some(BindTarget::Endpoint(endpoint)))
                        } else if let Some(group) = entity.zh_group() {
                            let group = self
                                .context
                                .zigbee
                                .group_by_id(group.group_id as u64)
                                .expect("resolved group");
                            (
                                None,
                                Some(group.clone()),
                                Some(BindTarget::Group(group.zh.clone())),
                            )
                        } else {
                            return Err(format!(
                                "Target device or group '{}' does not exist",
                                target_key
                            ));
                        }
                    };

                let resolved_source_endpoint = resolved_source.endpoint(Some(&source_endpoint_key));
                let Some(resolved_source_endpoint) = resolved_source_endpoint else {
                    return Err(format!(
                        "Source device '{}' does not have endpoint '{}'",
                        resolved_source.name(),
                        source_endpoint_key
                    ));
                };

                Ok(ParsedMessage {
                    r#type,
                    source_key: Some(source_key),
                    source_endpoint_key: Some(source_endpoint_key),
                    target_key: Some(target_key),
                    target_endpoint_key,
                    clusters,
                    skip_disable_reporting,
                    resolved_source: Some(resolved_source),
                    resolved_target_device,
                    resolved_target_group,
                    default_group,
                    resolved_source_endpoint: Some(resolved_source_endpoint),
                    resolved_bind_target,
                    raw,
                })
            })(),
        ))
    }

    async fn publish_response(
        &self,
        r#type: &str,
        request: &Value,
        data: Value,
        error: Option<String>,
    ) {
        let response = utils::get_response(request, data, error.clone());
        self.context
            .mqtt
            .lock()
            .await
            .publish(
                &format!("bridge/response/device/{type}"),
                stringify(&response),
                Default::default(),
            )
            .await;
        if let Some(error) = error {
            self.context.logger.lock().unwrap().error(&error, "z2m");
        }
    }

    /// Upstream `onMQTTMessage` (`bind.ts:282-425`).
    async fn on_mqtt_message(self: &Arc<Self>, data: Value) {
        let Some((raw, parsed)) = self.parse(&data) else {
            return;
        };
        let parsed = match parsed {
            Ok(parsed) => parsed,
            Err(error) => {
                let r#type = data
                    .get("topic")
                    .and_then(Value::as_str)
                    .map(|topic| {
                        if topic.ends_with("unbind") {
                            "unbind"
                        } else {
                            "bind"
                        }
                    })
                    .unwrap_or("bind");
                self.publish_response(r#type, &raw, json!({}), Some(error))
                    .await;
                return;
            }
        };

        if parsed.r#type == "binds/clear" {
            self.on_binds_clear(&parsed).await;
            return;
        }

        let source = parsed.resolved_source.clone().expect("parsed");
        let source_endpoint = parsed.resolved_source_endpoint.clone().expect("parsed");
        let bind_target = parsed.resolved_bind_target.clone().expect("parsed");
        let target_name = if parsed.default_group {
            DEFAULT_BIND_GROUP_NAME.to_string()
        } else if let Some(group) = &parsed.resolved_target_group {
            group.name()
        } else if let Some(device) = &parsed.resolved_target_device {
            device.name()
        } else {
            String::new()
        };

        let mut successful = Vec::new();
        let mut failed = Vec::new();
        let mut attempted = Vec::new();
        let candidates = parsed.clusters.clone().unwrap_or_else(|| {
            ALL_CLUSTER_CANDIDATES
                .iter()
                .map(|c| c.to_string())
                .collect()
        });

        let any_cluster_valid = parsed.default_group
            || parsed.resolved_target_group.is_some()
            || parsed
                .resolved_target_device
                .as_ref()
                .and_then(|device| device.zh_device())
                .is_some_and(|zh| zh.r#type() == DeviceType::Coordinator);

        for cluster in candidates {
            let cluster_value = Value::String(cluster.clone());
            let mut matching_clusters = false;
            if !any_cluster_valid {
                if let BindTarget::Endpoint(endpoint) = &bind_target {
                    matching_clusters = (endpoint.supports_input_cluster(cluster_value.clone())
                        && source_endpoint.supports_output_cluster(cluster_value.clone()))
                        || (source_endpoint.supports_input_cluster(cluster_value.clone())
                            && endpoint.supports_output_cluster(cluster_value.clone()));
                }
            }

            let source_valid = source_endpoint.supports_input_cluster(cluster_value.clone())
                || source_endpoint.supports_output_cluster(cluster_value.clone());

            if source_valid && (any_cluster_valid || matching_clusters) {
                let verb = if parsed.r#type == "bind" {
                    "bind"
                } else {
                    "unbind"
                };
                self.context.logger.lock().unwrap().debug(
                    &format!(
                        "{verb}ing cluster '{cluster}' from '{}' to '{target_name}'",
                        source.name()
                    ),
                    "z2m",
                );
                attempted.push(cluster.clone());

                let result = if parsed.r#type == "bind" {
                    source_endpoint
                        .bind(cluster_value.clone(), bind_target.clone())
                        .await
                } else {
                    source_endpoint
                        .unbind(cluster_value.clone(), bind_target.clone(), false)
                        .await
                };

                match result {
                    Ok(()) => {
                        successful.push(cluster.clone());
                        let past = if parsed.r#type == "bind" {
                            "bound"
                        } else {
                            "unbound"
                        };
                        self.context.logger.lock().unwrap().info(
                            &format!(
                                "Successfully {past} cluster '{cluster}' from '{}' to '{target_name}'",
                                source.name()
                            ),
                            "z2m",
                        );
                    }
                    Err(error) => {
                        failed.push(cluster.clone());
                        self.context.logger.lock().unwrap().error(
                            &format!(
                                "Failed to {} cluster '{cluster}' from '{}' to '{target_name}' ({error})",
                                parsed.r#type,
                                source.name()
                            ),
                            "z2m",
                        );
                    }
                }
            }
        }

        if attempted.is_empty() {
            self.context.logger.lock().unwrap().error(
                &format!(
                    "Nothing to {} from '{}' to '{target_name}'",
                    parsed.r#type,
                    source.name()
                ),
                "z2m",
            );
            self.publish_response(
                &parsed.r#type,
                &raw,
                json!({}),
                Some(format!("Nothing to {}", parsed.r#type)),
            )
            .await;
            return;
        }

        if failed.len() == attempted.len() {
            self.publish_response(
                &parsed.r#type,
                &raw,
                json!({}),
                Some(format!("Failed to {}", parsed.r#type)),
            )
            .await;
            return;
        }

        let response_data = json!({
            "from": parsed.source_key,
            "from_endpoint": parsed.source_endpoint_key,
            "to": parsed.target_key,
            "to_endpoint": parsed.target_endpoint_key,
            "clusters": successful,
            "failed": failed,
        });

        if !successful.is_empty() {
            if parsed.r#type == "bind" {
                let binds = source_endpoint
                    .binds()
                    .into_iter()
                    .filter(|bind| {
                        successful.contains(&bind.cluster.name)
                            && same_target(&bind.target, &bind_target)
                    })
                    .collect();
                self.setup_reporting(binds).await;
            } else if !parsed.skip_disable_reporting {
                if let Some(group) = &parsed.resolved_target_group {
                    self.disable_unnecessary_reportings_group(group.zh.clone())
                        .await;
                } else if let BindTarget::Endpoint(endpoint) = &bind_target {
                    self.disable_unnecessary_reportings_endpoint(endpoint.clone())
                        .await;
                }
            }
        }

        self.publish_response(&parsed.r#type, &raw, response_data, None)
            .await;
        self.context
            .event_bus
            .lock()
            .unwrap()
            .emit_devices_changed();
    }

    async fn on_binds_clear(&self, parsed: &ParsedMessage) {
        let request = &parsed.raw;
        let target = request.get("target").and_then(Value::as_str);
        let Some(target) = target else {
            self.publish_response(
                "binds/clear",
                request,
                json!({}),
                Some("Invalid payload".to_string()),
            )
            .await;
            return;
        };
        let device = self
            .context
            .zigbee
            .resolve_entity(target)
            .and_then(|entity| entity.zh_device())
            .and_then(|zh| self.context.zigbee.resolve_device(&zh.ieee_addr()));
        let Some(device) = device else {
            self.publish_response(
                "binds/clear",
                request,
                json!({}),
                Some("Invalid target".to_string()),
            )
            .await;
            return;
        };

        let eui64_list = request
            .get("ieee_list")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec!["0xffffffffffffffff".to_string()]);

        if let Err(error) = device.zh.clear_all_bindings(eui64_list.clone()).await {
            self.publish_response("binds/clear", request, json!({}), Some(error))
                .await;
            return;
        }

        self.publish_response(
            "binds/clear",
            request,
            json!({"target": target, "ieee_list": eui64_list}),
            None,
        )
        .await;
        self.context
            .event_bus
            .lock()
            .unwrap()
            .emit_devices_changed();
    }

    fn group_endpoints(group: &ZhGroup) -> Vec<Arc<Endpoint>> {
        group.members()
    }

    /// Upstream `setupReporting` (`bind.ts:484-522`).
    async fn setup_reporting(
        self: &Arc<Self>,
        binds: Vec<zigbee_apiarist::controller::model::endpoint::Bind>,
    ) {
        let Some(coordinator) = self.context.zigbee.first_coordinator_endpoint() else {
            return;
        };
        let configs = report_configs();

        for bind in binds {
            let endpoint_list: Vec<Arc<Endpoint>> = match &bind.target {
                BindTarget::Endpoint(endpoint) => vec![endpoint.clone()],
                BindTarget::Group(group) => Self::group_endpoints(group),
                BindTarget::GroupId(_) => Vec::new(),
            };

            for endpoint in endpoint_list {
                let cluster_value = Value::String(bind.cluster.name.clone());
                if !endpoint.supports_input_cluster(cluster_value.clone()) {
                    continue;
                }
                let configured = endpoint
                    .configured_reportings()
                    .iter()
                    .any(|reporting| reporting.cluster.name == bind.cluster.name);
                if configured {
                    let has_bind = endpoint.binds().iter().any(|candidate| {
                        candidate.cluster.name == bind.cluster.name
                            && same_target(
                                &candidate.target,
                                &BindTarget::Endpoint(coordinator.clone()),
                            )
                    });
                    if has_bind {
                        continue;
                    }
                }

                let device = endpoint.get_device();
                let name = self
                    .context
                    .zigbee
                    .resolve_device(&device.ieee_addr())
                    .map(|device| device.name())
                    .unwrap_or_else(|| device.ieee_addr());
                let entity = format!("{name}/{}", endpoint.ID);

                if let Err(error) = endpoint
                    .bind(
                        cluster_value.clone(),
                        BindTarget::Endpoint(coordinator.clone()),
                    )
                    .await
                {
                    self.context.logger.lock().unwrap().warning(
                        &format!(
                            "Failed to setup reporting for '{entity}' cluster '{}' ({error})",
                            bind.cluster.name
                        ),
                        "z2m",
                    );
                    continue;
                }

                let capabilities = self.color_capabilities(endpoint.clone()).await;
                let mut items = Vec::new();
                for config in configs
                    .iter()
                    .filter(|config| config.cluster == bind.cluster.name)
                {
                    let required = match config.requires {
                        Some("colorTemperature") => capabilities.0,
                        Some("colorXY") => capabilities.1,
                        _ => true,
                    };
                    if required {
                        items.push(ConfigureReportingItem {
                            attribute: json!(config.attribute),
                            minimum_report_interval: config.minimum,
                            maximum_report_interval: config.maximum,
                            reportable_change: Some(config.change),
                        });
                    }
                }

                match endpoint
                    .configure_reporting(cluster_value, items, None)
                    .await
                {
                    Ok(()) => self.context.logger.lock().unwrap().info(
                        &format!(
                            "Successfully setup reporting for '{entity}' cluster '{}'",
                            bind.cluster.name
                        ),
                        "z2m",
                    ),
                    Err(error) => self.context.logger.lock().unwrap().warning(
                        &format!(
                            "Failed to setup reporting for '{entity}' cluster '{}' ({error})",
                            bind.cluster.name
                        ),
                        "z2m",
                    ),
                }
            }
        }

        self.context
            .event_bus
            .lock()
            .unwrap()
            .emit_devices_changed();
    }

    async fn color_capabilities(&self, endpoint: Arc<Endpoint>) -> (bool, bool) {
        let mut value = endpoint
            .get_cluster_attribute_value(json!("lightingColorCtrl"), json!("colorCapabilities"))
            .and_then(|value| value.as_u64());
        if value.is_none() {
            let _ = endpoint
                .read(
                    json!("lightingColorCtrl"),
                    json!(["colorCapabilities"]),
                    None,
                )
                .await;
            value = endpoint
                .get_cluster_attribute_value(json!("lightingColorCtrl"), json!("colorCapabilities"))
                .and_then(|value| value.as_u64());
        }
        let value = value.unwrap_or(0);
        ((value & (1 << 4)) > 0, (value & (1 << 3)) > 0)
    }

    /// Upstream `disableUnnecessaryReportings` (`bind.ts:524-581`).
    async fn disable_unnecessary_reportings_group(self: &Arc<Self>, group: Arc<ZhGroup>) {
        self.disable_unnecessary_reportings(Self::group_endpoints(&group))
            .await;
    }

    async fn disable_unnecessary_reportings_endpoint(self: &Arc<Self>, endpoint: Arc<Endpoint>) {
        self.disable_unnecessary_reportings(vec![endpoint]).await;
    }

    async fn disable_unnecessary_reportings(self: &Arc<Self>, endpoints: Vec<Arc<Endpoint>>) {
        let Some(coordinator) = self.context.zigbee.first_coordinator_endpoint() else {
            return;
        };
        let configs = report_configs();

        let mut all_binds = Vec::new();
        for device in self
            .context
            .zigbee
            .devices_iterator(utils::device_not_coordinator)
        {
            for endpoint in device.zh.endpoints() {
                all_binds.extend(endpoint.binds());
            }
        }

        for endpoint in endpoints {
            let device = endpoint.get_device();
            let name = self
                .context
                .zigbee
                .resolve_device(&device.ieee_addr())
                .map(|device| device.name())
                .unwrap_or_else(|| device.ieee_addr());
            let entity = format!("{name}/{}", endpoint.ID);
            let mut required = Vec::new();
            for bind in &all_binds {
                let matches = match &bind.target {
                    BindTarget::Endpoint(target) => Arc::ptr_eq(target, &endpoint),
                    BindTarget::Group(group) => group
                        .members()
                        .iter()
                        .any(|member| Arc::ptr_eq(member, &endpoint)),
                    BindTarget::GroupId(_) => false,
                };
                if matches {
                    required.push(bind.cluster.name.clone());
                }
            }

            let mut bound = Vec::new();
            for bind in endpoint.binds() {
                if same_target(&bind.target, &BindTarget::Endpoint(coordinator.clone()))
                    && !required.contains(&bind.cluster.name)
                    && configs
                        .iter()
                        .any(|config| config.cluster == bind.cluster.name)
                {
                    bound.push(bind.cluster.name.clone());
                }
            }

            for cluster in bound {
                if let Err(error) = endpoint
                    .unbind(
                        Value::String(cluster.clone()),
                        BindTarget::Endpoint(coordinator.clone()),
                        false,
                    )
                    .await
                {
                    self.context.logger.lock().unwrap().warning(
                        &format!(
                            "Failed to disable reporting for '{entity}' cluster '{cluster}' ({error})"
                        ),
                        "z2m",
                    );
                    continue;
                }

                let capabilities = self.color_capabilities(endpoint.clone()).await;
                let mut items = Vec::new();
                for config in configs.iter().filter(|config| config.cluster == cluster) {
                    let required = match config.requires {
                        Some("colorTemperature") => capabilities.0,
                        Some("colorXY") => capabilities.1,
                        _ => true,
                    };
                    if required {
                        items.push(ConfigureReportingItem {
                            attribute: json!(config.attribute),
                            minimum_report_interval: config.minimum,
                            maximum_report_interval: 0xffff,
                            reportable_change: Some(config.change),
                        });
                    }
                }

                match endpoint
                    .configure_reporting(Value::String(cluster.clone()), items, None)
                    .await
                {
                    Ok(()) => self.context.logger.lock().unwrap().info(
                        &format!(
                            "Successfully disabled reporting for '{entity}' cluster '{cluster}'"
                        ),
                        "z2m",
                    ),
                    Err(error) => self.context.logger.lock().unwrap().warning(
                        &format!(
                            "Failed to disable reporting for '{entity}' cluster '{cluster}' ({error})"
                        ),
                        "z2m",
                    ),
                }
            }

            self.context
                .event_bus
                .lock()
                .unwrap()
                .emit_reconfigure(json!({"device": {"ieeeAddr": device.ieee_addr()}}));
        }
    }

    /// Upstream `poll` (`bind.ts:583-657`).
    async fn poll(self: &Arc<Self>, data: Value) {
        let cluster = data.get("cluster").and_then(Value::as_str).unwrap_or("");
        let r#type = data.get("type").and_then(Value::as_str).unwrap_or("");
        let payload = data.get("data").cloned().unwrap_or(Value::Null);

        let polls: Vec<PollRule> = poll_rules()
            .into_iter()
            .filter(|rule| {
                rule.cluster == cluster
                    && rule.r#type == r#type
                    && equals_partial(&payload, &rule.data)
            })
            .collect();
        if polls.is_empty() {
            return;
        }

        let mut to_poll: Vec<Arc<Endpoint>> = Vec::new();
        for device in self
            .context
            .zigbee
            .devices_iterator(utils::device_not_coordinator)
        {
            for endpoint in device.zh.endpoints() {
                for bind in endpoint.binds() {
                    if let BindTarget::Endpoint(target) = &bind.target {
                        if target.get_device().r#type() != DeviceType::Coordinator {
                            to_poll.push(target.clone());
                        }
                    }
                }
            }
        }

        let group_id = data.get("groupID").and_then(Value::as_u64).unwrap_or(0);
        if group_id != 0 {
            if let Some(group) = self.context.zigbee.group_by_id(group_id) {
                to_poll.extend(Self::group_endpoints(&group.zh));
            }
        }

        let mut seen = HashSet::new();
        to_poll.retain(|endpoint| {
            seen.insert((
                endpoint.device_ieee_address.lock().unwrap().clone(),
                endpoint.ID,
            ))
        });

        for endpoint in to_poll {
            let device = endpoint.get_device();
            for (index, poll) in polls.iter().enumerate() {
                let manufacturer_match = POLL_MANUFACTURER_IDS
                    .contains(&device.manufacturer_id().unwrap_or(0))
                    || device
                        .manufacturer_name()
                        .as_deref()
                        .is_some_and(|name| POLL_MANUFACTURER_NAMES.contains(&name));
                if !manufacturer_match
                    || !endpoint
                        .supports_input_cluster(Value::String(poll.read_cluster.to_string()))
                {
                    continue;
                }

                let mut attributes: Vec<String> = poll
                    .attributes
                    .iter()
                    .map(|attr| attr.to_string())
                    .collect();
                if poll.color_attributes {
                    let (color_temperature, color_xy) =
                        self.color_capabilities(endpoint.clone()).await;
                    if color_xy {
                        attributes.push("currentX".to_string());
                        attributes.push("currentY".to_string());
                    }
                    if color_temperature {
                        attributes.push("colorTemperature".to_string());
                    }
                }

                let key = format!("{}_{}_{index}", device.ieee_addr(), endpoint.ID);
                let generation = {
                    let mut debouncers = self.poll_debouncers.lock().unwrap();
                    let generation = debouncers.get(&key).copied().unwrap_or(0) + 1;
                    debouncers.insert(key.clone(), generation);
                    generation
                };

                let this = self.clone();
                let endpoint = endpoint.clone();
                let attributes = attributes.clone();
                let read_cluster = poll.read_cluster.to_string();
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                    if this.poll_debouncers.lock().unwrap().get(&key).copied() != Some(generation) {
                        return;
                    }
                    this.poll_debouncers.lock().unwrap().remove(&key);
                    if let Err(error) = endpoint
                        .read(Value::String(read_cluster), json!(attributes.clone()), None)
                        .await
                    {
                        let name = this
                            .context
                            .zigbee
                            .resolve_device(&endpoint.get_device().ieee_addr())
                            .map(|device| device.name())
                            .unwrap_or_else(|| endpoint.get_device().ieee_addr());
                        this.context.logger.lock().unwrap().error(
                            &format!("Failed to poll {attributes:?} from {name} ({error})"),
                            "z2m",
                        );
                    }
                });
            }
        }
    }

    /// Upstream `onGroupMembersChanged` (`bind.ts:441-462`).
    async fn on_group_members_changed(self: &Arc<Self>, data: Value) {
        if data.get("action").and_then(Value::as_str) == Some("add") {
            let group_id = data
                .get("group")
                .and_then(|group| group.get("ID"))
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let Some(group) = self.context.zigbee.group_by_id(group_id) else {
                return;
            };

            let mut binds = Vec::new();
            for device in self
                .context
                .zigbee
                .devices_iterator(utils::device_not_coordinator)
            {
                for endpoint in device.zh.endpoints() {
                    for bind in endpoint.binds() {
                        if same_target(&bind.target, &BindTarget::Group(group.zh.clone())) {
                            binds.push(bind);
                        }
                    }
                }
            }
            self.setup_reporting(binds).await;
        } else if data.get("skipDisableReporting").and_then(Value::as_bool) != Some(true) {
            if let Some(endpoint_id) = data
                .get("endpoint")
                .and_then(|endpoint| endpoint.get("ID"))
                .and_then(Value::as_u64)
            {
                let ieee = data
                    .get("endpoint")
                    .and_then(|endpoint| endpoint.get("deviceIeeeAddress"))
                    .and_then(Value::as_str);
                if let Some(ieee) = ieee {
                    if let Some(device) = self.context.zigbee.resolve_device(ieee) {
                        if let Some(endpoint) = device.zh.get_endpoint(endpoint_id as u8) {
                            self.disable_unnecessary_reportings_endpoint(endpoint).await;
                        }
                    }
                }
            }
        }
    }
}

fn same_target(left: &BindTarget, right: &BindTarget) -> bool {
    match (left, right) {
        (BindTarget::Endpoint(a), BindTarget::Endpoint(b)) => Arc::ptr_eq(a, b),
        (BindTarget::Group(a), BindTarget::Group(b)) => Arc::ptr_eq(a, b),
        (BindTarget::GroupId(a), BindTarget::GroupId(b)) => a == b,
        _ => false,
    }
}

/// Upstream `utils.equalsPartial(value, expected)`.
fn equals_partial(value: &Value, expected: &Value) -> bool {
    match (value, expected) {
        (Value::Object(value), Value::Object(expected)) => {
            expected.iter().all(|(key, expected)| {
                value
                    .get(key)
                    .is_some_and(|value| equals_partial(value, expected))
            })
        }
        (Value::Array(value), Value::Array(expected)) => {
            value.len() == expected.len()
                && value
                    .iter()
                    .zip(expected)
                    .all(|(value, expected)| equals_partial(value, expected))
        }
        _ => value == expected,
    }
}

impl Extension for Bind {
    fn name(&self) -> &'static str {
        "Bind"
    }

    fn context(&self) -> &ExtensionContext {
        &self.context
    }

    fn start(self: Arc<Self>) -> ExtensionFuture {
        Box::pin(async move {
            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_device_message(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move { this.poll(data).await });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_mqtt_message(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move { this.on_mqtt_message(data).await });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_group_members_changed(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move { this.on_group_members_changed(data).await });
                }),
            );

            Ok(())
        })
    }
}
