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
