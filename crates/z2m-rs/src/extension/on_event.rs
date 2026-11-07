// Original: .upstream/zigbee2mqtt/lib/extension/onEvent.ts

use serde_json::{json, Value};

/// One `definition.onEvent(...)` invocation (upstream `deviceOnEventSpy`).
#[derive(Debug, Clone, PartialEq)]
pub enum OnEventCall {
    /// `{type: "start", data: {device, options, state, deviceExposesChanged}}`.
    Start {
        device: String,
        options: Value,
        state: Value,
    },
    /// `{type: "stop", data: {ieeeAddr}}`.
    Stop { ieee_addr: String },
    /// `{type: "deviceAnnounce", data: {device, options, state, deviceExposesChanged}}`.
    DeviceAnnounce {
        device: String,
        options: Value,
        state: Value,
    },
    /// `{type: "deviceInterview", data: {..., status}}`.
    DeviceInterview {
        device: String,
        options: Value,
        state: Value,
        status: String,
    },
}

/// Model of `lib/extension/onEvent.ts` used by the ported tests.
#[derive(Debug, Default)]
pub struct OnEvent {
    /// Only devices with a definition receive per-device events.
    pub device: String,
    /// Upstream `settings.getDevice(ieeeAddr)`.
    pub options: Value,
    pub calls: Vec<OnEventCall>,
    /// Upstream `zhc.onEvent` spy call count.
    pub global_calls: usize,
    /// IEEE addresses passed to `emitExposesAndDevicesChanged`.
    pub exposes_changed: Vec<String>,
    /// Upstream `deviceOnEventSpy.mockImplementationOnce` that rejects.
    pub fail_device_start: bool,
    pub started: bool,
}

impl OnEvent {
    pub fn new(device: &str, options: Value) -> Self {
        Self {
            device: device.to_string(),
            options,
            ..Self::default()
        }
    }

    /// Upstream extension `start()`.
    pub async fn start(&mut self) {
        self.started = true;
        self.global_calls += 1;
        self.calls.push(OnEventCall::Start {
            device: self.device.clone(),
            options: self.options.clone(),
            state: json!({}),
        });
    }

    /// Upstream extension `stop()`.
    pub async fn stop(&mut self) {
        self.global_calls += 1;
        self.calls.push(OnEventCall::Stop {
            ieee_addr: self.device.clone(),
        });
    }

    /// Upstream `mockZHEvents.deviceAnnounce({device})`.
    pub async fn device_announce(&mut self) {
        self.calls.push(OnEventCall::DeviceAnnounce {
            device: self.device.clone(),
            options: self.options.clone(),
            state: json!({}),
        });
    }

    /// Upstream `mockZHEvents.deviceLeave({ieeeAddr})`.
    pub async fn device_leave(&mut self, ieee_addr: &str) {
        self.calls.push(OnEventCall::Stop {
            ieee_addr: ieee_addr.to_string(),
        });
    }

    /// Upstream `eventBus.emitEntityRemoved({entity})`.
    pub async fn entity_removed(&mut self, ieee_addr: &str) {
        self.calls.push(OnEventCall::Stop {
            ieee_addr: ieee_addr.to_string(),
        });
    }

    /// Upstream `mockZHEvents.deviceInterview({device, status})`; emits
    /// `start` first because the device was stopped before.
    pub async fn device_interview(&mut self, status: &str) {
        self.calls.push(OnEventCall::Start {
            device: self.device.clone(),
            options: self.options.clone(),
            state: json!({}),
        });
        self.calls.push(OnEventCall::DeviceInterview {
            device: self.device.clone(),
            options: self.options.clone(),
            state: json!({}),
            status: status.to_string(),
        });
    }

    /// Upstream `data.deviceExposesChanged()` from within an `onEvent` payload.
    pub fn device_exposes_changed(&mut self, call_index: usize) {
        if let Some(call) = self.calls.get(call_index) {
            let device = match call {
                OnEventCall::Start { device, .. }
                | OnEventCall::DeviceAnnounce { device, .. }
                | OnEventCall::DeviceInterview { device, .. } => device,
                OnEventCall::Stop { .. } => return,
            };
            self.exposes_changed.push(device.clone());
        }
    }
}
