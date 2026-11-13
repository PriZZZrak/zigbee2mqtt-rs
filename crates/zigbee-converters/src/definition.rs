// Original: .upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters/dist/lib/exposes.js (`Definition`)
// Upstream revision: zigbee-herdsman-converters 26.125.0 (zigbee2mqtt 2.14.3).
//
// Converter definition shape shared by the bridge, including the callable
// `fromZigbee`/`toZigbee` lists (`ConvertData`, `ConvertMeta`, publish callback).

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;

use crate::exposes::{Exposes, Fingerprint, WhiteLabel};

/// Boxed converter future (upstream `async convert(...)`).
pub type ConverterFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

/// Upstream `ConvertData` handed to `fromZigbee` converters.
#[derive(Clone)]
pub struct ConvertData {
    pub r#type: String,
    pub device: Arc<zigbee_apiarist::controller::model::device::Device>,
    pub endpoint: Arc<zigbee_apiarist::controller::model::endpoint::Endpoint>,
    pub cluster: String,
    pub data: Value,
    pub group_id: Option<u16>,
    pub linkquality: u8,
    pub meta: Value,
}

/// Upstream `publish` callback passed to `fromZigbee` converters.
pub type PublishCallback = Arc<dyn Fn(Value) -> ConverterFuture<()> + Send + Sync>;

/// Upstream converter `meta` argument.
#[derive(Clone)]
pub struct ConvertMeta {
    pub device: Arc<zigbee_apiarist::controller::model::device::Device>,
    pub state: Value,
    pub device_exposes_changed: Arc<dyn Fn() + Send + Sync>,
}

/// Upstream `fromZigbee` converter entry.
#[derive(Clone)]
pub struct FromZigbeeConverter {
    pub cluster: String,
    pub types: Vec<String>,
    #[allow(clippy::type_complexity)]
    pub convert: Arc<
        dyn Fn(
                DefinitionRef,
                ConvertData,
                PublishCallback,
                Value,
                ConvertMeta,
            ) -> ConverterFuture<Result<Option<Value>, String>>
            + Send
            + Sync,
    >,
}

/// Upstream `toZigbee` converter entry.
#[derive(Clone)]
pub struct ToZigbeeConverter {
    pub key: Vec<String>,
    #[allow(clippy::type_complexity)]
    pub convert: Arc<
        dyn Fn(
                to_zigbee::ToConvertData,
                PublishCallback,
                Value,
                ConvertMeta,
            ) -> ConverterFuture<Result<Option<Value>, String>>
            + Send
            + Sync,
    >,
}

/// Upstream `toZigbee` converter payload.
pub mod to_zigbee {
    use std::sync::Arc;

    use serde_json::Value;

    #[derive(Clone)]
    pub struct ToConvertData {
        pub device: Arc<zigbee_apiarist::controller::model::device::Device>,
        pub endpoint: Option<Arc<zigbee_apiarist::controller::model::endpoint::Endpoint>>,
        pub key: Vec<String>,
        pub value: Value,
        pub meta: Value,
    }
}

/// Upstream `Definition` (`lib/exposes.js`).
pub struct Definition {
    pub model: String,
    pub vendor: String,
    pub description: String,
    /// Upstream `zigbeeModel` (string or array).
    pub zigbee_model: Vec<String>,
    pub exposes: Exposes,
    /// Upstream `options` entries are passed through unchanged.
    pub options: Vec<Value>,
    /// Upstream `meta` (`{multiEndpoint, publishDuplicateTransaction, ...}`).
    pub meta: Value,
    /// Upstream `endpoint` mapping (name → endpoint ID).
    pub endpoint: Vec<(String, u8)>,
    pub configure: Option<Value>,
    pub ota: Option<Value>,
    pub from_zigbee: Vec<FromZigbeeConverter>,
    pub to_zigbee: Vec<ToZigbeeConverter>,
    pub generated: bool,
    pub white_label: Vec<WhiteLabel>,
    pub fingerprint: Vec<Fingerprint>,
}

impl std::fmt::Debug for Definition {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Definition")
            .field("model", &self.model)
            .field("vendor", &self.vendor)
            .field("description", &self.description)
            .field("zigbee_model", &self.zigbee_model)
            .field("generated", &self.generated)
            .finish_non_exhaustive()
    }
}

impl Definition {
    pub fn new(model: &str, vendor: &str, description: &str) -> Self {
        Self {
            model: model.to_string(),
            vendor: vendor.to_string(),
            description: description.to_string(),
            zigbee_model: Vec::new(),
            exposes: Exposes::Static(Vec::new()),
            options: Vec::new(),
            meta: Value::Null,
            endpoint: Vec::new(),
            configure: None,
            ota: None,
            from_zigbee: Vec::new(),
            to_zigbee: Vec::new(),
            generated: false,
            white_label: Vec::new(),
            fingerprint: Vec::new(),
        }
    }

    pub fn with_zigbee_model(mut self, model: &str) -> Self {
        self.zigbee_model.push(model.to_string());
        self
    }

    pub fn with_exposes(mut self, exposes: Vec<crate::exposes::Expose>) -> Self {
        self.exposes = Exposes::Static(exposes);
        self
    }

    pub fn with_fingerprint(mut self, fingerprint: Fingerprint) -> Self {
        self.fingerprint.push(fingerprint);
        self
    }

    pub fn with_white_label(mut self, white_label: WhiteLabel) -> Self {
        self.white_label.push(white_label);
        self
    }

    /// Upstream `definition.endpoint(device)` mapping.
    pub fn endpoint_id(&self, name: &str) -> Option<u8> {
        self.endpoint
            .iter()
            .find(|(endpoint_name, _)| endpoint_name == name)
            .map(|(_, id)| *id)
    }
}

pub type DefinitionRef = Arc<Definition>;
