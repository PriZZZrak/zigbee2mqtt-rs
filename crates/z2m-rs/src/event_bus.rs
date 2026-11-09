// Original: .upstream/zigbee2mqtt/lib/eventBus.ts
//
// Rust port of the typed EventEmitter. Listeners are keyed by extension so
// `removeListeners` mirrors upstream `eventBus.removeListeners(extension)`.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use zigbee_apiarist::utils::datetime::now_ms;

/// Upstream `EventBusMap` keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Event {
    AdapterDisconnected,
    AdapterDisconnectedEvent,
    PermitJoinChanged,
    PublishAvailability,
    EntityRenamed,
    EntityRemoved,
    LastSeenChanged,
    DeviceNetworkAddressChanged,
    DeviceAnnounce,
    DeviceInterview,
    DeviceJoined,
    EntityOptionsChanged,
    ExposesChanged,
    DeviceLeave,
    DeviceMessage,
    MqttMessage,
    MqttMessagePublished,
    PublishEntityState,
    GroupMembersChanged,
    DevicesChanged,
    ScenesChanged,
    Reconfigure,
    StateChange,
}

/// Listener callback. Payload is the upstream event data object.
pub type Listener = Arc<dyn Fn(Value) + Send + Sync>;

#[derive(Debug, Default, Clone)]
pub struct DeviceStats {
    pub last_seen_messages: u64,
    pub first_last_seen: Option<u64>,
    pub leave_counts: u64,
    pub network_address_changes: u64,
}

#[derive(Debug, Default, Clone)]
pub struct MqttStats {
    pub published: u64,
    pub received: u64,
}

#[derive(Debug, Default, Clone)]
pub struct Stats {
    pub devices: HashMap<String, DeviceStats>,
    pub mqtt: MqttStats,
}

#[derive(Clone)]
struct Registration {
    key: usize,
    callback: Listener,
}

/// Upstream `EventBus`.
#[derive(Default, Clone)]
pub struct EventBus {
    listeners: HashMap<Event, Vec<Registration>>,
    pub stats: Stats,
}

impl EventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on(&mut self, event: Event, key: usize, callback: Listener) {
        self.listeners
            .entry(event)
            .or_default()
            .push(Registration { key, callback });
    }

    pub fn remove_listeners(&mut self, key: usize) {
        for listeners in self.listeners.values_mut() {
            listeners.retain(|registration| registration.key != key);
        }
    }

    fn emit(&mut self, event: Event, data: Value) {
        if let Some(listeners) = self.listeners.get(&event) {
            for registration in listeners {
                (registration.callback)(data.clone());
            }
        }
    }

    fn emit_no_data(&mut self, event: Event) {
        self.emit(event, Value::Null);
    }

    // Upstream emit* methods.

    pub fn emit_adapter_disconnected(&mut self) {
        self.emit_no_data(Event::AdapterDisconnected);
    }

    pub fn emit_permit_join_changed(&mut self, data: Value) {
        self.emit(Event::PermitJoinChanged, data);
    }

    pub fn emit_publish_availability(&mut self) {
        self.emit_no_data(Event::PublishAvailability);
    }

    pub fn emit_entity_renamed(&mut self, data: Value) {
        self.emit(Event::EntityRenamed, data);
    }

    pub fn emit_entity_removed(&mut self, data: Value) {
        self.emit(Event::EntityRemoved, data);
    }

    pub fn emit_last_seen_changed(&mut self, data: Value) {
        if let Some(ieee) = data
            .get("device")
            .and_then(|device| device.get("ieeeAddr"))
            .and_then(Value::as_str)
        {
            let stats = self.stats.devices.entry(ieee.to_string()).or_default();
            stats.last_seen_messages += 1;
            stats.first_last_seen.get_or_insert(now_ms());
        }
        self.emit(Event::LastSeenChanged, data);
    }

    pub fn emit_device_network_address_changed(&mut self, data: Value) {
        if let Some(ieee) = data
            .get("device")
            .and_then(|device| device.get("ieeeAddr"))
            .and_then(Value::as_str)
        {
            self.stats
                .devices
                .entry(ieee.to_string())
                .or_default()
                .network_address_changes += 1;
        }
        self.emit(Event::DeviceNetworkAddressChanged, data);
    }

    pub fn emit_device_announce(&mut self, data: Value) {
        self.emit(Event::DeviceAnnounce, data);
    }

    pub fn emit_device_interview(&mut self, data: Value) {
        self.emit(Event::DeviceInterview, data);
    }

    pub fn emit_device_joined(&mut self, data: Value) {
        self.emit(Event::DeviceJoined, data);
    }

    pub fn emit_entity_options_changed(&mut self, data: Value) {
        self.emit(Event::EntityOptionsChanged, data);
    }

    pub fn emit_exposes_changed(&mut self, data: Value) {
        self.emit(Event::ExposesChanged, data);
    }

    pub fn emit_device_leave(&mut self, data: Value) {
        if let Some(ieee) = data.get("ieeeAddr").and_then(Value::as_str) {
            self.stats
                .devices
                .entry(ieee.to_string())
                .or_default()
                .leave_counts += 1;
        }
        self.emit(Event::DeviceLeave, data);
    }

    pub fn emit_device_message(&mut self, data: Value) {
        self.emit(Event::DeviceMessage, data);
    }

    pub fn emit_mqtt_message(&mut self, data: Value) {
        self.stats.mqtt.received += 1;
        self.emit(Event::MqttMessage, data);
    }

    pub fn emit_mqtt_message_published(&mut self, data: Value) {
        self.stats.mqtt.published += 1;
        self.emit(Event::MqttMessagePublished, data);
    }

    pub fn emit_publish_entity_state(&mut self, data: Value) {
        self.emit(Event::PublishEntityState, data);
    }

    pub fn emit_group_members_changed(&mut self, data: Value) {
        self.emit(Event::GroupMembersChanged, data);
    }

    pub fn emit_devices_changed(&mut self) {
        self.emit_no_data(Event::DevicesChanged);
    }

    pub fn emit_scenes_changed(&mut self, data: Value) {
        self.emit(Event::ScenesChanged, data);
    }

    pub fn emit_reconfigure(&mut self, data: Value) {
        self.emit(Event::Reconfigure, data);
    }

    pub fn emit_state_change(&mut self, data: Value) {
        self.emit(Event::StateChange, data);
    }

    pub fn emit_exposes_and_devices_changed(&mut self) {
        self.emit_no_data(Event::ExposesChanged);
        self.emit_no_data(Event::DevicesChanged);
    }

    // Upstream on* methods.

    pub fn on_adapter_disconnected(&mut self, key: usize, callback: Listener) {
        self.on(Event::AdapterDisconnected, key, callback);
    }

    pub fn on_permit_join_changed(&mut self, key: usize, callback: Listener) {
        self.on(Event::PermitJoinChanged, key, callback);
    }

    pub fn on_entity_renamed(&mut self, key: usize, callback: Listener) {
        self.on(Event::EntityRenamed, key, callback);
    }

    pub fn on_entity_removed(&mut self, key: usize, callback: Listener) {
        self.on(Event::EntityRemoved, key, callback);
    }

    pub fn on_last_seen_changed(&mut self, key: usize, callback: Listener) {
        self.on(Event::LastSeenChanged, key, callback);
    }

    pub fn on_device_network_address_changed(&mut self, key: usize, callback: Listener) {
        self.on(Event::DeviceNetworkAddressChanged, key, callback);
    }

    pub fn on_device_announce(&mut self, key: usize, callback: Listener) {
        self.on(Event::DeviceAnnounce, key, callback);
    }

    pub fn on_device_interview(&mut self, key: usize, callback: Listener) {
        self.on(Event::DeviceInterview, key, callback);
    }

    pub fn on_device_joined(&mut self, key: usize, callback: Listener) {
        self.on(Event::DeviceJoined, key, callback);
    }

    pub fn on_device_leave(&mut self, key: usize, callback: Listener) {
        self.on(Event::DeviceLeave, key, callback);
    }

    pub fn on_device_message(&mut self, key: usize, callback: Listener) {
        self.on(Event::DeviceMessage, key, callback);
    }

    pub fn on_entity_options_changed(&mut self, key: usize, callback: Listener) {
        self.on(Event::EntityOptionsChanged, key, callback);
    }

    pub fn on_exposes_changed(&mut self, key: usize, callback: Listener) {
        self.on(Event::ExposesChanged, key, callback);
    }

    pub fn on_group_members_changed(&mut self, key: usize, callback: Listener) {
        self.on(Event::GroupMembersChanged, key, callback);
    }

    pub fn on_mqtt_message(&mut self, key: usize, callback: Listener) {
        self.on(Event::MqttMessage, key, callback);
    }

    pub fn on_mqtt_message_published(&mut self, key: usize, callback: Listener) {
        self.on(Event::MqttMessagePublished, key, callback);
    }

    pub fn on_publish_entity_state(&mut self, key: usize, callback: Listener) {
        self.on(Event::PublishEntityState, key, callback);
    }

    pub fn on_devices_changed(&mut self, key: usize, callback: Listener) {
        self.on(Event::DevicesChanged, key, callback);
    }

    pub fn on_scenes_changed(&mut self, key: usize, callback: Listener) {
        self.on(Event::ScenesChanged, key, callback);
    }

    pub fn on_reconfigure(&mut self, key: usize, callback: Listener) {
        self.on(Event::Reconfigure, key, callback);
    }

    pub fn on_state_change(&mut self, key: usize, callback: Listener) {
        self.on(Event::StateChange, key, callback);
    }

    pub fn on_publish_availability(&mut self, key: usize, callback: Listener) {
        self.on(Event::PublishAvailability, key, callback);
    }
}
