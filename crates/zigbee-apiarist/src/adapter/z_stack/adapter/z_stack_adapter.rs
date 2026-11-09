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
            discover_route(znp, network_address).await?;
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
async fn discover_route(znp: &mut Znp, network_address: u16) -> Result<(), String> {
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
    wait(3000).await;
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
        self.listener = Some(tokio::spawn(async move {
            let _ = (znp, queue);
            while let Ok(object) = receiver.recv().await {
                dispatch_znp_object(&object, &waitress, &events).await;
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
                .request_with_reply(Subsystem::UTIL, "ledControl", payload)
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
        dispatch_znp_object(object, &self.waitress, &self.events).await;
    }
}

async fn dispatch_znp_object(
    object: &crate::adapter::z_stack::znp::zpi_object::ZpiObject,
    waitress: &crate::utils::waitress::Waitress<ZclPayload, ClusterWaitressMatcher>,
    events: &tokio::sync::broadcast::Sender<AdapterEvent>,
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
            if crate::adapter::z_stack::znp::definition::is_mt_cmd_areq_zdo(command) {
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
        let Some(header) = crate::zspec::zcl::zcl_header::Header::from_buffer(&data) else {
            return;
        };
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
        let payload = ZclPayload {
            cluster_id: object
                .payload
                .get("clusterid")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u16,
            data,
            header,
            address,
            endpoint: object
                .payload
                .get("srcendpoint")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u8,
            linkquality: object
                .payload
                .get("linkquality")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u8,
            group_id: object
                .payload
                .get("groupid")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u16,
            was_broadcast: object.payload.get("wasbroadcast").and_then(Value::as_u64) == Some(1),
            destination_endpoint: object
                .payload
                .get("dstendpoint")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u8,
        };
        waitress.resolve(&payload).await;
        let _ = events.send(AdapterEvent::ZclPayload(payload));
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
        dispatch_znp_object(&object, &waitress, &events).await;
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
        dispatch_znp_object(&object, &waitress, &events).await;
        let payload = waiter.await.expect("waiter").expect("payload");
        assert_eq!(payload.cluster_id, 6);
        assert_eq!(payload.endpoint, 1);
        match receiver.recv().await.expect("event") {
            AdapterEvent::ZclPayload(payload) => assert_eq!(payload.data, vec![0, 1, 0]),
            other => panic!("unexpected event {other:?}"),
        }
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

    fn to_address_string(address: u16) -> String {
        format!("0x{address:016x}")
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
        destination_address_or_group_id: u16,
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
                "dstaddr": Self::to_address_string(destination_address_or_group_id),
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
                        group_id,
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
                        destination,
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
