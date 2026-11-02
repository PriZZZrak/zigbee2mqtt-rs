use serde_json::Value;

#[derive(Debug, Clone)]
pub struct DefinitionWithExtend {
    pub zigbee_model: Vec<String>,
    pub model: String,
    pub vendor: String,
    pub description: String,
    pub from_zigbee: Vec<String>,
    pub to_zigbee: Vec<String>,
    pub exposes: Vec<Value>,
}

pub type FzConverter = fn(&Value) -> Option<Value>;
pub type TzConverter = fn(Value) -> Option<Vec<u8>>;
