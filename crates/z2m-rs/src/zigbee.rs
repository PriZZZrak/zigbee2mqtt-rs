// Original: .upstream/zigbee2mqtt/lib/zigbee.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// Bridge-side wrapper around the zigbee-herdsman controller. The shared
// converter host is ported (`converters/`); R1 registers no converter database,
// so `resolveDefinition()` returns the upstream "not found" fallback and
// devices stay definition-less (AGENTS §5 Q4). Touchlink methods are outside
// the R1 surface (§4) and are not ported.

use std::sync::{Arc, Mutex, OnceLock, Weak};

use serde_json::{json, Value};
use zigbee_apiarist::adapter::transport::{AdapterTransport, TransportOptions};
use zigbee_apiarist::adapter::z_stack::adapter::z_stack_adapter::ZStackAdapter;
use zigbee_apiarist::adapter::z_stack::runtime_adapter::ZStackRuntimeAdapter;
use zigbee_apiarist::controller::controller::{
    Controller, ControllerOptions, ControllerRuntimeAdapter, StartResult,
};
use zigbee_apiarist::controller::database::DeviceType;
use zigbee_apiarist::controller::events::{ControllerEvent, MessagePayload};
use zigbee_apiarist::controller::model::device::Device as ZhDevice;
use zigbee_apiarist::controller::model::endpoint::Endpoint as ZhEndpoint;
use zigbee_apiarist::models::network_options::NetworkOptions;
use zigbee_apiarist::models::startup_options::{AdapterOptions, ConfigNetworkOptions};

use crate::event_bus::EventBus;
use crate::model::device::Device;
use crate::model::entity::Entity;
use crate::model::group::{Group, ResolveDevice};
use crate::util::abort::AbortSignal;
use crate::util::data;
use crate::util::logger::Logger;
use crate::util::settings::Settings;
use crate::util::stringify::stringify;
use zigbee_converters::DefinitionRegistry;

/// Upstream `entityIDRegex` (`zigbee.ts:14`).
fn split_entity_and_endpoint(id: &str) -> Option<(&str, &str)> {
    let index = id.rfind('/')?;
    let endpoint = &id[index + 1..];
    if endpoint.is_empty() {
        return None;
    }
    Some((&id[..index], endpoint))
}

/// Upstream `resolveEntityAndEndpoint()` result (`zigbee.ts:346`).
pub struct EntityAndEndpoint {
    /// Upstream `ID`.
    pub id: String,
    pub entity: Option<Arc<dyn Entity>>,
    /// Upstream `endpointID`.
    pub endpoint_id: Option<String>,
    pub endpoint: Option<Arc<ZhEndpoint>>,
}

impl EntityAndEndpoint {
    /// Keep the upstream field spelling for call sites that use it.
    #[allow(non_snake_case)]
    pub fn ID(&self) -> &str {
        &self.id
    }
}

/// Upstream `Zigbee` (`lib/zigbee.ts:16`).
pub struct Zigbee {
    self_ref: OnceLock<Weak<Zigbee>>,
    herdsman: Mutex<Option<Arc<Controller>>>,
    event_bus: Arc<Mutex<EventBus>>,
    settings: Arc<Mutex<Settings>>,
    logger: Arc<Mutex<Logger>>,
    group_lookup: Mutex<std::collections::HashMap<u64, Arc<Group>>>,
    device_lookup: Mutex<std::collections::HashMap<String, Arc<Device>>>,
    coordinator_ieee_addr: Mutex<Option<String>>,
    converters: Arc<Mutex<DefinitionRegistry>>,
}

impl Zigbee {
    /// Upstream constructor (`zigbee.ts:23`). Groups need a `resolveDevice`
    /// callback pointing back at this instance, hence `Arc`.
    pub fn new(
        event_bus: Arc<Mutex<EventBus>>,
        settings: Arc<Mutex<Settings>>,
        logger: Arc<Mutex<Logger>>,
    ) -> Arc<Self> {
        Self::build(
            event_bus,
            settings,
            logger,
            Mutex::new(None),
            Mutex::new(None),
            Arc::new(Mutex::new(DefinitionRegistry::new())),
        )
    }

    /// Test/embedding constructor with an already-created controller.
    #[doc(hidden)]
    pub fn with_controller(
        controller: Arc<Controller>,
        event_bus: Arc<Mutex<EventBus>>,
        settings: Arc<Mutex<Settings>>,
        logger: Arc<Mutex<Logger>>,
    ) -> Arc<Self> {
        let coordinator_ieee_addr = controller
            .get_devices_by_type(DeviceType::Coordinator)
            .first()
            .map(|device| device.ieee_addr());
        Self::build(
            event_bus,
            settings,
            logger,
            Mutex::new(Some(controller)),
            Mutex::new(coordinator_ieee_addr),
            Arc::new(Mutex::new(DefinitionRegistry::new())),
        )
    }

    fn build(
        event_bus: Arc<Mutex<EventBus>>,
        settings: Arc<Mutex<Settings>>,
        logger: Arc<Mutex<Logger>>,
        herdsman: Mutex<Option<Arc<Controller>>>,
        coordinator_ieee_addr: Mutex<Option<String>>,
        converters: Arc<Mutex<DefinitionRegistry>>,
    ) -> Arc<Self> {
        let zigbee = Arc::new(Self {
            self_ref: OnceLock::new(),
            herdsman,
            event_bus,
            settings,
            logger,
            group_lookup: Mutex::new(std::collections::HashMap::new()),
            device_lookup: Mutex::new(std::collections::HashMap::new()),
            coordinator_ieee_addr,
            converters,
        });
        let _ = zigbee.self_ref.set(Arc::downgrade(&zigbee));
        zigbee
    }

    /// Converter registry handle (shared with every resolved `Device`).
    pub fn converters(&self) -> Arc<Mutex<DefinitionRegistry>> {
        self.converters.clone()
    }

    /// Upstream `get zhController()` (`zigbee.ts:27`).
    pub fn zh_controller(&self) -> Option<Arc<Controller>> {
        self.herdsman.lock().unwrap().clone()
    }

    fn require_controller(&self) -> Result<Arc<Controller>, String> {
        self.zh_controller()
            .ok_or_else(|| "Zigbee controller is not started".to_string())
    }

    fn resolve_device_fn(&self) -> ResolveDevice {
        let weak = self
            .self_ref
            .get()
            .expect("Zigbee must be created through Zigbee::new/with_controller")
            .clone();
        Arc::new(move |ieee: &str| {
            weak.upgrade()
                .and_then(|zigbee| zigbee.resolve_device(ieee))
        })
    }

    /// Upstream `start(abortSignal)` (`zigbee.ts:31`).
    pub async fn start(self: &Arc<Self>, abort: &AbortSignal) -> Result<bool, String> {
        self.logger.lock().unwrap().info(
            &format!("Starting zigbee-herdsman ({})", zigbee_apiarist::VERSION),
            "z2m",
        );

        let applied = self.settings.lock().unwrap().get();
        let advanced = applied
            .get("advanced")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let serial = applied.get("serial").cloned().unwrap_or_else(|| json!({}));

        let pan_id = match advanced.get("pan_id").cloned().unwrap_or(Value::Null) {
            Value::String(value) if value == "GENERATE" => self.generate_pan_id(),
            Value::Number(value) => value.as_u64().unwrap_or_default() as u16,
            _ => 0x1a62,
        };
        let extended_pan_id = match advanced.get("ext_pan_id").cloned().unwrap_or(Value::Null) {
            Value::String(value) if value == "GENERATE" => self.generate_ext_pan_id(),
            Value::Array(items) => items
                .iter()
                .filter_map(Value::as_u64)
                .map(|value| value as u8)
                .collect(),
            _ => vec![0xdd; 8],
        };
        let network_key = match advanced.get("network_key").cloned().unwrap_or(Value::Null) {
            Value::String(value) if value == "GENERATE" => self.generate_network_key(),
            Value::Array(items) => items
                .iter()
                .filter_map(Value::as_u64)
                .map(|value| value as u8)
                .collect(),
            _ => vec![1, 3, 5, 7, 9, 11, 13, 15, 0, 2, 4, 6, 8, 10, 12, 13],
        };
        let channel = advanced
            .get("channel")
            .and_then(Value::as_u64)
            .unwrap_or(11) as u8;

        let serial_path = serial
            .get("port")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let baud_rate = serial
            .get("baudrate")
            .and_then(Value::as_u64)
            .map(|value| value as u32);
        let rtscts = serial.get("rtscts").and_then(Value::as_bool);
        let adapter_name = serial
            .get("adapter")
            .and_then(Value::as_str)
            .map(str::to_string);
        if let Some(adapter_name) = adapter_name {
            if adapter_name != "zstack" {
                return Err(format!(
                    "Adapter '{adapter_name}' is not part of R1 (zstack only)"
                ));
            }
        }

        let herdsman_settings = json!({
            "network": {
                "panID": pan_id,
                "extendedPanID": extended_pan_id,
                "channelList": [channel],
                "networkKey": network_key,
            },
            "databasePath": data::join_path("database.db").display().to_string(),
            "databaseBackupPath": data::join_path("database.db.backup").display().to_string(),
            "backupPath": data::join_path("coordinator_backup.json").display().to_string(),
            "transport": {
                "baudRate": baud_rate,
                "rtscts": rtscts,
                "path": serial_path,
                "adapter": serial.get("adapter").cloned().unwrap_or(Value::Null),
            },
            "adapter": {
                "concurrent": advanced.get("adapter_concurrent").cloned().unwrap_or(Value::Null),
                "delay": advanced.get("adapter_delay").cloned().unwrap_or(Value::Null),
                "disableLED": serial.get("disable_led").cloned().unwrap_or(Value::Bool(false)),
                "transmitPower": advanced.get("transmit_power").cloned().unwrap_or(Value::Null),
            },
        });
        let hidden = stringify(&value_at(&herdsman_settings, &["network", "networkKey"]));
        self.logger.lock().unwrap().debug(
            &format!(
                "Using zigbee-herdsman with settings: '{}'",
                stringify(&herdsman_settings).replace(&hidden, "\"HIDDEN\"")
            ),
            "z2m",
        );

        let weak = Arc::downgrade(self);
        let accept_handler: Arc<
            dyn Fn(
                    String,
                )
                    -> zigbee_apiarist::controller::controller::ControllerFuture<'static, bool>
                + Send
                + Sync,
        > = Arc::new(move |ieee: String| {
            let weak = weak.clone();
            Box::pin(async move {
                Ok(weak
                    .upgrade()
                    .map(|zigbee| zigbee.accept_joining_device_handler(&ieee))
                    .unwrap_or(true))
            })
        });

        let options = ControllerOptions {
            network: NetworkOptions {
                pan_id,
                extended_pan_id: extended_pan_id.clone(),
                channel_list: vec![channel],
                network_key: network_key.clone(),
                network_key_distribute: true,
                has_default_extended_pan_id: false,
            },
            transport: herdsman_settings["transport"].clone(),
            database_path: data::join_path("database.db").display().to_string(),
            database_backup_path: Some(data::join_path("database.db.backup").display().to_string()),
            backup_path: Some(
                data::join_path("coordinator_backup.json")
                    .display()
                    .to_string(),
            ),
            adapter: herdsman_settings["adapter"].clone(),
            accept_joining_device_handler: Some(accept_handler),
        };

        let controller = Arc::new(Controller::new(options).map_err(|error| {
            self.logger
                .lock()
                .unwrap()
                .error("Error while starting zigbee-herdsman", "z2m");
            error
        })?);

        let transport = AdapterTransport::new(TransportOptions {
            path: serial_path,
            baud_rate,
            rtscts,
        });
        let zstack = ZStackAdapter::new(
            transport,
            ConfigNetworkOptions {
                pan_id,
                extended_pan_id: Some(extended_pan_id),
                network_key: Some(network_key),
                channel_list: vec![channel],
                network_key_distribute: None,
            },
            data::join_path("coordinator_backup.json")
                .display()
                .to_string(),
            AdapterOptions {
                concurrent: advanced
                    .get("adapter_concurrent")
                    .and_then(Value::as_u64)
                    .map(|value| value as u16),
                delay: advanced
                    .get("adapter_delay")
                    .and_then(Value::as_u64)
                    .map(|value| value as u16),
                disable_led: serial
                    .get("disable_led")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                transmit_power: advanced
                    .get("transmit_power")
                    .and_then(Value::as_i64)
                    .map(|value| value as i8),
                force_start_with_inconsistent_adapter_configuration: false,
            },
        );
        let runtime =
            Arc::new(ZStackRuntimeAdapter::new(Arc::new(tokio::sync::Mutex::new(zstack))).await)
                as Arc<dyn ControllerRuntimeAdapter>;

        let start_result = match controller.start_with_abort(runtime, abort.wait()).await {
            Ok(result) => result,
            Err(_error) if abort.aborted() => return Ok(false),
            Err(error) => {
                self.logger
                    .lock()
                    .unwrap()
                    .error("Error while starting zigbee-herdsman", "z2m");
                return Err(error);
            }
        };

        *self.herdsman.lock().unwrap() = Some(controller.clone());

        let coordinator = controller
            .get_devices_by_type(DeviceType::Coordinator)
            .first()
            .cloned()
            .ok_or_else(|| "Coordinator device not found".to_string())?;
        *self.coordinator_ieee_addr.lock().unwrap() = Some(coordinator.ieee_addr());

        if abort.aborted() {
            return Ok(false);
        }

        let weak = Arc::downgrade(self);
        controller.on(move |event| {
            if let Some(zigbee) = weak.upgrade() {
                zigbee.handle_controller_event(event);
            }
        });

        let started = match start_result {
            StartResult::Resumed => "resumed",
            StartResult::Reset => "reset",
            StartResult::Restored => "restored",
        };
        self.logger
            .lock()
            .unwrap()
            .info(&format!("zigbee-herdsman started ({started})"), "z2m");

        let version = self
            .get_coordinator_version()
            .await
            .unwrap_or_else(|error| json!({"error": error}));
        self.logger.lock().unwrap().info(
            &format!("Coordinator firmware version: '{}'", stringify(&version)),
            "z2m",
        );
        if let Ok(parameters) = self.get_network_parameters().await {
            self.logger.lock().unwrap().debug(
                &format!("Zigbee network parameters: {}", stringify(&parameters)),
                "z2m",
            );
        }

        if abort.aborted() {
            return Ok(false);
        }

        let passlist = self
            .settings
            .lock()
            .unwrap()
            .get()
            .get("passlist")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let passlist: Vec<String> = passlist
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();

        if !passlist.is_empty() {
            for device in self.devices_iterator(|zh| zh.r#type() != DeviceType::Coordinator) {
                if !passlist.contains(&device.ieee_addr()) {
                    self.logger.lock().unwrap().warning(
                        &format!(
                            "Device not on passlist currently on the network ({}), removing...",
                            device.ieee_addr()
                        ),
                        "z2m",
                    );
                    if let Err(error) = device.zh.remove_from_network(false).await {
                        self.logger.lock().unwrap().error(
                            &format!("Failed to remove '{}' ({error})", device.ieee_addr()),
                            "z2m",
                        );
                    }
                    if abort.aborted() {
                        return Ok(false);
                    }
                }
            }
        } else {
            let blocklist: Vec<String> = self
                .settings
                .lock()
                .unwrap()
                .get()
                .get("blocklist")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();

            for ieee in blocklist {
                let Some(device) = self.resolve_device(&ieee) else {
                    self.logger.lock().unwrap().debug(
                        &format!("Ignoring blocklist device {ieee}, not currently on the network"),
                        "z2m",
                    );
                    continue;
                };

                if device.zh.r#type() == DeviceType::Coordinator {
                    continue;
                }

                self.logger.lock().unwrap().warning(
                    &format!(
                        "Device on blocklist currently on the network ({}), removing...",
                        device.ieee_addr()
                    ),
                    "z2m",
                );
                if let Err(error) = device.zh.remove_from_network(false).await {
                    self.logger.lock().unwrap().error(
                        &format!("Failed to remove '{}' ({error})", device.ieee_addr()),
                        "z2m",
                    );
                }
                if abort.aborted() {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    fn handle_controller_event(&self, event: &ControllerEvent) {
        match event {
            ControllerEvent::AdapterDisconnected => {
                self.event_bus.lock().unwrap().emit_adapter_disconnected();
            }
            ControllerEvent::LastSeenChanged(payload) => {
                if let Some(device) = self.resolve_device(&payload.device.ieee_addr()) {
                    self.event_bus
                        .lock()
                        .unwrap()
                        .emit_last_seen_changed(json!({
                            "device": device.event_value(),
                            "reason": payload.reason,
                        }));
                }
            }
            ControllerEvent::PermitJoinChanged(payload) => {
                self.event_bus
                    .lock()
                    .unwrap()
                    .emit_permit_join_changed(json!({
                        "permitted": payload.permitted,
                        "time": payload.time,
                    }));
            }
            ControllerEvent::DeviceNetworkAddressChanged(payload) => {
                if let Some(device) = self.resolve_device(&payload.device.ieee_addr()) {
                    self.logger.lock().unwrap().debug(
                        &format!("Device '{}' changed network address", device.name()),
                        "z2m",
                    );
                    self.event_bus
                        .lock()
                        .unwrap()
                        .emit_device_network_address_changed(
                            json!({"device": device.event_value()}),
                        );
                }
            }
            ControllerEvent::DeviceAnnounce(payload) => {
                if let Some(device) = self.resolve_device(&payload.device.ieee_addr()) {
                    self.logger.lock().unwrap().debug(
                        &format!("Device '{}' announced itself", device.name()),
                        "z2m",
                    );
                    self.event_bus
                        .lock()
                        .unwrap()
                        .emit_device_announce(json!({"device": device.event_value()}));
                }
            }
            ControllerEvent::DeviceInterview(payload) => {
                let Some(device) = self.resolve_device(&payload.device.ieee_addr()) else {
                    return;
                };
                device.resolve_definition(false);
                self.log_device_interview(&device, &payload.status);
                self.event_bus.lock().unwrap().emit_device_interview(json!({
                    "device": device.event_value(),
                    "status": payload.status,
                }));
            }
            ControllerEvent::DeviceJoined(payload) => {
                let Some(device) = self.resolve_device(&payload.device.ieee_addr()) else {
                    return;
                };
                device.resolve_definition(false);
                self.logger
                    .lock()
                    .unwrap()
                    .info(&format!("Device '{}' joined", device.name()), "z2m");
                self.event_bus
                    .lock()
                    .unwrap()
                    .emit_device_joined(json!({"device": device.event_value()}));
            }
            ControllerEvent::DeviceLeave(payload) => {
                let friendly_name = self
                    .settings
                    .lock()
                    .unwrap()
                    .get_device(&payload.ieee_addr)
                    .and_then(|device| device.get("friendly_name").cloned())
                    .and_then(|name| name.as_str().map(str::to_string))
                    .unwrap_or_else(|| payload.ieee_addr.clone());
                self.logger
                    .lock()
                    .unwrap()
                    .warning(&format!("Device '{friendly_name}' left the network"), "z2m");
                let device = self
                    .device_lookup
                    .lock()
                    .unwrap()
                    .get(&payload.ieee_addr)
                    .cloned();
                self.event_bus.lock().unwrap().emit_device_leave(json!({
                    "ieeeAddr": payload.ieee_addr,
                    "name": friendly_name,
                    "device": device.map(|device| device.event_value()),
                }));
            }
            ControllerEvent::Message(payload) => self.handle_message(payload),
        }
    }

    fn handle_message(&self, payload: &MessagePayload) {
        let Some(device) = self.resolve_device(&payload.device.ieee_addr()) else {
            return;
        };
        device.resolve_definition(false);

        let from_coordinator = device.zh.r#type() == DeviceType::Coordinator;
        self.logger.lock().unwrap().debug(
            &format!(
                "Received Zigbee message from '{}', type '{}', cluster '{}', data '{}' from endpoint {}{}{}",
                device.name(),
                payload.r#type,
                payload.cluster,
                stringify(&payload.data),
                payload.endpoint.ID,
                if payload.group_id != 0 {
                    format!(" with groupID {}", payload.group_id)
                } else {
                    String::new()
                },
                if from_coordinator {
                    ", ignoring since it is from coordinator"
                } else {
                    ""
                }
            ),
            "z2m",
        );

        if from_coordinator {
            return;
        }

        let mut message = json!({
            "type": payload.r#type,
            "device": device.event_value(),
            "endpoint": {"ID": payload.endpoint.ID},
            "linkquality": payload.linkquality,
            "cluster": payload.cluster,
            "data": payload.data,
            "meta": {
                "zclTransactionSequenceNumber": payload.meta.zcl_transaction_sequence_number,
                "manufacturerCode": payload.meta.manufacturer_code,
                "rawData": payload.meta.raw_data,
            },
        });
        if payload.group_id != 0 {
            message["groupID"] = json!(payload.group_id);
        }
        self.event_bus.lock().unwrap().emit_device_message(message);
    }

    /// Upstream `logDeviceInterview()` (`zigbee.ts:194`).
    fn log_device_interview(&self, device: &Device, status: &str) {
        let name = device.name();
        match status {
            "successful" => {
                self.logger.lock().unwrap().info(
                    &format!(
                        "Successfully interviewed '{name}', device has successfully been paired"
                    ),
                    "z2m",
                );
                self.logger.lock().unwrap().warning(
                    &format!(
                        "Device '{name}' with Zigbee model '{}' and manufacturer name '{}' is NOT supported, please follow https://www.zigbee2mqtt.io/advanced/support-new-devices/01_support_new_devices.html",
                        device.zh.model_id().unwrap_or_default(),
                        device.zh.manufacturer_name().unwrap_or_default(),
                    ),
                    "z2m",
                );
            }
            "failed" => {
                self.logger.lock().unwrap().error(
                    &format!(
                        "Failed to interview '{name}', device has not successfully been paired"
                    ),
                    "z2m",
                );
            }
            _ => {
                self.logger
                    .lock()
                    .unwrap()
                    .info(&format!("Starting interview of '{name}'"), "z2m");
            }
        }
    }

    /// Upstream `generateNetworkKey()` (`zigbee.ts:216`).
    fn generate_network_key(&self) -> Vec<u8> {
        let key = random_bytes(16);
        self.settings
            .lock()
            .unwrap()
            .set(&["advanced", "network_key"], json!(key));
        key
    }

    /// Upstream `generateExtPanID()` (`zigbee.ts:222`).
    fn generate_ext_pan_id(&self) -> Vec<u8> {
        let key = random_bytes(8);
        self.settings
            .lock()
            .unwrap()
            .set(&["advanced", "ext_pan_id"], json!(key));
        key
    }

    /// Upstream `generatePanID()` (`zigbee.ts:228`).
    fn generate_pan_id(&self) -> u16 {
        let pan_id = 1 + (random_bytes(2)[0] as u16 * 256 + random_bytes(2)[1] as u16) % 0xfffd;
        self.settings
            .lock()
            .unwrap()
            .set(&["advanced", "pan_id"], json!(pan_id));
        pan_id
    }

    /// Upstream `getCoordinatorVersion()` (`zigbee.ts:234`).
    pub async fn get_coordinator_version(&self) -> Result<Value, String> {
        let version = self.require_controller()?.get_coordinator_version().await?;
        Ok(json!({"type": version.r#type, "meta": version.meta}))
    }

    /// Upstream `isStopping()` (`zigbee.ts:238`).
    pub fn is_stopping(&self) -> bool {
        self.zh_controller()
            .map(|controller| controller.is_stopping())
            .unwrap_or(false)
    }

    /// Upstream `backup()` (`zigbee.ts:242`).
    pub async fn backup(&self) -> Result<(), String> {
        self.require_controller()?.backup().await
    }

    /// Upstream `coordinatorCheck()` (`zigbee.ts:246`).
    pub async fn coordinator_check(&self) -> Result<Vec<Arc<Device>>, String> {
        let missing = self.require_controller()?.coordinator_check().await?;
        Ok(missing
            .into_iter()
            .filter_map(|device| self.resolve_device(&device.ieee_addr()))
            .collect())
    }

    /// Upstream `getNetworkParameters()` (`zigbee.ts:252`).
    pub async fn get_network_parameters(&self) -> Result<Value, String> {
        let parameters = self.require_controller()?.get_network_parameters().await?;
        Ok(json!({
            "panID": parameters.pan_id,
            "extendedPanID": parameters.extended_pan_id,
            "channel": parameters.channel,
            "nwkUpdateID": parameters.nwk_update_id,
        }))
    }

    /// Upstream `stop()` (`zigbee.ts:256`).
    pub async fn stop(&self) -> Result<(), String> {
        self.logger
            .lock()
            .unwrap()
            .info("Stopping zigbee-herdsman...", "z2m");
        if let Some(controller) = self.zh_controller() {
            controller.stop().await?;
        }
        self.logger
            .lock()
            .unwrap()
            .info("Stopped zigbee-herdsman", "z2m");
        Ok(())
    }

    /// Upstream `getPermitJoin()` (`zigbee.ts:262`).
    pub fn get_permit_join(&self) -> bool {
        self.zh_controller()
            .map(|controller| controller.get_permit_join())
            .unwrap_or(false)
    }

    /// Upstream `getPermitJoinEnd()` (`zigbee.ts:266`).
    pub fn get_permit_join_end(&self) -> Option<u64> {
        self.zh_controller()
            .and_then(|controller| controller.get_permit_join_end())
    }

    /// Upstream `permitJoin(time, device?)` (`zigbee.ts:270`).
    pub async fn permit_join(&self, time: u16, device: Option<&Arc<Device>>) -> Result<(), String> {
        if time > 0 {
            self.logger.lock().unwrap().info(
                &format!(
                    "Zigbee: allowing new devices to join{}.",
                    device
                        .map(|device| format!(" via {}", device.name()))
                        .unwrap_or_default()
                ),
                "z2m",
            );
        } else {
            self.logger
                .lock()
                .unwrap()
                .info("Zigbee: disabling joining new devices.", "z2m");
        }

        self.require_controller()?
            .permit_join(time, device.map(|device| &device.zh))
            .await
    }

    /// Upstream `resolveDevice(ieeeAddr)` (`zigbee.ts:290`).
    pub fn resolve_device(&self, ieee_addr: &str) -> Option<Arc<Device>> {
        let existing = self.device_lookup.lock().unwrap().get(ieee_addr).cloned();
        let device = match existing {
            Some(device) => device,
            None => {
                let zh = self.zh_controller()?.get_device_by_ieee_addr(ieee_addr)?;
                let device = Arc::new(Device::new(
                    zh,
                    self.settings.clone(),
                    self.converters.clone(),
                ));
                self.device_lookup
                    .lock()
                    .unwrap()
                    .insert(ieee_addr.to_string(), device.clone());
                device
            }
        };

        if device.zh.is_deleted() {
            return None;
        }

        device.ensure_in_settings(&mut self.settings.lock().unwrap());
        Some(device)
    }

    /// Upstream `resolveGroup(groupID)` (`zigbee.ts:305`).
    pub fn resolve_group(&self, group_id: u64) -> Option<Arc<Group>> {
        let existing = self.group_lookup.lock().unwrap().get(&group_id).cloned();
        let group = match existing {
            Some(group) => group,
            None => {
                let zh = self.zh_controller()?.get_group_by_id(group_id as u16)?;
                let group = Arc::new(Group::new(
                    zh,
                    self.resolve_device_fn(),
                    self.settings.clone(),
                ));
                self.group_lookup
                    .lock()
                    .unwrap()
                    .insert(group_id, group.clone());
                group
            }
        };

        group.ensure_in_settings(&mut self.settings.lock().unwrap());
        Some(group)
    }

    /// Upstream `resolveEntity(key)` (`zigbee.ts:322`) for string and numeric
    /// keys. The `zh.Device` overload is `resolve_device(ieeeAddr)`.
    pub fn resolve_entity(&self, key: &str) -> Option<Arc<dyn Entity>> {
        let coordinator = self.coordinator_ieee_addr.lock().unwrap().clone();
        if key.eq_ignore_ascii_case("coordinator") || coordinator.as_deref() == Some(key) {
            return self
                .resolve_device(coordinator.as_deref()?)
                .map(|device| device as Arc<dyn Entity>);
        }

        let (settings_device, settings_group) = {
            let settings = self.settings.lock().unwrap();
            (settings.get_device(key), settings.get_group(key))
        };

        if let Some(device) = settings_device {
            if let Some(id) = device.get("ID").and_then(Value::as_str) {
                return self
                    .resolve_device(id)
                    .map(|device| device as Arc<dyn Entity>);
            }
        }

        let group_settings = settings_group?;
        let group_id = group_settings.get("ID").and_then(Value::as_u64)?;
        match self.resolve_group(group_id) {
            Some(group) => Some(group as Arc<dyn Entity>),
            None => self
                .create_group(group_id)
                .map(|group| group as Arc<dyn Entity>),
        }
    }

    /// Upstream `resolveEntityAndEndpoint(id)` (`zigbee.ts:346`).
    pub fn resolve_entity_and_endpoint(&self, id: &str) -> EntityAndEndpoint {
        let mut entity_name = id.to_string();
        let mut entity = self.resolve_entity(id);
        let mut endpoint_name_or_id = None;

        if entity.is_none() {
            if let Some((name, endpoint)) = split_entity_and_endpoint(id) {
                entity_name = name.to_string();
                entity = self.resolve_entity(name);
                endpoint_name_or_id = Some(endpoint.to_string());
            }
        }

        let endpoint = match &entity {
            Some(entity) if entity.is_device() => {
                let device = self.resolve_device(entity.entity_id().key().as_str());
                device.and_then(|device| device.endpoint(endpoint_name_or_id.as_deref()))
            }
            _ => None,
        };

        EntityAndEndpoint {
            id: entity_name,
            entity,
            endpoint_id: endpoint_name_or_id,
            endpoint,
        }
    }

    /// Upstream `firstCoordinatorEndpoint()` (`zigbee.ts:381`).
    pub fn first_coordinator_endpoint(&self) -> Option<Arc<ZhEndpoint>> {
        self.zh_controller()?
            .get_devices_by_type(DeviceType::Coordinator)
            .first()
            .and_then(|device| device.endpoints().first().cloned())
    }

    /// Upstream `devicesAndGroupsIterator()` (`zigbee.ts:385`).
    pub fn devices_and_groups_iterator(&self) -> Vec<Arc<dyn Entity>> {
        let mut entities: Vec<Arc<dyn Entity>> = self
            .devices_iterator(|_| true)
            .into_iter()
            .map(|device| device as Arc<dyn Entity>)
            .collect();
        entities.extend(
            self.groups_iterator(|_| true)
                .into_iter()
                .map(|group| group as Arc<dyn Entity>),
        );
        entities
    }

    /// Upstream `groupsIterator(predicate?)` (`zigbee.ts:400`).
    pub fn groups_iterator(
        &self,
        predicate: impl Fn(&Arc<zigbee_apiarist::controller::model::group::Group>) -> bool,
    ) -> Vec<Arc<Group>> {
        let Some(controller) = self.zh_controller() else {
            return Vec::new();
        };
        controller
            .get_groups_iterator(predicate)
            .into_iter()
            .filter_map(|group| self.resolve_group(group.group_id as u64))
            .collect()
    }

    /// Upstream `devicesIterator(predicate?)` (`zigbee.ts:407`).
    pub fn devices_iterator(&self, predicate: impl Fn(&Arc<ZhDevice>) -> bool) -> Vec<Arc<Device>> {
        let Some(controller) = self.zh_controller() else {
            return Vec::new();
        };
        controller
            .get_devices_iterator(predicate)
            .into_iter()
            .filter_map(|device| self.resolve_device(&device.ieee_addr()))
            .collect()
    }

    /// Upstream `acceptJoiningDeviceHandler(ieeeAddr)` (`zigbee.ts:415`).
    pub fn accept_joining_device_handler(&self, ieee_addr: &str) -> bool {
        let applied = self.settings.lock().unwrap().get();
        let list = |key: &str| -> Vec<String> {
            applied
                .get(key)
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default()
        };
        let passlist = list("passlist");
        let blocklist = list("blocklist");

        if !passlist.is_empty() {
            if passlist.iter().any(|entry| entry == ieee_addr) {
                self.logger.lock().unwrap().info(
                    &format!("Accepting joining device which is on passlist '{ieee_addr}'"),
                    "z2m",
                );
                return true;
            }

            self.logger.lock().unwrap().info(
                &format!("Rejecting joining not in passlist device '{ieee_addr}'"),
                "z2m",
            );
            return false;
        }

        if !blocklist.is_empty() {
            if blocklist.iter().any(|entry| entry == ieee_addr) {
                self.logger.lock().unwrap().info(
                    &format!("Rejecting joining device which is on blocklist '{ieee_addr}'"),
                    "z2m",
                );
                return false;
            }

            self.logger.lock().unwrap().info(
                &format!("Accepting joining not in blocklist device '{ieee_addr}'"),
                "z2m",
            );
        }

        true
    }

    /// Upstream `addInstallCode(installCode)` (`zigbee.ts:449`).
    pub async fn add_install_code(&self, install_code: &str) -> Result<(), String> {
        self.require_controller()?
            .add_install_code(install_code)
            .await
    }

    /// Upstream `createGroup(id)` (`zigbee.ts:461`).
    pub fn create_group(&self, id: u64) -> Option<Arc<Group>> {
        self.zh_controller()?.create_group(id as u16).ok()?;
        self.resolve_group(id)
    }

    /// Upstream `deviceByNetworkAddress(networkAddress)` (`zigbee.ts:467`).
    pub fn device_by_network_address(&self, network_address: u16) -> Option<Arc<Device>> {
        let device = self
            .zh_controller()?
            .get_device_by_network_address(network_address)?;
        self.resolve_device(&device.ieee_addr())
    }

    /// Upstream `groupByID(id)` (`zigbee.ts:472`).
    pub fn group_by_id(&self, id: u64) -> Option<Arc<Group>> {
        self.resolve_group(id)
    }

    /// Upstream `removeDeviceFromLookup(ieee)` (`zigbee.ts:476`).
    pub fn remove_device_from_lookup(&self, ieee: &str) -> bool {
        self.device_lookup.lock().unwrap().remove(ieee).is_some()
    }

    /// Upstream `removeGroupFromLookup(id)` (`zigbee.ts:480`).
    pub fn remove_group_from_lookup(&self, id: u64) -> bool {
        self.group_lookup.lock().unwrap().remove(&id).is_some()
    }
}

fn value_at(value: &Value, path: &[&str]) -> Value {
    let mut current = value;
    for key in path {
        match current.get(*key) {
            Some(next) => current = next,
            None => return Value::Null,
        }
    }
    current.clone()
}

/// Upstream `crypto.randomInt` byte source (`zigbee.ts:216-232`).
fn random_bytes(length: usize) -> Vec<u8> {
    use std::io::Read;

    let mut bytes = vec![0u8; length];
    match std::fs::File::open("/dev/urandom") {
        Ok(mut file) => {
            if file.read_exact(&mut bytes).is_err() {
                seed_fallback(&mut bytes);
            }
        }
        Err(_) => seed_fallback(&mut bytes),
    }
    bytes
}

fn seed_fallback(bytes: &mut [u8]) {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = (seed >> ((index % 16) * 8)) as u8 ^ (index as u8).wrapping_mul(31);
    }
}
