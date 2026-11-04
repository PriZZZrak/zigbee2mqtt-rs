/// Home Assistant MQTT auto-discovery message generation.
/// Publishes discovery configs to `homeassistant/<component>/<node_id>/<object_id>/config`
/// matching the exact format zigbee2mqtt uses for HA compatibility.
use serde_json::{json, Value};
use tracing::warn;

use crate::devices::Device;
use crate::mqtt::MqttBridge;

async fn publish(mqtt: &MqttBridge, component: &str, ieee: &str, object_id: &str, config: &Value) {
    if let Err(e) = mqtt
        .publish_ha_discovery(component, ieee, object_id, config)
        .await
    {
        warn!("HA discovery error for {component}/{ieee}/{object_id}: {e}");
    }
}

/// Publish all HA discovery messages for a device.
pub async fn publish_discovery(
    mqtt: &MqttBridge,
    device: &Device,
    base_topic: &str,
    coordinator_ieee: &str,
) {
    let ieee_hex = device.ieee_addr.as_hex();
    let ha_device = device_block(device, base_topic, coordinator_ieee);
    let availability = availability_block(base_topic);
    let clusters = device.all_input_clusters();

    let has_on_off = clusters.contains(&0x0006);
    let has_level = clusters.contains(&0x0008);
    let has_color = clusters.contains(&0x0300);

    // ── Light or Switch ───────────────────────────────────────────────────
    if has_on_off {
        if has_level || has_color {
            let mut config = json!({
                "availability": availability,
                "brightness": has_level,
                "brightness_scale": 254,
                "command_topic": format!("{}/{}/set", base_topic, device.friendly_name),
                "state_topic": format!("{}/{}", base_topic, device.friendly_name),
                "schema": "json",
                "object_id": &device.friendly_name,
                "unique_id": format!("{ieee_hex}_light_{base_topic}"),
                "device": ha_device,
            });
            if has_color {
                // Note: no "color_mode" discovery key here -- that's not part
                // of HA's MQTT Light JSON schema (real z2m never sets it
                // either); only supported_color_modes is a discovery-time
                // config key, "color_mode" is a *state*-payload field.
                config["supported_color_modes"] = json!(["color_temp", "xy"]);
            } else if has_level {
                config["supported_color_modes"] = json!(["brightness"]);
            }
            publish(mqtt, "light", &ieee_hex, "light", &config).await;
        } else {
            let config = json!({
                "availability": availability,
                "command_topic": format!("{}/{}/set", base_topic, device.friendly_name),
                "state_topic": format!("{}/{}", base_topic, device.friendly_name),
                "object_id": &device.friendly_name,
                "unique_id": format!("{ieee_hex}_switch_{base_topic}"),
                "device": ha_device,
                "value_template": "{{ value_json.state }}",
                "payload_on": "ON",
                "payload_off": "OFF",
            });
            publish(mqtt, "switch", &ieee_hex, "switch", &config).await;
        }
    }

    // ── Climate (thermostat) ─────────────────────────────────────────────
    if clusters.contains(&0x0201) {
        let config = json!({
            "availability": availability,
            "current_temperature_topic": format!("{}/{}", base_topic, device.friendly_name),
            "current_temperature_template": "{{ value_json.local_temperature }}",
            "temperature_command_topic": format!("{}/{}/set", base_topic, device.friendly_name),
            "temperature_state_topic": format!("{}/{}", base_topic, device.friendly_name),
            "temperature_state_template": "{{ value_json.occupied_heating_setpoint }}",
            "mode_command_topic": format!("{}/{}/set", base_topic, device.friendly_name),
            "mode_state_topic": format!("{}/{}", base_topic, device.friendly_name),
            "mode_state_template": "{{ value_json.system_mode }}",
            "modes": ["off", "heat", "auto", "cool"],
            "action_topic": format!("{}/{}", base_topic, device.friendly_name),
            "action_template": "{{ value_json.running_state }}",
            "min_temp": 5,
            "max_temp": 40,
            "temp_step": 0.5,
            "temperature_unit": "C",
            "object_id": format!("{}_climate", device.friendly_name),
            "unique_id": format!("{ieee_hex}_climate_{base_topic}"),
            "device": ha_device,
        });
        publish(mqtt, "climate", &ieee_hex, "climate", &config).await;
    }

    // ── Current sensor ───────────────────────────────────────────────────
    if clusters.contains(&0x0B04) {
        let config = json!({
            "availability": availability,
            "state_topic": format!("{}/{}", base_topic, device.friendly_name),
            "object_id": format!("{}_current", device.friendly_name),
            "unique_id": format!("{ieee_hex}_current_{base_topic}"),
            "device": ha_device,
            "device_class": "current",
            "unit_of_measurement": "A",
            "state_class": "measurement",
            "value_template": "{{ value_json.current }}",
            "enabled_by_default": true,
        });
        publish(mqtt, "sensor", &ieee_hex, "current", &config).await;
    }

    // ── Power sensor ─────────────────────────────────────────────────────
    if clusters.contains(&0x0B04) {
        let config = json!({
            "availability": availability,
            "state_topic": format!("{}/{}", base_topic, device.friendly_name),
            "object_id": format!("{}_power", device.friendly_name),
            "unique_id": format!("{ieee_hex}_power_{base_topic}"),
            "device": ha_device,
            "device_class": "power",
            "unit_of_measurement": "W",
            "state_class": "measurement",
            "value_template": "{{ value_json.power }}",
            "enabled_by_default": true,
        });
        publish(mqtt, "sensor", &ieee_hex, "power", &config).await;
    }

    // ── Energy sensor ────────────────────────────────────────────────────
    if clusters.contains(&0x0702) {
        let config = json!({
            "availability": availability,
            "state_topic": format!("{}/{}", base_topic, device.friendly_name),
            "object_id": format!("{}_energy", device.friendly_name),
            "unique_id": format!("{ieee_hex}_energy_{base_topic}"),
            "device": ha_device,
            "device_class": "energy",
            "unit_of_measurement": "kWh",
            "state_class": "total_increasing",
            "value_template": "{{ value_json.energy }}",
            "enabled_by_default": true,
        });
        publish(mqtt, "sensor", &ieee_hex, "energy", &config).await;
    }

    // ── Fault reason sensor ──────────────────────────────────────────────
    if clusters.contains(&0x0201) {
        let config = json!({
            "availability": availability,
            "state_topic": format!("{}/{}", base_topic, device.friendly_name),
            "object_id": format!("{}_fault_reason", device.friendly_name),
            "unique_id": format!("{ieee_hex}_fault_reason_{base_topic}"),
            "device": ha_device,
            "icon": "mdi:alert-circle",
            "value_template": "{{ value_json.fault_reason }}",
            "enabled_by_default": true,
        });
        publish(mqtt, "sensor", &ieee_hex, "fault_reason", &config).await;
    }

    // ── Contact binary sensor (genBinaryInput) ───────────────────────────
    if clusters.contains(&0x000F) {
        let config = json!({
            "availability": availability,
            "state_topic": format!("{}/{}", base_topic, device.friendly_name),
            "object_id": format!("{}_contact", device.friendly_name),
            "unique_id": format!("{ieee_hex}_contact_{base_topic}"),
            "device": ha_device,
            "device_class": "door",
            "value_template": "{{ value_json.contact }}",
            "payload_on": "closed",
            "payload_off": "open",
            "enabled_by_default": true,
        });
        publish(mqtt, "binary_sensor", &ieee_hex, "contact", &config).await;
    }

    // ── Action sensor (genMultistateInput) ───────────────────────────────
    if clusters.contains(&0x0012) {
        let config = json!({
            "availability": availability,
            "state_topic": format!("{}/{}", base_topic, device.friendly_name),
            "object_id": format!("{}_action", device.friendly_name),
            "unique_id": format!("{ieee_hex}_action_{base_topic}"),
            "device": ha_device,
            "icon": "mdi:gesture-double-tap",
            "value_template": "{{ value_json.action }}",
            "enabled_by_default": true,
        });
        publish(mqtt, "sensor", &ieee_hex, "action", &config).await;
    }

    let sensor_ctx = SensorDiscoveryCtx {
        mqtt,
        device,
        base_topic,
        ieee_hex: &ieee_hex,
        ha_device: &ha_device,
        availability: &availability,
    };

    // ── Temperature sensor ────────────────────────────────────────────────
    if clusters.contains(&0x0402) {
        publish_sensor(&sensor_ctx, "temperature", "°C").await;
    }

    // ── Humidity sensor ───────────────────────────────────────────────────
    if clusters.contains(&0x0405) {
        publish_sensor(&sensor_ctx, "humidity", "%").await;
    }

    // ── Illuminance sensor ────────────────────────────────────────────────
    if clusters.contains(&0x0400) {
        publish_sensor(&sensor_ctx, "illuminance", "lx").await;
    }

    // ── Battery sensor ────────────────────────────────────────────────────
    if clusters.contains(&0x0001) {
        publish_sensor(&sensor_ctx, "battery", "%").await;
    }

    // ── Occupancy binary sensor ───────────────────────────────────────────
    if clusters.contains(&0x0406) {
        let config = json!({
            "availability": availability,
            "state_topic": format!("{}/{}", base_topic, device.friendly_name),
            "object_id": format!("{}_occupancy", device.friendly_name),
            "unique_id": format!("{ieee_hex}_occupancy_{base_topic}"),
            "device": ha_device,
            "device_class": "occupancy",
            "value_template": "{{ value_json.occupancy }}",
            "payload_on": true,
            "payload_off": false,
            "enabled_by_default": true,
        });
        publish(mqtt, "binary_sensor", &ieee_hex, "occupancy", &config).await;
    }

    // ── IAS Zone sensor (contact/occupancy/smoke/water_leak/...) ──────────
    if clusters.contains(&0x0500) {
        use crate::zigbee::zcl::clusters::ias_zone;
        let (field, payload_on, payload_off) = ias_zone::ha_binary_sensor_fields(device.zone_type);
        let device_class = ias_zone::ha_device_class(device.zone_type);
        let config = json!({
            "availability": availability,
            "state_topic": format!("{}/{}", base_topic, device.friendly_name),
            "object_id": format!("{}_{field}", device.friendly_name),
            "unique_id": format!("{ieee_hex}_{field}_{base_topic}"),
            "device": ha_device,
            "device_class": device_class,
            "value_template": format!("{{{{ value_json.{field} }}}}"),
            "payload_on": payload_on,
            "payload_off": payload_off,
            "enabled_by_default": true,
        });
        publish(mqtt, "binary_sensor", &ieee_hex, field, &config).await;
    }

    // ── Action sensor (remote-control button presses) ─────────────────────
    // Mirrors z2m's `homeassistant.legacy_action_sensor` option: a plain
    // sensor with an icon, no device_class/unit -- only published for
    // devices that can actually produce `action` values (i.e. ones with a
    // bound output cluster a button press would arrive on).
    let has_action_output = device
        .all_output_clusters()
        .iter()
        .any(|c| matches!(c, 0x0005 | 0x0006 | 0x0008));
    if has_action_output {
        let config = json!({
            "availability": availability,
            "state_topic": format!("{}/{}", base_topic, device.friendly_name),
            "object_id": format!("{}_action", device.friendly_name),
            "unique_id": format!("{ieee_hex}_action_{base_topic}"),
            "device": ha_device,
            "icon": "mdi:gesture-double-tap",
            "value_template": "{{ value_json.action }}",
            "enabled_by_default": true,
        });
        publish(mqtt, "sensor", &ieee_hex, "action", &config).await;
    }

    // ── Link quality diagnostic sensor ────────────────────────────────────
    {
        let config = json!({
            "availability": availability,
            "state_topic": format!("{}/{}", base_topic, device.friendly_name),
            "object_id": format!("{}_linkquality", device.friendly_name),
            "unique_id": format!("{ieee_hex}_linkquality_{base_topic}"),
            "device": ha_device,
            "icon": "mdi:signal",
            "unit_of_measurement": "lqi",
            "state_class": "measurement",
            "value_template": "{{ value_json.linkquality }}",
            "entity_category": "diagnostic",
            "enabled_by_default": true,
        });
        publish(mqtt, "sensor", &ieee_hex, "linkquality", &config).await;
    }
}

/// Shared context for the numeric-sensor discovery configs below (temperature,
/// humidity, illuminance, battery), which otherwise differ only in the
/// device_class/unit_of_measurement pair.
struct SensorDiscoveryCtx<'a> {
    mqtt: &'a MqttBridge,
    device: &'a Device,
    base_topic: &'a str,
    ieee_hex: &'a str,
    ha_device: &'a Value,
    availability: &'a Value,
}

async fn publish_sensor(ctx: &SensorDiscoveryCtx<'_>, device_class: &str, unit: &str) {
    let SensorDiscoveryCtx {
        mqtt,
        device,
        base_topic,
        ieee_hex,
        ha_device,
        availability,
    } = *ctx;
    let config = json!({
        "availability": availability,
        "state_topic": format!("{}/{}", base_topic, device.friendly_name),
        "object_id": format!("{}_{device_class}", device.friendly_name),
        "unique_id": format!("{ieee_hex}_{device_class}_{base_topic}"),
        "device": ha_device,
        "device_class": device_class,
        "unit_of_measurement": unit,
        "state_class": "measurement",
        "value_template": format!("{{{{ value_json.{device_class} }}}}"),
        "enabled_by_default": true,
    });
    publish(mqtt, "sensor", ieee_hex, device_class, &config).await;
}

fn expose_property(expose: &Value) -> Option<&str> {
    expose
        .get("property")
        .and_then(|value| value.as_str())
        .or_else(|| expose.get("name").and_then(|value| value.as_str()))
}

fn payload_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        _ => String::new(),
    }
}

fn sensor_class(property: &str) -> Option<&'static str> {
    Some(match property {
        "temperature" => "temperature",
        "humidity" => "humidity",
        "illuminance" => "illuminance",
        "battery" => "battery",
        "voltage" => "voltage",
        "current" => "current",
        "power" => "power",
        "energy" => "energy",
        "carbon_dioxide" => "carbon_dioxide",
        "pm25" => "pm25",
        _ => return None,
    })
}

fn binary_class(property: &str) -> Option<&'static str> {
    Some(match property {
        "contact" => "door",
        "occupancy" | "motion" => "motion",
        "tamper" => "tamper",
        "smoke" => "smoke",
        "water_leak" => "moisture",
        _ => return None,
    })
}

/// Map prepared upstream exposes to HA discovery configs.
///
/// Returns `(component, object_id, config)` tuples for the expose kinds with a
/// verified simple mapping. Unknown/complex exposes (climate, cover, lock, fan,
/// vendor features) are skipped until their contracts are ported.
pub(crate) fn configs_from_exposes(
    device: &Device,
    exposes: &[Value],
    base_topic: &str,
    coordinator_ieee: &str,
) -> Vec<(String, String, Value)> {
    let ieee_hex = device.ieee_addr.as_hex();
    let ha_device = device_block(device, base_topic, coordinator_ieee);
    let availability = availability_block(base_topic);
    let state_topic = format!("{}/{}", base_topic, device.friendly_name);
    let command_topic = format!("{}/set", state_topic);
    let mut configs = Vec::new();
    for expose in exposes {
        let Some(property) = expose_property(expose) else {
            continue;
        };
        let kind = expose
            .get("type")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let access = expose
            .get("access")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        match kind {
            "numeric" => {
                let mut config = json!({
                    "availability": availability,
                    "state_topic": state_topic,
                    "object_id": property,
                    "unique_id": format!("{ieee_hex}_{property}_{base_topic}"),
                    "device": ha_device,
                    "value_template": format!("{{{{ value_json.{property} }}}}"),
                    "state_class": "measurement",
                    "enabled_by_default": true,
                });
                if let Some(unit) = expose.get("unit").and_then(|value| value.as_str()) {
                    config["unit_of_measurement"] = json!(unit);
                }
                if let Some(class) = sensor_class(property) {
                    config["device_class"] = json!(class);
                }
                configs.push(("sensor".to_string(), property.to_string(), config));
            }
            "binary" if access & 0x02 != 0 && property == "state" => {
                let on = expose
                    .get("value_on")
                    .map(payload_string)
                    .unwrap_or_else(|| "ON".into());
                let off = expose
                    .get("value_off")
                    .map(payload_string)
                    .unwrap_or_else(|| "OFF".into());
                configs.push((
                    "switch".to_string(),
                    property.to_string(),
                    json!({
                        "availability": availability,
                        "command_topic": command_topic,
                        "state_topic": state_topic,
                        "object_id": property,
                        "unique_id": format!("{ieee_hex}_{property}_{base_topic}"),
                        "device": ha_device,
                        "value_template": format!("{{{{ value_json.{property} }}}}"),
                        "payload_on": on,
                        "payload_off": off,
                    }),
                ));
            }
            "binary" => {
                let on = expose
                    .get("value_on")
                    .cloned()
                    .unwrap_or_else(|| json!("ON"));
                let off = expose
                    .get("value_off")
                    .cloned()
                    .unwrap_or_else(|| json!("OFF"));
                let mut config = json!({
                    "availability": availability,
                    "state_topic": state_topic,
                    "object_id": property,
                    "unique_id": format!("{ieee_hex}_{property}_{base_topic}"),
                    "device": ha_device,
                    "value_template": format!("{{{{ value_json.{property} }}}}"),
                    "payload_on": on,
                    "payload_off": off,
                    "enabled_by_default": true,
                });
                if let Some(class) = binary_class(property) {
                    config["device_class"] = json!(class);
                }
                configs.push(("binary_sensor".to_string(), property.to_string(), config));
            }
            "switch" => {
                let on = expose
                    .get("value_on")
                    .map(payload_string)
                    .unwrap_or_else(|| "ON".into());
                let off = expose
                    .get("value_off")
                    .map(payload_string)
                    .unwrap_or_else(|| "OFF".into());
                configs.push((
                    "switch".to_string(),
                    property.to_string(),
                    json!({
                        "availability": availability,
                        "command_topic": command_topic,
                        "state_topic": state_topic,
                        "object_id": property,
                        "unique_id": format!("{ieee_hex}_{property}_{base_topic}"),
                        "device": ha_device,
                        "value_template": format!("{{{{ value_json.{property} }}}}"),
                        "payload_on": on,
                        "payload_off": off,
                    }),
                ));
            }
            "light" => {
                let features = expose
                    .get("features")
                    .and_then(|value| value.as_array())
                    .cloned()
                    .unwrap_or_default();
                let has_brightness = features.iter().any(|feature| {
                    feature.get("name").and_then(|name| name.as_str()) == Some("brightness")
                });
                let has_color_temp = features.iter().any(|feature| {
                    matches!(
                        feature.get("name").and_then(|name| name.as_str()),
                        Some("color_temp") | Some("color_temperature")
                    )
                });
                let has_color = features.iter().any(|feature| {
                    matches!(
                        feature.get("name").and_then(|name| name.as_str()),
                        Some("color_xy") | Some("color_hs")
                    )
                });
                let mut config = json!({
                    "availability": availability,
                    "command_topic": command_topic,
                    "state_topic": state_topic,
                    "schema": "json",
                    "object_id": device.friendly_name,
                    "unique_id": format!("{ieee_hex}_light_{base_topic}"),
                    "device": ha_device,
                    "brightness": has_brightness,
                    "brightness_scale": 254,
                });
                let mut modes = Vec::new();
                if has_brightness {
                    modes.push("brightness");
                }
                if has_color_temp {
                    modes.push("color_temp");
                }
                if has_color {
                    modes.push("xy");
                }
                if !modes.is_empty() {
                    config["supported_color_modes"] = json!(modes);
                }
                configs.push(("light".to_string(), "light".to_string(), config));
            }
            _ => {}
        }
    }
    configs
}

/// Discovery topics for a device, using exposes when available and the same
/// cluster rules as the fallback publisher otherwise.
pub(crate) fn discovery_topics(device: &Device, exposes: &[Value]) -> Vec<(String, String)> {
    if !exposes.is_empty() {
        let mut topics: Vec<(String, String)> =
            configs_from_exposes(device, exposes, "zigbee2mqtt", "")
                .into_iter()
                .map(|(component, object_id, _)| (component, object_id))
                .collect();
        topics.sort();
        topics.dedup();
        return topics;
    }
    let clusters = device.all_input_clusters();
    let mut topics = Vec::new();
    let mut push = |component: &str, object_id: &str| {
        topics.push((component.to_string(), object_id.to_string()));
    };
    if clusters.contains(&0x0006) {
        if clusters.contains(&0x0008) || clusters.contains(&0x0300) {
            push("light", "light");
        } else {
            push("switch", "switch");
        }
    }
    if clusters.contains(&0x0201) {
        push("climate", "climate");
    }
    if clusters.contains(&0x0B04) {
        for field in ["current", "power", "energy", "fault_reason"] {
            push("sensor", field);
        }
    }
    if clusters.contains(&0x000F) {
        push("binary_sensor", "contact");
    }
    if clusters.contains(&0x0012) {
        push("sensor", "action");
    }
    if clusters.contains(&0x0500) {
        let (field, _, _) =
            crate::zigbee::zcl::clusters::ias_zone::ha_binary_sensor_fields(device.zone_type);
        push("binary_sensor", field);
    }
    for (cluster, field) in [
        (0x0402, "temperature"),
        (0x0405, "humidity"),
        (0x0400, "illuminance"),
        (0x0001, "battery"),
    ] {
        if clusters.contains(&cluster) {
            push("sensor", field);
        }
    }
    if clusters.contains(&0x0406) {
        push("binary_sensor", "occupancy");
    }
    let has_action_output = device
        .all_output_clusters()
        .iter()
        .any(|cluster| matches!(cluster, 0x0005 | 0x0006 | 0x0008));
    if has_action_output {
        push("sensor", "action");
    }
    push("sensor", "linkquality");
    topics.sort();
    topics.dedup();
    topics
}

/// Remove retained discovery configs for a device.
pub async fn cleanup_discovery(mqtt: &MqttBridge, device: &Device, exposes: &[Value]) {
    cleanup_with(device, exposes, |topic| async move {
        mqtt.clear_ha_topic(&topic).await
    })
    .await;
}

/// Cleanup loop with a pluggable publisher (used by broker-free tests).
pub(crate) async fn cleanup_with<F, Fut>(device: &Device, exposes: &[Value], mut clear: F)
where
    F: FnMut(String) -> Fut,
    Fut: std::future::Future<Output = crate::error::Result<()>>,
{
    let ieee = device.ieee_addr.as_hex();
    for (component, object_id) in discovery_topics(device, exposes) {
        let topic = crate::mqtt::ha_discovery_topic(&component, &ieee, &object_id);
        if let Err(error) = clear(topic.clone()).await {
            warn!("HA discovery cleanup error for {topic}: {error}");
        }
    }
}

/// Publish discovery derived from upstream exposes.
pub async fn publish_exposes_discovery(
    mqtt: &MqttBridge,
    device: &Device,
    exposes: &[Value],
    base_topic: &str,
    coordinator_ieee: &str,
) {
    for (component, object_id, config) in
        configs_from_exposes(device, exposes, base_topic, coordinator_ieee)
    {
        publish(
            mqtt,
            &component,
            &device.ieee_addr.as_hex(),
            &object_id,
            &config,
        )
        .await;
    }
}

fn device_block(device: &Device, _base_topic: &str, coordinator_ieee: &str) -> Value {
    let ieee_hex = device.ieee_addr.as_hex();
    // z2m hardcodes the "zigbee2mqtt" identifier prefix regardless of the
    // configured MQTT base_topic (only *groups* incorporate base_topic, which
    // this project doesn't support) -- HA matches devices across restarts/
    // migrations by `identifiers`, so using our own base_topic here would
    // make HA treat every device as new whenever base_topic isn't literally
    // "zigbee2mqtt", breaking migration from a real zigbee2mqtt install.
    json!({
        "identifiers": [format!("zigbee2mqtt_{ieee_hex}")],
        "name": device.friendly_name,
        "manufacturer": device.manufacturer.as_deref().unwrap_or("Unknown"),
        "model": device.model.as_deref().unwrap_or("Unknown"),
        "sw_version": device.sw_build_id,
        "via_device": format!("zigbee2mqtt_{coordinator_ieee}"),
    })
}

fn availability_block(base_topic: &str) -> Value {
    json!([{
        "topic": format!("{base_topic}/bridge/state"),
        "value_template": "{{ value_json.state }}"
    }])
}

// ── z2m-compatible test suite ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::devices::Device;
    use crate::zigbee::{EndpointDesc, IeeeAddr};

    const BASE: &str = "zigbee2mqtt";
    const COORD_IEEE: &str = "0x00124b00120144ae";

    fn make_light() -> Device {
        let mut dev = Device::new(
            IeeeAddr([0xb2, 0xa5, 0xc6, 0xfe, 0xff, 0x57, 0x0b, 0x00]),
            0x1234,
        );
        dev.friendly_name = "bulb".to_string();
        dev.manufacturer = Some("IKEA".to_string());
        dev.model = Some("TRADFRI bulb E26/E27".to_string());
        dev.interview_complete = true;
        dev.endpoints.push(EndpointDesc {
            endpoint: 1,
            profile_id: 0x0104,
            device_id: 0x0100,
            input_clusters: vec![0x0000, 0x0006, 0x0008],
            output_clusters: vec![],
        });
        dev
    }

    fn make_color_light() -> Device {
        let mut dev = make_light();
        dev.endpoints[0].input_clusters.extend_from_slice(&[0x0300]);
        dev
    }

    fn make_switch() -> Device {
        let mut dev = Device::new(
            IeeeAddr([0x42, 0x55, 0xe4, 0x04, 0x01, 0x88, 0x17, 0x00]),
            0x5678,
        );
        dev.friendly_name = "wall_switch".to_string();
        dev.interview_complete = true;
        dev.endpoints.push(EndpointDesc {
            endpoint: 1,
            profile_id: 0x0104,
            device_id: 0x0000,
            input_clusters: vec![0x0000, 0x0006],
            output_clusters: vec![],
        });
        dev
    }

    fn make_sensor() -> Device {
        let mut dev = Device::new(
            IeeeAddr([0x22, 0x55, 0xe4, 0x04, 0x01, 0x88, 0x17, 0x00]),
            0x9ABC,
        );
        dev.friendly_name = "weather_sensor".to_string();
        dev.manufacturer = Some("Xiaomi".to_string());
        dev.model = Some("WSDCGQ11LM".to_string());
        dev.power_source = Some("battery".to_string());
        dev.interview_complete = true;
        dev.endpoints.push(EndpointDesc {
            endpoint: 1,
            profile_id: 0x0104,
            device_id: 0x0302,
            input_clusters: vec![0x0000, 0x0001, 0x0402, 0x0405],
            output_clusters: vec![],
        });
        dev
    }

    fn make_contact_sensor() -> Device {
        let mut dev = Device::new(
            IeeeAddr([0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0x00]),
            0xDEF0,
        );
        dev.friendly_name = "door_sensor".to_string();
        dev.interview_complete = true;
        dev.endpoints.push(EndpointDesc {
            endpoint: 1,
            profile_id: 0x0104,
            device_id: 0x0402,
            input_clusters: vec![0x0000, 0x0001, 0x0500],
            output_clusters: vec![],
        });
        dev
    }

    fn make_occupancy_sensor() -> Device {
        let mut dev = Device::new(
            IeeeAddr([0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11]),
            0x4321,
        );
        dev.friendly_name = "motion_sensor".to_string();
        dev.interview_complete = true;
        dev.endpoints.push(EndpointDesc {
            endpoint: 1,
            profile_id: 0x0104,
            device_id: 0x0107,
            input_clusters: vec![0x0000, 0x0001, 0x0406, 0x0400],
            output_clusters: vec![],
        });
        dev
    }

    // ── Discovery topic format ────────────────────────────────────────────

    #[test]
    fn discovery_topic_format_matches_z2m() {
        // z2m: homeassistant/light/0x000b57fffec6a5b2/light/config
        let ieee = make_light().ieee_addr.as_hex();
        let topic = format!("homeassistant/light/{ieee}/light/config");
        assert!(topic.starts_with("homeassistant/light/0x"));
        assert!(topic.ends_with("/light/config"));
    }

    // ── Availability block (z2m format) ───────────────────────────────────

    #[test]
    fn availability_matches_z2m() {
        let avail = availability_block(BASE);
        let arr = avail.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["topic"], "zigbee2mqtt/bridge/state");
        assert_eq!(arr[0]["value_template"], "{{ value_json.state }}");
    }

    // ── Device block (z2m format) ─────────────────────────────────────────

    #[test]
    fn device_block_matches_z2m() {
        let dev = make_light();
        let block = device_block(&dev, BASE, COORD_IEEE);

        // z2m: identifiers = ["zigbee2mqtt_0x..."]
        let ids = block["identifiers"].as_array().unwrap();
        assert!(ids[0].as_str().unwrap().starts_with("zigbee2mqtt_0x"));

        assert_eq!(block["name"], "bulb");
        assert_eq!(block["manufacturer"], "IKEA");
        assert_eq!(block["model"], "TRADFRI bulb E26/E27");

        // z2m: via_device = "zigbee2mqtt_0x..." (same identifier scheme as
        // any device, keyed by the coordinator's own IEEE -- no "_bridge_"
        // infix)
        let via = block["via_device"].as_str().unwrap();
        assert!(via.starts_with("zigbee2mqtt_0x"));
    }

    #[test]
    fn device_block_identifiers_ignore_configured_base_topic() {
        // z2m hardcodes the "zigbee2mqtt" identifier prefix regardless of
        // the configured MQTT base_topic (only *groups* incorporate
        // base_topic, which this project doesn't support). HA matches
        // devices across restarts/migrations by `identifiers`, so this must
        // NOT vary with base_topic -- otherwise every device would look "new"
        // to HA whenever base_topic isn't literally "zigbee2mqtt" (e.g. this
        // project's own config.example.yaml sets "zigbee2mqtt-rs").
        let dev = make_light();
        let block = device_block(&dev, "zigbee2mqtt-rs", COORD_IEEE);

        let ids = block["identifiers"].as_array().unwrap();
        assert!(ids[0].as_str().unwrap().starts_with("zigbee2mqtt_0x"));
        assert!(!ids[0].as_str().unwrap().contains("zigbee2mqtt-rs"));

        let via = block["via_device"].as_str().unwrap();
        assert!(via.starts_with("zigbee2mqtt_0x"));
        assert!(!via.contains("zigbee2mqtt-rs"));
    }

    // ── Light discovery (z2m format) ──────────────────────────────────────

    #[test]
    fn light_discovery_has_required_z2m_fields() {
        let dev = make_light();
        let ieee = dev.ieee_addr.as_hex();
        let ha_dev = device_block(&dev, BASE, COORD_IEEE);
        let avail = availability_block(BASE);

        // Simulate what publish_discovery builds for a brightness light
        let config = json!({
            "availability": avail,
            "brightness": true,
            "brightness_scale": 254,
            "command_topic": format!("{BASE}/bulb/set"),
            "state_topic": format!("{BASE}/bulb"),
            "schema": "json",
            "object_id": "bulb",
            "unique_id": format!("{ieee}_light_{BASE}"),
            "device": ha_dev,
            "supported_color_modes": ["brightness"],
        });

        assert_eq!(config["schema"], "json");
        assert_eq!(config["brightness"], true);
        assert_eq!(config["brightness_scale"], 254);
        assert_eq!(config["command_topic"], "zigbee2mqtt/bulb/set");
        assert_eq!(config["state_topic"], "zigbee2mqtt/bulb");
        assert_eq!(config["object_id"], "bulb");

        // unique_id must end with _zigbee2mqtt (z2m convention)
        let uid = config["unique_id"].as_str().unwrap();
        assert!(uid.ends_with("_zigbee2mqtt"));
        assert!(uid.starts_with("0x"));

        // supported_color_modes present
        let modes = config["supported_color_modes"].as_array().unwrap();
        assert!(modes.contains(&json!("brightness")));
    }

    #[test]
    fn color_light_discovery_has_no_color_mode_key() {
        // "color_mode" isn't part of HA's MQTT Light JSON discovery schema
        // (it's a *state*-payload field) -- real z2m never sets it in the
        // discovery config, only `supported_color_modes`.
        let dev = make_color_light();
        let ieee = dev.ieee_addr.as_hex();
        let ha_dev = device_block(&dev, BASE, COORD_IEEE);
        let avail = availability_block(BASE);

        let mut config = json!({
            "availability": avail,
            "brightness": true,
            "brightness_scale": 254,
            "command_topic": format!("{BASE}/color_bulb/set"),
            "state_topic": format!("{BASE}/color_bulb"),
            "schema": "json",
            "object_id": "color_bulb",
            "unique_id": format!("{ieee}_light_{BASE}"),
            "device": ha_dev,
        });
        config["supported_color_modes"] = json!(["color_temp", "xy"]);

        assert!(config.get("color_mode").is_none());
        assert_eq!(config["supported_color_modes"], json!(["color_temp", "xy"]));
    }

    #[test]
    fn action_sensor_only_for_devices_with_action_capable_output_cluster() {
        // Ordinary bulb: no output clusters -> no action sensor.
        let bulb = make_light();
        assert!(!bulb
            .all_output_clusters()
            .iter()
            .any(|c| matches!(c, 0x0005 | 0x0006 | 0x0008)));

        // Remote control: genOnOff/genLevelCtrl/genScenes as *output*
        // clusters (bound to the coordinator so button presses arrive) ->
        // action sensor should be published.
        let mut remote = Device::new(
            IeeeAddr([0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]),
            0x1111,
        );
        remote.friendly_name = "remote".to_string();
        remote.endpoints.push(EndpointDesc {
            endpoint: 1,
            profile_id: 0x0104,
            device_id: 0x0810,
            input_clusters: vec![0x0000, 0x0001],
            output_clusters: vec![0x0006, 0x0008, 0x0005],
        });
        assert!(remote
            .all_output_clusters()
            .iter()
            .any(|c| matches!(c, 0x0005 | 0x0006 | 0x0008)));
    }

    #[test]
    fn color_light_discovery_has_color_modes() {
        let dev = make_color_light();
        let clusters = dev.all_input_clusters();
        assert!(clusters.contains(&0x0300));

        // Color lights should have color_temp and xy modes
        let modes = json!(["color_temp", "xy"]);
        assert!(modes.as_array().unwrap().contains(&json!("color_temp")));
        assert!(modes.as_array().unwrap().contains(&json!("xy")));
    }

    // ── Switch discovery (z2m format) ─────────────────────────────────────

    #[test]
    fn switch_discovery_matches_z2m() {
        let dev = make_switch();
        let ieee = dev.ieee_addr.as_hex();
        let ha_dev = device_block(&dev, BASE, COORD_IEEE);
        let avail = availability_block(BASE);

        let config = json!({
            "availability": avail,
            "command_topic": format!("{BASE}/wall_switch/set"),
            "state_topic": format!("{BASE}/wall_switch"),
            "object_id": "wall_switch",
            "unique_id": format!("{ieee}_switch_{BASE}"),
            "device": ha_dev,
            "value_template": "{{ value_json.state }}",
            "payload_on": "ON",
            "payload_off": "OFF",
        });

        assert_eq!(config["payload_on"], "ON");
        assert_eq!(config["payload_off"], "OFF");
        assert_eq!(config["value_template"], "{{ value_json.state }}");
        assert_eq!(config["command_topic"], "zigbee2mqtt/wall_switch/set");
    }

    // ── Sensor discovery (z2m format) ─────────────────────────────────────

    #[test]
    fn temperature_sensor_discovery_matches_z2m() {
        let dev = make_sensor();
        let ieee = dev.ieee_addr.as_hex();
        let ha_dev = device_block(&dev, BASE, COORD_IEEE);
        let avail = availability_block(BASE);

        let config = json!({
            "availability": avail,
            "state_topic": format!("{BASE}/weather_sensor"),
            "object_id": "weather_sensor_temperature",
            "unique_id": format!("{ieee}_temperature_{BASE}"),
            "device": ha_dev,
            "device_class": "temperature",
            "unit_of_measurement": "°C",
            "state_class": "measurement",
            "value_template": "{{ value_json.temperature }}",
            "enabled_by_default": true,
        });

        // z2m exact field checks
        assert_eq!(config["device_class"], "temperature");
        assert_eq!(config["unit_of_measurement"], "°C");
        assert_eq!(config["state_class"], "measurement");
        assert_eq!(config["value_template"], "{{ value_json.temperature }}");
        assert_eq!(config["enabled_by_default"], true);

        let uid = config["unique_id"].as_str().unwrap();
        assert!(uid.contains("_temperature_"));
        assert!(uid.ends_with("_zigbee2mqtt"));
    }

    #[test]
    fn humidity_sensor_discovery_matches_z2m() {
        let dev = make_sensor();
        let ieee = dev.ieee_addr.as_hex();
        let ha_dev = device_block(&dev, BASE, COORD_IEEE);
        let avail = availability_block(BASE);

        let config = json!({
            "availability": avail,
            "state_topic": format!("{BASE}/weather_sensor"),
            "object_id": "weather_sensor_humidity",
            "unique_id": format!("{ieee}_humidity_{BASE}"),
            "device": ha_dev,
            "device_class": "humidity",
            "unit_of_measurement": "%",
            "state_class": "measurement",
            "value_template": "{{ value_json.humidity }}",
            "enabled_by_default": true,
        });

        assert_eq!(config["device_class"], "humidity");
        assert_eq!(config["unit_of_measurement"], "%");
    }

    #[test]
    fn battery_sensor_discovery_matches_z2m() {
        let dev = make_sensor();
        let ieee = dev.ieee_addr.as_hex();

        let uid = format!("{ieee}_battery_{BASE}");
        assert!(uid.ends_with("_zigbee2mqtt"));
        assert!(uid.contains("_battery_"));
    }

    // ── Binary sensor discovery ───────────────────────────────────────────

    #[test]
    fn contact_sensor_discovery_matches_z2m() {
        let dev = make_contact_sensor();
        let ieee = dev.ieee_addr.as_hex();
        let ha_dev = device_block(&dev, BASE, COORD_IEEE);
        let avail = availability_block(BASE);

        let config = json!({
            "availability": avail,
            "state_topic": format!("{BASE}/door_sensor"),
            "object_id": "door_sensor_contact",
            "unique_id": format!("{ieee}_contact_{BASE}"),
            "device": ha_dev,
            "device_class": "door",
            "value_template": "{{ value_json.contact }}",
            "payload_on": false,
            "payload_off": true,
            "enabled_by_default": true,
        });

        assert_eq!(config["device_class"], "door");
        // contact=false means door open (payload_on=false)
        assert_eq!(config["payload_on"], false);
        assert_eq!(config["payload_off"], true);
    }

    #[test]
    fn ias_zone_unknown_type_still_falls_back_to_contact() {
        use crate::zigbee::zcl::clusters::ias_zone;
        let (field, payload_on, payload_off) = ias_zone::ha_binary_sensor_fields(None);
        assert_eq!(field, "contact");
        assert!(!payload_on);
        assert!(payload_off);
        assert_eq!(ias_zone::ha_device_class(None), "door");
    }

    #[test]
    fn ias_zone_motion_sensor_discovery_uses_occupancy_not_contact() {
        use crate::zigbee::zcl::clusters::ias_zone;
        let (field, payload_on, payload_off) = ias_zone::ha_binary_sensor_fields(Some(0x000D));
        assert_eq!(field, "occupancy");
        assert!(payload_on);
        assert!(!payload_off);
        assert_eq!(ias_zone::ha_device_class(Some(0x000D)), "motion");
    }

    #[test]
    fn ias_zone_water_sensor_discovery_uses_water_leak() {
        use crate::zigbee::zcl::clusters::ias_zone;
        let (field, ..) = ias_zone::ha_binary_sensor_fields(Some(0x002A));
        assert_eq!(field, "water_leak");
        assert_eq!(ias_zone::ha_device_class(Some(0x002A)), "moisture");
    }

    #[test]
    fn occupancy_sensor_discovery_matches_z2m() {
        let dev = make_occupancy_sensor();
        let ieee = dev.ieee_addr.as_hex();

        let uid = format!("{ieee}_occupancy_{BASE}");
        assert!(uid.ends_with("_zigbee2mqtt"));

        // occupancy uses boolean payload
        let config = json!({
            "device_class": "occupancy",
            "payload_on": true,
            "payload_off": false,
        });
        assert_eq!(config["device_class"], "occupancy");
        assert_eq!(config["payload_on"], true);
    }

    // ── Linkquality diagnostic sensor ─────────────────────────────────────

    #[test]
    fn linkquality_sensor_is_diagnostic() {
        let dev = make_light();
        let ieee = dev.ieee_addr.as_hex();

        let config = json!({
            "object_id": format!("{}_linkquality", dev.friendly_name),
            "unique_id": format!("{ieee}_linkquality_{BASE}"),
            "icon": "mdi:signal",
            "unit_of_measurement": "lqi",
            "state_class": "measurement",
            "value_template": "{{ value_json.linkquality }}",
            "entity_category": "diagnostic",
            "enabled_by_default": true,
        });

        assert_eq!(config["entity_category"], "diagnostic");
        assert_eq!(config["unit_of_measurement"], "lqi");
        assert_eq!(config["icon"], "mdi:signal");
        assert_eq!(config["value_template"], "{{ value_json.linkquality }}");
    }

    #[test]
    fn prepared_exposes_map_to_discovery_configs() {
        let dev = make_light();
        let exposes = vec![
            json!({"type": "numeric", "name": "temperature", "property": "temperature", "access": 5, "unit": "°C"}),
            json!({"type": "numeric", "name": "battery", "property": "battery", "access": 1, "unit": "%"}),
            json!({"type": "binary", "name": "contact", "property": "contact", "access": 1, "value_on": false, "value_off": true}),
            json!({"type": "switch", "name": "state", "property": "state", "access": 7, "value_on": "ON", "value_off": "OFF"}),
            json!({"type": "light", "name": "light", "features": [{"name": "brightness"}, {"name": "color_temp"}]}),
        ];
        let configs = configs_from_exposes(&dev, &exposes, BASE, COORD_IEEE);
        assert_eq!(configs.len(), 5);

        let temperature = configs
            .iter()
            .find(|(component, id, _)| component == "sensor" && id == "temperature")
            .map(|(_, _, config)| config)
            .expect("temperature sensor");
        assert_eq!(temperature["unit_of_measurement"], "°C");
        assert_eq!(temperature["device_class"], "temperature");
        assert_eq!(
            temperature["value_template"],
            "{{ value_json.temperature }}"
        );
        assert_eq!(
            temperature["unique_id"],
            format!("{}_temperature_{BASE}", dev.ieee_addr.as_hex())
        );

        let contact = configs
            .iter()
            .find(|(component, id, _)| component == "binary_sensor" && id == "contact")
            .map(|(_, _, config)| config)
            .expect("contact binary sensor");
        // Upstream binary_sensor keeps the raw value_on/value_off types.
        assert_eq!(contact["payload_on"], false);
        assert_eq!(contact["payload_off"], true);
        assert_eq!(contact["device_class"], "door");

        let switch = configs
            .iter()
            .find(|(component, id, _)| component == "switch" && id == "state")
            .map(|(_, _, config)| config)
            .expect("switch");
        // Switch payloads use strings, matching the existing Rust contract.
        assert_eq!(switch["payload_on"], "ON");
        assert_eq!(switch["payload_off"], "OFF");

        let light = configs
            .iter()
            .find(|(component, _, _)| component == "light")
            .map(|(_, _, config)| config)
            .expect("light");
        assert_eq!(light["schema"], "json");
        assert_eq!(light["brightness"], true);
        assert_eq!(
            light["supported_color_modes"],
            json!(["brightness", "color_temp"])
        );
    }

    #[test]
    fn function_exposes_output_uses_the_same_mapping() {
        let dev = make_light();
        // This is the shape produced by ExternalConverter::exposes when the
        // upstream exposes is a function of the device.
        let exposes = vec![json!({
            "type": "numeric",
            "name": "temperature",
            "property": "temperature",
            "access": 1,
            "unit": "°C"
        })];
        let configs = configs_from_exposes(&dev, &exposes, BASE, COORD_IEEE);
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].0, "sensor");
        assert_eq!(configs[0].1, "temperature");
    }

    #[test]
    fn discovery_cleanup_topics_follow_cluster_fallback() {
        let dev = make_light();
        let topics = discovery_topics(&dev, &[]);
        assert_eq!(
            topics,
            vec![
                ("light".to_string(), "light".to_string()),
                ("sensor".to_string(), "linkquality".to_string())
            ]
        );

        let mut sensor = make_light();
        sensor.friendly_name = "sensor".into();
        sensor.endpoints[0].input_clusters = vec![0x0001, 0x000F, 0x0402, 0x0405, 0x0406];
        let topics = discovery_topics(&sensor, &[]);
        assert!(topics.contains(&("sensor".to_string(), "temperature".to_string())));
        assert!(topics.contains(&("sensor".to_string(), "humidity".to_string())));
        assert!(topics.contains(&("sensor".to_string(), "battery".to_string())));
        assert!(topics.contains(&("binary_sensor".to_string(), "contact".to_string())));
        assert!(topics.contains(&("binary_sensor".to_string(), "occupancy".to_string())));
        assert!(topics.contains(&("sensor".to_string(), "linkquality".to_string())));
        let mut sorted = topics.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(topics, sorted, "topics must be sorted and unique");
    }

    #[test]
    fn discovery_cleanup_topics_follow_exposes() {
        let dev = make_light();
        let exposes = vec![
            json!({"type": "numeric", "name": "temperature", "property": "temperature", "access": 1, "unit": "°C"}),
            json!({"type": "binary", "name": "contact", "property": "contact", "access": 1, "value_on": false, "value_off": true}),
            json!({"type": "switch", "name": "state", "property": "state", "access": 7, "value_on": "ON", "value_off": "OFF"}),
            json!({"type": "light", "name": "light", "features": [{"name": "brightness"}]}),
        ];
        assert_eq!(
            discovery_topics(&dev, &exposes),
            vec![
                ("binary_sensor".to_string(), "contact".to_string()),
                ("light".to_string(), "light".to_string()),
                ("sensor".to_string(), "temperature".to_string()),
                ("switch".to_string(), "state".to_string()),
            ]
        );
    }

    #[test]
    fn discovery_cleanup_topic_and_payload_are_exact() {
        assert_eq!(
            crate::mqtt::ha_discovery_topic("sensor", "0xec1bbdfffeaa66db", "temperature"),
            "homeassistant/sensor/0xec1bbdfffeaa66db/temperature/config"
        );
        assert!(crate::mqtt::ha_clear_payload().is_empty());
    }

    #[tokio::test]
    async fn cleanup_publishes_empty_retained_configs_to_all_topics() {
        let dev = make_light();
        let exposures = Vec::new();
        let mut cleared = Vec::new();
        cleanup_with(&dev, &exposures, |topic| {
            cleared.push(topic);
            async { Ok(()) }
        })
        .await;
        assert_eq!(
            cleared,
            vec![
                format!(
                    "homeassistant/light/{}/light/config",
                    dev.ieee_addr.as_hex()
                ),
                format!(
                    "homeassistant/sensor/{}/linkquality/config",
                    dev.ieee_addr.as_hex()
                ),
            ]
        );
        assert!(crate::mqtt::ha_clear_payload().is_empty());
    }
}
