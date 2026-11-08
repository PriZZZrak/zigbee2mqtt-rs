// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/adapter/adapter-backup.ts

use serde_json::{json, Value};

use super::adapter_nv_memory::AdapterNvMemory;

/// Upstream `AdapterBackup`.
pub struct AdapterBackup {
    pub nv: AdapterNvMemory,
    pub backup_path: String,
}

impl AdapterBackup {
    pub fn new(nv: AdapterNvMemory, backup_path: impl Into<String>) -> Self {
        Self {
            nv,
            backup_path: backup_path.into(),
        }
    }

    /// Upstream `getStoredBackup()`.
    pub fn get_stored_backup(&self) -> Result<Option<Value>, String> {
        match std::fs::read_to_string(&self.backup_path) {
            Ok(content) => serde_json::from_str(&content)
                .map(Some)
                .map_err(|error| format!("Coordinator backup is corrupted: {error}")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    /// Upstream `createBackup()`.
    pub async fn create_backup(&mut self) -> Result<Value, String> {
        // NIB (33), EXTADDR (1) and active key (58).
        let nib = self
            .nv
            .read_item(33, 0)
            .await?
            .ok_or_else(|| "Cannot backup - adapter not commissioned".to_string())?;
        let extaddr = self
            .nv
            .read_item(1, 0)
            .await?
            .ok_or_else(|| "Failed to read adapter IEEE address".to_string())?;
        let active_key = self
            .nv
            .read_item(58, 0)
            .await?
            .ok_or_else(|| "Cannot backup - missing active key info".to_string())?;

        let backup = json!({
            "data": {
                "ZCD_NV_NIB": {"value": nib},
                "ZCD_NV_EXTADDR": {"value": extaddr},
                "ZCD_NV_NWK_ACTIVE_KEY_INFO": {"value": active_key},
            }
        });
        if let Some(parent) = std::path::Path::new(&self.backup_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(
            &self.backup_path,
            serde_json::to_vec_pretty(&backup).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        Ok(backup)
    }

    /// Upstream `restoreBackup(backup)`.
    pub async fn restore_backup(&mut self, backup: &Value) -> Result<(), String> {
        let data = backup
            .get("data")
            .and_then(Value::as_object)
            .ok_or_else(|| "Backup corrupted - missing data".to_string())?;
        for (key, entry) in data {
            let Some(bytes) = entry.get("value").and_then(Value::as_array) else {
                continue;
            };
            let bytes = bytes
                .iter()
                .filter_map(Value::as_u64)
                .map(|value| value as u8)
                .collect::<Vec<_>>();
            let id = match key.as_str() {
                "ZCD_NV_EXTADDR" => 1,
                "ZCD_NV_NIB" => 33,
                "ZCD_NV_NWK_ACTIVE_KEY_INFO" => 58,
                _ => continue,
            };
            self.nv.update_item(id, &bytes, true).await?;
        }
        Ok(())
    }
}
