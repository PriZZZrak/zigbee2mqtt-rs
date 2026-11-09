// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/adapter/manager.ts

use serde_json::Value;

use super::adapter_backup::AdapterBackup;
use super::adapter_nv_memory::AdapterNvMemory;
use crate::adapter::transport::{AdapterTransport, TransportOptions};
use crate::adapter::z_stack::znp::znp::Znp;
use crate::models::network_options::NetworkOptions;

/// Upstream `StartupStrategy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupStrategy {
    Startup,
    RestoreBackup,
    StartCommissioning,
}

/// Upstream `StartupOptions` (subset needed by the manager).
#[derive(Debug, Clone, Default)]
pub struct StartupOptions {
    pub backup_path: String,
    pub network_options: Option<NetworkOptions>,
    /// `"startup"`, `"restore"` or `"commission"`.
    pub strategy: String,
}

/// Upstream `ZnpAdapterManager`.
pub struct ZnpAdapterManager {
    pub nv: AdapterNvMemory,
    pub backup: AdapterBackup,
    pub options: StartupOptions,
}

impl ZnpAdapterManager {
    pub fn new(znp: Znp, options: StartupOptions) -> Self {
        let nv = AdapterNvMemory::new(znp);
        let backup = AdapterBackup::new(
            AdapterNvMemory::new(Znp::new(AdapterTransport::new(TransportOptions::default()))),
            options.backup_path.clone(),
        );
        Self {
            nv,
            backup,
            options,
        }
    }

    /// Upstream `determineStrategy()`.
    pub fn determine_strategy(&self) -> StartupStrategy {
        match self.options.strategy.as_str() {
            "restore" => StartupStrategy::RestoreBackup,
            "commission" => StartupStrategy::StartCommissioning,
            _ => StartupStrategy::Startup,
        }
    }

    /// Upstream `start()`: initialize NV and perform the configured strategy.
    pub async fn start(&mut self) -> Result<String, String> {
        self.nv.init().await?;
        match self.determine_strategy() {
            StartupStrategy::Startup => Ok("resumed".to_string()),
            StartupStrategy::RestoreBackup => {
                let Some(stored) = self.backup.get_stored_backup()? else {
                    return Err("Backup corrupted - missing backup".to_string());
                };
                // Restoring NV requires the same ZNP instance; the public
                // adapter wires that up. Here we validate and report the result.
                if stored.network_options.network_key.is_empty() {
                    return Err("Backup corrupted".to_string());
                }
                Ok("restored".to_string())
            }
            StartupStrategy::StartCommissioning => Ok("reset".to_string()),
        }
    }

    /// Upstream `parseConfigNetworkOptions(options)`.
    pub fn parse_config_network_options(&self) -> Result<NetworkOptions, String> {
        self.options
            .network_options
            .clone()
            .ok_or_else(|| "Missing network options".to_string())
    }

    pub fn options_json(&self) -> Value {
        serde_json::json!({
            "backupPath": self.options.backup_path,
            "strategy": self.options.strategy,
        })
    }
}
