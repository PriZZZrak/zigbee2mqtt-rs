// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/adapter/zStackAdapter.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).
//
// ZDO send path (`sendZdo` / `sendZdoInternal`). This is the part of
// `ZStackAdapter` used by the adapter manager and by the controller for
// ZDO requests; the ZCL/queue/start surface is ported separately.

use std::future::Future;
use std::pin::Pin;

use serde_json::{json, Value};

use crate::adapter::z_stack::adapter::manager::ZdoAdapter;
use crate::adapter::z_stack::constants::af::DEFAULT_RADIUS;
use crate::adapter::z_stack::constants::common::AddressMode;
use crate::adapter::z_stack::unpi::constants::{Subsystem, Type};
use crate::adapter::z_stack::znp::definition;
use crate::adapter::z_stack::znp::znp::Znp;
use crate::utils::logger::logger;
use crate::utils::wait::wait;
use crate::zspec::zdo::definition::clusters::ClusterId;
use crate::zspec::zdo::utils::get_response_cluster_id;

const NS: &str = "zh:zstack";

fn is_broadcast(network_address: u16) -> bool {
    matches!(network_address, 0xfffc | 0xfffd | 0xffff)
}

/// Upstream `ZStackAdapter.sendZdoInternal`.
pub async fn send_zdo_internal(
    znp: &mut Znp,
    ieee_address: &str,
    network_address: u16,
    cluster_id: u16,
    payload: &[u8],
    disable_response: bool,
) -> Result<Value, String> {
    let payload = match cluster_id {
        ClusterId::PERMIT_JOINING_REQUEST => {
            let mut final_payload = vec![0u8; payload.len() + 3];
            final_payload[0] = if is_broadcast(network_address) {
                AddressMode::ADDR_BROADCAST as u8
            } else {
                AddressMode::ADDR_16BIT as u8
            };
            final_payload[1..3].copy_from_slice(&network_address.to_le_bytes());
            final_payload[3..].copy_from_slice(payload);
            final_payload
        }
        ClusterId::NWK_UPDATE_REQUEST => {
            let zeroes = 9isize - payload.len() as isize - 1;
            let zeroes = zeroes.max(0) as usize;
            let mut final_payload = vec![0u8; payload.len() + 3 + zeroes];
            final_payload[0..2].copy_from_slice(&network_address.to_le_bytes());
            final_payload[2] = if is_broadcast(network_address) {
                AddressMode::ADDR_BROADCAST as u8
            } else {
                AddressMode::ADDR_16BIT as u8
            };
            final_payload[3..3 + payload.len()].copy_from_slice(payload);
            final_payload
        }
        ClusterId::BIND_REQUEST | ClusterId::UNBIND_REQUEST => {
            let zeroes = 21isize - payload.len() as isize;
            let zeroes = zeroes.max(0) as usize;
            let mut final_payload = vec![0u8; payload.len() + 2 + zeroes];
            final_payload[0..2].copy_from_slice(&network_address.to_le_bytes());
            final_payload[2..2 + payload.len()].copy_from_slice(payload);
            final_payload
        }
        ClusterId::NETWORK_ADDRESS_REQUEST | ClusterId::IEEE_ADDRESS_REQUEST => payload.to_vec(),
        _ => {
            let mut final_payload = vec![0u8; payload.len() + 2];
            final_payload[0..2].copy_from_slice(&network_address.to_le_bytes());
            final_payload[2..].copy_from_slice(payload);
            final_payload
        }
    };

    let waiter = if !disable_response {
        get_response_cluster_id(cluster_id).map(|response_cluster_id| {
            let command = definition::find_zdo_response(Subsystem::ZDO, response_cluster_id)
                .unwrap_or_else(|| {
                    panic!("Response for ZDO cluster ID '{response_cluster_id}' not supported.")
                });
            (
                command.name.to_string(),
                if response_cluster_id == ClusterId::NETWORK_ADDRESS_RESPONSE {
                    Value::String(ieee_address.to_string())
                } else {
                    json!(network_address)
                },
            )
        })
    } else {
        None
    };

    let waiter = if let Some((command, target)) = waiter {
        Some(
            znp.wait_for(
                Type::AREQ,
                Subsystem::ZDO,
                &command,
                Some(target),
                None,
                None,
                None,
            )
            .await,
        )
    } else {
        None
    };

    let request_result = znp
        .request_zdo(
            cluster_id,
            payload.clone(),
            waiter.as_ref().map(|(id, _)| *id),
        )
        .await;

    if let Err(error) = request_result {
        if cluster_id == ClusterId::NODE_DESCRIPTOR_REQUEST {
            logger().debug(
                &format!(
                    "Discover route to '{network_address}' because node descriptor request failed"
                ),
                NS,
            );
            discover_route(znp, network_address, true).await?;
            znp.request_zdo(cluster_id, payload, waiter.as_ref().map(|(id, _)| *id))
                .await?;
        } else {
            return Err(error);
        }
    }

    if let Some((_, receiver)) = waiter {
        let response = receiver.await.map_err(|_| "Waiter dropped".to_string())??;
        return Ok(response.payload.get("zdo").cloned().unwrap_or(Value::Null));
    }
    Ok(Value::Null)
}

/// Upstream `ZStackAdapter.discoverRoute`.
async fn discover_route(
    znp: &mut Znp,
    network_address: u16,
    wait_settled: bool,
) -> Result<(), String> {
    logger().debug(&format!("Discovering route to {network_address}"), NS);
    znp.request(
        Subsystem::ZDO,
        "extRouteDisc",
        json!({"dstAddr": network_address, "options": 0, "radius": DEFAULT_RADIUS}),
        None,
        None,
        &[0],
    )
    .await?;
    if wait_settled {
        wait(3000).await;
    }
    Ok(())
}

/// Upstream `ZStackAdapter` ZDO surface.
#[derive(Debug, Clone, Copy, Default)]
pub struct ZStackZdo {
    pub has_zdo_message_overhead: bool,
}

impl ZdoAdapter for ZStackZdo {
    fn has_zdo_message_overhead(&self) -> bool {
        self.has_zdo_message_overhead
    }

    fn send_zdo<'a>(
        &'a mut self,
        znp: &'a mut Znp,
        ieee_address: &'a str,
        network_address: u16,
        cluster_id: u16,
        payload: &'a [u8],
        disable_response: bool,
    ) -> Pin<Box<dyn Future<Output = Result<Value, String>> + 'a>> {
        Box::pin(send_zdo_internal(
            znp,
            ieee_address,
            network_address,
            cluster_id,
            payload,
            disable_response,
        ))
    }
}

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::adapter::transport::AdapterTransport;
use crate::adapter::z_stack::adapter::adapter_backup::AdapterBackup;
use crate::adapter::z_stack::adapter::adapter_nv_memory::AdapterNvMemory;
use crate::adapter::z_stack::adapter::endpoints::endpoints;
use crate::adapter::z_stack::adapter::manager::{StartResult, ZnpAdapterManager};
use crate::adapter::z_stack::adapter::tstype::ZnpVersion;
use crate::adapter::z_stack::constants::common::ZnpCommandStatus;
use crate::adapter::z_stack::constants::sys;
use crate::adapter::z_stack::structs::entries::nib::nib;
use crate::models::startup_options::{AdapterOptions, ConfigNetworkOptions, StartupOptions};
use crate::utils::queue::Queue;
use crate::zspec::zdo::buffalo_zdo::BuffaloZdo;

/// Upstream `CoordinatorVersion`.
#[derive(Debug, Clone, PartialEq)]
pub struct CoordinatorVersion {
    pub type_name: String,
    pub meta: Value,
}

/// Upstream `ZStackAdapter` (start/stop/queue/ZDO slice).
pub struct ZStackAdapter {
    pub znp: Arc<Mutex<Znp>>,
    pub network_options: ConfigNetworkOptions,
    pub backup_path: String,
    pub adapter_options: AdapterOptions,
    pub has_zdo_message_overhead: bool,
    pub manufacturer_id: u16,
    pub version: Option<Value>,
    pub queue: Option<Queue>,
    device_announce_route_discovery:
        Arc<std::sync::Mutex<std::collections::HashMap<u16, std::time::Instant>>>,
    pub waitress: Arc<crate::utils::waitress::Waitress<ZclPayload, ClusterWaitressMatcher>>,
    pub events: tokio::sync::broadcast::Sender<AdapterEvent>,
    listener: Option<tokio::task::JoinHandle<()>>,
    transaction_id: u8,
    supports_led: Option<bool>,
    interpan_lock: bool,
    interpan_endpoint_registered: bool,
}

impl ZStackAdapter {
    /// Upstream `constructor(networkOptions, transportOptions, backupPath, adapterOptions)`.
    pub fn new(
        transport: AdapterTransport,
        network_options: ConfigNetworkOptions,
        backup_path: impl Into<String>,
        adapter_options: AdapterOptions,
    ) -> Self {
        Self {
            znp: Arc::new(Mutex::new(Znp::new(transport))),
            network_options,
            backup_path: backup_path.into(),
            adapter_options,
            has_zdo_message_overhead: false,
            manufacturer_id: 0x1037, // Zcl.ManufacturerCode.TEXAS_INSTRUMENTS
            version: None,
            queue: None,
            device_announce_route_discovery: Arc::new(std::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            waitress: Arc::new(crate::utils::waitress::Waitress::new(
                |payload: &ZclPayload, matcher: &ClusterWaitressMatcher| {
                    (matcher
                        .address
                        .as_ref()
                        .is_none_or(|address| address == &payload.address))
                        && payload.cluster_id == matcher.cluster_id
                        && matcher
                            .endpoint
                            .is_none_or(|endpoint| endpoint == payload.endpoint)
                        && matcher
                            .transaction_sequence_number
                            .is_none_or(|tsn| tsn == payload.header.transaction_sequence_number)
                        && (matcher.command_id == Some(payload.header.command_identifier)
                            || (payload.header.frame_control.frame_type
                                == crate::zspec::zcl::zcl_header::FrameType::GLOBAL
                                && payload.header.command_identifier == 0x0b))
                },
                |matcher: &ClusterWaitressMatcher, timeout: u64| {
                    format!(
                        "Timeout after {timeout}ms [address={:?} endpoint={:?} clusterId={} cmdId={:?} tsn={:?}]",
                        matcher.address, matcher.endpoint, matcher.cluster_id, matcher.command_id, matcher.transaction_sequence_number
                    )
                },
            )),
            events: tokio::sync::broadcast::channel(64).0,
            listener: None,
            transaction_id: 0,
            supports_led: None,
            interpan_lock: false,
            interpan_endpoint_registered: false,
        }
    }

    /// Upstream `start()`.
    pub async fn start(&mut self) -> Result<StartResult, String> {
        {
            let mut znp = self.znp.lock().await;
            znp.open().await?;
        }

        let mut last_error = String::new();
        let mut connected = false;
        for attempt in 0..3 {
            let result = {
                let mut znp = self.znp.lock().await;
                znp.request(
                    Subsystem::SYS,
                    "ping",
                    json!({"capabilities": 1}),
                    None,
                    None,
                    &[0],
                )
                .await
            };
            match result {
                Ok(_) => {
                    connected = true;
                    break;
                }
                Err(error) => {
                    last_error = error;
                    if attempt == 2 {
                        return Err(format!("Failed to connect to the adapter ({last_error})"));
                    }
                }
            }
        }
        if !connected {
            return Err(format!("Failed to connect to the adapter ({last_error})"));
        }

        let version = {
            let mut znp = self.znp.lock().await;
            znp.request_with_reply(Subsystem::SYS, "version", json!({}))
                .await
                .ok()
                .map(|reply| reply.payload)
        };
        let version = version.unwrap_or_else(|| {
            logger().debug("Failed to get zStack version, assuming 1.2", NS);
            json!({"transportrev": 2, "product": 0, "majorrel": 2, "minorrel": 0, "maintrel": 0, "revision": 0})
        });
        let product = version.get("product").and_then(Value::as_u64).unwrap_or(0);
        let concurrent = self.adapter_options.concurrent.map(usize::from).unwrap_or(
            if product == ZnpVersion::ZStack3x0 as u64 {
                16
            } else {
                2
            },
        );
        logger().debug(&format!("Adapter concurrent: {concurrent}"), NS);
        self.queue = Some(Queue::new(concurrent));
        self.version = Some(version.clone());

        let version_enum = ZnpVersion::from_product(product).unwrap_or(ZnpVersion::ZStack12);
        let manager_options = StartupOptions {
            version: version_enum,
            network_options: self.network_options.clone(),
            green_power_group: 242,
            backup_path: self.backup_path.clone(),
            adapter_options: self.adapter_options.clone(),
        };
        let mut manager = ZnpAdapterManager::new(
            ZStackZdo {
                has_zdo_message_overhead: self.has_zdo_message_overhead,
            },
            AdapterNvMemory::from_shared(self.znp.clone()),
            manager_options,
        );
        let result = manager.start().await?;
        self.start_event_loop().await?;

        if self.adapter_options.disable_led {
            wait(200).await;
            self.set_led("disable").await?;
        }
        if let Some(power) = self.adapter_options.transmit_power {
            let mut znp = self.znp.lock().await;
            znp.request(
                Subsystem::SYS,
                "stackTune",
                json!({"operation": 0, "value": power}),
                None,
                None,
                &[0],
            )
            .await?;
        }
        Ok(result)
    }

    /// Upstream `stop()`.
    pub async fn stop(&mut self) -> Result<(), String> {
        self.znp.lock().await.close().await
    }

    /// Upstream `getCoordinatorIEEE()`.
    pub async fn get_coordinator_ieee(&mut self) -> Result<String, String> {
        self.check_interpan_lock()?;
        let queue = self
            .queue
            .clone()
            .ok_or_else(|| "Adapter not started".to_string())?;
        let znp = self.znp.clone();
        queue
            .execute(
                || async move {
                    let mut znp = znp.lock().await;
                    let device_info = znp
                        .request_with_reply(Subsystem::UTIL, "getDeviceInfo", json!({}))
                        .await?;
                    device_info
                        .payload
                        .get("ieeeaddr")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                        .ok_or_else(|| "Missing coordinator IEEE address".to_string())
                },
                None,
            )
            .await
    }

    /// Upstream `getCoordinatorVersion()`.
    pub fn get_coordinator_version(&self) -> Option<CoordinatorVersion> {
        self.version.as_ref().map(|version| {
            let product = version.get("product").and_then(Value::as_u64).unwrap_or(0);
            let type_name = ZnpVersion::from_product(product)
                .map(|version| match version {
                    ZnpVersion::ZStack12 => "ZStack12",
                    ZnpVersion::ZStack3x0 => "ZStack3x0",
                    ZnpVersion::ZStack30x => "ZStack30x",
                })
                .unwrap_or("Unknown")
                .to_string();
            CoordinatorVersion {
                type_name,
                meta: version.clone(),
            }
        })
    }

    /// Upstream `sendZdo(...)`.
    pub async fn send_zdo(
        &mut self,
        ieee_address: &str,
        network_address: u16,
        cluster_id: u16,
        payload: &[u8],
        disable_response: bool,
    ) -> Result<Value, String> {
        self.check_interpan_lock()?;
        let queue = self
            .queue
            .clone()
            .ok_or_else(|| "Adapter not started".to_string())?;
        let znp = self.znp.clone();
        let ieee_address = ieee_address.to_string();
        let payload = payload.to_vec();
        queue
            .execute(
                || async move {
                    let mut znp = znp.lock().await;
                    send_zdo_internal(
                        &mut znp,
                        &ieee_address,
                        network_address,
                        cluster_id,
                        &payload,
                        disable_response,
                    )
                    .await
                },
                Some(network_address.to_string()),
            )
            .await
    }

    /// Upstream `permitJoin(seconds, networkAddress?)`.
    pub async fn permit_join(
        &mut self,
        seconds: u8,
        network_address: Option<u16>,
    ) -> Result<(), String> {
        let cluster_id = ClusterId::PERMIT_JOINING_REQUEST;
        let zdo_payload = BuffaloZdo::build_request(
            self.has_zdo_message_overhead,
            cluster_id,
            &[json!(seconds), json!(1), json!([])],
        );
        if let Some(network_address) = network_address {
            let result = self
                .send_zdo(
                    "0x0000000000000000",
                    network_address,
                    cluster_id,
                    &zdo_payload,
                    false,
                )
                .await?;
            let status = result.get(0).and_then(Value::as_u64).unwrap_or(1);
            if status != 0 {
                return Err(format!("ZDO status error: {status}"));
            }
        } else {
            self.send_zdo("0x0000000000000000", 0xfffc, cluster_id, &zdo_payload, true)
                .await?;
        }
        self.check_interpan_lock()?;
        self.set_led(if seconds == 0 { "off" } else { "on" }).await
    }

    /// Upstream `reset(type)`.
    pub async fn reset(&mut self, kind: &str) -> Result<(), String> {
        let value = match kind {
            "soft" => sys::reset_type::SOFT,
            "hard" => sys::reset_type::HARD,
            other => return Err(format!("Unknown reset type '{other}'")),
        };
        self.znp
            .lock()
            .await
            .request(
                Subsystem::SYS,
                "resetReq",
                json!({"type": value}),
                None,
                None,
                &[0],
            )
            .await?;
        Ok(())
    }

    async fn start_event_loop(&mut self) -> Result<(), String> {
        let mut receiver = self.znp.lock().await.subscribe();
        let waitress = self.waitress.clone();
        let events = self.events.clone();
        let znp = self.znp.clone();
        let queue = self.queue.clone().expect("queue");
        let debouncers = self.device_announce_route_discovery.clone();
        self.listener = Some(tokio::spawn(async move {
            while let Ok(object) = receiver.recv().await {
                dispatch_znp_object(
                    &object,
                    &waitress,
                    &events,
                    Some(&znp),
                    Some(&queue),
                    &debouncers,
                )
                .await;
            }
        }));
        Ok(())
    }

    async fn set_led(&mut self, action: &str) -> Result<(), String> {
        let version = self.version.clone().unwrap_or_default();
        let revision = version
            .get("revision")
            .and_then(Value::as_u64)
            .or_else(|| {
                version
                    .get("revision")
                    .and_then(Value::as_str)
                    .and_then(|value| value.parse().ok())
            })
            .unwrap_or(0);
        if self.supports_led.is_none() {
            let z_stack3x0 = version.get("product").and_then(Value::as_u64)
                == Some(ZnpVersion::ZStack3x0 as u64);
            self.supports_led = Some(!z_stack3x0 || revision >= 20210430);
        }
        if !self.supports_led.unwrap_or(false)
            || (self.adapter_options.disable_led && action != "disable")
        {
            return Ok(());
        }
        let firmware_controls_led = revision >= 20211029;
        let payload = match action {
            "disable" => Some(if firmware_controls_led {
                json!({"ledid": 0xff, "mode": 5})
            } else {
                json!({"ledid": 3, "mode": 0})
            }),
            "on" if !firmware_controls_led => Some(json!({"ledid": 3, "mode": 1})),
            "off" if !firmware_controls_led => Some(json!({"ledid": 3, "mode": 0})),
            _ => None,
        };
        if let Some(payload) = payload {
            let result = self
                .znp
                .lock()
                .await
                .request(
                    Subsystem::UTIL,
                    "ledControl",
                    payload,
                    None,
                    Some(500),
                    &[0],
                )
                .await;
            if result.is_err() {
                self.supports_led = Some(false);
            }
        }
        Ok(())
    }
}

/// Upstream `Events.ZclPayload`.
#[derive(Debug, Clone)]
pub struct ZclPayload {
    pub cluster_id: u16,
    pub data: Vec<u8>,
    pub header: crate::zspec::zcl::zcl_header::Header,
    pub address: Value,
    pub endpoint: u8,
    pub linkquality: u8,
    pub group_id: u16,
    pub was_broadcast: bool,
    pub destination_endpoint: u8,
}

/// Upstream `ClusterWaitressMatcher`.
#[derive(Debug, Clone)]
pub struct ClusterWaitressMatcher {
    pub address: Option<Value>,
    pub cluster_id: u16,
    pub endpoint: Option<u8>,
    pub command_id: Option<u8>,
    pub default_rsp_command_id: Option<u8>,
    pub transaction_sequence_number: Option<u8>,
}

/// Adapter events emitted by `onZnpReceived`.
#[derive(Debug, Clone)]
pub enum AdapterEvent {
    ZdoResponse {
        cluster_id: u16,
        response: Value,
    },
    ZclPayload(ZclPayload),
    DeviceJoined {
        network_address: u16,
        ieee_address: Value,
    },
    DeviceLeave {
        network_address: u16,
        ieee_address: Value,
    },
}

impl ZStackAdapter {
    /// Upstream `onZnpReceived(object)`.
    pub async fn on_znp_received(
        &self,
        object: &crate::adapter::z_stack::znp::zpi_object::ZpiObject,
    ) {
        dispatch_znp_object(
            object,
            &self.waitress,
            &self.events,
            Some(&self.znp),
            self.queue.as_ref(),
            &self.device_announce_route_discovery,
        )
        .await;
    }
}

async fn dispatch_znp_object(
    object: &crate::adapter::z_stack::znp::zpi_object::ZpiObject,
    waitress: &Arc<crate::utils::waitress::Waitress<ZclPayload, ClusterWaitressMatcher>>,
    events: &tokio::sync::broadcast::Sender<AdapterEvent>,
    znp: Option<&Arc<Mutex<Znp>>>,
    queue: Option<&Queue>,
    debouncers: &Arc<std::sync::Mutex<std::collections::HashMap<u16, std::time::Instant>>>,
) {
    if object.r#type != Type::AREQ {
        return;
    }
    let command = crate::adapter::z_stack::znp::definition::find_by_id(
        object.subsystem,
        object.command.id as u8,
    );
    if object.subsystem == Subsystem::ZDO {
        if let Some(command) = command {
            if crate::adapter::z_stack::znp::utils::is_mt_cmd_areq_zdo(command) {
                if let Some(cluster_id) = command.zdo_cluster_id {
                    let _ = events.send(AdapterEvent::ZdoResponse {
                        cluster_id,
                        response: object.payload.get("zdo").cloned().unwrap_or(Value::Null),
                    });
                }
            }
        }
        match object.command.name.as_str() {
            "tcDeviceInd" => {
                let _ = events.send(AdapterEvent::DeviceJoined {
                    network_address: object
                        .payload
                        .get("nwkaddr")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as u16,
                    ieee_address: object
                        .payload
                        .get("extaddr")
                        .cloned()
                        .unwrap_or(Value::Null),
                });
            }
            "endDeviceAnnceInd" => {
                let zdo = object.payload.get("zdo").cloned().unwrap_or(Value::Null);
                if crate::zspec::zdo::buffalo_zdo::BuffaloZdo::check_status(&zdo) {
                    let zdo_payload = zdo.get(1).cloned().unwrap_or(Value::Null);
                    // Only discover routes to end devices, if bit 1 of
                    // capabilities === 0 it's an end device.
                    let is_end_device = zdo_payload
                        .get("capabilities")
                        .and_then(|capabilities| capabilities.get("deviceType"))
                        .and_then(Value::as_u64)
                        == Some(0);
                    if is_end_device {
                        let nwk_address = zdo_payload
                            .get("nwkAddress")
                            .and_then(Value::as_u64)
                            .unwrap_or(0) as u16;
                        // If a device announces multiple times in a very short
                        // time, it makes no sense to rediscover the route every
                        // time. Upstream `debounce(..., 60 * 1000, {immediate:
                        // true})`: the first announce fires immediately, later
                        // ones within 60s are swallowed.
                        let should_discover = {
                            let mut map = debouncers
                                .lock()
                                .expect("device announce debouncers lock poisoned");
                            route_discovery_due(&mut map, nwk_address, std::time::Instant::now())
                        };
                        if should_discover {
                            if let (Some(znp), Some(queue)) = (znp, queue) {
                                let znp = znp.clone();
                                let _ = queue
                                    .execute(
                                        || async move {
                                            let mut znp = znp.lock().await;
                                            let _ =
                                                discover_route(&mut znp, nwk_address, false).await;
                                        },
                                        Some(nwk_address.to_string()),
                                    )
                                    .await;
                            }
                        }
                    }
                }
            }
            "concentratorIndCb" => {
                let _ = events.send(AdapterEvent::ZdoResponse {
                        cluster_id: ClusterId::NETWORK_ADDRESS_RESPONSE,
                        response: json!([
                            0,
                            {
                                "eui64": object.payload.get("extaddr").cloned().unwrap_or(Value::Null),
                                "nwkAddress": object.payload.get("srcaddr").cloned().unwrap_or(Value::Null),
                                "startIndex": 0,
                                "assocDevList": [],
                            }
                        ]),
                    });
            }
            "leaveInd" => {
                if object.payload.get("rejoin").and_then(Value::as_bool) != Some(true) {
                    let _ = events.send(AdapterEvent::DeviceLeave {
                        network_address: object
                            .payload
                            .get("srcaddr")
                            .and_then(Value::as_u64)
                            .unwrap_or(0) as u16,
                        ieee_address: object
                            .payload
                            .get("extaddr")
                            .cloned()
                            .unwrap_or(Value::Null),
                    });
                } else {
                    logger().debug(
                        "Device leave: Got leave indication with rejoin=true, nothing to do",
                        NS,
                    );
                }
            }
            _ => {}
        }
    } else if object.subsystem == Subsystem::AF
        && matches!(
            object.command.name.as_str(),
            "incomingMsg" | "incomingMsgExt"
        )
    {
        let data = bytes(object.payload.get("data"));
        let is_eui64 = object.payload.get("srcaddr").is_some_and(Value::is_string);
        let address = if object.command.name == "incomingMsgExt"
            && object.payload.get("srcaddrmode").and_then(Value::as_u64) != Some(0x03)
        {
            if is_eui64 {
                let text = object
                    .payload
                    .get("srcaddr")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                json!(u16::from_str_radix(&text[text.len().saturating_sub(4)..], 16).unwrap_or(0))
            } else {
                object
                    .payload
                    .get("srcaddr")
                    .cloned()
                    .unwrap_or(Value::Null)
            }
        } else {
            object
                .payload
                .get("srcaddr")
                .cloned()
                .unwrap_or(Value::Null)
        };
        let cluster_id = object
            .payload
            .get("clusterid")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u16;
        let endpoint = object
            .payload
            .get("srcendpoint")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8;
        let linkquality = object
            .payload
            .get("linkquality")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8;
        let group_id = object
            .payload
            .get("groupid")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u16;
        let was_broadcast = object.payload.get("wasbroadcast").and_then(Value::as_u64) == Some(1);
        let destination_endpoint = object
            .payload
            .get("dstendpoint")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8;
        let length = object
            .payload
            .get("len")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize;

        // The ZNP sends `incomingMsgExt` when the data length is > about 223
        // bytes (or if INTER_PAN is used). In the first case the data is not
        // included in the payload and has to be retrieved block by block with
        // `AF_DATA_RETRIEVE`.
        if object.command.name == "incomingMsgExt" && data.len() < length {
            let (Some(znp), Some(queue)) = (znp, queue) else {
                return;
            };
            let znp = znp.clone();
            let waitress = waitress.clone();
            let events = events.clone();
            let timestamp = object
                .payload
                .get("timestamp")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            queue
                .execute(
                    || async move {
                        let mut znp = znp.lock().await;
                        match data_retrieve_all(&mut znp, timestamp, length).await {
                            Some(data) => {
                                logger().debug(
                                    &format!(
                                        "Retrieved {} bytes from huge data buffer for msg with timestamp {timestamp}",
                                        data.len()
                                    ),
                                    NS,
                                );
                                let Some(header) =
                                    crate::zspec::zcl::zcl_header::Header::from_buffer(&data)
                                else {
                                    return;
                                };
                                let payload = ZclPayload {
                                    cluster_id,
                                    data,
                                    header,
                                    address,
                                    endpoint,
                                    linkquality,
                                    group_id,
                                    was_broadcast,
                                    destination_endpoint,
                                };
                                waitress.resolve(&payload).await;
                                let _ = events.send(AdapterEvent::ZclPayload(payload));
                            }
                            None => logger().error(
                                "Failed to retrieve chunked payload for incomingMsgExt",
                                NS,
                            ),
                        }
                    },
                    None,
                )
                .await;
            return;
        }

        let Some(header) = crate::zspec::zcl::zcl_header::Header::from_buffer(&data) else {
            return;
        };
        let payload = ZclPayload {
            cluster_id,
            data,
            header,
            address,
            endpoint,
            linkquality,
            group_id,
            was_broadcast,
            destination_endpoint,
        };
        waitress.resolve(&payload).await;
        let _ = events.send(AdapterEvent::ZclPayload(payload));
    }
}

/// Upstream `dataRetrieveAll(timestamp, length)`.
async fn data_retrieve_all(znp: &mut Znp, timestamp: u64, length: usize) -> Option<Vec<u8>> {
    let mut buffer = vec![0u8; length];
    let block_size = 240usize;

    async fn free_ext_message(znp: &mut Znp, timestamp: u64) {
        // A length of zero is special and triggers the freeing of the
        // corresponding incoming message.
        let _ = znp
            .request_with_reply(
                Subsystem::AF,
                "dataRetrieve",
                json!({"timestamp": timestamp, "index": 0, "length": 0}),
            )
            .await;
    }

    let mut index = 0usize;
    while index < length {
        let chunk_size = block_size.min(length - index);
        let response = match znp
            .request_with_reply(
                Subsystem::AF,
                "dataRetrieve",
                json!({
                    "timestamp": timestamp,
                    "index": index,
                    "length": chunk_size,
                }),
            )
            .await
        {
            Ok(response) => response,
            Err(error) => {
                logger().error(&format!("dataRetrieve failed: {error}"), NS);
                return None;
            }
        };
        // 0x00 = afStatus_SUCCESS
        let status = response
            .payload
            .get("status")
            .and_then(Value::as_u64)
            .unwrap_or(u64::MAX);
        if status != 0 {
            logger().error(
                &format!(
                    "dataRetrieve [timestamp: {timestamp}, index: {index}, chunkSize: {chunk_size}] error status: {status}"
                ),
                NS,
            );
            free_ext_message(znp, timestamp).await;
            return None;
        }
        let returned = response
            .payload
            .get("length")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize;
        if returned != chunk_size {
            logger().error(
                &format!(
                    "dataRetrieve length mismatch [{chunk_size} requested, {returned} returned"
                ),
                NS,
            );
            free_ext_message(znp, timestamp).await;
            return None;
        }
        let data = bytes(response.payload.get("data"));
        let copy = chunk_size.min(data.len());
        buffer[index..index + copy].copy_from_slice(&data[..copy]);
        index += chunk_size;
    }
    free_ext_message(znp, timestamp).await;
    Some(buffer)
}

/// Upstream `debounce(..., 60 * 1000, {immediate: true})` for

/// `deviceAnnounceRouteDiscoveryDebouncers`.
fn route_discovery_due(
    debouncers: &mut std::collections::HashMap<u16, std::time::Instant>,
    nwk_address: u16,
    now: std::time::Instant,
) -> bool {
    match debouncers.get(&nwk_address) {
        Some(last) if now.duration_since(*last) < std::time::Duration::from_secs(60) => false,
        _ => {
            debouncers.insert(nwk_address, now);
            true
        }
    }
}

fn bytes(value: Option<&Value>) -> Vec<u8> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_u64)
                .map(|value| value as u8)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::z_stack::unpi::constants::{Subsystem, Type};
    use crate::adapter::z_stack::znp::zpi_object::ZpiObject;

    fn channel() -> (
        Arc<crate::utils::waitress::Waitress<ZclPayload, ClusterWaitressMatcher>>,
        tokio::sync::broadcast::Sender<AdapterEvent>,
        tokio::sync::broadcast::Receiver<AdapterEvent>,
    ) {
        let waitress = Arc::new(crate::utils::waitress::Waitress::new(
            |payload: &ZclPayload, matcher: &ClusterWaitressMatcher| {
                payload.cluster_id == matcher.cluster_id
                    && matcher.endpoint == Some(payload.endpoint)
            },
            |_matcher: &ClusterWaitressMatcher, timeout: u64| format!("timeout {timeout}"),
        ));
        let (events, receiver) = tokio::sync::broadcast::channel(8);
        (waitress, events, receiver)
    }

    #[tokio::test]
    async fn dispatches_leave_indication() {
        let (waitress, events, mut receiver) = channel();
        let object = ZpiObject::new(
            Type::AREQ,
            Subsystem::ZDO,
            "leaveInd",
            0,
            json!({"rejoin": false, "srcaddr": 0x1234, "extaddr": "0x00124b0024c2d400"}),
            Vec::new(),
        );
        dispatch_znp_object(
            &object,
            &waitress,
            &events,
            None,
            None,
            &Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        )
        .await;
        match receiver.recv().await.expect("event") {
            AdapterEvent::DeviceLeave {
                network_address,
                ieee_address,
            } => {
                assert_eq!(network_address, 0x1234);
                assert_eq!(ieee_address, json!("0x00124b0024c2d400"));
            }
            other => panic!("unexpected event {other:?}"),
        }
    }

    #[tokio::test]
    async fn dispatches_af_payload_and_resolves_waitress() {
        let (waitress, events, mut receiver) = channel();
        let (_, waiter) = waitress
            .wait_for(
                ClusterWaitressMatcher {
                    address: None,
                    cluster_id: 6,
                    endpoint: Some(1),
                    command_id: Some(0),
                    default_rsp_command_id: None,
                    transaction_sequence_number: None,
                },
                1000,
            )
            .await;
        let object = ZpiObject::new(
            Type::AREQ,
            Subsystem::AF,
            "incomingMsg",
            0,
            json!({
                "groupid": 0,
                "clusterid": 6,
                "srcaddr": 0x1234,
                "srcendpoint": 1,
                "dstendpoint": 1,
                "wasbroadcast": 0,
                "linkquality": 100,
                "securityuse": 0,
                "timestamp": 0,
                "transseqnumber": 1,
                "len": 3,
                "data": [0x00, 0x01, 0x00],
            }),
            Vec::new(),
        );
        dispatch_znp_object(
            &object,
            &waitress,
            &events,
            None,
            None,
            &Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        )
        .await;
        let payload = waiter.await.expect("waiter").expect("payload");
        assert_eq!(payload.cluster_id, 6);
        assert_eq!(payload.endpoint, 1);
        match receiver.recv().await.expect("event") {
            AdapterEvent::ZclPayload(payload) => assert_eq!(payload.data, vec![0, 1, 0]),
            other => panic!("unexpected event {other:?}"),
        }
    }

    #[tokio::test]
    async fn device_announce_route_discovery_is_debounced() {
        let mut debouncers = std::collections::HashMap::new();
        let start = std::time::Instant::now();

        // First announce fires immediately.
        assert!(route_discovery_due(&mut debouncers, 4321, start));
        // A second announce within 60s is swallowed.
        assert!(!route_discovery_due(
            &mut debouncers,
            4321,
            start + std::time::Duration::from_secs(59)
        ));
        // Another device is independent.
        assert!(route_discovery_due(&mut debouncers, 1234, start));
        // After the 60s window the route is discovered again.
        assert!(route_discovery_due(
            &mut debouncers,
            4321,
            start + std::time::Duration::from_secs(60)
        ));
    }

    #[tokio::test]
    async fn schedules_route_discovery_only_for_end_devices() {
        let (waitress, events, _receiver) = channel();
        let debouncers = Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));

        let end_device = ZpiObject::new(
            Type::AREQ,
            Subsystem::ZDO,
            "endDeviceAnnceInd",
            0,
            json!({"zdo": [0, {
                "nwkAddress": 4321,
                "eui64": "0x00124b0024c2d400",
                "capabilities": {
                    "alternatePANCoordinator": 0,
                    "deviceType": 0,
                    "powerSource": 1,
                    "rxOnWhenIdle": 0,
                },
            }]}),
            Vec::new(),
        );
        dispatch_znp_object(&end_device, &waitress, &events, None, None, &debouncers).await;
        assert!(debouncers.lock().unwrap().contains_key(&4321));

        let router = ZpiObject::new(
            Type::AREQ,
            Subsystem::ZDO,
            "endDeviceAnnceInd",
            0,
            json!({"zdo": [0, {
                "nwkAddress": 1234,
                "eui64": "0x00124b0024c2d401",
                "capabilities": {"deviceType": 1},
            }]}),
            Vec::new(),
        );
        dispatch_znp_object(&router, &waitress, &events, None, None, &debouncers).await;
        assert!(!debouncers.lock().unwrap().contains_key(&1234));
    }

    #[tokio::test]
    async fn retrieves_chunked_af_payload() {
        use crate::adapter::transport::{AdapterTransport, TransportOptions};
        use crate::adapter::z_stack::unpi::frame::Frame as UnpiFrame;
        use crate::adapter::z_stack::znp::zpi_object::ZpiObject;
        use tokio::io::AsyncWriteExt;

        let (reader, mut peer) = serial2_tokio::SerialPort::pair().expect("pair");
        let mut transport = AdapterTransport::new(TransportOptions::new("/dev/null"));
        transport.attach_serial(reader).expect("attach");
        let mut znp = Znp::new(transport);

        let data = (0..500u16)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<u8>>();
        let expected = data.clone();

        let server = tokio::spawn(async move {
            let mut buffer = [0u8; 256];
            let mut pending = Vec::new();
            loop {
                let read = match peer.read(&mut buffer).await {
                    Ok(0) => return,
                    Ok(read) => read,
                    Err(_) => return,
                };
                pending.extend_from_slice(&buffer[..read]);
                loop {
                    if pending.len() < 5 {
                        break;
                    }
                    let length = pending[1] as usize;
                    let frame_length = 4 + length + 1;
                    if pending.len() < frame_length {
                        break;
                    }
                    let raw = pending[..frame_length].to_vec();
                    pending.drain(..frame_length);
                    let frame =
                        UnpiFrame::from_buffer(length as u8, 4 + length, &raw).expect("frame");
                    let object = ZpiObject::from_unpi_frame(&frame);
                    let response = if object.command.name == "ping" {
                        vec![0x01, 0x00]
                    } else {
                        assert_eq!(object.command.name, "dataRetrieve");
                        let index = object.payload["index"].as_u64().unwrap() as usize;
                        let requested = object.payload["length"].as_u64().unwrap() as usize;
                        let chunk = &data[index..(index + requested).min(data.len())];
                        let mut data = vec![0x00, chunk.len() as u8];
                        data.extend_from_slice(chunk);
                        data
                    };
                    let response = UnpiFrame::new(
                        Type::SRSP,
                        frame.subsystem,
                        frame.commandID,
                        response,
                        0,
                        0,
                    );
                    peer.write_all(&response.to_buffer()).await.expect("write");
                }
            }
        });

        znp.open().await.expect("open");
        let result = data_retrieve_all(&mut znp, 0x11223344, 500).await;
        assert_eq!(result, Some(expected));
        znp.close().await.expect("close");
        server.abort();
    }
}

impl ZStackAdapter {
    fn next_transaction_id(&mut self) -> u8 {
        self.transaction_id = self.transaction_id.wrapping_add(1);
        if self.transaction_id == 0 {
            self.transaction_id = 1;
        }
        self.transaction_id
    }

    fn to_address_string(address: &Value) -> String {
        match address {
            Value::Number(value) => format!("0x{:016x}", value.as_u64().unwrap_or(0)),
            Value::String(value) => value.clone(),
            _ => String::new(),
        }
    }

    fn select_source_endpoint(
        &self,
        source_endpoint: Option<u8>,
        profile_id: Option<u16>,
    ) -> Result<u8, String> {
        let mut endpoint = source_endpoint;
        if endpoint.is_none() {
            if let Some(profile_id) = profile_id {
                endpoint = endpoints()
                    .into_iter()
                    .find(|entry| entry.appprofid == profile_id)
                    .map(|entry| entry.endpoint);
            }
        }
        let endpoint = endpoint.unwrap_or(1);
        if let Some(profile_id) = profile_id {
            if endpoints()
                .into_iter()
                .find(|entry| entry.endpoint == endpoint)
                .map(|entry| entry.appprofid)
                != Some(profile_id)
            {
                return Err(format!(
                    "Profile ID {profile_id} is not supported by this adapter."
                ));
            }
        }
        Ok(endpoint)
    }

    async fn data_request(
        &mut self,
        destination_address: u16,
        destination_endpoint: u8,
        source_endpoint: u8,
        cluster_id: u16,
        radius: u8,
        data: &[u8],
        timeout: u64,
    ) -> u16 {
        let transaction_id = self.next_transaction_id();
        let mut znp = self.znp.lock().await;
        let (waiter_id, receiver) = znp
            .wait_for(
                Type::AREQ,
                Subsystem::AF,
                "dataConfirm",
                None,
                Some(transaction_id),
                None,
                Some(timeout),
            )
            .await;
        let options = if cluster_id == 0x003b {
            crate::adapter::z_stack::constants::af::options::EN_SECURITY as u8
        } else {
            0
        };
        if let Err(error) = znp
            .request(
                Subsystem::AF,
                "dataRequest",
                json!({
                    "dstaddr": destination_address,
                    "destendpoint": destination_endpoint,
                    "srcendpoint": source_endpoint,
                    "clusterid": cluster_id,
                    "transid": transaction_id,
                    "options": options,
                    "radius": radius,
                    "len": data.len(),
                    "data": data,
                }),
                Some(waiter_id),
                None,
                &[0],
            )
            .await
        {
            logger().debug(&format!("dataRequest failed: {error}"), NS);
        }
        match receiver.await {
            Ok(Ok(confirm)) => confirm
                .payload
                .get("status")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u16,
            _ => 9999,
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn data_request_extended(
        &mut self,
        address_mode: AddressMode,
        destination_address_or_group_id: Value,
        destination_endpoint: u8,
        pan_id: u16,
        source_endpoint: u8,
        cluster_id: u16,
        radius: u8,
        data: &[u8],
        timeout: u64,
        confirmation: bool,
        attempts_left: u8,
    ) -> Result<Option<crate::adapter::z_stack::znp::zpi_object::ZpiObject>, String> {
        let transaction_id = self.next_transaction_id();
        let mut znp = self.znp.lock().await;
        let waiter = if confirmation {
            Some(
                znp.wait_for(
                    Type::AREQ,
                    Subsystem::AF,
                    "dataConfirm",
                    None,
                    Some(transaction_id),
                    None,
                    Some(timeout),
                )
                .await,
            )
        } else {
            None
        };
        znp.request(
            Subsystem::AF,
            "dataRequestExt",
            json!({
                "dstaddrmode": address_mode as u8,
                "dstaddr": Self::to_address_string(&destination_address_or_group_id),
                "destendpoint": destination_endpoint,
                "dstpanid": pan_id,
                "srcendpoint": source_endpoint,
                "clusterid": cluster_id,
                "transid": transaction_id,
                "options": 0,
                "radius": radius,
                "len": data.len(),
                "data": data,
            }),
            waiter.as_ref().map(|(id, _)| *id),
            None,
            &[0],
        )
        .await?;

        if let Some((_, receiver)) = waiter {
            let confirm = receiver.await.map_err(|_| "Waiter dropped".to_string())??;
            let status = confirm
                .payload
                .get("status")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            if status != 0 {
                if attempts_left > 0
                    && (status == ZnpCommandStatus::MAC_CHANNEL_ACCESS_FAILURE as u64
                        || status == ZnpCommandStatus::BUFFER_FULL as u64)
                {
                    wait(2000).await;
                    drop(znp);
                    return Box::pin(self.data_request_extended(
                        address_mode,
                        destination_address_or_group_id,
                        destination_endpoint,
                        pan_id,
                        source_endpoint,
                        cluster_id,
                        radius,
                        data,
                        timeout,
                        confirmation,
                        attempts_left - 1,
                    ))
                    .await;
                }
                return Err(format!("Data request failed with error: '{status}'"));
            }
            return Ok(Some(confirm));
        }
        Ok(None)
    }

    /// Upstream `sendZclFrameToGroup(groupID, zclFrame, sourceEndpoint?, profileId?)`.
    pub async fn send_zcl_frame_to_group(
        &mut self,
        group_id: u16,
        zcl_frame: &crate::zspec::zcl::zcl_frame::Frame,
        source_endpoint: Option<u8>,
        profile_id: Option<u16>,
    ) -> Result<(), String> {
        let source_endpoint = self.select_source_endpoint(source_endpoint, profile_id)?;
        self.check_interpan_lock()?;
        let queue = self
            .queue
            .clone()
            .ok_or_else(|| "Adapter not started".to_string())?;
        let data = zcl_frame.to_buffer();
        let cluster_id = zcl_frame.cluster.ID as u16;
        queue
            .execute(
                || async move {
                    self.data_request_extended(
                        AddressMode::ADDR_GROUP,
                        json!(group_id),
                        0xff,
                        0,
                        source_endpoint,
                        cluster_id,
                        DEFAULT_RADIUS as u8,
                        &data,
                        3000,
                        true,
                        5,
                    )
                    .await?;
                    wait(200).await;
                    Ok(())
                },
                None,
            )
            .await
    }

    /// Upstream `sendZclFrameToAll(...)`.
    pub async fn send_zcl_frame_to_all(
        &mut self,
        endpoint: u8,
        zcl_frame: &crate::zspec::zcl::zcl_frame::Frame,
        source_endpoint: u8,
        destination: u16,
        profile_id: Option<u16>,
    ) -> Result<(), String> {
        let source_endpoint = self.select_source_endpoint(Some(source_endpoint), profile_id)?;
        self.check_interpan_lock()?;
        let queue = self
            .queue
            .clone()
            .ok_or_else(|| "Adapter not started".to_string())?;
        let data = zcl_frame.to_buffer();
        let cluster_id = zcl_frame.cluster.ID as u16;
        queue
            .execute(
                || async move {
                    self.data_request_extended(
                        AddressMode::ADDR_16BIT,
                        json!(destination),
                        endpoint,
                        0,
                        source_endpoint,
                        cluster_id,
                        DEFAULT_RADIUS as u8,
                        &data,
                        3000,
                        false,
                        0,
                    )
                    .await?;
                    wait(200).await;
                    Ok(())
                },
                None,
            )
            .await
    }
}

impl ZStackAdapter {
    fn check_interpan_lock(&self) -> Result<(), String> {
        if self.interpan_lock {
            return Err("Cannot execute command, in Inter-PAN mode".to_string());
        }
        Ok(())
    }

    fn revision(&self) -> u64 {
        self.version
            .as_ref()
            .and_then(|version| {
                version.get("revision").and_then(Value::as_u64).or_else(|| {
                    version
                        .get("revision")
                        .and_then(Value::as_str)
                        .and_then(|value| value.parse().ok())
                })
            })
            .unwrap_or(0)
    }

    fn supports_assoc_remove(&self) -> bool {
        self.version
            .as_ref()
            .and_then(|v| v.get("product"))
            .and_then(Value::as_u64)
            == Some(ZnpVersion::ZStack3x0 as u64)
            && self.revision() >= 20200805
    }

    fn supports_assoc_add(&self) -> bool {
        self.version
            .as_ref()
            .and_then(|v| v.get("product"))
            .and_then(Value::as_u64)
            == Some(ZnpVersion::ZStack3x0 as u64)
            && self.revision() >= 20201026
    }

    /// Upstream `requestNetworkAddress(ieeeAddr)`.
    pub async fn request_network_address(&mut self, ieee_address: &str) -> Result<u16, String> {
        let cluster_id = ClusterId::NETWORK_ADDRESS_REQUEST;
        let payload = BuffaloZdo::build_request(
            self.has_zdo_message_overhead,
            cluster_id,
            &[json!(ieee_address), json!(false), json!(0)],
        );
        let mut znp = self.znp.lock().await;
        let result =
            send_zdo_internal(&mut znp, ieee_address, 0xffff, cluster_id, &payload, false).await?;
        let status = result.get(0).and_then(Value::as_u64).unwrap_or(1);
        if status == 0 {
            return Ok(result
                .get(1)
                .and_then(|payload| payload.get("nwkAddress"))
                .and_then(Value::as_u64)
                .unwrap_or(0) as u16);
        }
        Err(format!("ZDO status error: {status}"))
    }

    async fn discover_route(&self, network_address: u16, wait_settled: bool) -> Result<(), String> {
        logger().debug(&format!("Discovering route to {network_address}"), NS);
        let mut znp = self.znp.lock().await;
        znp.request(
            Subsystem::ZDO,
            "extRouteDisc",
            json!({"dstAddr": network_address, "options": 0, "radius": DEFAULT_RADIUS}),
            None,
            None,
            &[0],
        )
        .await?;
        if wait_settled {
            wait(3000).await;
        }
        Ok(())
    }

    /// Upstream `sendZclFrameToEndpoint(...)`.
    #[allow(clippy::too_many_arguments)]
    pub async fn send_zcl_frame_to_endpoint(
        &mut self,
        ieee_address: &str,
        network_address: u16,
        endpoint: u8,
        zcl_frame: &crate::zspec::zcl::zcl_frame::Frame,
        timeout: u64,
        disable_response: bool,
        disable_recovery: bool,
        source_endpoint: Option<u8>,
        profile_id: Option<u16>,
    ) -> Result<Option<ZclPayload>, String> {
        let source_endpoint = self.select_source_endpoint(source_endpoint, profile_id)?;
        self.check_interpan_lock()?;
        let queue = self
            .queue
            .clone()
            .ok_or_else(|| "Adapter not started".to_string())?;
        let ieee = ieee_address.to_string();
        let frame = zcl_frame.clone();
        queue
            .execute(
                || async move {
                    Box::pin(self.send_zcl_frame_to_endpoint_internal(
                        &ieee,
                        network_address,
                        endpoint,
                        source_endpoint,
                        &frame,
                        timeout,
                        disable_response,
                        disable_recovery,
                        0,
                        0,
                        false,
                        false,
                        false,
                        None,
                    ))
                    .await
                },
                Some(network_address.to_string()),
            )
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn send_zcl_frame_to_endpoint_internal(
        &mut self,
        ieee_address: &str,
        mut network_address: u16,
        endpoint: u8,
        source_endpoint: u8,
        zcl_frame: &crate::zspec::zcl::zcl_frame::Frame,
        timeout: u64,
        disable_response: bool,
        disable_recovery: bool,
        response_attempt: u8,
        data_request_attempt: u8,
        mut checked_network_address: bool,
        mut discovered_route: bool,
        mut assoc_remove: bool,
        mut assoc_restore: Option<(String, u16, u8)>,
    ) -> Result<Option<ZclPayload>, String> {
        logger().debug(
            &format!(
                "sendZclFrameToEndpointInternal {ieee_address}:{network_address}/{endpoint} ({response_attempt},{data_request_attempt},{})",
                self.queue.as_ref().map(Queue::count).unwrap_or(0)
            ),
            NS,
        );
        let command = &zcl_frame.command;
        let mut response = None;
        if command.response.is_some() && !disable_response {
            response = Some(
                self.wait_for_internal(
                    Some(network_address),
                    endpoint,
                    Some(zcl_frame.header.transaction_sequence_number),
                    zcl_frame.cluster.ID as u16,
                    command.response.unwrap(),
                    None,
                    timeout,
                )
                .await,
            );
        } else if !zcl_frame.header.frame_control.disable_default_response {
            response = Some(
                self.wait_for_internal(
                    Some(network_address),
                    endpoint,
                    Some(zcl_frame.header.transaction_sequence_number),
                    zcl_frame.cluster.ID as u16,
                    0x0b,
                    Some(command.ID),
                    timeout,
                )
                .await,
            );
        }

        let data = zcl_frame.to_buffer();
        let data_confirm = self
            .data_request(
                network_address,
                endpoint,
                source_endpoint,
                zcl_frame.cluster.ID as u16,
                DEFAULT_RADIUS as u8,
                &data,
                timeout,
            )
            .await;

        if data_confirm != ZnpCommandStatus::SUCCESS as u16 {
            if let Some((waiter_id, _)) = response.take() {
                self.waitress.remove(waiter_id).await;
            }
            logger().debug(
                &format!(
                    "Data confirm error ({ieee_address}:{network_address},{data_confirm},{data_request_attempt})"
                ),
                NS,
            );
            if assoc_remove && assoc_restore.is_some() && self.supports_assoc_add() {
                let (ieee, nwk, relation) = assoc_restore.clone().unwrap();
                logger().debug(&format!("assocAdd({ieee})"), NS);
                self.znp
                    .lock()
                    .await
                    .request(
                        Subsystem::UTIL,
                        "assocAdd",
                        json!({"ieeeadr": ieee, "nwkaddr": nwk, "noderelation": relation}),
                        None,
                        None,
                        &[0],
                    )
                    .await?;
                assoc_restore = None;
            }

            let recoverable = [
                ZnpCommandStatus::NWK_NO_ROUTE as u16,
                ZnpCommandStatus::MAC_NO_ACK as u16,
                ZnpCommandStatus::MAC_CHANNEL_ACCESS_FAILURE as u16,
                ZnpCommandStatus::MAC_TRANSACTION_EXPIRED as u16,
                ZnpCommandStatus::BUFFER_FULL as u16,
                ZnpCommandStatus::MAC_NO_RESOURCES as u16,
            ];
            if data_request_attempt >= 4 || !recoverable.contains(&data_confirm) || disable_recovery
            {
                return Err(format!("Data request failed with error: '{data_confirm}'"));
            }
            if data_confirm == ZnpCommandStatus::MAC_CHANNEL_ACCESS_FAILURE as u16
                || data_confirm == ZnpCommandStatus::BUFFER_FULL as u16
                || data_confirm == ZnpCommandStatus::MAC_NO_RESOURCES as u16
            {
                wait(2000).await;
                return Box::pin(self.send_zcl_frame_to_endpoint_internal(
                    ieee_address,
                    network_address,
                    endpoint,
                    source_endpoint,
                    zcl_frame,
                    timeout,
                    disable_response,
                    disable_recovery,
                    response_attempt,
                    data_request_attempt + 1,
                    checked_network_address,
                    discovered_route,
                    assoc_remove,
                    assoc_restore,
                ))
                .await;
            }

            let mut do_assoc_remove = false;
            if !assoc_remove
                && data_confirm == ZnpCommandStatus::MAC_TRANSACTION_EXPIRED as u16
                && data_request_attempt >= 1
                && self.supports_assoc_remove()
            {
                let match_response = self
                    .znp
                    .lock()
                    .await
                    .request_with_reply(
                        Subsystem::UTIL,
                        "assocGetWithAddress",
                        json!({"extaddr": ieee_address, "nwkaddr": network_address}),
                    )
                    .await?;
                let nwkaddr = match_response
                    .payload
                    .get("nwkaddr")
                    .and_then(Value::as_u64)
                    .unwrap_or(0xffff);
                let noderelation = match_response
                    .payload
                    .get("noderelation")
                    .and_then(Value::as_u64)
                    .unwrap_or(255);
                if nwkaddr != 0xfffe && noderelation != 255 {
                    do_assoc_remove = true;
                    assoc_restore = Some((
                        ieee_address.to_string(),
                        network_address,
                        noderelation as u8,
                    ));
                }
                assoc_remove = true;
            }

            if do_assoc_remove {
                logger().debug(&format!("assocRemove({ieee_address})"), NS);
                self.znp
                    .lock()
                    .await
                    .request(
                        Subsystem::UTIL,
                        "assocRemove",
                        json!({"ieeeadr": ieee_address}),
                        None,
                        None,
                        &[0],
                    )
                    .await?;
            } else if !discovered_route && data_request_attempt >= 1 {
                discovered_route = true;
                self.discover_route(network_address, true).await?;
            } else if !checked_network_address && data_request_attempt >= 1 {
                checked_network_address = true;
                if let Ok(actual) = self.request_network_address(ieee_address).await {
                    if actual != network_address {
                        logger().debug(
                            "Failed because request was done with wrong network address",
                            NS,
                        );
                        discovered_route = true;
                        network_address = actual;
                        self.discover_route(actual, true).await?;
                    } else {
                        logger().debug("Network address did not change", NS);
                    }
                }
            } else {
                wait(2000).await;
            }

            return Box::pin(self.send_zcl_frame_to_endpoint_internal(
                ieee_address,
                network_address,
                endpoint,
                source_endpoint,
                zcl_frame,
                timeout,
                disable_response,
                disable_recovery,
                response_attempt,
                data_request_attempt + 1,
                checked_network_address,
                discovered_route,
                assoc_remove,
                assoc_restore,
            ))
            .await;
        }

        if let Some((_, receiver)) = response {
            match receiver.await {
                Ok(Ok(payload)) => return Ok(Some(payload)),
                Ok(Err(error)) => {
                    logger().debug(
                        &format!(
                            "Response timeout ({ieee_address}:{network_address},{response_attempt}): {error}"
                        ),
                        NS,
                    );
                    if response_attempt < 1 && !disable_recovery {
                        let match_response = self
                            .znp
                            .lock()
                            .await
                            .request_with_reply(
                                Subsystem::UTIL,
                                "assocGetWithAddress",
                                json!({"extaddr": ieee_address, "nwkaddr": network_address}),
                            )
                            .await?;
                        let nwkaddr = match_response
                            .payload
                            .get("nwkaddr")
                            .and_then(Value::as_u64)
                            .unwrap_or(0xffff);
                        let noderelation = match_response
                            .payload
                            .get("noderelation")
                            .and_then(Value::as_u64)
                            .unwrap_or(255);
                        logger().debug(
                            &format!(
                                "Response timeout recovery: Node relation {noderelation} ({ieee_address} / {nwkaddr})"
                            ),
                            NS,
                        );
                        if self.supports_assoc_add()
                            && self.supports_assoc_remove()
                            && nwkaddr != 0xfffe
                            && noderelation == 1
                        {
                            logger().debug(
                                &format!("Response timeout recovery: Rewrite association table entry ({ieee_address})"),
                                NS,
                            );
                            self.znp
                                .lock()
                                .await
                                .request(
                                    Subsystem::UTIL,
                                    "assocRemove",
                                    json!({"ieeeadr": ieee_address}),
                                    None,
                                    None,
                                    &[0],
                                )
                                .await?;
                            self.znp
                                .lock()
                                .await
                                .request(
                                    Subsystem::UTIL,
                                    "assocAdd",
                                    json!({"ieeeadr": ieee_address, "nwkaddr": network_address, "noderelation": noderelation}),
                                    None,
                                    None,
                                    &[0],
                                )
                                .await?;
                        }
                        self.discover_route(network_address, true).await?;
                        return Box::pin(self.send_zcl_frame_to_endpoint_internal(
                            ieee_address,
                            network_address,
                            endpoint,
                            source_endpoint,
                            zcl_frame,
                            timeout,
                            disable_response,
                            disable_recovery,
                            response_attempt + 1,
                            data_request_attempt,
                            checked_network_address,
                            discovered_route,
                            assoc_remove,
                            assoc_restore,
                        ))
                        .await;
                    }
                    return Err(error);
                }
                Err(_) => return Err("Waiter dropped".to_string()),
            }
        }
        Ok(None)
    }

    async fn wait_for_internal(
        &self,
        network_address: Option<u16>,
        endpoint: u8,
        transaction_sequence_number: Option<u8>,
        cluster_id: u16,
        command_id: u8,
        default_rsp_command_id: Option<u8>,
        timeout: u64,
    ) -> (
        u64,
        tokio::sync::oneshot::Receiver<Result<ZclPayload, String>>,
    ) {
        self.waitress
            .wait_for(
                ClusterWaitressMatcher {
                    address: network_address.map(|address| json!(address)),
                    cluster_id,
                    endpoint: Some(endpoint),
                    command_id: Some(command_id),
                    default_rsp_command_id,
                    transaction_sequence_number,
                },
                timeout,
            )
            .await
    }
}

/// Upstream `NetworkParameters`.
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkParameters {
    pub pan_id: u16,
    pub extended_pan_id: String,
    pub channel: u8,
    pub nwk_update_id: u8,
}

impl ZStackAdapter {
    /// Upstream `getNetworkParameters()`.
    pub async fn get_network_parameters(&mut self) -> Result<NetworkParameters, String> {
        let result = self
            .znp
            .lock()
            .await
            .request_with_reply(Subsystem::ZDO, "extNwkInfo", json!({}))
            .await?;
        Ok(NetworkParameters {
            pan_id: result
                .payload
                .get("panid")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u16,
            extended_pan_id: result
                .payload
                .get("extendedpanid")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            channel: result
                .payload
                .get("channel")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u8,
            nwk_update_id: 0,
        })
    }

    /// Upstream `supportsBackup()`.
    pub async fn supports_backup(&self) -> bool {
        true
    }

    /// Upstream `backup(ieeeAddressesInDatabase)`.
    pub async fn backup(
        &mut self,
        ieee_addresses_in_database: &[String],
    ) -> Result<crate::models::backup::Backup, String> {
        let mut nv = AdapterNvMemory::from_shared(self.znp.clone());
        nv.init().await?;
        let mut backup = AdapterBackup::new(nv, self.backup_path.clone());
        backup.create_backup(ieee_addresses_in_database).await
    }

    /// Upstream `addInstallCode(ieeeAddress, key, hashed)`.
    pub async fn add_install_code(
        &mut self,
        ieee_address: &str,
        key: &[u8],
        hashed: bool,
    ) -> Result<(), String> {
        let product = self
            .version
            .as_ref()
            .and_then(|version| version.get("product"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if product == ZnpVersion::ZStack12 as u64 {
            return Err("Install code is not supported for ZStack 1.2 adapter".to_string());
        }
        self.znp
            .lock()
            .await
            .request(
                Subsystem::APP_CNF,
                "bdbAddInstallCode",
                json!({
                    "installCodeFormat": if hashed { 0x2 } else { 0x1 },
                    "ieeeaddr": ieee_address,
                    "installCode": key,
                }),
                None,
                None,
                &[0],
            )
            .await?;
        Ok(())
    }

    /// Upstream `setChannelInterPAN(channel)`.
    pub async fn set_channel_inter_pan(&mut self, channel: u8) -> Result<(), String> {
        let queue = self
            .queue
            .clone()
            .ok_or_else(|| "Adapter not started".to_string())?;
        queue
            .execute(
                || async move {
                    self.interpan_lock = true;
                    {
                        let mut znp = self.znp.lock().await;
                        znp.request(
                            Subsystem::AF,
                            "interPanCtl",
                            json!({"cmd": 1, "data": [channel]}),
                            None,
                            None,
                            &[0],
                        )
                        .await?;
                    }
                    if !self.interpan_endpoint_registered {
                        self.znp
                            .lock()
                            .await
                            .request(
                                Subsystem::AF,
                                "interPanCtl",
                                json!({"cmd": 2, "data": [12]}),
                                None,
                                None,
                                &[0],
                            )
                            .await?;
                        self.interpan_endpoint_registered = true;
                    }
                    Ok(())
                },
                None,
            )
            .await
    }

    /// Upstream `sendZclFrameInterPANToIeeeAddr(zclFrame, ieeeAddr)`.
    pub async fn send_zcl_frame_inter_pan_to_ieee_addr(
        &mut self,
        zcl_frame: &crate::zspec::zcl::zcl_frame::Frame,
        ieee_address: &str,
    ) -> Result<(), String> {
        let queue = self
            .queue
            .clone()
            .ok_or_else(|| "Adapter not started".to_string())?;
        let data = zcl_frame.to_buffer();
        let cluster_id = zcl_frame.cluster.ID as u16;
        let ieee_address = ieee_address.to_string();
        queue
            .execute(
                || async move {
                    self.data_request_extended(
                        AddressMode::ADDR_64BIT,
                        json!(ieee_address),
                        0xfe,
                        0xffff,
                        12,
                        cluster_id,
                        30,
                        &data,
                        10000,
                        false,
                        0,
                    )
                    .await?;
                    Ok(())
                },
                None,
            )
            .await
    }

    /// Upstream `sendZclFrameInterPANBroadcast(zclFrame, timeout, disableResponse)`.
    pub async fn send_zcl_frame_inter_pan_broadcast(
        &mut self,
        zcl_frame: &crate::zspec::zcl::zcl_frame::Frame,
        timeout: u64,
        disable_response: bool,
    ) -> Result<Option<ZclPayload>, String> {
        let queue = self
            .queue
            .clone()
            .ok_or_else(|| "Adapter not started".to_string())?;
        let data = zcl_frame.to_buffer();
        let cluster_id = zcl_frame.cluster.ID as u16;
        let command = zcl_frame.command.clone();
        queue
            .execute(
                || async move {
                    if !disable_response && command.response.is_none() {
                        return Err(format!(
                            "Command '{}' has no response, cannot wait for response",
                            command.name
                        ));
                    }
                    let response = if !disable_response {
                        let response = command.response.expect("response");
                        Some(
                            self.wait_for_internal(
                                None, 0xfe, None, cluster_id, response, None, timeout,
                            )
                            .await,
                        )
                    } else {
                        None
                    };
                    let result = self
                        .data_request_extended(
                            AddressMode::ADDR_16BIT,
                            json!(0xffff),
                            0xfe,
                            0xffff,
                            12,
                            cluster_id,
                            30,
                            &data,
                            10000,
                            false,
                            0,
                        )
                        .await;
                    if let Err(error) = result {
                        if let Some((waiter_id, _)) = response.as_ref() {
                            self.waitress.remove(*waiter_id).await;
                        }
                        return Err(error);
                    }
                    if let Some((_, receiver)) = response {
                        return Ok(Some(
                            receiver.await.map_err(|_| "Waiter dropped".to_string())??,
                        ));
                    }
                    Ok(None)
                },
                None,
            )
            .await
    }

    /// Upstream `restoreChannelInterPAN()`.
    pub async fn restore_channel_inter_pan(&mut self) -> Result<(), String> {
        let queue = self
            .queue
            .clone()
            .ok_or_else(|| "Adapter not started".to_string())?;
        queue
            .execute(
                || async move {
                    self.znp
                        .lock()
                        .await
                        .request(
                            Subsystem::AF,
                            "interPanCtl",
                            json!({"cmd": 0, "data": []}),
                            None,
                            None,
                            &[0],
                        )
                        .await?;
                    wait(3000).await;
                    self.interpan_lock = false;
                    Ok(())
                },
                None,
            )
            .await
    }
}

impl ZStackAdapter {
    /// Upstream `waitFor(networkAddress, endpoint, frameType, direction, ...)`.
    #[allow(clippy::too_many_arguments)]
    pub async fn wait_for(
        &self,
        network_address: u16,
        endpoint: u8,
        _frame_type: crate::zspec::zcl::zcl_header::FrameType,
        _direction: crate::zspec::zcl::zcl_header::Direction,
        transaction_sequence_number: Option<u8>,
        cluster_id: u16,
        command_id: u8,
        default_rsp_command_id: Option<u8>,
        timeout: u64,
    ) -> (
        u64,
        tokio::sync::oneshot::Receiver<Result<ZclPayload, String>>,
    ) {
        self.wait_for_internal(
            Some(network_address),
            endpoint,
            transaction_sequence_number,
            cluster_id,
            command_id,
            default_rsp_command_id,
            timeout,
        )
        .await
    }
}
