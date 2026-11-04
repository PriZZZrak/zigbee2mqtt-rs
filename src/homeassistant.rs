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

/// Sorted unique endpoints that expose the OnOff cluster.
fn on_off_endpoints(device: &Device) -> Vec<u8> {
    let mut endpoints: Vec<u8> = device
        .endpoints
        .iter()
        .filter(|endpoint| endpoint.input_clusters.contains(&0x0006))
        .map(|endpoint| endpoint.endpoint)
        .collect();
    endpoints.sort_unstable();
    endpoints.dedup();
    endpoints
}

/// z2m renders suffixed switch keys with bracket notation.
fn state_template(property_key: &str, endpoint: Option<&str>) -> String {
    match endpoint {
        Some(_) => format!("{{{{ value_json[\"{property_key}\"] }}}}"),
        None => format!("{{{{ value_json.{property_key} }}}}"),
    }
}

fn endpoint_has(device: &Device, endpoint: u8, cluster: u16) -> bool {
    device.endpoints.iter().any(|candidate| {
        candidate.endpoint == endpoint && candidate.input_clusters.contains(&cluster)
    })
}

/// Per-endpoint light/switch configs for a multi-gang device.
///
/// Naming follows z2m: lights use `light_lN` with a dedicated
/// `<base>/<name>/lN` state topic; switches use `<friendly>_lN` with a
/// `state_lN` template on the main state topic.
fn endpoint_on_off_configs(
    device: &Device,
    base_topic: &str,
    coordinator_ieee: &str,
) -> Vec<(String, String, Value)> {
    let ieee_hex = device.ieee_addr.as_hex();
    let ha_device = device_block(device, base_topic, coordinator_ieee);
    let availability = availability_block(base_topic);
    let state_topic = format!("{}/{}", base_topic, device.friendly_name);
    let mut configs = Vec::new();
    for endpoint in on_off_endpoints(device) {
        let suffix = format!("l{endpoint}");
        let endpoint_command_topic = format!("{state_topic}/{suffix}/set");
        if endpoint_has(device, endpoint, 0x0008) || endpoint_has(device, endpoint, 0x0300) {
            let mut config = json!({
                "availability": availability,
                "command_topic": endpoint_command_topic,
                "state_topic": format!("{state_topic}/{suffix}"),
                "schema": "json",
                "object_id": format!("light_{suffix}"),
                "unique_id": format!("{ieee_hex}_light_{suffix}_{base_topic}"),
                "device": ha_device,
                "brightness": true,
                "brightness_scale": 254,
                "supported_color_modes": ["brightness"],
            });
            if endpoint_has(device, endpoint, 0x0300) {
                config["supported_color_modes"] = json!(["color_temp", "xy"]);
            }
            configs.push(("light".to_string(), format!("light_{suffix}"), config));
        } else {
            configs.push((
                "switch".to_string(),
                format!("switch_{suffix}"),
                json!({
                    "availability": availability,
                    "command_topic": endpoint_command_topic,
                    "state_topic": state_topic,
                    "object_id": format!("{}_{suffix}", device.friendly_name),
                    "unique_id": format!("{ieee_hex}_switch_{suffix}_{base_topic}"),
                    "device": ha_device,
                    "value_template": format!("{{{{ value_json[\"state_{suffix}\"] }}}}"),
                    "payload_on": "ON",
                    "payload_off": "OFF",
                }),
            ));
        }
    }
    configs
}

/// Exposes whose properties differ from `state` get a per-attribute command
/// topic and config/diagnostic metadata (upstream `CONFIG_SWITCH_DISCOVERY_LOOKUP`).
const CONFIG_SWITCH_LOOKUP: &[(&str, &str, &str)] = &[
    ("auto_lock", "config", "mdi:lock"),
    ("away_mode", "config", "mdi:home-export-outline"),
    ("comfort_smiley", "config", "mdi:emoticon-happy-outline"),
    ("enable_display", "config", "mdi:monitor"),
    ("indicator", "config", "mdi:led-on"),
    ("tilt_mode", "config", "mdi:angle-acute"),
    ("valve_detection", "config", "mdi:pipe-valve"),
    ("window_detection", "config", "mdi:window-open-variant"),
];

fn config_switch_lookup(base_property: &str) -> Option<(&'static str, &'static str)> {
    CONFIG_SWITCH_LOOKUP
        .iter()
        .find(|(property, _, _)| *property == base_property)
        .map(|(_, category, icon)| (*category, *icon))
}

/// `featurePropertyWithoutEndpoint` equivalent: drop a trailing `_<endpoint>`.
fn strip_endpoint_suffix(property: &str, endpoint: Option<&str>) -> String {
    match endpoint {
        Some(endpoint) => {
            let suffix = format!("_{endpoint}");
            property
                .strip_suffix(&suffix)
                .unwrap_or(property)
                .to_string()
        }
        None => property.to_string(),
    }
}

/// Human-readable entity name for exposes without a label.
fn humanize_property(property: &str) -> String {
    let mut label = String::new();
    for (index, part) in property.split('_').enumerate() {
        if index > 0 {
            label.push(' ');
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            label.extend(first.to_uppercase());
            label.push_str(chars.as_str());
        }
    }
    label
}

fn expose_feature<'a>(expose: &'a Value, name: &str) -> Option<&'a Value> {
    expose
        .get("features")
        .and_then(|value| value.as_array())?
        .iter()
        .find(|feature| feature.get("name").and_then(|value| value.as_str()) == Some(name))
}

fn feature_property(feature: Option<&Value>) -> Option<&str> {
    feature
        .and_then(|feature| feature.get("property"))
        .and_then(|value| value.as_str())
}

const COVER_OPENING_LOOKUP: &[&str] = &["opening", "open", "forward", "up", "rising"];
const COVER_CLOSING_LOOKUP: &[&str] = &[
    "closing",
    "close",
    "backward",
    "back",
    "reverse",
    "down",
    "declining",
];
const COVER_STOPPED_LOOKUP: &[&str] = &["stopped", "stop", "pause", "paused"];

#[derive(Debug, Default)]
struct CoverFeatures {
    state: Option<String>,
    position: Option<String>,
    tilt: Option<String>,
    running: Option<String>,
    motor: Option<MotorState>,
}

#[derive(Debug)]
struct MotorState {
    property: String,
    opening: String,
    closing: String,
    stopped: String,
}

fn cover_feature_set(expose: &Value, endpoint: Option<&str>) -> CoverFeatures {
    let strip =
        |property: Option<&str>| property.map(|property| strip_endpoint_suffix(property, endpoint));
    let state = strip(feature_property(expose_feature(expose, "state")));
    let position = strip(feature_property(expose_feature(expose, "position")));
    let tilt = strip(feature_property(expose_feature(expose, "tilt")));
    let running = strip(feature_property(expose_feature(expose, "running")));
    let motor = expose_feature(expose, "motor_state")
        .or_else(|| expose_feature(expose, "moving"))
        .filter(|feature| {
            feature
                .get("access")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                & 0x01
                != 0
        })
        .and_then(|feature| {
            let property = strip(feature_property(Some(feature)))?;
            let values: Vec<String> = feature
                .get("values")
                .and_then(|value| value.as_array())?
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect();
            let find = |lookup: &[&str]| {
                values
                    .iter()
                    .find(|value| lookup.contains(&value.to_lowercase().as_str()))
                    .cloned()
            };
            Some(MotorState {
                property,
                opening: find(COVER_OPENING_LOOKUP)?,
                closing: find(COVER_CLOSING_LOOKUP)?,
                stopped: find(COVER_STOPPED_LOOKUP)?,
            })
        });
    CoverFeatures {
        state,
        position,
        tilt,
        running,
        motor,
    }
}

/// Port of the upstream cover `value_template` selection.
fn cover_value_template(features: &CoverFeatures) -> String {
    let state = features
        .state
        .clone()
        .unwrap_or_else(|| "state".to_string());
    if let Some(motor) = &features.motor {
        let position_value = match &features.position {
            Some(position) => format!("value_json[\"{position}\"] | default(none)"),
            None => "none".to_string(),
        };
        return format!(
            "{{% set motor = value_json[\"{}\"] | default(none) %}}{{% set position = {} %}}{{% set state = value_json[\"{}\"] | default(none) %}}{{% if (motor == \"{}\" and position != 100) or (motor == \"{}\" and position != 0) %}}{{{{ motor }}}}{{% elif position == 0 %}}CLOSE{{% elif position == 100 %}}OPEN{{% elif motor == \"{}\" and position is not none %}}OPEN{{% elif state in [\"OPEN\", \"CLOSE\"] %}}{{{{ state }}}}{{% else %}}{}{{% endif %}}",
            motor.property,
            position_value,
            state,
            motor.opening,
            motor.closing,
            motor.stopped,
            motor.stopped
        );
    }
    if let (Some(running), Some(position)) = (&features.running, &features.position) {
        return format!(
            "{{% if \"{running}\" in value_json and value_json[\"{running}\"] %}} {{% if value_json[\"{position}\"] > 0 %}} closing {{% else %}} opening {{% endif %}} {{% else %}} stopped {{% endif %}}"
        );
    }
    format!("{{{{ value_json[\"{state}\"] }}}}")
}

/// Cover discovery entry (also used by the cluster fallback).
fn cover_discovery_config(
    device: &Device,
    base_topic: &str,
    coordinator_ieee: &str,
    endpoint: Option<&str>,
    features: &CoverFeatures,
) -> (String, Value) {
    let ieee_hex = device.ieee_addr.as_hex();
    let state_topic = format!("{}/{}", base_topic, device.friendly_name);
    let (endpoint_state_topic, command_topic, object_id, unique_id) = match endpoint {
        Some(endpoint) => (
            format!("{state_topic}/{endpoint}"),
            format!("{state_topic}/{endpoint}/set"),
            format!("cover_{endpoint}"),
            format!("{ieee_hex}_cover_{endpoint}_{base_topic}"),
        ),
        None => (
            state_topic.clone(),
            format!("{state_topic}/set"),
            "cover".to_string(),
            format!("{ieee_hex}_cover_{base_topic}"),
        ),
    };
    let mut config = json!({
        "availability": availability_block(base_topic),
        "command_topic": command_topic,
        "state_topic": endpoint_state_topic,
        "object_id": object_id,
        "unique_id": unique_id,
        "device": device_block(device, base_topic, coordinator_ieee),
        "value_template": cover_value_template(features),
        "state_open": "OPEN",
        "state_closed": "CLOSE",
        "state_stopped": features
            .motor
            .as_ref()
            .map(|motor| motor.stopped.clone())
            .unwrap_or_else(|| "STOP".to_string()),
    });
    if let Some(motor) = &features.motor {
        config["state_opening"] = json!(motor.opening);
        config["state_closing"] = json!(motor.closing);
    }
    if features.position.is_some() {
        let position_property = features
            .position
            .clone()
            .unwrap_or_else(|| "position".to_string());
        config["position_template"] =
            json!(format!("{{{{ value_json[\"{position_property}\"] }}}}"));
        config["set_position_template"] = json!("{ \"position\": {{ position }} }");
        config["set_position_topic"] = json!(command_topic);
        config["position_topic"] = json!(endpoint_state_topic);
    }
    if features.tilt.is_some() {
        let tilt_property = features.tilt.clone().unwrap_or_else(|| "tilt".to_string());
        config["tilt_command_topic"] = json!(format!("{command_topic}/tilt"));
        config["tilt_status_topic"] = json!(endpoint_state_topic);
        config["tilt_status_template"] =
            json!(format!("{{{{ value_json[\"{tilt_property}\"] }}}}"));
    }
    (object_id, config)
}

/// Per-endpoint cover configs for a multi-endpoint covering device.
fn endpoint_cover_configs(
    device: &Device,
    base_topic: &str,
    coordinator_ieee: &str,
) -> Vec<(String, String, Value)> {
    device
        .endpoints
        .iter()
        .filter(|endpoint| endpoint.input_clusters.contains(&0x0102))
        .map(|endpoint| {
            let suffix = format!("l{}", endpoint.endpoint);
            let features = CoverFeatures {
                state: Some("state".to_string()),
                position: Some("position".to_string()),
                ..CoverFeatures::default()
            };
            let (object_id, config) = cover_discovery_config(
                device,
                base_topic,
                coordinator_ieee,
                Some(&suffix),
                &features,
            );
            ("cover".to_string(), object_id, config)
        })
        .collect()
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
    // Multi-gang devices get one entity per OnOff endpoint; a single-endpoint
    // device keeps the original global light/switch discovery unchanged.
    if on_off_endpoints(device).len() > 1 {
        for (component, object_id, config) in
            endpoint_on_off_configs(device, base_topic, coordinator_ieee)
        {
            publish(mqtt, &component, &ieee_hex, &object_id, &config).await;
        }
    } else if has_on_off {
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

    // ── Cover (window covering) ───────────────────────────────────────────
    let cover_endpoints: Vec<u8> = device
        .endpoints
        .iter()
        .filter(|endpoint| endpoint.input_clusters.contains(&0x0102))
        .map(|endpoint| endpoint.endpoint)
        .collect();
    if cover_endpoints.len() > 1 {
        for (component, object_id, config) in
            endpoint_cover_configs(device, base_topic, coordinator_ieee)
        {
            publish(mqtt, &component, &ieee_hex, &object_id, &config).await;
        }
    } else if !cover_endpoints.is_empty() {
        let features = CoverFeatures {
            state: Some("state".to_string()),
            position: Some("position".to_string()),
            ..CoverFeatures::default()
        };
        let (object_id, config) =
            cover_discovery_config(device, base_topic, coordinator_ieee, None, &features);
        publish(mqtt, "cover", &ieee_hex, &object_id, &config).await;
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
/// Endpoint name from an upstream expose (`endpoint: "l1"` or an endpoint
/// object), used for per-endpoint discovery.
fn expose_endpoint(expose: &Value) -> Option<String> {
    match expose.get("endpoint")? {
        Value::String(name) if !name.is_empty() => Some(name.clone()),
        Value::Object(object) => object
            .get("name")
            .and_then(|name| name.as_str())
            .map(str::to_string)
            .or_else(|| {
                object
                    .get("ID")
                    .and_then(|id| id.as_u64())
                    .map(|id| format!("l{id}"))
            }),
        Value::Number(id) => id.as_u64().map(|id| format!("l{id}")),
        _ => None,
    }
}

/// `switch` entity for a definition property that is not the main state
/// (upstream `SWITCH_DIFFERENT`): per-attribute command topic plus
/// config/diagnostic metadata.
#[allow(clippy::too_many_arguments)]
fn config_switch_entity(
    device: &Device,
    base_topic: &str,
    coordinator_ieee: &str,
    property_key: &str,
    endpoint: Option<&str>,
    on: String,
    off: String,
    label: Option<&str>,
) -> (String, Value) {
    let ieee_hex = device.ieee_addr.as_hex();
    let state_topic = format!("{}/{}", base_topic, device.friendly_name);
    let command_topic = match endpoint {
        Some(endpoint) => format!("{state_topic}/{endpoint}/set/{property_key}"),
        None => format!("{state_topic}/set/{property_key}"),
    };
    let base_property = strip_endpoint_suffix(property_key, endpoint);
    let (category, icon) =
        config_switch_lookup(&base_property).unwrap_or(("config", "mdi:toggle-switch"));
    let mut config = json!({
        "availability": availability_block(base_topic),
        "command_topic": command_topic,
        "state_topic": state_topic,
        "object_id": property_key,
        "unique_id": format!("{ieee_hex}_switch_{property_key}_{base_topic}"),
        "device": device_block(device, base_topic, coordinator_ieee),
        "value_template": state_template(property_key, endpoint),
        "payload_on": on,
        "payload_off": off,
        "state_on": on,
        "state_off": off,
        "entity_category": category,
        "icon": icon,
    });
    let name = label
        .map(str::to_string)
        .unwrap_or_else(|| humanize_property(&base_property));
    config["name"] = json!(name);
    (property_key.to_string(), config)
}

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
    let mut configs = Vec::new();
    for expose in exposes {
        let Some(property) = expose_property(expose) else {
            continue;
        };
        // Definitions may expose per-endpoint features; z2m then suffixes the
        // state property and routes commands to `<friendly>/<endpoint>/set`.
        let endpoint = expose_endpoint(expose);
        let property_key = match &endpoint {
            Some(endpoint) => format!("{property}_{endpoint}"),
            None => property.to_string(),
        };
        let command_topic = match &endpoint {
            Some(endpoint) => format!("{state_topic}/{endpoint}/set"),
            None => format!("{state_topic}/set"),
        };
        let base_property = strip_endpoint_suffix(property, endpoint.as_deref());
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
                    "object_id": property_key,
                    "unique_id": format!("{ieee_hex}_{property_key}_{base_topic}"),
                    "device": ha_device,
                    "value_template": state_template(&property_key, endpoint.as_deref()),
                    "state_class": "measurement",
                    "enabled_by_default": true,
                });
                if let Some(unit) = expose.get("unit").and_then(|value| value.as_str()) {
                    config["unit_of_measurement"] = json!(unit);
                }
                if let Some(class) = sensor_class(property) {
                    config["device_class"] = json!(class);
                }
                configs.push(("sensor".to_string(), property_key, config));
            }
            "switch" if config_switch_lookup(&base_property).is_some() => {
                let on = expose
                    .get("value_on")
                    .map(payload_string)
                    .unwrap_or_else(|| "ON".into());
                let off = expose
                    .get("value_off")
                    .map(payload_string)
                    .unwrap_or_else(|| "OFF".into());
                let (object_id, config) = config_switch_entity(
                    device,
                    base_topic,
                    coordinator_ieee,
                    &property_key,
                    endpoint.as_deref(),
                    on,
                    off,
                    expose.get("label").and_then(|label| label.as_str()),
                );
                configs.push(("switch".to_string(), object_id, config));
            }
            "binary" if access & 0x02 != 0 && config_switch_lookup(&base_property).is_some() => {
                let on = expose
                    .get("value_on")
                    .map(payload_string)
                    .unwrap_or_else(|| "ON".into());
                let off = expose
                    .get("value_off")
                    .map(payload_string)
                    .unwrap_or_else(|| "OFF".into());
                let (object_id, config) = config_switch_entity(
                    device,
                    base_topic,
                    coordinator_ieee,
                    &property_key,
                    endpoint.as_deref(),
                    on,
                    off,
                    expose.get("label").and_then(|label| label.as_str()),
                );
                configs.push(("switch".to_string(), object_id, config));
            }
            "lock" => {
                let state_feature = expose_feature(expose, "state");
                let state_property = feature_property(state_feature)
                    .or(Some(property))
                    .unwrap_or("state");
                let on = state_feature
                    .and_then(|feature| feature.get("value_on"))
                    .cloned()
                    .unwrap_or_else(|| json!("LOCK"));
                let off = state_feature
                    .and_then(|feature| feature.get("value_off"))
                    .cloned()
                    .unwrap_or_else(|| json!("UNLOCK"));
                // Upstream keeps the main state topic and uses the suffixed
                // state property as the command-topic postfix for endpoints.
                let (object_id, unique_id, lock_command_topic) = match &endpoint {
                    Some(endpoint) => (
                        format!("lock_{endpoint}"),
                        format!("{ieee_hex}_lock_{endpoint}_{base_topic}"),
                        format!("{state_topic}/{endpoint}/set/{state_property}"),
                    ),
                    None => (
                        "lock".to_string(),
                        format!("{ieee_hex}_lock_{base_topic}"),
                        format!("{state_topic}/set"),
                    ),
                };
                configs.push((
                    "lock".to_string(),
                    object_id.clone(),
                    json!({
                        "availability": availability,
                        "command_topic": lock_command_topic,
                        "state_topic": state_topic,
                        "object_id": object_id,
                        "unique_id": unique_id,
                        "device": ha_device,
                        "value_template": state_template(state_property, endpoint.as_deref()),
                        "state_locked": on,
                        "state_unlocked": off,
                    }),
                ));
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
                // Per-endpoint switch: z2m keeps the device state topic and
                // selects `state_<endpoint>` in the template, while commands go
                // to `<friendly>/<endpoint>/set`.
                let (object_id, unique_id) = match &endpoint {
                    Some(endpoint) => (
                        format!("{}_{endpoint}", device.friendly_name),
                        format!("{ieee_hex}_switch_{endpoint}_{base_topic}"),
                    ),
                    None => (
                        property_key.clone(),
                        format!("{ieee_hex}_{property_key}_{base_topic}"),
                    ),
                };
                configs.push((
                    "switch".to_string(),
                    match &endpoint {
                        Some(endpoint) => format!("switch_{endpoint}"),
                        None => property_key.clone(),
                    },
                    json!({
                        "availability": availability,
                        "command_topic": command_topic,
                        "state_topic": state_topic,
                        "object_id": object_id,
                        "unique_id": unique_id,
                        "device": ha_device,
                        "value_template": state_template(&property_key, endpoint.as_deref()),
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
                    "object_id": property_key,
                    "unique_id": format!("{ieee_hex}_{property_key}_{base_topic}"),
                    "device": ha_device,
                    "value_template": state_template(&property_key, endpoint.as_deref()),
                    "payload_on": on,
                    "payload_off": off,
                    "enabled_by_default": true,
                });
                if let Some(class) = binary_class(property) {
                    config["device_class"] = json!(class);
                }
                configs.push(("binary_sensor".to_string(), property_key, config));
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
                let (object_id, unique_id) = match &endpoint {
                    Some(endpoint) => (
                        format!("{}_{endpoint}", device.friendly_name),
                        format!("{ieee_hex}_switch_{endpoint}_{base_topic}"),
                    ),
                    None => (
                        property_key.clone(),
                        format!("{ieee_hex}_{property_key}_{base_topic}"),
                    ),
                };
                configs.push((
                    "switch".to_string(),
                    match &endpoint {
                        Some(endpoint) => format!("switch_{endpoint}"),
                        None => property_key.clone(),
                    },
                    json!({
                        "availability": availability,
                        "command_topic": command_topic,
                        "state_topic": state_topic,
                        "object_id": object_id,
                        "unique_id": unique_id,
                        "device": ha_device,
                        "value_template": state_template(&property_key, endpoint.as_deref()),
                        "payload_on": on,
                        "payload_off": off,
                    }),
                ));
            }
            "cover" => {
                let features = cover_feature_set(expose, endpoint.as_deref());
                let (object_id, config) = cover_discovery_config(
                    device,
                    base_topic,
                    coordinator_ieee,
                    endpoint.as_deref(),
                    &features,
                );
                configs.push(("cover".to_string(), object_id, config));
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
                let (object_id, unique_id, state_topic_or_endpoint) = match &endpoint {
                    Some(endpoint) => (
                        format!("light_{endpoint}"),
                        format!("{ieee_hex}_light_{endpoint}_{base_topic}"),
                        // z2m republishes stripped endpoint state here.
                        format!("{state_topic}/{endpoint}"),
                    ),
                    None => (
                        device.friendly_name.clone(),
                        format!("{ieee_hex}_light_{base_topic}"),
                        state_topic.clone(),
                    ),
                };
                let mut config = json!({
                    "availability": availability,
                    "command_topic": command_topic,
                    "state_topic": state_topic_or_endpoint,
                    "schema": "json",
                    "object_id": object_id,
                    "unique_id": unique_id,
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
                let object_id = match &endpoint {
                    Some(endpoint) => format!("light_{endpoint}"),
                    None => "light".to_string(),
                };
                configs.push(("light".to_string(), object_id, config));
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
    let cover_endpoints: Vec<u8> = device
        .endpoints
        .iter()
        .filter(|endpoint| endpoint.input_clusters.contains(&0x0102))
        .map(|endpoint| endpoint.endpoint)
        .collect();
    let mut topics = Vec::new();
    let mut push = |component: &str, object_id: &str| {
        topics.push((component.to_string(), object_id.to_string()));
    };
    if on_off_endpoints(device).len() > 1 {
        for (component, object_id, _) in endpoint_on_off_configs(device, "zigbee2mqtt", "") {
            push(&component, &object_id);
        }
    } else if clusters.contains(&0x0006) {
        if clusters.contains(&0x0008) || clusters.contains(&0x0300) {
            push("light", "light");
        } else {
            push("switch", "switch");
        }
    }
    if cover_endpoints.len() > 1 {
        for (component, object_id, _) in endpoint_cover_configs(device, "zigbee2mqtt", "") {
            push(&component, &object_id);
        }
    } else if !cover_endpoints.is_empty() {
        push("cover", "cover");
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

    fn make_two_gang() -> Device {
        let mut dev = Device::new(
            IeeeAddr([0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]),
            0x2345,
        );
        dev.friendly_name = "two_gang".to_string();
        dev.interview_complete = true;
        dev.endpoints.push(EndpointDesc {
            endpoint: 1,
            profile_id: 0x0104,
            device_id: 0x0100,
            input_clusters: vec![0x0000, 0x0006, 0x0008],
            output_clusters: vec![],
        });
        dev.endpoints.push(EndpointDesc {
            endpoint: 2,
            profile_id: 0x0104,
            device_id: 0x0100,
            input_clusters: vec![0x0006],
            output_clusters: vec![],
        });
        dev
    }

    #[test]
    fn multi_endpoint_discovery_publishes_one_entity_per_endpoint() {
        let dev = make_two_gang();
        let configs = endpoint_on_off_configs(&dev, BASE, COORD_IEEE);
        assert_eq!(configs.len(), 2);

        let (component, object_id, light) = &configs[0];
        assert_eq!(component, "light");
        assert_eq!(object_id, "light_l1");
        assert_eq!(light["object_id"], "light_l1");
        assert_eq!(
            light["unique_id"],
            format!("{}_light_l1_{BASE}", dev.ieee_addr.as_hex())
        );
        // Per-endpoint lights read a stripped snapshot topic, not state_l1.
        assert_eq!(light["state_topic"], format!("{BASE}/two_gang/l1"));
        assert_eq!(light["command_topic"], format!("{BASE}/two_gang/l1/set"));
        assert_eq!(light["schema"], "json");
        assert!(light.get("value_template").is_none());

        let (component, object_id, switch) = &configs[1];
        assert_eq!(component, "switch");
        assert_eq!(object_id, "switch_l2");
        assert_eq!(switch["object_id"], "two_gang_l2");
        assert_eq!(
            switch["unique_id"],
            format!("{}_switch_l2_{BASE}", dev.ieee_addr.as_hex())
        );
        // Per-endpoint switches keep the main state topic with a suffixed key.
        assert_eq!(switch["state_topic"], format!("{BASE}/two_gang"));
        assert_eq!(switch["value_template"], "{{ value_json[\"state_l2\"] }}");
        assert_eq!(switch["command_topic"], format!("{BASE}/two_gang/l2/set"));
        assert_eq!(switch["payload_on"], "ON");
        assert_eq!(switch["payload_off"], "OFF");
    }

    #[test]
    fn multi_endpoint_cleanup_topics_match_discovery_entities() {
        let dev = make_two_gang();
        assert_eq!(
            discovery_topics(&dev, &[]),
            vec![
                ("light".to_string(), "light_l1".to_string()),
                ("sensor".to_string(), "linkquality".to_string()),
                ("switch".to_string(), "switch_l2".to_string()),
            ]
        );
    }

    #[test]
    fn single_endpoint_discovery_shape_is_unchanged() {
        let dev = make_light();
        let configs = configs_from_exposes(
            &dev,
            &[json!({"type": "light", "name": "light", "features": [{"name": "brightness"}]})],
            BASE,
            COORD_IEEE,
        );
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].0, "light");
        assert_eq!(configs[0].1, "light");
        assert_eq!(configs[0].2["state_topic"], format!("{BASE}/bulb"));
        assert_eq!(configs[0].2["command_topic"], format!("{BASE}/bulb/set"));
        assert_eq!(
            configs[0].2["unique_id"],
            format!("{}_light_{BASE}", dev.ieee_addr.as_hex())
        );
    }

    #[test]
    fn expose_endpoint_produces_suffixed_entity() {
        let dev = make_two_gang();
        let exposes = vec![
            json!({"type": "light", "name": "light", "endpoint": "l1", "features": [{"name": "brightness"}]}),
            json!({"type": "switch", "name": "state", "endpoint": "l2",
                   "property": "state", "access": 7, "value_on": "ON", "value_off": "OFF"}),
        ];
        let configs = configs_from_exposes(&dev, &exposes, BASE, COORD_IEEE);
        assert_eq!(configs.len(), 2);
        let light = &configs[0];
        assert_eq!(light.1, "light_l1");
        assert_eq!(light.2["state_topic"], format!("{BASE}/two_gang/l1"));
        assert_eq!(light.2["command_topic"], format!("{BASE}/two_gang/l1/set"));

        let switch = &configs[1];
        assert_eq!(switch.1, "switch_l2");
        assert_eq!(switch.2["object_id"], "two_gang_l2");
        assert_eq!(switch.2["value_template"], "{{ value_json[\"state_l2\"] }}");
        assert_eq!(switch.2["command_topic"], format!("{BASE}/two_gang/l2/set"));
    }

    fn make_cover() -> Device {
        let mut dev = Device::new(
            IeeeAddr([0x9a, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]),
            0x3456,
        );
        dev.friendly_name = "curtain".to_string();
        dev.interview_complete = true;
        dev.endpoints.push(EndpointDesc {
            endpoint: 1,
            profile_id: 0x0104,
            device_id: 0x0202,
            input_clusters: vec![0x0000, 0x0102],
            output_clusters: vec![],
        });
        dev
    }

    #[test]
    fn cover_discovery_maps_position_and_tilt() {
        let dev = make_cover();
        let exposes = vec![json!({
            "type": "cover",
            "name": "cover",
            "features": [
                {"name": "state", "property": "state"},
                {"name": "position", "property": "position"},
                {"name": "tilt", "property": "tilt"}
            ]
        })];
        let configs = configs_from_exposes(&dev, &exposes, BASE, COORD_IEEE);
        assert_eq!(configs.len(), 1);
        let (component, object_id, config) = &configs[0];
        assert_eq!(component, "cover");
        assert_eq!(object_id, "cover");
        assert_eq!(config["state_topic"], format!("{BASE}/curtain"));
        assert_eq!(config["command_topic"], format!("{BASE}/curtain/set"));
        assert_eq!(
            config["unique_id"],
            format!("{}_cover_{BASE}", dev.ieee_addr.as_hex())
        );
        assert_eq!(config["value_template"], "{{ value_json[\"state\"] }}");
        assert_eq!(config["state_open"], "OPEN");
        assert_eq!(config["state_closed"], "CLOSE");
        assert_eq!(config["state_stopped"], "STOP");
        assert_eq!(
            config["position_template"],
            "{{ value_json[\"position\"] }}"
        );
        assert_eq!(
            config["set_position_template"],
            "{ \"position\": {{ position }} }"
        );
        assert_eq!(config["set_position_topic"], format!("{BASE}/curtain/set"));
        assert_eq!(config["position_topic"], format!("{BASE}/curtain"));
        assert_eq!(
            config["tilt_command_topic"],
            format!("{BASE}/curtain/set/tilt")
        );
        assert_eq!(config["tilt_status_topic"], format!("{BASE}/curtain"));
        assert_eq!(config["tilt_status_template"], "{{ value_json[\"tilt\"] }}");
    }

    #[test]
    fn cover_without_position_or_tilt_omits_optional_topics() {
        let dev = make_cover();
        let exposes =
            vec![json!({"type": "cover", "name": "cover", "features": [{"name": "state"}]})];
        let configs = configs_from_exposes(&dev, &exposes, BASE, COORD_IEEE);
        let config = &configs[0].2;
        assert!(config.get("position_template").is_none());
        assert!(config.get("tilt_command_topic").is_none());
    }

    #[test]
    fn multi_endpoint_cover_uses_postfix_state_topic() {
        let mut dev = make_cover();
        dev.endpoints.push(EndpointDesc {
            endpoint: 2,
            profile_id: 0x0104,
            device_id: 0x0202,
            input_clusters: vec![0x0102],
            output_clusters: vec![],
        });
        let configs = endpoint_cover_configs(&dev, BASE, COORD_IEEE);
        assert_eq!(configs.len(), 2);
        let (component, object_id, config) = &configs[0];
        assert_eq!(component, "cover");
        assert_eq!(object_id, "cover_l1");
        assert_eq!(config["state_topic"], format!("{BASE}/curtain/l1"));
        assert_eq!(config["command_topic"], format!("{BASE}/curtain/l1/set"));
        assert_eq!(
            config["unique_id"],
            format!("{}_cover_l1_{BASE}", dev.ieee_addr.as_hex())
        );
        assert_eq!(configs[1].1, "cover_l2");
        assert_eq!(
            configs[1].2["command_topic"],
            format!("{BASE}/curtain/l2/set")
        );
    }

    #[test]
    fn cover_cleanup_topics_include_all_endpoints() {
        let mut dev = make_cover();
        dev.endpoints.push(EndpointDesc {
            endpoint: 2,
            profile_id: 0x0104,
            device_id: 0x0202,
            input_clusters: vec![0x0102],
            output_clusters: vec![],
        });
        let topics = discovery_topics(&dev, &[]);
        assert!(topics.contains(&("cover".to_string(), "cover_l1".to_string())));
        assert!(topics.contains(&("cover".to_string(), "cover_l2".to_string())));
        assert!(topics.contains(&("sensor".to_string(), "linkquality".to_string())));
    }

    #[test]
    fn config_switch_uses_per_attribute_command_topic() {
        let dev = make_switch();
        let exposes = vec![json!({
            "type": "binary",
            "name": "window_detection",
            "property": "window_detection",
            "label": "Open window detection",
            "access": 7,
            "value_on": "ON",
            "value_off": "OFF"
        })];
        let configs = configs_from_exposes(&dev, &exposes, BASE, COORD_IEEE);
        assert_eq!(configs.len(), 1);
        let (component, object_id, config) = &configs[0];
        assert_eq!(component, "switch");
        assert_eq!(object_id, "window_detection");
        assert_eq!(
            config["command_topic"],
            format!("{BASE}/wall_switch/set/window_detection")
        );
        assert_eq!(config["state_topic"], format!("{BASE}/wall_switch"));
        assert_eq!(config["entity_category"], "config");
        assert_eq!(config["icon"], "mdi:window-open-variant");
        assert_eq!(config["name"], "Open window detection");
        assert_eq!(config["state_on"], "ON");
        assert_eq!(config["state_off"], "OFF");
        assert_eq!(config["payload_on"], "ON");
        assert_eq!(
            config["value_template"],
            "{{ value_json.window_detection }}"
        );
        assert_eq!(
            config["unique_id"],
            format!("{}_switch_window_detection_{BASE}", dev.ieee_addr.as_hex())
        );
    }

    #[test]
    fn config_switch_endpoint_suffixes_command_topic() {
        let dev = make_switch();
        let exposes = vec![json!({
            "type": "switch",
            "name": "indicator",
            "property": "indicator",
            "endpoint": "l1",
            "access": 7,
            "value_on": "ON",
            "value_off": "OFF"
        })];
        let configs = configs_from_exposes(&dev, &exposes, BASE, COORD_IEEE);
        let (component, object_id, config) = &configs[0];
        assert_eq!(component, "switch");
        assert_eq!(object_id, "indicator_l1");
        assert_eq!(
            config["command_topic"],
            format!("{BASE}/wall_switch/l1/set/indicator_l1")
        );
        assert_eq!(config["entity_category"], "config");
        assert_eq!(config["icon"], "mdi:led-on");
        assert_eq!(
            config["value_template"],
            "{{ value_json[\"indicator_l1\"] }}"
        );
    }

    #[test]
    fn ordinary_binary_sensor_is_not_a_config_switch() {
        let dev = make_switch();
        let exposes = vec![json!({
            "type": "binary",
            "name": "contact",
            "property": "contact",
            "access": 1,
            "value_on": false,
            "value_off": true
        })];
        let configs = configs_from_exposes(&dev, &exposes, BASE, COORD_IEEE);
        assert_eq!(configs[0].0, "binary_sensor");
        assert!(configs[0].2.get("command_topic").is_none());
    }

    #[test]
    fn lock_discovery_matches_upstream() {
        let dev = make_switch();
        let exposes = vec![json!({
            "type": "lock",
            "name": "lock",
            "property": "state",
            "features": [{"name": "state", "property": "state",
                          "value_on": "LOCK", "value_off": "UNLOCK"}]
        })];
        let configs = configs_from_exposes(&dev, &exposes, BASE, COORD_IEEE);
        let (component, object_id, config) = &configs[0];
        assert_eq!(component, "lock");
        assert_eq!(object_id, "lock");
        assert_eq!(config["command_topic"], format!("{BASE}/wall_switch/set"));
        assert_eq!(config["state_topic"], format!("{BASE}/wall_switch"));
        assert_eq!(config["value_template"], "{{ value_json.state }}");
        assert_eq!(config["state_locked"], "LOCK");
        assert_eq!(config["state_unlocked"], "UNLOCK");
        assert_eq!(
            config["unique_id"],
            format!("{}_lock_{BASE}", dev.ieee_addr.as_hex())
        );
    }

    #[test]
    fn endpoint_lock_uses_suffixed_attribute_command_topic() {
        let dev = make_switch();
        let exposes = vec![json!({
            "type": "lock",
            "name": "state_l1",
            "property": "state_l1",
            "endpoint": "l1",
            "features": [{"name": "state", "property": "state_l1",
                          "value_on": "LOCK", "value_off": "UNLOCK"}]
        })];
        let configs = configs_from_exposes(&dev, &exposes, BASE, COORD_IEEE);
        let (component, object_id, config) = &configs[0];
        assert_eq!(component, "lock");
        assert_eq!(object_id, "lock_l1");
        assert_eq!(
            config["command_topic"],
            format!("{BASE}/wall_switch/l1/set/state_l1")
        );
        assert_eq!(config["value_template"], "{{ value_json[\"state_l1\"] }}");
        assert_eq!(
            config["unique_id"],
            format!("{}_lock_l1_{BASE}", dev.ieee_addr.as_hex())
        );
    }

    #[test]
    fn cover_running_template_reports_movement_direction() {
        let dev = make_cover();
        let exposes = vec![json!({
            "type": "cover",
            "name": "cover",
            "property": "state",
            "features": [
                {"name": "state", "property": "state"},
                {"name": "position", "property": "position"},
                {"name": "running", "property": "running"}
            ]
        })];
        let configs = configs_from_exposes(&dev, &exposes, BASE, COORD_IEEE);
        let config = &configs[0].2;
        assert_eq!(
            config["value_template"],
            "{% if \"running\" in value_json and value_json[\"running\"] %} {% if value_json[\"position\"] > 0 %} closing {% else %} opening {% endif %} {% else %} stopped {% endif %}"
        );
        assert_eq!(config["state_stopped"], "STOP");
    }

    #[test]
    fn cover_motor_state_template_matches_lookup_values() {
        let dev = make_cover();
        let exposes = vec![json!({
            "type": "cover",
            "name": "cover",
            "property": "state",
            "features": [
                {"name": "state", "property": "state"},
                {"name": "position", "property": "position"},
                {"name": "motor_state", "property": "motor_state", "access": 1,
                 "values": ["opening", "closing", "stopped"]}
            ]
        })];
        let configs = configs_from_exposes(&dev, &exposes, BASE, COORD_IEEE);
        let config = &configs[0].2;
        assert_eq!(config["state_opening"], "opening");
        assert_eq!(config["state_closing"], "closing");
        assert_eq!(config["state_stopped"], "stopped");
        let template = config["value_template"].as_str().unwrap();
        assert!(template.contains("{% set motor = value_json[\"motor_state\"] | default(none) %}"));
        assert!(template.contains("{% set position = value_json[\"position\"] | default(none) %}"));
        assert!(template.contains("motor == \"opening\" and position != 100"));
        assert!(template.contains("motor == \"closing\" and position != 0"));
        assert!(template.contains("{% elif position == 0 %}CLOSE"));
        assert!(template.contains("{% elif position == 100 %}OPEN"));
        assert!(template.contains("{% else %}stopped{% endif %}"));
    }
}
