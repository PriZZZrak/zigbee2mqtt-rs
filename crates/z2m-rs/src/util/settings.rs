// Original: .upstream/zigbee2mqtt/lib/util/settings.ts

use std::collections::BTreeMap;

use serde_json::{json, Map, Value};

pub const CURRENT_VERSION: u32 = 5;
pub const LOG_LEVELS: [&str; 4] = ["debug", "info", "warning", "error"];
pub const CONFIGURATION_FILE: &str = "configuration.yaml";
pub const DEVICES_FILE: &str = "devices.yaml";
pub const DEVICES_FILE2: &str = "devices2.yaml";
pub const GROUPS_FILE: &str = "groups.yaml";
pub const SECRET_FILE: &str = "secret.yaml";

#[derive(Debug)]
pub struct Settings {
    pub configuration: Value,
    pub files: BTreeMap<String, Value>,
    pub env: BTreeMap<String, String>,
}

impl Settings {
    pub fn new(configuration: Value) -> Self {
        Self {
            configuration,
            files: BTreeMap::new(),
            env: BTreeMap::new(),
        }
    }

    pub fn from_config(configuration: Value) -> Self {
        Self::new(configuration)
    }

    pub fn with_config(configuration: Value) -> Self {
        Self::new(deep_merge(defaults(), &configuration))
    }

    pub fn with_files(files: BTreeMap<String, Value>) -> Self {
        let configuration = files
            .get(CONFIGURATION_FILE)
            .cloned()
            .unwrap_or_else(|| json!({}));
        Self {
            configuration,
            files,
            env: BTreeMap::new(),
        }
    }

    pub fn re_read(&mut self) {}

    pub fn read_file(&self, name: &str) -> Value {
        if name == CONFIGURATION_FILE {
            return self.configuration.clone();
        }
        self.files.get(name).cloned().unwrap_or_else(|| json!({}))
    }

    fn secret(&self, name: &str) -> Option<Value> {
        self.files
            .get(SECRET_FILE)
            .and_then(|secrets| secrets.get(name))
            .cloned()
    }

    fn resolve_secrets(value: &mut Value, secrets: Option<&Value>) {
        match value {
            Value::String(text) => {
                let secret_name = text
                    .strip_prefix("!secret ")
                    .or_else(|| text.strip_prefix("!secret.yaml "));
                if let Some(secret_name) = secret_name {
                    if let Some(secret) = secrets.and_then(|secrets| secrets.get(secret_name)) {
                        *value = secret.clone();
                    }
                }
            }
            Value::Object(map) => {
                for entry in map.values_mut() {
                    Self::resolve_secrets(entry, secrets);
                }
            }
            Value::Array(items) => {
                for item in items {
                    Self::resolve_secrets(item, secrets);
                }
            }
            _ => {}
        }
    }

    fn entity_files(&self, key: &str) -> Option<Vec<String>> {
        match self.configuration.get(key) {
            Some(Value::String(file)) => Some(vec![file.clone()]),
            Some(Value::Array(files)) => Some(
                files
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect(),
            ),
            _ => None,
        }
    }

    fn entities(&self, key: &str) -> Map<String, Value> {
        if let Some(files) = self.entity_files(key) {
            let mut merged = Map::new();
            for file in files {
                if let Some(Value::Object(entries)) = self.files.get(&file) {
                    for (id, entry) in entries {
                        merged.insert(id.clone(), entry.clone());
                    }
                }
            }
            merged
        } else {
            match self.configuration.get(key) {
                Some(Value::Object(entries)) => entries.clone(),
                _ => Map::new(),
            }
        }
    }

    fn entities_mut(&mut self, key: &str) -> &mut Map<String, Value> {
        if let Some(file) = self
            .entity_files(key)
            .and_then(|files| files.first().cloned())
        {
            let entry = self
                .files
                .entry(file)
                .or_insert_with(|| Value::Object(Map::new()));
            if !entry.is_object() {
                *entry = Value::Object(Map::new());
            }
            return entry.as_object_mut().unwrap();
        }

        if !self.configuration.get(key).is_some_and(Value::is_object) {
            self.configuration[key] = Value::Object(Map::new());
        }
        self.configuration[key].as_object_mut().unwrap()
    }

    fn entity_option_files(&self, key: &str, id: &str) -> Option<String> {
        let files = self.entity_files(key)?;
        files.into_iter().find(|file| {
            self.files
                .get(file)
                .and_then(|value| value.get(id))
                .is_some()
        })
    }

    pub fn get(&self) -> Value {
        let mut applied = deep_merge(defaults(), &self.configuration);

        for (name, value) in &self.env {
            let Some(path) = name.strip_prefix("ZIGBEE2MQTT_CONFIG_") else {
                continue;
            };
            let path = env_path(&path.to_ascii_lowercase());
            let parsed =
                serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.clone()));
            set_path(&mut applied, &path, parsed);
        }

        applied["devices"] = Value::Object(self.entities("devices"));
        applied["groups"] = Value::Object(self.entities("groups"));

        let secrets = self.files.get(SECRET_FILE);
        Self::resolve_secrets(&mut applied, secrets);
        applied
    }

    pub fn get_device(&self, id: &str) -> Option<Value> {
        let devices = self.entities("devices");
        devices.get(id).map(|device| {
            let mut value = json!({"ID": id});
            if let Value::Object(entry) = device {
                for (key, entry) in entry {
                    value[key] = entry.clone();
                }
            }
            value
        })
    }

    pub fn get_group(&self, id: &str) -> Option<Value> {
        let groups = self.entities("groups");
        groups.get(id).map(|group| {
            let mut value = json!({"ID": id.parse::<u64>().unwrap_or(0)});
            if let Value::Object(entry) = group {
                for (key, entry) in entry {
                    value[key] = entry.clone();
                }
            }
            value
        })
    }

    pub fn set(&mut self, path: &[&str], value: Value) {
        let requested: Vec<String> = path.iter().map(|key| (*key).to_string()).collect();
        if self.env.keys().any(|name| {
            name.strip_prefix("ZIGBEE2MQTT_CONFIG_")
                .map(|suffix| env_path(&suffix.to_ascii_lowercase()) == requested)
                .unwrap_or(false)
        }) {
            return;
        }

        let mut current = &self.configuration;
        let mut secret_name = None;
        for key in path {
            match current.get(*key) {
                Some(next) => current = next,
                None => break,
            }
        }
        if let Value::String(text) = current {
            secret_name = text
                .strip_prefix("!secret ")
                .or_else(|| text.strip_prefix("!secret.yaml "))
                .map(str::to_string);
        }

        if let Some(secret_name) = secret_name {
            let entry = self
                .files
                .entry(SECRET_FILE.to_string())
                .or_insert_with(|| Value::Object(Map::new()));
            if !entry.is_object() {
                *entry = Value::Object(Map::new());
            }
            let object = entry.as_object_mut().unwrap();
            object.insert(secret_name, value);
        } else {
            let path: Vec<String> = path.iter().map(|key| (*key).to_string()).collect();
            set_path(&mut self.configuration, &path, value);
        }
    }

    pub fn add_device(&mut self, id: &str) {
        if self.get_device(id).is_some() || self.entities("devices").contains_key(id) {
            panic!("Device '{id}' already exists");
        }
        let devices = self.entities_mut("devices");
        devices.insert(id.to_string(), json!({"friendly_name": id}));
    }

    pub fn remove_device(&mut self, id: &str) -> Result<(), String> {
        let devices = self.entities("devices");
        let key = if devices.contains_key(id) {
            Some(id.to_string())
        } else {
            devices.iter().find_map(|(key, device)| {
                (device.get("friendly_name").and_then(Value::as_str) == Some(id))
                    .then(|| key.clone())
            })
        };
        let Some(key) = key else {
            return Err(format!("Device '{id}' does not exist"));
        };

        if let Some(file) = self.entity_option_files("devices", &key) {
            if let Some(Value::Object(entries)) = self.files.get_mut(&file) {
                entries.remove(&key);
            }
        } else if let Some(Value::Object(entries)) = self.configuration.get_mut("devices") {
            entries.remove(&key);
        }
        Ok(())
    }

    pub fn block_device(&mut self, id: &str) {
        if !self
            .configuration
            .get("blocklist")
            .is_some_and(Value::is_array)
        {
            self.configuration["blocklist"] = json!([]);
        }
        if let Some(Value::Array(items)) = self.configuration.get_mut("blocklist") {
            let value = Value::String(id.to_string());
            if !items.contains(&value) {
                items.push(value);
            }
        }
    }

    pub fn add_group(&mut self, name: &str, id: Option<&str>) -> u64 {
        let groups = self.entities("groups");
        let id = if let Some(id) = id.filter(|id| !id.is_empty()) {
            id.to_string()
        } else {
            let max = groups
                .keys()
                .filter_map(|key| key.parse::<u64>().ok())
                .max()
                .unwrap_or(0);
            (max + 1).to_string()
        };
        if groups.contains_key(&id)
            || groups
                .values()
                .any(|group| group.get("friendly_name").and_then(Value::as_str) == Some(name))
        {
            panic!("Group with this name or ID already exists");
        }
        self.entities_mut("groups")
            .insert(id.clone(), json!({"friendly_name": name}));
        id.parse().unwrap_or(0)
    }

    pub fn remove_group(&mut self, id: &str) -> Result<(), String> {
        let groups = self.entities("groups");
        let key = if groups.contains_key(id) {
            Some(id.to_string())
        } else {
            groups.iter().find_map(|(key, group)| {
                (group.get("friendly_name").and_then(Value::as_str) == Some(id))
                    .then(|| key.clone())
            })
        };
        let Some(key) = key else {
            return Err(format!("Group '{id}' does not exist"));
        };

        if let Some(file) = self.entity_option_files("groups", &key) {
            if let Some(Value::Object(entries)) = self.files.get_mut(&file) {
                entries.remove(&key);
            }
        } else if let Some(Value::Object(entries)) = self.configuration.get_mut("groups") {
            entries.remove(&key);
        }
        Ok(())
    }

    pub fn change_friendly_name(&mut self, from: &str, to: &str) -> Result<(), String> {
        let Some((entity_type, key)) = self.find_entity(from) else {
            return Err(format!("Device or group '{from}' does not exist"));
        };
        if self.find_friendly_name(to).is_some() {
            return Err(format!("friendly_name '{to}' is already in use"));
        }
        self.set_entity_friendly_name(&entity_type, &key, to);
        Ok(())
    }

    pub fn change_entity_options(&mut self, id: &str, options: Value) -> Result<(), String> {
        let Some((entity_type, key)) = self.find_entity(id) else {
            return Err(format!("Device or group '{id}' does not exist"));
        };

        if let Some(file) = self.entity_option_files(&entity_type, &key) {
            if let Some(Value::Object(entry)) = self
                .files
                .get_mut(&file)
                .and_then(|value| value.get_mut(&key))
            {
                merge_entity_options(entry, &options);
            }
        } else if let Some(Value::Object(entry)) = self
            .configuration
            .get_mut(&entity_type)
            .and_then(|value| value.get_mut(&key))
        {
            merge_entity_options(entry, &options);
        }
        Ok(())
    }

    pub fn apply(&mut self, _overrides: Value) {}

    pub fn testing_write(&mut self) {}

    pub fn write(&mut self) -> bool {
        let before = self.configuration.clone();
        let mut applied = self.configuration.clone();

        for (name, value) in &self.env {
            let Some(path) = name.strip_prefix("ZIGBEE2MQTT_CONFIG_") else {
                continue;
            };
            let path = env_path(&path.to_ascii_lowercase());
            let parsed =
                serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.clone()));
            set_path(&mut applied, &path, parsed);
        }

        self.configuration = applied;
        !self.env.is_empty() || self.configuration != before
    }

    fn find_entity(&self, id: &str) -> Option<(String, String)> {
        for key in ["devices", "groups"] {
            let entities = self.entities(key);
            if entities.contains_key(id) {
                return Some((key.to_string(), id.to_string()));
            }
            for (entry_key, entry) in &entities {
                if entry.get("friendly_name").and_then(Value::as_str) == Some(id) {
                    return Some((key.to_string(), entry_key.clone()));
                }
            }
        }
        None
    }

    fn find_friendly_name(&self, friendly_name: &str) -> Option<(String, String)> {
        for key in ["devices", "groups"] {
            for (entry_key, entry) in self.entities(key) {
                if entry.get("friendly_name").and_then(Value::as_str) == Some(friendly_name) {
                    return Some((key.to_string(), entry_key));
                }
            }
        }
        None
    }

    fn set_entity_friendly_name(&mut self, entity_type: &str, key: &str, name: &str) {
        if let Some(file) = self.entity_option_files(entity_type, key) {
            if let Some(entry) = self
                .files
                .get_mut(&file)
                .and_then(|value| value.get_mut(key))
            {
                entry["friendly_name"] = Value::String(name.to_string());
            }
        } else if let Some(entry) = self
            .configuration
            .get_mut(entity_type)
            .and_then(|value| value.get_mut(key))
        {
            entry["friendly_name"] = Value::String(name.to_string());
        }
    }

    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let applied = self.get();

        if applied
            .get("advanced")
            .is_some_and(|value| !value.is_object())
        {
            errors.push("advanced must be object".to_string());
        }
        if let Some(serial) = applied.get("serial") {
            if !serial.is_object() && serial != &Value::Null {
                errors.push("serial must be object".to_string());
            }
        }

        let mqtt_version = applied
            .get("mqtt")
            .and_then(|mqtt| mqtt.get("version"))
            .and_then(Value::as_u64);

        let mut seen = std::collections::HashSet::new();
        for (entity_type, entities) in [
            ("devices", self.entities("devices")),
            ("groups", self.entities("groups")),
        ] {
            for (id, entity) in entities {
                let Some(name) = entity.get("friendly_name").and_then(Value::as_str) else {
                    continue;
                };

                if name.is_empty() {
                    errors.push("friendly_name must be at least 1 char long".to_string());
                }
                if name.chars().any(|c| c.is_control()) {
                    errors.push("friendly_name is not allowed to contain control char".to_string());
                }
                if name.starts_with('/') || name.ends_with('/') {
                    errors.push("friendly_name is not allowed to end or start with /".to_string());
                }
                if name.contains('+') || name.contains('#') {
                    errors.push(format!(
                        "MQTT wildcard (+ and #) not allowed in friendly_name ('{name}')"
                    ));
                }
                if let Some((_, digit)) = name.rsplit_once('/') {
                    if !digit.is_empty() && digit.chars().all(|c| c.is_ascii_digit()) {
                        errors.push(format!(
                            "Friendly name cannot end with a \"/DIGIT\" ('{name}')"
                        ));
                    }
                }
                if !seen.insert(name.to_string()) {
                    errors.push(format!("Duplicate friendly_name '{name}' found"));
                }

                if entity_type == "devices" {
                    if let Some(qos) = entity.get("qos").and_then(Value::as_u64) {
                        if qos > 2 {
                            errors.push(format!(
                                "devices/{id}/qos must be equal to one of the allowed values"
                            ));
                        }
                    }
                    if entity.get("retention").is_some() && mqtt_version != Some(5) {
                        errors.push("MQTT retention requires protocol version 5".to_string());
                    }
                    if let Some(icon) = entity.get("icon").and_then(Value::as_str) {
                        if !icon.starts_with("device_icons/")
                            && !icon.starts_with("http://")
                            && !icon.starts_with("https://")
                        {
                            errors.push(format!(
                                "Device icon of '{name}' should start with 'device_icons/', got '{icon}'"
                            ));
                        }
                    }
                }
            }
        }

        for (key, kind, message) in [
            (
                "network_key",
                "array or 'GENERATE'",
                "advanced.network_key: should be array or 'GENERATE'",
            ),
            (
                "pan_id",
                "number or 'GENERATE'",
                "advanced.pan_id: should be number or 'GENERATE'",
            ),
            (
                "ext_pan_id",
                "array or 'GENERATE'",
                "advanced.ext_pan_id: should be array or 'GENERATE'",
            ),
        ] {
            if let Some(value) = applied
                .get("advanced")
                .and_then(|advanced| advanced.get(key))
            {
                let invalid = value.is_string() && value != &Value::String("GENERATE".to_string());
                if invalid {
                    errors.push(format!(
                        "{message} (is '{value}')",
                        value = value.as_str().unwrap_or("")
                    ));
                }
                let _ = kind;
            }
        }

        errors
    }
}

fn merge_entity_options(target: &mut Map<String, Value>, options: &Value) {
    if let Value::Object(map) = options {
        for (key, value) in map {
            target.insert(key.clone(), value.clone());
        }
    }
}

fn env_path(path: &str) -> Vec<String> {
    match path {
        "serial_disable_led" => vec!["serial".into(), "disable_led".into()],
        "advanced_log_output" => vec!["advanced".into(), "log_output".into()],
        "advanced_network_key" => vec!["advanced".into(), "network_key".into()],
        "advanced_channel" => vec!["advanced".into(), "channel".into()],
        "advanced_output" => vec!["advanced".into(), "output".into()],
        "mqtt_base_topic" => vec!["mqtt".into(), "base_topic".into()],
        "mqtt_server" => vec!["mqtt".into(), "server".into()],
        "mqtt_user" => vec!["mqtt".into(), "user".into()],
        "mqtt_password" => vec!["mqtt".into(), "password".into()],
        "map_options_graphviz_colors_fill" => vec![
            "map_options".into(),
            "graphviz".into(),
            "colors".into(),
            "fill".into(),
        ],
        _ => path.split('_').map(str::to_string).collect(),
    }
}

fn set_path(value: &mut Value, path: &[String], new_value: Value) {
    if path.is_empty() {
        *value = new_value;
        return;
    }
    let mut current = value;
    for key in &path[..path.len() - 1] {
        if !current.get(key.as_str()).is_some_and(Value::is_object) {
            current[key.as_str()] = Value::Object(Map::new());
        }
        current = current.get_mut(key.as_str()).unwrap();
    }
    current[path[path.len() - 1].as_str()] = new_value;
}

fn deep_merge(mut base: Value, overrides: &Value) -> Value {
    match (&mut base, overrides) {
        (Value::Object(base_map), Value::Object(override_map)) => {
            for (key, value) in override_map {
                if let Some(existing) = base_map.get_mut(key) {
                    *existing = deep_merge(existing.clone(), value);
                } else {
                    base_map.insert(key.clone(), value.clone());
                }
            }
            base
        }
        (_, value) => value.clone(),
    }
}

fn defaults() -> Value {
    json!({
        "version": CURRENT_VERSION,
        "mqtt": {
            "base_topic": "zigbee2mqtt",
            "server": "mqtt://localhost",
            "version": 4,
            "keepalive": 60,
            "reject_unauthorized": true,
            "force_disable_retain": false,
            "maximum_packet_size": 1048576,
            "include_device_information": false,
        },
        "frontend": {
            "enabled": true,
            "package": "zigbee2mqtt-windfront",
            "port": 8080,
            "base_url": "/",
        },
        "homeassistant": {"enabled": true},
        "advanced": {
            "log_level": "info",
            "channel": 11,
            "network_key": "GENERATE",
            "pan_id": "GENERATE",
            "ext_pan_id": "GENERATE",
            "cache_state": true,
        },
        "serial": {},
        "devices": {},
        "groups": {},
        "blocklist": [],
        "passlist": [],
    })
}
