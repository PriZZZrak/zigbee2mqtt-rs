// Original: .upstream/zigbee-herdsman/src/utils/backup.ts

use serde_json::{json, Value};

use crate::adapter::z_stack::structs::entries::nib::nib;
use crate::adapter::z_stack::structs::entries::nwk_key_descriptor::nwk_key_descriptor;
use crate::adapter::z_stack::structs::entries::nwk_sec_material_descriptor_entry::nwk_sec_material_descriptor_entry;
use crate::adapter::z_stack::utils::channel_list::unpack_channel_list;
use crate::models::backup::{Backup, BackupDevice, LinkKey, NetworkKeyInfo, UnifiedBackupStorage};
use crate::models::network_options::NetworkOptions;
use crate::utils::datetime::now_iso8601;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn from_hex(value: &str) -> Vec<u8> {
    (0..value.len())
        .step_by(2)
        .filter_map(|index| u8::from_str_radix(value.get(index..index + 2)?, 16).ok())
        .collect()
}

/// Upstream `packageInfo.name` / `packageInfo.version` read from `package.json`.
const HERDSMAN_PACKAGE_NAME: &str = "zigbee-herdsman";
const HERDSMAN_PACKAGE_VERSION: &str = "11.2.0";

/// Upstream `toUnifiedBackup(backup)`.
pub fn to_unified_backup(backup: &Backup) -> UnifiedBackupStorage {
    json!({
        "metadata": {
            "format": "zigpy/open-coordinator-backup",
            "version": 1,
            "source": format!("{HERDSMAN_PACKAGE_NAME}@{HERDSMAN_PACKAGE_VERSION}"),
            "internal": {
                "date": now_iso8601(),
                "znpVersion": backup.znp_version,
                "ezspVersion": backup.ezsp_version,
            },
        },
        "stack_specific": {
            "zstack": backup.trust_center_link_key_seed.as_ref().map(|seed| json!({"tclk_seed": hex(seed)})),
            "ezsp": backup.hashed_tclk.as_ref().map(|key| json!({"hashed_tclk": hex(key)})),
        },
        "coordinator_ieee": hex(&backup.coordinator_ieee_address),
        "pan_id": format!("{:x}", backup.network_options.pan_id),
        "extended_pan_id": hex(&backup.network_options.extended_pan_id),
        "nwk_update_id": backup.network_update_id,
        "security_level": backup.security_level,
        "channel": backup.logical_channel,
        "channel_mask": backup.network_options.channel_list,
        "network_key": {
            "key": hex(&backup.network_options.network_key),
            "sequence_number": backup.network_key_info.sequence_number,
            "frame_counter": backup.network_key_info.frame_counter,
        },
        "devices": backup.devices.iter().map(|device| json!({
            "nwk_address": device.network_address.map(|address| format!("{address:x}")),
            "ieee_address": hex(&device.ieee_address),
            "is_child": device.is_direct_child,
            "link_key": device.link_key.as_ref().map(|key| json!({
                "key": hex(&key.key),
                "rx_counter": key.rx_counter,
                "tx_counter": key.tx_counter,
            })),
        })).collect::<Vec<_>>(),
    })
}

/// Upstream `fromUnifiedBackup(backup)`.
pub fn from_unified_backup(backup: &UnifiedBackupStorage) -> Result<Backup, String> {
    let network_key = backup
        .get("network_key")
        .ok_or_else(|| "Backup corrupted - missing network key".to_string())?;
    let devices = backup
        .get("devices")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|device| BackupDevice {
            network_address: device
                .get("nwk_address")
                .and_then(Value::as_str)
                .and_then(|value| u16::from_str_radix(value, 16).ok()),
            ieee_address: from_hex(
                device
                    .get("ieee_address")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            ),
            is_direct_child: device
                .get("is_child")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            link_key: device.get("link_key").map(|key| LinkKey {
                key: from_hex(key.get("key").and_then(Value::as_str).unwrap_or_default()),
                rx_counter: key.get("rx_counter").and_then(Value::as_u64).unwrap_or(0) as u32,
                tx_counter: key.get("tx_counter").and_then(Value::as_u64).unwrap_or(0) as u32,
            }),
        })
        .collect();

    Ok(Backup {
        network_options: NetworkOptions {
            pan_id: u16::from_str_radix(
                backup.get("pan_id").and_then(Value::as_str).unwrap_or("0"),
                16,
            )
            .unwrap_or(0),
            extended_pan_id: from_hex(
                backup
                    .get("extended_pan_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            ),
            channel_list: backup
                .get("channel_mask")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_u64)
                        .map(|v| v as u8)
                        .collect()
                })
                .unwrap_or_default(),
            network_key: from_hex(
                network_key
                    .get("key")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            ),
            network_key_distribute: false,
            has_default_extended_pan_id: false,
        },
        logical_channel: backup.get("channel").and_then(Value::as_u64).unwrap_or(0) as u8,
        network_key_info: NetworkKeyInfo {
            sequence_number: network_key
                .get("sequence_number")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            frame_counter: network_key
                .get("frame_counter")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
        },
        coordinator_ieee_address: from_hex(
            backup
                .get("coordinator_ieee")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        ),
        security_level: backup
            .get("security_level")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        network_update_id: backup
            .get("nwk_update_id")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8,
        devices,
        znp_version: backup
            .pointer("/metadata/internal/znpVersion")
            .and_then(Value::as_str)
            .map(str::to_string),
        trust_center_link_key_seed: backup
            .pointer("/stack_specific/zstack/tclk_seed")
            .and_then(Value::as_str)
            .map(from_hex),
        ezsp_version: backup
            .pointer("/metadata/internal/ezspVersion")
            .and_then(Value::as_str)
            .map(str::to_string),
        hashed_tclk: backup
            .pointer("/stack_specific/ezsp/hashed_tclk")
            .and_then(Value::as_str)
            .map(from_hex),
    })
}

/// Upstream `fromLegacyBackup(backup)`.
pub fn from_legacy_backup(backup: &Value) -> Result<Backup, String> {
    let data = backup
        .get("data")
        .ok_or_else(|| "Backup corrupted - missing data".to_string())?;
    let nv = |key: &str| -> Result<Vec<u8>, String> {
        data.get(key)
            .and_then(|entry| entry.get("value"))
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_u64)
                    .map(|v| v as u8)
                    .collect()
            })
            .ok_or_else(|| format!("Backup corrupted - missing {key}"))
    };

    let nib_bytes = nv("ZCD_NV_NIB")?;
    let active_key = nv("ZCD_NV_NWK_ACTIVE_KEY_INFO")?;
    let preconfigured = nv("ZCD_NV_PRECFGKEY_ENABLE")?;
    let material = match nv("ZCD_NV_EX_NWK_SEC_MATERIAL_TABLE")
        .or_else(|_| nv("ZCD_NV_LEGACY_NWK_SEC_MATERIAL_TABLE_START"))
    {
        Ok(value) => value,
        Err(_) => {
            return Err("Backup corrupted - missing network security material table".to_string())
        }
    };
    let extaddr = nv("ZCD_NV_EXTADDR")?;

    let nib = nib(Some(&nib_bytes));
    let key_descriptor = nwk_key_descriptor(Some(&active_key));
    let material = nwk_sec_material_descriptor_entry(Some(&material));

    Ok(Backup {
        network_options: NetworkOptions {
            pan_id: nib.get_u16("nwkPanId").unwrap_or(0),
            extended_pan_id: nib.get_bytes("extendedPANID").unwrap_or_default(),
            channel_list: unpack_channel_list(nib.get_u32("channelList").unwrap_or(0)),
            network_key: key_descriptor.get_bytes("key").unwrap_or_default(),
            network_key_distribute: preconfigured.first().copied().unwrap_or(0) != 0,
            has_default_extended_pan_id: false,
        },
        logical_channel: nib.get_u8("nwkLogicalChannel").unwrap_or(0),
        network_key_info: NetworkKeyInfo {
            sequence_number: key_descriptor.get_u8("keySeqNum").unwrap_or(0) as u32,
            frame_counter: material.get_u32("FrameCounter").unwrap_or(0),
        },
        coordinator_ieee_address: extaddr.into_iter().rev().collect(),
        security_level: nib.get_u8("SecurityLevel").unwrap_or(0),
        network_update_id: nib.get_u8("nwkUpdateId").unwrap_or(0),
        devices: Vec::new(),
        znp_version: None,
        trust_center_link_key_seed: None,
        ezsp_version: None,
        hashed_tclk: None,
    })
}
