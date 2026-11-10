// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/adapter/zStackAdapter.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).
//
// ControllerRuntimeAdapter implementation for the Z-Stack adapter. The
// upstream adapter is an EventEmitter; the Rust port forwards the adapter's
// broadcast events into the controller event channel.

use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio::sync::{broadcast, mpsc};

use crate::adapter::events::{DeviceJoinedPayload, DeviceLeavePayload, ZclPayload};
use crate::adapter::z_stack::adapter::manager::StartResult as ZStackStartResult;
use crate::adapter::z_stack::adapter::z_stack_adapter::{AdapterEvent, ZStackAdapter};
use crate::controller::controller::{
    ControllerAdapterEvent, ControllerCoordinatorVersion, ControllerFuture,
    ControllerNetworkParameters, ControllerRuntimeAdapter, StartResult,
};
use crate::controller::green_power::{GreenPowerAdapter, GreenPowerNetworkParameters};
use crate::controller::model::entity::ControllerAdapter;
use crate::controller::touchlink::TouchlinkAdapter;
use crate::models::backup::Backup;
use crate::zspec::zcl::zcl_frame::Frame;

pub struct ZStackRuntimeAdapter {
    adapter: Arc<tokio::sync::Mutex<ZStackAdapter>>,
    has_zdo_message_overhead: bool,
    manufacturer_id: u16,
    event_receiver: Mutex<Option<mpsc::UnboundedReceiver<ControllerAdapterEvent>>>,
}

fn value_to_ieee(value: &Value) -> Option<String> {
    match value {
        Value::String(address) if !address.is_empty() => Some(address.clone()),
        Value::Number(number) => number.as_u64().map(|address| format!("0x{address:016x}")),
        _ => None,
    }
}

pub(crate) fn map_adapter_event(event: AdapterEvent) -> Option<ControllerAdapterEvent> {
    match event {
        AdapterEvent::ZdoResponse {
            cluster_id,
            response,
        } => Some(ControllerAdapterEvent::ZdoResponse {
            cluster_id,
            response,
        }),
        AdapterEvent::ZclPayload(payload) => Some(ControllerAdapterEvent::ZclPayload(payload)),
        AdapterEvent::DeviceJoined {
            network_address,
            ieee_address,
        } => value_to_ieee(&ieee_address).map(|ieee_addr| {
            ControllerAdapterEvent::DeviceJoined(DeviceJoinedPayload {
                network_address,
                ieee_addr,
            })
        }),
        AdapterEvent::DeviceLeave {
            network_address,
            ieee_address,
        } => Some(ControllerAdapterEvent::DeviceLeave(DeviceLeavePayload {
            network_address: Some(network_address),
            ieee_addr: value_to_ieee(&ieee_address),
        })),
    }
}

impl ZStackRuntimeAdapter {
    /// Build the controller-facing wrapper and start forwarding adapter events.
    pub async fn new(adapter: Arc<tokio::sync::Mutex<ZStackAdapter>>) -> Self {
        let (has_zdo_message_overhead, manufacturer_id, events) = {
            let adapter = adapter.lock().await;
            (
                adapter.has_zdo_message_overhead,
                adapter.manufacturer_id,
                adapter.events.clone(),
            )
        };

        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let forwarder = event_sender.clone();
        let mut source = events.subscribe();
        tokio::spawn(async move {
            loop {
                match source.recv().await {
                    Ok(event) => {
                        if let Some(event) = map_adapter_event(event) {
                            if forwarder.send(event).is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => {
                        let _ = forwarder.send(ControllerAdapterEvent::Disconnected);
                        break;
                    }
                }
            }
        });

        Self {
            adapter,
            has_zdo_message_overhead,
            manufacturer_id,
            event_receiver: Mutex::new(Some(event_receiver)),
        }
    }
}

impl ControllerAdapter for ZStackRuntimeAdapter {
    fn has_zdo_message_overhead(&self) -> bool {
        self.has_zdo_message_overhead
    }

    fn send_zdo(
        &self,
        ieee_address: &str,
        network_address: u16,
        cluster_id: u16,
        payload: &[u8],
        disable_response: bool,
    ) -> ControllerFuture<'_, Value> {
        let adapter = self.adapter.clone();
        let ieee_address = ieee_address.to_string();
        let payload = payload.to_vec();
        Box::pin(async move {
            adapter
                .lock()
                .await
                .send_zdo(
                    &ieee_address,
                    network_address,
                    cluster_id,
                    &payload,
                    disable_response,
                )
                .await
        })
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
        let adapter = self.adapter.clone();
        let ieee_address = ieee_address.to_string();
        let frame = frame.clone();
        Box::pin(async move {
            adapter
                .lock()
                .await
                .send_zcl_frame_to_endpoint(
                    &ieee_address,
                    network_address,
                    endpoint,
                    &frame,
                    timeout,
                    disable_response,
                    disable_recovery,
                    source_endpoint,
                    profile_id,
                )
                .await
        })
    }

    fn send_zcl_frame_to_group(
        &self,
        group_id: u16,
        frame: &Frame,
        source_endpoint: Option<u8>,
        profile_id: Option<u16>,
    ) -> ControllerFuture<'_, ()> {
        let adapter = self.adapter.clone();
        let frame = frame.clone();
        Box::pin(async move {
            adapter
                .lock()
                .await
                .send_zcl_frame_to_group(group_id, &frame, source_endpoint, profile_id)
                .await
        })
    }

    fn send_zcl_frame_to_all(
        &self,
        endpoint: u8,
        frame: &Frame,
        source_endpoint: u8,
        destination: u16,
        profile_id: Option<u16>,
    ) -> ControllerFuture<'_, ()> {
        let adapter = self.adapter.clone();
        let frame = frame.clone();
        Box::pin(async move {
            adapter
                .lock()
                .await
                .send_zcl_frame_to_all(endpoint, &frame, source_endpoint, destination, profile_id)
                .await
        })
    }
}

impl GreenPowerAdapter for ZStackRuntimeAdapter {
    fn get_coordinator_ieee(&self) -> ControllerFuture<'_, String> {
        let adapter = self.adapter.clone();
        Box::pin(async move { adapter.lock().await.get_coordinator_ieee().await })
    }

    fn get_network_parameters(&self) -> ControllerFuture<'_, GreenPowerNetworkParameters> {
        let adapter = self.adapter.clone();
        Box::pin(async move {
            let parameters = adapter.lock().await.get_network_parameters().await?;
            Ok(GreenPowerNetworkParameters {
                channel: parameters.channel,
            })
        })
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

impl TouchlinkAdapter for ZStackRuntimeAdapter {
    fn set_channel_inter_pan(&self, channel: u8) -> ControllerFuture<'_, ()> {
        let adapter = self.adapter.clone();
        Box::pin(async move { adapter.lock().await.set_channel_inter_pan(channel).await })
    }

    fn restore_channel_inter_pan(&self) -> ControllerFuture<'_, ()> {
        let adapter = self.adapter.clone();
        Box::pin(async move { adapter.lock().await.restore_channel_inter_pan().await })
    }

    fn send_zcl_frame_inter_pan_broadcast(
        &self,
        frame: &Frame,
        timeout: u64,
        disable_response: bool,
    ) -> ControllerFuture<'_, Option<ZclPayload>> {
        let adapter = self.adapter.clone();
        let frame = frame.clone();
        Box::pin(async move {
            adapter
                .lock()
                .await
                .send_zcl_frame_inter_pan_broadcast(&frame, timeout, disable_response)
                .await
        })
    }

    fn send_zcl_frame_inter_pan_to_ieee_addr(
        &self,
        frame: &Frame,
        ieee_address: &str,
    ) -> ControllerFuture<'_, ()> {
        let adapter = self.adapter.clone();
        let frame = frame.clone();
        let ieee_address = ieee_address.to_string();
        Box::pin(async move {
            adapter
                .lock()
                .await
                .send_zcl_frame_inter_pan_to_ieee_addr(&frame, &ieee_address)
                .await
        })
    }
}

impl ControllerRuntimeAdapter for ZStackRuntimeAdapter {
    fn start(&self) -> ControllerFuture<'_, StartResult> {
        let adapter = self.adapter.clone();
        Box::pin(async move {
            let result = adapter.lock().await.start().await?;
            Ok(match result {
                ZStackStartResult::Resumed => StartResult::Resumed,
                ZStackStartResult::Restored => StartResult::Restored,
                ZStackStartResult::Reset => StartResult::Reset,
            })
        })
    }

    fn stop(&self) -> ControllerFuture<'_, ()> {
        let adapter = self.adapter.clone();
        Box::pin(async move { adapter.lock().await.stop().await })
    }

    fn subscribe(&self) -> mpsc::UnboundedReceiver<ControllerAdapterEvent> {
        self.event_receiver
            .lock()
            .unwrap()
            .take()
            .expect("ZStackRuntimeAdapter subscribed twice")
    }

    fn permit_join(&self, time: u8, network_address: Option<u16>) -> ControllerFuture<'_, ()> {
        let adapter = self.adapter.clone();
        Box::pin(async move {
            adapter
                .lock()
                .await
                .permit_join(time, network_address)
                .await
        })
    }

    fn get_network_parameters_full(&self) -> ControllerFuture<'_, ControllerNetworkParameters> {
        let adapter = self.adapter.clone();
        Box::pin(async move {
            let parameters = adapter.lock().await.get_network_parameters().await?;
            Ok(ControllerNetworkParameters {
                pan_id: parameters.pan_id,
                extended_pan_id: parameters.extended_pan_id,
                channel: parameters.channel,
                nwk_update_id: parameters.nwk_update_id,
            })
        })
    }

    fn get_coordinator_version(&self) -> ControllerFuture<'_, ControllerCoordinatorVersion> {
        let adapter = self.adapter.clone();
        Box::pin(async move {
            let version = adapter
                .lock()
                .await
                .get_coordinator_version()
                .ok_or_else(|| "Coordinator version unavailable".to_string())?;
            Ok(ControllerCoordinatorVersion {
                r#type: version.type_name,
                meta: version.meta,
            })
        })
    }

    fn supports_backup(&self) -> ControllerFuture<'_, bool> {
        let adapter = self.adapter.clone();
        Box::pin(async move { Ok(adapter.lock().await.supports_backup().await) })
    }

    fn backup(&self, ieee_addresses: &[String]) -> ControllerFuture<'_, Backup> {
        let adapter = self.adapter.clone();
        let ieee_addresses = ieee_addresses.to_vec();
        Box::pin(async move { adapter.lock().await.backup(&ieee_addresses).await })
    }

    fn reset(&self, r#type: &str) -> ControllerFuture<'_, ()> {
        let adapter = self.adapter.clone();
        let r#type = r#type.to_string();
        Box::pin(async move { adapter.lock().await.reset(&r#type).await })
    }

    fn add_install_code(
        &self,
        ieee_address: &str,
        key: &[u8],
        hashed: bool,
    ) -> ControllerFuture<'_, ()> {
        let adapter = self.adapter.clone();
        let ieee_address = ieee_address.to_string();
        let key = key.to_vec();
        Box::pin(async move {
            adapter
                .lock()
                .await
                .add_install_code(&ieee_address, &key, hashed)
                .await
        })
    }

    fn manufacturer_id(&self) -> Option<u16> {
        Some(self.manufacturer_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maps_adapter_events_to_controller_events() {
        let joined = map_adapter_event(AdapterEvent::DeviceJoined {
            network_address: 0x1234,
            ieee_address: json!("0x00124b0000000001"),
        });
        match joined {
            Some(ControllerAdapterEvent::DeviceJoined(payload)) => {
                assert_eq!(payload.network_address, 0x1234);
                assert_eq!(payload.ieee_addr, "0x00124b0000000001");
            }
            _ => panic!("expected DeviceJoined"),
        }

        let left = map_adapter_event(AdapterEvent::DeviceLeave {
            network_address: 0x1234,
            ieee_address: Value::Null,
        });
        match left {
            Some(ControllerAdapterEvent::DeviceLeave(payload)) => {
                assert_eq!(payload.network_address, Some(0x1234));
                assert_eq!(payload.ieee_addr, None);
            }
            _ => panic!("expected DeviceLeave"),
        }

        let zdo = map_adapter_event(AdapterEvent::ZdoResponse {
            cluster_id: 0x8000,
            response: json!([0, {"eui64": "0x1", "nwkAddress": 2}]),
        });
        match zdo {
            Some(ControllerAdapterEvent::ZdoResponse {
                cluster_id,
                response,
            }) => {
                assert_eq!(cluster_id, 0x8000);
                assert_eq!(response[0], json!(0));
            }
            _ => panic!("expected ZdoResponse"),
        }
    }
}
