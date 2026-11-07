// Original: .upstream/zigbee-herdsman/src/adapter/adapter.ts
// TODO: port the adapter factory.

use serde_json::Value;

use crate::adapter::adapter_discovery::DiscoveryContext;

/// Upstream adapter classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterKind {
    Deconz,
    Ember,
    Ezsp,
    ZStack,
    ZBoss,
    ZiGate,
    ZoH,
}

/// Upstream created adapter instance (class + resolved serial options).
#[derive(Debug, Clone, PartialEq)]
pub struct AdapterInstance {
    pub kind: AdapterKind,
    /// Upstream protected `serialPortOptions`.
    pub serial_port_options: Value,
}

/// Upstream `Adapter` factory.
pub struct Adapter;

impl Adapter {
    /// Upstream `Adapter.create(networkOptions, serialPortOptions, backupPath, options)`.
    pub async fn create(
        _network_options: Value,
        _serial_port_options: Value,
        _backup_path: &str,
        _options: Value,
    ) -> Result<AdapterInstance, String> {
        todo!()
    }

    /// Test/dev variant with an injected `SerialPort.list()` / mDNS context.
    pub async fn create_with_context(
        _context: &DiscoveryContext,
        _network_options: Value,
        _serial_port_options: Value,
        _backup_path: &str,
        _options: Value,
    ) -> Result<AdapterInstance, String> {
        todo!()
    }
}
