// Original: .upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters/dist/lib/exposes.js
// Upstream revision: zigbee-herdsman-converters 26.125.0 (zigbee2mqtt 2.14.3).
//
// Shared exposes model. R1 ports the data model and the lookup/loading
// mechanisms; the converter database and vendor-specific `modernExtend`
// helpers stay out (AGENTS §5 Q4).

use std::sync::Arc;

use serde_json::Value;

/// Upstream `access` bit flags (`lib/access.js`).
pub mod access {
    pub const NONE: u32 = 0;
    pub const STATE: u32 = 1;
    pub const SET: u32 = 2;
    pub const GET: u32 = 4;
    pub const ALL: u32 = STATE | SET | GET;
}

/// Upstream `Expose` union (`lib/exposes.js`). Common fields are flattened,
/// type-specific payloads stay in `values`/`features`/numeric bounds.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Expose {
    pub r#type: String,
    pub name: Option<String>,
    pub property: Option<String>,
    pub description: Option<String>,
    pub access: u32,
    pub category: Option<String>,
    pub unit: Option<String>,
    pub values: Vec<String>,
    pub value_min: Option<f64>,
    pub value_max: Option<f64>,
    pub value_step: Option<f64>,
    /// Upstream `exposes`/`features` for composite/light/switch/cover/list.
    pub features: Vec<Expose>,
    pub endpoint: Option<String>,
    /// Upstream `label` (HA-facing friendly feature name).
    pub label: Option<String>,
    /// Upstream `value_on` / `value_off` for binary/switch exposes.
    pub value_on: Value,
    pub value_off: Value,
    /// Upstream `homeassistant` expose overrides.
    pub homeassistant: Value,
    /// Raw upstream fields not modelled yet (e.g. `presets`, `color`).
    pub extra: Value,
}

impl Expose {
    /// Upstream `new Numeric(...)` (`exposes.js`).
    pub fn numeric(name: &str, access: u32) -> Self {
        Self {
            r#type: "numeric".to_string(),
            name: Some(name.to_string()),
            property: Some(name.to_string()),
            access,
            label: Some(name.to_string()),
            ..Default::default()
        }
    }

    pub fn binary(name: &str, access: u32) -> Self {
        Self {
            r#type: "binary".to_string(),
            name: Some(name.to_string()),
            property: Some(name.to_string()),
            access,
            label: Some(name.to_string()),
            value_on: Value::Bool(true),
            value_off: Value::Bool(false),
            ..Default::default()
        }
    }

    pub fn enumeration(name: &str, access: u32, values: &[&str]) -> Self {
        Self {
            r#type: "enum".to_string(),
            name: Some(name.to_string()),
            property: Some(name.to_string()),
            access,
            label: Some(name.to_string()),
            values: values.iter().map(|value| (*value).to_string()).collect(),
            ..Default::default()
        }
    }

    pub fn text(name: &str, access: u32) -> Self {
        Self {
            r#type: "text".to_string(),
            name: Some(name.to_string()),
            property: Some(name.to_string()),
            access,
            label: Some(name.to_string()),
            ..Default::default()
        }
    }
}

/// Upstream `Definition.exposes` accepts an array or a function of the device.
pub enum Exposes {
    Static(Vec<Expose>),
    Dynamic(Arc<dyn Fn(&Value, &Value) -> Vec<Expose> + Send + Sync>),
}

impl Clone for Exposes {
    fn clone(&self) -> Self {
        match self {
            Self::Static(exposes) => Self::Static(exposes.clone()),
            Self::Dynamic(resolve) => Self::Dynamic(resolve.clone()),
        }
    }
}

impl std::fmt::Debug for Exposes {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static(exposes) => formatter.debug_tuple("Static").field(exposes).finish(),
            Self::Dynamic(_) => formatter.write_str("Dynamic(..)"),
        }
    }
}

impl Exposes {
    pub fn resolve(&self, device: &Value, options: &Value) -> Vec<Expose> {
        match self {
            Self::Static(exposes) => exposes.clone(),
            Self::Dynamic(resolve) => resolve(device, options),
        }
    }
}

/// Upstream `Fingerprint` (`lib/exposes.js`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Fingerprint {
    pub model_id: Option<String>,
    pub manufacturer_name: Option<String>,
    pub type_: Option<String>,
    pub endpoint_id: Option<u8>,
    pub priority: i32,
}

/// Upstream `WhiteLabel` (`lib/exposes.js`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WhiteLabel {
    pub model: String,
    pub vendor: Option<String>,
    pub description: Option<String>,
    pub fingerprint: Vec<Fingerprint>,
}

/// Upstream `utils.isLightExpose` (`utils.ts:352`).
pub fn is_light_expose(expose: &Expose) -> bool {
    expose.r#type == "light"
}

impl Expose {
    pub fn feature_names(&self) -> Vec<&str> {
        self.features
            .iter()
            .filter_map(|feature| feature.name.as_deref())
            .collect()
    }
}

impl Expose {
    /// Upstream expose JSON (`bridge/devices[].definition.exposes`).
    pub fn to_json(&self) -> Value {
        let mut map = serde_json::Map::new();
        map.insert("type".to_string(), Value::String(self.r#type.clone()));
        if let Some(name) = &self.name {
            map.insert("name".to_string(), Value::String(name.clone()));
        }
        if let Some(property) = &self.property {
            map.insert("property".to_string(), Value::String(property.clone()));
        }
        if self.access != 0 {
            map.insert("access".to_string(), Value::from(self.access));
        }
        if let Some(description) = &self.description {
            map.insert("description".to_string(), Value::String(description.clone()));
        }
        if let Some(category) = &self.category {
            map.insert("category".to_string(), Value::String(category.clone()));
        }
        if let Some(unit) = &self.unit {
            map.insert("unit".to_string(), Value::String(unit.clone()));
        }
        if !self.values.is_empty() {
            map.insert(
                "values".to_string(),
                Value::Array(self.values.iter().cloned().map(Value::String).collect()),
            );
        }
        if let Some(minimum) = self.value_min {
            map.insert("value_min".to_string(), Value::from(minimum));
        }
        if let Some(maximum) = self.value_max {
            map.insert("value_max".to_string(), Value::from(maximum));
        }
        if let Some(step) = self.value_step {
            map.insert("value_step".to_string(), Value::from(step));
        }
        if !self.features.is_empty() {
            map.insert(
                "features".to_string(),
                Value::Array(self.features.iter().map(Expose::to_json).collect()),
            );
        }
        if let Some(endpoint) = &self.endpoint {
            map.insert("endpoint".to_string(), Value::String(endpoint.clone()));
        }
        if let Value::Object(extra) = &self.extra {
            for (key, value) in extra {
                map.insert(key.clone(), value.clone());
            }
        }
        Value::Object(map)
    }
}

impl Expose {
    /// Upstream `stateOn/stateOff` for switch-like exposes.
    pub fn with_value_on(mut self, value: Value) -> Self {
        self.value_on = value;
        self
    }
    pub fn with_value_off(mut self, value: Value) -> Self {
        self.value_off = value;
        self
    }
}
