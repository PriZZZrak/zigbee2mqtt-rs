// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/adapter/manager.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use std::future::Future;
use std::pin::Pin;

use serde_json::{json, Value};

use crate::adapter::z_stack::adapter::adapter_backup::AdapterBackup;
use crate::adapter::z_stack::adapter::adapter_nv_memory::AdapterNvMemory;
use crate::adapter::z_stack::adapter::endpoints::endpoints;
use crate::adapter::z_stack::adapter::tstype::ZnpVersion;
use crate::adapter::z_stack::constants::common::{DevStates, NvItemsIds, ZnpCommandStatus};
use crate::adapter::z_stack::constants::{self as ZnpConstants, sys, zdo};
use crate::adapter::z_stack::structs::entries::channel_list::channel_list;
use crate::adapter::z_stack::structs::entries::has_configured::is_configured;
use crate::adapter::z_stack::structs::entries::nib::nib;
use crate::adapter::z_stack::structs::entries::nwk_key::nwk_key;
use crate::adapter::z_stack::structs::entries::nwk_key_descriptor::nwk_key_descriptor;
use crate::adapter::z_stack::structs::entries::nwk_pan_id::nwk_pan_id;
use crate::adapter::z_stack::structs::r#struct::Struct;
use crate::adapter::z_stack::structs::serializable_memory_object::SerializableMemoryObject;
use crate::adapter::z_stack::unpi::constants::{Subsystem, Type};
use crate::adapter::z_stack::utils::channel_list::{compare_channel_lists, pack_channel_list};
use crate::adapter::z_stack::utils::network_options::compare_network_options;
use crate::adapter::z_stack::znp::znp::Znp;
use crate::models::network_options::NetworkOptions;
use crate::models::startup_options::{AdapterOptions, ConfigNetworkOptions, StartupOptions};
use crate::utils::logger::logger;
use crate::utils::wait::wait;
use crate::zspec::zdo::buffalo_zdo::BuffaloZdo;
use crate::zspec::zdo::definition::clusters::ClusterId;

const NS: &str = "zh:adapter:zstack:manager";
const BLANK_EUI64: &str = "0x0000000000000000";
const COORDINATOR_ADDRESS: u16 = 0;
const NULL_NODE_ID: u16 = 0xffff;
const GREEN_POWER_ENDPOINT: u8 = 242;

/// Upstream `TsType.StartResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartResult {
    Resumed,
    Restored,
    Reset,
}

/// Upstream `StartupStrategy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartupStrategy {
    Startup,
    RestoreBackup,
    StartCommissioning,
}

/// Adapter surface used by the manager (`sendZdo` + overhead flag).
pub trait ZdoAdapter {
    fn has_zdo_message_overhead(&self) -> bool;
    fn send_zdo<'a>(
        &'a mut self,
        znp: &'a mut Znp,
        ieee_address: &'a str,
        network_address: u16,
        cluster_id: u16,
        payload: &'a [u8],
        disable_response: bool,
    ) -> Pin<Box<dyn Future<Output = Result<Value, String>> + Send + 'a>>;
}

/// Upstream `ZnpAdapterManager`.
pub struct ZnpAdapterManager<A: ZdoAdapter> {
    pub backup: AdapterBackup,
    pub adapter: A,
    pub options: StartupOptions,
    nwk_options: Option<NetworkOptions>,
}

impl<A: ZdoAdapter> ZnpAdapterManager<A> {
    async fn znp_request(
        &mut self,
        subsystem: Subsystem,
        command: &str,
        payload: Value,
        waiter_id: Option<u64>,
        timeout: Option<u64>,
        expected_statuses: &[u8],
    ) -> Result<Option<crate::adapter::z_stack::znp::zpi_object::ZpiObject>, String> {
        let mut znp = self.backup.nv.znp.lock().await;
        znp.request(
            subsystem,
            command,
            payload,
            waiter_id,
            timeout,
            expected_statuses,
        )
        .await
    }

    async fn znp_request_with_reply(
        &mut self,
        subsystem: Subsystem,
        command: &str,
        payload: Value,
    ) -> Result<crate::adapter::z_stack::znp::zpi_object::ZpiObject, String> {
        let mut znp = self.backup.nv.znp.lock().await;
        znp.request_with_reply(subsystem, command, payload).await
    }

    async fn znp_wait_for(
        &mut self,
        r#type: Type,
        subsystem: Subsystem,
        command: &str,
        target: Option<Value>,
        transid: Option<u8>,
        state: Option<u8>,
        timeout: Option<u64>,
    ) -> (
        u64,
        tokio::sync::oneshot::Receiver<
            Result<crate::adapter::z_stack::znp::zpi_object::ZpiObject, String>,
        >,
    ) {
        let znp = self.backup.nv.znp.lock().await;
        znp.wait_for(r#type, subsystem, command, target, transid, state, timeout)
            .await
    }

    pub fn new(adapter: A, nv: AdapterNvMemory, options: StartupOptions) -> Self {
        let backup = AdapterBackup::new(nv, options.backup_path.clone());
        Self {
            backup,
            adapter,
            options,
            nwk_options: None,
        }
    }

    /// Upstream `start()`.
    pub async fn start(&mut self) -> Result<StartResult, String> {
        logger().debug("beginning znp startup", NS);
        self.nwk_options = Some(self.parse_config_network_options().await?);
        self.backup.nv.init().await?;

        let strategy = self.determine_strategy().await?;
        logger().debug(&format!("determined startup strategy: {strategy:?}"), NS);

        let result = match strategy {
            StartupStrategy::Startup => {
                self.begin_startup().await?;
                StartResult::Resumed
            }
            StartupStrategy::RestoreBackup => {
                if self.options.version == ZnpVersion::ZStack12 {
                    logger().debug(
                        "performing recommissioning instead of restore for z-stack 1.2",
                        NS,
                    );
                    let nwk = self.nwk_options.clone().expect("network options");
                    self.begin_commissioning(nwk, true, true).await?;
                    self.begin_startup().await?;
                } else {
                    self.begin_restore().await?;
                }
                StartResult::Restored
            }
            StartupStrategy::StartCommissioning => {
                if self.options.version == ZnpVersion::ZStack12 {
                    let has_configured = self
                        .backup
                        .nv
                        .read_item_struct(
                            NvItemsIds::ZNP_HAS_CONFIGURED_ZSTACK1,
                            0,
                            super::super::structs::entries::has_configured::has_configured,
                        )
                        .await?;
                    let nwk = self.nwk_options.clone().expect("network options");
                    self.begin_commissioning(nwk, true, true).await?;
                    self.begin_startup().await?;
                    if has_configured.as_ref().is_some_and(is_configured) {
                        StartResult::Reset
                    } else {
                        StartResult::Restored
                    }
                } else {
                    let nwk = self.nwk_options.clone().expect("network options");
                    self.begin_commissioning(nwk, true, true).await?;
                    StartResult::Reset
                }
            }
        };

        self.register_endpoints().await?;
        self.add_to_group(GREEN_POWER_ENDPOINT, self.options.green_power_group)
            .await?;
        Ok(result)
    }

    async fn determine_strategy(&mut self) -> Result<StartupStrategy, String> {
        logger().debug("determining znp startup strategy", NS);
        let nwk_options = self.nwk_options.clone().expect("network options");
        let version = self.options.version;

        let has_configured_id = if version == ZnpVersion::ZStack12 {
            NvItemsIds::ZNP_HAS_CONFIGURED_ZSTACK1
        } else {
            NvItemsIds::ZNP_HAS_CONFIGURED_ZSTACK3
        };
        let has_configured = self
            .backup
            .nv
            .read_item_struct(
                has_configured_id,
                0,
                super::super::structs::entries::has_configured::has_configured,
            )
            .await?;
        let nib_value = self
            .backup
            .nv
            .read_item_struct(NvItemsIds::NIB, 0, nib)
            .await?;

        let preconfigured_key = if version == ZnpVersion::ZStack12 {
            let response = self
                .znp_request_with_reply(
                    Subsystem::SAPI,
                    "readConfiguration",
                    json!({"configid": NvItemsIds::PRECFGKEY}),
                )
                .await?;
            let key_bytes = response
                .payload
                .get("value")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_u64)
                        .map(|value| value as u8)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            nwk_key(Some(&key_bytes))
        } else {
            self.backup
                .nv
                .read_item_struct(NvItemsIds::PRECFGKEY, 0, nwk_key)
                .await?
                .ok_or_else(|| "Cannot read preconfigured key".to_string())?
        };

        let mut active_key_info = self
            .backup
            .nv
            .read_item_struct(NvItemsIds::NWK_ACTIVE_KEY_INFO, 0, nwk_key_descriptor)
            .await?;
        let mut alternate_key_info = self
            .backup
            .nv
            .read_item_struct(NvItemsIds::NWK_ALTERN_KEY_INFO, 0, nwk_key_descriptor)
            .await?;

        if version == ZnpVersion::ZStack12 {
            let key = preconfigured_key.get_bytes("key").unwrap_or_default();
            let mut active = nwk_key_descriptor(None);
            active.set_bytes("key", &key);
            let mut alternate = nwk_key_descriptor(None);
            alternate.set_bytes("key", &key);
            active_key_info = Some(active);
            alternate_key_info = Some(alternate);
        }

        let backup = self.backup.get_stored_backup()?;
        let nib_value = nib_value;
        let is_extended_pan_id_reversed = nib_value.as_ref().is_some_and(|nib| {
            let mut reversed = nib.get_bytes("extendedPANID").unwrap_or_default();
            reversed.reverse();
            reversed == nwk_options.extended_pan_id
        });

        let config_matches_adapter = nib_value.as_ref().is_some_and(|nib| {
            let active = active_key_info
                .as_ref()
                .and_then(|key| key.get_bytes("key"))
                .unwrap_or_default();
            let alternate = alternate_key_info
                .as_ref()
                .and_then(|key| key.get_bytes("key"))
                .unwrap_or_default();
            nwk_options.pan_id == nib.get_u16("nwkPanId").unwrap_or(0)
                && (nwk_options.extended_pan_id
                    == nib.get_bytes("extendedPANID").unwrap_or_default()
                    || is_extended_pan_id_reversed
                    || nwk_options.has_default_extended_pan_id)
                && (version != ZnpVersion::ZStack12
                    || nwk_options.network_key
                        == preconfigured_key.get_bytes("key").unwrap_or_default())
                && nwk_options.network_key == active
                && nwk_options.network_key == alternate
        });

        let backup_matches_adapter = backup.as_ref().is_some_and(|backup| {
            nib_value.as_ref().is_some_and(|nib| {
                backup.network_options.pan_id == nib.get_u16("nwkPanId").unwrap_or(0)
                    && backup.network_options.extended_pan_id
                        == nib.get_bytes("extendedPANID").unwrap_or_default()
                    && compare_channel_lists(
                        &backup.network_options.channel_list,
                        &crate::adapter::z_stack::utils::channel_list::unpack_channel_list(
                            nib.get_u32("channelList").unwrap_or(0),
                        ),
                    )
                    && active_key_info.as_ref().is_some_and(|key| {
                        backup.network_options.network_key
                            == key.get_bytes("key").unwrap_or_default()
                    })
            })
        });

        let config_matches_backup = backup.as_ref().is_some_and(|backup| {
            compare_network_options(&nwk_options, &backup.network_options, true)
        });

        let check_restore_version_compatibility = |backup: &Option<
            crate::models::backup::Backup,
        >| {
            if version == ZnpVersion::ZStack12 {
                if let Some(backup) = backup {
                    if backup.znp_version.is_some()
                        && backup.znp_version != Some(ZnpVersion::ZStack12 as u16)
                    {
                        return Err("your backup is from newer platform version (Z-Stack 3.0.x+) and cannot be restored onto Z-Stack 1.2 adapter - please remove backup before proceeding".to_string());
                    }
                }
            }
            Ok(())
        };

        if !has_configured.as_ref().is_some_and(is_configured) || nib_value.is_none() {
            logger().debug("(stage-1) adapter is not configured / not commissioned", NS);
            if config_matches_backup {
                logger().debug("(stage-2) configuration matches backup", NS);
                check_restore_version_compatibility(&backup)?;
                return Ok(StartupStrategy::RestoreBackup);
            }
            if backup.is_none() {
                logger().debug("(stage-2) adapter backup does not exist", NS);
            } else {
                logger().debug("(stage-2) configuration does not match backup", NS);
            }
            return Ok(StartupStrategy::StartCommissioning);
        }
        logger().debug("(stage-1) adapter is configured", NS);
        let nib_value = nib_value.expect("nib");

        if config_matches_adapter {
            if version != ZnpVersion::ZStack12
                && nwk_options.network_key != preconfigured_key.get_bytes("key").unwrap_or_default()
            {
                logger().warning(
                    &format!(
                        "Adapter preconfigured (transport) key does not match configured network key (preconfigured={}, configured={}). This is typically harmless on Z-Stack 3.x adapters where only the active key matters.",
                        hex(&preconfigured_key.get_bytes("key").unwrap_or_default()),
                        hex(&nwk_options.network_key)
                    ),
                    NS,
                );
            }
            if is_extended_pan_id_reversed {
                logger().debug("(stage-2) extended pan id is reversed", NS);
                logger().warning(
                    &format!(
                        "Extended PAN ID is reversed (expected={}, actual={})",
                        hex(&nwk_options.extended_pan_id),
                        hex(&nib_value.get_bytes("extendedPANID").unwrap_or_default())
                    ),
                    NS,
                );
            }
            logger().debug("(stage-2) adapter state matches configuration", NS);
            return Ok(StartupStrategy::Startup);
        }

        logger().debug("(stage-2) adapter state does not match configuration", NS);
        if let Some(backup) = backup {
            logger().debug("(stage-3) got adapter backup", NS);
            if backup_matches_adapter {
                logger().debug("(stage-4) adapter state matches backup", NS);
                logger().error(
                    "Configuration is not consistent with adapter state/backup!",
                    NS,
                );
                logger().error(
                    &format!(
                        "- PAN ID: configured={}, adapter={}",
                        nwk_options.pan_id,
                        nib_value.get_u16("nwkPanId").unwrap_or(0)
                    ),
                    NS,
                );
                logger().error(
                    &format!(
                        "- Extended PAN ID: configured={}, adapter={}",
                        hex(&nwk_options.extended_pan_id),
                        hex(&nib_value.get_bytes("extendedPANID").unwrap_or_default())
                    ),
                    NS,
                );
                logger().error(
                    &format!(
                        "- Network Key: configured={}, adapter:active={} adapter:preconfigured={}, adapter:alternate={}",
                        hex(&nwk_options.network_key),
                        hex(&active_key_info.as_ref().and_then(|key| key.get_bytes("key")).unwrap_or_default()),
                        hex(&preconfigured_key.get_bytes("key").unwrap_or_default()),
                        hex(&alternate_key_info.as_ref().and_then(|key| key.get_bytes("key")).unwrap_or_default())
                    ),
                    NS,
                );
                logger().error(
                    &format!(
                        "- Channel List: configured={:?}, adapter={:?}",
                        nwk_options.channel_list,
                        crate::adapter::z_stack::utils::channel_list::unpack_channel_list(
                            nib_value.get_u32("channelList").unwrap_or(0)
                        )
                    ),
                    NS,
                );
                logger().error("Please update configuration to prevent further issues.", NS);
                logger().error(
                    &format!(
                        "If you wish to re-commission your network, please remove coordinator backup at {}.",
                        self.options.backup_path
                    ),
                    NS,
                );
                logger().error(
                    "Re-commissioning your network will require re-pairing of all devices!",
                    NS,
                );
                if self
                    .options
                    .adapter_options
                    .force_start_with_inconsistent_adapter_configuration
                {
                    logger().error(
                        "Running despite adapter configuration mismatch as configured. Please update the adapter to compatible firmware and recreate your network as soon as possible.",
                        NS,
                    );
                    return Ok(StartupStrategy::Startup);
                }
                return Err("startup failed - configuration-adapter mismatch - see logs above for more information".to_string());
            }
            logger().debug("(stage-4) adapter state does not match backup", NS);
            if config_matches_backup {
                logger().debug("(stage-5) adapter backup matches configuration", NS);
                check_restore_version_compatibility(&Some(backup))?;
                return Ok(StartupStrategy::RestoreBackup);
            }
            logger().debug("(stage-5) adapter backup does not match configuration", NS);
            return Ok(StartupStrategy::StartCommissioning);
        }
        logger().debug("(stage-3) configuration-adapter mismatch (no backup)", NS);
        Ok(StartupStrategy::StartCommissioning)
    }

    async fn begin_startup(&mut self) -> Result<(), String> {
        let device_info = self
            .znp_request_with_reply(Subsystem::UTIL, "getDeviceInfo", json!({}))
            .await?;
        if device_info
            .payload
            .get("devicestate")
            .and_then(Value::as_u64)
            != Some(DevStates::ZB_COORD as u64)
        {
            logger().debug("starting adapter as coordinator", NS);
            let (_, receiver) = self
                .znp_wait_for(
                    Type::AREQ,
                    Subsystem::ZDO,
                    "stateChangeInd",
                    None,
                    None,
                    Some(9),
                    Some(60000),
                )
                .await;
            self.znp_request(
                Subsystem::ZDO,
                "startupFromApp",
                json!({"startdelay": 100}),
                None,
                None,
                &[
                    ZnpCommandStatus::SUCCESS as u8,
                    ZnpCommandStatus::FAILURE as u8,
                ],
            )
            .await?;
            receiver.await.map_err(|_| "Waiter dropped".to_string())??;
            logger().debug("adapter successfully started in coordinator mode", NS);
        } else {
            logger().debug("adapter is already running in coordinator mode", NS);
        }
        Ok(())
    }

    async fn begin_restore(&mut self) -> Result<(), String> {
        let backup = self
            .backup
            .get_stored_backup()?
            .ok_or_else(|| "Cannot restore backup - none is available".to_string())?;
        let provisioning = NetworkOptions {
            pan_id: 1 + (f64::from(random_u16()) / 65535.0 * 65532.0).round() as u16,
            extended_pan_id: random_bytes(8),
            channel_list: vec![11 + (f64::from(random_u8()) / 255.0 * 15.0).round() as u8],
            network_key: random_bytes(16),
            network_key_distribute: false,
            has_default_extended_pan_id: false,
        };
        logger().debug("commissioning random provisioning network:", NS);
        logger().debug(&format!(" - panId: {}", provisioning.pan_id), NS);
        logger().debug(
            &format!(" - extendedPanId: {}", hex(&provisioning.extended_pan_id)),
            NS,
        );
        logger().debug(
            &format!(" - channelList: {:?}", provisioning.channel_list),
            NS,
        );
        logger().debug(
            &format!(" - networkKey: {}", hex(&provisioning.network_key)),
            NS,
        );
        logger().debug(
            &format!(
                " - networkKeyDistribute: {}",
                provisioning.network_key_distribute
            ),
            NS,
        );
        self.begin_commissioning(provisioning, false, false).await?;
        self.backup.restore_backup(&backup).await?;
        let nwk = self.nwk_options.clone().expect("network options");
        self.update_commissioning_nv_items(&nwk).await?;
        logger().debug("giving adapter some time to settle", NS);
        wait(1000).await;
        self.reset_adapter().await?;
        self.begin_startup().await?;
        self.write_configuration_flag().await?;
        Ok(())
    }

    async fn begin_commissioning(
        &mut self,
        nwk_options: NetworkOptions,
        fail_on_collision: bool,
        write_configured_flag: bool,
    ) -> Result<(), String> {
        if nwk_options.pan_id == 65535 {
            return Err("network commissioning failed - cannot use pan id 65535".to_string());
        }
        self.backup.nv.delete_item(NvItemsIds::NIB).await?;
        self.clear_adapter().await?;
        self.update_commissioning_nv_items(&nwk_options).await?;
        logger().debug("beginning network commissioning", NS);
        if matches!(
            self.options.version,
            ZnpVersion::ZStack30x | ZnpVersion::ZStack3x0
        ) {
            let packed = pack_channel_list(&nwk_options.channel_list)?;
            self.znp_request(
                Subsystem::APP_CNF,
                "bdbSetChannel",
                json!({"isPrimary": 1, "channel": packed}),
                None,
                None,
                &[0],
            )
            .await?;
            self.znp_request(
                Subsystem::APP_CNF,
                "bdbSetChannel",
                json!({"isPrimary": 0, "channel": 0}),
                None,
                None,
                &[0],
            )
            .await?;
            let (_, receiver) = self
                .znp_wait_for(
                    Type::AREQ,
                    Subsystem::ZDO,
                    "stateChangeInd",
                    None,
                    None,
                    Some(9),
                    Some(100000),
                )
                .await;
            self.znp_request(
                Subsystem::APP_CNF,
                "bdbStartCommissioning",
                json!({"mode": 0x04}),
                None,
                None,
                &[0],
            )
            .await?;
            if receiver
                .await
                .map_err(|_| "Waiter dropped".to_string())?
                .is_err()
            {
                return Err("network commissioning timed out - most likely network with the same panId or extendedPanId already exists nearby".to_string());
            }
        } else {
            self.begin_startup().await?;
        }

        logger().debug("waiting for NIB to settle", NS);
        let mut reads = 0;
        let mut nib_value = None;
        loop {
            wait(3000).await;
            nib_value = self
                .backup
                .nv
                .read_item_struct(NvItemsIds::NIB, 0, nib)
                .await?;
            reads += 1;
            let settled = nib_value.as_ref().is_some_and(|nib| {
                nib.get_u16("nwkPanId") != Some(65535) && nib.get_u8("nwkLogicalChannel") != Some(0)
            });
            if settled || reads >= 10 {
                break;
            }
        }
        if !nib_value.as_ref().is_some_and(|nib| {
            nib.get_u16("nwkPanId") != Some(65535) && nib.get_u8("nwkLogicalChannel") != Some(0)
        }) {
            return Err(
                "network commissioning failed - timed out waiting for nib to settle".to_string(),
            );
        }

        let ext_nwk_info = self
            .znp_request_with_reply(Subsystem::ZDO, "extNwkInfo", json!({}))
            .await?;
        let panid = ext_nwk_info.payload.get("panid").and_then(Value::as_u64);
        if panid != Some(nwk_options.pan_id as u64) && fail_on_collision {
            return Err(format!(
                "network commissioning failed - panId collision detected (expected={}, actual={})",
                nwk_options.pan_id,
                panid.unwrap_or(0)
            ));
        }
        logger().debug("network commissioned", NS);
        if write_configured_flag {
            self.write_configuration_flag().await?;
        }
        Ok(())
    }

    async fn update_commissioning_nv_items(
        &mut self,
        options: &NetworkOptions,
    ) -> Result<(), String> {
        let mut pan = nwk_pan_id(None);
        pan.set_u16("panId", options.pan_id);
        let mut channels = channel_list(None);
        channels.set_u32("channelList", pack_channel_list(&options.channel_list)?);
        let mut extended_pan_id_reversed = options.extended_pan_id.clone();
        extended_pan_id_reversed.reverse();

        self.backup
            .nv
            .update_item(NvItemsIds::STARTUP_OPTION, &[0x00], true)
            .await?;
        self.backup
            .nv
            .update_item(
                NvItemsIds::LOGICAL_TYPE,
                &[zdo::device_logical_type::COORDINATOR as u8],
                true,
            )
            .await?;
        self.backup
            .nv
            .update_item(NvItemsIds::ZDO_DIRECT_CB, &[0x01], true)
            .await?;
        self.backup
            .nv
            .update_item(
                NvItemsIds::CHANLIST,
                &channels.serialize_memory(
                    crate::adapter::z_stack::structs::r#struct::Alignment::Unaligned,
                ),
                true,
            )
            .await?;
        self.backup
            .nv
            .update_item(
                NvItemsIds::PANID,
                &pan.serialize_memory(
                    crate::adapter::z_stack::structs::r#struct::Alignment::Unaligned,
                ),
                true,
            )
            .await?;
        self.backup
            .nv
            .update_item(NvItemsIds::EXTENDED_PAN_ID, &extended_pan_id_reversed, true)
            .await?;
        self.backup
            .nv
            .update_item(
                NvItemsIds::APS_USE_EXT_PANID,
                &extended_pan_id_reversed,
                true,
            )
            .await?;
        self.backup
            .nv
            .update_item(
                NvItemsIds::PRECFGKEYS_ENABLE,
                &[if options.network_key_distribute { 1 } else { 0 }],
                true,
            )
            .await?;

        if matches!(
            self.options.version,
            ZnpVersion::ZStack30x | ZnpVersion::ZStack3x0
        ) {
            self.backup
                .nv
                .update_item(NvItemsIds::PRECFGKEY, &options.network_key, true)
                .await?;
        } else {
            self.znp_request(
                    Subsystem::SAPI,
                    "writeConfiguration",
                    json!({"configid": NvItemsIds::PRECFGKEY, "len": options.network_key.len(), "value": options.network_key}),
                    None,
                    None,
                    &[0],
                )
                .await?;
            self.backup
                .nv
                .write_item(
                    NvItemsIds::LEGACY_TCLK_TABLE_START_12,
                    &[
                        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x5a, 0x69, 0x67, 0x42,
                        0x65, 0x65, 0x41, 0x6c, 0x6c, 0x69, 0x61, 0x6e, 0x63, 0x65, 0x30, 0x39,
                        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    ],
                    0,
                    true,
                )
                .await?;
        }
        Ok(())
    }

    async fn register_endpoints(&mut self) -> Result<(), String> {
        let cluster_id = ClusterId::ACTIVE_ENDPOINTS_REQUEST;
        let payload = BuffaloZdo::build_request(
            self.adapter.has_zdo_message_overhead(),
            cluster_id,
            &[json!(COORDINATOR_ADDRESS)],
        );
        let mut znp = self.backup.nv.znp.lock().await;
        let response = self
            .adapter
            .send_zdo(
                &mut znp,
                BLANK_EUI64,
                COORDINATOR_ADDRESS,
                cluster_id,
                &payload,
                false,
            )
            .await?;
        let status = response.get(0).and_then(Value::as_u64).unwrap_or(1);
        if status != 0 {
            return Err(format!("ZDO status error: {status}"));
        }
        let active_endpoints = response
            .get(1)
            .and_then(|payload| payload.get("endpointList"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for endpoint in endpoints() {
            if active_endpoints
                .iter()
                .any(|value| value.as_u64() == Some(endpoint.endpoint as u64))
            {
                logger().debug(
                    &format!("endpoint '{}' already registered", endpoint.endpoint),
                    NS,
                );
            } else {
                logger().debug(&format!("registering endpoint '{}'", endpoint.endpoint), NS);
                znp.request(
                    Subsystem::AF,
                    "register",
                    json!({
                        "endpoint": endpoint.endpoint,
                        "appprofid": endpoint.appprofid,
                        "appdeviceid": endpoint.appdeviceid,
                        "appdevver": endpoint.appdevver,
                        "latencyreq": endpoint.latencyreq,
                        "appnuminclusters": endpoint.appnuminclusters,
                        "appinclusterlist": endpoint.appinclusterlist,
                        "appnumoutclusters": endpoint.appnumoutclusters,
                        "appoutclusterlist": endpoint.appoutclusterlist,
                    }),
                    None,
                    None,
                    &[0],
                )
                .await?;
            }
        }
        Ok(())
    }

    async fn add_to_group(&mut self, endpoint: u8, group: u16) -> Result<(), String> {
        let result = self
            .znp_request(
                Subsystem::ZDO,
                "extFindGroup",
                json!({"endpoint": endpoint, "groupid": group}),
                None,
                None,
                &[
                    ZnpCommandStatus::SUCCESS as u8,
                    ZnpCommandStatus::FAILURE as u8,
                ],
            )
            .await?
            .ok_or_else(|| "extFindGroup has no reply".to_string())?;
        if result.payload.get("status").and_then(Value::as_u64)
            == Some(ZnpCommandStatus::FAILURE as u64)
        {
            self.znp_request(
                Subsystem::ZDO,
                "extAddGroup",
                json!({"endpoint": endpoint, "groupid": group, "namelen": 0, "groupname": []}),
                None,
                None,
                &[0],
            )
            .await?;
        }
        Ok(())
    }

    async fn reset_adapter(&mut self) -> Result<(), String> {
        logger().debug("adapter reset requested", NS);
        self.znp_request(
            Subsystem::SYS,
            "resetReq",
            json!({"type": sys::reset_type::SOFT}),
            None,
            None,
            &[0],
        )
        .await?;
        logger().debug("adapter reset successful", NS);
        Ok(())
    }

    async fn clear_adapter(&mut self) -> Result<(), String> {
        logger().debug("clearing adapter using startup option 3", NS);
        self.backup
            .nv
            .write_item(NvItemsIds::STARTUP_OPTION, &[0x03], 0, true)
            .await?;
        self.reset_adapter().await?;
        self.backup
            .nv
            .write_item(NvItemsIds::STARTUP_OPTION, &[0x00], 0, true)
            .await?;
        Ok(())
    }

    async fn parse_config_network_options(&mut self) -> Result<NetworkOptions, String> {
        let mut channel_list = self.options.network_options.channel_list.clone();
        channel_list.sort_unstable();
        let extended_pan_id = self
            .options
            .network_options
            .extended_pan_id
            .clone()
            .ok_or_else(|| "Missing extendedPanID".to_string())?;
        let network_key = self
            .options
            .network_options
            .network_key
            .clone()
            .ok_or_else(|| "Missing networkKey".to_string())?;
        let mut parsed = NetworkOptions {
            channel_list,
            pan_id: self.options.network_options.pan_id,
            extended_pan_id,
            network_key,
            network_key_distribute: self
                .options
                .network_options
                .network_key_distribute
                .unwrap_or(false),
            has_default_extended_pan_id: false,
        };
        if parsed.extended_pan_id == [0xdd; 8] {
            let response = self
                .znp_request_with_reply(Subsystem::SYS, "getExtAddr", json!({}))
                .await?;
            let extaddress = response
                .payload
                .get("extaddress")
                .and_then(Value::as_str)
                .unwrap_or_default();
            parsed.extended_pan_id = hex_to_bytes(extaddress.split("0x").nth(1).unwrap_or(""));
            parsed.has_default_extended_pan_id = true;
        }
        Ok(parsed)
    }

    async fn write_configuration_flag(&mut self) -> Result<(), String> {
        logger().debug("writing configuration flag to adapter NV memory", NS);
        let id = if self.options.version == ZnpVersion::ZStack12 {
            NvItemsIds::ZNP_HAS_CONFIGURED_ZSTACK1
        } else {
            NvItemsIds::ZNP_HAS_CONFIGURED_ZSTACK3
        };
        self.backup.nv.write_item(id, &[0x55], 0, true).await
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

fn random_bytes(length: usize) -> Vec<u8> {
    use std::io::Read;
    let mut bytes = vec![0u8; length];
    if let Ok(mut file) = std::fs::File::open("/dev/urandom") {
        let _ = file.read_exact(&mut bytes);
    }
    bytes
}

fn random_u8() -> u8 {
    random_bytes(1)[0]
}

fn random_u16() -> u16 {
    let bytes = random_bytes(2);
    u16::from_le_bytes([bytes[0], bytes[1]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::transport::{AdapterTransport, TransportOptions};
    use crate::adapter::z_stack::adapter::adapter_nv_memory::AdapterNvMemory;
    use crate::adapter::z_stack::adapter::z_stack_adapter::ZStackZdo;
    use crate::adapter::z_stack::structs::entries::nib::nib;
    use crate::adapter::z_stack::structs::r#struct::Alignment;
    use crate::adapter::z_stack::unpi::frame::Frame;
    use crate::adapter::z_stack::znp::zpi_object::ZpiObject;
    use std::collections::BTreeMap;
    use tokio::io::AsyncWriteExt;

    const NETWORK_KEY: [u8; 16] = [
        0x01, 0x03, 0x05, 0x07, 0x09, 0x0b, 0x0d, 0x0f, 0x00, 0x02, 0x04, 0x06, 0x08, 0x0a, 0x0c,
        0x0d,
    ];
    const EXTENDED_PAN_ID: [u8; 8] = [0x00, 0x12, 0x4b, 0x00, 0x24, 0xc2, 0xd4, 0x00];
    const PAN_ID: u16 = 0x1a62;
    const CHANNEL: u8 = 11;

    struct FakeCoordinator {
        server: tokio::task::JoinHandle<()>,
    }

    impl FakeCoordinator {
        fn spawn(items: BTreeMap<u16, Vec<u8>>) -> (AdapterTransport, Self) {
            let (reader, mut peer) = serial2_tokio::SerialPort::pair().expect("pair");
            let mut transport = AdapterTransport::new(TransportOptions::new("/dev/null"));
            transport.attach_serial(reader).expect("attach");
            let server = tokio::spawn(async move {
                let mut buffer = [0u8; 512];
                let mut pending = Vec::new();
                loop {
                    let read = match peer.read(&mut buffer).await {
                        Ok(0) => return,
                        Ok(read) => read,
                        Err(_) => return,
                    };
                    pending.extend_from_slice(&buffer[..read]);
                    loop {
                        if pending.len() < 5 {
                            break;
                        }
                        let length = pending[1] as usize;
                        let frame_length = 4 + length + 1;
                        if pending.len() < frame_length {
                            break;
                        }
                        let raw = pending[..frame_length].to_vec();
                        pending.drain(..frame_length);
                        let frame =
                            Frame::from_buffer(length as u8, 4 + length, &raw).expect("frame");
                        let object = ZpiObject::from_unpi_frame(&frame);
                        let payload = &object.payload;
                        let data = match object.command.name.as_str() {
                            "ping" => vec![0x01, 0x00],
                            "osalNvLength" => {
                                let id = payload["id"].as_u64().unwrap() as u16;
                                let length = items.get(&id).map(Vec::len).unwrap_or(0) as u16;
                                length.to_le_bytes().to_vec()
                            }
                            "osalNvReadExt" => {
                                let id = payload["id"].as_u64().unwrap() as u16;
                                let offset = payload["offset"].as_u64().unwrap() as usize;
                                let value = items.get(&id).cloned().unwrap_or_default();
                                let end = (offset + 240).min(value.len());
                                let chunk = &value[offset..end];
                                let mut data = vec![0x00, chunk.len() as u8];
                                data.extend_from_slice(chunk);
                                data
                            }
                            other => panic!("unexpected NV command '{other}'"),
                        };
                        let response =
                            Frame::new(Type::SRSP, frame.subsystem, frame.commandID, data, 0, 0);
                        peer.write_all(&response.to_buffer()).await.expect("write");
                    }
                }
            });
            (transport, Self { server })
        }
    }

    impl Drop for FakeCoordinator {
        fn drop(&mut self) {
            self.server.abort();
        }
    }

    fn nib_bytes(pan_id: u16, channel: u8, extended_pan_id: &[u8]) -> Vec<u8> {
        let mut value = nib(None);
        value.set_u16("nwkPanId", pan_id);
        value.set_u32("channelList", 1u32 << channel);
        value.set_u8("nwkLogicalChannel", channel);
        value.set_bytes("extendedPANID", extended_pan_id);
        value.set_u8("SecurityLevel", 5);
        value.set_u8("nwkUpdateId", 0);
        value.serialize(Alignment::Unaligned, true, 0)
    }

    fn items(
        has_configured: u8,
        pan_id: u16,
        channel: u8,
        extended_pan_id: &[u8],
    ) -> BTreeMap<u16, Vec<u8>> {
        let mut items = BTreeMap::new();
        items.insert(NvItemsIds::ZNP_HAS_CONFIGURED_ZSTACK3, vec![has_configured]);
        items.insert(NvItemsIds::NIB, nib_bytes(pan_id, channel, extended_pan_id));
        items.insert(NvItemsIds::PRECFGKEY, NETWORK_KEY.to_vec());
        let mut key_info = vec![0u8];
        key_info.extend_from_slice(&NETWORK_KEY);
        items.insert(NvItemsIds::NWK_ACTIVE_KEY_INFO, key_info.clone());
        items.insert(NvItemsIds::NWK_ALTERN_KEY_INFO, key_info);
        // 21 bytes => unaligned memory detection.
        let mut nwk_key = vec![0u8];
        nwk_key.extend_from_slice(&NETWORK_KEY);
        nwk_key.extend_from_slice(&[0u8; 4]);
        items.insert(NvItemsIds::NWKKEY, nwk_key);
        items
    }

    fn options(pan_id: u16, extended_pan_id: &[u8], channel: u8, path: &str) -> StartupOptions {
        StartupOptions {
            version: ZnpVersion::ZStack3x0,
            network_options: ConfigNetworkOptions {
                pan_id,
                extended_pan_id: Some(extended_pan_id.to_vec()),
                network_key: Some(NETWORK_KEY.to_vec()),
                channel_list: vec![channel],
                network_key_distribute: Some(false),
            },
            green_power_group: 242,
            backup_path: path.to_string(),
            adapter_options: AdapterOptions::default(),
        }
    }

    async fn manager_with(
        items: BTreeMap<u16, Vec<u8>>,
        options: StartupOptions,
    ) -> (ZnpAdapterManager<ZStackZdo>, FakeCoordinator) {
        let (transport, coordinator) = FakeCoordinator::spawn(items);
        let mut znp = Znp::new(transport);
        znp.open().await.expect("open fake coordinator");
        let mut nv = AdapterNvMemory::new(znp);
        nv.init().await.expect("NV init");
        let mut manager = ZnpAdapterManager::new(ZStackZdo::default(), nv, options);
        manager.nwk_options = Some(
            manager
                .parse_config_network_options()
                .await
                .expect("network options"),
        );
        (manager, coordinator)
    }

    #[tokio::test]
    async fn selects_startup_when_config_matches_adapter() {
        let (mut manager, _coordinator) = manager_with(
            items(0x55, PAN_ID, CHANNEL, &EXTENDED_PAN_ID),
            options(
                PAN_ID,
                &EXTENDED_PAN_ID,
                CHANNEL,
                "/tmp/opencode/does-not-exist.json",
            ),
        )
        .await;
        assert_eq!(
            manager.determine_strategy().await.expect("strategy"),
            StartupStrategy::Startup
        );
    }

    #[tokio::test]
    async fn selects_restore_backup_when_unconfigured_and_backup_matches() {
        let path = "/tmp/opencode/manager-strategy-restore.json";
        std::fs::write(
            path,
            serde_json::json!({
                "metadata": {"format": "zigpy/open-coordinator-backup", "version": 1},
                "coordinator_ieee": "00124b0024c2d400",
                "pan_id": "1a62",
                "extended_pan_id": "00124b0024c2d400",
                "nwk_update_id": 0,
                "security_level": 5,
                "channel": 11,
                "channel_mask": [11],
                "network_key": {
                    "key": "01030507090b0d0f00020406080a0c0d",
                    "sequence_number": 0,
                    "frame_counter": 1000
                },
                "devices": []
            })
            .to_string(),
        )
        .expect("write backup");
        let (mut manager, _coordinator) = manager_with(
            items(0x00, PAN_ID, CHANNEL, &EXTENDED_PAN_ID),
            options(PAN_ID, &EXTENDED_PAN_ID, CHANNEL, path),
        )
        .await;
        assert_eq!(
            manager.determine_strategy().await.expect("strategy"),
            StartupStrategy::RestoreBackup
        );
    }

    #[tokio::test]
    async fn selects_commissioning_when_config_mismatches_and_backup_missing() {
        let (mut manager, _coordinator) = manager_with(
            items(0x55, PAN_ID, CHANNEL, &EXTENDED_PAN_ID),
            options(
                0x1b63,
                &EXTENDED_PAN_ID,
                CHANNEL,
                "/tmp/opencode/does-not-exist.json",
            ),
        )
        .await;
        assert_eq!(
            manager.determine_strategy().await.expect("strategy"),
            StartupStrategy::StartCommissioning
        );
    }
}
