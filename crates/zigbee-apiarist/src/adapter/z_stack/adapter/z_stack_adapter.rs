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
use crate::adapter::z_stack::adapter::manager::{StartResult, ZnpAdapterManager};
use crate::adapter::z_stack::adapter::tstype::ZnpVersion;
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
