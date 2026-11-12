// Original: .upstream/zigbee-herdsman/src/controller/model/group.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use serde_json::{json, Value};

use crate::controller::database::EntityType;
use crate::controller::helpers::zcl_transaction_sequence_number;
use crate::controller::model::device::Device;
use crate::controller::model::endpoint::Endpoint;
use crate::controller::model::entity::Entity;
use crate::controller::model::zigbee_entity::ZigbeeEntity;
use crate::controller::tstype::KeyValue;
use crate::utils::logger::logger;
use crate::zspec::zcl::utils::{Cluster, Command};
use crate::zspec::zcl::utils::{CustomClusters, Utils};
use crate::zspec::zcl::zcl_frame::Frame;
use crate::zspec::zcl::zcl_header::Direction;
use crate::zspec::zcl::zcl_header::FrameType;

const NS: &str = "zh:controller:group";

static GROUPS: OnceLock<RwLock<HashMap<u16, Arc<Group>>>> = OnceLock::new();
static LOADED: AtomicBool = AtomicBool::new(false);

fn groups_slot() -> &'static RwLock<HashMap<u16, Arc<Group>>> {
    GROUPS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Upstream `Options`.
#[derive(Debug, Clone, Default)]
pub struct Options {
    pub manufacturer_code: Option<u16>,
    pub direction: Option<u8>,
    pub src_endpoint: Option<u8>,
    pub reserved_bits: Option<u8>,
    pub transaction_sequence_number: Option<u8>,
}

/// Upstream `OptionsWithDefaults`.
#[derive(Debug, Clone)]
pub struct OptionsWithDefaults {
    pub manufacturer_code: Option<u16>,
    pub direction: u8,
    pub src_endpoint: Option<u8>,
    pub reserved_bits: u8,
    pub transaction_sequence_number: Option<u8>,
}

/// Upstream `Group`.
pub struct Group {
    database_id: u64,
    pub group_id: u16,
    members: Mutex<Vec<Arc<Endpoint>>>,
    custom_clusters: Mutex<(CustomClusters, CustomClusters)>,
    pub meta: KeyValue,
}

impl Group {
    fn new(
        database_id: u64,
        group_id: u16,
        members: Vec<Arc<Endpoint>>,
        meta: KeyValue,
    ) -> Arc<Self> {
        let group = Arc::new(Self {
            database_id,
            group_id,
            members: Mutex::new(members),
            custom_clusters: Mutex::new((CustomClusters::empty(), CustomClusters::empty())),
            meta,
        });
        let clusters = group.identify_custom_clusters();
        *group.custom_clusters.lock().unwrap() = clusters;
        group
    }

    /// Upstream `get members()`.
    pub fn members(&self) -> Vec<Arc<Endpoint>> {
        self.members
            .lock()
            .unwrap()
            .iter()
            .filter(|endpoint| {
                Device::by_ieee_addr(&endpoint.device_ieee_address.lock().unwrap(), false).is_some()
            })
            .cloned()
            .collect()
    }

    /// Upstream `get customClusters()`.
    pub fn custom_clusters(&self) -> (CustomClusters, CustomClusters) {
        self.custom_clusters.lock().unwrap().clone()
    }

    /// Upstream `resetCache()`.
    pub fn reset_cache() {
        groups_slot().write().unwrap().clear();
        LOADED.store(false, Ordering::SeqCst);
    }

    /// Upstream `fromDatabaseEntry(entry)`.
    fn from_database_entry(entry: &Value) -> Arc<Group> {
        let mut members = Vec::new();
        if let Some(list) = entry.get("members").and_then(Value::as_array) {
            for member in list {
                let Some(ieee) = member.get("deviceIeeeAddr").and_then(Value::as_str) else {
                    continue;
                };
                let Some(endpoint_id) = member.get("endpointID").and_then(Value::as_u64) else {
                    continue;
                };
                if let Some(device) = Device::by_ieee_addr(ieee, false) {
                    if let Some(endpoint) = device.get_endpoint(endpoint_id as u8) {
                        members.push(endpoint);
                    }
                }
            }
        }
        Self::new(
            entry.get("id").and_then(Value::as_u64).unwrap_or(0),
            entry.get("groupID").and_then(Value::as_u64).unwrap_or(0) as u16,
            members,
            entry.get("meta").cloned().unwrap_or_else(|| json!({})),
        )
    }

    /// Upstream `toDatabaseRecord()`.
    fn to_database_record(&self) -> Value {
        let mut members = Vec::new();
        for member in self.members.lock().unwrap().iter() {
            if let Some(device) =
                Device::by_ieee_addr(&member.device_ieee_address.lock().unwrap(), false)
            {
                members.push(json!({
                    "deviceIeeeAddr": device.ieee_addr(),
                    "endpointID": member.ID,
                }));
            }
        }
        json!({
            "id": self.database_id,
            "type": "Group",
            "groupID": self.group_id,
            "members": members,
            "meta": self.meta,
        })
    }

    /// Upstream `loadFromDatabaseIfNecessary()`.
    fn load_from_database_if_necessary() {
        if !LOADED.load(Ordering::SeqCst) {
            let entries = Entity::database()
                .lock()
                .unwrap()
                .get_entries_iterator(&[EntityType::Group])
                .cloned()
                .collect::<Vec<_>>();
            for entry in &entries {
                let group = Group::from_database_entry(entry);
                groups_slot().write().unwrap().insert(group.group_id, group);
            }
            LOADED.store(true, Ordering::SeqCst);
        }
    }

    /// Upstream `byGroupID(groupID)`.
    pub fn by_group_id(group_id: u16) -> Option<Arc<Group>> {
        Group::load_from_database_if_necessary();
        groups_slot().read().unwrap().get(&group_id).cloned()
    }

    /// Upstream `all()`.
    pub fn all() -> Vec<Arc<Group>> {
        Group::load_from_database_if_necessary();
        groups_slot().read().unwrap().values().cloned().collect()
    }

    /// Upstream `allIterator(predicate?)`.
    pub fn all_iterator(predicate: impl Fn(&Arc<Group>) -> bool) -> Vec<Arc<Group>> {
        Group::all()
            .into_iter()
            .filter(|group| predicate(group))
            .collect()
    }

    /// Upstream `create(groupID)`.
    pub fn create(group_id: u16) -> Result<Arc<Group>, String> {
        if group_id < 0x0001 || group_id > 0xfff7 {
            return Err("GroupID must be >= 1 and <= 65527".to_string());
        }
        Group::load_from_database_if_necessary();
        if groups_slot().read().unwrap().contains_key(&group_id) {
            return Err(format!("Group with groupID '{group_id}' already exists"));
        }
        let database_id = Entity::database().lock().unwrap().new_id();
        let group = Self::new(database_id, group_id, Vec::new(), json!({}));
        Entity::database()
            .lock()
            .unwrap()
            .insert(group.to_database_record())?;
        groups_slot()
            .write()
            .unwrap()
            .insert(group_id, group.clone());
        Ok(group)
    }

    /// Upstream `removeFromNetwork()`.
    pub async fn remove_from_network(&self) -> Result<(), String> {
        let members = self.members.lock().unwrap().clone();
        for endpoint in members {
            endpoint.remove_from_group(self).await?;
        }
        self.remove_from_database();
        Ok(())
    }

    /// Upstream `removeFromDatabase()`.
    pub fn remove_from_database(&self) {
        Group::load_from_database_if_necessary();
        let database_arc = Entity::database();
        let mut database = database_arc.lock().unwrap();
        if database.has(self.database_id) {
            let _ = database.remove(self.database_id);
        }
        drop(database);
        groups_slot().write().unwrap().remove(&self.group_id);
    }

    /// Upstream `save(writeDatabase = true)`.
    pub fn save(&self, write_database: bool) {
        let record = self.to_database_record();
        let _ = Entity::database()
            .lock()
            .unwrap()
            .update(record, write_database);
    }

    /// Upstream `addMember(endpoint)`.
    pub fn add_member(&self, endpoint: &Arc<Endpoint>) {
        let mut members = self.members.lock().unwrap();
        if !members.iter().any(|member| Arc::ptr_eq(member, endpoint)) {
            members.push(endpoint.clone());
            drop(members);
            self.save(true);
            let clusters = self.identify_custom_clusters();
            *self.custom_clusters.lock().unwrap() = clusters;
        }
    }

    /// Upstream `removeMember(endpoint)`.
    pub fn remove_member(&self, endpoint: &Arc<Endpoint>) {
        let mut members = self.members.lock().unwrap();
        if let Some(index) = members
            .iter()
            .position(|member| Arc::ptr_eq(member, endpoint))
        {
            members.remove(index);
            drop(members);
            self.save(true);
            let clusters = self.identify_custom_clusters();
            *self.custom_clusters.lock().unwrap() = clusters;
        }
    }

    /// Upstream `hasMember(endpoint)`.
    pub fn has_member(&self, endpoint: &Arc<Endpoint>) -> bool {
        self.members
            .lock()
            .unwrap()
            .iter()
            .any(|member| Arc::ptr_eq(member, endpoint))
    }

    /// Upstream `#identifyCustomClusters()`.
    fn identify_custom_clusters(&self) -> (CustomClusters, CustomClusters) {
        let members = self.members();
        if members.is_empty() {
            return (CustomClusters::empty(), CustomClusters::empty());
        }
        let first = members[0].get_device();
        let custom = first.custom_clusters();
        let mut input = CustomClusters::empty();
        let mut output = CustomClusters::empty();
        for cluster in custom.clusters() {
            let mut has_input = true;
            let mut has_output = true;
            for member in &members {
                let device = member.get_device();
                if let Some(custom_cluster) = device.custom_clusters().get(&cluster.name) {
                    has_input = member
                        .input_clusters
                        .lock()
                        .unwrap()
                        .contains(&(custom_cluster.ID as u16));
                    has_output = member
                        .output_clusters
                        .lock()
                        .unwrap()
                        .contains(&(custom_cluster.ID as u16));
                    if !has_input && !has_output {
                        break;
                    }
                } else {
                    has_input = false;
                    has_output = false;
                    break;
                }
            }
            if has_input {
                input.insert(cluster.clone());
            }
            if has_output {
                output.insert(cluster.clone());
            }
        }
        (input, output)
    }

    /// Upstream `write(clusterKey, attributes, options?)`.
    pub async fn write(
        &self,
        cluster_key: Value,
        attributes: Value,
        options: Option<Options>,
    ) -> Result<(), String> {
        let options = self.get_options_with_defaults(options, Direction::CLIENT_TO_SERVER as u8);
        let (input_clusters, output_clusters) = self.custom_clusters();
        let custom_clusters = if options.direction == Direction::SERVER_TO_CLIENT as u8 {
            &output_clusters
        } else {
            &input_clusters
        };
        let cluster = Utils::get_cluster(
            key_from_value(&cluster_key),
            options.manufacturer_code,
            custom_clusters,
        );
        let mut payload = Vec::new();
        for (name_or_id, value) in attributes.as_object().cloned().unwrap_or_default() {
            if let Some(attribute) = Utils::get_cluster_attribute(
                &cluster,
                name_or_id.as_str(),
                options.manufacturer_code,
            ) {
                let attr_data = Utils::process_attribute_write(
                    &attribute,
                    crate::zspec::zcl::utils::ZclValue::from_json(&value),
                );
                payload.push(json!({
                    "attrId": attribute.ID,
                    "attrData": attr_data.to_json(),
                    "dataType": attribute.r#type as u16,
                }));
            } else if let Ok(id) = name_or_id.parse::<u16>() {
                payload.push(json!({
                    "attrId": id,
                    "attrData": value.get("value").cloned().unwrap_or(Value::Null),
                    "dataType": value.get("type").and_then(Value::as_u64).unwrap_or(0),
                }));
            } else {
                return Err(format!(
                    "Unknown attribute '{name_or_id}', specify either an existing attribute or a number"
                ));
            }
        }
        let command = command_by_name(&cluster, "write");
        let frame = Frame::create_with_definitions(
            cluster.clone(),
            command,
            FrameType::GLOBAL as u8,
            options.direction,
            true,
            options.manufacturer_code,
            options
                .transaction_sequence_number
                .unwrap_or_else(zcl_transaction_sequence_number::next),
            Value::Array(payload),
            options.reserved_bits,
        );
        let log = format!(
            "Write {} {}({}, {})",
            self.group_id,
            cluster.name,
            serde_json::to_string(&attributes).unwrap_or_else(|_| "null".to_string()),
            options_to_upstream_json(&options)
        );
        logger().debug(&log, NS);
        Entity::adapter()
            .send_zcl_frame_to_group(self.group_id, &frame, options.src_endpoint, None)
            .await
            .map_err(|error| format!("{log} failed ({error})"))
    }

    /// Upstream `read(clusterKey, attributes, options?)`.
    pub async fn read(
        &self,
        cluster_key: Value,
        attributes: Value,
        options: Option<Options>,
    ) -> Result<(), String> {
        let options = self.get_options_with_defaults(options, Direction::CLIENT_TO_SERVER as u8);
        let (input_clusters, output_clusters) = self.custom_clusters();
        let custom_clusters = if options.direction == Direction::SERVER_TO_CLIENT as u8 {
            &output_clusters
        } else {
            &input_clusters
        };
        let cluster = Utils::get_cluster(
            key_from_value(&cluster_key),
            options.manufacturer_code,
            custom_clusters,
        );
        let mut payload = Vec::new();
        for attribute in attributes.as_array().cloned().unwrap_or_default() {
            if let Some(id) = attribute.as_u64() {
                payload.push(json!({"attrId": id}));
            } else if let Some(name) = attribute.as_str() {
                if let Some(attribute) =
                    Utils::get_cluster_attribute(&cluster, name, options.manufacturer_code)
                {
                    Utils::process_attribute_pre_read(&attribute);
                    payload.push(json!({"attrId": attribute.ID}));
                } else {
                    logger().warning(
                        &format!(
                            "Ignoring unknown attribute {name} in cluster {}",
                            cluster.name
                        ),
                        NS,
                    );
                }
            }
        }
        let command = command_by_name(&cluster, "read");
        let frame = Frame::create_with_definitions(
            cluster.clone(),
            command,
            FrameType::GLOBAL as u8,
            options.direction,
            true,
            options.manufacturer_code,
            options
                .transaction_sequence_number
                .unwrap_or_else(zcl_transaction_sequence_number::next),
            Value::Array(payload),
            options.reserved_bits,
        );
        let log = format!(
            "Read {} {}({}, {})",
            self.group_id,
            cluster.name,
            serde_json::to_string(&attributes).unwrap_or_else(|_| "null".to_string()),
            options_to_upstream_json(&options)
        );
        logger().debug(&log, NS);
        Entity::adapter()
            .send_zcl_frame_to_group(self.group_id, &frame, options.src_endpoint, None)
            .await
            .map_err(|error| format!("{log} failed ({error})"))
    }

    /// Upstream `command(clusterKey, commandKey, payload, options?)`.
    pub async fn command(
        &self,
        cluster_key: Value,
        command_key: Value,
        payload: Value,
        options: Option<Options>,
    ) -> Result<(), String> {
        let options = self.get_options_with_defaults(options, Direction::CLIENT_TO_SERVER as u8);
        let (input_clusters, output_clusters) = self.custom_clusters();
        let custom_clusters = if options.direction == Direction::SERVER_TO_CLIENT as u8 {
            &output_clusters
        } else {
            &input_clusters
        };
        let cluster = Utils::get_cluster(
            key_from_value(&cluster_key),
            options.manufacturer_code,
            custom_clusters,
        );
        let command = Utils::get_cluster_command(&cluster, key_from_command(&command_key));
        let log = format!(
            "Command {} {}.{}({})",
            self.group_id,
            cluster.name,
            command.name,
            serde_json::to_string(&payload).unwrap_or_else(|_| "null".to_string())
        );
        logger().debug(&log, NS);
        let frame = Frame::create_with_definitions(
            cluster,
            command,
            FrameType::SPECIFIC as u8,
            options.direction,
            true,
            options.manufacturer_code,
            options
                .transaction_sequence_number
                .unwrap_or_else(zcl_transaction_sequence_number::next),
            payload,
            options.reserved_bits,
        );
        Entity::adapter()
            .send_zcl_frame_to_group(self.group_id, &frame, options.src_endpoint, None)
            .await
            .map_err(|error| format!("{log} failed ({error})"))
    }

    fn get_options_with_defaults(
        &self,
        options: Option<Options>,
        direction: u8,
    ) -> OptionsWithDefaults {
        let options = options.unwrap_or_default();
        OptionsWithDefaults {
            direction: options.direction.unwrap_or(direction),
            src_endpoint: options.src_endpoint,
            reserved_bits: options.reserved_bits.unwrap_or(0),
            manufacturer_code: options.manufacturer_code,
            transaction_sequence_number: options.transaction_sequence_number,
        }
    }
}

/// Serializes group options like upstream `JSON.stringify(optionsWithDefaults)`.
fn options_to_upstream_json(options: &OptionsWithDefaults) -> String {
    let mut object = serde_json::Map::new();
    object.insert("direction".to_string(), json!(options.direction));
    if let Some(src_endpoint) = options.src_endpoint {
        object.insert("srcEndpoint".to_string(), json!(src_endpoint));
    }
    object.insert("reservedBits".to_string(), json!(options.reserved_bits));
    if let Some(manufacturer_code) = options.manufacturer_code {
        object.insert("manufacturerCode".to_string(), json!(manufacturer_code));
    }
    if let Some(transaction_sequence_number) = options.transaction_sequence_number {
        object.insert(
            "transactionSequenceNumber".to_string(),
            json!(transaction_sequence_number),
        );
    }
    serde_json::to_string(&Value::Object(object)).unwrap_or_else(|_| "{}".to_string())
}

fn command_by_name(cluster: &Cluster, name: &str) -> Command {
    Utils::get_global_command(name)
}

impl ZigbeeEntity for Group {
    fn read<'a>(
        &'a mut self,
        cluster_key: Value,
        attributes: Value,
        options: Option<Value>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Option<Value>, String>> + Send + 'a>,
    > {
        let options = options.and_then(options_from_value);
        Box::pin(async move {
            Group::read(self, cluster_key, attributes, options).await?;
            Ok(None)
        })
    }

    fn write<'a>(
        &'a mut self,
        cluster_key: Value,
        attributes: Value,
        options: Option<Value>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>> {
        let options = options.and_then(options_from_value);
        Box::pin(async move { Group::write(self, cluster_key, attributes, options).await })
    }

    fn command<'a>(
        &'a mut self,
        cluster_key: Value,
        command_key: Value,
        payload: Value,
        options: Option<Value>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Option<Value>, String>> + Send + 'a>,
    > {
        let options = options.and_then(options_from_value);
        Box::pin(async move {
            Group::command(self, cluster_key, command_key, payload, options).await?;
            Ok(None)
        })
    }
}

fn options_from_value(value: Value) -> Option<Options> {
    let object = value.as_object()?;
    Some(Options {
        manufacturer_code: object
            .get("manufacturerCode")
            .and_then(Value::as_u64)
            .map(|v| v as u16),
        direction: object
            .get("direction")
            .and_then(Value::as_u64)
            .map(|v| v as u8),
        src_endpoint: object
            .get("srcEndpoint")
            .and_then(Value::as_u64)
            .map(|v| v as u8),
        reserved_bits: object
            .get("reservedBits")
            .and_then(Value::as_u64)
            .map(|v| v as u8),
        transaction_sequence_number: object
            .get("transactionSequenceNumber")
            .and_then(Value::as_u64)
            .map(|v| v as u8),
    })
}

fn key_from_value(value: &Value) -> crate::zspec::zcl::utils::ClusterKey {
    match value {
        Value::String(name) => crate::zspec::zcl::utils::ClusterKey::Name(name.clone()),
        other => crate::zspec::zcl::utils::ClusterKey::Id(other.as_u64().unwrap_or(0) as u32),
    }
}

fn key_from_command(value: &Value) -> crate::zspec::zcl::utils::CommandKey {
    match value {
        Value::String(name) => crate::zspec::zcl::utils::CommandKey::Name(name.clone()),
        other => crate::zspec::zcl::utils::CommandKey::Id(other.as_u64().unwrap_or(0) as u32),
    }
}
