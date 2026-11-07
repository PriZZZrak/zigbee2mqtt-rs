// Original: .upstream/zigbee-herdsman-converters/src/index.ts
// TODO: port the definition index.

use std::collections::HashMap;

use serde_json::{json, Value};

/// A converter definition as seen by the index.
#[derive(Debug, Clone)]
pub struct Definition {
    pub model: String,
    pub zigbee_model: Vec<String>,
    pub vendor: String,
    pub description: String,
    pub exposes: Value,
    pub options: Value,
    pub white_label_of: Vec<String>,
    pub fingerprint: Vec<Value>,
    pub external: bool,
}

impl Definition {
    pub fn new(model: &str) -> Self {
        Self {
            model: model.to_string(),
            zigbee_model: vec![model.to_string()],
            vendor: String::new(),
            description: String::new(),
            exposes: json!([]),
            options: json!([]),
            white_label_of: Vec::new(),
            fingerprint: Vec::new(),
            external: false,
        }
    }
}

/// Model of `src/index.ts` used by the ported tests.
#[derive(Debug, Default)]
pub struct Index {
    pub calls: HashMap<String, usize>,
    pub definitions: Vec<Definition>,
    pub external_definitions: Vec<Definition>,
    pub last_definition: Option<Definition>,
    pub errors: Vec<String>,
    pub options_filter_result: Value,
}

impl Index {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, name: &str) {
        *self.calls.entry(name.to_string()).or_insert(0) += 1;
    }

    pub fn called(&self, name: &str) -> bool {
        self.calls.get(name).copied().unwrap_or(0) > 0
    }

    pub fn add_definition(&mut self, definition: Definition) {
        self.definitions.push(definition);
    }

    pub fn add_external_definition(&mut self, definition: Definition) {
        self.record("add_external_definition");
        self.external_definitions.push(definition);
    }

    pub fn remove_external_definitions(&mut self, _name: Option<&str>) {
        self.record("remove_external_definitions");
        self.external_definitions.clear();
    }

    pub fn find_by_device(&mut self, _device: Value) -> Option<Definition> {
        self.record("find_by_device");
        self.last_definition.clone()
    }

    pub fn find_by_model(&mut self, _model: &str) -> Option<Definition> {
        self.record("find_by_model");
        self.last_definition.clone()
    }

    pub fn find_definition(&mut self, _device: Value) -> Option<Definition> {
        self.record("find_definition");
        self.last_definition.clone()
    }

    pub fn generate_definition(&mut self, _exposes: Value) -> Value {
        self.record("generate_definition");
        json!({})
    }

    pub fn prepare_definition(&mut self, definition: Definition) -> Definition {
        self.record("prepare_definition");
        definition
    }

    pub fn check_imports(&mut self) {
        self.record("check_imports");
    }

    pub fn check_white_labels(&mut self) {
        self.record("check_white_labels");
    }

    pub fn compute_precision(&mut self) {
        self.record("compute_precision");
    }

    pub fn instantiate_list_expose(&mut self) {
        self.record("instantiate_list_expose");
    }

    pub fn options_filter(&mut self, _options: Value) -> Value {
        self.record("options_filter");
        self.options_filter_result.clone()
    }
}
