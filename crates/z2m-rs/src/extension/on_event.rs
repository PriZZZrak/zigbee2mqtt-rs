// Original: .upstream/zigbee2mqtt/lib/extension/onEvent.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).
//
// The global `onEvent` shared hook from zigbee-herdsman-converters is a
// vendor-specific Legrand workaround and is out of the minimal converter
// surface (AGENTS §5 Q4); per-definition `onEvent` handlers are dispatched.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::extension::extension::{Extension, ExtensionContext, ExtensionFuture};
use crate::model::entity::Entity;

pub struct OnEvent {
    context: ExtensionContext,
    start_called: Mutex<HashSet<String>>,
}

impl OnEvent {
    pub fn new(context: ExtensionContext) -> Arc<Self> {
        Arc::new(Self {
            context,
            start_called: Mutex::new(HashSet::new()),
        })
    }

    fn base_data(&self, device: &Arc<crate::model::Device>) -> zigbee_converters::OnEventBaseData {
        let exposes_bus = self.context.event_bus.clone();
        zigbee_converters::OnEventBaseData {
            device: device.zh.clone(),
            state: self.context.state.lock().unwrap().get(&device.entity_id()),
            options: device.options(),
            device_exposes_changed: Arc::new(move || {
                exposes_bus
                    .lock()
                    .unwrap()
                    .emit_exposes_and_devices_changed();
            }),
        }
    }

    /// Upstream `callOnEvent` (`onEvent.ts:74-92`).
    async fn call_on_event(
        self: &Arc<Self>,
        device: Arc<crate::model::Device>,
        event: zigbee_converters::OnEventEvent,
    ) {
        if device
            .options()
            .get("disabled")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            return;
        }

        let ieee = device.ieee_addr();
        let event_type = event.event_type().to_string();
        let definition = device.definition();

        if event_type == "start" {
            self.start_called.lock().unwrap().insert(ieee.clone());
        } else if event_type != "stop"
            && !self.start_called.lock().unwrap().contains(&ieee)
            && definition
                .as_ref()
                .is_some_and(|definition| !definition.on_event.is_empty())
        {
            self.start_called.lock().unwrap().insert(ieee.clone());
            if let Some(definition) = &definition {
                let start = zigbee_converters::OnEventEvent::Start(self.base_data(&device));
                for handler in &definition.on_event {
                    if let Err(error) = handler(start.clone()).await {
                        self.log_error(&device, &error);
                    }
                }
            }
        }

        if let Some(definition) = &definition {
            for handler in &definition.on_event {
                if let Err(error) = handler(event.clone()).await {
                    self.log_error(&device, &error);
                }
            }
        }

        if event_type == "stop" {
            self.start_called.lock().unwrap().remove(&ieee);
        }
    }

    fn log_error(&self, device: &crate::model::Device, error: &str) {
        self.context.logger.lock().unwrap().error(
            &format!("onEvent for '{}' failed: {error}", device.name()),
            "z2m",
        );
    }

    fn devices(&self) -> Vec<Arc<crate::model::Device>> {
        self.context.zigbee.devices_iterator(|device| {
            device.r#type() != zigbee_apiarist::controller::database::DeviceType::Coordinator
        })
    }
}

impl Extension for OnEvent {
    fn name(&self) -> &'static str {
        "OnEvent"
    }

    fn context(&self) -> &ExtensionContext {
        &self.context
    }

    fn start(self: Arc<Self>) -> ExtensionFuture {
        Box::pin(async move {
            for device in self.devices() {
                device.resolve_definition(false);
                let this = self.clone();
                tokio::spawn(async move {
                    let event = zigbee_converters::OnEventEvent::Start(this.base_data(&device));
                    this.call_on_event(device.clone(), event).await;
                });
            }

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_device_joined(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(device) = this.device_from(&data) {
                            let event = zigbee_converters::OnEventEvent::DeviceJoined(
                                this.base_data(&device),
                            );
                            this.call_on_event(device, event).await;
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_device_leave(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(device) = this.device_from(&data) {
                            let event = zigbee_converters::OnEventEvent::Stop {
                                ieee_addr: device.ieee_addr(),
                            };
                            this.call_on_event(device, event).await;
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_entity_removed(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(entity) = data.get("entity") {
                            if let Some(ieee) = entity.get("ieeeAddr").and_then(Value::as_str) {
                                if let Some(device) = this.context.zigbee.resolve_device(ieee) {
                                    let event = zigbee_converters::OnEventEvent::Stop {
                                        ieee_addr: device.ieee_addr(),
                                    };
                                    this.call_on_event(device, event).await;
                                }
                            }
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_device_interview(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(device) = this.device_from(&data) {
                            let event = zigbee_converters::OnEventEvent::DeviceInterview {
                                data: this.base_data(&device),
                                status: data
                                    .get("status")
                                    .and_then(Value::as_str)
                                    .unwrap_or("started")
                                    .to_string(),
                            };
                            this.call_on_event(device, event).await;
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_device_announce(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(device) = this.device_from(&data) {
                            let event = zigbee_converters::OnEventEvent::DeviceAnnounce(
                                this.base_data(&device),
                            );
                            this.call_on_event(device, event).await;
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_device_network_address_changed(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        if let Some(device) = this.device_from(&data) {
                            let event =
                                zigbee_converters::OnEventEvent::DeviceNetworkAddressChanged(
                                    this.base_data(&device),
                                );
                            this.call_on_event(device, event).await;
                        }
                    });
                }),
            );

            let bus = self.context.event_bus.clone();
            let this = self.clone();
            bus.lock().unwrap().on_entity_options_changed(
                self.context.key,
                Arc::new(move |data| {
                    let this = this.clone();
                    tokio::spawn(async move {
                        let Some(entity) = data.get("entity") else {
                            return;
                        };
                        if entity.get("ieeeAddr").is_some() {
                            let Some(device) = this.device_from(&json!({"device": entity})) else {
                                return;
                            };
                            let event = zigbee_converters::OnEventEvent::DeviceOptionsChanged {
                                data: this.base_data(&device),
                                from: data.get("from").cloned().unwrap_or(Value::Null),
                                to: data.get("to").cloned().unwrap_or(Value::Null),
                            };
                            this.call_on_event(device, event).await;
                            this.context
                                .event_bus
                                .lock()
                                .unwrap()
                                .emit_devices_changed();
                        }
                    });
                }),
            );

            Ok(())
        })
    }

    fn stop(self: Arc<Self>) -> ExtensionFuture {
        Box::pin(async move {
            self.context
                .event_bus
                .lock()
                .unwrap()
                .remove_listeners(self.context.key);

            if self.context.zigbee.zh_controller().is_some() {
                for device in self.devices() {
                    let event = zigbee_converters::OnEventEvent::Stop {
                        ieee_addr: device.ieee_addr(),
                    };
                    self.call_on_event(device, event).await;
                }
            }
            Ok(())
        })
    }
}

impl OnEvent {
    fn device_from(&self, data: &Value) -> Option<Arc<crate::model::Device>> {
        let ieee = data
            .get("device")
            .and_then(|device| device.get("ieeeAddr"))
            .and_then(Value::as_str)?;
        self.context.zigbee.resolve_device(ieee)
    }
}
