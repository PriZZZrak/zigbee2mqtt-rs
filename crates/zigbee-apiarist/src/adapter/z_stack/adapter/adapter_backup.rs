// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/adapter/adapter-backup.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use serde_json::Value;

use crate::adapter::utils::read_backup;
use crate::adapter::z_stack::adapter::adapter_nv_memory::{AdapterNvMemory, TableMode};
use crate::adapter::z_stack::adapter::tstype::ZnpVersion;
use crate::adapter::z_stack::constants::common::{NvItemsIds, NvSystemIds};
use crate::adapter::z_stack::structs::entries::address_manager_entry::AddressManagerUser;
use crate::adapter::z_stack::structs::entries::address_manager_table::address_manager_table;
use crate::adapter::z_stack::structs::entries::aps_link_key_data_table::aps_link_key_data_table;
use crate::adapter::z_stack::structs::entries::aps_tc_link_key_table::aps_tc_link_key_table;
use crate::adapter::z_stack::structs::entries::nib::nib;
use crate::adapter::z_stack::structs::entries::nwk_key::nwk_key;
use crate::adapter::z_stack::structs::entries::nwk_key_descriptor::nwk_key_descriptor;
use crate::adapter::z_stack::structs::entries::nwk_sec_material_descriptor_entry::nwk_sec_material_descriptor_entry;
use crate::adapter::z_stack::structs::entries::nwk_sec_material_descriptor_table::nwk_sec_material_descriptor_table;
use crate::adapter::z_stack::structs::entries::security_manager_entry::SecurityManagerAuthenticationOption;
use crate::adapter::z_stack::structs::entries::security_manager_table::security_manager_table;
use crate::adapter::z_stack::structs::r#struct::Struct;
use crate::adapter::z_stack::structs::serializable_memory_object::SerializableMemoryObject;
use crate::adapter::z_stack::structs::table::{Table, TableData};
use crate::adapter::z_stack::unpi::constants::Subsystem;
use crate::adapter::z_stack::utils::channel_list::{pack_channel_list, unpack_channel_list};
use crate::models::backup::{Backup, BackupDevice, LinkKey, NetworkKeyInfo};
use crate::models::network_options::NetworkOptions;
use crate::utils::backup::{from_legacy_backup, from_unified_backup};

/// Upstream `AdapterBackup`.
pub struct AdapterBackup {
    pub nv: AdapterNvMemory,
    pub default_path: String,
}

impl AdapterBackup {
    pub fn new(nv: AdapterNvMemory, path: impl Into<String>) -> Self {
        Self {
            nv,
            default_path: path.into(),
        }
    }

    /// Upstream `getStoredBackup()`.
    pub fn get_stored_backup(&self) -> Result<Option<Backup>, String> {
        let Some(data) = read_backup(&self.default_path)? else {
            return Ok(None);
        };
        if data.get("adapterType").is_some() {
            return from_legacy_backup(&data).map(Some);
        }
        if data.pointer("/metadata/format").and_then(Value::as_str)
            == Some("zigpy/open-coordinator-backup")
        {
            let version = data
                .pointer("/metadata/version")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            if version != 1 {
                return Err(format!(
                    "Unsupported open coordinator backup version (version={version})"
                ));
            }
            return from_unified_backup(&data).map(Some);
        }
        Err("Unknown backup format".to_string())
    }

    /// Upstream `createBackup(ieeeAddressesInDatabase)`.
    pub async fn create_backup(
        &mut self,
        ieee_addresses_in_database: &[String],
    ) -> Result<Backup, String> {
        let version = self.get_adapter_version().await?;

        let ieee_response = {
            let mut znp = self.nv.znp.lock().await;
            znp.request_with_reply(Subsystem::SYS, "getExtAddr", serde_json::json!({}))
                .await?
        };
        let extaddress = ieee_response
            .payload
            .get("extaddress")
            .and_then(Value::as_str)
            .ok_or_else(|| "Failed to read adapter IEEE address".to_string())?;
        let Some(ieee_hex) = extaddress.strip_prefix("0x") else {
            return Err("Failed to read adapter IEEE address".to_string());
        };
        let ieee_address = hex_to_bytes(ieee_hex);

        let nib = self
            .nv
            .read_item_struct(NvItemsIds::NIB, 0, nib)
            .await?
            .ok_or_else(|| "Cannot backup - adapter not commissioned".to_string())?;

        let active_key_info = if version == ZnpVersion::ZStack12 {
            let key_response = {
                let mut znp = self.nv.znp.lock().await;
                znp.request_with_reply(
                    Subsystem::SAPI,
                    "readConfiguration",
                    serde_json::json!({"configid": NvItemsIds::PRECFGKEY}),
                )
                .await?
            };
            let key_bytes = bytes(key_response.payload.get("value"));
            let key = nwk_key(Some(&key_bytes));
            let mut descriptor = nwk_key_descriptor(None);
            descriptor.set_bytes("key", &key.get_bytes("key").unwrap_or_default());
            Some(descriptor)
        } else {
            self.nv
                .read_item_struct(NvItemsIds::NWK_ACTIVE_KEY_INFO, 0, nwk_key_descriptor)
                .await?
        }
        .ok_or_else(|| "Cannot backup - missing active key info".to_string())?;

        let preconfigured_key_enabled = self.nv.read_item(NvItemsIds::PRECFGKEYS_ENABLE, 0).await?;
        let address_manager_table = self.get_address_manager_table(version).await?;
        let security_manager_table = self.get_security_manager_table().await?;
        let aps_link_key_data_table = self.get_aps_link_key_data_table(version).await?;
        let tclk_seed = if version == ZnpVersion::ZStack12 {
            None
        } else {
            self.nv
                .read_item_struct(NvItemsIds::TCLK_SEED, 0, nwk_key)
                .await?
        };
        let tclk_table = self.get_tclk_table(version).await?;
        let sec_material_table = self.get_network_security_material_table(version).await?;

        let mut sec_material_descriptor = None;
        let generic_extended_pan_id = [0xffu8; 8];
        for entry in sec_material_table.used() {
            if entry.get_bytes("extendedPanID").unwrap_or_default()
                == nib.get_bytes("extendedPANID").unwrap_or_default()
            {
                sec_material_descriptor = Some(entry.clone());
                break;
            }
            if sec_material_descriptor.is_none()
                && entry.get_bytes("extendedPanID").unwrap_or_default() == generic_extended_pan_id
            {
                sec_material_descriptor = Some(entry.clone());
            }
        }
        let sec_material_descriptor = sec_material_descriptor.unwrap_or_else(|| {
            let mut entry = nwk_sec_material_descriptor_entry(None);
            entry.set_bytes(
                "extendedPanID",
                &nib.get_bytes("extendedPANID").unwrap_or_default(),
            );
            entry.set_u32(
                "FrameCounter",
                if version == ZnpVersion::ZStack12 {
                    0
                } else {
                    1250
                },
            );
            entry
        });

        let mut devices = Vec::new();
        for (ami, ame) in address_manager_table.used().iter().enumerate() {
            if !crate::adapter::z_stack::structs::entries::address_manager_entry::is_set(ame) {
                continue;
            }
            let user = ame.get_u8("user").unwrap_or(0);
            if user & (AddressManagerUser::Assoc as u8 | AddressManagerUser::Security as u8) == 0 {
                continue;
            }

            let mut link_key_info: Option<LinkKey> = None;
            if let Some(sme) = security_manager_table
                .used()
                .iter()
                .find(|entry| entry.get_u16("ami") == Some(ami as u16))
            {
                let key_nv_id = sme.get_u16("keyNvId").unwrap_or(0);
                let aps_key_data_index = if version == ZnpVersion::ZStack30x {
                    key_nv_id.saturating_sub(NvItemsIds::APS_LINK_KEY_DATA_START)
                } else {
                    key_nv_id
                } as usize;
                if let Some(aps_key_data) = aps_link_key_data_table.used().get(aps_key_data_index) {
                    link_key_info = Some(LinkKey {
                        key: aps_key_data.get_bytes("key").unwrap_or_default(),
                        rx_counter: aps_key_data.get_u32("rxFrmCntr").unwrap_or(0),
                        tx_counter: aps_key_data.get_u32("txFrmCntr").unwrap_or(0),
                    });
                }
            } else {
                let ext_addr = ame.get_bytes("extAddr").unwrap_or_default();
                if let Some(tclk_entry) = tclk_table
                    .used()
                    .iter()
                    .find(|entry| entry.get_bytes("extAddr").unwrap_or_default() == ext_addr)
                {
                    let Some(seed) = tclk_seed.as_ref() else {
                        return Err(
                            "Cannot be undefined because this is only called for ZStack 3"
                                .to_string(),
                        );
                    };
                    let seed_key = seed.get_bytes("key").unwrap_or_default();
                    let shift = tclk_entry.get_u8("SeedShift_IcIndex").unwrap_or(0) as usize;
                    let mut rotated = Vec::with_capacity(seed_key.len());
                    rotated.extend_from_slice(&seed_key[shift..]);
                    rotated.extend_from_slice(&seed_key[..shift]);
                    let mut ext_addr_reversed = ext_addr.clone();
                    ext_addr_reversed.reverse();
                    let mut ext_addr_repeated = ext_addr_reversed.clone();
                    ext_addr_repeated.extend_from_slice(&ext_addr_reversed);
                    let mut derived = vec![0u8; 16];
                    for index in 0..16 {
                        derived[index] = rotated[index] ^ ext_addr_repeated[index];
                    }
                    link_key_info = Some(LinkKey {
                        key: derived,
                        rx_counter: tclk_entry.get_u32("rxFrmCntr").unwrap_or(0),
                        tx_counter: tclk_entry.get_u32("txFrmCntr").unwrap_or(0),
                    });
                }
            }

            devices.push(BackupDevice {
                network_address: ame.get_u16("nwkAddr"),
                ieee_address: ame.get_bytes("extAddr").unwrap_or_default(),
                is_direct_child: user & AddressManagerUser::Assoc as u8 > 0,
                link_key: link_key_info,
            });
        }

        let mut backup = Backup {
            network_options: NetworkOptions {
                pan_id: nib.get_u16("nwkPanId").unwrap_or(0),
                extended_pan_id: nib.get_bytes("extendedPANID").unwrap_or_default(),
                channel_list: unpack_channel_list(nib.get_u32("channelList").unwrap_or(0)),
                network_key: active_key_info.get_bytes("key").unwrap_or_default(),
                network_key_distribute: preconfigured_key_enabled
                    .as_deref()
                    .and_then(|value| value.first())
                    .is_some_and(|value| *value == 0x01),
                has_default_extended_pan_id: false,
            },
            logical_channel: nib.get_u8("nwkLogicalChannel").unwrap_or(0),
            network_key_info: NetworkKeyInfo {
                sequence_number: active_key_info.get_u8("keySeqNum").unwrap_or(0) as u32,
                frame_counter: sec_material_descriptor.get_u32("FrameCounter").unwrap_or(0),
            },
            security_level: nib.get_u8("SecurityLevel").unwrap_or(0),
            network_update_id: nib.get_u8("nwkUpdateId").unwrap_or(0),
            coordinator_ieee_address: ieee_address,
            devices,
            znp_version: Some(version as u16),
            trust_center_link_key_seed: tclk_seed.as_ref().and_then(|seed| seed.get_bytes("key")),
            ezsp_version: None,
            hashed_tclk: None,
        };

        if let Ok(Some(old_backup)) = self.get_stored_backup() {
            let missing = old_backup
                .devices
                .iter()
                .filter(|device| {
                    device.link_key.is_some()
                        && ieee_addresses_in_database
                            .contains(&format!("0x{}", hex(&device.ieee_address)))
                        && !backup
                            .devices
                            .iter()
                            .any(|new| new.ieee_address == device.ieee_address)
                })
                .cloned()
                .collect::<Vec<_>>();
            backup.devices.extend(missing);
        }

        Ok(backup)
    }

    /// Upstream `restoreBackup(backup)`.
    pub async fn restore_backup(&mut self, backup: &Backup) -> Result<(), String> {
        let version = self.get_adapter_version().await?;
        if version == ZnpVersion::ZStack12 {
            return Err("backup cannot be restored on Z-Stack 1.2 adapter".to_string());
        }

        let mut nib = self
            .nv
            .read_item_struct(NvItemsIds::NIB, 0, nib)
            .await?
            .ok_or_else(|| "Cannot restore - adapter not commissioned".to_string())?;
        nib.set_u16("nwkPanId", backup.network_options.pan_id);
        nib.set_u32(
            "channelList",
            pack_channel_list(&backup.network_options.channel_list)?,
        );
        nib.set_u8(
            "nwkLogicalChannel",
            backup
                .network_options
                .channel_list
                .first()
                .copied()
                .unwrap_or(0),
        );
        nib.set_bytes("extendedPANID", &backup.network_options.extended_pan_id);
        nib.set_u8("SecurityLevel", backup.security_level);
        nib.set_u8("nwkUpdateId", backup.network_update_id);

        let mut key_descriptor = nwk_key_descriptor(None);
        key_descriptor.set_u8("keySeqNum", backup.network_key_info.sequence_number as u8);
        key_descriptor.set_bytes("key", &backup.network_options.network_key);

        let current_address_manager_table = self.get_address_manager_table(version).await?;
        let current_security_manager_table = self.get_security_manager_table().await?;
        let current_aps_link_key_data_table = self.get_aps_link_key_data_table(version).await?;
        let current_tclk_table = self.get_tclk_table(version).await?;
        let current_nwk_sec_material_table =
            self.get_network_security_material_table(version).await?;

        let mut address_manager_table = address_manager_table(
            TableData::Capacity(current_address_manager_table.capacity()),
            crate::adapter::z_stack::structs::r#struct::Alignment::Unaligned,
        );
        let mut security_manager_table = security_manager_table(
            TableData::Capacity(current_security_manager_table.capacity()),
            crate::adapter::z_stack::structs::r#struct::Alignment::Unaligned,
        );
        let mut aps_link_key_data_table = aps_link_key_data_table(
            TableData::Capacity(current_aps_link_key_data_table.capacity()),
            crate::adapter::z_stack::structs::r#struct::Alignment::Unaligned,
        );
        let mut tclk_table = aps_tc_link_key_table(
            TableData::Capacity(current_tclk_table.capacity()),
            crate::adapter::z_stack::structs::r#struct::Alignment::Unaligned,
        );
        let mut nwk_security_material_table = nwk_sec_material_descriptor_table(
            TableData::Capacity(current_nwk_sec_material_table.capacity()),
            crate::adapter::z_stack::structs::r#struct::Alignment::Unaligned,
        );

        if nwk_security_material_table.capacity() > 0 {
            let first = nwk_security_material_table.entry_mut(0).expect("entry");
            first.set_bytes("extendedPanID", &backup.network_options.extended_pan_id);
            first.set_u32("FrameCounter", backup.network_key_info.frame_counter + 2500);
        }
        if nwk_security_material_table.capacity() > 1 {
            let last_index = nwk_security_material_table.capacity() - 1;
            let last = nwk_security_material_table
                .entry_mut(last_index)
                .expect("entry");
            last.set_bytes("extendedPanID", &[0xff; 8]);
            last.set_u32("FrameCounter", backup.network_key_info.frame_counter + 2500);
        }

        for device in &backup.devices {
            let ame_index = address_manager_table.next_free_index().ok_or_else(|| {
                "target adapter address manager table size insufficient".to_string()
            })?;
            {
                let ame = address_manager_table.entry_mut(ame_index).expect("entry");
                ame.set_u16("nwkAddr", device.network_address.unwrap_or(0xffff));
                ame.set_bytes("extAddr", &device.ieee_address);
                ame.set_u8(
                    "user",
                    if device.is_direct_child {
                        AddressManagerUser::Assoc as u8
                    } else {
                        AddressManagerUser::Default as u8
                    },
                );
            }
            if let Some(link_key) = &device.link_key {
                let mut link_key_processed = false;
                if let Some(seed) = backup.trust_center_link_key_seed.as_ref() {
                    let mut ext_addr_reversed = device.ieee_address.clone();
                    ext_addr_reversed.reverse();
                    let mut ext_addr_repeated = ext_addr_reversed.clone();
                    ext_addr_repeated.extend_from_slice(&ext_addr_reversed);
                    let mut recovered_key = vec![0u8; 16];
                    for index in 0..16 {
                        recovered_key[index] = link_key.key[index] ^ ext_addr_repeated[index];
                    }
                    let mut recovered_seed_shift = None;
                    for shift in 0..16 {
                        let mut rotated = Vec::with_capacity(16);
                        rotated.extend_from_slice(&recovered_key[16 - shift..]);
                        rotated.extend_from_slice(&recovered_key[..16 - shift]);
                        if rotated == *seed {
                            recovered_seed_shift = Some(shift as u8);
                            break;
                        }
                    }
                    if let Some(seed_shift) = recovered_seed_shift {
                        {
                            let ame = address_manager_table.entry_mut(ame_index).expect("entry");
                            ame.set_u8(
                                "user",
                                ame.get_u8("user").unwrap_or(0)
                                    | AddressManagerUser::Security as u8,
                            );
                        }
                        let tclk_index = tclk_table.next_free_index().ok_or_else(|| {
                            format!(
                                "target adapter tclk table size insufficient (size={})",
                                tclk_table.capacity()
                            )
                        })?;
                        {
                            let entry = tclk_table.entry_mut(tclk_index).expect("entry");
                            entry.set_bytes("extAddr", &device.ieee_address);
                            entry.set_u8("SeedShift_IcIndex", seed_shift);
                            entry.set_u8("keyAttributes", 2);
                            entry.set_u8("keyType", 0);
                            entry.set_u32("rxFrmCntr", link_key.rx_counter);
                            entry.set_u32("txFrmCntr", link_key.tx_counter + 2500);
                        }
                        link_key_processed = true;
                    }
                }
                if !link_key_processed {
                    {
                        let ame = address_manager_table.entry_mut(ame_index).expect("entry");
                        ame.set_u8(
                            "user",
                            ame.get_u8("user").unwrap_or(0) | AddressManagerUser::Security as u8,
                        );
                    }
                    let aps_index = aps_link_key_data_table.next_free_index().ok_or_else(|| {
                        format!(
                            "target adapter aps link key data table size insufficient (size={})",
                            aps_link_key_data_table.capacity()
                        )
                    })?;
                    {
                        let entry = aps_link_key_data_table.entry_mut(aps_index).expect("entry");
                        entry.set_bytes("key", &link_key.key);
                        entry.set_u32("rxFrmCntr", link_key.rx_counter);
                        entry.set_u32("txFrmCntr", link_key.tx_counter + 2500);
                    }
                    let sme_index = security_manager_table.next_free_index().ok_or_else(|| {
                        format!(
                            "target adapter security manager table size insufficient (size={})",
                            security_manager_table.capacity()
                        )
                    })?;
                    let key_nv_id = if version == ZnpVersion::ZStack3x0 {
                        aps_index as u16
                    } else {
                        NvItemsIds::APS_LINK_KEY_DATA_START + aps_index as u16
                    };
                    {
                        let sme = security_manager_table.entry_mut(sme_index).expect("entry");
                        sme.set_u16("ami", ame_index as u16);
                        sme.set_u16("keyNvId", key_nv_id);
                        sme.set_u8(
                            "authenticationOption",
                            SecurityManagerAuthenticationOption::AuthenticatedCBCK as u8,
                        );
                    }
                }
            }
        }

        let mut reversed_adapter_ieee = backup.coordinator_ieee_address.clone();
        reversed_adapter_ieee.reverse();
        self.nv
            .write_item(NvItemsIds::EXTADDR, &reversed_adapter_ieee, 0, true)
            .await?;
        self.nv
            .write_item_object(NvItemsIds::NIB, &nib, 0, true)
            .await?;
        // Upstream calls `keyDescriptor.serialize()` (unaligned) here, not the
        // adapter memory alignment.
        let key_bytes = key_descriptor
            .serialize_memory(crate::adapter::z_stack::structs::r#struct::Alignment::Unaligned);
        self.nv
            .update_item(NvItemsIds::NWK_ACTIVE_KEY_INFO, &key_bytes, true)
            .await?;
        self.nv
            .update_item(NvItemsIds::NWK_ALTERN_KEY_INFO, &key_bytes, true)
            .await?;
        if let Some(seed) = backup.trust_center_link_key_seed.as_ref() {
            self.nv
                .write_item(NvItemsIds::TCLK_SEED, seed, 0, true)
                .await?;
        }

        let extended = version == ZnpVersion::ZStack3x0;
        if extended {
            self.nv
                .write_table(
                    TableMode::Extended {
                        sys_id: NvSystemIds::ZSTACK,
                        id: NvItemsIds::EX_NWK_SEC_MATERIAL_TABLE,
                        max_length: None,
                    },
                    &nwk_security_material_table,
                )
                .await?;
        } else {
            self.nv
                .write_table(
                    TableMode::Legacy {
                        id: NvItemsIds::LEGACY_NWK_SEC_MATERIAL_TABLE_START,
                        max_length: 12,
                    },
                    &nwk_security_material_table,
                )
                .await?;
        }

        if extended {
            self.nv
                .write_table(
                    TableMode::Extended {
                        sys_id: NvSystemIds::ZSTACK,
                        id: NvItemsIds::ZCD_NV_EX_ADDRMGR,
                        max_length: None,
                    },
                    &address_manager_table,
                )
                .await?;
        } else {
            self.nv
                .write_item_object(NvItemsIds::ADDRMGR, &address_manager_table, 0, true)
                .await?;
        }
        self.nv
            .write_item_object(
                NvItemsIds::APS_LINK_KEY_TABLE,
                &security_manager_table,
                0,
                true,
            )
            .await?;

        if extended {
            self.nv
                .write_table(
                    TableMode::Extended {
                        sys_id: NvSystemIds::ZSTACK,
                        id: NvItemsIds::ZCD_NV_EX_APS_KEY_DATA_TABLE,
                        max_length: None,
                    },
                    &aps_link_key_data_table,
                )
                .await?;
        } else {
            self.nv
                .write_table(
                    TableMode::Legacy {
                        id: NvItemsIds::APS_LINK_KEY_DATA_START,
                        max_length: 255,
                    },
                    &aps_link_key_data_table,
                )
                .await?;
        }

        if extended {
            self.nv
                .write_table(
                    TableMode::Extended {
                        sys_id: NvSystemIds::ZSTACK,
                        id: NvItemsIds::EX_TCLK_TABLE,
                        max_length: None,
                    },
                    &tclk_table,
                )
                .await?;
        } else {
            self.nv
                .write_table(
                    TableMode::Legacy {
                        id: NvItemsIds::LEGACY_TCLK_TABLE_START,
                        max_length: 239,
                    },
                    &tclk_table,
                )
                .await?;
        }

        Ok(())
    }

    async fn get_adapter_version(&mut self) -> Result<ZnpVersion, String> {
        let version = {
            let mut znp = self.nv.znp.lock().await;
            znp.request_with_reply(Subsystem::SYS, "version", serde_json::json!({}))
                .await?
        };
        let product = version
            .payload
            .get("product")
            .and_then(Value::as_u64)
            .ok_or_else(|| "Cannot determine adapter version".to_string())?;
        ZnpVersion::from_product(product)
            .ok_or_else(|| format!("Unsupported adapter version (product={product})"))
    }

    async fn get_address_manager_table(&mut self, version: ZnpVersion) -> Result<Table, String> {
        if version == ZnpVersion::ZStack3x0 {
            return self
                .nv
                .read_table_as(
                    TableMode::Extended {
                        sys_id: NvSystemIds::ZSTACK,
                        id: NvItemsIds::ZCD_NV_EX_ADDRMGR,
                        max_length: None,
                    },
                    address_manager_table,
                )
                .await;
        }
        self.nv
            .read_item_table(NvItemsIds::ADDRMGR, 0, address_manager_table)
            .await?
            .ok_or_else(|| "Cannot read address manager table".to_string())
    }

    async fn get_security_manager_table(&mut self) -> Result<Table, String> {
        self.nv
            .read_item_table(NvItemsIds::APS_LINK_KEY_TABLE, 0, security_manager_table)
            .await?
            .ok_or_else(|| "Cannot read security manager table".to_string())
    }

    async fn get_aps_link_key_data_table(&mut self, version: ZnpVersion) -> Result<Table, String> {
        if version == ZnpVersion::ZStack3x0 {
            return self
                .nv
                .read_table_as(
                    TableMode::Extended {
                        sys_id: NvSystemIds::ZSTACK,
                        id: NvItemsIds::ZCD_NV_EX_APS_KEY_DATA_TABLE,
                        max_length: None,
                    },
                    aps_link_key_data_table,
                )
                .await;
        }
        self.nv
            .read_table_as(
                TableMode::Legacy {
                    id: NvItemsIds::APS_LINK_KEY_DATA_START,
                    max_length: 255,
                },
                aps_link_key_data_table,
            )
            .await
    }

    async fn get_tclk_table(&mut self, version: ZnpVersion) -> Result<Table, String> {
        if version == ZnpVersion::ZStack3x0 {
            return self
                .nv
                .read_table_as(
                    TableMode::Extended {
                        sys_id: NvSystemIds::ZSTACK,
                        id: NvItemsIds::EX_TCLK_TABLE,
                        max_length: None,
                    },
                    aps_tc_link_key_table,
                )
                .await;
        }
        self.nv
            .read_table_as(
                TableMode::Legacy {
                    id: NvItemsIds::LEGACY_TCLK_TABLE_START,
                    max_length: 239,
                },
                aps_tc_link_key_table,
            )
            .await
    }

    async fn get_network_security_material_table(
        &mut self,
        version: ZnpVersion,
    ) -> Result<Table, String> {
        if version == ZnpVersion::ZStack3x0 {
            return self
                .nv
                .read_table_as(
                    TableMode::Extended {
                        sys_id: NvSystemIds::ZSTACK,
                        id: NvItemsIds::EX_NWK_SEC_MATERIAL_TABLE,
                        max_length: None,
                    },
                    nwk_sec_material_descriptor_table,
                )
                .await;
        }
        self.nv
            .read_table_as(
                TableMode::Legacy {
                    id: NvItemsIds::LEGACY_NWK_SEC_MATERIAL_TABLE_START,
                    max_length: 12,
                },
                nwk_sec_material_descriptor_table,
            )
            .await
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hex_to_bytes(value: &str) -> Vec<u8> {
    (0..value.len())
        .step_by(2)
        .filter_map(|index| u8::from_str_radix(value.get(index..index + 2)?, 16).ok())
        .collect()
}

fn bytes(value: Option<&Value>) -> Vec<u8> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_u64)
                .map(|value| value as u8)
                .collect()
        })
        .unwrap_or_default()
}
