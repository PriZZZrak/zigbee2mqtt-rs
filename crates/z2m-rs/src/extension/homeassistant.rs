// Original: .upstream/zigbee2mqtt/lib/extension/homeassistant.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// First slice: discovery infrastructure, bridge entities (connection state /
// restart required / version), and numeric/binary/enum/switch expose mapping.
// Remaining expose types (light, climate, lock, cover, fan, text, composite,
// list, device_automation/event) and the remaining bridge buttons/sensors are
// being ported next; see docs/parity-matrix.md.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::{json, Map, Value};

use zigbee_converters::{access, Expose};

use crate::extension::extension::{Extension, ExtensionContext, ExtensionFuture};
use crate::model::entity::Entity;
use crate::mqtt::MqttPublishOptions;
use crate::util::stringify::stringify;
use crate::util::utils;

struct Discovered {
    messages: HashMap<String, (String, bool)>,
    triggers: HashSet<String>,
    discovered: bool,
}

#[derive(Clone)]
struct Entry {
    r#type: String,
    object_id: String,
    payload: Map<String, Value>,
}

pub struct HomeAssistant {
    context: ExtensionContext,
    discovery_topic: String,
    status_topic: String,
    legacy_action_sensor: bool,
    experimental_event_entities: bool,
    discovered: Mutex<HashMap<String, Arc<Mutex<Discovered>>>>,
    group_member_lookup: Mutex<HashMap<String, String>>,
    action_value_template: String,
    bridge_identifier: Mutex<String>,
    origin: Mutex<Value>,
    version: Mutex<String>,
}

fn lookups() -> &'static Value {
    static LOOKUPS: OnceLock<Value> = OnceLock::new();
    LOOKUPS.get_or_init(|| {
        serde_json::from_str(include_str!("homeassistant_lookups.json"))
            .expect("embedded homeassistant lookups are valid JSON")
    })
}

fn lookup(table: &str, key: &str) -> Option<&'static Map<String, Value>> {
    lookups()[table].get(key).and_then(Value::as_object)
}

fn merge_lookup(target: &mut Map<String, Value>, table: &str, key: &str) {
    if let Some(extra) = lookup(table, key) {
        for (name, value) in extra {
            target.insert(name.clone(), value.clone());
        }
    }
}

fn merge(target: &mut Map<String, Value>, extra: Map<String, Value>) {
    for (key, value) in extra {
        target.insert(key, value);
    }
}

fn clean_name(name: &str) -> String {
    name.strip_prefix("analog_in_")
        .or_else(|| name.strip_prefix("analog_out_"))
        .unwrap_or(name)
        .to_string()
}

fn state_value_template(property: &str) -> String {
    state_value_template_body(property, &format!("{{{{ value_json[\"{property}\"] }}}}"))
}

fn state_value_template_body(property: &str, body: &str) -> String {
    format!("{{% if \"{property}\" in value_json %}}{body}{{% endif %}}")
}

fn capitalize(value: &str) -> String {
    utils::capitalize(value)
}

impl HomeAssistant {
    pub fn new(context: ExtensionContext) -> Arc<Self> {
        let settings = context.settings_value();
        let discovery_topic = settings["homeassistant"]["discovery_topic"]
            .as_str()
            .unwrap_or("homeassistant")
            .to_string();
        Arc::new(Self {
            context,
            discovery_topic,
            status_topic: settings["homeassistant"]["status_topic"]
                .as_str()
                .unwrap_or("homeassistant/status")
                .to_string(),
            legacy_action_sensor: settings["homeassistant"]["legacy_action_sensor"]
                .as_bool()
                .unwrap_or(false),
            experimental_event_entities: settings["homeassistant"]["experimental_event_entities"]
                .as_bool()
                .unwrap_or(false),
            discovered: Mutex::new(HashMap::new()),
            group_member_lookup: Mutex::new(HashMap::new()),
            action_value_template: action_value_template(),
            bridge_identifier: Mutex::new(String::new()),
            origin: Mutex::new(Value::Null),
            version: Mutex::new("unknown".to_string()),
        })
    }

    fn get_discovered(&self, id: &str) -> Arc<Mutex<Discovered>> {
        let mut map = self.discovered.lock().unwrap();
        map.entry(id.to_string())
            .or_insert_with(|| {
                Arc::new(Mutex::new(Discovered {
                    messages: HashMap::new(),
                    triggers: HashSet::new(),
                    discovered: false,
                }))
            })
            .clone()
    }

    fn get_configs(&self, entity: &Arc<dyn Entity>) -> Vec<Entry> {
        let all_exposes: Vec<Expose> = if entity.is_device() {
            entity
                .zh_device()
                .and_then(|zh| self.context.zigbee.resolve_device(&zh.ieee_addr()))
                .map(|device| device.exposes().unwrap_or_default())
                .unwrap_or_default()
        } else {
            entity
                .members_devices()
                .iter()
                .flat_map(|member| {
                    member
                        .zh_device()
                        .and_then(|zh| self.context.zigbee.resolve_device(&zh.ieee_addr()))
                        .and_then(|device| device.exposes().ok())
                        .unwrap_or_default()
                })
                .collect()
        };
        let exposes: Vec<Expose> = if entity.is_device() {
            all_exposes.clone()
        } else {
            // Group exposes are merged by type so light features from all
            // members drive a single discovery entry (`homeassistant.ts:1650-1685`).
            let mut grouped: Vec<Expose> = Vec::new();
            for expose in all_exposes
                .iter()
                .filter(|expose| ["light", "switch", "lock", "cover"].contains(&expose.r#type.as_str()))
            {
                let mut key = expose.r#type.clone();
                if ["switch", "lock", "cover"].contains(&expose.r#type.as_str())
                    && expose.endpoint.is_some()
                {
                    if let Some(state) = expose
                        .features
                        .iter()
                        .find(|feature| feature.name.as_deref() == Some("state"))
                    {
                        key.push_str(state.property.as_deref().unwrap_or(""));
                    }
                }
                if let Some(existing) = grouped.iter_mut().find(|existing| {
                    existing.r#type == expose.r#type
                        && (existing.features.first().and_then(|f| f.property.clone())
                            == expose.features.first().and_then(|f| f.property.clone()))
                }) {
                    for feature in &expose.features {
                        if existing
                            .features
                            .iter()
                            .any(|f| f.name == feature.name)
                        {
                            continue;
                        }
                        existing.features.push(feature.clone());
                    }
                } else {
                    let _ = key;
                    grouped.push(expose.clone());
                }
            }
            grouped
        };

        let mut entries = Vec::new();
        for expose in &exposes {
            let start = entries.len();
            match expose.r#type.as_str() {
                "switch" => {
                    let Some(state) = expose.features.iter().find(|feature| {
                        feature.r#type == "binary" && feature.name.as_deref() == Some("state")
                    }) else {
                        continue;
                    };
                    let property = state.property.clone().unwrap_or_default();
                    let mut payload = Map::new();
                    payload.insert("name".into(), Value::Null);
                    payload.insert("payload_off".into(), state.value_off.clone());
                    payload.insert("payload_on".into(), state.value_on.clone());
                    payload.insert(
                        "value_template".into(),
                        Value::String(state_value_template(&property)),
                    );
                    payload.insert("command_topic".into(), Value::Bool(true));
                    if [
                        "auto_lock",
                        "away_mode",
                        "comfort_smiley",
                        "enable_display",
                        "indicator",
                        "tilt_mode",
                        "valve_detection",
                        "window_detection",
                    ]
                    .contains(&property.as_str())
                    {
                        payload.insert(
                            "name".into(),
                            expose
                                .label
                                .clone()
                                .map(Value::String)
                                .unwrap_or(Value::Null),
                        );
                        payload.insert(
                            "command_topic_postfix".into(),
                            Value::String(property.clone()),
                        );
                        payload.insert("state_off".into(), state.value_off.clone());
                        payload.insert("state_on".into(), state.value_on.clone());
                        merge_lookup(&mut payload, "CONFIG_SWITCH_DISCOVERY_LOOKUP", &property);
                    }
                    entries.push(Entry {
                        r#type: "switch".into(),
                        object_id: "switch".into(),
                        payload,
                    });
                }
                "climate" => {
                    let find = |name: &str| {
                        expose
                            .features
                            .iter()
                            .find(|feature| feature.name.as_deref() == Some(name))
                    };
                    let heating = find("occupied_heating_setpoint")
                        .or_else(|| find("current_heating_setpoint"));
                    let cooling = find("occupied_cooling_setpoint");
                    let Some(primary) = heating.or(cooling) else {
                        continue;
                    };
                    let (Some(min), Some(max)) = (primary.value_min, primary.value_max) else {
                        continue;
                    };
                    let Some(temperature) = find("local_temperature") else {
                        continue;
                    };

                    let mut payload = Map::new();
                    payload.insert("name".into(), Value::Null);
                    payload.insert("state_topic".into(), Value::Bool(false));
                    payload.insert("temperature_unit".into(), json!("C"));
                    if let Some(step) = primary.value_step {
                        payload.insert("temp_step".into(), Value::from(step));
                    }
                    payload.insert("min_temp".into(), Value::String(min.to_string()));
                    payload.insert("max_temp".into(), Value::String(max.to_string()));
                    payload.insert("current_temperature_topic".into(), Value::Bool(true));
                    payload.insert(
                        "current_temperature_template".into(),
                        Value::String(state_value_template(
                            temperature
                                .property
                                .as_deref()
                                .unwrap_or("local_temperature"),
                        )),
                    );

                    if let Some(mode) = find("system_mode") {
                        let mut values: Vec<String> = mode
                            .values
                            .iter()
                            .filter(|value| value.as_str() != "sleep")
                            .cloned()
                            .collect();
                        payload.insert("mode_state_topic".into(), Value::Bool(true));
                        payload.insert(
                            "modes".into(),
                            Value::Array(values.iter().cloned().map(Value::String).collect()),
                        );
                        if values.iter().any(|value| value == "emergency_heating") {
                            let property = mode.property.as_deref().unwrap_or("system_mode");
                            payload.insert(
                                "mode_state_template".into(),
                                Value::String(state_value_template_body(
                                    property,
                                    &format!("{{% set mode = value_json[\"{property}\"] %}}{{{{ heat if mode == emergency_heating else mode }}}}"),
                                )),
                            );
                            values.retain(|value| value != "emergency_heating");
                            payload.insert(
                                "modes".into(),
                                Value::Array(values.into_iter().map(Value::String).collect()),
                            );
                        } else {
                            payload.insert(
                                "mode_state_template".into(),
                                Value::String(state_value_template(
                                    mode.property.as_deref().unwrap_or("system_mode"),
                                )),
                            );
                        }
                        payload.insert("mode_command_topic".into(), Value::Bool(true));
                    }

                    if let Some(state) = find("running_state") {
                        let property = state.property.as_deref().unwrap_or("running_state");
                        payload.insert("action_topic".into(), Value::Bool(true));
                        payload.insert(
                            "action_template".into(),
                            Value::String(state_value_template_body(
                                property,
                                &format!("{{% set values = {{None:None,idle:idle,heat:heating,cool:cooling,fan_only:fan}} %}}{{{{ values[value_json[\"{property}\"]] }}}}"),
                            )),
                        );
                        let _ = state;
                    }

                    if let Some(cooling) = cooling.filter(|_| heating.is_some()) {
                        let heating = heating.unwrap();
                        payload.insert("temperature_low_command_topic".into(), json!(heating.name));
                        payload.insert(
                            "temperature_low_state_template".into(),
                            Value::String(state_value_template(
                                heating.property.as_deref().unwrap_or(""),
                            )),
                        );
                        payload.insert("temperature_low_state_topic".into(), Value::Bool(true));
                        payload
                            .insert("temperature_high_command_topic".into(), json!(cooling.name));
                        payload.insert(
                            "temperature_high_state_template".into(),
                            Value::String(state_value_template(
                                cooling.property.as_deref().unwrap_or(""),
                            )),
                        );
                        payload.insert("temperature_high_state_topic".into(), Value::Bool(true));
                    } else {
                        payload.insert("temperature_command_topic".into(), json!(primary.name));
                        payload.insert(
                            "temperature_state_template".into(),
                            Value::String(state_value_template(
                                primary.property.as_deref().unwrap_or(""),
                            )),
                        );
                        payload.insert("temperature_state_topic".into(), Value::Bool(true));
                    }

                    if let Some(fan_mode) = find("fan_mode") {
                        payload.insert(
                            "fan_modes".into(),
                            Value::Array(
                                fan_mode.values.iter().cloned().map(Value::String).collect(),
                            ),
                        );
                        payload.insert("fan_mode_command_topic".into(), Value::Bool(true));
                        payload.insert(
                            "fan_mode_state_template".into(),
                            Value::String(state_value_template(
                                fan_mode.property.as_deref().unwrap_or("fan_mode"),
                            )),
                        );
                        payload.insert("fan_mode_state_topic".into(), Value::Bool(true));
                    }
                    if let Some(swing_mode) = find("swing_mode") {
                        payload.insert(
                            "swing_modes".into(),
                            Value::Array(
                                swing_mode
                                    .values
                                    .iter()
                                    .cloned()
                                    .map(Value::String)
                                    .collect(),
                            ),
                        );
                        payload.insert("swing_mode_command_topic".into(), Value::Bool(true));
                        payload.insert(
                            "swing_mode_state_template".into(),
                            Value::String(state_value_template(
                                swing_mode.property.as_deref().unwrap_or("swing_mode"),
                            )),
                        );
                        payload.insert("swing_mode_state_topic".into(), Value::Bool(true));
                    }
                    if let Some(preset) = find("preset") {
                        payload.insert(
                            "preset_modes".into(),
                            Value::Array(
                                preset.values.iter().cloned().map(Value::String).collect(),
                            ),
                        );
                        payload.insert("preset_mode_command_topic".into(), json!("preset"));
                        payload.insert(
                            "preset_mode_value_template".into(),
                            Value::String(state_value_template(
                                preset.property.as_deref().unwrap_or("preset"),
                            )),
                        );
                        payload.insert("preset_mode_state_topic".into(), Value::Bool(true));
                    }

                    if let Some(calibration) = find("local_temperature_calibration") {
                        let mut number = Map::new();
                        number.insert("name".into(), json!(calibration.label));
                        number.insert(
                            "value_template".into(),
                            Value::String(state_value_template(
                                calibration.property.as_deref().unwrap_or(""),
                            )),
                        );
                        number.insert("command_topic".into(), Value::Bool(true));
                        number.insert("command_topic_postfix".into(), json!(calibration.property));
                        number.insert("device_class".into(), json!("temperature_delta"));
                        number.insert("entity_category".into(), json!("config"));
                        if let Some(unit) = &calibration.unit {
                            number.insert("unit_of_measurement".into(), json!(unit));
                        }
                        if let Some(v) = calibration.value_min {
                            number.insert("min".into(), Value::from(v));
                        }
                        if let Some(v) = calibration.value_max {
                            number.insert("max".into(), Value::from(v));
                        }
                        if let Some(v) = calibration.value_step {
                            number.insert("step".into(), Value::from(v));
                        }
                        entries.push(Entry {
                            r#type: "number".into(),
                            object_id: calibration
                                .name
                                .clone()
                                .unwrap_or_else(|| "local_temperature_calibration".to_string()),
                            payload: number,
                        });
                    }

                    if let Some(demand) = find("pi_heating_demand") {
                        let mut entry = Map::new();
                        entry.insert("name".into(), json!(demand.label));
                        entry.insert(
                            "value_template".into(),
                            Value::String(state_value_template(
                                demand.property.as_deref().unwrap_or(""),
                            )),
                        );
                        if let Some(unit) = &demand.unit {
                            entry.insert("unit_of_measurement".into(), json!(unit));
                        }
                        entry.insert("icon".into(), json!("mdi:radiator"));
                        if demand.access & access::SET != 0 {
                            entry.insert("command_topic".into(), Value::Bool(true));
                            entry.insert("command_topic_postfix".into(), json!(demand.property));
                            if let Some(v) = demand.value_min {
                                entry.insert("min".into(), Value::from(v));
                            }
                            if let Some(v) = demand.value_max {
                                entry.insert("max".into(), Value::from(v));
                            }
                            entries.push(Entry {
                                r#type: "number".into(),
                                object_id: demand
                                    .name
                                    .clone()
                                    .unwrap_or_else(|| "pi_heating_demand".to_string()),
                                payload: entry,
                            });
                        } else {
                            entry.insert("entity_category".into(), json!("diagnostic"));
                            entries.push(Entry {
                                r#type: "sensor".into(),
                                object_id: demand
                                    .name
                                    .clone()
                                    .unwrap_or_else(|| "pi_heating_demand".to_string()),
                                payload: entry,
                            });
                        }
                    }
                    if let Some(demand) = find("pi_cooling_demand") {
                        let mut entry = Map::new();
                        entry.insert("name".into(), json!(demand.label));
                        entry.insert(
                            "value_template".into(),
                            Value::String(state_value_template(
                                demand.property.as_deref().unwrap_or(""),
                            )),
                        );
                        if let Some(unit) = &demand.unit {
                            entry.insert("unit_of_measurement".into(), json!(unit));
                        }
                        entry.insert("entity_category".into(), json!("diagnostic"));
                        entry.insert("icon".into(), json!("mdi:air-conditioner"));
                        entries.push(Entry {
                            r#type: "sensor".into(),
                            object_id: demand
                                .name
                                .clone()
                                .unwrap_or_else(|| "pi_cooling_demand".to_string()),
                            payload: entry,
                        });
                    }

                    let has_temperature_sensor = all_exposes.iter().any(|e| {
                        e.r#type == "numeric"
                            && e.name
                                .as_deref()
                                .is_some_and(|n| n == "temperature" || n == "local_temperature")
                            && e.access & access::STATE != 0
                    });
                    if !has_temperature_sensor {
                        if let Some(local) = find("local_temperature") {
                            let mut entry = Map::new();
                            entry.insert("name".into(), json!(local.label));
                            entry.insert(
                                "value_template".into(),
                                Value::String(state_value_template(
                                    local.property.as_deref().unwrap_or(""),
                                )),
                            );
                            if let Some(unit) = &local.unit {
                                entry.insert("unit_of_measurement".into(), json!(unit));
                            }
                            entry.insert("device_class".into(), json!("temperature"));
                            entry.insert("state_class".into(), json!("measurement"));
                            entries.push(Entry {
                                r#type: "sensor".into(),
                                object_id: local
                                    .name
                                    .clone()
                                    .unwrap_or_else(|| "local_temperature".to_string()),
                                payload: entry,
                            });
                        }
                    }
                    if let Some(humidity) = all_exposes
                        .iter()
                        .filter(|e| {
                            e.r#type == "numeric"
                                && e.name.as_deref() == Some("humidity")
                                && e.access & access::STATE != 0
                        })
                        .find(|e| e.endpoint.is_none())
                    {
                        payload.insert(
                            "current_humidity_template".into(),
                            Value::String(state_value_template(
                                humidity.property.as_deref().unwrap_or(""),
                            )),
                        );
                        payload.insert("current_humidity_topic".into(), Value::Bool(true));
                    }

                    entries.push(Entry {
                        r#type: "climate".into(),
                        object_id: "climate".into(),
                        payload,
                    });
                }
                "light" => {
                    let feature = |name: &str| {
                        all_exposes.iter().find_map(|expose| {
                            expose
                                .features
                                .iter()
                                .find(|feature| feature.name.as_deref() == Some(name))
                        })
                    };
                    let has_color_xy = all_exposes.iter().any(|expose| {
                        expose
                            .features
                            .iter()
                            .any(|f| f.name.as_deref() == Some("color_xy"))
                    });
                    let has_color_hs = all_exposes.iter().any(|expose| {
                        expose
                            .features
                            .iter()
                            .any(|f| f.name.as_deref() == Some("color_hs"))
                    });
                    let has_brightness = all_exposes.iter().any(|expose| {
                        expose
                            .features
                            .iter()
                            .any(|f| f.name.as_deref() == Some("brightness"))
                    });
                    let has_color_temp = all_exposes.iter().any(|expose| {
                        expose
                            .features
                            .iter()
                            .any(|f| f.name.as_deref() == Some("color_temp"))
                    });
                    let Some(_state) = feature("state") else {
                        continue;
                    };
                    let prefer_hs = all_exposes.iter().any(|expose| {
                        let xy = expose
                            .features
                            .iter()
                            .position(|f| f.name.as_deref() == Some("color_xy"));
                        let hs = expose
                            .features
                            .iter()
                            .position(|f| f.name.as_deref() == Some("color_hs"));
                        xy.is_some() && hs.is_some() && hs < xy
                    });

                    let mut payload = Map::new();
                    payload.insert("name".into(), Value::Null);
                    payload.insert("brightness".into(), Value::Bool(has_brightness));
                    payload.insert("schema".into(), json!("json"));
                    payload.insert("command_topic".into(), Value::Bool(true));
                    payload.insert("brightness_scale".into(), json!(254));
                    let mut modes: Vec<Value> = Vec::new();
                    if has_color_xy && !prefer_hs {
                        modes.push(json!("xy"));
                    }
                    if has_color_hs && (!has_color_xy || prefer_hs) {
                        modes.push(json!("hs"));
                    }
                    if has_color_temp {
                        modes.push(json!("color_temp"));
                    }
                    if modes.is_empty() {
                        modes.push(json!("brightness"));
                    }
                    payload.insert("supported_color_modes".into(), Value::Array(modes));

                    if has_color_temp {
                        let color_temps: Vec<&Expose> = all_exposes
                            .iter()
                            .filter_map(|expose| {
                                expose
                                    .features
                                    .iter()
                                    .find(|f| f.name.as_deref() == Some("color_temp"))
                            })
                            .collect();
                        let max = color_temps
                            .iter()
                            .filter_map(|e| e.value_max)
                            .reduce(f64::min);
                        let min = color_temps
                            .iter()
                            .filter_map(|e| e.value_min)
                            .reduce(f64::max);
                        if let Some(max) = max {
                            payload.insert("max_mireds".into(), Value::from(max));
                        }
                        if let Some(min) = min {
                            payload.insert("min_mireds".into(), Value::from(min));
                        }
                    }

                    let mut effects: Vec<Value> = Vec::new();
                    for expose in &all_exposes {
                        if expose.r#type == "enum" && expose.name.as_deref() == Some("effect") {
                            for value in &expose.values {
                                if !effects.contains(&json!(value)) {
                                    effects.push(json!(value));
                                }
                            }
                        }
                    }
                    if !effects.is_empty() {
                        payload.insert("effect".into(), Value::Bool(true));
                        payload.insert("effect_list".into(), Value::Array(effects));
                    }
                    entries.push(Entry {
                        r#type: "light".into(),
                        object_id: "light".into(),
                        payload,
                    });
                }
                "lock" => {
                    let Some(state) = expose
                        .features
                        .iter()
                        .find(|f| f.r#type == "binary" && f.name.as_deref() == Some("state"))
                    else {
                        continue;
                    };
                    let mut payload = Map::new();
                    payload.insert("name".into(), Value::Null);
                    payload.insert("command_topic".into(), Value::Bool(true));
                    payload.insert(
                        "value_template".into(),
                        Value::String(state_value_template(
                            state.property.as_deref().unwrap_or("state"),
                        )),
                    );
                    payload.insert("state_locked".into(), state.value_on.clone());
                    payload.insert("state_unlocked".into(), state.value_off.clone());
                    entries.push(Entry {
                        r#type: "lock".into(),
                        object_id: "lock".into(),
                        payload,
                    });
                }
                "cover" => {
                    let feature = |name: &str| {
                        all_exposes.iter().find_map(|expose| {
                            expose
                                .features
                                .iter()
                                .find(|feature| feature.name.as_deref() == Some(name))
                        })
                    };
                    let Some(state) = feature("state") else {
                        continue;
                    };
                    let position = feature("position");
                    let tilt = feature("tilt");
                    let motor_state = all_exposes.iter().find(|expose| {
                        expose.r#type == "enum"
                            && expose
                                .name
                                .as_deref()
                                .is_some_and(|n| n == "motor_state" || n == "moving")
                            && expose.access & access::STATE != 0
                    });
                    let running = all_exposes.iter().find(|expose| {
                        expose.r#type == "binary" && expose.name.as_deref() == Some("running")
                    });

                    let mut payload = Map::new();
                    payload.insert("name".into(), Value::Null);
                    payload.insert("command_topic".into(), Value::Bool(true));
                    payload.insert("state_topic".into(), Value::Bool(true));

                    if let Some(running) = running {
                        if let Some(position) = position {
                            let running_property = running.property.as_deref().unwrap_or("running");
                            let position_property =
                                position.property.as_deref().unwrap_or("position");
                            payload.insert("value_template".into(), Value::String(format!("{{% if \"{running_property}\" in value_json and value_json[\"{running_property}\"] and \"{position_property}\" in value_json %}} {{% if value_json[\"{position_property}\"] > 0 %}} closing {{% else %}} opening {{% endif %}} {{% else %}} stopped {{% endif %}}")));
                        }
                    }
                    if let Some(motor_state) = motor_state {
                        let motor_property =
                            motor_state.property.as_deref().unwrap_or("motor_state");
                        let state_property = state.property.as_deref().unwrap_or("state");
                        let position_property = position.and_then(|p| p.property.as_deref());
                        let opening = motor_state
                            .values
                            .iter()
                            .find(|v| {
                                ["opening", "open", "forward", "up", "rising"]
                                    .contains(&v.to_lowercase().as_str())
                            })
                            .cloned();
                        let closing = motor_state
                            .values
                            .iter()
                            .find(|v| {
                                [
                                    "closing",
                                    "close",
                                    "backward",
                                    "back",
                                    "reverse",
                                    "down",
                                    "declining",
                                ]
                                .contains(&v.to_lowercase().as_str())
                            })
                            .cloned();
                        let stopped = motor_state
                            .values
                            .iter()
                            .find(|v| {
                                ["stopped", "stop", "pause", "paused"]
                                    .contains(&v.to_lowercase().as_str())
                            })
                            .cloned();
                        if let (Some(opening), Some(closing), Some(stopped)) =
                            (opening, closing, stopped)
                        {
                            payload.insert("state_opening".into(), json!(opening));
                            payload.insert("state_closing".into(), json!(closing));
                            payload.insert("state_open".into(), json!("OPEN"));
                            payload.insert("state_closed".into(), json!("CLOSE"));
                            payload.insert("state_stopped".into(), json!(stopped));
                            let position_value = position_property
                                .map(|p| format!("value_json[\"{p}\"] | default(none)"))
                                .unwrap_or_else(|| "none".to_string());
                            let template = format!("{{% set motor = value_json[\"{motor_property}\"] | default(none) %}}{{% set position = {position_value} %}}{{% set state = value_json[\"{state_property}\"] | default(none) %}}{{% if (motor == \"{opening}\" and position != 100) or (motor == \"{closing}\" and position != 0) %}}{{{{ motor }}}}{{% elif position == 0 %}}CLOSE{{% elif position == 100 %}}OPEN{{% elif motor == \"{stopped}\" and position is not none %}}OPEN{{% elif state in [\"OPEN\", \"CLOSE\"] %}}{{{{ state }}}}{{% else %}}{stopped}{{% endif %}}");
                            payload.insert("value_template".into(), Value::String(template));
                        }
                    }
                    if !payload.contains_key("value_template") {
                        payload.insert(
                            "value_template".into(),
                            Value::String(state_value_template(
                                state.property.as_deref().unwrap_or("state"),
                            )),
                        );
                        payload.insert("state_open".into(), json!("OPEN"));
                        payload.insert("state_closed".into(), json!("CLOSE"));
                        payload.insert("state_stopped".into(), json!("STOP"));
                    }
                    if position.is_none() && tilt.is_none() {
                        payload.insert("optimistic".into(), Value::Bool(true));
                    }
                    if let Some(position) = position {
                        let property = position.property.as_deref().unwrap_or("position");
                        payload.insert(
                            "position_template".into(),
                            Value::String(state_value_template(property)),
                        );
                        payload.insert(
                            "set_position_template".into(),
                            Value::String(format!("{{ \"{property}\": {{{{ position }}}} }}")),
                        );
                        payload.insert("set_position_topic".into(), Value::Bool(true));
                        payload.insert("position_topic".into(), Value::Bool(true));
                    }
                    if let Some(tilt) = tilt {
                        payload.insert("tilt_command_topic".into(), Value::Bool(true));
                        payload.insert("tilt_status_topic".into(), Value::Bool(true));
                        payload.insert(
                            "tilt_status_template".into(),
                            Value::String(state_value_template(
                                tilt.property.as_deref().unwrap_or("tilt"),
                            )),
                        );
                    }
                    entries.push(Entry {
                        r#type: "cover".into(),
                        object_id: "cover".into(),
                        payload,
                    });
                }
                "fan" => {
                    let state = expose
                        .features
                        .iter()
                        .find(|f| f.r#type == "binary" && f.name.as_deref() == Some("state"));
                    let state_property = state
                        .and_then(|s| s.property.clone())
                        .unwrap_or_else(|| "state".to_string());
                    let mode = expose
                        .features
                        .iter()
                        .find(|f| f.r#type == "enum" && f.name.as_deref() == Some("mode"));
                    let speed = expose
                        .features
                        .iter()
                        .find(|f| f.r#type == "numeric" && f.name.as_deref() == Some("speed"));
                    let mut payload = Map::new();
                    payload.insert("name".into(), Value::Null);
                    payload.insert("command_topic".into(), Value::Bool(true));
                    payload.insert("state_topic".into(), Value::Bool(true));
                    if let Some(mode) = mode {
                        let property = mode.property.as_deref().unwrap_or("fan_mode");
                        let speeds: Vec<Value> = std::iter::once("off".to_string())
                            .chain(
                                [
                                    "low", "medium", "high", "1", "2", "3", "4", "5", "6", "7",
                                    "8", "9",
                                ]
                                .iter()
                                .filter(|s| mode.values.iter().any(|v| v == *s))
                                .map(|s| s.to_string()),
                            )
                            .map(Value::String)
                            .collect();
                        let presets: Vec<Value> = ["on", "auto", "smart"]
                            .iter()
                            .filter(|s| mode.values.iter().any(|v| v == *s))
                            .map(|s| json!(s))
                            .collect();
                        payload.insert("percentage_state_topic".into(), Value::Bool(true));
                        payload.insert("percentage_command_topic".into(), json!(property));
                        let percent_values = speeds
                            .iter()
                            .enumerate()
                            .map(|(i, s)| format!("'{}':{i}", s.as_str().unwrap_or_default()))
                            .collect::<Vec<_>>()
                            .join(", ");
                        let percent_commands = speeds
                            .iter()
                            .enumerate()
                            .map(|(i, s)| format!("{i}:'{}'", s.as_str().unwrap_or_default()))
                            .collect::<Vec<_>>()
                            .join(", ");
                        payload.insert(
                            "percentage_value_template".into(),
                            Value::String(state_value_template_body(
                                property,
                                &format!(
                                    "{{{{ {{{percent_values}}}[value_json[\"{property}\"]] | default('None') }}}}"
                                ),
                            )),
                        );
                        payload.insert(
                            "percentage_command_template".into(),
                            Value::String(format!(
                                "{{{{ {{{percent_commands}}}[value] | default('') }}}}"
                            )),
                        );
                        payload.insert("speed_range_min".into(), json!(1));
                        payload.insert("speed_range_max".into(), json!(speeds.len() as u64 - 1));
                        payload.insert("preset_mode_state_topic".into(), Value::Bool(true));
                        payload.insert("preset_mode_command_topic".into(), json!(property));
                        let preset_list = presets
                            .iter()
                            .map(|s| format!("'{}'", s.as_str().unwrap_or_default()))
                            .collect::<Vec<_>>()
                            .join(", ");
                        payload.insert(
                            "preset_mode_value_template".into(),
                            Value::String(state_value_template_body(
                                property,
                                &format!(
                                    "{{{{ value_json[\"{property}\"] if value_json[\"{property}\"] in [{preset_list}] else 'None' }}}}"
                                ),
                            )),
                        );
                        payload.insert("preset_modes".into(), Value::Array(presets));
                        payload.insert(
                            "state_value_template".into(),
                            Value::String(state_value_template(&state_property)),
                        );
                        payload.insert("command_topic_postfix".into(), json!(state_property));
                    } else if let Some(speed) = speed {
                        let property = speed.property.as_deref().unwrap_or("speed");
                        payload.insert("percentage_state_topic".into(), Value::Bool(true));
                        payload.insert("percentage_command_topic".into(), json!(property));
                        payload.insert(
                            "percentage_value_template".into(),
                            Value::String(state_value_template(property)),
                        );
                        payload.insert(
                            "percentage_command_template".into(),
                            Value::String("{{ value | default() }}".to_string()),
                        );
                        if let Some(min) = speed.value_min {
                            payload.insert("speed_range_min".into(), Value::from(min));
                        }
                        if let Some(max) = speed.value_max {
                            payload.insert("speed_range_max".into(), Value::from(max));
                        }
                        payload.insert(
                            "state_value_template".into(),
                            Value::String(state_value_template(&state_property)),
                        );
                        payload.insert("command_topic_postfix".into(), json!(state_property));
                    } else {
                        continue;
                    }
                    entries.push(Entry {
                        r#type: "fan".into(),
                        object_id: "fan".into(),
                        payload,
                    });
                }
                "text" | "composite" | "list" => {
                    if expose.r#type == "composite" && expose.name.as_deref() == Some("warning") && expose.access & access::SET != 0 {
                        let mode = expose.features.iter().find(|f| f.r#type == "enum" && f.name.as_deref() == Some("mode"));
                        let level = expose.features.iter().find(|f| f.r#type == "enum" && f.name.as_deref() == Some("level"));
                        let duration = expose.features.iter().find(|f| f.r#type == "numeric" && f.name.as_deref() == Some("duration"));
                        let mut payload = Map::new();
                        payload.insert("name".into(), Value::Null);
                        payload.insert("command_topic".into(), Value::Bool(true));
                        payload.insert("state_topic".into(), Value::Bool(false));
                        payload.insert("optimistic".into(), Value::Bool(true));
                        if let Some(mode) = mode {
                            let tones: Vec<Value> = mode.values.iter().filter(|v| v.as_str() != "stop").cloned().map(Value::String).collect();
                            if !tones.is_empty() { payload.insert("available_tones".into(), Value::Array(tones)); }
                        }
                        if level.is_some() { payload.insert("support_volume_set".into(), Value::Bool(true)); }
                        if duration.is_some() { payload.insert("support_duration".into(), Value::Bool(true)); }
                        let level_template = "{% if volume_level is defined %}{% if volume_level | float <= 0.25 %}low{% elif volume_level | float <= 0.5 %}medium{% elif volume_level | float <= 0.75 %}high{% else %}very_high{% endif %}{% else %}medium{% endif %}";
                        payload.insert("command_template".into(), Value::String(format!("{{\"warning\": {{\"mode\": \"{{{{ tone | default(emergency) }}}}\", \"level\": \"{level_template}\", \"duration\": {{{{ duration | default(10) }}}}}}}}")));
                        payload.insert("command_off_template".into(), Value::String("{\"warning\": {\"mode\": \"stop\"}}".to_string()));
                        entries.push(Entry { r#type: "siren".into(), object_id: "siren".into(), payload });
                        continue;
                    }
                    if expose.r#type == "text" && expose.access & access::SET != 0 {
                        let property = expose.property.clone().unwrap_or_default();
                        let mut payload = Map::new();
                        payload.insert("name".into(), json!(expose.label));
                        payload.insert("state_topic".into(), Value::Bool(expose.access & access::STATE != 0));
                        payload.insert("value_template".into(), Value::String(state_value_template(&property)));
                        payload.insert("command_topic".into(), Value::Bool(true));
                        payload.insert("command_topic_postfix".into(), Value::String(property.clone()));
                        merge_lookup(&mut payload, "LIST_DISCOVERY_LOOKUP", expose.name.as_deref().unwrap_or(""));
                        entries.push(Entry { r#type: "text".into(), object_id: property, payload });
                        continue;
                    }
                    if expose.access & access::STATE != 0 {
                        let property = expose.property.clone().unwrap_or_default();
                        let mut payload = Map::new();
                        payload.insert("name".into(), json!(expose.label));
                        payload.insert("value_template".into(), Value::String(state_value_template_body(&property, &format!("{{{{ value_json[\"{property}\"] | default(,True) | string | truncate(254, True, , 0) }}}}"))));
                        merge_lookup(&mut payload, "LIST_DISCOVERY_LOOKUP", expose.name.as_deref().unwrap_or(""));
                        entries.push(Entry { r#type: "sensor".into(), object_id: property, payload });
                    }
                }
                "binary" => {
                    let property = expose.property.clone().unwrap_or_default();
                    let label = expose
                        .label
                        .clone()
                        .or_else(|| expose.name.clone())
                        .unwrap_or_else(|| property.clone());
                    if expose.access & access::SET != 0 {
                        let mut payload = Map::new();
                        payload.insert("name".into(), Value::String(label));
                        payload.insert(
                            "value_template".into(),
                            Value::String(state_value_template(&property)),
                        );
                        payload.insert(
                            "payload_on".into(),
                            Value::String(expose.value_on.to_string()),
                        );
                        payload.insert(
                            "payload_off".into(),
                            Value::String(expose.value_off.to_string()),
                        );
                        payload.insert("command_topic".into(), Value::Bool(true));
                        payload.insert(
                            "command_topic_postfix".into(),
                            Value::String(property.clone()),
                        );
                        merge_lookup(
                            &mut payload,
                            "BINARY_DISCOVERY_LOOKUP",
                            expose.name.as_deref().unwrap_or(""),
                        );
                        entries.push(Entry {
                            r#type: "switch".into(),
                            object_id: format!("switch_{property}"),
                            payload,
                        });
                    } else {
                        let mut payload = Map::new();
                        payload.insert("name".into(), Value::String(label));
                        payload.insert(
                            "value_template".into(),
                            Value::String(state_value_template(&property)),
                        );
                        payload.insert("payload_on".into(), expose.value_on.clone());
                        payload.insert("payload_off".into(), expose.value_off.clone());
                        merge_lookup(
                            &mut payload,
                            "BINARY_DISCOVERY_LOOKUP",
                            expose.name.as_deref().unwrap_or(""),
                        );
                        entries.push(Entry {
                            r#type: "binary_sensor".into(),
                            object_id: expose.name.clone().unwrap_or(property),
                            payload,
                        });
                    }
                }
                "numeric" => {
                    let property = expose.property.clone().unwrap_or_default();
                    let label = expose
                        .label
                        .clone()
                        .or_else(|| expose.name.clone())
                        .unwrap_or_else(|| property.clone());
                    let allows_set = expose.access & access::SET != 0;
                    let mut payload = Map::new();
                    payload.insert("name".into(), Value::String(label.clone()));
                    payload.insert(
                        "value_template".into(),
                        Value::String(state_value_template(&property)),
                    );
                    if allows_set {
                        payload.insert("command_topic".into(), Value::Bool(true));
                        payload.insert(
                            "command_topic_postfix".into(),
                            Value::String(property.clone()),
                        );
                    } else {
                        payload.insert("enabled_by_default".into(), Value::Bool(true));
                    }
                    if let Some(unit) = &expose.unit {
                        payload.insert("unit_of_measurement".into(), Value::String(unit.clone()));
                    }
                    if let Some(step) = expose.value_step {
                        payload.insert("step".into(), Value::from(step));
                    }
                    let mut lookup_extra = if allows_set {
                        lookup(
                            "NUMERIC_DISCOVERY_LOOKUP",
                            &clean_name(expose.name.as_deref().unwrap_or("")),
                        )
                        .cloned()
                        .unwrap_or_default()
                    } else {
                        let mut key = clean_name(expose.name.as_deref().unwrap_or(""));
                        if expose.name.as_deref() == Some("voc")
                            && expose
                                .unit
                                .as_deref()
                                .is_some_and(|unit| unit == "ppb" || unit == "ppm")
                        {
                            key = "voc_parts".to_string();
                        }
                        lookup("NUMERIC_DISCOVERY_LOOKUP", &key)
                            .cloned()
                            .unwrap_or_default()
                    };
                    if allows_set {
                        if lookup_extra.get("device_class").and_then(Value::as_str)
                            != Some("temperature")
                        {
                            lookup_extra.remove("device_class");
                        }
                    } else if let Some(unit) = &expose.unit {
                        if ["Wh", "kWh"].contains(&unit.as_str()) {
                            lookup_extra.insert("device_class".into(), json!("energy"));
                            lookup_extra.insert("state_class".into(), json!("total_increasing"));
                        } else if ["A", "mA"].contains(&unit.as_str()) {
                            lookup_extra.insert("device_class".into(), json!("current"));
                            lookup_extra.insert("state_class".into(), json!("measurement"));
                        } else if ["mW", "W", "kW"].contains(&unit.as_str()) {
                            lookup_extra.insert("device_class".into(), json!("power"));
                            lookup_extra.insert("state_class".into(), json!("measurement"));
                        }
                    }
                    merge(&mut payload, lookup_extra);
                    if payload.get("device_class").is_some()
                        && !payload.contains_key("unit_of_measurement")
                    {
                        payload.remove("device_class");
                    }
                    if payload.get("entity_category").and_then(Value::as_str) == Some("config") {
                        payload.insert("entity_category".into(), json!("diagnostic"));
                    }
                    if allows_set {
                        if let Some(min) = expose.value_min {
                            payload.insert("min".into(), Value::from(min));
                        }
                        if let Some(max) = expose.value_max {
                            payload.insert("max".into(), Value::from(max));
                        }
                        entries.push(Entry {
                            r#type: "number".into(),
                            object_id: expose.name.clone().unwrap_or(property.clone()),
                            payload,
                        });
                    } else {
                        entries.push(Entry {
                            r#type: "sensor".into(),
                            object_id: expose.name.clone().unwrap_or(property.clone()),
                            payload,
                        });
                    }
                }
                "enum" => {
                    let property = expose.property.clone().unwrap_or_default();
                    if property == "action" {
                        if self.experimental_event_entities
                            && expose.access & access::STATE != 0
                            && expose.access & access::SET == 0
                        {
                            let mut payload = Map::new();
                            payload.insert("state_topic".into(), Value::Bool(true));
                            payload.insert(
                                "event_types".into(),
                                Value::Array(
                                    prepare_action_event_types(&expose.values)
                                        .into_iter()
                                        .map(Value::String)
                                        .collect(),
                                ),
                            );
                            payload.insert(
                                "value_template".into(),
                                Value::String(self.action_value_template.clone()),
                            );
                            merge_lookup(
                                &mut payload,
                                "ENUM_DISCOVERY_LOOKUP",
                                expose.name.as_deref().unwrap_or(""),
                            );
                            entries.push(Entry {
                                r#type: "event".into(),
                                object_id: property.clone(),
                                payload,
                            });
                        }
                        if !self.legacy_action_sensor {
                            continue;
                        }
                    }
                    let label = expose
                        .label
                        .clone()
                        .or_else(|| expose.name.clone())
                        .unwrap_or_else(|| property.clone());
                    let mut payload = Map::new();
                    payload.insert("name".into(), Value::String(label));
                    if expose.access & access::STATE != 0 {
                        payload.insert(
                            "value_template".into(),
                            Value::String(state_value_template(&property)),
                        );
                    }
                    merge_lookup(
                        &mut payload,
                        "ENUM_DISCOVERY_LOOKUP",
                        expose.name.as_deref().unwrap_or(""),
                    );
                    if expose.access & access::SET != 0 && expose.values.len() == 1 {
                        payload.insert("state_topic".into(), Value::Bool(false));
                        payload.insert("command_topic".into(), Value::Bool(true));
                        payload.insert(
                            "command_topic_postfix".into(),
                            Value::String(property.clone()),
                        );
                        payload.insert(
                            "payload_press".into(),
                            Value::String(expose.values[0].clone()),
                        );
                        entries.push(Entry {
                            r#type: "button".into(),
                            object_id: property,
                            payload,
                        });
                    } else if expose.access & access::SET != 0 {
                        payload.insert("command_topic".into(), Value::Bool(true));
                        payload.insert(
                            "command_topic_postfix".into(),
                            Value::String(property.clone()),
                        );
                        payload.insert(
                            "options".into(),
                            Value::Array(
                                expose.values.iter().cloned().map(Value::String).collect(),
                            ),
                        );
                        entries.push(Entry {
                            r#type: "select".into(),
                            object_id: property,
                            payload,
                        });
                    } else if expose.access & access::STATE != 0 {
                        entries.push(Entry {
                            r#type: "sensor".into(),
                            object_id: property,
                            payload,
                        });
                    }
                }
                _ => {}
            }

            let new_entries = &mut entries[start..];
            if expose.category.as_deref() == Some("config") || expose.category.as_deref() == Some("diagnostic") {
                let category = expose.category.clone().unwrap_or_default();
                for entry in new_entries.iter_mut() {
                    entry.payload.insert("entity_category".into(), json!(category));
                }
            }
            if let Value::Object(ha) = &expose.homeassistant {
                let apply = |target: &mut Map<String, Value>, ignore_name: bool| {
                    if let Some(kind) = ha.get("type").and_then(Value::as_str) {
                        target.insert("__type".into(), json!(kind));
                    }
                    let mapping = [
                        ("schema", "schema"),
                        ("entityCategory", "entity_category"),
                        ("deviceClass", "device_class"),
                        ("enabledByDefault", "enabled_by_default"),
                        ("icon", "icon"),
                        ("name", "name"),
                        ("valueTemplate", "value_template"),
                    ];
                    for (source, destination) in mapping {
                        if ignore_name && destination == "name" { continue; }
                        match ha.get(source) {
                            Some(Value::Null) => { target.remove(destination); }
                            Some(value) => { target.insert(destination.to_string(), value.clone()); }
                            None => {}
                        }
                    }
                };
                for entry in new_entries.iter_mut() {
                    apply(&mut entry.payload, true);
                    if let Some(overrides) = ha.get(&entry.object_id).and_then(Value::as_object) {
                        for (key, value) in overrides {
                            if key == "type" || key == "object_id" { continue; }
                            match value {
                                Value::Null => { entry.payload.remove(key); }
                                Value::Object(device) if key == "device" => {
                                    if let Some(target) = entry.payload.get_mut("device").and_then(Value::as_object_mut) {
                                        for (device_key, device_value) in device { target.insert(device_key.clone(), device_value.clone()); }
                                    }
                                }
                                _ => { entry.payload.insert(key.clone(), value.clone()); }
                            }
                        }
                    }
                    if let Some(kind) = entry.payload.remove("__type") {
                        if let Some(kind) = kind.as_str() { entry.r#type = kind.to_string(); }
                    }
                }
            }
            for entry in new_entries.iter_mut() {
                if (entry.r#type == "binary_sensor" || entry.r#type == "sensor")
                    && entry.payload.get("entity_category").and_then(Value::as_str) == Some("config")
                {
                    entry.payload.insert("entity_category".into(), json!("diagnostic"));
                }
                if entry.r#type == "event" {
                    entry.payload.remove("entity_category");
                }
                let ha_name = expose.homeassistant.get("name");
                if entry.payload.get("device_class").is_some() && ha_name.is_none() {
                    entry.payload.remove("name");
                }
            }
        }

        if entity.is_device() {
            let continuous = self.context.settings_value()["advanced"]["last_seen"]
                .as_str()
                .unwrap_or("disable")
                .to_string();
            if continuous != "disable" {
                let mut payload = Map::new();
                payload.insert("name".into(), json!("Last seen"));
                payload.insert("value_template".into(), Value::String(state_value_template("last_seen")));
                payload.insert("icon".into(), json!("mdi:clock"));
                payload.insert("enabled_by_default".into(), Value::Bool(false));
                payload.insert("entity_category".into(), json!("diagnostic"));
                if continuous.starts_with("ISO_8601") {
                    payload.insert("device_class".into(), json!("timestamp"));
                }
                entries.push(Entry { r#type: "sensor".into(), object_id: "last_seen".into(), payload });
            }
            if entity.definition().is_some_and(|definition| definition.ota.is_some()) {
                let base = self.context.settings_value()["mqtt"]["base_topic"].as_str().unwrap_or("zigbee2mqtt").to_string();
                let mut payload = Map::new();
                payload.insert("name".into(), Value::Null);
                payload.insert("entity_picture".into(), json!("https://github.com/Koenkk/zigbee2mqtt/raw/master/images/logo.png"));
                payload.insert("state_topic".into(), Value::Bool(true));
                payload.insert("device_class".into(), json!("firmware"));
                payload.insert("entity_category".into(), json!("config"));
                payload.insert("command_topic".into(), json!(format!("{base}/bridge/request/device/ota_update/update")));
                payload.insert(
                    "payload_install".into(),
                    json!(format!(r#"{{"id": "{}"}}"#, entity.entity_id().key())),
                );
                payload.insert(
                    "value_template".into(),
                    Value::String(state_value_template_body(
                        "update",
                        r#"{"latest_version":"{{ value_json['update']['latest_version'] }}","installed_version":"{{ value_json['update']['installed_version'] }}","update_percentage":{{ value_json['update'].get('progress', 'null') }},"in_progress":{{ (value_json['update'].get('state') == 'updating')|lower }}}"#,
                    )),
                );
                entries.push(Entry { r#type: "update".into(), object_id: "update".into(), payload });
            }
        }
        let scene_endpoints: Vec<Arc<zigbee_apiarist::controller::model::endpoint::Endpoint>> = if entity.is_device() {
            entity.zh_device().map(|zh| zh.endpoints()).unwrap_or_default()
        } else {
            entity.zh_group().map(|group| group.members()).unwrap_or_default()
        };
        for endpoint in scene_endpoints {
            let group_id = if entity.is_device() { 0 } else { entity.entity_id().key().parse::<u64>().unwrap_or(0) };
            let scenes = if entity.is_device() {
                utils::get_scenes_endpoint(&endpoint)
            } else {
                entity.zh_group().map(|group| utils::get_scenes_group(&group)).unwrap_or_default()
            };
            for scene in scenes {
                let scene_id = scene.get("id").and_then(Value::as_u64).unwrap_or(0);
                let scene_name = scene.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
                let mut payload = Map::new();
                payload.insert("name".into(), json!(scene_name));
                payload.insert("state_topic".into(), Value::Bool(false));
                payload.insert("command_topic".into(), Value::Bool(true));
                payload.insert(
                    "payload_on".into(),
                    json!(format!(r#"{{ "scene_recall": {scene_id} }}"#)),
                );
                payload.insert(
                    "object_id_postfix".into(),
                    json!(format!(
                        "_{}",
                        scene_name.replace(' ', "_").to_lowercase()
                    )),
                );
                entries.push(Entry { r#type: "scene".into(), object_id: format!("scene_{scene_id}"), payload });
            }
        }

        entries
    }

    fn device_payload(&self, entity: &Arc<dyn Entity>) -> Value {
        let mut payload = Map::new();
        let identifier = if entity.is_group() {
            format!("zigbee2mqtt_group_{}", entity.entity_id().key())
        } else {
            format!("zigbee2mqtt_{}", entity.entity_id().key())
        };
        payload.insert("identifiers".into(), json!([identifier]));
        let settings = self.context.settings_value();
        let mut name = entity.name();
        if let Some(overridden) = entity
            .options()
            .get("homeassistant")
            .and_then(|ha| ha.get("name"))
            .and_then(Value::as_str)
        {
            name = overridden.to_string();
        }
        payload.insert("name".into(), Value::String(name));
        payload.insert(
            "sw_version".into(),
            json!(format!("Zigbee2MQTT {}", self.version.lock().unwrap())),
        );
        if entity.is_device() {
            if let Some(definition) = entity.definition() {
                payload.insert("model".into(), json!(definition.description));
                payload.insert("model_id".into(), json!(definition.model));
                payload.insert("manufacturer".into(), json!(definition.vendor));
            }
            if let Some(build) = entity.zh_device().and_then(|zh| zh.software_build_id()) {
                payload.insert("sw_version".into(), json!(build));
            }
        } else if entity.is_group() {
            payload.insert("model".into(), json!("Group"));
            payload.insert("manufacturer".into(), json!("Zigbee2MQTT"));
        }
        let url = settings["frontend"]["url"].as_str().unwrap_or("");
        if !url.is_empty() {
            let postfix = if entity.is_device() {
                format!("/#/device/0/{}/info", entity.entity_id().key())
            } else {
                format!("/#/group/0/{}", entity.entity_id().key())
            };
            payload.insert("configuration_url".into(), json!(format!("{url}{postfix}")));
        }
        let bridge = self.bridge_identifier.lock().unwrap().clone();
        if !bridge.is_empty() && bridge != identifier {
            payload.insert("via_device".into(), json!(bridge));
        }
        Value::Object(payload)
    }

    fn discovery_topic_for(&self, entry: &Entry, entity: &Arc<dyn Entity>) -> String {
        let key = if entity.is_device() {
            entity.entity_id().key()
        } else {
            format!("group_{}", entity.entity_id().key())
        };
        format!("{}/{}/{}/config", entry.r#type, key, entry.object_id)
    }

    async fn discover(&self, entity: Arc<dyn Entity>, publish: bool) {
        if entity.is_group() && entity.members_devices().is_empty() {
            return;
        }
        if entity.is_device()
            && (entity.definition().is_none()
                || !entity.interviewed()
                || entity
                    .options()
                    .get("homeassistant")
                    .map(|value| !value.as_bool().unwrap_or(false))
                    .unwrap_or(false))
        {
            return;
        }

        let id = entity.entity_id().key();
        let discovered = self.get_discovered(&id);
        let last_topics: Vec<String> = {
            let mut state = discovered.lock().unwrap();
            state.discovered = true;
            state.messages.keys().cloned().collect()
        };
        let mut new_topics = HashSet::new();
        let mut to_publish: Vec<(String, String)> = Vec::new();

        let mut configs = self.get_configs(&entity);
        let ha_options = entity.options().get("homeassistant").cloned().unwrap_or(Value::Null);
        if let Value::Object(overrides) = &ha_options {
            configs.retain(|config| !matches!(overrides.get(&config.object_id), Some(Value::Null)));
            for config in &mut configs {
                if let Some(over) = overrides.get(&config.object_id).and_then(Value::as_object) {
                    if config.r#type == "switch" && over.get("type").and_then(Value::as_str) == Some("light") {
                        self.context.logger.lock().unwrap().warning(
                            &format!("Overriding the Home Assistant type of '{}' from switch to light is deprecated and will be removed in a future release, use 'Show as' in Home Assistant instead", entity.name()),
                            "z2m",
                        );
                    }
                    if let Some(object_id) = over.get("object_id").and_then(Value::as_str) {
                        config.object_id = object_id.to_string();
                    }
                    if let Some(kind) = over.get("type").and_then(Value::as_str) {
                        config.r#type = kind.to_string();
                    }
                }
            }
        }
        let device_payload = self.device_payload(&entity);
        let settings = self.context.settings_value();
        let base_topic = format!(
            "{}/{}",
            settings["mqtt"]["base_topic"]
                .as_str()
                .unwrap_or("zigbee2mqtt"),
            entity.name()
        );
        let availability_enabled =
            utils::is_availability_enabled_for_entity(entity.as_ref(), &settings);

        for config in configs {
            let mut payload = config.payload.clone();
            let mut state_topic = base_topic.clone();
            if let Some(postfix) = payload
                .remove("state_topic_postfix")
                .and_then(|value| value.as_str().map(str::to_string))
            {
                state_topic = format!("{state_topic}/{postfix}");
            }
            if payload
                .get("state_topic")
                .map(|value| !value.is_boolean())
                .unwrap_or(false)
                || payload.get("state_topic").and_then(Value::as_bool) == Some(true)
                || !payload.contains_key("state_topic")
            {
                payload.insert("state_topic".into(), Value::String(state_topic.clone()));
            } else {
                payload.remove("state_topic");
            }

            payload.insert("device".into(), device_payload.clone());
            if entity.is_device() {
                if let Some(endpoint) = entity.endpoint(None) {
                    let member_key = format!("{}_{}_{}", entity.entity_id().key(), endpoint.ID, config.r#type);
                    self.group_member_lookup.lock().unwrap().insert(member_key, payload["unique_id"].as_str().unwrap_or_default().to_string());
                }
            } else if entity.is_group() {
                let mut member_ids = Vec::new();
                for member in entity.zh_group().map(|group| group.members()).unwrap_or_default() {
                    let key = format!("{}_{}_{}", member.device_ieee_address.lock().unwrap(), member.ID, config.r#type);
                    if let Some(unique) = self.group_member_lookup.lock().unwrap().get(&key) {
                        member_ids.push(Value::String(unique.clone()));
                    }
                }
                if !member_ids.is_empty() {
                    payload.insert("group".into(), Value::Array(member_ids));
                }
            }
            payload.insert("origin".into(), self.origin.lock().unwrap().clone());

            let mut object_id = device_payload["name"]
                .as_str()
                .unwrap_or_default()
                .replace(' ', "_")
                .to_lowercase();
            if config.object_id.starts_with(&config.r#type) && config.object_id.contains('_') {
                if let Some((_, suffix)) = config.object_id.split_once('_') {
                    object_id = format!("{object_id}_{suffix}");
                }
            } else if !config.object_id.starts_with(&config.r#type) {
                object_id = format!("{object_id}_{}", config.object_id);
            }
            payload.insert("object_id".into(), json!(object_id));
            payload.insert(
                "default_entity_id".into(),
                json!(format!("{}.{}", config.r#type, object_id)),
            );
            payload.insert(
                "unique_id".into(),
                json!(format!(
                    "{}_{}_{}",
                    entity
                        .options()
                        .get("ID")
                        .and_then(Value::as_str)
                        .unwrap_or(&id),
                    config.object_id,
                    settings["mqtt"]["base_topic"]
                        .as_str()
                        .unwrap_or("zigbee2mqtt")
                )),
            );

            if payload.get("availability").is_none()
                || payload.get("availability").and_then(Value::as_bool) != Some(false)
            {
                let mut availability = vec![
                    json!({"topic": format!("{}/bridge/state", settings["mqtt"]["base_topic"].as_str().unwrap_or("zigbee2mqtt")), "value_template": "{{ value_json.state }}"}),
                ];
                if availability_enabled && !entity.is_device()
                    || availability_enabled && entity.is_device()
                {
                    availability.push(json!({"topic": format!("{base_topic}/availability"), "value_template": "{{ value_json.state }}"}));
                }
                payload.insert("availability".into(), Value::Array(availability));
                payload.insert("availability_mode".into(), json!("all"));
            }

            let command_prefix = payload
                .remove("command_topic_prefix")
                .and_then(|value| value.as_str().map(|value| format!("{value}/")))
                .unwrap_or_default();
            let command_postfix = payload
                .remove("command_topic_postfix")
                .and_then(|value| value.as_str().map(|value| format!("/{value}")))
                .unwrap_or_default();
            if payload.get("position_topic").is_some() {
                payload.insert("position_topic".into(), Value::String(state_topic.clone()));
            }
            if payload.get("tilt_status_topic").is_some() {
                payload.insert(
                    "tilt_status_topic".into(),
                    Value::String(state_topic.clone()),
                );
            }
            if payload.get("command_topic").and_then(Value::as_bool) == Some(true) {
                payload.insert(
                    "command_topic".into(),
                    json!(format!("{base_topic}/{command_prefix}set{command_postfix}")),
                );
            }
            let command_topic = format!("{base_topic}/{command_prefix}set{command_postfix}");
            if payload.get("set_position_topic").is_some() {
                payload.insert(
                    "set_position_topic".into(),
                    Value::String(command_topic.clone()),
                );
            }
            for key in [
                "mode_state_topic",
                "current_temperature_topic",
                "temperature_state_topic",
                "temperature_low_state_topic",
                "temperature_high_state_topic",
                "fan_mode_state_topic",
                "swing_mode_state_topic",
                "preset_mode_state_topic",
                "action_topic",
                "current_humidity_topic",
                "percentage_state_topic",
            ] {
                if payload.get(key).is_some() {
                    payload.insert(key.to_string(), Value::String(state_topic.clone()));
                }
            }
            if payload.get("tilt_command_topic").is_some() {
                payload.insert(
                    "tilt_command_topic".into(),
                    Value::String(format!("{base_topic}/{command_prefix}set/tilt")),
                );
            }
            for (key, postfix) in [
                ("mode_command_topic", "system_mode".to_string()),
                ("fan_mode_command_topic", "fan_mode".to_string()),
                ("swing_mode_command_topic", "swing_mode".to_string()),
            ] {
                if payload.get(key).is_some() {
                    payload.insert(
                        key.to_string(),
                        Value::String(format!("{base_topic}/{command_prefix}set/{postfix}")),
                    );
                }
            }
            for key in [
                "temperature_command_topic",
                "temperature_low_command_topic",
                "temperature_high_command_topic",
                "percentage_command_topic",
                "preset_mode_command_topic",
            ] {
                if let Some(value) = payload.get(key).and_then(Value::as_str) {
                    let value = value.to_string();
                    payload.insert(
                        key.to_string(),
                        Value::String(format!("{base_topic}/{command_prefix}set/{value}")),
                    );
                }
            }

            if entity.is_device() {
                if let Some(definition) = entity.definition() {
                    if let Some(override_payload) = definition.override_ha_discovery_payload.clone() {
                        let options = entity.options();
                        let mut value = Value::Object(payload.clone());
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            override_payload(&mut value, &options);
                        }));
                        if result.is_ok() {
                            if let Value::Object(updated) = value {
                                payload = updated;
                            }
                        }
                        if result.is_err() {
                            self.context.logger.lock().unwrap().error(
                                "Failed to override HA discovery payload",
                                "z2m",
                            );
                        }
                    }
                }
            }

            let topic = self.discovery_topic_for(&config, &entity);
            let payload_str = stringify(&Value::Object(payload));
            new_topics.insert(topic.clone());
            let should_publish = {
                let mut state = discovered.lock().unwrap();
                let should_publish = state
                    .messages
                    .get(&topic)
                    .map(|(previous, published)| !*published || previous != &payload_str)
                    .unwrap_or(true);
                if should_publish {
                    state
                        .messages
                        .insert(topic.clone(), (payload_str.clone(), publish));
                }
                should_publish
            };
            if should_publish && publish {
                to_publish.push((topic, payload_str));
            }
        }

        let mut cleared = Vec::new();
        {
            let mut state = discovered.lock().unwrap();
            for topic in last_topics {
                if !new_topics.contains(&topic) {
                    state.messages.remove(&topic);
                    if publish && !topic.starts_with("device_automation") {
                        cleared.push(topic);
                    }
                }
            }
        }

        let mqtt = self.context.mqtt.clone();
        let discovery_topic = self.discovery_topic.clone();
        for (topic, payload) in to_publish {
            mqtt.lock()
                .await
                .publish(
                    &topic,
                    payload,
                    MqttPublishOptions {
                        retain: true,
                        qos: 1,
                        base_topic: Some(discovery_topic.clone()),
                        ..Default::default()
                    },
                )
                .await;
        }
        for topic in cleared {
            mqtt.lock()
                .await
                .publish(
                    &topic,
                    "",
                    MqttPublishOptions {
                        retain: true,
                        qos: 1,
                        base_topic: Some(discovery_topic.clone()),
                        ..Default::default()
                    },
                )
                .await;
        }
    }

    /// Upstream discovery-topic cleanup (`homeassistant.ts:2078-2144`).
    async fn on_discovery_message(self: &Arc<Self>, data: &Value) {
        let topic = data.get("topic").and_then(Value::as_str).unwrap_or("");
        let Some(regex) = self.discovery_regex() else {
            return;
        };
        let Some(captures) = regex.captures(topic) else {
            return;
        };
        let component = captures.get(1).map(|value| value.as_str()).unwrap_or("");
        let id_part = captures.get(2).map(|value| value.as_str()).unwrap_or("");
        let object_id = captures.get(3).map(|value| value.as_str()).unwrap_or("");
        let is_device_automation = component == "device_automation";
        let text = data.get("message").and_then(Value::as_str).unwrap_or("");
        let Ok(message) = serde_json::from_str::<Value>(text) else {
            return;
        };
        let base = format!(
            "{}/",
            self.context.settings_value()["mqtt"]["base_topic"]
                .as_str()
                .unwrap_or("zigbee2mqtt")
        );
        let valid_base = if is_device_automation {
            message
                .get("topic")
                .and_then(Value::as_str)
                .is_some_and(|topic| topic.starts_with(&base))
        } else {
            message
                .get("availability")
                .and_then(|availability| availability.get(0))
                .and_then(|entry| entry.get("topic"))
                .and_then(Value::as_str)
                .is_some_and(|topic| topic.starts_with(&base))
        };
        if !valid_base {
            return;
        }

        let id = id_part
            .strip_prefix("group_")
            .unwrap_or(id_part)
            .to_string();
        let bridge_id = self.bridge_identifier.lock().unwrap().clone();
        let entity = if id == self.context.zigbee.coordinator_ieee_addr().unwrap_or_default()
            || bridge_id.ends_with(&id)
        {
            None
        } else {
            self.context.zigbee.resolve_entity(&id)
        };
        let mut clear = entity
            .as_ref()
            .map(|entity| entity.is_device() && entity.definition().is_none())
            .unwrap_or(true);

        if let Some(entity) = &entity {
            let key = object_id.split('_').next().unwrap_or("");
            let trigger_topic = format!("{base}/{}/{key}", entity.name());
            if is_device_automation
                && message.get("topic").and_then(Value::as_str) == Some(trigger_topic.as_str())
            {
                self.get_discovered(&entity.entity_id().key())
                    .lock()
                    .unwrap()
                    .triggers
                    .insert(object_id.to_string());
            }
        }
        let discovery_topic = topic
            .strip_prefix(&format!("{}/", self.discovery_topic))
            .unwrap_or(topic)
            .to_string();
        if !clear && !is_device_automation {
            if let Some(entity) = &entity {
                clear = !self
                    .get_discovered(&entity.entity_id().key())
                    .lock()
                    .unwrap()
                    .messages
                    .contains_key(&discovery_topic);
            }
        }
        clear = clear
            || entity.as_ref().is_some_and(|entity| {
                entity
                    .options()
                    .get("homeassistant")
                    .is_some_and(|value| value.as_bool() == Some(false))
            });

        if clear {
            self.context.logger.lock().unwrap().debug(
                &format!("Clearing outdated Home Assistant config '{topic}'"),
                "z2m",
            );
            self.context
                .mqtt
                .lock()
                .await
                .publish(
                    &discovery_topic,
                    "",
                    MqttPublishOptions {
                        retain: true,
                        qos: 1,
                        base_topic: Some(self.discovery_topic.clone()),
                        ..Default::default()
                    },
                )
                .await;
        } else if let Some(entity) = entity {
            self.get_discovered(&entity.entity_id().key())
                .lock()
                .unwrap()
                .messages
                .insert(discovery_topic, (stringify(&message), true));
        }
    }

    /// Upstream `getBridgeEntity` (`homeassistant.ts:2304-2420`).
    async fn publish_bridge(self: &Arc<Self>) {
        let settings = self.context.settings_value();
        let base = settings["mqtt"]["base_topic"].as_str().unwrap_or("zigbee2mqtt");
        let coordinator_ieee = self
            .context
            .zigbee
            .coordinator_ieee_addr()
            .unwrap_or_default();
        let version = self
            .context
            .zigbee
            .get_coordinator_version()
            .await
            .unwrap_or(Value::Null);
        let revision = version
            .get("meta")
            .and_then(|meta| meta.get("revision"))
            .and_then(Value::as_u64)
            .map(|value| value.to_string())
            .unwrap_or_default();
        let coordinator_type = version
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        let device = json!({
            "identifiers": [format!("zigbee2mqtt_bridge_{coordinator_ieee}")],
            "name": "Zigbee2MQTT Bridge",
            "model": "Bridge",
            "manufacturer": "Zigbee2MQTT",
            "sw_version": self.version.lock().unwrap().clone(),
            "hw_version": format!("{coordinator_type} {revision}"),
        });
        let origin = self.origin.lock().unwrap().clone();

        let entries: Vec<(&str, &str, Value)> = vec![
            ("binary_sensor", "connection_state", json!({
                "name": "Connection state", "device_class": "connectivity", "entity_category": "diagnostic",
                "state_topic": format!("{base}/bridge/state"), "value_template": "{{ value_json.state }}",
                "payload_on": "online", "payload_off": "offline", "availability": false,
            })),
            ("binary_sensor", "restart_required", json!({
                "name": "Restart required", "device_class": "problem", "entity_category": "diagnostic",
                "enabled_by_default": false, "state_topic": format!("{base}/bridge/info"),
                "value_template": "{{ value_json.restart_required }}", "payload_on": true, "payload_off": false,
            })),
            ("button", "restart", json!({
                "name": "Restart", "device_class": "restart", "state_topic": false,
                "command_topic": format!("{base}/bridge/request/restart"), "payload_press": "",
            })),
            ("select", "log_level", json!({
                "name": "Log level", "entity_category": "config", "state_topic": format!("{base}/bridge/info"),
                "value_template": "{{ value_json.log_level | lower }}",
                "command_topic": format!("{base}/bridge/request/options"),
                "command_template": "{\"options\": {\"advanced\": {\"log_level\": \"{{ value }}\"} } }",
                "options": ["error", "warning", "info", "debug"],
            })),
            ("sensor", "version", json!({
                "name": "Version", "icon": "mdi:zigbee", "entity_category": "diagnostic",
                "state_topic": format!("{base}/bridge/info"), "value_template": "{{ value_json.version }}",
            })),
            ("sensor", "coordinator_version", json!({
                "name": "Coordinator version", "icon": "mdi:chip", "entity_category": "diagnostic",
                "enabled_by_default": false, "state_topic": format!("{base}/bridge/info"),
                "value_template": "{{ value_json.coordinator.meta.revision }}",
            })),
            ("sensor", "network_map", json!({
                "name": "Network map", "entity_category": "diagnostic", "enabled_by_default": false,
                "state_topic": format!("{base}/bridge/response/networkmap"),
                "value_template": "{{ now().strftime('%Y-%m-%d %H:%M:%S') }}",
                "json_attributes_topic": format!("{base}/bridge/response/networkmap"),
                "json_attributes_template": "{{ value_json.data.value | tojson }}",
            })),
            ("switch", "permit_join", json!({
                "name": "Permit join", "icon": "mdi:human-greeting-proximity",
                "state_topic": format!("{base}/bridge/info"),
                "value_template": "{{ value_json.permit_join | lower }}",
                "command_topic": format!("{base}/bridge/request/permit_join"),
                "state_on": "true", "state_off": "false",
                "payload_on": "{\"time\": 254}", "payload_off": "{\"time\": 0}",
            })),
        ];

        for (kind, object_id, mut payload) in entries {
            if let Value::Object(map) = &mut payload {
                map.insert("device".into(), device.clone());
                map.insert("origin".into(), origin.clone());
                if map.get("availability").and_then(Value::as_bool) != Some(false) {
                    map.insert(
                        "availability".into(),
                        json!([{"topic": format!("{base}/bridge/state"), "value_template": "{{ value_json.state }}"}]),
                    );
                    map.insert("availability_mode".into(), json!("all"));
                } else {
                    map.remove("availability");
                }
                map.insert(
                    "unique_id".into(),
                    json!(format!("{coordinator_ieee}_{object_id}_{base}")),
                );
                map.insert("object_id".into(), json!(format!("bridge_{object_id}")));
                map.insert(
                    "default_entity_id".into(),
                    json!(format!("{kind}.bridge_{object_id}")),
                );
            }
            self.context
                .mqtt
                .lock()
                .await
                .publish(
                    &format!("{kind}/{coordinator_ieee}/{object_id}/config"),
                    stringify(&payload),
                    MqttPublishOptions {
                        retain: true,
                        qos: 1,
                        base_topic: Some(self.discovery_topic.clone()),
                        ..Default::default()
                    },
                )
                .await;
        }
    }

    async fn publish_device_trigger_discover(
        self: &Arc<Self>,
        device: &Arc<dyn Entity>,
        key: &str,
        value: &str,
        force: bool,
    ) {
        if matches!(device.options().get("homeassistant"), Some(Value::Bool(false)) | Some(Value::Null)) {
            return;
        }
        let discovered = self.get_discovered(&device.entity_id().key());
        let discovered_key = format!("{key}_{value}");
        {
            let state = discovered.lock().unwrap();
            if state.triggers.contains(&discovered_key) && !force {
                return;
            }
        }
        let base = self.context.settings_value()["mqtt"]["base_topic"]
            .as_str()
            .unwrap_or("zigbee2mqtt")
            .to_string();
        let topic = format!(
            "device_automation/{}/{key}_{value}/config",
            device.entity_id().key()
        );
        let payload = json!({
            "automation_type": "trigger",
            "type": key,
            "subtype": value,
            "payload": value,
            "topic": format!("{base}/{}/{key}", device.name()),
            "device": self.device_payload(device),
            "origin": self.origin.lock().unwrap().clone(),
        });
        self.context
            .mqtt
            .lock()
            .await
            .publish(
                &topic,
                stringify(&payload),
                MqttPublishOptions {
                    retain: true,
                    qos: 1,
                    base_topic: Some(self.discovery_topic.clone()),
                    ..Default::default()
                },
            )
            .await;
        discovered
            .lock()
            .unwrap()
            .triggers
            .insert(discovered_key);
    }

    /// Upstream `onPublishEntityState` (`homeassistant.ts:1520-1620`).
    async fn on_publish_entity_state(self: &Arc<Self>, data: Value) {
        let name = data
            .get("entity")
            .and_then(|entity| entity.get("name"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let Some(entity) = self.context.zigbee.resolve_entity(&name) else {
            return;
        };
        let message = data.get("message").cloned().unwrap_or(Value::Null);

        if entity.is_device() {
            let discovered = self.get_discovered(&entity.entity_id().key());
            let topics: Vec<String> = discovered
                .lock()
                .unwrap()
                .messages
                .keys()
                .cloned()
                .collect();
            for topic in topics {
                let Some((component, rest)) = topic.split_once('/') else {
                    continue;
                };
                if component != "light" && component != "cover" {
                    continue;
                }
                let object_id = rest.split('/').next().unwrap_or("");
                let endpoint = object_id
                    .rsplit_once('_')
                    .map(|(_, endpoint)| endpoint)
                    .unwrap_or("");
                if endpoint.is_empty() {
                    continue;
                }
                let suffix = format!("_{endpoint}");
                let mut payload = Map::new();
                for (key, value) in message.as_object().cloned().unwrap_or_default() {
                    if let Some(stripped) = key.strip_suffix(&suffix) {
                        payload.insert(stripped.to_string(), value);
                    }
                }
                if !payload.is_empty() {
                    self.context
                        .mqtt
                        .lock()
                        .await
                        .publish(
                            &format!("{name}/{endpoint}"),
                            stringify(&Value::Object(payload)),
                            Default::default(),
                        )
                        .await;
                }
            }
        }

        if self.legacy_action_sensor
            && message
                .get("action")
                .and_then(Value::as_str)
                .is_some_and(|action| !action.is_empty())
        {
            (self.context.publish_entity_state)(
                entity.clone(),
                json!({"action": ""}),
                None,
            )
            .await;
        }

        if entity.is_device() && entity.definition().is_some() {
            if let Some(action) = message.get("action").and_then(Value::as_str) {
                if !action.is_empty() {
                    self.publish_device_trigger_discover(&entity, "action", action, false)
                        .await;
                    if self.context.settings_value()["advanced"]["output"].as_str()
                        == Some("json")
                    {
                        self.context
                            .mqtt
                            .lock()
                            .await
                            .publish(&format!("{name}/action"), action, Default::default())
                            .await;
                    }
                }
            }
        }
    }

    async fn clear_entity(&self, entity: &Arc<dyn Entity>) {
        let id = entity.entity_id().key();
        let discovered = self.discovered.lock().unwrap().remove(&id);
        if let Some(discovered) = discovered {
            let topics: Vec<String> = discovered
                .lock()
                .unwrap()
                .messages
                .keys()
                .cloned()
                .collect();
            for topic in topics {
                self.context
                    .mqtt
                    .lock()
                    .await
                    .publish(
                        &topic,
                        "",
                        MqttPublishOptions {
                            retain: true,
                            qos: 1,
                            base_topic: Some(self.discovery_topic.clone()),
                            ..Default::default()
                        },
                    )
                    .await;
            }
        }
    }
}

impl Extension for HomeAssistant {
    fn name(&self) -> &'static str {
        "HomeAssistant"
    }

    fn context(&self) -> &ExtensionContext {
        &self.context
    }

    fn start(self: Arc<Self>) -> ExtensionFuture {
        Box::pin(async move {
            let settings = self.context.settings_value();
            if !settings["homeassistant"]["enabled"]
                .as_bool()
                .unwrap_or(false)
            {
                return Err(
                    "Home Assistant extension created with setting 'enabled: false'".to_string(),
                );
            }
            if settings["advanced"]["output"].as_str() == Some("attribute") {
                return Err(
                    "Home Assistant integration requires 'output: json' under 'advanced'"
                        .to_string(),
                );
            }
            if settings["homeassistant"]["discovery_topic"].as_str()
                == settings["mqtt"]["base_topic"].as_str()
            {
                return Err(format!(
                    "'homeassistant.discovery_topic' cannot not be equal to the 'mqtt.base_topic' (got '{}')",
                    settings["mqtt"]["base_topic"].as_str().unwrap_or("zigbee2mqtt")
                ));
            }

            *self.version.lock().unwrap() = utils::get_zigbee2mqtt_version().await["version"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            *self.origin.lock().unwrap() = json!({
                "name": "Zigbee2MQTT",
                "sw": self.version.lock().unwrap().clone(),
                "url": "https://www.zigbee2mqtt.io",
            });

            let coordinator = self
                .context
                .zigbee
                .first_coordinator_endpoint()
                .map(|endpoint| endpoint.device_ieee_address.lock().unwrap().clone())
                .unwrap_or_default();
            let bridge_id = if coordinator.is_empty() {
                String::new()
            } else {
                format!("zigbee2mqtt_bridge_{coordinator}")
            };
            *self.bridge_identifier.lock().unwrap() = bridge_id;

            self.publish_bridge().await;

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_entity_removed(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(ieee) = data
                            .get("entity")
                            .and_then(|entity| entity.get("ieeeAddr"))
                            .and_then(Value::as_str)
                        {
                            if let Some(device) = this.context.zigbee.resolve_device(ieee) {
                                this.clear_entity(&(device as Arc<dyn Entity>)).await;
                            }
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_entity_renamed(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(ieee) = data
                            .get("entity")
                            .and_then(|entity| entity.get("ieeeAddr"))
                            .and_then(Value::as_str)
                        {
                            if let Some(device) = this.context.zigbee.resolve_device(ieee) {
                                let device = device as Arc<dyn Entity>;
                                this.clear_entity(&device).await;
                                this.discover(device.clone(), true).await;
                                let triggers: Vec<String> = this
                                    .get_discovered(&device.entity_id().key())
                                    .lock()
                                    .unwrap()
                                    .triggers
                                    .iter()
                                    .cloned()
                                    .collect();
                                for trigger in triggers {
                                    if let Some((key, value)) = trigger.split_once('_') {
                                        this.publish_device_trigger_discover(
                                            &device, key, value, true,
                                        )
                                        .await;
                                    }
                                }
                            }
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_device_joined(
                self.context.key,
                Arc::new(move |data| {
                    let ieee = data
                        .get("device")
                        .and_then(|device| device.get("ieeeAddr"))
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(device) =
                            ieee.and_then(|ieee| this.context.zigbee.resolve_device(&ieee))
                        {
                            let id = device.entity_id().key();
                            let discovered = this.get_discovered(&id).lock().unwrap().discovered;
                            if !discovered {
                                this.discover(device as Arc<dyn Entity>, true).await;
                            }
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_device_message(
                self.context.key,
                Arc::new(move |data| {
                    let ieee = data
                        .get("device")
                        .and_then(|device| device.get("ieeeAddr"))
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(device) =
                            ieee.and_then(|ieee| this.context.zigbee.resolve_device(&ieee))
                        {
                            let id = device.entity_id().key();
                            let discovered = this.get_discovered(&id).lock().unwrap().discovered;
                            if !discovered {
                                this.discover(device as Arc<dyn Entity>, true).await;
                            }
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_mqtt_message(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        this.on_discovery_message(&data).await;
                        if data.get("topic").and_then(Value::as_str)
                            == Some(this.status_topic.as_str())
                            && data
                                .get("message")
                                .and_then(Value::as_str)
                                .map(str::to_lowercase)
                                .as_deref()
                                == Some("online")
                        {
                            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                            this.context
                                .mqtt
                                .lock()
                                .await
                                .publish(
                                    "bridge/state",
                                    stringify(&json!({"state": "online"})),
                                    MqttPublishOptions {
                                        retain: true,
                                        qos: 1,
                                        ..Default::default()
                                    },
                                )
                                .await;
                            for entity in this.context.zigbee.devices_and_groups_iterator() {
                                let id = entity.entity_id();
                                let exists = this.context.state.lock().unwrap().exists(&id);
                                if entity.is_device() && exists {
                                    let payload = this.context.state.lock().unwrap().get(&id);
                                    (this.context.publish_entity_state)(
                                        entity,
                                        payload,
                                        Some("publishCached".to_string()),
                                    )
                                    .await;
                                }
                            }
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_publish_entity_state(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move { this.on_publish_entity_state(data).await });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_group_members_changed(
                self.context.key,
                Arc::new(move |data| {
                    let id = data
                        .get("group")
                        .and_then(|group| group.get("ID"))
                        .map(|id| id.as_str().map(str::to_string).unwrap_or_else(|| id.to_string()));
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(group) = id
                            .and_then(|id| this.context.zigbee.group_by_id(id.parse().unwrap_or(0)))
                        {
                            this.discover(group as Arc<dyn Entity>, true).await;
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_scenes_changed(
                self.context.key,
                Arc::new(move |data| {
                    let name = data
                        .get("entity")
                        .and_then(|entity| entity.get("name"))
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    let this = this.clone();
                    tokio::spawn(async move {
                        let Some(entity) = name
                            .and_then(|name| this.context.zigbee.resolve_entity(&name))
                        else {
                            return;
                        };
                        let discovered = this.get_discovered(&entity.entity_id().key());
                        let topics: Vec<String> = discovered
                            .lock()
                            .unwrap()
                            .messages
                            .keys()
                            .filter(|topic| topic.starts_with("scene"))
                            .cloned()
                            .collect();
                        for topic in topics {
                            discovered.lock().unwrap().messages.remove(&topic);
                            this.context
                                .mqtt
                                .lock()
                                .await
                                .publish(
                                    &topic,
                                    "",
                                    MqttPublishOptions {
                                        retain: true,
                                        qos: 1,
                                        base_topic: Some(this.discovery_topic.clone()),
                                        ..Default::default()
                                    },
                                )
                                .await;
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        this.discover(entity, true).await;
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_entity_options_changed(
                self.context.key,
                Arc::new(move |data| {
                    let name = data
                        .get("entity")
                        .and_then(|entity| entity.get("name"))
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(entity) = name
                            .and_then(|name| this.context.zigbee.resolve_entity(&name))
                        {
                            this.discover(entity, true).await;
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_exposes_changed(
                self.context.key,
                Arc::new(move |data| {
                    let ieee = data
                        .get("device")
                        .and_then(|device| device.get("ieeeAddr"))
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(ieee) = ieee {
                            if let Some(device) = this.context.zigbee.resolve_device(&ieee) {
                                this.discover(device as Arc<dyn Entity>, true).await;
                                return;
                            }
                        }
                        for device in this
                            .context
                            .zigbee
                            .devices_iterator(utils::device_not_coordinator)
                        {
                            this.discover(device as Arc<dyn Entity>, true).await;
                        }
                    });
                }),
            );

            self.context
                .mqtt
                .lock()
                .await
                .subscribe(&self.status_topic, 0)
                .await;

            for device in self
                .context
                .zigbee
                .devices_iterator(utils::device_not_coordinator)
            {
                self.discover(device as Arc<dyn Entity>, false).await;
            }
            for group in self.context.zigbee.groups_iterator(|_| true) {
                self.discover(group as Arc<dyn Entity>, false).await;
            }
            for device in self
                .context
                .zigbee
                .devices_iterator(utils::device_not_coordinator)
            {
                self.discover(device as Arc<dyn Entity>, true).await;
            }
            for group in self.context.zigbee.groups_iterator(|_| true) {
                self.discover(group as Arc<dyn Entity>, true).await;
            }

            Ok(())
        })
    }

    fn adjust_message_before_publish(&self, entity: &dyn Entity, message: &mut Value) {
        self.adjust_color(message);
        if entity.is_device()
            && entity
                .definition()
                .is_some_and(|definition| definition.ota.is_some())
            && message
                .get("update")
                .and_then(|update| update.get("latest_version"))
                .is_none()
        {
            message["update"] = json!({"installed_version": -1, "latest_version": -1});
        }
    }
}

impl HomeAssistant {
    fn discovery_regex(&self) -> Option<regex::Regex> {
        regex::Regex::new(&format!(
            "{}/(.*)/(.*)/(.*)/config",
            regex::escape(&self.discovery_topic)
        ))
        .ok()
    }

    fn adjust_color(&self, message: &mut Value) {
        if let Some(color) = message.get_mut("color").and_then(Value::as_object_mut) {
            if let Some(hue) = color.get("hue").cloned() {
                color.insert("h".to_string(), hue);
            }
            if let Some(saturation) = color.get("saturation").cloned() {
                color.insert("s".to_string(), saturation);
            }
        }
    }
}

/// Upstream `ACTION_PATTERNS` (`homeassistant.ts:31-37`).
const ACTION_PATTERNS: [&str; 5] = [
    "^(?<button>(?:button_)?[a-z0-9]+)_(?<action>(?:press|hold|single|double)(?:_release)?)$",
    "^(?<action>recall|scene)_(?<scene>[0-9]+)$",
    "^(?<actionPrefix>region_)(?<region>[1-9]|10)_(?<action>enter|leave|occupied|unoccupied)$",
    "^(?<action>dial_rotate)_(?<direction>left|right)_(?<speed>step|slow|fast)$",
    "^(?<action>brightness_step|color_temperature_step|color_saturation_step|color_hue_step)(?:_(?<direction>up|down))?$",
];

fn action_value_template() -> String {
    let patterns = ACTION_PATTERNS
        .iter()
        .map(|pattern| {
            let rust_pattern = pattern.replace("?<", "?P<");
            let groups = action_groups(pattern);
            let groups = groups
                .iter()
                .map(|group| format!("\"{group}\""))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{\"pattern\": '{rust_pattern}', \"groups\": [{groups}]}}")
        })
        .collect::<Vec<_>>()
        .join(",\n");

    format!(
        "{{% set patterns = [\n{patterns}\n] %}}\n\
{{% set action_value = value_json.action|default('') %}}\n\
{{% set ns = namespace(r=[('action', action_value)]) %}}\n\
{{% for p in patterns %}}\n\
  {{% set m = action_value|regex_findall(p.pattern) %}}\n\
  {{% if m[0] is undefined %}}{{% continue %}}{{% endif %}}\n\
  {{% for key, value in zip(p.groups, m[0]) %}}\n\
    {{% set ns.r = ns.r|rejectattr(0, 'eq', key)|list + [(key, value)] %}}\n\
  {{% endfor %}}\n\
{{% endfor %}}\n\
{{% if (ns.r|selectattr(0, 'eq', 'actionPrefix')|first) is defined %}}\n\
  {{% set ns.r = ns.r|rejectattr(0, 'eq', 'action')|list + [('action', ns.r|selectattr(0, 'eq', 'actionPrefix')|map(attribute=1)|first + ns.r|selectattr(0, 'eq', 'action')|map(attribute=1)|first)] %}}\n\
{{% endif %}}\n\
{{% set ns.r = ns.r + [('event_type', ns.r|selectattr(0, 'eq', 'action')|map(attribute=1)|first)] %}}\n\
{{{{dict.from_keys(ns.r|rejectattr(0, 'in', ('action', 'actionPrefix'))|reject('eq', ('event_type', None))|reject('eq', ('event_type', '')))|to_json}}}}"
    )
}

fn action_groups(pattern: &str) -> Vec<String> {
    let mut groups = Vec::new();
    let mut rest = pattern;
    while let Some(index) = rest.find("?<") {
        rest = &rest[index + 2..];
        if let Some(end) = rest.find('>') {
            let name = &rest[..end];
            if name.chars().all(|c| c.is_ascii_alphabetic()) {
                groups.push(name.to_string());
            }
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }
    groups
}

/// Upstream `parseActionValue` (`homeassistant.ts:2425-2450`) for trigger names.
fn action_patterns() -> &'static Vec<regex::Regex> {
    static PATTERNS: OnceLock<Vec<regex::Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        ACTION_PATTERNS
            .iter()
            .map(|pattern| {
                regex::Regex::new(&pattern.replace("?<", "?P<")).expect("valid action pattern")
            })
            .collect()
    })
}

/// Upstream `parseActionValue` (`homeassistant.ts:2425-2450`).
fn parse_action_value(action: &str) -> Value {
    for pattern in action_patterns() {
        if let Some(captures) = pattern.captures(action) {
            let mut groups = Map::new();
            for name in pattern.capture_names().flatten() {
                if let Some(value) = captures.name(name) {
                    groups.insert(name.to_string(), json!(value.as_str()));
                }
            }
            if let Some(prefix) = groups
                .remove("actionPrefix")
                .and_then(|value| value.as_str().map(str::to_string))
            {
                if let Some(value) = groups.get("action").and_then(Value::as_str) {
                    groups.insert("action".into(), json!(format!("{prefix}{value}")));
                }
            }
            return Value::Object(groups);
        }
    }

    static WILDCARD_SCENE: OnceLock<regex::Regex> = OnceLock::new();
    let scene = WILDCARD_SCENE.get_or_init(|| {
        regex::Regex::new(r"^(?P<action>recall|scene)_\*(?:_(?P<endpoint>e1|e2|s1|s2))?$")
            .expect("valid wildcard scene pattern")
    });
    if let Some(captures) = scene.captures(action) {
        return json!({
            "action": captures.name("action").map(|value| value.as_str()).unwrap_or(""),
            "scene": "wildcard",
        });
    }

    static WILDCARD_REGION: OnceLock<regex::Regex> = OnceLock::new();
    let region = WILDCARD_REGION.get_or_init(|| {
        regex::Regex::new(r"^(?P<actionPrefix>region_)\*_(?P<action>enter|leave|occupied|unoccupied)$")
            .expect("valid wildcard region pattern")
    });
    if let Some(captures) = region.captures(action) {
        let prefix = captures
            .name("actionPrefix")
            .map(|value| value.as_str())
            .unwrap_or("");
        let action = captures.name("action").map(|value| value.as_str()).unwrap_or("");
        return json!({
            "action": format!("{prefix}{action}"),
            "region": "wildcard",
        });
    }

    json!({"action": action})
}
fn prepare_action_event_types(values: &[String]) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    for value in values {
        let parsed = parse_action_value(value);
        if let Some(action) = parsed.get("action").and_then(Value::as_str) {
            if !action.contains('*') && !result.iter().any(|existing| existing == action) {
                result.push(action.to_string());
            }
        }
    }
    result
}
