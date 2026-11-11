// Original: .upstream/zigbee-herdsman/src/zspec/zcl/utils.ts
//
// The types below mirror upstream names from
// `definition/tstype.ts` (`Cluster`, `Attribute`, `Parameter`,
// `CustomClusters`) and only cover what the ported tests exercise.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::zspec::zcl::definition::enums::{BuffaloZclDataType, DataType};

/// Upstream runtime value (JS number/string/null, including `NaN`).
#[derive(Debug, Clone, PartialEq)]
pub enum ZclValue {
    Null,
    Bool(bool),
    Number(f64),
    Text(String),
}

impl ZclValue {
    pub fn num(value: impl Into<ZclValue>) -> Self {
        value.into()
    }

    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    pub fn is_nan(&self) -> bool {
        matches!(self, Self::Number(value) if value.is_nan())
    }

    /// Conversion used by the controller layer (upstream JS values).
    pub fn from_json(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(value) => Self::Bool(*value),
            serde_json::Value::Number(value) => Self::Number(value.as_f64().unwrap_or(f64::NAN)),
            serde_json::Value::String(value) => Self::Text(value.clone()),
            other => Self::Text(other.to_string()),
        }
    }

    /// Conversion used by the controller layer (upstream JS values).
    ///
    /// JavaScript has a single number type: `JSON.stringify(5)` is `5`, so
    /// integral values are emitted as integers to keep the frame encoders
    /// (`Value::as_u64`) and wire bytes identical to upstream.
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(value) => serde_json::Value::Bool(*value),
            Self::Number(value) => {
                if value.is_nan() {
                    serde_json::Value::Null
                } else if value.fract() == 0.0
                    && *value >= -(2f64.powi(63))
                    && *value < 2f64.powi(63)
                {
                    serde_json::json!(*value as i64)
                } else {
                    serde_json::json!(value)
                }
            }
            Self::Text(value) => serde_json::Value::String(value.clone()),
        }
    }

    fn is_len(&self, length: usize) -> bool {
        match self {
            Self::Text(value) => value.len() == length,
            _ => false,
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Text(value) => value.len(),
            _ => 0,
        }
    }
}

impl From<bool> for ZclValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i32> for ZclValue {
    fn from(value: i32) -> Self {
        Self::Number(value as f64)
    }
}

impl From<u16> for ZclValue {
    fn from(value: u16) -> Self {
        Self::Number(value as f64)
    }
}

impl From<u32> for ZclValue {
    fn from(value: u32) -> Self {
        Self::Number(value as f64)
    }
}

impl From<i64> for ZclValue {
    fn from(value: i64) -> Self {
        Self::Number(value as f64)
    }
}

impl From<f64> for ZclValue {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

/// Upstream `Attribute`.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    /// Upstream camelCase: `ID`.
    pub ID: u16,
    pub name: String,
    /// Upstream camelCase: `type`.
    pub r#type: DataType,
    pub write: bool,
    pub read: bool,
    pub required: bool,
    pub default: Option<ZclValue>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub minExcl: Option<f64>,
    pub maxExcl: Option<f64>,
    pub length: Option<usize>,
    pub minLen: Option<usize>,
    pub maxLen: Option<usize>,
    pub manufacturerCode: Option<u16>,
    /// Upstream `special` map values that bypass restrictions / sentinel handling.
    pub special: Vec<ZclValue>,
}

impl Attribute {
    pub fn new(name: &str, ID: u16, r#type: DataType) -> Self {
        Self {
            ID,
            name: name.to_string(),
            r#type,
            write: false,
            read: true,
            required: false,
            default: None,
            min: None,
            max: None,
            minExcl: None,
            maxExcl: None,
            length: None,
            minLen: None,
            maxLen: None,
            manufacturerCode: None,
            special: Vec::new(),
        }
    }

    pub fn writable(mut self) -> Self {
        self.write = true;
        self
    }
}

/// Upstream `ParameterCondition` with owned strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterCondition {
    MINIMUM_REMAINING_BUFFER_BYTES(u32),
    BITMASK_SET {
        field: String,
        value: u32,
        reversed: bool,
    },
    BITFIELD_ENUM {
        field: String,
        offset: u8,
        size: u8,
        value: u32,
    },
    FIELD_EQUAL {
        field: String,
        value: u32,
        reversed: bool,
    },
    FIELD_GT {
        field: String,
        value: u32,
    },
}

impl ParameterCondition {
    pub fn from_static(
        condition: &crate::zspec::zcl::definition::cluster::ParameterCondition,
    ) -> Self {
        use crate::zspec::zcl::definition::cluster::ParameterCondition as Static;
        match condition {
            Static::MINIMUM_REMAINING_BUFFER_BYTES(value) => {
                Self::MINIMUM_REMAINING_BUFFER_BYTES(*value)
            }
            Static::BITMASK_SET {
                field,
                value,
                reversed,
            } => Self::BITMASK_SET {
                field: (*field).to_string(),
                value: *value,
                reversed: *reversed,
            },
            Static::BITFIELD_ENUM {
                field,
                offset,
                size,
                value,
            } => Self::BITFIELD_ENUM {
                field: (*field).to_string(),
                offset: *offset,
                size: *size,
                value: *value,
            },
            Static::FIELD_EQUAL {
                field,
                value,
                reversed,
            } => Self::FIELD_EQUAL {
                field: (*field).to_string(),
                value: *value,
                reversed: *reversed,
            },
        }
    }
}

/// Upstream `Parameter`.
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    /// Upstream camelCase: `type`.
    pub r#type: DataType,
    /// Upstream `type` may also be a `BuffaloZclDataType` (e.g. `GPD_FRAME`).
    pub buffalo_type: Option<BuffaloZclDataType>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub minExcl: Option<f64>,
    pub maxExcl: Option<f64>,
    pub length: Option<usize>,
    pub minLen: Option<usize>,
    pub maxLen: Option<usize>,
    pub special: Vec<ZclValue>,
    pub conditions: Vec<ParameterCondition>,
}

impl Parameter {
    pub fn new(name: &str, r#type: DataType) -> Self {
        Self {
            name: name.to_string(),
            r#type,
            buffalo_type: None,
            min: None,
            max: None,
            minExcl: None,
            maxExcl: None,
            length: None,
            minLen: None,
            maxLen: None,
            special: Vec::new(),
            conditions: Vec::new(),
        }
    }

    pub fn with_conditions(mut self, conditions: Vec<ParameterCondition>) -> Self {
        self.conditions = conditions;
        self
    }

    pub fn with_buffalo_type(mut self, buffalo_type: BuffaloZclDataType) -> Self {
        self.buffalo_type = Some(buffalo_type);
        self
    }
}

impl From<&crate::zspec::zcl::definition::cluster::Parameter> for Parameter {
    fn from(parameter: &crate::zspec::zcl::definition::cluster::Parameter) -> Self {
        Self {
            name: parameter.name.to_string(),
            r#type: parameter.r#type,
            buffalo_type: None,
            min: None,
            max: parameter.max.map(f64::from),
            minExcl: None,
            maxExcl: None,
            length: None,
            minLen: None,
            maxLen: None,
            special: Vec::new(),
            conditions: parameter
                .conditions
                .iter()
                .map(ParameterCondition::from_static)
                .collect(),
        }
    }
}

/// Upstream `Command`.
#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub name: String,
    /// Upstream camelCase: `ID`.
    pub ID: u8,
    pub response: Option<u8>,
    pub parameters: Vec<Parameter>,
    pub required: bool,
}

impl Command {
    pub fn new(name: &str, ID: u8) -> Self {
        Self {
            name: name.to_string(),
            ID,
            response: None,
            parameters: Vec::new(),
            required: true,
        }
    }

    pub fn with_parameters(mut self, parameters: Vec<Parameter>) -> Self {
        self.parameters = parameters;
        self
    }

    pub fn with_response(mut self, response: u8) -> Self {
        self.response = Some(response);
        self
    }
}

/// Upstream `Cluster`.
#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    pub name: String,
    /// Upstream camelCase: `ID`.
    pub ID: u32,
    pub manufacturerCode: Option<u16>,
    pub attributes: HashMap<String, Attribute>,
    pub commands: HashMap<String, Command>,
    pub commandsResponse: HashMap<String, Command>,
}

impl Cluster {
    pub fn new(name: &str, ID: u32) -> Self {
        Self {
            name: name.to_string(),
            ID,
            manufacturerCode: None,
            attributes: HashMap::new(),
            commands: HashMap::new(),
            commandsResponse: HashMap::new(),
        }
    }

    fn with_attribute(mut self, attribute: Attribute) -> Self {
        self.attributes.insert(attribute.name.clone(), attribute);
        self
    }

    fn with_command(mut self, command: Command) -> Self {
        self.commands.insert(command.name.clone(), command);
        self
    }

    fn with_command_response(mut self, command: Command) -> Self {
        self.commandsResponse.insert(command.name.clone(), command);
        self
    }

    fn commands_response_mut(&mut self) -> &mut HashMap<String, Command> {
        &mut self.commandsResponse
    }
}

/// Upstream `CustomClusters` — a map of cluster name to cluster definition.
#[derive(Debug, Clone, Default)]
pub struct CustomClusters {
    clusters: Vec<Cluster>,
}

impl CustomClusters {
    pub fn empty() -> Self {
        Self {
            clusters: Vec::new(),
        }
    }

    pub fn from_clusters(clusters: Vec<Cluster>) -> Self {
        Self { clusters }
    }

    /// Insert/replace by cluster name (upstream object assignment semantics).
    pub fn insert(&mut self, cluster: Cluster) {
        if let Some(existing) = self
            .clusters
            .iter_mut()
            .find(|existing| existing.name == cluster.name)
        {
            *existing = cluster;
        } else {
            self.clusters.push(cluster);
        }
    }

    pub fn get(&self, name: &str) -> Option<&Cluster> {
        self.clusters.iter().find(|cluster| cluster.name == name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Cluster> {
        self.clusters
            .iter_mut()
            .find(|cluster| cluster.name == name)
    }

    pub fn clusters(&self) -> impl Iterator<Item = &Cluster> {
        self.clusters.iter()
    }

    pub fn len(&self) -> usize {
        self.clusters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.clusters.is_empty()
    }
}

/// Upstream `ManufacturerCode` values used by the ported tests.
pub mod manufacturer_code {
    pub const AMAZON_LAB126: u16 = 0x1217;
    pub const DANFOSS_A_S: u16 = 0x1246;
}

pub enum ClusterKey {
    Id(u32),
    Name(String),
}

impl From<u16> for ClusterKey {
    fn from(value: u16) -> Self {
        Self::Id(value as u32)
    }
}

impl From<u32> for ClusterKey {
    fn from(value: u32) -> Self {
        Self::Id(value)
    }
}

impl From<i32> for ClusterKey {
    fn from(value: i32) -> Self {
        Self::Id(value as u32)
    }
}

impl From<&str> for ClusterKey {
    fn from(value: &str) -> Self {
        Self::Name(value.to_string())
    }
}

impl From<String> for ClusterKey {
    fn from(value: String) -> Self {
        Self::Name(value)
    }
}

pub enum AttributeKey {
    Id(u32),
    Name(String),
}

impl From<u16> for AttributeKey {
    fn from(value: u16) -> Self {
        Self::Id(value as u32)
    }
}

impl From<u32> for AttributeKey {
    fn from(value: u32) -> Self {
        Self::Id(value)
    }
}

impl From<i32> for AttributeKey {
    fn from(value: i32) -> Self {
        Self::Id(value as u32)
    }
}

impl From<&str> for AttributeKey {
    fn from(value: &str) -> Self {
        Self::Name(value.to_string())
    }
}

pub enum CommandKey {
    Id(u32),
    Name(String),
}

impl From<u16> for CommandKey {
    fn from(value: u16) -> Self {
        Self::Id(value as u32)
    }
}

impl From<u32> for CommandKey {
    fn from(value: u32) -> Self {
        Self::Id(value)
    }
}

impl From<i32> for CommandKey {
    fn from(value: i32) -> Self {
        Self::Id(value as u32)
    }
}

impl From<u8> for CommandKey {
    fn from(value: u8) -> Self {
        Self::Id(value as u32)
    }
}

impl From<&str> for CommandKey {
    fn from(value: &str) -> Self {
        Self::Name(value.to_string())
    }
}

/// Upstream `Zcl.Utils`.
pub struct Utils;

impl Utils {
    /// Upstream `Utils.isAnalogDataType(type)`.
    pub fn is_analog_data_type(r#type: DataType) -> bool {
        matches!(
            r#type,
            DataType::UINT8
                | DataType::UINT16
                | DataType::UINT24
                | DataType::UINT32
                | DataType::INT8
                | DataType::INT16
                | DataType::INT24
                | DataType::INT32
                | DataType::TOD
                | DataType::DATE
                | DataType::UTC
        )
    }

    /// Upstream `Utils.getCluster(key, manufacturerCode, customClusters)`.
    pub fn get_cluster(
        key: impl Into<ClusterKey>,
        _manufacturer_code: Option<u16>,
        custom_clusters: &CustomClusters,
    ) -> Cluster {
        let key = key.into();

        for cluster in custom_clusters.clusters() {
            let matched = match &key {
                ClusterKey::Id(id) => cluster.ID == *id,
                ClusterKey::Name(name) => &cluster.name == name,
            };
            if matched
                && (!cluster.attributes.is_empty()
                    || !cluster.commands.is_empty()
                    || !cluster.commandsResponse.is_empty()
                    || builtin_cluster_by_name(&cluster.name).is_none())
            {
                return cluster.clone();
            }
        }

        match key {
            ClusterKey::Id(id) => builtin_cluster_by_id(id),
            ClusterKey::Name(name) => builtin_cluster_by_name(&name)
                .unwrap_or_else(|| panic!("Cluster with name '{name}' does not exist")),
        }
    }

    /// Upstream `Utils.getClusterAttribute(cluster, key, manufacturerCode)`.
    pub fn get_cluster_attribute(
        cluster: &Cluster,
        key: impl Into<AttributeKey>,
        manufacturer_code: Option<u16>,
    ) -> Option<Attribute> {
        let key = key.into();
        let attribute = match key {
            AttributeKey::Id(id) => cluster
                .attributes
                .values()
                .find(|attr| attr.ID as u32 == id),
            AttributeKey::Name(name) => cluster.attributes.get(&name),
        }?;

        if let Some(attribute_code) = attribute.manufacturerCode {
            if Some(attribute_code) != manufacturer_code {
                return None;
            }
        }

        Some(attribute.clone())
    }

    /// Upstream `Utils.getClusterCommand(cluster, key)`.
    pub fn get_cluster_command(cluster: &Cluster, key: impl Into<CommandKey>) -> Command {
        let key = key.into();
        let command = match key {
            CommandKey::Id(id) => cluster
                .commands
                .values()
                .find(|command| command.ID as u32 == id),
            CommandKey::Name(name) => cluster.commands.get(&name),
        };

        command
            .cloned()
            .unwrap_or_else(|| panic!("Command does not exist"))
    }

    /// Upstream `Utils.getClusterCommandResponse(cluster, key)`.
    pub fn get_cluster_command_response(cluster: &Cluster, key: impl Into<CommandKey>) -> Command {
        let key = key.into();
        let command = match key {
            CommandKey::Id(id) => cluster
                .commandsResponse
                .values()
                .find(|command| command.ID as u32 == id),
            CommandKey::Name(name) => cluster.commandsResponse.get(&name),
        };

        command
            .cloned()
            .unwrap_or_else(|| panic!("Command response does not exist"))
    }

    /// Upstream `Utils.getGlobalCommand(key)`.
    pub fn get_global_command(key: impl Into<CommandKey>) -> Command {
        use crate::zspec::zcl::definition::foundation::Foundation;
        let key = key.into();
        let name = match key {
            CommandKey::Id(id) => match id {
                0x00 => "read",
                0x01 => "readRsp",
                0x02 => "write",
                0x03 => "writeUndiv",
                0x04 => "writeRsp",
                0x05 => "writeNoRsp",
                0x06 => "configReport",
                0x07 => "configReportRsp",
                0x08 => "readReportConfig",
                0x09 => "readReportConfigRsp",
                0x0a => "report",
                0x0b => "defaultRsp",
                0x0c => "discover",
                0x0d => "discoverRsp",
                0x0e => "readStructured",
                0x0f => "writeStructured",
                0x10 => "writeStructuredRsp",
                0x11 => "discoverCommands",
                0x12 => "discoverCommandsRsp",
                0x13 => "discoverCommandsGen",
                0x14 => "discoverCommandsGenRsp",
                0x15 => "discoverExt",
                0x16 => "discoverExtRsp",
                _ => panic!("Global command does not exist"),
            },
            CommandKey::Name(name) => match name.as_str() {
                "read" => "read",
                "readRsp" => "readRsp",
                "write" => "write",
                "writeUndiv" => "writeUndiv",
                "writeRsp" => "writeRsp",
                "writeNoRsp" => "writeNoRsp",
                "configReport" => "configReport",
                "configReportRsp" => "configReportRsp",
                "readReportConfig" => "readReportConfig",
                "readReportConfigRsp" => "readReportConfigRsp",
                "report" => "report",
                "defaultRsp" => "defaultRsp",
                "discover" => "discover",
                "discoverRsp" => "discoverRsp",
                "readStructured" => "readStructured",
                "writeStructured" => "writeStructured",
                "writeStructuredRsp" => "writeStructuredRsp",
                "discoverCommands" => "discoverCommands",
                "discoverCommandsRsp" => "discoverCommandsRsp",
                "discoverCommandsGen" => "discoverCommandsGen",
                "discoverCommandsGenRsp" => "discoverCommandsGenRsp",
                "discoverExt" => "discoverExt",
                "discoverExtRsp" => "discoverExtRsp",
                _ => panic!("Global command does not exist"),
            },
        };
        let (id, response) = match name {
            "read" => (0u8, Some(1u8)),
            "readRsp" => (1, None),
            "write" => (2, Some(4)),
            "writeUndiv" => (3, None),
            "writeRsp" => (4, None),
            "writeNoRsp" => (5, None),
            "configReport" => (6, Some(7)),
            "configReportRsp" => (7, None),
            "readReportConfig" => (8, Some(9)),
            "readReportConfigRsp" => (9, None),
            "report" => (10, None),
            "defaultRsp" => (11, None),
            "discover" => (12, Some(13)),
            "discoverRsp" => (13, None),
            "readStructured" => (14, None),
            "writeStructured" => (15, Some(16)),
            "writeStructuredRsp" => (16, None),
            "discoverCommands" => (17, None),
            "discoverCommandsRsp" => (18, None),
            "discoverCommandsGen" => (19, None),
            "discoverCommandsGenRsp" => (20, None),
            "discoverExt" => (21, None),
            "discoverExtRsp" => (22, None),
            _ => unreachable!(),
        };
        let command = Command::new(name, id);
        match response {
            Some(response) => command.with_response(response),
            None => command,
        }
    }

    /// Upstream `Utils.isClusterName(name)`.
    pub fn is_cluster_name(name: &str) -> bool {
        builtin_cluster_by_name(name).is_some()
    }

    /// Upstream `Utils.getFoundationCommand(id)`.
    pub fn get_foundation_command(id: u16) -> Command {
        match id {
            0 => Command::new("read", 0).with_response(1),
            2 => Command::new("writeUndiv", 2),
            _ => panic!("Status 'UNSUP_COMMAND' foundation:{id}"),
        }
    }

    /// Upstream `Utils.getFoundationCommandByName(name)`.
    pub fn get_foundation_command_by_name(name: &str) -> Command {
        match name {
            "read" => Command::new("read", 0).with_response(1),
            "writeUndiv" => Command::new("writeUndiv", 2),
            _ => panic!("Status 'UNSUP_COMMAND' foundation:{name}"),
        }
    }

    /// Upstream `Utils.processAttributeWrite(attribute, value)`.
    pub fn process_attribute_write(attribute: &Attribute, value: ZclValue) -> ZclValue {
        if !attribute.write {
            panic!("{} ({}) is not writable", attribute.name, attribute.ID);
        }

        process_write(attribute_fields(attribute), value)
    }

    /// Upstream `Utils.processAttributePreRead(attribute)`.
    pub fn process_attribute_pre_read(attribute: &Attribute) {
        if !attribute.read {
            panic!("{} ({}) is not readable", attribute.name, attribute.ID);
        }
    }

    /// Upstream `Utils.processAttributePostRead(attribute, value)`.
    pub fn process_attribute_post_read(attribute: &Attribute, value: ZclValue) -> ZclValue {
        process_read(attribute_fields(attribute), value)
    }

    /// Upstream `Utils.processParameterWrite(parameter, value)`.
    pub fn process_parameter_write(parameter: &Parameter, value: ZclValue) -> ZclValue {
        process_write(parameter_fields(parameter), value)
    }

    /// Upstream `Utils.processParameterRead(parameter, value)`.
    pub fn process_parameter_read(parameter: &Parameter, value: ZclValue) -> ZclValue {
        process_read(parameter_fields(parameter), value)
    }
}

struct EntryFields<'a> {
    name: &'a str,
    ID: u16,
    r#type: DataType,
    default: Option<&'a ZclValue>,
    min: Option<f64>,
    max: Option<f64>,
    minExcl: Option<f64>,
    maxExcl: Option<f64>,
    length: Option<usize>,
    minLen: Option<usize>,
    maxLen: Option<usize>,
    special: &'a [ZclValue],
}

fn attribute_fields(attribute: &Attribute) -> EntryFields<'_> {
    EntryFields {
        name: &attribute.name,
        ID: attribute.ID,
        r#type: attribute.r#type,
        default: attribute.default.as_ref(),
        min: attribute.min,
        max: attribute.max,
        minExcl: attribute.minExcl,
        maxExcl: attribute.maxExcl,
        length: attribute.length,
        minLen: attribute.minLen,
        maxLen: attribute.maxLen,
        special: &attribute.special,
    }
}

fn parameter_fields(parameter: &Parameter) -> EntryFields<'_> {
    EntryFields {
        name: &parameter.name,
        ID: 0,
        r#type: parameter.r#type,
        default: None,
        min: parameter.min,
        max: parameter.max,
        minExcl: parameter.minExcl,
        maxExcl: parameter.maxExcl,
        length: parameter.length,
        minLen: parameter.minLen,
        maxLen: parameter.maxLen,
        special: &parameter.special,
    }
}

fn process_write(entry: EntryFields<'_>, value: ZclValue) -> ZclValue {
    if value == ZclValue::Null {
        return entry.default.cloned().unwrap_or(ZclValue::Null);
    }

    if let Some(default) = entry.default {
        if *default == value {
            return value;
        }
    }

    if value.is_nan() {
        if let Some(default) = entry.default {
            return default.clone();
        }

        return match invalid_value(entry.r#type) {
            Some(non_value) => non_value,
            None => panic!(
                "{} ({}) does not have a default nor a non-value",
                entry.name, entry.ID
            ),
        };
    }

    if entry.special.contains(&value) {
        return value;
    }

    process_restrictions(&entry, &value);

    value
}

fn process_read(entry: EntryFields<'_>, value: ZclValue) -> ZclValue {
    if value == ZclValue::Null {
        return value;
    }

    if let Some(default) = entry.default {
        if *default == value {
            return value;
        }
    }

    if entry.special.contains(&value) {
        return value;
    }

    if invalid_value(entry.r#type).as_ref() == Some(&value) {
        if let ZclValue::Number(number) = value {
            if Some(number) == entry.min || Some(number) == entry.max {
                return value;
            }
        }

        return ZclValue::Number(f64::NAN);
    }

    process_restrictions(&entry, &value);

    value
}

fn process_restrictions(entry: &EntryFields<'_>, value: &ZclValue) {
    let number = match value {
        ZclValue::Number(number) => Some(*number),
        _ => None,
    };

    if let (Some(min), Some(number)) = (entry.min, number) {
        if number < min {
            panic!("{} requires min of {min}", entry.name);
        }
    }

    if let (Some(min_excl), Some(number)) = (entry.minExcl, number) {
        if number <= min_excl {
            panic!("{} requires min exclusive of {min_excl}", entry.name);
        }
    }

    if let (Some(max), Some(number)) = (entry.max, number) {
        if number > max {
            panic!("{} requires max of {max}", entry.name);
        }
    }

    if let (Some(max_excl), Some(number)) = (entry.maxExcl, number) {
        if number >= max_excl {
            panic!("{} requires max exclusive of {max_excl}", entry.name);
        }
    }

    if let Some(length) = entry.length {
        if !value.is_len(length) {
            panic!("{} requires length of {length}", entry.name);
        }
    }

    if let Some(min_len) = entry.minLen {
        if value.len() < min_len {
            panic!("{} requires min length of {min_len}", entry.name);
        }
    }

    if let Some(max_len) = entry.maxLen {
        if value.len() > max_len {
            panic!("{} requires max length of {max_len}", entry.name);
        }
    }
}

/// Upstream `ZCL_TYPE_INVALID_BY_TYPE`.
fn invalid_value(r#type: DataType) -> Option<ZclValue> {
    match r#type {
        DataType::BOOLEAN | DataType::UINT8 | DataType::ENUM8 => Some(ZclValue::num(255)),
        DataType::UINT16 | DataType::ENUM16 | DataType::CLUSTER_ID | DataType::ATTR_ID => {
            Some(ZclValue::num(65535))
        }
        DataType::INT8 => Some(ZclValue::num(-128)),
        DataType::INT16 => Some(ZclValue::num(-32768)),
        DataType::UINT32 => Some(ZclValue::num(4294967295u32)),
        DataType::INT32 => Some(ZclValue::num(-2147483648i32)),
        _ => None,
    }
}

fn builtin_cluster_by_name(name: &str) -> Option<Cluster> {
    catalog().by_name.get(name).cloned()
}

fn builtin_cluster_by_id(id: u32) -> Cluster {
    if let Some(cluster) = catalog().by_name.get(&builtin_name(id)) {
        return cluster.clone();
    }
    Cluster::new(&id.to_string(), id)
}

fn builtin_name(id: u32) -> String {
    catalog()
        .by_id
        .get(&id)
        .cloned()
        .unwrap_or_else(|| id.to_string())
}

struct ClusterCatalog {
    by_name: HashMap<String, Cluster>,
    by_id: HashMap<u32, String>,
}

static CATALOG: OnceLock<ClusterCatalog> = OnceLock::new();

fn catalog() -> &'static ClusterCatalog {
    CATALOG.get_or_init(|| {
        let root: serde_json::Value =
            serde_json::from_str(include_str!("definition/clusters.json"))
                .expect("generated ZCL cluster catalog must be valid JSON");
        let mut by_name = HashMap::new();
        let mut by_id = HashMap::new();
        for (name, value) in root.as_object().expect("cluster catalog must be an object") {
            let cluster = parse_cluster(name, value);
            by_id.insert(cluster.ID, name.clone());
            by_name.insert(name.clone(), cluster);
        }
        ClusterCatalog { by_name, by_id }
    })
}

fn parse_cluster(name: &str, value: &serde_json::Value) -> Cluster {
    let mut cluster = Cluster::new(name, value["ID"].as_u64().unwrap_or(0) as u32);
    cluster.manufacturerCode = value
        .get("manufacturerCode")
        .and_then(serde_json::Value::as_u64)
        .map(|code| code as u16);
    if let Some(attributes) = value.get("attributes").and_then(|v| v.as_object()) {
        for (attribute_name, attribute_value) in attributes {
            cluster.attributes.insert(
                attribute_name.clone(),
                parse_attribute(attribute_name, attribute_value),
            );
        }
    }
    if let Some(commands) = value.get("commands").and_then(|v| v.as_object()) {
        for (command_name, command_value) in commands {
            cluster.commands.insert(
                command_name.clone(),
                parse_command(command_name, command_value),
            );
        }
    }
    if let Some(commands) = value.get("commandsResponse").and_then(|v| v.as_object()) {
        for (command_name, command_value) in commands {
            cluster.commands_response_mut().insert(
                command_name.clone(),
                parse_command(command_name, command_value),
            );
        }
    }
    cluster
}

fn parse_attribute(name: &str, value: &serde_json::Value) -> Attribute {
    let (data_type, _) = parse_parameter_type(value.get("type"));
    let mut attribute = Attribute::new(name, value["ID"].as_u64().unwrap_or(0) as u16, data_type);
    attribute.write = value
        .get("write")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    attribute.read = value
        .get("read")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    attribute.required = value
        .get("required")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    attribute.default = value.get("default").map(ZclValue::from_json);
    attribute.min = value.get("min").and_then(serde_json::Value::as_f64);
    attribute.max = value.get("max").and_then(serde_json::Value::as_f64);
    attribute.minExcl = value.get("minExcl").and_then(serde_json::Value::as_f64);
    attribute.maxExcl = value.get("maxExcl").and_then(serde_json::Value::as_f64);
    attribute.length = value
        .get("length")
        .and_then(serde_json::Value::as_u64)
        .map(|length| length as usize);
    attribute.minLen = value
        .get("minLen")
        .and_then(serde_json::Value::as_u64)
        .map(|length| length as usize);
    attribute.maxLen = value
        .get("maxLen")
        .and_then(serde_json::Value::as_u64)
        .map(|length| length as usize);
    attribute.special = parse_special(value.get("special"));
    attribute
}

fn parse_command(name: &str, value: &serde_json::Value) -> Command {
    let mut command = Command::new(name, value["ID"].as_u64().unwrap_or(0) as u8);
    command.response = value
        .get("response")
        .and_then(serde_json::Value::as_u64)
        .map(|response| response as u8);
    command.required = value
        .get("required")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if let Some(parameters) = value.get("parameters").and_then(|v| v.as_array()) {
        command.parameters = parameters.iter().map(parse_parameter).collect();
    }
    command
}

fn parse_parameter(value: &serde_json::Value) -> Parameter {
    let name = value["name"].as_str().unwrap_or_default();
    let (data_type, buffalo_type) = parse_parameter_type(value.get("type"));
    let mut parameter = Parameter::new(name, data_type);
    parameter.buffalo_type = buffalo_type;
    parameter.min = value.get("min").and_then(serde_json::Value::as_f64);
    parameter.max = value.get("max").and_then(serde_json::Value::as_f64);
    parameter.minExcl = value.get("minExcl").and_then(serde_json::Value::as_f64);
    parameter.maxExcl = value.get("maxExcl").and_then(serde_json::Value::as_f64);
    parameter.length = value
        .get("length")
        .and_then(serde_json::Value::as_u64)
        .map(|length| length as usize);
    parameter.minLen = value
        .get("minLen")
        .and_then(serde_json::Value::as_u64)
        .map(|length| length as usize);
    parameter.maxLen = value
        .get("maxLen")
        .and_then(serde_json::Value::as_u64)
        .map(|length| length as usize);
    parameter.special = parse_special(value.get("special"));
    if let Some(conditions) = value.get("conditions").and_then(|v| v.as_array()) {
        parameter.conditions = conditions.iter().filter_map(parse_condition).collect();
    }
    parameter
}

fn parse_parameter_type(
    value: Option<&serde_json::Value>,
) -> (DataType, Option<BuffaloZclDataType>) {
    let code = value
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(DataType::UNKNOWN as u64) as u32;
    if let Some(buffalo_type) = buffalo_type_from_u32(code) {
        (DataType::UNKNOWN, Some(buffalo_type))
    } else {
        (
            DataType::from_u8(code as u8).unwrap_or(DataType::UNKNOWN),
            None,
        )
    }
}

fn buffalo_type_from_u32(value: u32) -> Option<BuffaloZclDataType> {
    Some(match value {
        1000 => BuffaloZclDataType::USE_DATA_TYPE,
        1001 => BuffaloZclDataType::LIST_UINT8,
        1002 => BuffaloZclDataType::LIST_UINT16,
        1003 => BuffaloZclDataType::LIST_UINT24,
        1004 => BuffaloZclDataType::LIST_UINT32,
        1005 => BuffaloZclDataType::LIST_ZONEINFO,
        1006 => BuffaloZclDataType::EXTENSION_FIELD_SETS,
        1007 => BuffaloZclDataType::LIST_THERMO_TRANSITIONS,
        1008 => BuffaloZclDataType::BUFFER,
        1009 => BuffaloZclDataType::GPD_FRAME,
        1010 => BuffaloZclDataType::STRUCTURED_SELECTOR,
        1011 => BuffaloZclDataType::LIST_TUYA_DATAPOINT_VALUES,
        1012 => BuffaloZclDataType::LIST_MIBOXER_ZONES,
        1013 => BuffaloZclDataType::BIG_ENDIAN_UINT24,
        1014 => BuffaloZclDataType::MI_STRUCT,
        _ => return None,
    })
}

fn parse_special(value: Option<&serde_json::Value>) -> Vec<ZclValue> {
    value
        .and_then(|value| value.as_object())
        .map(|object| {
            object
                .keys()
                .map(|key| {
                    if let Some(hex) = key.strip_prefix("0x") {
                        if hex.len() > 10 {
                            return ZclValue::Text(key.clone());
                        }
                        if let Ok(number) = u64::from_str_radix(hex, 16) {
                            return ZclValue::Number(number as f64);
                        }
                    }
                    if let Ok(number) = key.parse::<f64>() {
                        ZclValue::Number(number)
                    } else {
                        ZclValue::Text(key.clone())
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_condition(value: &serde_json::Value) -> Option<ParameterCondition> {
    Some(match value["type"].as_str()? {
        "minimumRemainingBufferBytes" => ParameterCondition::MINIMUM_REMAINING_BUFFER_BYTES(
            value["value"].as_u64().unwrap_or(0) as u32,
        ),
        "bitMaskSet" => ParameterCondition::BITMASK_SET {
            field: value["param"].as_str().unwrap_or_default().to_string(),
            value: value["mask"].as_u64().unwrap_or(0) as u32,
            reversed: value
                .get("reversed")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
        },
        "bitFieldEnum" => ParameterCondition::BITFIELD_ENUM {
            field: value["param"].as_str().unwrap_or_default().to_string(),
            offset: value["offset"].as_u64().unwrap_or(0) as u8,
            size: value["size"].as_u64().unwrap_or(0) as u8,
            value: value["value"].as_u64().unwrap_or(0) as u32,
        },
        "fieldEquals" => ParameterCondition::FIELD_EQUAL {
            field: value["field"].as_str().unwrap_or_default().to_string(),
            value: value["value"].as_u64().unwrap_or(0) as u32,
            reversed: value
                .get("reversed")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
        },
        "fieldGT" => ParameterCondition::FIELD_GT {
            field: value["field"].as_str().unwrap_or_default().to_string(),
            value: value["value"].as_u64().unwrap_or(0) as u32,
        },
        _ => return None,
    })
}
