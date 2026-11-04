//! Z-Stack startup strategy, ported from herdsman `manager.ts:determineStrategy`.
//!
//! Pure decision logic: the ZNP layer supplies the adapter state it read over
//! the wire and the optional stored backup; tests replay every branch without a
//! coordinator.

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZnpVersion {
    ZStack12,
    ZStack3,
}

impl ZnpVersion {
    #[must_use]
    pub fn from_product_id(product_id: u8) -> Self {
        if product_id == 0 {
            Self::ZStack12
        } else {
            Self::ZStack3
        }
    }

    fn backup_version(self) -> u8 {
        match self {
            Self::ZStack12 => 0,
            Self::ZStack3 => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkOptions {
    pub pan_id: u16,
    /// Normal byte order (not the reversed NIB representation).
    pub extended_pan_id: [u8; 8],
    /// Packed channel mask, `1 << channel`.
    pub channel_mask: u32,
    pub network_key: [u8; 16],
    pub network_key_distribute: bool,
    /// Configuration used the `dd:dd:...` placeholder, which upstream replaces
    /// with the coordinator IEEE address.
    pub has_default_extended_pan_id: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nib {
    pub pan_id: u16,
    pub extended_pan_id: [u8; 8],
    pub channel_mask: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AdapterNvState {
    pub has_configured: bool,
    pub nib: Option<Nib>,
    pub preconfigured_key: Option<[u8; 16]>,
    pub active_key: Option<[u8; 16]>,
    pub alternate_key: Option<[u8; 16]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackupNetwork {
    pub network: NetworkOptions,
    /// `metadata.internal.znpVersion` from a unified backup.
    pub znp_version: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupStrategy {
    Startup,
    RestoreBackup,
    StartCommissioning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub strategy: StartupStrategy,
    pub warnings: Vec<String>,
}

fn key_matches(expected: Option<&[u8; 16]>, configured: &[u8; 16]) -> bool {
    expected == Some(configured)
}

fn config_matches_adapter(
    version: ZnpVersion,
    config: &NetworkOptions,
    adapter: &AdapterNvState,
) -> bool {
    let Some(nib) = adapter.nib else {
        return false;
    };
    let reversed: [u8; 8] = {
        let mut bytes = nib.extended_pan_id;
        bytes.reverse();
        bytes
    };
    let extended_pan_matches = config.extended_pan_id == nib.extended_pan_id
        || config.extended_pan_id == reversed
        || config.has_default_extended_pan_id;
    let preconfigured_matches = version != ZnpVersion::ZStack12
        || key_matches(adapter.preconfigured_key.as_ref(), &config.network_key);
    config.pan_id == nib.pan_id
        && extended_pan_matches
        && preconfigured_matches
        && key_matches(adapter.active_key.as_ref(), &config.network_key)
        && key_matches(adapter.alternate_key.as_ref(), &config.network_key)
}

fn backup_matches_adapter(backup: &BackupNetwork, adapter: &AdapterNvState) -> bool {
    let Some(nib) = adapter.nib else {
        return false;
    };
    backup.network.pan_id == nib.pan_id
        && backup.network.extended_pan_id == nib.extended_pan_id
        && backup.network.channel_mask == nib.channel_mask
        && key_matches(adapter.active_key.as_ref(), &backup.network.network_key)
}

fn config_matches_backup(config: &NetworkOptions, backup: &BackupNetwork) -> bool {
    let reversed: [u8; 8] = {
        let mut bytes = backup.network.extended_pan_id;
        bytes.reverse();
        bytes
    };
    config.pan_id == backup.network.pan_id
        && (config.extended_pan_id == backup.network.extended_pan_id
            || config.has_default_extended_pan_id
            || backup.network.has_default_extended_pan_id
            || config.extended_pan_id == reversed)
        && config.network_key == backup.network.network_key
        && config.channel_mask == backup.network.channel_mask
        && config.network_key_distribute == backup.network.network_key_distribute
}

fn check_restore_version(version: ZnpVersion, backup: &BackupNetwork) -> Result<()> {
    if version == ZnpVersion::ZStack12
        && backup
            .znp_version
            .is_some_and(|backup_version| backup_version != version.backup_version())
    {
        return Err(Error::Znp(
            "backup is from a newer Z-Stack platform and cannot be restored onto Z-Stack 1.2; remove the backup before proceeding"
                .into(),
        ));
    }
    Ok(())
}

/// Port of `determineStrategy` (manager.ts:110–279).
///
/// `force_inconsistent` mirrors
/// `adapterOptions.forceStartWithInconsistentAdapterConfiguration`.
pub fn determine_strategy(
    version: ZnpVersion,
    config: &NetworkOptions,
    adapter: &AdapterNvState,
    backup: Option<&BackupNetwork>,
    force_inconsistent: bool,
) -> Result<Decision> {
    let mut warnings = Vec::new();

    if !adapter.has_configured || adapter.nib.is_none() {
        if let Some(backup) = backup {
            if config_matches_backup(config, backup) {
                check_restore_version(version, backup)?;
                return Ok(Decision {
                    strategy: StartupStrategy::RestoreBackup,
                    warnings,
                });
            }
        }
        return Ok(Decision {
            strategy: StartupStrategy::StartCommissioning,
            warnings,
        });
    }

    if config_matches_adapter(version, config, adapter) {
        if version != ZnpVersion::ZStack12
            && !adapter
                .preconfigured_key
                .is_some_and(|key| key == config.network_key)
        {
            warnings.push(
                "adapter preconfigured (transport) key does not match the configured network key; harmless on Z-Stack 3.x when active/alternate keys match"
                    .into(),
            );
        }
        let nib = adapter.nib.expect("checked above");
        if nib.extended_pan_id == {
            let mut reversed = config.extended_pan_id;
            reversed.reverse();
            reversed
        } {
            warnings.push(format!(
                "extended PAN ID is reversed (configured={}, adapter={})",
                hex::encode(config.extended_pan_id),
                hex::encode(nib.extended_pan_id)
            ));
        }
        return Ok(Decision {
            strategy: StartupStrategy::Startup,
            warnings,
        });
    }

    if let Some(backup) = backup {
        if backup_matches_adapter(backup, adapter) {
            if force_inconsistent {
                warnings.push(
                    "running despite configuration-adapter mismatch because forceStartWithInconsistentAdapterConfiguration is set"
                        .into(),
                );
                return Ok(Decision {
                    strategy: StartupStrategy::Startup,
                    warnings,
                });
            }
            return Err(Error::Znp(
                "startup failed: configuration is not consistent with adapter state/backup; remove the coordinator backup only if you intend to re-commission and re-pair all devices"
                    .into(),
            ));
        }
        if config_matches_backup(config, backup) {
            check_restore_version(version, backup)?;
            return Ok(Decision {
                strategy: StartupStrategy::RestoreBackup,
                warnings,
            });
        }
        return Ok(Decision {
            strategy: StartupStrategy::StartCommissioning,
            warnings,
        });
    }

    Ok(Decision {
        strategy: StartupStrategy::StartCommissioning,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 16] = [0x11; 16];
    const OTHER_KEY: [u8; 16] = [0x22; 16];
    const EXT: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

    fn config() -> NetworkOptions {
        NetworkOptions {
            pan_id: 0x1a62,
            extended_pan_id: EXT,
            channel_mask: 1 << 15,
            network_key: KEY,
            network_key_distribute: false,
            has_default_extended_pan_id: false,
        }
    }

    fn adapter() -> AdapterNvState {
        AdapterNvState {
            has_configured: true,
            nib: Some(Nib {
                pan_id: 0x1a62,
                extended_pan_id: EXT,
                channel_mask: 1 << 15,
            }),
            preconfigured_key: Some(KEY),
            active_key: Some(KEY),
            alternate_key: Some(KEY),
        }
    }

    fn backup() -> BackupNetwork {
        BackupNetwork {
            network: config(),
            znp_version: Some(1),
        }
    }

    fn decide(adapter: AdapterNvState, backup: Option<BackupNetwork>) -> Result<Decision> {
        determine_strategy(
            ZnpVersion::ZStack3,
            &config(),
            &adapter,
            backup.as_ref(),
            false,
        )
    }

    #[test]
    fn configured_matching_adapter_resumes() {
        let decision = decide(adapter(), None).unwrap();
        assert_eq!(decision.strategy, StartupStrategy::Startup);
        assert!(decision.warnings.is_empty());
    }

    #[test]
    fn zstack3_ignores_preconfigured_key_mismatch_with_warning() {
        let mut adapter = adapter();
        adapter.preconfigured_key = Some(OTHER_KEY);
        let decision = decide(adapter, None).unwrap();
        assert_eq!(decision.strategy, StartupStrategy::Startup);
        assert_eq!(decision.warnings.len(), 1);
        assert!(decision.warnings[0].contains("preconfigured"));
    }

    #[test]
    fn zstack12_requires_preconfigured_key() {
        let mut adapter = adapter();
        adapter.preconfigured_key = Some(OTHER_KEY);
        let decision =
            determine_strategy(ZnpVersion::ZStack12, &config(), &adapter, None, false).unwrap();
        assert_eq!(decision.strategy, StartupStrategy::StartCommissioning);
    }

    #[test]
    fn reversed_extended_pan_id_is_accepted_with_warning() {
        let mut adapter = adapter();
        let mut reversed = EXT;
        reversed.reverse();
        adapter.nib.as_mut().unwrap().extended_pan_id = reversed;
        let decision = decide(adapter, None).unwrap();
        assert_eq!(decision.strategy, StartupStrategy::Startup);
        assert!(decision
            .warnings
            .iter()
            .any(|warning| warning.contains("reversed")));
    }

    #[test]
    fn default_extended_pan_id_is_accepted() {
        let mut config = config();
        config.extended_pan_id = [0xdd; 8];
        config.has_default_extended_pan_id = true;
        let decision =
            determine_strategy(ZnpVersion::ZStack3, &config, &adapter(), None, false).unwrap();
        assert_eq!(decision.strategy, StartupStrategy::Startup);
    }

    #[test]
    fn unconfigured_without_backup_commissions() {
        let mut adapter = adapter();
        adapter.has_configured = false;
        assert_eq!(
            decide(adapter, None).unwrap().strategy,
            StartupStrategy::StartCommissioning
        );
    }

    #[test]
    fn unconfigured_with_matching_backup_restores() {
        let mut adapter = adapter();
        adapter.has_configured = false;
        assert_eq!(
            decide(adapter, Some(backup())).unwrap().strategy,
            StartupStrategy::RestoreBackup
        );
    }

    #[test]
    fn zstack12_rejects_zstack3_backup() {
        let mut adapter = adapter();
        adapter.has_configured = false;
        let mut backup = backup();
        backup.znp_version = Some(1);
        let error = determine_strategy(
            ZnpVersion::ZStack12,
            &config(),
            &adapter,
            Some(&backup),
            false,
        )
        .unwrap_err();
        assert!(error.to_string().contains("cannot be restored"));
    }

    #[test]
    fn mismatched_config_and_backup_matching_adapter_is_an_error() {
        let mut config = config();
        config.network_key = OTHER_KEY;
        let error = determine_strategy(
            ZnpVersion::ZStack3,
            &config,
            &adapter(),
            Some(&backup()),
            false,
        )
        .unwrap_err();
        assert!(error.to_string().contains("not consistent"));
    }

    #[test]
    fn force_allows_inconsistent_adapter_state() {
        let mut config = config();
        config.network_key = OTHER_KEY;
        let decision = determine_strategy(
            ZnpVersion::ZStack3,
            &config,
            &adapter(),
            Some(&backup()),
            true,
        )
        .unwrap();
        assert_eq!(decision.strategy, StartupStrategy::Startup);
        assert!(decision.warnings[0].contains("force"));
    }

    #[test]
    fn config_matching_backup_restores_when_adapter_does_not_match() {
        // Config key differs from adapter/backup? No: config must match backup.
        // Make the adapter state stale while the backup still matches config.
        let mut adapter = adapter();
        adapter.active_key = Some(OTHER_KEY);
        adapter.nib.as_mut().unwrap().pan_id = 0x9999;
        assert_eq!(
            decide(adapter, Some(backup())).unwrap().strategy,
            StartupStrategy::RestoreBackup
        );
    }

    #[test]
    fn backup_not_matching_anything_commissions() {
        let mut adapter = adapter();
        adapter.active_key = Some(OTHER_KEY);
        adapter.nib.as_mut().unwrap().pan_id = 0x9999;
        let mut backup = backup();
        backup.network.pan_id = 0x7777;
        assert_eq!(
            decide(adapter, Some(backup)).unwrap().strategy,
            StartupStrategy::StartCommissioning
        );
    }

    #[test]
    fn adapter_mismatch_without_backup_commissions() {
        let mut adapter = adapter();
        adapter.active_key = Some(OTHER_KEY);
        assert_eq!(
            decide(adapter, None).unwrap().strategy,
            StartupStrategy::StartCommissioning
        );
    }
}

/// Parse the Z-Stack NIB (unaligned layout, 112 bytes).
///
/// Offsets follow `structs/entries/nib.ts`. Only the fields needed by strategy
/// selection are decoded; aligned-memory targets are rejected by the caller's
/// length check.
#[must_use]
pub fn parse_nib(bytes: &[u8]) -> Option<Nib> {
    if bytes.len() < 61 {
        return None;
    }
    let pan_id = u16::from_le_bytes([bytes[33], bytes[34]]);
    let channel_mask = u32::from_le_bytes([bytes[36], bytes[37], bytes[38], bytes[39]]);
    let mut extended_pan_id = [0u8; 8];
    extended_pan_id.copy_from_slice(&bytes[53..61]);
    extended_pan_id.reverse();
    Some(Nib {
        pan_id,
        extended_pan_id,
        channel_mask,
    })
}

#[must_use]
pub fn parse_has_configured(bytes: &[u8]) -> bool {
    bytes.first() == Some(&0x55)
}

#[must_use]
pub fn parse_key_descriptor(bytes: &[u8]) -> Option<[u8; 16]> {
    if bytes.len() < 17 {
        return None;
    }
    let mut key = [0u8; 16];
    key.copy_from_slice(&bytes[1..17]);
    Some(key)
}

fn hex_bytes(value: &str) -> Option<Vec<u8>> {
    let value = value.trim().trim_start_matches("0x");
    hex::decode(value).ok()
}

/// Load a unified (`zigpy/open-coordinator-backup`) backup.
///
/// Legacy or unknown formats return an error: commissioning over an existing
/// backup that we cannot interpret would risk wiping a working network.
pub fn load_backup(path: &std::path::Path) -> Result<Option<BackupNetwork>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path).map_err(|error| {
        Error::Config(format!("cannot read backup {}: {error}", path.display()))
    })?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|error| Error::Config(format!("corrupted coordinator backup: {error}")))?;
    let format = value
        .get("metadata")
        .and_then(|metadata| metadata.get("format"))
        .and_then(|format| format.as_str());
    let version = value
        .get("metadata")
        .and_then(|metadata| metadata.get("version"))
        .and_then(|version| version.as_u64());
    if format != Some("zigpy/open-coordinator-backup") || version != Some(1) {
        return Err(Error::Config(
            "unsupported coordinator backup format; refusing to commission over it".into(),
        ));
    }
    let pan_id = value
        .get("pan_id")
        .and_then(|value| value.as_str())
        .and_then(|value| u16::from_str_radix(value.trim_start_matches("0x"), 16).ok())
        .ok_or_else(|| Error::Config("backup is missing a valid pan_id".into()))?;
    let ext = value
        .get("extended_pan_id")
        .and_then(|value| value.as_str())
        .and_then(hex_bytes)
        .filter(|bytes| bytes.len() == 8)
        .ok_or_else(|| Error::Config("backup is missing a valid extended_pan_id".into()))?;
    let key = value
        .get("network_key")
        .and_then(|value| value.get("key"))
        .and_then(|value| value.as_str())
        .and_then(hex_bytes)
        .filter(|bytes| bytes.len() == 16)
        .ok_or_else(|| Error::Config("backup is missing a valid network key".into()))?;
    let channels = value
        .get("channel_mask")
        .and_then(|value| value.as_array())
        .ok_or_else(|| Error::Config("backup is missing channel_mask".into()))?;
    let mut channel_mask = 0u32;
    for channel in channels.iter().filter_map(|channel| channel.as_u64()) {
        if !(11..=26).contains(&channel) {
            return Err(Error::Config(format!(
                "backup contains unsupported channel {channel}"
            )));
        }
        channel_mask |= 1 << channel;
    }
    let mut extended_pan_id = [0u8; 8];
    extended_pan_id.copy_from_slice(&ext);
    let mut network_key = [0u8; 16];
    network_key.copy_from_slice(&key);
    let znp_version = value
        .get("metadata")
        .and_then(|metadata| metadata.get("internal"))
        .and_then(|internal| internal.get("znpVersion"))
        .and_then(|version| version.as_u64())
        .map(|version| version as u8);
    Ok(Some(BackupNetwork {
        network: NetworkOptions {
            pan_id,
            extended_pan_id,
            channel_mask,
            network_key,
            network_key_distribute: false,
            has_default_extended_pan_id: false,
        },
        znp_version,
    }))
}

#[cfg(test)]
mod parse_tests {
    use super::*;

    const KEY: [u8; 16] = [0x11; 16];
    const EXT: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

    #[test]
    fn parses_unaligned_nib_fields() {
        let mut bytes = vec![0u8; 112];
        bytes[33..35].copy_from_slice(&0x1a62u16.to_le_bytes());
        bytes[36..40].copy_from_slice(&(1u32 << 15).to_le_bytes());
        let mut reversed = EXT;
        reversed.reverse();
        bytes[53..61].copy_from_slice(&reversed);
        let nib = parse_nib(&bytes).unwrap();
        assert_eq!(nib.pan_id, 0x1a62);
        assert_eq!(nib.channel_mask, 1 << 15);
        assert_eq!(nib.extended_pan_id, EXT);
    }

    #[test]
    fn rejects_truncated_nib() {
        assert!(parse_nib(&[0u8; 60]).is_none());
    }

    #[test]
    fn has_configured_requires_0x55() {
        assert!(parse_has_configured(&[0x55]));
        assert!(!parse_has_configured(&[0x00]));
        assert!(!parse_has_configured(&[]));
    }

    #[test]
    fn parses_key_descriptor_skipping_sequence_byte() {
        let mut bytes = [0u8; 18];
        bytes[0] = 7;
        bytes[1..17].copy_from_slice(&KEY);
        assert_eq!(parse_key_descriptor(&bytes), Some(KEY));
        assert!(parse_key_descriptor(&bytes[..16]).is_none());
    }

    #[test]
    fn loads_unified_backup_and_rejects_unknown_formats() {
        let dir = std::env::temp_dir().join(format!(
            "z2m_backup_parse_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("coordinator_backup.json");
        std::fs::write(
            &path,
            r#"{
                "metadata": {"format": "zigpy/open-coordinator-backup", "version": 1, "internal": {"znpVersion": 1}},
                "pan_id": "0x1a62",
                "extended_pan_id": "0102030405060708",
                "channel_mask": [15],
                "network_key": {"key": "11111111111111111111111111111111", "sequence_number": 0, "frame_counter": 0}
            }"#,
        )
        .unwrap();
        let backup = load_backup(&path).unwrap().unwrap();
        assert_eq!(backup.network.pan_id, 0x1a62);
        assert_eq!(backup.network.extended_pan_id, EXT);
        assert_eq!(backup.network.channel_mask, 1 << 15);
        assert_eq!(backup.network.network_key, KEY);
        assert_eq!(backup.znp_version, Some(1));

        std::fs::write(&path, r#"{"adapterType": "zstack", "data": {}}"#).unwrap();
        let error = load_backup(&path).unwrap_err();
        assert!(error.to_string().contains("unsupported"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
