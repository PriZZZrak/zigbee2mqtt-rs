// Original: .upstream/zigbee-herdsman/src/zspec/zcl/utils.ts
//
// The types below mirror upstream names from
// `definition/tstype.ts` (`Cluster`, `Attribute`, `Parameter`,
// `CustomClusters`) and only cover what the ported tests exercise.

use std::collections::HashMap;

use crate::zspec::zcl::definition::enums::DataType;

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
            Static::BITMASK_SET { field, value } => Self::BITMASK_SET {
                field: (*field).to_string(),
                value: *value,
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
}

impl From<&crate::zspec::zcl::definition::cluster::Parameter> for Parameter {
    fn from(parameter: &crate::zspec::zcl::definition::cluster::Parameter) -> Self {
        Self {
            name: parameter.name.to_string(),
            r#type: parameter.r#type,
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

fn builtin_cluster_by_id(id: u32) -> Cluster {
    if let Some(cluster) = builtin_cluster_by_name(&builtin_name(id)) {
        return cluster;
    }

    Cluster::new(&id.to_string(), id)
}

fn builtin_name(id: u32) -> String {
    match id {
        0x0000 => "genBasic",
        0x0002 => "genDeviceTempCfg",
        0x0003 => "genIdentify",
        0x0005 => "genScenes",
        0x0006 => "genOnOff",
        0x0008 => "genLevelCtrl",
        0x0004 => "genGroups",
        0x0009 => "genAlarms",
        0x000a => "genTime",
        0x0019 => "genOta",
        0x0020 => "genPollCtrl",
        0x0021 => "greenPower",
        0x0500 => "ssIasZone",
        0x1000 => "touchlink",
        0x000b => "genRssiLocation",
        0x0300 => "lightingColorCtrl",
        0x0b05 => "haDiagnostic",
        0xfc57 => "manuSpecificAmazonWWAH",
        _ => return id.to_string(),
    }
    .to_string()
}

/// Upstream `Clusters.genBasic.attributes` — definition/cluster.ts:2.
fn gen_basic_attributes() -> Vec<Attribute> {
    vec![
        Attribute {
            max: Some(0xff.into()),
            default: Some(ZclValue::num(8)),
            ..Attribute::new("zclVersion", 0x0000, DataType::UINT8)
        },
        Attribute {
            max: Some(0xff.into()),
            default: Some(ZclValue::num(0)),
            ..Attribute::new("appVersion", 0x0001, DataType::UINT8)
        },
        Attribute {
            max: Some(0xff.into()),
            default: Some(ZclValue::num(0)),
            ..Attribute::new("stackVersion", 0x0002, DataType::UINT8)
        },
        Attribute {
            max: Some(0xff.into()),
            default: Some(ZclValue::num(0)),
            ..Attribute::new("hwVersion", 0x0003, DataType::UINT8)
        },
        Attribute {
            maxLen: Some(32),
            default: Some(ZclValue::text("")),
            ..Attribute::new("manufacturerName", 0x0004, DataType::CHAR_STR)
        },
        Attribute {
            maxLen: Some(32),
            default: Some(ZclValue::text("")),
            ..Attribute::new("modelId", 0x0005, DataType::CHAR_STR)
        },
        Attribute {
            maxLen: Some(16),
            default: Some(ZclValue::text("")),
            ..Attribute::new("dateCode", 0x0006, DataType::CHAR_STR)
        },
        Attribute {
            default: Some(ZclValue::num(0xff)),
            ..Attribute::new("powerSource", 0x0007, DataType::ENUM8)
        },
        Attribute {
            default: Some(ZclValue::num(0xff)),
            ..Attribute::new("genericDeviceClass", 0x0008, DataType::ENUM8)
        },
        Attribute {
            default: Some(ZclValue::num(0xff)),
            ..Attribute::new("genericDeviceType", 0x0009, DataType::ENUM8)
        },
        Attribute {
            default: Some(ZclValue::text("")),
            ..Attribute::new("productCode", 0x000a, DataType::OCTET_STR)
        },
        Attribute {
            default: Some(ZclValue::text("")),
            ..Attribute::new("productUrl", 0x000b, DataType::CHAR_STR)
        },
        Attribute {
            default: Some(ZclValue::text("")),
            ..Attribute::new("manufacturerVersionDetails", 0x000c, DataType::CHAR_STR)
        },
        Attribute {
            default: Some(ZclValue::text("")),
            ..Attribute::new("serialNumber", 0x000d, DataType::CHAR_STR)
        },
        Attribute {
            default: Some(ZclValue::text("")),
            ..Attribute::new("productLabel", 0x000e, DataType::CHAR_STR)
        },
        Attribute {
            maxLen: Some(16),
            default: Some(ZclValue::text("")),
            ..Attribute::new("locationDesc", 0x0010, DataType::CHAR_STR).writable()
        },
        Attribute {
            default: Some(ZclValue::num(0)),
            ..Attribute::new("physicalEnv", 0x0011, DataType::ENUM8).writable()
        },
        Attribute {
            default: Some(ZclValue::num(1)),
            ..Attribute::new("deviceEnabled", 0x0012, DataType::BOOLEAN).writable()
        },
        Attribute {
            default: Some(ZclValue::num(0)),
            ..Attribute::new("alarmMask", 0x0013, DataType::BITMAP8).writable()
        },
        Attribute {
            default: Some(ZclValue::num(0)),
            ..Attribute::new("disableLocalConfig", 0x0014, DataType::BITMAP8).writable()
        },
        Attribute {
            maxLen: Some(16),
            default: Some(ZclValue::text("")),
            ..Attribute::new("swBuildId", 0x4000, DataType::CHAR_STR)
        },
    ]
}

fn builtin_cluster_by_name(name: &str) -> Option<Cluster> {
    match name {
        "genBasic" => {
            let mut cluster = Cluster::new("genBasic", 0x0000);
            for attribute in gen_basic_attributes() {
                cluster = cluster.with_attribute(attribute);
            }
            Some(cluster.with_command(Command::new("resetFactDefault", 0x00)))
        }
        "genDeviceTempCfg" => Some(Cluster::new("genDeviceTempCfg", 0x0002).with_attribute(
            Attribute {
                min: Some(-200.0),
                max: Some(200.0),
                ..Attribute::new("currentTemperature", 0x0000, DataType::INT16)
            },
        )),
        "genIdentify" => Some(
            Cluster::new("genIdentify", 0x0003).with_command_response(
                Command::new("identifyQueryRsp", 0x00)
                    .with_parameters(vec![Parameter::new("timeout", DataType::UINT16)]),
            ),
        ),
        "genOnOff" => Some(
            Cluster::new("genOnOff", 0x0006)
                .with_attribute(
                    Attribute {
                        default: Some(ZclValue::num(0)),
                        ..Attribute::new("onOff", 0x0000, DataType::BOOLEAN)
                    }
                    .writable(),
                )
                .with_attribute(Attribute {
                    default: Some(ZclValue::num(1)),
                    ..Attribute::new("globalSceneCtrl", 0x4000, DataType::BOOLEAN)
                })
                .with_attribute(Attribute {
                    max: Some(0xffff.into()),
                    default: Some(ZclValue::num(0)),
                    ..Attribute::new("onTime", 0x4001, DataType::UINT16).writable()
                })
                .with_attribute(Attribute {
                    max: Some(0xffff.into()),
                    default: Some(ZclValue::num(0)),
                    ..Attribute::new("offWaitTime", 0x4002, DataType::UINT16).writable()
                })
                .with_attribute(Attribute {
                    max: Some(0xff.into()),
                    special: vec![ZclValue::num(255)],
                    ..Attribute::new("startUpOnOff", 0x4003, DataType::ENUM8).writable()
                })
                .with_command(Command::new("off", 0x00))
                .with_command(Command::new("on", 0x01))
                .with_command(Command::new("toggle", 0x02)),
        ),
        "genScenes" => Some(
            Cluster::new("genScenes", 0x0005)
                .with_attribute(Attribute {
                    special: vec![ZclValue::text("0xffffffffffffffff")],
                    ..Attribute::new("lastCfgBy", 0x0005, DataType::IEEE_ADDR)
                })
                .with_command_response(
                    Command::new("getSceneMembershipRsp", 0x06).with_parameters(vec![
                        Parameter::new("status", DataType::ENUM8),
                        Parameter {
                            min: Some(0.0),
                            max: Some(255.0),
                            special: vec![ZclValue::num(254), ZclValue::num(255)],
                            ..Parameter::new("capacity", DataType::UINT8)
                        },
                    ]),
                ),
        ),
        "genLevelCtrl" => Some(
            Cluster::new("genLevelCtrl", 0x0008)
                .with_attribute(Attribute {
                    default: Some(ZclValue::num(255)),
                    ..Attribute::new("currentLevel", 0x0000, DataType::UINT8)
                })
                .with_attribute(
                    Attribute::new("options", 0x000f, DataType::BITMAP8)
                        .writable()
                        .with_default(0),
                )
                .with_command(
                    Command::new("moveToLevelWithOnOff", 0x04).with_parameters(vec![
                        Parameter::new("level", DataType::UINT8),
                        Parameter::new("transtime", DataType::UINT16),
                        Parameter::new("optionsMask", DataType::BITMAP8).with_conditions(vec![
                            ParameterCondition::MINIMUM_REMAINING_BUFFER_BYTES(2),
                        ]),
                        Parameter::new("optionsOverride", DataType::BITMAP8).with_conditions(vec![
                            ParameterCondition::MINIMUM_REMAINING_BUFFER_BYTES(2),
                        ]),
                    ]),
                ),
        ),
        "genAlarms" => Some(
            Cluster::new("genAlarms", 0x0009)
                .with_attribute(Attribute {
                    max: Some(65535.0),
                    default: Some(ZclValue::num(0)),
                    ..Attribute::new("alarmCount", 0x0000, DataType::UINT16)
                })
                .with_command(Command::new("reset", 0x00).with_parameters(vec![
                    Parameter::new("alarmcode", DataType::ENUM8),
                    Parameter::new("clusterid", DataType::UINT16),
                ]))
                .with_command(Command::new("resetAll", 0x01))
                .with_command(Command::new("getAlarm", 0x02))
                .with_command(Command::new("resetLog", 0x03))
                .with_command_response(Command::new("alarm", 0x00).with_parameters(vec![
                    Parameter::new("alarmcode", DataType::ENUM8),
                    Parameter::new("clusterid", DataType::CLUSTER_ID),
                ]))
                .with_command_response(Command::new("getRsp", 0x01).with_parameters(vec![
                    Parameter::new("status", DataType::ENUM8),
                    Parameter::new("alarmcode", DataType::ENUM8),
                ])),
        ),
        "genRssiLocation" => Some(
            Cluster::new("genRssiLocation", 0x000b)
                .with_attribute(
                    Attribute {
                        min: Some(-32768.0),
                        max: Some(32767.0),
                        ..Attribute::new("coordinate1", 0x0010, DataType::INT16)
                    }
                    .writable(),
                )
                .with_attribute(
                    Attribute::new("pathLossExponent", 0x0014, DataType::UINT16).writable(),
                )
                .with_command(Command::new("setAbsolute", 0x00).with_parameters(vec![
                    Parameter::new("coordinate1", DataType::INT16),
                    Parameter::new("coordinate2", DataType::INT16),
                    Parameter::new("coordinate3", DataType::INT16),
                    Parameter::new("power", DataType::INT16),
                    Parameter::new("pathLossExponent", DataType::UINT16),
                ]))
                .with_command_response(
                    Command::new("locationDataNotification", 0x02).with_parameters(vec![
                        Parameter::new("type", DataType::DATA8),
                        Parameter::new("coordinate1", DataType::INT16),
                        Parameter::new("coordinate2", DataType::INT16),
                        Parameter::new("coordinate3", DataType::INT16),
                        Parameter::new("power", DataType::INT16),
                        Parameter::new("pathLossExponent", DataType::UINT16),
                    ]),
                ),
        ),
        "lightingColorCtrl" => Some(Cluster::new("lightingColorCtrl", 0x0300).with_attribute(
            Attribute {
                max: Some(0xfeff as f64),
                special: vec![ZclValue::num(65535)],
                ..Attribute::new("startUpColorTemperature", 0x4010, DataType::UINT16).writable()
            },
        )),
        "haDiagnostic" => Some(Cluster::new("haDiagnostic", 0x0b05)),
        "manuSpecificAmazonWWAH" => Some(Cluster {
            manufacturerCode: Some(manufacturer_code::AMAZON_LAB126),
            ..Cluster::new("manuSpecificAmazonWWAH", 0xfc57)
        }),
        "genGroups" => Some(
            Cluster::new("genGroups", 0x0004)
                .with_attribute(Attribute {
                    default: Some(ZclValue::num(0)),
                    ..Attribute::new("nameSupport", 0x0000, DataType::BITMAP8)
                })
                .with_command(
                    Command::new("add", 0x00)
                        .with_response(0x00)
                        .with_parameters(vec![
                            Parameter::new("groupid", DataType::UINT16),
                            Parameter::new("groupname", DataType::CHAR_STR),
                        ]),
                )
                .with_command(
                    Command::new("view", 0x01)
                        .with_response(0x01)
                        .with_parameters(vec![Parameter::new("groupid", DataType::UINT16)]),
                )
                .with_command(
                    Command::new("getMembership", 0x02)
                        .with_response(2)
                        .with_parameters(vec![
                            Parameter::new("groupcount", DataType::UINT8),
                            Parameter::new("grouplist", DataType::UNKNOWN),
                        ]),
                )
                .with_command(
                    Command::new("remove", 0x03)
                        .with_response(0x03)
                        .with_parameters(vec![Parameter::new("groupid", DataType::UINT16)]),
                )
                .with_command(Command::new("removeAll", 0x04))
                .with_command(Command::new("addIfIdentifying", 0x05).with_parameters(vec![
                    Parameter::new("groupid", DataType::UINT16),
                    Parameter::new("groupname", DataType::CHAR_STR),
                ]))
                .with_command_response(Command::new("addRsp", 0x00).with_parameters(vec![
                    Parameter::new("status", DataType::ENUM8),
                    Parameter::new("groupid", DataType::UINT16),
                ]))
                .with_command_response(Command::new("viewRsp", 0x01).with_parameters(vec![
                    Parameter::new("status", DataType::ENUM8),
                    Parameter::new("groupid", DataType::UINT16),
                    Parameter::new("groupname", DataType::CHAR_STR),
                ]))
                .with_command_response(Command::new("getMembershipRsp", 0x02).with_parameters(
                    vec![
                        Parameter::new("capacity", DataType::UINT8),
                        Parameter::new("groupcount", DataType::UINT8),
                        Parameter::new("grouplist", DataType::UNKNOWN),
                    ],
                ))
                .with_command_response(Command::new("removeRsp", 0x03).with_parameters(vec![
                    Parameter::new("status", DataType::ENUM8),
                    Parameter::new("groupid", DataType::UINT16),
                ])),
        ),
        "genTime" => Some(
            Cluster::new("genTime", 0x000a)
                .with_attribute(Attribute {
                    max: Some(0xfffffffeu32 as f64),
                    default: Some(ZclValue::num(0xffffffffu32)),
                    ..Attribute::new("time", 0x0000, DataType::UTC).writable()
                })
                .with_attribute(Attribute {
                    default: Some(ZclValue::num(0)),
                    ..Attribute::new("timeStatus", 0x0001, DataType::BITMAP8).writable()
                })
                .with_attribute(Attribute {
                    min: Some(-86400.0),
                    max: Some(86400.0),
                    default: Some(ZclValue::num(0)),
                    ..Attribute::new("timeZone", 0x0002, DataType::INT32).writable()
                })
                .with_attribute(Attribute {
                    max: Some(0xfffffffeu32 as f64),
                    default: Some(ZclValue::num(0xffffffffu32)),
                    ..Attribute::new("dstStart", 0x0003, DataType::UINT32).writable()
                })
                .with_attribute(Attribute {
                    max: Some(0xfffffffeu32 as f64),
                    default: Some(ZclValue::num(0xffffffffu32)),
                    ..Attribute::new("dstEnd", 0x0004, DataType::UINT32).writable()
                })
                .with_attribute(Attribute {
                    min: Some(-86400.0),
                    max: Some(86400.0),
                    default: Some(ZclValue::num(0)),
                    ..Attribute::new("dstShift", 0x0005, DataType::INT32).writable()
                })
                .with_attribute(Attribute {
                    max: Some(0xfffffffeu32 as f64),
                    default: Some(ZclValue::num(0xffffffffu32)),
                    ..Attribute::new("standardTime", 0x0006, DataType::UINT32)
                })
                .with_attribute(Attribute {
                    max: Some(0xfffffffeu32 as f64),
                    default: Some(ZclValue::num(0xffffffffu32)),
                    ..Attribute::new("localTime", 0x0007, DataType::UINT32)
                })
                .with_attribute(Attribute {
                    default: Some(ZclValue::num(0xffffffffu32)),
                    ..Attribute::new("lastSetTime", 0x0008, DataType::UTC)
                })
                .with_attribute(Attribute {
                    default: Some(ZclValue::num(0xffffffffu32)),
                    ..Attribute::new("validUntilTime", 0x0009, DataType::UTC).writable()
                }),
        ),
        "ssIasZone" => Some(
            Cluster::new("ssIasZone", 0x0500)
                .with_attribute(Attribute {
                    default: Some(ZclValue::num(0)),
                    ..Attribute::new("zoneState", 0x0000, DataType::ENUM8)
                })
                .with_attribute(Attribute::new("zoneType", 0x0001, DataType::ENUM16))
                .with_attribute(Attribute {
                    default: Some(ZclValue::num(0)),
                    ..Attribute::new("zoneStatus", 0x0002, DataType::BITMAP16)
                })
                .with_attribute(
                    Attribute::new("iasCieAddr", 0x0010, DataType::IEEE_ADDR).writable(),
                )
                .with_attribute(Attribute {
                    max: Some(0xff.into()),
                    default: Some(ZclValue::num(0xff)),
                    ..Attribute::new("zoneId", 0x0011, DataType::UINT8)
                })
                .with_attribute(Attribute {
                    min: Some(2.0),
                    max: Some(0xff.into()),
                    default: Some(ZclValue::num(2)),
                    ..Attribute::new("numZoneSensitivityLevelsSupported", 0x0012, DataType::UINT8)
                })
                .with_attribute(Attribute {
                    max: Some(0xff.into()),
                    default: Some(ZclValue::num(0)),
                    ..Attribute::new("currentZoneSensitivityLevel", 0x0013, DataType::UINT8)
                        .writable()
                })
                .with_command(Command::new("enrollRsp", 0x00).with_parameters(vec![
                    Parameter::new("enrollrspcode", DataType::ENUM8),
                    Parameter::new("zoneid", DataType::UINT8),
                ]))
                .with_command(Command::new("initNormalOpMode", 0x01))
                .with_command(Command::new("initTestMode", 0x02).with_parameters(vec![
                    Parameter::new("testModeDuration", DataType::UINT8),
                    Parameter::new("currentZoneSensitivityLevel", DataType::UINT8),
                ]))
                .with_command_response(
                    Command::new("statusChangeNotification", 0x00).with_parameters(vec![
                        Parameter::new("zonestatus", DataType::BITMAP16),
                        Parameter::new("extendedstatus", DataType::BITMAP8),
                        Parameter::new("zoneID", DataType::UINT8),
                        Parameter::new("delay", DataType::UINT16),
                    ]),
                )
                .with_command_response(Command::new("enrollReq", 0x01).with_parameters(vec![
                    Parameter::new("zonetype", DataType::ENUM16),
                    Parameter::new("manucode", DataType::UINT16),
                ])),
        ),
        "genPollCtrl" => Some(
            Cluster::new("genPollCtrl", 0x0020)
                .with_attribute(Attribute {
                    max: Some(7208960.0),
                    default: Some(ZclValue::num(14400)),
                    ..Attribute::new("checkinInterval", 0x0000, DataType::UINT32).writable()
                })
                .with_attribute(Attribute {
                    min: Some(4.0),
                    max: Some(7208960.0),
                    default: Some(ZclValue::num(20)),
                    ..Attribute::new("longPollInterval", 0x0001, DataType::UINT32)
                })
                .with_attribute(Attribute {
                    min: Some(1.0),
                    max: Some(0xffff.into()),
                    default: Some(ZclValue::num(2)),
                    ..Attribute::new("shortPollInterval", 0x0002, DataType::UINT16)
                })
                .with_attribute(Attribute {
                    min: Some(1.0),
                    max: Some(0xffff.into()),
                    default: Some(ZclValue::num(40)),
                    ..Attribute::new("fastPollTimeout", 0x0003, DataType::UINT16).writable()
                })
                .with_attribute(Attribute {
                    default: Some(ZclValue::num(0)),
                    ..Attribute::new("checkinIntervalMin", 0x0004, DataType::UINT32)
                })
                .with_attribute(Attribute {
                    default: Some(ZclValue::num(0)),
                    ..Attribute::new("longPollIntervalMin", 0x0005, DataType::UINT32)
                })
                .with_attribute(Attribute {
                    default: Some(ZclValue::num(0)),
                    ..Attribute::new("fastPollTimeoutMax", 0x0006, DataType::UINT16)
                })
                .with_command(Command::new("checkinRsp", 0x00).with_parameters(vec![
                    Parameter::new("startFastPolling", DataType::BOOLEAN),
                    Parameter::new("fastPollTimeout", DataType::UINT16),
                ]))
                .with_command(Command::new("fastPollStop", 0x01))
                .with_command(
                    Command::new("setLongPollInterval", 0x02).with_parameters(vec![
                        Parameter::new("newLongPollInterval", DataType::UINT32),
                    ]),
                )
                .with_command(
                    Command::new("setShortPollInterval", 0x03).with_parameters(vec![
                        Parameter::new("newShortPollInterval", DataType::UINT16),
                    ]),
                )
                .with_command_response(Command::new("checkin", 0x00)),
        ),
        "greenPower" => Some(
            // Only the `pairing` command used by `Device.removeFromNetwork` for
            // Green Power devices is defined; the rest of the cluster is not
            // needed by the R1 surface (`controller/greenPower.ts` is BLOCKED).
            Cluster::new("greenPower", 0x0021).with_command(
                Command::new("pairing", 0x01).with_parameters(vec![
                    Parameter::new("options", DataType::BITMAP24),
                    Parameter::new("srcID", DataType::UINT32).with_conditions(vec![
                        ParameterCondition::BITFIELD_ENUM {
                            field: "options".to_string(),
                            offset: 0,
                            size: 3,
                            value: 0b000,
                        },
                    ]),
                    Parameter::new("gpdIEEEAddr", DataType::IEEE_ADDR).with_conditions(vec![
                        ParameterCondition::BITFIELD_ENUM {
                            field: "options".to_string(),
                            offset: 0,
                            size: 3,
                            value: 0b010,
                        },
                    ]),
                    Parameter::new("gpdEndpoint", DataType::UINT8).with_conditions(vec![
                        ParameterCondition::BITFIELD_ENUM {
                            field: "options".to_string(),
                            offset: 0,
                            size: 3,
                            value: 0b010,
                        },
                    ]),
                    Parameter::new("sinkIEEEAddr", DataType::IEEE_ADDR).with_conditions(vec![
                        ParameterCondition::BITFIELD_ENUM {
                            field: "options".to_string(),
                            offset: 4,
                            size: 3,
                            value: 0b110,
                        },
                    ]),
                    Parameter::new("sinkIEEEAddr", DataType::IEEE_ADDR).with_conditions(vec![
                        ParameterCondition::BITFIELD_ENUM {
                            field: "options".to_string(),
                            offset: 4,
                            size: 3,
                            value: 0b000,
                        },
                    ]),
                    Parameter::new("sinkNwkAddr", DataType::UINT16).with_conditions(vec![
                        ParameterCondition::BITFIELD_ENUM {
                            field: "options".to_string(),
                            offset: 4,
                            size: 3,
                            value: 0b110,
                        },
                    ]),
                    Parameter::new("sinkNwkAddr", DataType::UINT16).with_conditions(vec![
                        ParameterCondition::BITFIELD_ENUM {
                            field: "options".to_string(),
                            offset: 4,
                            size: 3,
                            value: 0b000,
                        },
                    ]),
                    Parameter::new("sinkGroupID", DataType::UINT16).with_conditions(vec![
                        ParameterCondition::BITFIELD_ENUM {
                            field: "options".to_string(),
                            offset: 4,
                            size: 3,
                            value: 0b100,
                        },
                    ]),
                    Parameter::new("sinkGroupID", DataType::UINT16).with_conditions(vec![
                        ParameterCondition::BITFIELD_ENUM {
                            field: "options".to_string(),
                            offset: 4,
                            size: 3,
                            value: 0b010,
                        },
                    ]),
                    Parameter::new("deviceID", DataType::UINT8).with_conditions(vec![
                        ParameterCondition::BITMASK_SET {
                            field: "options".to_string(),
                            value: 0x0008,
                        },
                    ]),
                    Parameter::new("frameCounter", DataType::UINT32).with_conditions(vec![
                        ParameterCondition::BITMASK_SET {
                            field: "options".to_string(),
                            value: 0x4000,
                        },
                    ]),
                    Parameter::new("gpdKey", DataType::SEC_KEY).with_conditions(vec![
                        ParameterCondition::BITMASK_SET {
                            field: "options".to_string(),
                            value: 0x8000,
                        },
                    ]),
                    Parameter::new("assignedAlias", DataType::UINT16).with_conditions(vec![
                        ParameterCondition::BITMASK_SET {
                            field: "options".to_string(),
                            value: 0x10000,
                        },
                    ]),
                    Parameter::new("groupcastRadius", DataType::UINT8).with_conditions(vec![
                        ParameterCondition::BITMASK_SET {
                            field: "options".to_string(),
                            value: 0x20000,
                        },
                    ]),
                ]),
            ),
        ),
        "genOta" => Some(
            Cluster::new("genOta", 0x0019)
                .with_attribute(Attribute::new(
                    "upgradeServerID",
                    0x0000,
                    DataType::IEEE_ADDR,
                ))
                .with_attribute(Attribute::new("fileOffset", 0x0001, DataType::UINT32))
                .with_attribute(Attribute::new(
                    "currentFileVersion",
                    0x0002,
                    DataType::UINT32,
                ))
                .with_attribute(Attribute::new(
                    "currentZigbeeStackVersion",
                    0x0003,
                    DataType::UINT16,
                ))
                .with_attribute(Attribute::new(
                    "downloadedFileVersion",
                    0x0004,
                    DataType::UINT32,
                ))
                .with_attribute(Attribute::new(
                    "downloadedZigbeeStackVersion",
                    0x0005,
                    DataType::UINT16,
                ))
                .with_attribute(Attribute::new(
                    "imageUpgradeStatus",
                    0x0006,
                    DataType::ENUM8,
                ))
                .with_attribute(Attribute::new("manufacturerId", 0x0007, DataType::UINT16))
                .with_attribute(Attribute::new("imageTypeId", 0x0008, DataType::UINT16))
                .with_attribute(Attribute::new(
                    "minimumBlockPeriod",
                    0x0009,
                    DataType::UINT16,
                ))
                .with_attribute(Attribute::new("imageStamp", 0x000a, DataType::UINT32))
                .with_command(
                    Command::new("queryNextImageRequest", 0x01).with_parameters(vec![
                        Parameter::new("fieldControl", DataType::BITMAP8),
                        Parameter::new("manufacturerCode", DataType::UINT16),
                        Parameter::new("imageType", DataType::UINT16),
                        Parameter::new("fileVersion", DataType::UINT32),
                    ]),
                )
                .with_command(
                    Command::new("imageBlockRequest", 0x03).with_parameters(vec![
                        Parameter::new("fieldControl", DataType::BITMAP8),
                        Parameter::new("manufacturerCode", DataType::UINT16),
                        Parameter::new("imageType", DataType::UINT16),
                        Parameter::new("fileVersion", DataType::UINT32),
                        Parameter::new("fileOffset", DataType::UINT32),
                        Parameter::new("maximumDataSize", DataType::UINT8),
                        Parameter::new("minimumBlockPeriod", DataType::UINT16),
                    ]),
                )
                .with_command(Command::new("imagePageRequest", 0x04).with_parameters(vec![
                    Parameter::new("fieldControl", DataType::BITMAP8),
                    Parameter::new("manufacturerCode", DataType::UINT16),
                    Parameter::new("imageType", DataType::UINT16),
                    Parameter::new("fileVersion", DataType::UINT32),
                    Parameter::new("fileOffset", DataType::UINT32),
                    Parameter::new("maximumDataSize", DataType::UINT8),
                    Parameter::new("pageSize", DataType::UINT16),
                    Parameter::new("responseSpacing", DataType::UINT16),
                ]))
                .with_command(
                    Command::new("upgradeEndRequest", 0x06).with_parameters(vec![
                        Parameter::new("status", DataType::ENUM8),
                        Parameter::new("manufacturerCode", DataType::UINT16),
                        Parameter::new("imageType", DataType::UINT16),
                        Parameter::new("fileVersion", DataType::UINT32),
                    ]),
                )
                .with_command_response(Command::new("imageNotify", 0x00).with_parameters(vec![
                    Parameter::new("payloadType", DataType::ENUM8),
                    Parameter::new("queryJitter", DataType::UINT8),
                    Parameter::new("manufacturerCode", DataType::UINT16),
                    Parameter::new("imageType", DataType::UINT16),
                    Parameter::new("fileVersion", DataType::UINT32),
                ]))
                .with_command_response(
                    Command::new("queryNextImageResponse", 0x02).with_parameters(vec![
                        Parameter::new("status", DataType::ENUM8),
                        Parameter::new("manufacturerCode", DataType::UINT16).with_conditions(vec![
                            ParameterCondition::FIELD_EQUAL {
                                field: "status".to_string(),
                                value: 0,
                                reversed: false,
                            },
                        ]),
                        Parameter {
                            max: Some(65471.0),
                            ..Parameter::new("imageType", DataType::UINT16).with_conditions(vec![
                                ParameterCondition::FIELD_EQUAL {
                                    field: "status".to_string(),
                                    value: 0,
                                    reversed: false,
                                },
                            ])
                        },
                        Parameter::new("fileVersion", DataType::UINT32).with_conditions(vec![
                            ParameterCondition::FIELD_EQUAL {
                                field: "status".to_string(),
                                value: 0,
                                reversed: false,
                            },
                        ]),
                        Parameter::new("imageSize", DataType::UINT32).with_conditions(vec![
                            ParameterCondition::FIELD_EQUAL {
                                field: "status".to_string(),
                                value: 0,
                                reversed: false,
                            },
                        ]),
                    ]),
                )
                .with_command_response(Command::new("imageBlockResponse", 0x05).with_parameters(
                    vec![
                        Parameter::new("status", DataType::ENUM8),
                        Parameter::new("manufacturerCode", DataType::UINT16),
                        Parameter::new("imageType", DataType::UINT16),
                        Parameter::new("fileVersion", DataType::UINT32),
                        Parameter::new("fileOffset", DataType::UINT32),
                        Parameter::new("dataSize", DataType::UINT8),
                        Parameter::new("imageData", DataType::UNKNOWN),
                    ],
                ))
                .with_command_response(Command::new("upgradeEndResponse", 0x07).with_parameters(
                    vec![
                        Parameter::new("manufacturerCode", DataType::UINT16),
                        Parameter::new("imageType", DataType::UINT16),
                        Parameter::new("fileVersion", DataType::UINT32),
                        Parameter::new("currentTime", DataType::UTC),
                        Parameter::new("upgradeTime", DataType::UTC),
                    ],
                )),
        ),
        "touchlink" => Some(
            Cluster::new("touchlink", 0x1000)
                .with_command(
                    Command::new("scanRequest", 0x00)
                        .with_response(0x01)
                        .with_parameters(vec![
                            Parameter::new("transactionID", DataType::UINT32),
                            Parameter::new("zigbeeInformation", DataType::BITMAP8),
                            Parameter::new("touchlinkInformation", DataType::BITMAP8),
                        ]),
                )
                .with_command(
                    Command::new("deviceInformation", 0x02)
                        .with_response(0x03)
                        .with_parameters(vec![
                            Parameter::new("transactionID", DataType::UINT32),
                            Parameter::new("startIndex", DataType::UINT8),
                        ]),
                )
                .with_command(Command::new("identifyRequest", 0x06).with_parameters(vec![
                    Parameter::new("transactionID", DataType::UINT32),
                    Parameter::new("duration", DataType::UINT16),
                ]))
                .with_command(
                    Command::new("resetToFactoryNew", 0x07)
                        .with_parameters(vec![Parameter::new("transactionID", DataType::UINT32)]),
                )
                .with_command_response(Command::new("scanResponse", 0x01).with_parameters(vec![
                    Parameter::new("transactionID", DataType::UINT32),
                    Parameter::new("rssiCorrection", DataType::UINT8),
                    Parameter::new("zigbeeInformation", DataType::BITMAP8),
                    Parameter::new("touchlinkInformation", DataType::BITMAP8),
                    Parameter::new("keyBitmask", DataType::BITMAP16),
                    Parameter::new("responseID", DataType::UINT32),
                    Parameter::new("extendedPanID", DataType::IEEE_ADDR),
                    Parameter::new("networkUpdateID", DataType::UINT8),
                    Parameter::new("logicalChannel", DataType::UINT8),
                    Parameter::new("panID", DataType::UINT16),
                    Parameter::new("networkAddress", DataType::UINT16),
                    Parameter::new("numberOfSubDevices", DataType::UINT8),
                    Parameter::new("totalGroupIdentifiers", DataType::UINT8),
                    Parameter::new("endpointID", DataType::UINT8).with_conditions(vec![
                        ParameterCondition::FIELD_EQUAL {
                            field: "numberOfSubDevices".to_string(),
                            value: 1,
                            reversed: false,
                        },
                    ]),
                    Parameter::new("profileID", DataType::UINT16).with_conditions(vec![
                        ParameterCondition::FIELD_EQUAL {
                            field: "numberOfSubDevices".to_string(),
                            value: 1,
                            reversed: false,
                        },
                    ]),
                    Parameter::new("deviceID", DataType::UINT16).with_conditions(vec![
                        ParameterCondition::FIELD_EQUAL {
                            field: "numberOfSubDevices".to_string(),
                            value: 1,
                            reversed: false,
                        },
                    ]),
                    Parameter::new("version", DataType::UINT8).with_conditions(vec![
                        ParameterCondition::FIELD_EQUAL {
                            field: "numberOfSubDevices".to_string(),
                            value: 1,
                            reversed: false,
                        },
                    ]),
                    Parameter::new("groupIDCount", DataType::UINT8).with_conditions(vec![
                        ParameterCondition::FIELD_EQUAL {
                            field: "numberOfSubDevices".to_string(),
                            value: 1,
                            reversed: false,
                        },
                    ]),
                ])),
        ),
        _ => None,
    }
}

impl Attribute {
    fn with_default(mut self, value: i32) -> Self {
        self.default = Some(ZclValue::num(value));
        self
    }
}
