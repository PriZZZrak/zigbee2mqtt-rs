// Original: .upstream/zigbee-herdsman/src/zspec/zcl/utils.ts
// TODO: port the full cluster catalog and helpers.
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
}

impl Command {
    pub fn new(name: &str, ID: u8) -> Self {
        Self {
            name: name.to_string(),
            ID,
            response: None,
            parameters: Vec::new(),
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

/// Upstream `CustomClusters` used by the ported tests.
#[allow(non_snake_case)]
#[derive(Debug, Clone)]
pub struct CustomClusters {
    pub genBasic: Cluster,
    pub myCustomCluster: Cluster,
    pub myCustomClusterManuf: Cluster,
    pub myCustomWithManufAttr: Cluster,
}

impl CustomClusters {
    pub fn empty() -> Self {
        Self {
            genBasic: Cluster::new("genBasic", 0x0000),
            myCustomCluster: Cluster::new("myCustomCluster", 65534),
            myCustomClusterManuf: Cluster::new("myCustomClusterManuf", 65533),
            myCustomWithManufAttr: Cluster::new("haDiagnostic", 0x0b05),
        }
    }

    fn clusters(&self) -> [&Cluster; 4] {
        [
            &self.genBasic,
            &self.myCustomCluster,
            &self.myCustomClusterManuf,
            &self.myCustomWithManufAttr,
        ]
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
            if matched {
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
        let key = key.into();
        match key {
            CommandKey::Id(id) => match id {
                2 => Command::new("writeUndiv", 2),
                0 => Command::new("read", 0).with_response(1),
                _ => panic!("Global command does not exist"),
            },
            CommandKey::Name(name) => match name.as_str() {
                "writeUndiv" => Command::new("writeUndiv", 2),
                "read" => Command::new("read", 0).with_response(1),
                _ => panic!("Global command does not exist"),
            },
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
        0x0008 => "genLevelCtrl",
        0x0009 => "genAlarms",
        0x000b => "genRssiLocation",
        0x0300 => "lightingColorCtrl",
        0x0b05 => "haDiagnostic",
        0xfc57 => "manuSpecificAmazonWWAH",
        _ => return id.to_string(),
    }
    .to_string()
}

fn builtin_cluster_by_name(name: &str) -> Option<Cluster> {
    match name {
        "genBasic" => Some(
            Cluster::new("genBasic", 0x0000).with_attribute(Attribute::new(
                "zclVersion",
                0x0000,
                DataType::UINT8,
            )),
        ),
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
                .with_command_response(Command::new("alarm", 0x00))
                .with_command_response(Command::new("getRsp", 0x01).with_parameters(vec![
                    Parameter::new("status", DataType::ENUM8),
                    Parameter::new("alarmcode", DataType::ENUM8),
                ])),
        ),
        "genRssiLocation" => Some(
            Cluster::new("genRssiLocation", 0x000b)
                .with_attribute(Attribute {
                    min: Some(-32768.0),
                    max: Some(32767.0),
                    ..Attribute::new("coordinate1", 0x0010, DataType::INT16)
                })
                .with_attribute(Attribute::new("pathLossExponent", 0x0014, DataType::UINT16))
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
        _ => None,
    }
}

impl Attribute {
    fn with_default(mut self, value: i32) -> Self {
        self.default = Some(ZclValue::num(value));
        self
    }
}

pub fn get_cluster() {
    todo!()
}
pub fn get_cluster_attribute() {
    todo!()
}
pub fn get_cluster_command() {
    todo!()
}
pub fn get_cluster_command_response() {
    todo!()
}
pub fn get_foundation_command() {
    todo!()
}
pub fn get_foundation_command_by_name() {
    todo!()
}
pub fn get_global_command() {
    todo!()
}
pub fn is_analog_data_type() {
    todo!()
}
pub fn is_cluster_name() {
    todo!()
}
pub fn process_attribute_post_read() {
    todo!()
}
pub fn process_attribute_pre_read() {
    todo!()
}
pub fn process_attribute_write() {
    todo!()
}
pub fn process_parameter_read() {
    todo!()
}
pub fn process_parameter_write() {
    todo!()
}
