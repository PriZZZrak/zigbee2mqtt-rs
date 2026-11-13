// Original: .upstream/zigbee2mqtt/lib/extension/extension.ts
// Upstream revision: zigbee2mqtt 2.14.3 (3c841fdf).

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;

use crate::event_bus::EventBus;
use crate::model::entity::{Entity, PublishEntityState};
use crate::mqtt::Mqtt;
use crate::state::State;
use crate::zigbee::Zigbee;

pub type ExtensionFuture = Pin<Box<dyn Future<Output = Result<(), String>> + Send>>;

/// Shared constructor arguments (`extension.ts:23-41`).
pub struct ExtensionContext {
    pub zigbee: Arc<Zigbee>,
    pub mqtt: Arc<tokio::sync::Mutex<Mqtt>>,
    pub state: Arc<std::sync::Mutex<State>>,
    pub publish_entity_state: PublishEntityState,
    pub event_bus: Arc<std::sync::Mutex<EventBus>>,
    pub logger: Arc<std::sync::Mutex<crate::util::logger::Logger>>,
    pub settings: Arc<std::sync::Mutex<crate::util::settings::Settings>>,
    pub enable_disable_extension: Arc<
        dyn Fn(bool, String) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>>
            + Send
            + Sync,
    >,
    pub restart: Arc<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>,
    pub add_extension:
        Arc<dyn Fn(Arc<dyn Extension>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>,
    /// EventBus listener key replacing upstream object identity.
    pub key: usize,
}

impl ExtensionContext {
    /// Upstream module-level `settings.get()`.
    pub fn settings_value(&self) -> Value {
        self.settings.lock().unwrap().get()
    }
}

/// Upstream abstract `Extension` (`extension.ts:1`).
pub trait Extension: Send + Sync {
    fn name(&self) -> &'static str;

    fn context(&self) -> &ExtensionContext;

    /// Upstream `start()` (`extension.ts:46`).
    fn start(self: Arc<Self>) -> ExtensionFuture;

    /// Upstream `stop()` (`extension.ts:53-55`): removes every listener.
    fn stop(self: Arc<Self>) -> ExtensionFuture {
        let event_bus = self.context().event_bus.clone();
        let key = self.context().key;
        Box::pin(async move {
            event_bus.lock().unwrap().remove_listeners(key);
            Ok(())
        })
    }

    /// Upstream `adjustMessageBeforePublish(entity, message)` (`extension.ts:57`).
    fn adjust_message_before_publish(&self, _entity: &dyn Entity, _message: &mut Value) {}
}
