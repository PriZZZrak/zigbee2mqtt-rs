// Original: .upstream/zigbee2mqtt/lib/extension/extension.ts

use serde_json::Value;

/// Upstream abstract `Extension`.
///
/// Extensions are started/stopped by the controller and can adjust a message
/// before it is published.
pub trait Extension: Send + Sync {
    fn name(&self) -> &'static str;

    fn start(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Upstream `adjustMessageBeforePublish(entity, message)`.
    fn adjust_message_before_publish(&mut self, _entity: &str, _message: &mut Value) {}
}
