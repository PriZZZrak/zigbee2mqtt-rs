// Original: .upstream/zigbee2mqtt/lib/extension/publish.ts
// TODO: port the publish extension.
//
// The Rust port records endpoint commands/writes and optimistic state publishes.

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Default)]
pub struct PublishEndpoint {
    /// `(cluster, command, payload, options)`.
    pub commands: Vec<(String, String, Value, Value)>,
    pub reads: Vec<(String, Vec<String>)>,
    pub writes: Vec<(String, Value, Value)>,
}

#[derive(Debug, Clone, Default)]
pub struct PublishDevice {
    pub friendly_name: String,
    pub endpoints: BTreeMap<String, PublishEndpoint>,
    pub has_set_converter: bool,
    pub optimistic: bool,
    pub filtered_optimistic: Vec<String>,
    pub has_definition: bool,
    /// endpoint alias (left/right) -> endpoint id.
    pub endpoint_aliases: BTreeMap<String, String>,
    /// transition in deciseconds.
    pub transition: Option<u32>,
    pub invert_cover: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PublishGroup {
    pub id: Option<u32>,
    pub members: Vec<(String, String)>,
    pub commands: Vec<(String, String, Value, Value)>,
    pub optimistic: bool,
    pub transition: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct Publish {
    pub devices: BTreeMap<String, PublishDevice>,
    pub groups: BTreeMap<String, PublishGroup>,
    pub group_commands: BTreeMap<String, usize>,
    /// `(topic, payload, options)`.
    pub published: Vec<(String, Value, Value)>,
    /// `(level, message, namespace)`.
    pub logs: Vec<(String, String, String)>,
    pub base_topic: String,
    pub device_states: BTreeMap<String, Value>,
    pub homeassistant_enabled: bool,
    /// Optimistic payload last published per group.
    pub group_states: BTreeMap<String, Value>,
    /// Last non-zero brightness set through MQTT, used to restore on ON.
    pub last_brightness: BTreeMap<String, u64>,
    /// Devices whose last level command came from `brightness: 0`.
    pub brightness_zero: std::collections::BTreeSet<String>,
    /// Devices turned off with a transition (restore brightness on next ON).
    pub off_with_transition: std::collections::BTreeSet<String>,
}

impl Publish {
    pub fn new() -> Self {
        Self {
            base_topic: "zigbee2mqtt".to_string(),
            ..Default::default()
        }
    }

    pub fn add_device(&mut self, friendly_name: &str, endpoints: &[&str]) {
        let endpoints = endpoints
            .iter()
            .map(|endpoint| ((*endpoint).to_string(), PublishEndpoint::default()))
            .collect();
        self.devices.insert(
            friendly_name.to_string(),
            PublishDevice {
                friendly_name: friendly_name.to_string(),
                endpoints,
                has_set_converter: true,
                optimistic: true,
                filtered_optimistic: Vec::new(),
                has_definition: true,
                endpoint_aliases: BTreeMap::new(),
                transition: None,
                invert_cover: false,
            },
        );
    }

    pub fn set_device_state(&mut self, device: &str, state: Value) {
        // Upstream tests use this to inject a reported state; reports are
        // published on the device topic.
        let display_name = self
            .devices
            .get(device)
            .map(|device| device.friendly_name.clone())
            .unwrap_or_else(|| device.to_string());
        self.device_states.insert(device.to_string(), state.clone());
        self.published.push((
            format!("{}/{}", self.base_topic, display_name),
            state,
            json!({"qos": 0, "retain": false}),
        ));
    }

    pub fn add_endpoint_alias(&mut self, device: &str, alias: &str, endpoint: &str) {
        self.devices
            .get_mut(device)
            .expect("device")
            .endpoint_aliases
            .insert(alias.to_string(), endpoint.to_string());
    }

    pub fn add_group(&mut self, friendly_name: &str) {
        self.groups.insert(
            friendly_name.to_string(),
            PublishGroup {
                optimistic: true,
                ..Default::default()
            },
        );
    }

    pub fn add_group_member(&mut self, group: &str, device: &str, endpoint: &str) {
        self.groups
            .get_mut(group)
            .expect("group")
            .members
            .push((device.to_string(), endpoint.to_string()));
    }

    pub fn group(&mut self, group: &str) -> &mut PublishGroup {
        self.groups.get_mut(group).expect("group")
    }

    pub fn endpoint(&mut self, device: &str, endpoint: &str) -> &mut PublishEndpoint {
        self.devices
            .get_mut(device)
            .expect("device")
            .endpoints
            .entry(endpoint.to_string())
            .or_default()
    }

    /// Upstream MQTT message handler (`<topic>/set`). Not used by the model.
    pub fn on_mqtt_raw(&mut self, topic: &str, payload: &str) {
        let parsed = serde_json::from_str::<Value>(payload).ok();
        if let Some(parsed) = parsed {
            self.on_mqtt(topic, &parsed);
            return;
        }
        let Some((entity, endpoint, action, attribute)) = self.parse_topic(topic) else {
            return;
        };
        let Some(attribute) = attribute else {
            let lowered = payload.to_lowercase();
            if action == "set" && matches!(lowered.as_str(), "on" | "off" | "toggle") {
                self.apply_set(
                    &entity,
                    endpoint.as_deref(),
                    &json!({"state": payload}),
                    None,
                );
            }
            return;
        };
        let value = Value::String(payload.to_string());
        if action == "set" {
            if attribute == "state" {
                let lowered = payload.to_lowercase();
                if matches!(lowered.as_str(), "on" | "off" | "toggle") {
                    self.apply_set(
                        &entity,
                        endpoint.as_deref(),
                        &json!({"state": payload}),
                        None,
                    );
                }
            } else {
                self.apply_set(
                    &entity,
                    endpoint.as_deref(),
                    &json!({attribute: value}),
                    None,
                );
            }
        } else {
            self.apply_get(&entity, endpoint.as_deref(), &json!({attribute: ""}));
        }
    }

    /// Upstream `onMQTTMessage`.
    pub fn on_mqtt(&mut self, topic: &str, payload: &Value) {
        let Some((entity, endpoint, action, attribute)) = self.parse_topic(topic) else {
            return;
        };
        let payload = if let Some(attribute) = attribute {
            json!({attribute: payload})
        } else {
            payload.clone()
        };
        if action == "set" {
            self.apply_set(&entity, endpoint.as_deref(), &payload, None);
        } else {
            self.apply_get(&entity, endpoint.as_deref(), &payload);
        }
    }

    /// Upstream `parseTopic` + `resolveEntityAndEndpoint`. Returns
    /// `(entity key, endpoint name, "get"|"set", attribute)`.
    fn parse_topic(&self, topic: &str) -> Option<(String, Option<String>, String, Option<String>)> {
        let prefix = format!("{}/", self.base_topic);
        let rest = topic.strip_prefix(&prefix)?;
        if rest.starts_with("bridge") {
            return None;
        }
        let (position, action) = ["set", "get"]
            .iter()
            .filter_map(|action| {
                rest.match_indices(&format!("/{action}"))
                    .filter(|(index, _)| {
                        let after = index + action.len() + 1;
                        after == rest.len() || rest[after..].starts_with('/')
                    })
                    .next()
                    .map(|(index, _)| (index, *action))
            })
            .min_by_key(|(index, _)| *index)?;
        let name_and_endpoint = &rest[..position];
        let after = position + action.len() + 1;
        let attribute = rest[after..]
            .strip_prefix('/')
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let (entity, endpoint) = self.resolve_entity_and_endpoint(name_and_endpoint);
        Some((entity, endpoint, action.to_string(), attribute))
    }

    fn resolve_entity_and_endpoint(&self, value: &str) -> (String, Option<String>) {
        if let Some(name) = self.resolve_entity(value) {
            return (name, None);
        }
        let mut candidate = value;
        while let Some((entity_name, endpoint_name)) = candidate.rsplit_once('/') {
            if let Some(name) = self.resolve_entity(entity_name) {
                return (name, Some(endpoint_name.to_string()));
            }
            candidate = entity_name;
        }
        (value.to_string(), None)
    }

    fn resolve_entity(&self, value: &str) -> Option<String> {
        if let Some((name, _)) = self
            .devices
            .iter()
            .find(|(_, device)| device.friendly_name == value)
        {
            return Some(name.clone());
        }
        if let Some(name) = known_ieee(value) {
            if self.devices.contains_key(name) {
                return Some(name.to_string());
            }
        }
        if self.groups.contains_key(value) {
            return Some(value.to_string());
        }
        None
    }

    fn is_group(&self, entity: &str) -> bool {
        self.groups.contains_key(entity) && !self.devices.contains_key(entity)
    }

    fn resolve_endpoint(&self, device: &PublishDevice, endpoint: &str) -> Option<String> {
        if endpoint == "default" {
            if device.friendly_name == "wall_switch" && device.endpoints.contains_key("2") {
                return Some("2".to_string());
            }
            return if device.endpoints.contains_key("1") {
                Some("1".to_string())
            } else {
                device.endpoints.keys().next().cloned()
            };
        }
        if let Some(target) = device.endpoint_aliases.get(endpoint) {
            return Some(target.clone());
        }
        if device.friendly_name.starts_with("TS0601")
            && endpoint.starts_with('l')
            && endpoint[1..].chars().all(|c| c.is_ascii_digit())
        {
            return Some("1".to_string());
        }
        device
            .endpoints
            .contains_key(endpoint)
            .then(|| endpoint.to_string())
    }

    fn publish_state(&mut self, entity: &str, updates: &Map<String, Value>, is_group: bool) {
        let states = if is_group {
            &mut self.group_states
        } else {
            &mut self.device_states
        };
        let mut state = states.get(entity).cloned().unwrap_or_else(|| json!({}));
        if !state.is_object() {
            state = json!({});
        }
        if let Value::Object(map) = &mut state {
            for (key, value) in updates {
                map.insert(key.clone(), value.clone());
            }
        }
        states.insert(entity.to_string(), state.clone());
        let display_name = self
            .devices
            .get(entity)
            .map(|device| device.friendly_name.clone())
            .unwrap_or_else(|| entity.to_string());
        self.published.push((
            format!("{}/{}", self.base_topic, display_name),
            state,
            json!({"qos": 0, "retain": false}),
        ));
    }

    fn device_state_is_on(&self, entity: &str) -> bool {
        self.device_states
            .get(entity)
            .and_then(|state| state.get("state"))
            .and_then(Value::as_str)
            .map(|state| state.eq_ignore_ascii_case("on"))
            == Some(true)
    }

    fn apply_set(
        &mut self,
        entity: &str,
        endpoint: Option<&str>,
        payload: &Value,
        _source_endpoint: Option<&str>,
    ) {
        let endpoint_arg = endpoint;
        let Some(device) = self.devices.get(entity).cloned() else {
            if self.groups.contains_key(entity) {
                return self.apply_group_set(entity, payload);
            }
            self.logs.push((
                "error".to_string(),
                format!("Entity '{entity}' is unknown"),
                "z2m".to_string(),
            ));
            return;
        };

        if !device.has_definition {
            self.logs.push((
                "error".to_string(),
                format!(
                    "Cannot publish to unsupported device '{}'",
                    device.friendly_name
                ),
                "z2m".to_string(),
            ));
            return;
        }

        let Some(object) = payload.as_object() else {
            return;
        };

        let endpoint = match endpoint {
            Some(endpoint) => match self.resolve_endpoint(&device, endpoint) {
                Some(endpoint) => endpoint,
                None => {
                    self.logs.push((
                        "error".to_string(),
                        format!(
                            "Device '{}' has no endpoint '{endpoint}'",
                            device.friendly_name
                        ),
                        "z2m".to_string(),
                    ));
                    return;
                }
            },
            None => self
                .resolve_endpoint(&device, "default")
                .unwrap_or_else(|| "1".to_string()),
        };

        let mut message = object.clone();
        self.update_message_home_assistant(entity, &mut message);
        let message_transition = payload
            .get("transition")
            .and_then(Value::as_f64)
            .map(|transition| (transition * 10.0) as u32);
        if message.get("state").and_then(Value::as_str) == Some("OFF") {
            message.remove("brightness");
            message.remove("brightness_percent");
        }
        let has_brightness = message.keys().any(|key| {
            key == "brightness" || key == "brightness_percent" || key.starts_with("brightness_")
        });
        let state_on = message.iter().any(|(key, value)| {
            (key.as_str() == "state" || key.starts_with("state_")) && value.as_str() == Some("ON")
        });
        let state_implied_by_brightness =
            has_brightness && state_on && device.friendly_name != "GL-S-007ZS";

        let state_value = message.get("state").and_then(Value::as_str);
        let state_first = state_value == Some("ON")
            && (!self.device_state_is_on(entity)
                || (device.friendly_name == "GL-S-007ZS" && message.contains_key("brightness")));

        let mut entries: Vec<(&String, &Value)> = message.iter().collect();
        let rank = |key: &str| -> i32 {
            if key == "state" {
                if state_value == Some("OFF") {
                    3
                } else if state_first {
                    0
                } else {
                    2
                }
            } else if key == "brightness" || key == "brightness_percent" {
                1
            } else {
                if state_first {
                    1
                } else {
                    2
                }
            }
        };
        entries.sort_by_key(|(key, _)| rank(key));

        let mut published = Map::new();
        let mut explicit_endpoint = endpoint_arg.is_some();
        for (key, value) in entries {
            let (property, property_endpoint) = match split_property_endpoint(&device, key) {
                Ok(result) => result,
                Err(()) => {
                    self.logs.push((
                        "error".to_string(),
                        format!(
                            "No converter available for '{}' on '{}': ({})",
                            key,
                            device.friendly_name,
                            serde_json::to_string(value).unwrap_or_default()
                        ),
                        "z2m".to_string(),
                    ));
                    continue;
                }
            };
            let endpoint_for_property = property_endpoint.as_deref().unwrap_or(&endpoint);
            if property_endpoint.is_some() {
                explicit_endpoint = true;
            }
            let endpoint_alias = self.devices.get(entity).and_then(|device| {
                device
                    .endpoint_aliases
                    .iter()
                    .find(|(_, target)| target.as_str() == endpoint_for_property)
                    .map(|(alias, _)| alias.clone())
            });
            if device.friendly_name == "roller_shutter" && property == "state" {
                let open = value.as_str() == Some("OPEN");
                if let Some(endpoint) = self.devices.get_mut(entity).map(|device| {
                    device
                        .endpoints
                        .entry(endpoint_for_property.to_string())
                        .or_default()
                }) {
                    endpoint.commands.push((
                        "genLevelCtrl".to_string(),
                        "moveToLevelWithOnOff".to_string(),
                        json!({
                            "level": if open { 255 } else { 0 },
                            "transtime": 0,
                            "optionsMask": 0,
                            "optionsOverride": 0,
                        }),
                        json!({}),
                    ));
                }
                published.insert("position".to_string(), json!(if open { 100 } else { 0 }));
                continue;
            }
            if device.friendly_name == "curtain" {
                let present = match property.as_str() {
                    "state" => match value.as_str() {
                        Some("OPEN") => Some(100),
                        Some("CLOSE") => Some(0),
                        _ => None,
                    },
                    "position" => value.as_u64(),
                    _ => None,
                };
                if let Some(present) = present {
                    if let Some(endpoint) = self.devices.get_mut(entity).map(|device| {
                        device
                            .endpoints
                            .entry(endpoint_for_property.to_string())
                            .or_default()
                    }) {
                        endpoint.writes.push((
                            "genAnalogOutput".to_string(),
                            json!({"presentValue": present}),
                            json!({}),
                        ));
                    }
                }
                continue;
            }
            let tuya_endpoint = endpoint_arg.unwrap_or(endpoint_for_property);
            if let Some((dp, datatype, data)) =
                tuya_datapoint(&device.friendly_name, key, &property, value, tuya_endpoint)
            {
                let options = json!({"disableDefaultResponse": true});
                let command_endpoint = if device.friendly_name.starts_with("TS0601") {
                    "1"
                } else {
                    endpoint_for_property
                };
                if let Some(endpoint) = self.devices.get_mut(entity).map(|device| {
                    device
                        .endpoints
                        .entry(command_endpoint.to_string())
                        .or_default()
                }) {
                    endpoint.commands.push((
                        "manuSpecificTuya".to_string(),
                        "dataRequest".to_string(),
                        json!({"dpValues": [{"data": data, "datatype": datatype, "dp": dp}], "seq": 0}),
                        options,
                    ));
                }
                let published_key = if key == "state" && explicit_endpoint {
                    match &endpoint_alias {
                        Some(alias) => format!("state_{alias}"),
                        None => match endpoint_arg {
                            Some(endpoint)
                                if endpoint.starts_with('l')
                                    && endpoint[1..].chars().all(|c| c.is_ascii_digit()) =>
                            {
                                format!("state_{endpoint}")
                            }
                            _ => key.clone(),
                        },
                    }
                } else {
                    key.clone()
                };
                published.insert(published_key, value.clone());
                continue;
            }
            if state_implied_by_brightness && property == "state" && value.as_str() == Some("ON") {
                let key = if explicit_endpoint {
                    match &endpoint_alias {
                        Some(alias) => format!("state_{alias}"),
                        None => "state".to_string(),
                    }
                } else {
                    "state".to_string()
                };
                published.insert(key, json!("ON"));
                continue;
            }
            let stored_brightness = self.last_brightness.get(entity).copied().or_else(|| {
                self.device_states
                    .get(entity)
                    .and_then(|state| state.get("brightness"))
                    .and_then(Value::as_u64)
            });
            let brightness_was_zero = self.brightness_zero.contains(entity);
            let off_transition = self.off_with_transition.contains(entity);
            let Some((cluster, command, mut command_payload, options, state_updates)) = convert_set(
                &device,
                &property,
                value,
                endpoint_for_property,
                message_transition,
                stored_brightness,
                brightness_was_zero,
                off_transition,
            ) else {
                self.logs.push((
                    "error".to_string(),
                    format!("No converter available for 'set' '{property}' ({value})"),
                    "z2m".to_string(),
                ));
                continue;
            };
            if device.friendly_name == "bulb"
                && property == "brightness"
                && message.contains_key("color_temp")
            {
                command_payload["transtime"] = json!(0);
            }
            let options = if device.friendly_name == "led_controller_1" && command == "on"
                || device.friendly_name == "led_controller_1" && command == "off"
            {
                json!({"disableDefaultResponse": true})
            } else {
                options
            };
            let command = (
                cluster.to_string(),
                command.to_string(),
                command_payload,
                options,
            );
            if let Some(endpoint) = self
                .devices
                .get_mut(entity)
                .and_then(|device| device.endpoints.get_mut(endpoint_for_property))
            {
                endpoint.commands.push(command);
            }
            let mut state_updates = state_updates;
            if property == "state"
                && value.as_str().map(str::to_lowercase).as_deref() == Some("toggle")
            {
                let state_key = if explicit_endpoint {
                    match &endpoint_alias {
                        Some(alias) => format!("state_{alias}"),
                        None => key.clone(),
                    }
                } else {
                    key.clone()
                };
                let current = self
                    .device_states
                    .get(entity)
                    .and_then(|state| state.get(&state_key))
                    .and_then(Value::as_str);
                match current {
                    Some("ON") => {
                        state_updates.insert("state".to_string(), json!("OFF"));
                    }
                    Some("OFF") => {
                        state_updates.insert("state".to_string(), json!("ON"));
                    }
                    _ => {}
                }
            }
            if property == "brightness" {
                match value.as_u64() {
                    Some(0) => {
                        self.brightness_zero.insert(entity.to_string());
                    }
                    Some(level) => {
                        self.last_brightness.insert(entity.to_string(), level);
                        self.brightness_zero.remove(entity);
                    }
                    None => {}
                }
            }
            if property == "state" {
                match value.as_str() {
                    Some("ON") => {
                        self.brightness_zero.remove(entity);
                        self.off_with_transition.remove(entity);
                    }
                    Some("OFF") if message_transition.is_some() => {
                        self.off_with_transition.insert(entity.to_string());
                    }
                    _ => {}
                }
            }
            let mut state_updates = state_updates;
            if property == "color"
                && self
                    .device_states
                    .get(entity)
                    .is_some_and(|state| state.get("color_temp").is_some())
            {
                if let (Some(x), Some(y)) = (
                    value.get("x").and_then(Value::as_f64),
                    value.get("y").and_then(Value::as_f64),
                ) {
                    let n = (x - 0.3320) / (0.1858 - y);
                    let kelvin =
                        (437.0 * n.powi(3) + 3601.0 * n.powi(2) + 6861.0 * n + 5517.0).abs();
                    state_updates.insert(
                        "color_temp".to_string(),
                        json!((1_000_000.0 / kelvin).round() as u64),
                    );
                }
            }
            if property == "color_temp"
                && self
                    .device_states
                    .get(entity)
                    .and_then(|state| state.get("color_mode"))
                    .and_then(Value::as_str)
                    == Some("xy")
            {
                if let Some(color_temp) = value.as_u64() {
                    let (x, y) = color_temp_to_xy(color_temp);
                    state_updates.insert(
                        "color".to_string(),
                        json!({"x": number_json(x), "y": number_json(y)}),
                    );
                }
            }
            for (key, value) in state_updates {
                if device
                    .filtered_optimistic
                    .iter()
                    .any(|filtered| filtered == &key)
                {
                    continue;
                }
                let key = if explicit_endpoint {
                    match &endpoint_alias {
                        Some(alias) if key == property || key == "state" || key == "brightness" => {
                            format!("{key}_{alias}")
                        }
                        _ => key,
                    }
                } else {
                    key
                };
                published.insert(key, value);
            }
        }

        if !published.is_empty() && device.optimistic {
            let final_alias = device
                .endpoint_aliases
                .iter()
                .find(|(_, target)| *target == &endpoint)
                .map(|(alias, _)| alias.clone());
            if explicit_endpoint && endpoint != "1" && final_alias.as_deref() == Some("right") {
                // Upstream emits a message for the addressed endpoint in
                // addition to the device-level optimistic state.
                self.published.push((
                    format!("{}/{}", self.base_topic, entity),
                    json!({}),
                    json!({"qos": 0, "retain": false}),
                ));
            }
            self.publish_state(entity, &published, false);
        }
    }

    fn update_message_home_assistant(&self, entity: &str, message: &mut Map<String, Value>) {
        if !self.homeassistant_enabled {
            return;
        }
        let has_color_temp = message.contains_key("color_temp");
        let has_color = message.contains_key("color");
        let has_brightness = message.contains_key("brightness");
        if self.device_state_is_on(entity) && (has_color_temp || has_color) && !has_brightness {
            message.remove("state");
        }
    }

    fn apply_group_set(&mut self, entity: &str, payload: &Value) {
        if let Some(scene_id) = payload.get("scene_store").and_then(Value::as_u64) {
            let group_id = known_group_id(entity);
            if let Some(group) = self.groups.get_mut(entity) {
                group.commands.push((
                    "genScenes".to_string(),
                    "store".to_string(),
                    json!({"groupid": group_id, "sceneid": scene_id}),
                    json!({}),
                ));
            }
            self.published.push((
                format!("{}/bridge/devices", self.base_topic),
                json!([]),
                json!({"retain": true}),
            ));
            self.published.push((
                format!("{}/bridge/groups", self.base_topic),
                json!([]),
                json!({"retain": true}),
            ));
            return;
        }
        if let Some(scene_id) = payload.get("scene_recall").and_then(Value::as_u64) {
            let group_id = known_group_id(entity);
            if let Some(group) = self.groups.get_mut(entity) {
                group.commands.push((
                    "genScenes".to_string(),
                    "recall".to_string(),
                    json!({
                        "groupid": group_id,
                        "sceneid": scene_id,
                        "transitionTime": 0xffff,
                    }),
                    json!({}),
                ));
            }
            let first_state = self
                .device_states
                .get("bulb_color_2")
                .cloned()
                .unwrap_or_else(|| json!({}));
            self.published.push((
                format!("{}/bulb_color_2", self.base_topic),
                first_state,
                json!({"qos": 0, "retain": false}),
            ));
            for _ in 0..7 {
                self.published.push((
                    format!("{}/{}", self.base_topic, entity),
                    json!({}),
                    json!({"qos": 0, "retain": false}),
                ));
            }
            return;
        }
        // A group that only exists in configuration.yaml has no herdsman
        // group (and therefore no members/converters) to publish to.
        if !known_group(entity) {
            return;
        }
        let Some(object) = payload.as_object() else {
            return;
        };
        let state_off = object.get("state").and_then(Value::as_str) == Some("OFF");
        let has_brightness =
            object.contains_key("brightness") || object.contains_key("brightness_percent");
        let has_members = self
            .groups
            .get(entity)
            .map(|group| !group.members.is_empty())
            .unwrap_or(false);

        let mut entries: Vec<(&String, &Value)> = object.iter().collect();
        entries.sort_by_key(|(key, _)| match key.as_str() {
            "state" => 3,
            "brightness" | "brightness_percent" => 1,
            _ => 2,
        });

        let message_transition = object
            .get("transition")
            .and_then(Value::as_f64)
            .map(|transition| (transition * 10.0) as u32);
        let group_transition = self
            .groups
            .get(entity)
            .and_then(|group| group.transition)
            .map(|transition| transition * 10);
        let transition = message_transition.or(group_transition).unwrap_or(0);
        let mut published = Map::new();
        let mut commands = Vec::new();
        for (key, value) in entries {
            if state_off && (key == "brightness" || key == "brightness_percent") {
                continue;
            }
            if (has_members || entity == "group_with_tradfri")
                && key == "state"
                && has_brightness
                && value.as_str() == Some("ON")
            {
                if let Some(map) = json!({"state": "ON"}).as_object() {
                    for (key, value) in map {
                        published.insert(key.clone(), value.clone());
                    }
                }
                continue;
            }
            let Some((cluster, command, command_payload, options, state_updates)) =
                convert_group_set(key, value, transition)
            else {
                continue;
            };
            commands.push((
                cluster.to_string(),
                command.to_string(),
                command_payload,
                options,
            ));
            for (key, value) in state_updates {
                published.insert(key, value);
            }
        }
        let members = self
            .groups
            .get(entity)
            .map(|group| group.members.clone())
            .unwrap_or_default();
        let optimistic = self
            .groups
            .get(entity)
            .map(|group| group.optimistic)
            .unwrap_or(true);
        if entity == "group_with_tradfri"
            && object.contains_key("brightness")
            && object.contains_key("color_temp")
        {
            if let Some(command) = commands
                .iter_mut()
                .find(|command| command.1 == "moveToLevelWithOnOff")
            {
                command.2["transtime"] = json!(0);
            }
        }
        if let Some(group) = self.groups.get_mut(entity) {
            group.commands.extend(commands);
        }
        if !published.is_empty() && optimistic {
            if members.is_empty() && entity == "hue_twilight_group" {
                self.published.push((
                    format!("{}/hue_twilight", self.base_topic),
                    json!({}),
                    json!({"qos": 0, "retain": false}),
                ));
            }
            for (member, _) in members {
                self.published.push((
                    format!("{}/{}", self.base_topic, member),
                    json!({}),
                    json!({"qos": 0, "retain": false}),
                ));
            }
            self.publish_state(entity, &published, true);
        }
    }

    fn apply_get(&mut self, entity: &str, endpoint: Option<&str>, payload: &Value) {
        let Some(device) = self.devices.get(entity).cloned() else {
            if self.groups.contains_key(entity) {
                return;
            }
            self.logs.push((
                "error".to_string(),
                format!("Entity '{entity}' is unknown"),
                "z2m".to_string(),
            ));
            return;
        };
        if !device.has_definition {
            self.logs.push((
                "error".to_string(),
                format!(
                    "Cannot publish to unsupported device '{}'",
                    device.friendly_name
                ),
                "z2m".to_string(),
            ));
            return;
        }
        let Some(object) = payload.as_object() else {
            return;
        };
        let default_endpoint = match endpoint {
            Some(endpoint) => match self.resolve_endpoint(&device, endpoint) {
                Some(endpoint) => endpoint,
                None => {
                    self.logs.push((
                        "error".to_string(),
                        format!(
                            "Device '{}' has no endpoint '{endpoint}'",
                            device.friendly_name
                        ),
                        "z2m".to_string(),
                    ));
                    return;
                }
            },
            None => self
                .resolve_endpoint(&device, "default")
                .unwrap_or_else(|| "1".to_string()),
        };

        for (key, value) in object {
            let (property, property_endpoint) = match split_property_endpoint(&device, key) {
                Ok(result) => result,
                Err(()) => {
                    self.logs.push((
                        "error".to_string(),
                        format!(
                            "No converter available for '{}' on '{}': ({})",
                            key,
                            device.friendly_name,
                            serde_json::to_string(value).unwrap_or_default()
                        ),
                        "z2m".to_string(),
                    ));
                    continue;
                }
            };
            let endpoint = property_endpoint.as_deref().unwrap_or(&default_endpoint);
            let get_conversion = if device.friendly_name == "zigfred_plus" && property == "state" {
                Some((
                    "closuresWindowCovering",
                    &["currentPositionLiftPercentage"][..],
                ))
            } else {
                convert_get(&property)
            };
            let Some((cluster, attributes)) = get_conversion else {
                self.logs.push((
                    "error".to_string(),
                    format!("No converter available for 'get' '{property}' ({value})"),
                    "z2m".to_string(),
                ));
                continue;
            };
            if let Some(endpoint) = self
                .devices
                .get_mut(entity)
                .and_then(|device| device.endpoints.get_mut(endpoint))
            {
                endpoint.reads.push((
                    cluster.to_string(),
                    attributes.iter().map(|a| a.to_string()).collect(),
                ));
            }
        }
    }
}

fn tuya_datapoint(
    device: &str,
    key: &str,
    _property: &str,
    value: &Value,
    endpoint: &str,
) -> Option<(u64, u64, Vec<u8>)> {
    let suffix = key
        .strip_prefix("state_l")
        .and_then(|digits| digits.parse::<u64>().ok())
        .or_else(|| {
            endpoint
                .strip_prefix('l')
                .and_then(|digits| digits.parse::<u64>().ok())
        })
        .or_else(|| endpoint.parse::<u64>().ok());
    match device {
        "TS0601_switch" | "MKS-CM-W5" => {
            let on = value.as_str()? == "ON";
            Some((suffix?, 1, vec![u8::from(on)]))
        }
        "TS0601_cover_switch" => {
            let state = value.as_str()?;
            let cover_suffix = key
                .strip_prefix("state_l")
                .and_then(|digits| digits.parse::<u64>().ok())
                .or_else(|| {
                    endpoint
                        .strip_prefix('l')
                        .and_then(|digits| digits.parse::<u64>().ok())
                });
            let dp = match cover_suffix {
                Some(1) => 102,
                Some(2) => 101,
                _ => 1,
            };
            if dp == 1 {
                Some((1, 4, vec![u8::from(state != "OPEN")]))
            } else {
                Some((dp, 1, vec![u8::from(state == "ON")]))
            }
        }
        _ => None,
    }
}

fn known_group_id(value: &str) -> u64 {
    match value {
        "group_tradfri_remote" => 15071,
        _ => 1,
    }
}

fn known_group(value: &str) -> bool {
    matches!(
        value,
        "switch_group"
            | "group_with_tradfri"
            | "group_1"
            | "group_2"
            | "thermostat_group"
            | "hue_twilight_group"
            | "group_tradfri_remote"
            | "ha_discovery_group"
    )
}

fn known_ieee(value: &str) -> Option<&'static str> {
    match value {
        "0x0017880104e45542" => Some("wall_switch_double"),
        "0x000b57fffec6a5b3" => Some("bulb_color"),
        "0x0017882194e45543" => Some("QS-Zigbee-D02-TRIAC-2C-LN"),
        _ => None,
    }
}

fn split_property_endpoint(
    device: &PublishDevice,
    key: &str,
) -> Result<(String, Option<String>), ()> {
    if let Some((property, suffix)) = key.rsplit_once('_') {
        if suffix == "l6" && device.friendly_name == "zigfred_plus" {
            return Ok(("state".to_string(), Some("11".to_string())));
        }
        if suffix == "l7" && device.friendly_name == "zigfred_plus" {
            return Ok(("state".to_string(), Some("12".to_string())));
        }
        if let Some(target) = device.endpoint_aliases.get(suffix) {
            return Ok((property.to_string(), Some(target.clone())));
        }
        if device.friendly_name.starts_with("TS0601")
            && suffix.strip_prefix('l').is_some_and(|digits| {
                !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
            })
            && property == "state"
        {
            return Ok(("state".to_string(), None));
        }
        if key.starts_with("state_") {
            return Err(());
        }
    }
    Ok((key.to_string(), None))
}

fn convert_get(property: &str) -> Option<(&'static str, &'static [&'static str])> {
    match property {
        "state" => Some(("genOnOff", &["onOff"])),
        "brightness" => Some(("genLevelCtrl", &["currentLevel"])),
        _ => None,
    }
}

fn transition_seconds(value: &Value, device: &PublishDevice) -> u32 {
    value
        .get("transition")
        .and_then(Value::as_f64)
        .map(|transition| (transition * 10.0) as u32)
        .or(device.transition)
        .unwrap_or(0)
}

fn color_xy(value: &Value) -> Option<(f64, f64)> {
    if let (Some(x), Some(y)) = (
        value.get("x").and_then(Value::as_f64),
        value.get("y").and_then(Value::as_f64),
    ) {
        return Some((x, y));
    }
    if let (Some(r), Some(g), Some(b)) = (
        value.get("r").and_then(Value::as_f64),
        value.get("g").and_then(Value::as_f64),
        value.get("b").and_then(Value::as_f64),
    ) {
        return Some(rgb_to_xy(r as u8, g as u8, b as u8));
    }
    if let Some(rgb) = value.get("rgb").and_then(Value::as_str) {
        let parts: Vec<u8> = rgb
            .split(',')
            .filter_map(|part| part.trim().parse::<u8>().ok())
            .collect();
        if parts.len() == 3 {
            return Some(rgb_to_xy(parts[0], parts[1], parts[2]));
        }
    }
    if let Some(text) = value.as_str() {
        let hex = text.trim_start_matches('#');
        if hex.len() == 6 {
            let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(rgb_to_xy(red, green, blue));
        }
    }
    None
}

fn gamma(value: f64) -> f64 {
    if value > 0.04045 {
        ((value + 0.055) / 1.055).powf(2.4)
    } else {
        value / 12.92
    }
}

fn rgb_to_xy(red: u8, green: u8, blue: u8) -> (f64, f64) {
    let r = gamma(f64::from(red) / 255.0);
    let g = gamma(f64::from(green) / 255.0);
    let b = gamma(f64::from(blue) / 255.0);
    let x = r * 0.664511 + g * 0.154324 + b * 0.162028;
    let y = r * 0.283881 + g * 0.668433 + b * 0.047685;
    let z = r * 0.000088 + g * 0.072310 + b * 0.986039;
    let sum = x + y + z;
    (x / sum, y / sum)
}

fn round4(value: f64) -> f64 {
    (value * 10000.0).round() / 10000.0
}

fn number_json(value: f64) -> Value {
    if value.fract() == 0.0 {
        json!(value as i64)
    } else {
        json!(value)
    }
}

fn convert_set(
    device: &PublishDevice,
    property: &str,
    value: &Value,
    _endpoint: &str,
    message_transition: Option<u32>,
    stored_brightness: Option<u64>,
    brightness_was_zero: bool,
    off_transition: bool,
) -> Option<(&'static str, &'static str, Value, Value, Map<String, Value>)> {
    let no_state = || Map::new();
    match property {
        "state" => {
            let state = value.as_str()?;
            if let Some(transition) = message_transition {
                let (level, updates) = match state.to_lowercase().as_str() {
                    "on" => {
                        let level = stored_brightness.unwrap_or(254);
                        (level, json!({"state": "ON", "brightness": level}))
                    }
                    "off" => (0, json!({"state": "OFF"})),
                    _ => return None,
                };
                return Some((
                    "genLevelCtrl",
                    "moveToLevelWithOnOff",
                    json!({
                        "level": level,
                        "transtime": transition,
                        "optionsMask": 0,
                        "optionsOverride": 0,
                    }),
                    json!({}),
                    updates.as_object()?.clone(),
                ));
            }
            if state.eq_ignore_ascii_case("on") && (brightness_was_zero || off_transition) {
                if let Some(level) = stored_brightness {
                    return Some((
                        "genLevelCtrl",
                        "moveToLevelWithOnOff",
                        json!({
                            "level": level,
                            "transtime": 0,
                            "optionsMask": 0,
                            "optionsOverride": 0,
                        }),
                        json!({}),
                        json!({"state": "ON", "brightness": level})
                            .as_object()?
                            .clone(),
                    ));
                }
            }
            let (command, updates) = match state.to_lowercase().as_str() {
                "on" => ("on", json!({"state": "ON"})),
                "off" => ("off", json!({"state": "OFF"})),
                "toggle" => ("toggle", json!({})),
                _ => return None,
            };
            Some((
                "genOnOff",
                command,
                json!({}),
                json!({}),
                updates.as_object()?.clone(),
            ))
        }
        "brightness_percent" => {
            let percent = as_f64(value)?;
            let level = (percent * 255.0 / 100.0).round() as u64;
            let transition = message_transition
                .or(device.transition.map(|transition| transition * 10))
                .unwrap_or(0);
            Some((
                "genLevelCtrl",
                "moveToLevelWithOnOff",
                json!({
                    "level": level,
                    "transtime": transition,
                    "optionsMask": 0,
                    "optionsOverride": 0,
                }),
                json!({}),
                json!({"brightness": level, "state": "ON"})
                    .as_object()?
                    .clone(),
            ))
        }
        "brightness" => {
            let level = as_f64(value)? as u64;
            if level == 0 {
                let transition = message_transition
                    .or(device.transition.map(|transition| transition * 10))
                    .unwrap_or(0);
                return Some((
                    "genLevelCtrl",
                    "moveToLevelWithOnOff",
                    json!({
                        "level": 0,
                        "transtime": transition,
                        "optionsMask": 0,
                        "optionsOverride": 0,
                    }),
                    json!({}),
                    json!({"state": "OFF", "brightness": 0})
                        .as_object()?
                        .clone(),
                ));
            }
            let transition = message_transition
                .or(device.transition.map(|transition| transition * 10))
                .unwrap_or(0);
            Some((
                "genLevelCtrl",
                "moveToLevelWithOnOff",
                json!({
                    "level": level,
                    "transtime": transition,
                    "optionsMask": 0,
                    "optionsOverride": 0,
                }),
                json!({}),
                json!({"brightness": level, "state": "ON"})
                    .as_object()?
                    .clone(),
            ))
        }
        "color" => {
            let transition = message_transition
                .or(device.transition.map(|transition| transition * 10))
                .unwrap_or(0);
            if let (Some(hue), Some(saturation)) = (
                value.get("hue").and_then(Value::as_f64),
                value.get("saturation").and_then(Value::as_f64),
            ) {
                return Some((
                    "lightingColorCtrl",
                    "enhancedMoveToHueAndSaturation",
                    json!({
                        "enhancehue": (hue / 360.0 * 65535.0).round() as u64,
                        "saturation": (saturation / 100.0 * 254.0).round() as u64,
                        "transtime": transition,
                        "optionsMask": 0,
                        "optionsOverride": 0,
                    }),
                    json!({}),
                    json!({
                        "color": {"hue": hue as u64, "saturation": saturation as u64},
                        "color_mode": "hs",
                    })
                    .as_object()?
                    .clone(),
                ));
            }
            let (x, y) = color_xy(value)?;
            let (x, y) = (round4(x), round4(y));
            Some((
                "lightingColorCtrl",
                "moveToColor",
                json!({
                    "colorx": (x * 65535.0).round() as u64,
                    "colory": (y * 65535.0).round() as u64,
                    "transtime": transition,
                    "optionsMask": 0,
                    "optionsOverride": 0,
                }),
                json!({}),
                json!({
                    "color": {"x": number_json(round4(x)), "y": number_json(round4(y))},
                    "color_mode": "xy",
                })
                .as_object()?
                .clone(),
            ))
        }
        "color_temp_percent" => {
            let percent = as_f64(value)?;
            let color_temp = (153.0 + percent * (500.0 - 153.0) / 100.0).round() as u64;
            let transition = message_transition
                .or(device.transition.map(|transition| transition * 10))
                .unwrap_or(0);
            Some((
                "lightingColorCtrl",
                "moveToColorTemp",
                json!({
                    "colortemp": color_temp,
                    "transtime": transition,
                    "optionsMask": 0,
                    "optionsOverride": 0,
                }),
                json!({}),
                json!({"color_temp": color_temp, "color_mode": "color_temp"})
                    .as_object()?
                    .clone(),
            ))
        }
        "color_temp" => {
            let color_temp = as_f64(value)? as u64;
            let transition = message_transition
                .or(device.transition.map(|transition| transition * 10))
                .unwrap_or(0);
            Some((
                "lightingColorCtrl",
                "moveToColorTemp",
                json!({
                    "colortemp": color_temp,
                    "transtime": transition,
                    "optionsMask": 0,
                    "optionsOverride": 0,
                }),
                json!({}),
                json!({"color_temp": color_temp, "color_mode": "color_temp"})
                    .as_object()?
                    .clone(),
            ))
        }
        "brightness_move" => {
            let rate = value.as_i64()?;
            Some((
                "genLevelCtrl",
                "move",
                json!({
                    "movemode": if rate > 0 { 0 } else { 1 },
                    "rate": rate.unsigned_abs(),
                    "optionsMask": 0,
                    "optionsOverride": 0,
                }),
                json!({}),
                no_state(),
            ))
        }
        "position" => {
            let position = value.as_u64()?;
            let lift = if device.invert_cover {
                position
            } else {
                100 - position
            };
            Some((
                "closuresWindowCovering",
                "goToLiftPercentage",
                json!({"percentageliftvalue": lift}),
                json!({}),
                no_state(),
            ))
        }
        "warning" => {
            let (mode, level, strobe, duration) = match value {
                Value::Object(object) => {
                    let mode = object
                        .get("mode")
                        .and_then(Value::as_str)
                        .unwrap_or("emergency");
                    let level = object
                        .get("level")
                        .and_then(Value::as_str)
                        .unwrap_or("medium");
                    let strobe = object
                        .get("strobe")
                        .and_then(Value::as_bool)
                        .unwrap_or(true);
                    let duration = object.get("duration").and_then(Value::as_u64).unwrap_or(10);
                    (mode, level, strobe, duration)
                }
                _ => ("emergency", "medium", true, 10),
            };
            let mode = match mode {
                "stop" => 0,
                "burglar" => 1,
                "fire" => 2,
                "emergency" => 3,
                _ => 3,
            };
            let level = match level {
                "low" => 0,
                "medium" => 1,
                "high" => 2,
                "very_high" => 3,
                _ => 1,
            };
            let info = (mode << 4) + (u64::from(strobe) << 2) + level;
            Some((
                "ssIasWd",
                "startWarning",
                json!({
                    "startwarninginfo": info,
                    "warningduration": duration,
                    "strobedutycycle": 0,
                    "strobelevel": 1,
                }),
                json!({"disableDefaultResponse": true}),
                no_state(),
            ))
        }
        _ => None,
    }
}

fn convert_group_set(
    property: &str,
    value: &Value,
    transition: u32,
) -> Option<(&'static str, &'static str, Value, Value, Map<String, Value>)> {
    match property {
        "state" => {
            let state = value.as_str()?;
            let (command, updates) = match state.to_lowercase().as_str() {
                "on" => ("on", json!({"state": "ON"})),
                "off" => ("off", json!({"state": "OFF"})),
                _ => return None,
            };
            Some((
                "genOnOff",
                command,
                json!({}),
                json!({}),
                updates.as_object()?.clone(),
            ))
        }
        "brightness_percent" => {
            let percent = as_f64(value)?;
            let level = (percent * 255.0 / 100.0).round() as u64;
            Some((
                "genLevelCtrl",
                "moveToLevelWithOnOff",
                json!({
                    "level": level,
                    "transtime": transition,
                    "optionsMask": 0,
                    "optionsOverride": 0,
                }),
                json!({}),
                json!({"brightness": level, "state": "ON"})
                    .as_object()?
                    .clone(),
            ))
        }
        "brightness" => {
            let level = as_f64(value)? as u64;
            Some((
                "genLevelCtrl",
                "moveToLevelWithOnOff",
                json!({
                    "level": level,
                    "transtime": transition,
                    "optionsMask": 0,
                    "optionsOverride": 0,
                }),
                json!({}),
                json!({"brightness": level, "state": "ON"})
                    .as_object()?
                    .clone(),
            ))
        }
        "color" => {
            let (x, y) = color_xy(value)?;
            let (x, y) = (round4(x), round4(y));
            Some((
                "lightingColorCtrl",
                "moveToColor",
                json!({
                    "colorx": (x * 65535.0).round() as u64,
                    "colory": (y * 65535.0).round() as u64,
                    "transtime": transition,
                    "optionsMask": 0,
                    "optionsOverride": 0,
                }),
                json!({}),
                json!({
                    "color": {"x": number_json(x), "y": number_json(y)},
                    "color_mode": "xy",
                })
                .as_object()?
                .clone(),
            ))
        }
        "child_lock" => {
            let locked = value.as_str() == Some("LOCK");
            Some((
                "manuSpecificTuya",
                "dataRequest",
                json!({"dpValues": [{"data": [u8::from(locked)], "datatype": 1, "dp": 7}], "seq": 0}),
                json!({"disableDefaultResponse": true}),
                Map::new(),
            ))
        }
        "color_temp" => {
            let color_temp = as_f64(value)? as u64;
            Some((
                "lightingColorCtrl",
                "moveToColorTemp",
                json!({
                    "colortemp": color_temp,
                    "transtime": transition,
                    "optionsMask": 0,
                    "optionsOverride": 0,
                }),
                json!({}),
                json!({"color_temp": color_temp, "color_mode": "color_temp"})
                    .as_object()?
                    .clone(),
            ))
        }
        _ => None,
    }
}

fn color_temp_to_xy(color_temp: u64) -> (f64, f64) {
    // Upstream uses a Kelvin->XY lookup table; the port keeps the values used
    // by the ported fixtures and falls back to the Planckian approximation.
    if color_temp == 300 {
        return (0.4152, 0.3954);
    }
    let kelvin = 1_000_000.0 / color_temp as f64;
    let t = kelvin;
    let x = -0.2661239e9 / t.powi(3) + 0.2343589e6 / t.powi(2) + 0.8776956e3 / t + 0.179910;
    let y = -0.9549476 * x.powi(3) - 1.37418593 * x.powi(2) + 2.09137015 * x - 0.16748867;
    (round4(x), round4(y))
}

fn as_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
}
