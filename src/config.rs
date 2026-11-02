use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;

use crate::error::{Error, Result};

fn deserialize_device_map<'de, D>(
    deserializer: D,
) -> std::result::Result<HashMap<String, DeviceConfig>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = HashMap<String, DeviceConfig>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "a map or string")
        }
        fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut hm = HashMap::new();
            while let Some((k, v)) = map.next_entry::<String, DeviceConfig>()? {
                hm.insert(k, v);
            }
            Ok(hm)
        }
        fn visit_str<E>(self, _: &str) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(HashMap::new())
        }
        fn visit_string<E>(self, _: String) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(HashMap::new())
        }
        fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            while let Some::<String>(_) = seq.next_element()? {}
            Ok(HashMap::new())
        }
    }
    deserializer.deserialize_any(Visitor)
}

fn deserialize_group_map<'de, D>(
    deserializer: D,
) -> std::result::Result<HashMap<String, GroupConfig>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = HashMap<String, GroupConfig>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "a map or string")
        }
        fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut hm = HashMap::new();
            while let Some((k, v)) = map.next_entry::<String, GroupConfig>()? {
                hm.insert(k, v);
            }
            Ok(hm)
        }
        fn visit_str<E>(self, _: &str) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(HashMap::new())
        }
        fn visit_string<E>(self, _: String) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(HashMap::new())
        }
        fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            while let Some::<String>(_) = seq.next_element()? {}
            Ok(HashMap::new())
        }
    }
    deserializer.deserialize_any(Visitor)
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub serial: SerialConfig,
    pub mqtt: MqttConfig,
    pub permit_join: bool,
    pub homeassistant: HomeAssistantConfig,
    #[serde(deserialize_with = "deserialize_device_map", default)]
    pub devices: HashMap<String, DeviceConfig>,
    #[serde(deserialize_with = "deserialize_group_map", default)]
    pub groups: HashMap<String, GroupConfig>,
    #[serde(default)]
    pub blocklist: Vec<String>,
    #[serde(default)]
    pub passlist: Vec<String>,
    pub advanced: AdvancedConfig,
}

/// Home Assistant discovery toggle. zigbee2mqtt allows either a boolean
/// (`homeassistant: true`) or a mapping (`homeassistant: { enabled: true, ... }`),
/// so accept both forms.
#[derive(Debug, Clone, Default)]
pub struct HomeAssistantConfig {
    pub enabled: bool,
}

impl HomeAssistantConfig {
    pub fn enabled(&self) -> bool {
        self.enabled
    }
}

impl<'de> serde::Deserialize<'de> for HomeAssistantConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        struct BoolOrMap;
        impl<'de> Visitor<'de> for BoolOrMap {
            type Value = HomeAssistantConfig;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a boolean or a mapping")
            }

            fn visit_bool<E>(self, v: bool) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(HomeAssistantConfig { enabled: v })
            }

            fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut enabled = true;
                while let Some(key) = map.next_key::<String>()? {
                    if key == "enabled" {
                        enabled = map.next_value()?;
                    } else {
                        let _: serde::de::IgnoredAny = map.next_value()?;
                    }
                }
                Ok(HomeAssistantConfig { enabled })
            }
        }

        deserializer.deserialize_any(BoolOrMap)
    }
}

impl serde::Serialize for HomeAssistantConfig {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bool(self.enabled)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct SerialConfig {
    pub port: String,
    pub baudrate: u32,
    pub adapter: AdapterType,
    pub rtscts: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AdapterType {
    Znp,
    Zstack,
    Ezsp,
    Auto,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct MqttConfig {
    pub server: String,
    pub port: u16,
    pub base_topic: String,
    pub client_id: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub keepalive: u16,
    pub reject_unauthorized: bool,
}

impl std::fmt::Debug for MqttConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MqttConfig")
            .field("server", &self.server)
            .field("port", &self.port)
            .field("base_topic", &self.base_topic)
            .field("client_id", &self.client_id)
            .field("username", &self.username)
            .field("password", &self.password.as_ref().map(|_| "<redacted>"))
            .field("keepalive", &self.keepalive)
            .field("reject_unauthorized", &self.reject_unauthorized)
            .finish()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DeviceConfig {
    pub friendly_name: Option<String>,
    pub retain: Option<bool>,
    pub qos: Option<u8>,
    pub disabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct GroupConfig {
    pub friendly_name: Option<String>,
}

/// How device state is published to MQTT (z2m: `advanced.output`).
///
/// - `Json`: single merged JSON message to `<base_topic>/<friendly_name>`
///   (this project's only behavior prior to this option, and still real
///   zigbee2mqtt's own default). Required for the Home Assistant discovery
///   configs in `homeassistant.rs`, which reference the JSON
///   state_topic/value_template.
/// - `Attribute`: each state key published as its own raw-value subtopic,
///   e.g. `<base_topic>/<friendly_name>/action` with payload `toggle`
///   (unquoted). No merged JSON message is published in this mode -- this
///   alone would break Home Assistant discovery.
/// - `AttributeAndJson`: both of the above.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    #[default]
    Json,
    Attribute,
    AttributeAndJson,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct AdvancedConfig {
    pub pan_id: u16,
    pub ext_pan_id: [u8; 8],
    pub channel: u8,
    pub network_key: [u8; 16],
    pub log_level: String,
    pub cache_state: bool,
    pub output: OutputMode,
}

impl std::fmt::Debug for AdvancedConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdvancedConfig")
            .field("pan_id", &self.pan_id)
            .field("ext_pan_id", &self.ext_pan_id)
            .field("channel", &self.channel)
            .field("network_key", &"<redacted>")
            .field("log_level", &self.log_level)
            .field("cache_state", &self.cache_state)
            .field("output", &self.output)
            .finish()
    }
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            port: "/dev/ttyACM0".to_string(),
            baudrate: 115_200,
            adapter: AdapterType::Auto,
            rtscts: false,
        }
    }
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            server: "localhost".to_string(),
            port: 1883,
            base_topic: "zigbee2mqtt".to_string(),
            client_id: "zigbee2mqtt-rs".to_string(),
            username: None,
            password: None,
            keepalive: 60,
            reject_unauthorized: true,
        }
    }
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            pan_id: 0x1a62,
            ext_pan_id: [0xDD, 0xDD, 0xDD, 0xDD, 0xDD, 0xDD, 0xDD, 0xDD],
            channel: 11,
            network_key: [1, 3, 5, 7, 9, 11, 13, 15, 0, 2, 4, 6, 8, 10, 12, 13],
            log_level: "info".to_string(),
            cache_state: true,
            output: OutputMode::default(),
        }
    }
}

fn parse_value_ref(text: &str) -> Option<(String, String)> {
    let t = text.trim();
    if !t.starts_with('!') {
        return None;
    }
    // "!secret key" or "!env VAR"
    let rest = t[1..].trim();
    let mut parts = rest.splitn(2, |c: char| c.is_whitespace());
    let filename = parts.next()?.trim();
    let key = parts.next()?.trim();
    if filename.is_empty() || key.is_empty() {
        return None;
    }
    let mut fname = filename.to_string();
    if !fname.ends_with(".yaml") && !fname.ends_with(".yml") {
        fname.push_str(".yaml");
    }
    Some((fname, key.to_string()))
}

fn resolve_secret(value: &mut String, config_dir: &Path) {
    if let Some((fname, key)) = parse_value_ref(value) {
        let secret_path = config_dir.join(&fname);
        if let Ok(content) = std::fs::read_to_string(&secret_path) {
            if let Ok(v) = serde_yml::from_str::<serde_json::Value>(&content) {
                if let Some(val) = v.get(&key).and_then(|x| x.as_str()) {
                    *value = val.to_string();
                }
            }
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("cannot read {}: {e}", path.display())))?;
        let mut config: Config = serde_yml::from_str(&content)?;
        // Handle devices/groups separate files as in settings.ts readDevicesOrGroups
        let raw: serde_json::Value =
            serde_yml::from_str(&content).unwrap_or(serde_json::Value::Null);
        let config_dir = path.parent().unwrap_or(Path::new("."));
        let load_map = |key: &str| -> HashMap<String, serde_json::Value> {
            let mut map = HashMap::new();
            if let Some(v) = raw.get(key) {
                if let Some(s) = v.as_str() {
                    // Single file reference
                    let p = config_dir.join(s);
                    if let Ok(c) = std::fs::read_to_string(&p) {
                        if let Ok(m) = serde_yml::from_str::<HashMap<String, serde_json::Value>>(&c)
                        {
                            map.extend(m);
                        }
                    }
                } else if let Some(arr) = v.as_array() {
                    for (i, elem) in arr.iter().enumerate() {
                        if let Some(s) = elem.as_str() {
                            let p = config_dir.join(s);
                            if let Ok(c) = std::fs::read_to_string(&p) {
                                if let Ok(m) =
                                    serde_yml::from_str::<HashMap<String, serde_json::Value>>(&c)
                                {
                                    // For array, second files override? Upstream deletes keys from first file that exist in later files
                                    // For simplicity, just extend, later files override
                                    if i == 0 {
                                        map.extend(m);
                                    } else {
                                        for (k, v) in m {
                                            map.insert(k, v);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            map
        };
        // If devices/groups were specified as string/array, they were deserialized as empty, now fill from files
        if config.devices.is_empty() {
            let m = load_map("devices");
            if !m.is_empty() {
                for (k, v) in m {
                    if let Ok(dc) = serde_json::from_value::<DeviceConfig>(v) {
                        config.devices.insert(k, dc);
                    }
                }
            }
        }
        if config.groups.is_empty() {
            let m = load_map("groups");
            if !m.is_empty() {
                for (k, v) in m {
                    if let Ok(gc) = serde_json::from_value::<GroupConfig>(v) {
                        config.groups.insert(k, gc);
                    }
                }
            }
        }
        // Resolve !secret / !env refs as in settings.ts interpretValue
        resolve_secret(&mut config.mqtt.server, config_dir);
        if let Some(ref mut u) = config.mqtt.username {
            resolve_secret(u, config_dir);
        }
        if let Some(ref mut p) = config.mqtt.password {
            resolve_secret(p, config_dir);
        }
        // Accept zigbee2mqtt-style URLs like "mqtt://localhost:1883" in mqtt.server.
        let raw = config.mqtt.server.trim().to_string();
        let stripped = raw
            .strip_prefix("mqtt://")
            .or_else(|| raw.strip_prefix("mqtts://"))
            .unwrap_or(&raw)
            .trim()
            .to_string();
        let colon_count = stripped.matches(':').count();
        let bracketed = stripped.starts_with('[');
        if let Some((host, port)) = stripped.rsplit_once(':') {
            if !host.is_empty() && (bracketed || colon_count == 1) {
                if let Ok(p) = port.parse::<u16>() {
                    config.mqtt.server = host.to_string();
                    config.mqtt.port = p;
                    // done
                } else {
                    config.mqtt.server = stripped;
                }
            } else {
                config.mqtt.server = stripped;
            }
        } else {
            config.mqtt.server = stripped;
        }
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.advanced.channel < 11 || self.advanced.channel > 26 {
            return Err(Error::Config(format!(
                "Zigbee channel must be 11-26, got {}",
                self.advanced.channel
            )));
        }
        if self.serial.port.trim().is_empty() {
            return Err(Error::Config("serial.port must not be empty".to_string()));
        }
        if self.mqtt.server.trim().is_empty() {
            return Err(Error::Config("mqtt.server must not be empty".to_string()));
        }
        if self.mqtt.base_topic.trim().is_empty() {
            return Err(Error::Config(
                "mqtt.base_topic must not be empty".to_string(),
            ));
        }
        Ok(())
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        // Read original file to detect !secret and separate files as in settings.ts write()
        let actual_content = std::fs::read_to_string(path).unwrap_or_default();
        let actual: serde_json::Value =
            serde_yml::from_str(&actual_content).unwrap_or(serde_json::Value::Null);
        let config_dir = path.parent().unwrap_or(Path::new("."));
        let mut to_write = serde_json::to_value(self)
            .map_err(|e| Error::Config(format!("cannot serialize config: {e}")))?;

        // Handle !secret refs for mqtt/server, mqtt/user, mqtt/password, advanced/network_key, frontend/auth_token
        for (ns, key) in [
            ("mqtt", "server"),
            ("mqtt", "user"),
            ("mqtt", "password"),
            ("advanced", "network_key"),
            ("frontend", "auth_token"),
        ] {
            if let Some(actual_ns) = actual
                .get(ns)
                .and_then(|v| v.get(key).and_then(|x| x.as_str()))
            {
                if let Some((fname, ref_key)) = parse_value_ref(actual_ns) {
                    if let Some(new_val) = to_write.get(ns).and_then(|v| v.get(key)).cloned() {
                        // Write the actual value to the referenced file
                        let secret_path = config_dir.join(&fname);
                        let mut secret_content: serde_json::Value =
                            std::fs::read_to_string(&secret_path)
                                .ok()
                                .and_then(|c| serde_yml::from_str(&c).ok())
                                .unwrap_or(json!({}));
                        if let Some(obj) = secret_content.as_object_mut() {
                            obj.insert(ref_key, new_val);
                        }
                        if let Ok(yaml) = serde_yml::to_string(&secret_content) {
                            let _ = std::fs::write(&secret_path, yaml);
                        }
                        // Keep the !secret ref in main file
                        if let Some(obj) = to_write.get_mut(ns).and_then(|v| v.as_object_mut()) {
                            obj.insert(
                                key.to_string(),
                                serde_json::Value::String(actual_ns.to_string()),
                            );
                        }
                    }
                }
            }
        }

        // Handle devices/groups separate files
        for key in ["devices", "groups"] {
            if let Some(actual_val) = actual.get(key) {
                if let Some(s) = actual_val.as_str() {
                    // Single file
                    let p = config_dir.join(s);
                    if let Some(map) = to_write.get(key).and_then(|v| v.as_object()) {
                        let _ = std::fs::write(&p, serde_yml::to_string(map).unwrap_or_default());
                    }
                    // Keep the string reference in main file
                    if let Some(obj) = to_write.as_object_mut() {
                        obj.insert(key.to_string(), serde_json::Value::String(s.to_string()));
                    }
                } else if let Some(arr) = actual_val.as_array() {
                    if let Some(first) = arr.first().and_then(|x| x.as_str()) {
                        let p = config_dir.join(first);
                        if let Some(map) = to_write.get(key).and_then(|v| v.as_object()) {
                            // For array, only write to first file, excluding keys from other files
                            let mut to_write_map = map.clone();
                            for elem in arr.iter().skip(1) {
                                if let Some(s) = elem.as_str() {
                                    let pp = config_dir.join(s);
                                    if let Ok(c) = std::fs::read_to_string(&pp) {
                                        if let Ok(m) = serde_yml::from_str::<
                                            HashMap<String, serde_json::Value>,
                                        >(&c)
                                        {
                                            for k in m.keys() {
                                                to_write_map.remove(k);
                                            }
                                        }
                                    }
                                }
                            }
                            let _ = std::fs::write(
                                &p,
                                serde_yml::to_string(&to_write_map).unwrap_or_default(),
                            );
                        }
                        if let Some(obj) = to_write.as_object_mut() {
                            obj.insert(key.to_string(), actual_val.clone());
                        }
                    }
                }
            }
        }

        let yaml = serde_yml::to_string(&to_write)
            .map_err(|e| Error::Config(format!("cannot serialize config: {e}")))?;
        // Atomic write via tmp
        let tmp = path.with_extension("yaml.tmp");
        std::fs::write(&tmp, yaml)
            .map_err(|e| Error::Config(format!("cannot write {}: {e}", tmp.display())))?;
        std::fs::rename(&tmp, path)
            .map_err(|e| Error::Config(format!("cannot rename {}: {e}", path.display())))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mqtt_config_debug_redacts_password() {
        let cfg = MqttConfig {
            password: Some("hunter2".to_string()),
            ..MqttConfig::default()
        };
        let debug = format!("{cfg:?}");
        assert!(!debug.contains("hunter2"));
        assert!(debug.contains("<redacted>"));
    }

    #[test]
    fn advanced_config_debug_redacts_network_key() {
        let cfg = AdvancedConfig::default();
        let debug = format!("{cfg:?}");
        let raw_key_debug = format!("{:?}", cfg.network_key);
        assert!(!debug.contains(&raw_key_debug));
        assert!(debug.contains("<redacted>"));
    }

    #[test]
    fn validate_accepts_defaults() {
        assert!(Config::default().validate().is_ok());
    }

    #[test]
    fn validate_rejects_channel_out_of_range() {
        let mut cfg = Config::default();
        cfg.advanced.channel = 30;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_serial_port() {
        let mut cfg = Config::default();
        cfg.serial.port = "  ".to_string();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_mqtt_server() {
        let mut cfg = Config::default();
        cfg.mqtt.server = String::new();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_base_topic() {
        let mut cfg = Config::default();
        cfg.mqtt.base_topic = String::new();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn output_mode_defaults_to_json() {
        // Matches upstream zigbee2mqtt's own default -- see OutputMode's
        // doc comment.
        assert_eq!(AdvancedConfig::default().output, OutputMode::Json);
        // Omitting `output` entirely from YAML must default the same way.
        let cfg: AdvancedConfig = serde_yml::from_str("channel: 15").unwrap();
        assert_eq!(cfg.output, OutputMode::Json);
    }

    #[test]
    fn output_mode_parses_all_three_z2m_values() {
        assert_eq!(
            serde_yml::from_str::<OutputMode>("json").unwrap(),
            OutputMode::Json
        );
        assert_eq!(
            serde_yml::from_str::<OutputMode>("attribute").unwrap(),
            OutputMode::Attribute
        );
        assert_eq!(
            serde_yml::from_str::<OutputMode>("attribute_and_json").unwrap(),
            OutputMode::AttributeAndJson
        );
    }

    #[test]
    fn output_mode_rejects_unknown_value() {
        assert!(serde_yml::from_str::<OutputMode>("bogus").is_err());
    }

    #[test]
    fn shipped_example_config_parses() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config.example.yaml");
        let content = std::fs::read_to_string(&path).expect("config.example.yaml must exist");
        let cfg: Config = serde_yml::from_str(&content).expect("must parse as valid Config");
        // The example omits `advanced:` entirely -- it must still resolve
        // to actual defaults via Config's struct-level `#[serde(default)]`.
        assert_eq!(cfg.advanced.channel, 11);
        assert_eq!(cfg.advanced.output, OutputMode::Json);
        cfg.validate().expect("example config must pass validation");
    }
}
