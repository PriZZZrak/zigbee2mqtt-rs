// Original: .upstream/zigbee-herdsman/src/controller/controller.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};

use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::adapter::events::{DeviceJoinedPayload, DeviceLeavePayload, ZclPayload};
use crate::controller::database::Database;
use crate::controller::database::DeviceType;
use crate::controller::events::{
    ControllerEvent, DeviceAnnouncePayload, DeviceInterviewPayload,
    DeviceJoinedPayload as ModelDeviceJoinedPayload, DeviceLeavePayload as ModelDeviceLeavePayload,
    DeviceNetworkAddressChangedPayload, GreenPowerDeviceJoinedPayload, LastSeenChangedPayload,
    MessageMeta, MessagePayload, PermitJoinChangedPayload,
};
use crate::controller::green_power::{
    GreenPower, GreenPowerAdapter, GreenPowerEvent, GreenPowerNetworkParameters,
};
use crate::controller::helpers::install_codes::{check_install_code, parse_install_code};
use crate::controller::helpers::zcl_frame_converter::{attribute_key_value, attribute_list};
use crate::controller::helpers::zcl_transaction_sequence_number;
use crate::controller::model::device::{Device, InterviewState};
use crate::controller::model::endpoint::Endpoint;
use crate::controller::model::entity::{ControllerAdapter, Entity};
use crate::controller::model::group::Group;
use crate::controller::touchlink::{Touchlink, TouchlinkAdapter};
use crate::models::backup::Backup;
use crate::models::network_options::NetworkOptions;
use crate::utils::backup::to_unified_backup;
use crate::utils::datetime::now_ms;
use crate::utils::logger::logger;
use crate::utils::wait::wait;
use crate::zspec::consts::{BLANK_EUI64, COORDINATOR_ADDRESS, HA_ENDPOINT, HA_PROFILE_ID};
use crate::zspec::enums::BroadcastAddress;
use crate::zspec::utils::eui64_be_buffer_to_hex;
use crate::zspec::zcl::utils::{ClusterKey, CommandKey, CustomClusters, Utils};
use crate::zspec::zcl::zcl_frame::Frame;
use crate::zspec::zcl::zcl_header::{Direction, FrameType};
use crate::zspec::zdo::buffalo_zdo::BuffaloZdo;
use crate::zspec::zdo::definition::clusters::ClusterId;
use crate::zspec::zdo::definition::consts::ZDO_PROFILE_ID;

const NS: &str = "zh:controller";

pub type ControllerFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'a>>;

/// Upstream `TsType.StartResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartResult {
    Resumed,
    Reset,
    Restored,
}

/// Upstream `AdapterTsType.NetworkParameters`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerNetworkParameters {
    pub pan_id: u16,
    pub extended_pan_id: String,
    pub channel: u8,
    pub nwk_update_id: u8,
}

/// Upstream `AdapterTsType.CoordinatorVersion`.
#[derive(Debug, Clone, PartialEq)]
pub struct ControllerCoordinatorVersion {
    pub r#type: String,
    pub meta: Value,
}

/// Upstream `controller/tstype.ts` `RawPayload.zcl`.
#[derive(Debug, Clone, Default)]
pub struct RawZclPayload {
    pub frame_type: Option<u8>,
    pub direction: Option<u8>,
    pub disable_default_response: Option<bool>,
    pub manufacturer_code: Option<u16>,
    pub tsn: Option<u8>,
    pub command_key: Value,
    pub payload: Option<Value>,
}

/// Upstream `RawPayload` — tstype.ts:134.
#[derive(Debug, Clone, Default)]
pub struct RawPayload {
    pub ieee_address: Option<String>,
    pub network_address: Option<u16>,
    pub group_id: Option<u16>,
    pub dst_endpoint: Option<u8>,
    pub src_endpoint: Option<u8>,
    pub inter_pan: Option<bool>,
    pub profile_id: Option<u16>,
    pub cluster_key: Option<Value>,
    pub zdo_params: Option<Vec<Value>>,
    pub zcl: Option<RawZclPayload>,
    pub disable_response: Option<bool>,
    pub timeout: Option<u64>,
}

/// Upstream `sendRaw` result union (`GenericZdoResponse | ZclPayload | undefined`).
#[derive(Debug, Clone)]
pub enum SendRawResponse {
    Zdo(Value),
    Zcl(ZclPayload),
}

/// Upstream adapter events consumed by the controller.
#[derive(Debug, Clone)]
pub enum ControllerAdapterEvent {
    Disconnected,
    DeviceJoined(DeviceJoinedPayload),
    ZclPayload(ZclPayload),
    /// Upstream `zdoResponse` (clusterId, response).
    ZdoResponse {
        cluster_id: u16,
        response: Value,
    },
    DeviceLeave(DeviceLeavePayload),
}

/// Full adapter surface used by the controller. The Z-Stack wrapper implements
/// this trait; the controller itself only depends on the boundary.
pub trait ControllerRuntimeAdapter:
    ControllerAdapter + GreenPowerAdapter + TouchlinkAdapter
{
    /// Upstream `adapter.start()`.
    fn start(&self) -> ControllerFuture<'_, StartResult>;
    /// Upstream `adapter.stop()`.
    fn stop(&self) -> ControllerFuture<'_, ()>;
    /// Upstream `adapter.on(...)` registrations.
    fn subscribe(&self) -> mpsc::UnboundedReceiver<ControllerAdapterEvent>;
    /// Upstream `adapter.permitJoin(time, networkAddress)`.
    fn permit_join(&self, time: u8, network_address: Option<u16>) -> ControllerFuture<'_, ()>;
    /// Upstream `adapter.getNetworkParameters()`.
    fn get_network_parameters_full(&self) -> ControllerFuture<'_, ControllerNetworkParameters>;
    /// Upstream `adapter.getCoordinatorVersion()`.
    fn get_coordinator_version(&self) -> ControllerFuture<'_, ControllerCoordinatorVersion>;
    /// Upstream `adapter.supportsBackup()`.
    fn supports_backup(&self) -> ControllerFuture<'_, bool>;
    /// Upstream `adapter.backup(ieeeAddresses)`.
    fn backup(&self, ieee_addresses: &[String]) -> ControllerFuture<'_, Backup>;
    /// Upstream `adapter.reset(type)`.
    fn reset(&self, r#type: &str) -> ControllerFuture<'_, ()>;
    /// Upstream `adapter.addInstallCode(ieeeAddress, key, hashed)`.
    fn add_install_code(
        &self,
        ieee_address: &str,
        key: &[u8],
        hashed: bool,
    ) -> ControllerFuture<'_, ()>;
    /// Upstream `adapter.manufacturerID`.
    fn manufacturer_id(&self) -> Option<u16>;
}

type AcceptJoiningDeviceHandler =
    Arc<dyn Fn(String) -> ControllerFuture<'static, bool> + Send + Sync>;

/// Upstream `Options` — controller.ts:31.
#[derive(Clone)]
pub struct ControllerOptions {
    pub network: NetworkOptions,
    pub transport: Value,
    pub database_path: String,
    pub database_backup_path: Option<String>,
    pub backup_path: Option<String>,
    pub adapter: Value,
    pub accept_joining_device_handler: Option<AcceptJoiningDeviceHandler>,
}

impl std::fmt::Debug for ControllerOptions {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ControllerOptions")
            .field("network", &self.network)
            .field("database_path", &self.database_path)
            .field("database_backup_path", &self.database_backup_path)
            .field("backup_path", &self.backup_path)
            .field(
                "accept_joining_device_handler",
                &self.accept_joining_device_handler.is_some(),
            )
            .finish()
    }
}

type Listener = Arc<dyn Fn(&ControllerEvent) + Send + Sync>;

/// Upstream `class Controller extends EventEmitter<ControllerEventMap>`.
pub struct Controller {
    options: ControllerOptions,
    database: OnceLock<Arc<Mutex<Database>>>,
    adapter: OnceLock<Arc<dyn ControllerRuntimeAdapter>>,
    green_power: Mutex<Option<Arc<GreenPower>>>,
    touchlink: Arc<tokio::sync::Mutex<Option<Touchlink<Arc<dyn ControllerRuntimeAdapter>>>>>,
    permit_join_end: Mutex<Option<u64>>,
    permit_join_generation: AtomicU64,
    timers: Mutex<Vec<tokio::task::JoinHandle<()>>>,
    stopping: AtomicBool,
    adapter_disconnected: AtomicBool,
    network_parameters_cached: tokio::sync::Mutex<Option<ControllerNetworkParameters>>,
    unknown_devices: Mutex<HashSet<u16>>,
    listeners: Mutex<Vec<Listener>>,
}

impl Controller {
    /// Upstream `constructor(options)` — controller.ts:83.
    pub fn new(options: ControllerOptions) -> Result<Self, String> {
        for channel in &options.network.channel_list {
            if *channel < 11 || *channel > 26 {
                return Err(format!(
                    "'{channel}' is an invalid channel, use a channel between 11 - 26."
                ));
            }
        }
        if options.network.network_key.len() != 16 {
            return Err(format!(
                "Network key must be a 16 digits long array, got {:?}.",
                options.network.network_key
            ));
        }
        if options.network.extended_pan_id.len() != 8 {
            return Err(format!(
                "ExtendedPanID must be an 8 digits long array, got {:?}.",
                options.network.extended_pan_id
            ));
        }
        if options.network.pan_id == 0 || options.network.pan_id == u16::MAX {
            return Err(format!(
                "PanID must have a value of 0x0001 (1) - 0xFFFE (65534), got {}.",
                options.network.pan_id
            ));
        }

        Ok(Self {
            options,
            database: OnceLock::new(),
            adapter: OnceLock::new(),
            green_power: Mutex::new(None),
            touchlink: Arc::new(tokio::sync::Mutex::new(None)),
            permit_join_end: Mutex::new(None),
            permit_join_generation: AtomicU64::new(0),
            timers: Mutex::new(Vec::new()),
            stopping: AtomicBool::new(false),
            adapter_disconnected: AtomicBool::new(true),
            network_parameters_cached: tokio::sync::Mutex::new(None),
            unknown_devices: Mutex::new(HashSet::new()),
            listeners: Mutex::new(Vec::new()),
        })
    }

    /// Upstream `get greenPower()`.
    pub fn green_power(&self) -> Option<Arc<GreenPower>> {
        self.green_power
            .lock()
            .expect("green power slot poisoned")
            .clone()
    }

    /// Upstream `get touchlink()`.
    pub fn touchlink(
        &self,
    ) -> Arc<tokio::sync::Mutex<Option<Touchlink<Arc<dyn ControllerRuntimeAdapter>>>>> {
        self.touchlink.clone()
    }

    /// Upstream `on(event, listener)`.
    pub fn on<F>(&self, listener: F)
    where
        F: Fn(&ControllerEvent) + Send + Sync + 'static,
    {
        self.listeners
            .lock()
            .expect("controller listeners poisoned")
            .push(Arc::new(listener));
    }

    fn emit(&self, event: &ControllerEvent) {
        let listeners = self
            .listeners
            .lock()
            .expect("controller listeners poisoned")
            .clone();
        for listener in listeners {
            listener(event);
        }
    }

    fn self_and_device_emit(&self, event: ControllerEvent) {
        // Upstream also forwards the event to the device's EventEmitter. The
        // Rust model layer has no per-device emitter; consumers subscribe to
        // the controller bus and filter by `device`.
        self.emit(&event);
    }

    fn adapter(&self) -> Result<Arc<dyn ControllerRuntimeAdapter>, String> {
        self.adapter
            .get()
            .cloned()
            .ok_or_else(|| "Controller not started".to_string())
    }

    fn database(&self) -> Result<Arc<Mutex<Database>>, String> {
        self.database
            .get()
            .cloned()
            .ok_or_else(|| "Controller database not opened".to_string())
    }

    fn options_log_json(&self) -> Value {
        json!({
            "network": {
                "panID": self.options.network.pan_id,
                "extendedPanID": self.options.network.extended_pan_id,
                "channelList": self.options.network.channel_list,
                "networkKey": "HIDDEN",
                "networkKeyDistribute": self.options.network.network_key_distribute,
            },
            "transport": self.options.transport,
            "adapter": self.options.adapter,
            "databasePath": self.options.database_path,
            "databaseBackupPath": self.options.database_backup_path,
            "backupPath": self.options.backup_path,
        })
    }

    /// Upstream `start(abortSignal?)` — controller.ts:134.
    ///
    /// The adapter is injected by the caller (upstream constructs it from the
    /// transport options). Abort signals are expressed by dropping the future.
    pub async fn start(
        self: &Arc<Self>,
        adapter: Arc<dyn ControllerRuntimeAdapter>,
    ) -> Result<StartResult, String> {
        // Database (create and inject)
        let database = Arc::new(Mutex::new(Database::open(&self.options.database_path)));
        let _ = self.database.set(database.clone());
        Entity::inject_database(database);

        // Adapter (inject before starting so timers can use it)
        let _ = self.adapter.set(adapter.clone());

        logger().debug(
            &format!("Starting with options '{}'", self.options_log_json()),
            NS,
        );
        let start_result = adapter.start().await?;
        logger().debug(&format!("Started with result '{start_result:?}'"), NS);
        self.adapter_disconnected.store(false, Ordering::SeqCst);

        // Register early to always handle transport issues immediately after `start`.
        let mut events = adapter.subscribe();
        let weak = Arc::downgrade(self);
        let event_task = tokio::spawn(async move {
            while let Some(event) = events.recv().await {
                let Some(controller) = weak.upgrade() else {
                    break;
                };
                if let Err(error) = controller.handle_adapter_event(event).await {
                    logger().error(&format!("Failed to handle adapter event: {error}"), NS);
                }
            }
        });
        self.timers
            .lock()
            .expect("controller timers poisoned")
            .push(event_task);

        // Check if we have to change the channel, only when adapter `resumed`.
        if start_result == StartResult::Resumed {
            let net_params = self.get_network_parameters().await?;
            let configured_channel = self
                .options
                .network
                .channel_list
                .first()
                .copied()
                .unwrap_or(0);
            let adapter_channel = net_params.channel;
            let nwk_update_id = net_params.nwk_update_id;

            if configured_channel != adapter_channel {
                logger().info(
                    &format!(
                        "Configured channel '{configured_channel}' does not match adapter channel '{adapter_channel}', changing channel"
                    ),
                    NS,
                );
                self.change_channel(adapter_channel, configured_channel, nwk_update_id)
                    .await?;
            }
        }

        Entity::inject_adapter(adapter.clone());

        logger().debug(
            &format!(
                "Injected database: {}, adapter: true",
                self.database.get().is_some()
            ),
            NS,
        );

        let green_power = Arc::new(GreenPower::new(adapter.clone()));
        {
            let weak = Arc::downgrade(self);
            green_power.on(move |event| {
                let Some(controller) = weak.upgrade() else {
                    return;
                };
                match event {
                    GreenPowerEvent::DeviceJoined(payload) => {
                        controller.on_device_joined_green_power(payload.clone())
                    }
                    GreenPowerEvent::DeviceLeave(source_id) => {
                        controller.on_device_leave_green_power(*source_id)
                    }
                }
            });
        }
        *self.green_power.lock().expect("green power slot poisoned") = Some(green_power);

        if start_result == StartResult::Reset {
            if let Some(database_backup_path) = &self.options.database_backup_path {
                if std::path::Path::new(&self.options.database_path).exists() {
                    std::fs::copy(&self.options.database_path, database_backup_path)
                        .map_err(|error| error.to_string())?;
                }
            }

            self.database()?
                .lock()
                .expect("database poisoned")
                .clear()?;
            Group::reset_cache();
            Device::reset_cache();
        }

        let backup_missing = self
            .options
            .backup_path
            .as_ref()
            .is_some_and(|path| !std::path::Path::new(path).exists());
        if start_result == StartResult::Reset || backup_missing {
            self.backup().await?;
        }

        // Add coordinator to the database if it is not there yet.
        let coordinator_ieee = adapter.get_coordinator_ieee().await?;

        if Device::by_type(DeviceType::Coordinator).is_empty() {
            logger().debug("No coordinator in database, querying...", NS);
            let coordinator = Device::create(
                DeviceType::Coordinator,
                &coordinator_ieee,
                COORDINATOR_ADDRESS,
                adapter.manufacturer_id(),
                None,
                None,
                None,
                InterviewState::Successful,
                None,
            )?;

            coordinator.update_active_endpoints().await?;

            for endpoint in coordinator.endpoints() {
                endpoint.update_simple_descriptor().await?;
            }

            coordinator.save(true);
        }

        // Update coordinator ieeeAddr if changed.
        let database_coordinator = Device::by_type(DeviceType::Coordinator)
            .into_iter()
            .next()
            .ok_or_else(|| "Coordinator missing from database".to_string())?;
        if database_coordinator.ieee_addr() != coordinator_ieee {
            logger().info(
                &format!("Coordinator address changed, updating to '{coordinator_ieee}'"),
                NS,
            );
            database_coordinator.change_ieee_address(coordinator_ieee);
        }

        // Set backup timer to 1 day.
        {
            let weak = Arc::downgrade(self);
            let handle = tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(86_400_000)).await;
                    let Some(controller) = weak.upgrade() else {
                        break;
                    };
                    if let Err(error) = controller.backup().await {
                        logger().error(&format!("Failed to backup: {error}"), NS);
                    }
                }
            });
            self.timers
                .lock()
                .expect("controller timers poisoned")
                .push(handle);
        }

        // Set database save timer to 5 minutes.
        {
            let weak = Arc::downgrade(self);
            let handle = tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(300_000)).await;
                    let Some(controller) = weak.upgrade() else {
                        break;
                    };
                    if let Err(error) = controller.database_save() {
                        logger().error(&format!("Failed to save database: {error}"), NS);
                    }
                }
            });
            self.timers
                .lock()
                .expect("controller timers poisoned")
                .push(handle);
        }

        *self.touchlink.lock().await = Some(Touchlink::new(adapter));

        Ok(start_result)
    }

    /// Upstream `sendRaw(rawPayload, customClusters)` — controller.ts:258.
    pub async fn send_raw(
        &self,
        raw_payload: RawPayload,
        custom_clusters: &CustomClusters,
    ) -> Result<Option<SendRawResponse>, String> {
        let profile_id = raw_payload.profile_id.unwrap_or(HA_PROFILE_ID);
        let src_endpoint = raw_payload.src_endpoint.unwrap_or(HA_ENDPOINT);
        let disable_response = raw_payload.disable_response.unwrap_or(false);
        let timeout = raw_payload.timeout.unwrap_or(10000);

        if profile_id == ZDO_PROFILE_ID {
            let ieee_address = raw_payload
                .ieee_address
                .as_ref()
                .ok_or_else(|| "ieeeAddress required for ZDO".to_string())?;
            let network_address = raw_payload
                .network_address
                .ok_or_else(|| "networkAddress required for ZDO".to_string())?;
            let cluster_id = raw_payload
                .cluster_key
                .as_ref()
                .and_then(Value::as_u64)
                .ok_or_else(|| "ZDO clusterKey must be a number".to_string())?
                as u16;
            let zdo_params = raw_payload
                .zdo_params
                .as_ref()
                .filter(|params| !params.is_empty())
                .ok_or_else(|| "ZDO requires at least one parameter".to_string())?;

            let adapter = self.adapter()?;
            let buffer = BuffaloZdo::build_request(
                adapter.has_zdo_message_overhead(),
                cluster_id,
                zdo_params,
            );
            let response = adapter
                .send_zdo(
                    ieee_address,
                    network_address,
                    cluster_id,
                    &buffer,
                    disable_response,
                )
                .await?;
            return Ok(Some(SendRawResponse::Zdo(response)));
        }

        let zcl = raw_payload
            .zcl
            .as_ref()
            .ok_or_else(|| "zcl payload required".to_string())?;
        let cluster_key = raw_payload
            .cluster_key
            .clone()
            .ok_or_else(|| "clusterKey required".to_string())?;

        if raw_payload.inter_pan.unwrap_or(false) {
            if zcl.frame_type.is_some() || zcl.direction.is_some() || zcl.payload.is_none() {
                return Err("Invalid inter-PAN payload".to_string());
            }
            if zcl.command_key.is_null() {
                return Err("commandKey required".to_string());
            }

            let cluster = match &cluster_key {
                Value::String(name) => {
                    Utils::get_cluster(name.clone(), zcl.manufacturer_code, custom_clusters)
                }
                Value::Number(number) => Utils::get_cluster(
                    number.as_u64().unwrap_or(0) as u16,
                    zcl.manufacturer_code,
                    custom_clusters,
                ),
                _ => return Err("Invalid clusterKey".to_string()),
            };
            let command_key = match &zcl.command_key {
                Value::String(name) => CommandKey::Name(name.clone()),
                Value::Number(number) => CommandKey::Id(number.as_u64().unwrap_or(0) as u32),
                _ => return Err("Invalid commandKey".to_string()),
            };
            let command = Utils::get_cluster_command(&cluster, command_key);
            let frame = Frame::create_with_definitions(
                cluster,
                command,
                FrameType::SPECIFIC as u8,
                Direction::CLIENT_TO_SERVER as u8,
                true,
                zcl.manufacturer_code,
                0,
                zcl.payload.clone().unwrap_or(Value::Null),
                0,
            );

            let adapter = self.adapter()?;
            if let Some(ieee_address) = &raw_payload.ieee_address {
                adapter
                    .send_zcl_frame_inter_pan_to_ieee_addr(&frame, ieee_address)
                    .await?;
                return Ok(None);
            }

            let response = adapter
                .send_zcl_frame_inter_pan_broadcast(&frame, timeout, disable_response)
                .await?;
            return Ok(response.map(SendRawResponse::Zcl));
        }

        let frame_type = zcl
            .frame_type
            .ok_or_else(|| "zcl.frameType required".to_string())?;
        let direction = zcl
            .direction
            .ok_or_else(|| "zcl.direction required".to_string())?;
        let manufacturer_code = zcl.manufacturer_code;
        let cluster = match &cluster_key {
            Value::String(name) => {
                Utils::get_cluster(name.clone(), manufacturer_code, custom_clusters)
            }
            Value::Number(number) => Utils::get_cluster(
                number.as_u64().unwrap_or(0) as u16,
                manufacturer_code,
                custom_clusters,
            ),
            _ => return Err("Invalid clusterKey".to_string()),
        };
        let command = if frame_type == FrameType::GLOBAL as u8 {
            let key = match &zcl.command_key {
                Value::String(name) => CommandKey::Name(name.clone()),
                Value::Number(number) => CommandKey::Id(number.as_u64().unwrap_or(0) as u32),
                _ => return Err("Invalid commandKey".to_string()),
            };
            Utils::get_global_command(key)
        } else {
            let key = match &zcl.command_key {
                Value::String(name) => CommandKey::Name(name.clone()),
                Value::Number(number) => CommandKey::Id(number.as_u64().unwrap_or(0) as u32),
                _ => return Err("Invalid commandKey".to_string()),
            };
            if direction == Direction::CLIENT_TO_SERVER as u8 {
                Utils::get_cluster_command(&cluster, key)
            } else {
                Utils::get_cluster_command_response(&cluster, key)
            }
        };

        let frame = Frame::create_with_definitions(
            cluster,
            command,
            frame_type,
            direction,
            zcl.disable_default_response.unwrap_or(false),
            manufacturer_code,
            zcl.tsn
                .unwrap_or_else(zcl_transaction_sequence_number::next),
            zcl.payload.clone().unwrap_or(Value::Null),
            0,
        );

        let adapter = self.adapter()?;

        if let Some(group_id) = raw_payload.group_id {
            adapter
                .send_zcl_frame_to_group(group_id, &frame, Some(src_endpoint), Some(profile_id))
                .await?;
            return Ok(None);
        }

        let dst_endpoint = raw_payload
            .dst_endpoint
            .ok_or_else(|| "dstEndpoint required".to_string())?;
        let network_address = raw_payload
            .network_address
            .ok_or_else(|| "networkAddress required".to_string())?;

        if network_address >= 0xfff8 {
            ControllerAdapter::send_zcl_frame_to_all(
                &*adapter,
                dst_endpoint,
                &frame,
                src_endpoint,
                network_address,
                Some(profile_id),
            )
            .await?;
            return Ok(None);
        }

        let ieee_address = raw_payload
            .ieee_address
            .as_ref()
            .ok_or_else(|| "ieeeAddress required".to_string())?;
        let response = ControllerAdapter::send_zcl_frame_to_endpoint(
            &*adapter,
            ieee_address,
            network_address,
            dst_endpoint,
            &frame,
            timeout,
            disable_response,
            false,
            Some(src_endpoint),
            Some(profile_id),
        )
        .await?;
        Ok(response.map(SendRawResponse::Zcl))
    }

    /// Upstream `addInstallCode(installCode)` — controller.ts:362.
    pub async fn add_install_code(&self, install_code: &str) -> Result<(), String> {
        let (ieee_addr, key_str) = parse_install_code(install_code);
        if ieee_addr.is_empty() || key_str.is_empty() {
            return Err(format!("Failed to parse install code '{install_code}'"));
        }
        let key: Vec<u8> = key_str
            .as_bytes()
            .chunks(2)
            .map(|chunk| {
                u8::from_str_radix(std::str::from_utf8(chunk).unwrap_or("0"), 16).unwrap_or(0)
            })
            .collect();
        let (adjusted_key, adjusted) = std::panic::catch_unwind(|| check_install_code(&key, true))
            .map_err(|_| format!("Invalid install code '{install_code}'"))?;

        if let Some(adjusted) = adjusted {
            logger().info(
                &format!("Install code was adjusted for reason '{adjusted}'."),
                NS,
            );
        }

        logger().info(&format!("Adding install code for {ieee_addr}."), NS);
        let adapter = self.adapter()?;
        adapter
            .add_install_code(&ieee_addr, &adjusted_key, false)
            .await?;

        if adjusted == Some("missing CRC") {
            // The code could also be an already-hashed key; send both.
            adapter.add_install_code(&ieee_addr, &key, true).await?;
        }

        Ok(())
    }

    /// Upstream `permitJoin(time, device?)` — controller.ts:385.
    pub async fn permit_join(
        self: &Arc<Self>,
        time: u16,
        device: Option<&Arc<Device>>,
    ) -> Result<(), String> {
        self.cancel_permit_join_timer();

        if time > 0 {
            if time > 254 {
                return Err("Cannot permit join for more than 254 seconds.".to_string());
            }

            let adapter = self.adapter()?;
            let network_address = device.map(|device| device.network_address());
            adapter.permit_join(time as u8, network_address).await?;
            if let Some(green_power) = self.green_power() {
                green_power.permit_join(time, network_address).await?;
            }

            let time_ms = u64::from(time) * 1000;
            let end = now_ms() + time_ms;
            *self
                .permit_join_end
                .lock()
                .expect("permit join end poisoned") = Some(end);
            let generation = self.permit_join_generation.fetch_add(1, Ordering::SeqCst) + 1;

            let weak = Arc::downgrade(self);
            let handle = tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(time_ms)).await;
                let Some(controller) = weak.upgrade() else {
                    return;
                };
                if controller.permit_join_generation.load(Ordering::SeqCst) != generation {
                    return;
                }
                controller.emit(&ControllerEvent::PermitJoinChanged(
                    PermitJoinChangedPayload {
                        permitted: false,
                        time: None,
                    },
                ));
                *controller
                    .permit_join_end
                    .lock()
                    .expect("permit join end poisoned") = None;
            });
            self.timers
                .lock()
                .expect("controller timers poisoned")
                .push(handle);

            self.emit(&ControllerEvent::PermitJoinChanged(
                PermitJoinChangedPayload {
                    permitted: true,
                    time: Some(u64::from(time)),
                },
            ));
        } else {
            logger().debug("Disable joining", NS);

            if let Some(green_power) = self.green_power() {
                green_power.permit_join(0, None).await?;
            }
            self.adapter()?.permit_join(0, None).await?;

            self.emit(&ControllerEvent::PermitJoinChanged(
                PermitJoinChangedPayload {
                    permitted: false,
                    time: None,
                },
            ));
        }

        Ok(())
    }

    fn cancel_permit_join_timer(&self) {
        self.permit_join_generation.fetch_add(1, Ordering::SeqCst);
        *self
            .permit_join_end
            .lock()
            .expect("permit join end poisoned") = None;
    }

    /// Upstream `getPermitJoin()`.
    pub fn get_permit_join(&self) -> bool {
        self.permit_join_end
            .lock()
            .expect("permit join end poisoned")
            .is_some()
    }

    /// Upstream `getPermitJoinEnd()`.
    pub fn get_permit_join_end(&self) -> Option<u64> {
        *self
            .permit_join_end
            .lock()
            .expect("permit join end poisoned")
    }

    /// Upstream `isStopping()`.
    pub fn is_stopping(&self) -> bool {
        self.stopping.load(Ordering::SeqCst)
    }

    /// Upstream `isAdapterDisconnected()`.
    pub fn is_adapter_disconnected(&self) -> bool {
        self.adapter_disconnected.load(Ordering::SeqCst)
    }

    /// Upstream `stop()` — controller.ts:433.
    pub async fn stop(self: &Arc<Self>) -> Result<(), String> {
        self.stopping.store(true, Ordering::SeqCst);

        {
            let mut timers = self.timers.lock().expect("controller timers poisoned");
            for timer in timers.drain(..) {
                timer.abort();
            }
        }

        if self.is_adapter_disconnected() {
            self.database_save()?;
        } else {
            if let Some(mut touchlink) = self.touchlink.lock().await.take() {
                if let Err(error) = touchlink.stop().await {
                    logger().error(&format!("Failed to stop Touchlink: {error}"), NS);
                }
            }

            if let Err(error) = self.permit_join(0, None).await {
                logger().error(&format!("Failed to disable join on stop: {error}"), NS);
            }

            self.backup().await?;
            self.adapter()?.stop().await?;

            self.adapter_disconnected.store(true, Ordering::SeqCst);
        }

        Device::reset_cache();
        Group::reset_cache();

        Ok(())
    }

    /// Upstream `databaseSave()` — controller.ts:467.
    fn database_save(&self) -> Result<(), String> {
        for device in Device::all_iterator(|_| true) {
            device.save(false);
        }
        for group in Group::all_iterator(|_| true) {
            group.save(false);
        }
        self.database()?.lock().expect("database poisoned").write()
    }

    /// Upstream `backup()` — controller.ts:479.
    pub async fn backup(&self) -> Result<(), String> {
        self.database_save()?;

        let Some(backup_path) = &self.options.backup_path else {
            return Ok(());
        };

        let adapter = self.adapter()?;
        if adapter.supports_backup().await? {
            logger().debug("Creating coordinator backup", NS);
            let backup = adapter.backup(&self.get_device_ieee_addresses()).await?;
            let unified_backup = to_unified_backup(&backup);
            let tmp_backup_path = format!("{backup_path}.tmp");
            std::fs::write(
                &tmp_backup_path,
                serde_json::to_string_pretty(&unified_backup).map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
            std::fs::rename(&tmp_backup_path, backup_path).map_err(|error| error.to_string())?;
            logger().info(&format!("Wrote coordinator backup to '{backup_path}'"), NS);
        }

        Ok(())
    }

    /// Upstream `coordinatorCheck()` — controller.ts:492.
    pub async fn coordinator_check(&self) -> Result<Vec<Arc<Device>>, String> {
        let adapter = self.adapter()?;
        if !adapter.supports_backup().await? {
            return Err(
                "Coordinator does not coordinator check because it doesn't support backups"
                    .to_string(),
            );
        }

        let backup = adapter.backup(&self.get_device_ieee_addresses()).await?;
        let devices_in_backup: Vec<String> = backup
            .devices
            .iter()
            .map(|device| eui64_be_buffer_to_hex(&device.ieee_address))
            .collect();

        let missing_routers = self.get_devices_iterator(|device| {
            device.r#type() == DeviceType::Router
                && !devices_in_backup.contains(&device.ieee_addr())
        });

        Ok(missing_routers)
    }

    /// Upstream `reset(type)`.
    pub async fn reset(&self, r#type: &str) -> Result<(), String> {
        self.adapter()?.reset(r#type).await
    }

    /// Upstream `getCoordinatorVersion()`.
    pub async fn get_coordinator_version(&self) -> Result<ControllerCoordinatorVersion, String> {
        self.adapter()?.get_coordinator_version().await
    }

    /// Upstream `getNetworkParameters()` — controller.ts:516.
    pub async fn get_network_parameters(&self) -> Result<ControllerNetworkParameters, String> {
        let mut cached = self.network_parameters_cached.lock().await;
        if cached.is_none() {
            *cached = Some(self.adapter()?.get_network_parameters_full().await?);
        }
        cached
            .clone()
            .ok_or_else(|| "Network parameters unavailable".to_string())
    }

    /// Upstream `getDevices()`.
    pub fn get_devices(&self) -> Vec<Arc<Device>> {
        Device::all()
    }

    /// Upstream `getDevicesIterator(predicate?)`.
    pub fn get_devices_iterator(
        &self,
        predicate: impl Fn(&Arc<Device>) -> bool,
    ) -> Vec<Arc<Device>> {
        Device::all_iterator(predicate)
    }

    /// Upstream `getDevicesByType(type)`.
    pub fn get_devices_by_type(&self, r#type: DeviceType) -> Vec<Arc<Device>> {
        Device::by_type(r#type)
    }

    /// Upstream `getDeviceByIeeeAddr(ieeeAddr)`.
    pub fn get_device_by_ieee_addr(&self, ieee_addr: &str) -> Option<Arc<Device>> {
        Device::by_ieee_addr(ieee_addr, false)
    }

    /// Upstream `getDeviceByNetworkAddress(networkAddress)`.
    pub fn get_device_by_network_address(&self, network_address: u16) -> Option<Arc<Device>> {
        Device::by_network_address(network_address, false)
    }

    /// Upstream `getDeviceIeeeAddresses()`.
    pub fn get_device_ieee_addresses(&self) -> Vec<String> {
        Device::all_iterator(|_| true)
            .into_iter()
            .map(|device| device.ieee_addr())
            .collect()
    }

    /// Upstream `getGroupByID(groupID)`.
    pub fn get_group_by_id(&self, group_id: u16) -> Option<Arc<Group>> {
        Group::by_group_id(group_id)
    }

    /// Upstream `getGroups()`.
    pub fn get_groups(&self) -> Vec<Arc<Group>> {
        Group::all()
    }

    /// Upstream `getGroupsIterator(predicate?)`.
    pub fn get_groups_iterator(&self, predicate: impl Fn(&Arc<Group>) -> bool) -> Vec<Arc<Group>> {
        Group::all_iterator(predicate)
    }

    /// Upstream `createGroup(groupID)`.
    pub fn create_group(&self, group_id: u16) -> Result<Arc<Group>, String> {
        Group::create(group_id)
    }

    /// Upstream `changeChannel(oldChannel, newChannel, nwkUpdateID)` — controller.ts:606.
    async fn change_channel(
        &self,
        old_channel: u8,
        new_channel: u8,
        nwk_update_id: u8,
    ) -> Result<(), String> {
        logger().warning(
            &format!("Changing channel from '{old_channel}' to '{new_channel}'"),
            NS,
        );

        let mut nwk_update_id = u32::from(nwk_update_id) + 1;
        if nwk_update_id > 0xff {
            nwk_update_id = 0;
        }

        let cluster_id = ClusterId::NWK_UPDATE_REQUEST;
        let adapter = self.adapter()?;
        let zdo_payload = BuffaloZdo::build_request(
            adapter.has_zdo_message_overhead(),
            cluster_id,
            &[
                json!([new_channel]),
                json!(0xfe),
                Value::Null,
                json!(nwk_update_id),
                Value::Null,
            ],
        );

        adapter
            .send_zdo(
                BLANK_EUI64,
                BroadcastAddress::SLEEPY as u16,
                cluster_id,
                &zdo_payload,
                true,
            )
            .await?;
        logger().info(&format!("Channel changed to '{new_channel}'"), NS);

        *self.network_parameters_cached.lock().await = None;
        // Wait for the broadcast to propagate and the adapter to actually change.
        wait(12000).await;

        Ok(())
    }

    /// Upstream `identifyUnknownDevice(nwkAddress)` — controller.ts:636.
    pub async fn identify_unknown_device(
        &self,
        nwk_address: u16,
    ) -> Result<Option<Arc<Device>>, String> {
        {
            let unknown = self
                .unknown_devices
                .lock()
                .expect("unknown devices poisoned");
            if unknown.contains(&nwk_address) {
                return Ok(None);
            }
        }

        logger().debug(
            &format!("Trying to identify unknown device with address '{nwk_address}'"),
            NS,
        );
        self.unknown_devices
            .lock()
            .expect("unknown devices poisoned")
            .insert(nwk_address);

        let cluster_id = ClusterId::IEEE_ADDRESS_REQUEST;
        let adapter = self.adapter()?;
        let zdo_payload = BuffaloZdo::build_request(
            adapter.has_zdo_message_overhead(),
            cluster_id,
            &[json!(nwk_address), json!(false), json!(0)],
        );

        match adapter
            .send_zdo(BLANK_EUI64, nwk_address, cluster_id, &zdo_payload, false)
            .await
        {
            Ok(response) => {
                if BuffaloZdo::check_status(&response) {
                    let payload = response.get(1).cloned().unwrap_or(Value::Null);
                    let eui64 = payload
                        .get("eui64")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    let device = Device::by_ieee_addr(&eui64, false);

                    if let Some(device) = &device {
                        let payload_nwk = payload
                            .get("nwkAddress")
                            .and_then(Value::as_u64)
                            .unwrap_or(u64::from(nwk_address))
                            as u16;
                        self.check_device_network_address(device, &eui64, payload_nwk);
                        self.unknown_devices
                            .lock()
                            .expect("unknown devices poisoned")
                            .remove(&payload_nwk);
                    }

                    return Ok(device);
                }

                logger().debug(
                    &format!(
                        "Failed to retrieve IEEE address for device '{nwk_address}': status {}",
                        response.get(0).and_then(Value::as_u64).unwrap_or(0)
                    ),
                    NS,
                );
            }
            Err(error) => {
                logger().debug(
                    &format!("Failed to retrieve IEEE address for device '{nwk_address}': {error}"),
                    NS,
                );
            }
        }

        // NOTE: keep nwkAddress in `unknown_devices` on failure to prevent spam
        // until the next reboot.
        Ok(None)
    }

    /// Upstream `checkDeviceNetworkAddress(device, ieeeAddress, nwkAddress)`.
    fn check_device_network_address(
        &self,
        device: &Arc<Device>,
        ieee_address: &str,
        nwk_address: u16,
    ) {
        if device.network_address() != nwk_address {
            logger().debug(
                &format!("Device '{ieee_address}' got new networkAddress '{nwk_address}'"),
                NS,
            );
            device.set_network_address(nwk_address);
            device.save(true);

            self.self_and_device_emit(ControllerEvent::DeviceNetworkAddressChanged(
                DeviceNetworkAddressChangedPayload {
                    device: device.clone(),
                },
            ));
        }
    }

    fn on_network_address(&self, payload: &Value) {
        let eui64 = payload
            .get("eui64")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let nwk_address = payload
            .get("nwkAddress")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u16;
        logger().debug(&format!("Network address from '{eui64}:{nwk_address}'"), NS);
        let Some(device) = Device::by_ieee_addr(&eui64, false) else {
            logger().debug(
                &format!("Network address is from unknown device '{eui64}:{nwk_address}'"),
                NS,
            );
            return;
        };

        device.update_last_seen();
        self.self_and_device_emit(ControllerEvent::LastSeenChanged(LastSeenChangedPayload {
            device: device.clone(),
            reason: "networkAddress".to_string(),
        }));
        self.check_device_network_address(&device, &eui64, nwk_address);
    }

    fn on_ieee_address(&self, payload: &Value) {
        let eui64 = payload
            .get("eui64")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let nwk_address = payload
            .get("nwkAddress")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u16;
        logger().debug(&format!("IEEE address from '{eui64}:{nwk_address}'"), NS);
        let Some(device) = Device::by_ieee_addr(&eui64, false) else {
            logger().debug(
                &format!("IEEE address is from unknown device '{eui64}:{nwk_address}'"),
                NS,
            );
            return;
        };

        device.update_last_seen();
        self.self_and_device_emit(ControllerEvent::LastSeenChanged(LastSeenChangedPayload {
            device: device.clone(),
            reason: "networkAddress".to_string(),
        }));
        self.check_device_network_address(&device, &eui64, nwk_address);
    }

    fn on_device_announce(&self, payload: &Value) {
        let eui64 = payload
            .get("eui64")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let nwk_address = payload
            .get("nwkAddress")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u16;
        logger().debug(&format!("Device announce from '{eui64}:{nwk_address}'"), NS);
        let Some(device) = Device::by_ieee_addr(&eui64, false) else {
            logger().debug(
                &format!("Device announce is from unknown device '{eui64}:{nwk_address}'"),
                NS,
            );
            return;
        };

        device.update_last_seen();
        self.self_and_device_emit(ControllerEvent::LastSeenChanged(LastSeenChangedPayload {
            device: device.clone(),
            reason: "deviceAnnounce".to_string(),
        }));
        device.implicit_checkin();
        self.check_device_network_address(&device, &eui64, nwk_address);
        device.reset_transient(false);
        self.self_and_device_emit(ControllerEvent::DeviceAnnounce(DeviceAnnouncePayload {
            device: device.clone(),
        }));
    }

    fn on_device_leave(&self, payload: &DeviceLeavePayload) {
        logger().debug(
            &format!(
                "Device leave '{}'",
                payload.ieee_addr.as_deref().unwrap_or("")
            ),
            NS,
        );

        let device = match &payload.ieee_addr {
            Some(ieee_addr) if !ieee_addr.is_empty() => Device::by_ieee_addr(ieee_addr, false),
            _ => payload
                .network_address
                .and_then(|address| Device::by_network_address(address, false)),
        };

        let Some(device) = device else {
            logger().debug(
                &format!(
                    "Device leave is from unknown or already deleted device '{}'",
                    payload
                        .ieee_addr
                        .clone()
                        .or_else(|| payload.network_address.map(|address| address.to_string()))
                        .unwrap_or_default()
                ),
                NS,
            );
            return;
        };

        logger().debug(
            &format!("Removing device from database '{}'", device.ieee_addr()),
            NS,
        );
        device.remove_from_database(false);

        self.self_and_device_emit(ControllerEvent::DeviceLeave(ModelDeviceLeavePayload {
            ieee_addr: device.ieee_addr(),
        }));
    }

    async fn on_adapter_disconnected(&self) -> Result<(), String> {
        logger().debug("Adapter disconnected", NS);
        self.adapter_disconnected.store(true, Ordering::SeqCst);

        if let Ok(adapter) = self.adapter() {
            if let Err(error) = adapter.stop().await {
                logger().error(
                    &format!("Failed to stop adapter on disconnect: {error}"),
                    NS,
                );
            }
        }

        self.emit(&ControllerEvent::AdapterDisconnected);
        Ok(())
    }

    fn on_device_joined_green_power(&self, payload: GreenPowerDeviceJoinedPayload) {
        logger().debug(
            &format!(
                "Green power device '{}' joined",
                json!({
                    "sourceID": payload.source_id,
                    "deviceID": payload.device_id,
                    "networkAddress": payload.network_address,
                    "securityKey": payload.security_key,
                })
                .to_string()
                .replace('"', "")
            ),
            NS,
        );

        let ieee_addr = GreenPower::source_id_to_ieee_address(payload.source_id);
        let model_id = format!("GreenPower_{}", payload.device_id);
        let mut device = Device::by_ieee_addr(&ieee_addr, true);

        if device.is_none() {
            logger().debug(&format!("New green power device '{ieee_addr}' joined"), NS);
            logger().debug(&format!("Creating device '{ieee_addr}'"), NS);
            device = Device::create(
                DeviceType::GreenPower,
                &ieee_addr,
                payload.network_address,
                None,
                None,
                None,
                Some(model_id),
                InterviewState::Successful,
                payload.security_key.clone(),
            )
            .ok();

            if let Some(device) = &device {
                device.save(true);
                self.self_and_device_emit(ControllerEvent::DeviceJoined(
                    ModelDeviceJoinedPayload {
                        device: device.clone(),
                    },
                ));
                self.self_and_device_emit(ControllerEvent::DeviceInterview(
                    DeviceInterviewPayload {
                        status: "successful".to_string(),
                        device: device.clone(),
                    },
                ));
            }
        } else if let Some(existing) = &device {
            if existing.is_deleted() {
                logger().debug(
                    &format!("Deleted green power device '{ieee_addr}' joined, undeleting"),
                    NS,
                );
                let _ = existing.undelete();
                self.self_and_device_emit(ControllerEvent::DeviceJoined(
                    ModelDeviceJoinedPayload {
                        device: existing.clone(),
                    },
                ));
                self.self_and_device_emit(ControllerEvent::DeviceInterview(
                    DeviceInterviewPayload {
                        status: "successful".to_string(),
                        device: existing.clone(),
                    },
                ));
            }
        }
    }

    fn on_device_leave_green_power(&self, source_id: u32) {
        logger().debug(&format!("Green power device '{source_id}' left"), NS);

        let ieee_addr = GreenPower::source_id_to_ieee_address(source_id);
        let Some(device) = Device::by_ieee_addr(&ieee_addr, false) else {
            logger().debug(
                &format!(
                    "Green power device leave is from unknown or already deleted device '{ieee_addr}'"
                ),
                NS,
            );
            return;
        };

        logger().debug(
            &format!(
                "Removing green power device from database '{}'",
                device.ieee_addr()
            ),
            NS,
        );
        device.remove_from_database(false);

        self.self_and_device_emit(ControllerEvent::DeviceLeave(ModelDeviceLeavePayload {
            ieee_addr: device.ieee_addr(),
        }));
    }

    async fn on_device_joined(&self, payload: &DeviceJoinedPayload) -> Result<(), String> {
        logger().debug(&format!("Device '{}' joined", payload.ieee_addr), NS);

        if let Some(handler) = &self.options.accept_joining_device_handler {
            let accepted = handler(payload.ieee_addr.clone()).await?;
            if !accepted {
                logger().debug(
                    &format!(
                        "Device '{}' rejected by handler, removing it",
                        payload.ieee_addr
                    ),
                    NS,
                );

                let cluster_id = ClusterId::LEAVE_REQUEST;
                let adapter = self.adapter()?;
                let zdo_payload = BuffaloZdo::build_request(
                    adapter.has_zdo_message_overhead(),
                    cluster_id,
                    &[
                        json!(payload.ieee_addr),
                        json!(crate::zspec::zdo::definition::consts::WITHOUT_REJOIN),
                    ],
                );
                let response = adapter
                    .send_zdo(
                        &payload.ieee_addr,
                        payload.network_address,
                        cluster_id,
                        &zdo_payload,
                        false,
                    )
                    .await;

                match response {
                    Ok(response) if BuffaloZdo::check_status(&response) => {}
                    Ok(response) => {
                        logger().error(
                            &format!(
                                "Failed to remove rejected device: status {}",
                                response.get(0).and_then(Value::as_u64).unwrap_or(0)
                            ),
                            NS,
                        );
                    }
                    Err(error) => {
                        logger().error(&format!("Failed to remove rejected device: {error}"), NS);
                    }
                }

                return Ok(());
            }

            logger().debug(
                &format!("Device '{}' accepted by handler", payload.ieee_addr),
                NS,
            );
        }

        let mut device = Device::by_ieee_addr(&payload.ieee_addr, true);
        if device.is_none() {
            logger().debug(&format!("New device '{}' joined", payload.ieee_addr), NS);
            logger().debug(&format!("Creating device '{}'", payload.ieee_addr), NS);
            device = Device::create(
                DeviceType::Unknown,
                &payload.ieee_addr,
                payload.network_address,
                None,
                None,
                None,
                None,
                InterviewState::Pending,
                None,
            )
            .ok();

            if let Some(device) = &device {
                device.reset_transient(true);
                self.self_and_device_emit(ControllerEvent::DeviceJoined(
                    ModelDeviceJoinedPayload {
                        device: device.clone(),
                    },
                ));
            }
        } else if let Some(existing) = &device {
            if existing.is_deleted() {
                logger().debug(
                    &format!("Deleted device '{}' joined, undeleting", payload.ieee_addr),
                    NS,
                );
                let _ = existing.undelete();
                existing.reset_transient(true);
                self.self_and_device_emit(ControllerEvent::DeviceJoined(
                    ModelDeviceJoinedPayload {
                        device: existing.clone(),
                    },
                ));
            }
        }

        let Some(device) = device else {
            return Ok(());
        };

        if device.network_address() != payload.network_address {
            logger().debug(
                &format!(
                    "Device '{}' is already in database with different network address, updating network address",
                    payload.ieee_addr
                ),
                NS,
            );
            device.set_network_address(payload.network_address);
            device.save(true);
        }

        device.update_last_seen();
        self.self_and_device_emit(ControllerEvent::LastSeenChanged(LastSeenChangedPayload {
            device: device.clone(),
            reason: "deviceJoined".to_string(),
        }));
        device.implicit_checkin();

        if matches!(
            device.interview_state(),
            InterviewState::Pending | InterviewState::Failed
        ) {
            logger().info(
                &format!("Interview for '{}' started", device.ieee_addr()),
                NS,
            );
            self.self_and_device_emit(ControllerEvent::DeviceInterview(DeviceInterviewPayload {
                status: "started".to_string(),
                device: device.clone(),
            }));

            match device.interview(false).await {
                Ok(()) => {
                    logger().info(
                        &format!("Succesfully interviewed '{}'", device.ieee_addr()),
                        NS,
                    );
                    self.self_and_device_emit(ControllerEvent::DeviceInterview(
                        DeviceInterviewPayload {
                            status: "successful".to_string(),
                            device: device.clone(),
                        },
                    ));
                }
                Err(error) => {
                    logger().error(
                        &format!(
                            "Interview failed for '{} with error '{error}'",
                            device.ieee_addr()
                        ),
                        NS,
                    );
                    self.self_and_device_emit(ControllerEvent::DeviceInterview(
                        DeviceInterviewPayload {
                            status: "failed".to_string(),
                            device: device.clone(),
                        },
                    ));
                }
            }
        } else {
            logger().debug(
                &format!(
                    "Not interviewing '{}', interviewState={:?}'",
                    payload.ieee_addr,
                    device.interview_state()
                ),
                NS,
            );
        }

        Ok(())
    }

    fn on_zdo_response(&self, cluster_id: u16, response: &Value) {
        logger().debug(
            &format!(
                "Received ZDO response: clusterId={cluster_id}, status={}, payload={}",
                response.get(0).and_then(Value::as_u64).unwrap_or(0),
                response.get(1).cloned().unwrap_or(Value::Null)
            ),
            NS,
        );

        if !BuffaloZdo::check_status(response) {
            return;
        }

        let Some(payload) = response.get(1) else {
            return;
        };
        match cluster_id {
            ClusterId::NETWORK_ADDRESS_RESPONSE => self.on_network_address(payload),
            ClusterId::IEEE_ADDRESS_RESPONSE => self.on_ieee_address(payload),
            ClusterId::END_DEVICE_ANNOUNCE => self.on_device_announce(payload),
            _ => {}
        }
    }

    /// Upstream `onZclPayload(payload)` — controller.ts:937.
    async fn on_zcl_payload(&self, payload: ZclPayload) -> Result<(), String> {
        let mut frame: Option<Frame> = None;
        let mut device: Option<Arc<Device>> = None;
        let mut default_response: Option<u8> = None;

        let touchlink_id = Utils::get_cluster(
            ClusterKey::Name("touchlink".to_string()),
            None,
            &CustomClusters::empty(),
        )
        .ID as u16;
        let green_power_id = 0x0021;

        if payload.cluster_id == touchlink_id {
            return Ok(());
        }

        if payload.cluster_id == green_power_id {
            frame = match Frame::try_from_buffer(
                payload.cluster_id,
                Some(payload.header),
                &payload.data,
                &CustomClusters::empty(),
            ) {
                Ok(frame) => Some(frame),
                Err(error) => {
                    logger().debug(
                        &format!("Failed to parse frame green power frame, ignoring it: {error}"),
                        NS,
                    );
                    return Ok(());
                }
            };

            let frame_value = frame.as_ref().unwrap();
            if frame_value.payload.get("commandID").is_none() {
                device = Device::find(payload.address.clone(), false);
            } else {
                let Some(source_id) = frame_value.payload.get("srcID").and_then(Value::as_u64)
                else {
                    logger().debug(
                        "Data is from unsupported green power device with IEEE addressing, skipping...",
                        NS,
                    );
                    return Ok(());
                };

                let ieee_addr = GreenPower::source_id_to_ieee_address(source_id as u32);
                device = Device::by_ieee_addr(&ieee_addr, false);
                let security_key = device.as_ref().and_then(|device| device.gp_security_key());
                let processed = if let Some(green_power) = self.green_power() {
                    green_power
                        .process_command(&payload, frame.take().unwrap(), security_key.as_deref())
                        .await
                } else {
                    frame.take().unwrap()
                };
                frame = Some(processed);

                // lookup encapsulated gpDevice for further processing (re-fetch)
                device = Device::by_ieee_addr(&ieee_addr, false);
                if device.is_none() {
                    logger().debug(
                        &format!(
                            "Data is from unknown green power device with address '{ieee_addr}' ({source_id}), skipping..."
                        ),
                        NS,
                    );
                    return Ok(());
                }
            }
        } else {
            device = Device::find(payload.address.clone(), false);

            if let Some(found) = &device {
                if found.manufacturer_name().as_deref() == Some("LUMI")
                    && found.r#type() == DeviceType::Router
                    && payload.group_id != 0
                {
                    logger().debug(
                        &format!(
                            "Handling re-transmitted Xiaomi message {} -> {}",
                            found.network_address(),
                            payload.group_id
                        ),
                        NS,
                    );
                    device = Device::by_network_address(payload.group_id, false);
                }
            }

            let custom_clusters = device
                .as_ref()
                .map(|device| device.custom_clusters())
                .unwrap_or_else(CustomClusters::empty);
            match Frame::try_from_buffer(
                payload.cluster_id,
                Some(payload.header),
                &payload.data,
                &custom_clusters,
            ) {
                Ok(parsed) => frame = Some(parsed),
                Err(error) => {
                    logger().debug(&format!("Failed to parse frame: {error}"), NS);
                }
            }
        }

        if device.is_none() {
            if let Some(address) = payload.address.as_u64() {
                if !Device::is_deleted_by_network_address(address as u16) {
                    device = self.identify_unknown_device(address as u16).await?;
                }
            }
        }

        let Some(device) = device else {
            logger().debug(
                &format!(
                    "Data is from unknown device with address '{}', skipping...",
                    payload.address
                ),
                NS,
            );
            return Ok(());
        };

        logger().debug(
            &format!(
                "Received payload: clusterID={}, address={}, groupID={}, endpoint={}, destinationEndpoint={}, wasBroadcast={}, linkQuality={}, frame={}",
                payload.cluster_id,
                payload.address,
                payload.group_id,
                payload.endpoint,
                payload.destination_endpoint,
                payload.was_broadcast,
                payload.linkquality,
                frame
                    .as_ref()
                    .map(Frame::to_string)
                    .unwrap_or_else(|| "undefined".to_string())
            ),
            NS,
        );

        device.update_last_seen();

        let is_poll_ctrl = frame
            .as_ref()
            .map(|frame| frame.cluster.name == "genPollCtrl")
            .unwrap_or(false);
        if !is_poll_ctrl {
            device.implicit_checkin();
        }

        device.set_linkquality(payload.linkquality);
        let mut endpoint = device.get_endpoint(payload.endpoint);
        if endpoint.is_none() {
            logger().debug(
                &format!(
                    "Data is from unknown endpoint '{}' from device with network address '{}', creating it...",
                    payload.endpoint, payload.address
                ),
                NS,
            );
            endpoint = Some(device.create_endpoint(payload.endpoint)?);
        }
        let endpoint = endpoint.expect("endpoint created");

        let mut message_type: Option<String> = None;
        let mut data = json!({});
        let mut cluster_name = Value::Null;
        let mut meta = MessageMeta {
            zcl_transaction_sequence_number: None,
            manufacturer_code: None,
            frame_control: None,
            raw_data: payload.data.clone(),
        };

        if let Some(frame) = &frame {
            let command = &frame.command;
            cluster_name = json!(frame.cluster.name);
            meta.zcl_transaction_sequence_number = Some(frame.header.transaction_sequence_number);
            meta.manufacturer_code = frame.header.manufacturer_code;
            meta.frame_control = Some(frame.header.frame_control);

            if frame.header.is_global() {
                match command.name.as_str() {
                    "report" => {
                        message_type = Some("attributeReport".to_string());
                        data = attribute_key_value(
                            frame,
                            device.manufacturer_id(),
                            &device.custom_clusters(),
                        );
                    }
                    "read" => {
                        message_type = Some("read".to_string());
                        data = Value::Array(attribute_list(
                            frame,
                            device.manufacturer_id(),
                            &device.custom_clusters(),
                        ));
                    }
                    "write" => {
                        message_type = Some("write".to_string());
                        data = attribute_key_value(
                            frame,
                            device.manufacturer_id(),
                            &device.custom_clusters(),
                        );
                    }
                    "readRsp" => {
                        message_type = Some("readResponse".to_string());
                        data = attribute_key_value(
                            frame,
                            device.manufacturer_id(),
                            &device.custom_clusters(),
                        );
                    }
                    "defaultRsp" => {
                        let status = frame
                            .payload
                            .get("statusCode")
                            .and_then(Value::as_u64)
                            .unwrap_or(0);
                        if status != 0 {
                            logger().debug(
                                &format!(
                                    "Failure default response from '{}': clusterID={} cmdId={} status={status}",
                                    payload.address,
                                    payload.cluster_id,
                                    frame
                                        .payload
                                        .get("cmdId")
                                        .and_then(Value::as_u64)
                                        .unwrap_or(0)
                                ),
                                NS,
                            );
                        }
                    }
                    _ => {}
                }

                if message_type.as_deref() == Some("readResponse")
                    || message_type.as_deref() == Some("attributeReport")
                {
                    if frame.cluster.name == "genBasic" {
                        device.update_gen_basic(data.clone());
                    }
                    endpoint
                        .save_cluster_attribute_key_value(json!(frame.cluster.ID), data.clone());
                }
            } else if frame.header.is_specific() {
                let mut chars = command.name.chars();
                let first = chars
                    .next()
                    .map(|char| char.to_uppercase().to_string())
                    .unwrap_or_default();
                message_type = Some(format!("command{first}{}", chars.as_str()));
                data = frame.payload.clone();
            }
        } else {
            message_type = Some("raw".to_string());
            data = json!(payload.data);
            let cluster = Utils::get_cluster(
                payload.cluster_id,
                device.manufacturer_id(),
                &device.custom_clusters(),
            );
            cluster_name = match cluster.name.parse::<u64>() {
                Ok(number) => json!(number),
                Err(_) => json!(cluster.name),
            };
        }

        if let Some(message_type) = message_type {
            let linkquality = payload.linkquality;
            let group_id = payload.group_id;
            self.self_and_device_emit(ControllerEvent::Message(MessagePayload {
                r#type: message_type,
                device: device.clone(),
                endpoint: endpoint.clone(),
                linkquality,
                group_id,
                cluster: cluster_name,
                data,
                meta,
            }));
            self.self_and_device_emit(ControllerEvent::LastSeenChanged(LastSeenChangedPayload {
                device: device.clone(),
                reason: "messageEmitted".to_string(),
            }));
        } else {
            self.self_and_device_emit(ControllerEvent::LastSeenChanged(LastSeenChangedPayload {
                device: device.clone(),
                reason: "messageNonEmitted".to_string(),
            }));
        }

        device
            .on_zcl_data(&payload, frame.as_ref(), &endpoint, default_response)
            .await
    }

    /// Entry point for the adapter event task.
    pub async fn handle_adapter_event(&self, event: ControllerAdapterEvent) -> Result<(), String> {
        match event {
            ControllerAdapterEvent::Disconnected => self.on_adapter_disconnected().await,
            ControllerAdapterEvent::DeviceJoined(payload) => self.on_device_joined(&payload).await,
            ControllerAdapterEvent::ZclPayload(payload) => self.on_zcl_payload(payload).await,
            ControllerAdapterEvent::ZdoResponse {
                cluster_id,
                response,
            } => {
                self.on_zdo_response(cluster_id, &response);
                Ok(())
            }
            ControllerAdapterEvent::DeviceLeave(payload) => {
                self.on_device_leave(&payload);
                Ok(())
            }
        }
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        for timer in self
            .timers
            .lock()
            .expect("controller timers poisoned")
            .drain(..)
        {
            timer.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::events::{DeviceJoinedPayload as AdapterDeviceJoined, ZclPayload};
    use crate::controller::model::entity::ControllerAdapter;
    use crate::zspec::zcl::zcl_header::{FrameControl, Header};
    use serde_json::json;

    struct FakeRuntimeAdapter {
        events: Mutex<Option<mpsc::UnboundedReceiver<ControllerAdapterEvent>>>,
        event_sender: mpsc::UnboundedSender<ControllerAdapterEvent>,
        sent_all: Mutex<Vec<Frame>>,
        sent_endpoint: Mutex<Vec<Frame>>,
        sent_group: Mutex<Vec<Frame>>,
        permit_joins: Mutex<Vec<(u8, Option<u16>)>>,
        channel: u8,
        coordinator_ieee: String,
        stopped: AtomicBool,
    }

    impl FakeRuntimeAdapter {
        fn new(channel: u8) -> Self {
            let (event_sender, receiver) = mpsc::unbounded_channel();
            Self {
                events: Mutex::new(Some(receiver)),
                event_sender,
                sent_all: Mutex::new(Vec::new()),
                sent_endpoint: Mutex::new(Vec::new()),
                sent_group: Mutex::new(Vec::new()),
                permit_joins: Mutex::new(Vec::new()),
                channel,
                coordinator_ieee: "0x00124b0000000001".to_string(),
                stopped: AtomicBool::new(false),
            }
        }
    }

    impl ControllerAdapter for FakeRuntimeAdapter {
        fn has_zdo_message_overhead(&self) -> bool {
            false
        }

        fn send_zdo(
            &self,
            _ieee_address: &str,
            _network_address: u16,
            cluster_id: u16,
            _payload: &[u8],
            _disable_response: bool,
        ) -> ControllerFuture<'_, Value> {
            let response = match cluster_id {
                ClusterId::ACTIVE_ENDPOINTS_REQUEST => json!([0, {"endpointList": [1]}]),
                ClusterId::SIMPLE_DESCRIPTOR_REQUEST => json!([
                    0,
                    {
                        "profileId": 260,
                        "deviceId": 5,
                        "inClusterList": [],
                        "outClusterList": []
                    }
                ]),
                ClusterId::IEEE_ADDRESS_REQUEST => json!([
                    0,
                    {"eui64": "0x00124b0000000002", "nwkAddress": 0x1234}
                ]),
                _ => json!([0, {}]),
            };
            Box::pin(async move { Ok(response) })
        }

        fn send_zcl_frame_to_endpoint(
            &self,
            _ieee_address: &str,
            _network_address: u16,
            _endpoint: u8,
            frame: &Frame,
            _timeout: u64,
            _disable_response: bool,
            _disable_recovery: bool,
            _source_endpoint: Option<u8>,
            _profile_id: Option<u16>,
        ) -> ControllerFuture<'_, Option<ZclPayload>> {
            self.sent_endpoint.lock().unwrap().push(frame.clone());
            Box::pin(async move { Ok(None) })
        }

        fn send_zcl_frame_to_group(
            &self,
            _group_id: u16,
            frame: &Frame,
            _source_endpoint: Option<u8>,
            _profile_id: Option<u16>,
        ) -> ControllerFuture<'_, ()> {
            self.sent_group.lock().unwrap().push(frame.clone());
            Box::pin(async move { Ok(()) })
        }

        fn send_zcl_frame_to_all(
            &self,
            _endpoint: u8,
            frame: &Frame,
            _source_endpoint: u8,
            _destination: u16,
            _profile_id: Option<u16>,
        ) -> ControllerFuture<'_, ()> {
            self.sent_all.lock().unwrap().push(frame.clone());
            Box::pin(async move { Ok(()) })
        }
    }

    impl GreenPowerAdapter for FakeRuntimeAdapter {
        fn get_coordinator_ieee(&self) -> ControllerFuture<'_, String> {
            let ieee = self.coordinator_ieee.clone();
            Box::pin(async move { Ok(ieee) })
        }

        fn get_network_parameters(&self) -> ControllerFuture<'_, GreenPowerNetworkParameters> {
            let channel = self.channel;
            Box::pin(async move { Ok(GreenPowerNetworkParameters { channel }) })
        }

        fn send_zcl_frame_to_all(
            &self,
            endpoint: u8,
            frame: &Frame,
            source_endpoint: u8,
            destination: u16,
            profile_id: Option<u16>,
        ) -> ControllerFuture<'_, ()> {
            <Self as ControllerAdapter>::send_zcl_frame_to_all(
                self,
                endpoint,
                frame,
                source_endpoint,
                destination,
                profile_id,
            )
        }

        fn send_zcl_frame_to_endpoint(
            &self,
            ieee_address: &str,
            network_address: u16,
            endpoint: u8,
            frame: &Frame,
            timeout: u64,
            disable_response: bool,
            disable_recovery: bool,
            source_endpoint: Option<u8>,
            profile_id: Option<u16>,
        ) -> ControllerFuture<'_, Option<ZclPayload>> {
            <Self as ControllerAdapter>::send_zcl_frame_to_endpoint(
                self,
                ieee_address,
                network_address,
                endpoint,
                frame,
                timeout,
                disable_response,
                disable_recovery,
                source_endpoint,
                profile_id,
            )
        }
    }

    impl TouchlinkAdapter for FakeRuntimeAdapter {
        fn set_channel_inter_pan(&self, _channel: u8) -> ControllerFuture<'_, ()> {
            Box::pin(async move { Ok(()) })
        }

        fn restore_channel_inter_pan(&self) -> ControllerFuture<'_, ()> {
            Box::pin(async move { Ok(()) })
        }

        fn send_zcl_frame_inter_pan_broadcast(
            &self,
            _frame: &Frame,
            _timeout: u64,
            _disable_response: bool,
        ) -> ControllerFuture<'_, Option<ZclPayload>> {
            Box::pin(async move { Ok(None) })
        }

        fn send_zcl_frame_inter_pan_to_ieee_addr(
            &self,
            _frame: &Frame,
            _ieee_address: &str,
        ) -> ControllerFuture<'_, ()> {
            Box::pin(async move { Ok(()) })
        }
    }

    impl ControllerRuntimeAdapter for FakeRuntimeAdapter {
        fn start(&self) -> ControllerFuture<'_, StartResult> {
            Box::pin(async move { Ok(StartResult::Resumed) })
        }

        fn stop(&self) -> ControllerFuture<'_, ()> {
            self.stopped.store(true, Ordering::SeqCst);
            Box::pin(async move { Ok(()) })
        }

        fn subscribe(&self) -> mpsc::UnboundedReceiver<ControllerAdapterEvent> {
            self.events
                .lock()
                .unwrap()
                .take()
                .expect("adapter subscribed twice")
        }

        fn permit_join(&self, time: u8, network_address: Option<u16>) -> ControllerFuture<'_, ()> {
            self.permit_joins
                .lock()
                .unwrap()
                .push((time, network_address));
            Box::pin(async move { Ok(()) })
        }

        fn get_network_parameters_full(&self) -> ControllerFuture<'_, ControllerNetworkParameters> {
            let channel = self.channel;
            Box::pin(async move {
                Ok(ControllerNetworkParameters {
                    pan_id: 0x1a62,
                    extended_pan_id: "0xdddddddddddddddd".to_string(),
                    channel,
                    nwk_update_id: 0,
                })
            })
        }

        fn get_coordinator_version(&self) -> ControllerFuture<'_, ControllerCoordinatorVersion> {
            Box::pin(async move {
                Ok(ControllerCoordinatorVersion {
                    r#type: "zStack".to_string(),
                    meta: json!({}),
                })
            })
        }

        fn supports_backup(&self) -> ControllerFuture<'_, bool> {
            Box::pin(async move { Ok(false) })
        }

        fn backup(&self, _ieee_addresses: &[String]) -> ControllerFuture<'_, Backup> {
            Box::pin(async move { Ok(Backup::default()) })
        }

        fn reset(&self, _kind: &str) -> ControllerFuture<'_, ()> {
            Box::pin(async move { Ok(()) })
        }

        fn add_install_code(
            &self,
            _ieee_address: &str,
            _key: &[u8],
            _hashed: bool,
        ) -> ControllerFuture<'_, ()> {
            Box::pin(async move { Ok(()) })
        }

        fn manufacturer_id(&self) -> Option<u16> {
            None
        }
    }

    fn options(database_path: &str, channel: u8) -> ControllerOptions {
        ControllerOptions {
            network: NetworkOptions {
                pan_id: 0x1a62,
                extended_pan_id: vec![0xdd; 8],
                channel_list: vec![channel],
                network_key: vec![
                    0x01, 0x03, 0x05, 0x07, 0x09, 0x0b, 0x0d, 0x0f, 0x00, 0x02, 0x04, 0x06, 0x08,
                    0x0a, 0x0c, 0x0d,
                ],
                network_key_distribute: false,
                has_default_extended_pan_id: false,
            },
            transport: json!({}),
            database_path: database_path.to_string(),
            database_backup_path: None,
            backup_path: None,
            adapter: json!({}),
            accept_joining_device_handler: None,
        }
    }

    fn message_frame() -> Frame {
        Frame::create(
            0,
            Direction::CLIENT_TO_SERVER as u8,
            false,
            None,
            7,
            "report",
            "genOnOff",
            json!([{"attrId": 0, "attrData": true, "dataType": 0x10}]),
            json!({}),
            0,
        )
    }

    #[test]
    fn rejects_invalid_options() {
        let mut bad_channel = options("/tmp/opencode/controller-invalid.db", 9);
        bad_channel.network.channel_list = vec![9];
        assert!(Controller::new(bad_channel)
            .err()
            .unwrap()
            .contains("invalid channel"));

        let mut bad_key = options("/tmp/opencode/controller-invalid.db", 15);
        bad_key.network.network_key = vec![0; 8];
        assert!(Controller::new(bad_key)
            .err()
            .unwrap()
            .contains("Network key"));

        let mut bad_pan = options("/tmp/opencode/controller-invalid.db", 15);
        bad_pan.network.pan_id = 0;
        assert!(Controller::new(bad_pan).err().unwrap().contains("PanID"));
    }

    #[tokio::test]
    async fn start_permit_join_and_stop() {
        let _lock = crate::controller::model::test_support::serial();
        let path = std::env::temp_dir().join(format!(
            "z2m-controller-{}.db",
            crate::utils::datetime::now_ms()
        ));
        Device::reset_cache();
        Group::reset_cache();

        let controller = Arc::new(Controller::new(options(path.to_str().unwrap(), 15)).unwrap());
        let adapter = Arc::new(FakeRuntimeAdapter::new(15));
        let events: Arc<Mutex<Vec<ControllerEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let captured = events.clone();
        controller.on(move |event| captured.lock().unwrap().push(event.clone()));

        let started = controller.start(adapter.clone()).await.expect("start");
        assert_eq!(started, StartResult::Resumed);

        let coordinator = Device::by_type(DeviceType::Coordinator);
        assert_eq!(coordinator.len(), 1);
        assert_eq!(coordinator[0].ieee_addr(), "0x00124b0000000001");
        assert_eq!(coordinator[0].network_address(), COORDINATOR_ADDRESS);
        assert!(coordinator[0].get_endpoint(1).is_some());

        controller.permit_join(60, None).await.expect("permit join");
        assert!(controller.get_permit_join());
        assert!(controller.get_permit_join_end().is_some());
        assert_eq!(
            adapter.permit_joins.lock().unwrap().as_slice(),
            &[(60, None)]
        );
        {
            let sent = adapter.sent_all.lock().unwrap();
            assert!(sent
                .iter()
                .any(|frame| frame.command.name == "commisioningMode"));
        }
        {
            let events = events.lock().unwrap();
            assert!(events.iter().any(|event| matches!(
                event,
                ControllerEvent::PermitJoinChanged(payload) if payload.permitted
            )));
        }

        controller.stop().await.expect("stop");
        assert!(controller.is_stopping());
        assert!(controller.is_adapter_disconnected());
        assert!(adapter.stopped.load(Ordering::SeqCst));

        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn zcl_report_emits_message_event() {
        let _lock = crate::controller::model::test_support::serial();
        let path = std::env::temp_dir().join(format!(
            "z2m-controller-zcl-{}.db",
            crate::utils::datetime::now_ms()
        ));
        Device::reset_cache();
        Group::reset_cache();

        let controller = Arc::new(Controller::new(options(path.to_str().unwrap(), 15)).unwrap());
        let adapter = Arc::new(FakeRuntimeAdapter::new(15));
        let events: Arc<Mutex<Vec<ControllerEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let captured = events.clone();
        controller.on(move |event| captured.lock().unwrap().push(event.clone()));
        controller.start(adapter.clone()).await.expect("start");

        let device = Device::create(
            DeviceType::Router,
            "0x00124b0000000002",
            0x1234,
            Some(0x1234),
            Some("Test".to_string()),
            Some("Mains (single phase)".to_string()),
            Some("TEST".to_string()),
            InterviewState::Successful,
            None,
        )
        .expect("create device");
        device.create_endpoint(1).expect("create endpoint");
        device.save(true);

        let frame = message_frame();
        let data = frame.to_buffer();
        let zcl_payload = ZclPayload {
            cluster_id: 0x0006,
            data: data.clone(),
            header: Header::from_buffer(&data).expect("header"),
            address: json!(0x1234),
            endpoint: 1,
            linkquality: 120,
            group_id: 0,
            was_broadcast: false,
            destination_endpoint: 1,
        };

        controller
            .handle_adapter_event(ControllerAdapterEvent::ZclPayload(zcl_payload))
            .await
            .expect("zcl payload");

        let events = events.lock().unwrap();
        let message = events
            .iter()
            .find_map(|event| match event {
                ControllerEvent::Message(payload) => Some(payload),
                _ => None,
            })
            .expect("message event");
        assert_eq!(message.r#type, "attributeReport");
        assert_eq!(message.cluster, json!("genOnOff"));
        assert_eq!(message.data, json!({"onOff": 1}));
        assert_eq!(message.linkquality, 120);
        assert_eq!(message.meta.zcl_transaction_sequence_number, Some(7));
        assert!(events.iter().any(|event| matches!(
            event,
            ControllerEvent::LastSeenChanged(payload) if payload.reason == "messageEmitted"
        )));

        drop(events);
        controller.stop().await.expect("stop");
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn device_joined_creates_and_interviews_unknown_device() {
        let _lock = crate::controller::model::test_support::serial();
        let path = std::env::temp_dir().join(format!(
            "z2m-controller-join-{}.db",
            crate::utils::datetime::now_ms()
        ));
        Device::reset_cache();
        Group::reset_cache();

        let controller = Arc::new(Controller::new(options(path.to_str().unwrap(), 15)).unwrap());
        let adapter = Arc::new(FakeRuntimeAdapter::new(15));
        let events: Arc<Mutex<Vec<ControllerEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let captured = events.clone();
        controller.on(move |event| captured.lock().unwrap().push(event.clone()));
        controller.start(adapter.clone()).await.expect("start");

        controller
            .handle_adapter_event(ControllerAdapterEvent::DeviceJoined(AdapterDeviceJoined {
                network_address: 0x2345,
                ieee_addr: "0x00124b0000000003".to_string(),
            }))
            .await
            .expect("device joined");

        let device = Device::by_ieee_addr("0x00124b0000000003", false).expect("device");
        assert_eq!(device.network_address(), 0x2345);
        assert_eq!(device.interview_state(), InterviewState::Failed);

        let events = events.lock().unwrap();
        assert!(events
            .iter()
            .any(|event| matches!(event, ControllerEvent::DeviceJoined(_))));
        assert!(events.iter().any(|event| matches!(
            event,
            ControllerEvent::DeviceInterview(payload) if payload.status == "started"
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            ControllerEvent::DeviceInterview(payload) if payload.status == "failed"
        )));

        drop(events);
        controller.stop().await.expect("stop");
        let _ = std::fs::remove_file(path);
    }
}
