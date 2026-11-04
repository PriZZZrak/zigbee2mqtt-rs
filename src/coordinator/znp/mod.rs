pub mod commands;
pub mod frame;
pub mod strategy;
pub mod transaction;
pub mod transport;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};

use commands::*;
use frame::ZnpFrame;
use transport::{ZnpEvent, ZnpTransport};

use crate::config::Config;
use crate::coordinator::{CoordinatorEvent, CoordinatorHandle, CoordinatorInfo};
use crate::error::{Error, Result};
use crate::zigbee::IeeeAddr;
use strategy::{
    determine_strategy, load_backup, parse_has_configured, parse_key_descriptor, parse_nib,
    AdapterNvState, BackupNetwork, NetworkOptions, StartupStrategy, ZnpVersion,
};
use transaction::{ZclIncoming, ZclRequest, ZclWaiters};

pub struct ZnpCoordinator {
    transport: ZnpTransport,
    event_rx: mpsc::Receiver<ZnpEvent>,
}

impl ZnpCoordinator {
    pub fn open(port: &str, baud: u32) -> Result<Self> {
        let (transport, event_rx) = ZnpTransport::open(port, baud)?;
        Ok(Self {
            transport,
            event_rx,
        })
    }

    /// Full coordinator startup sequence.
    pub async fn start(mut self, cfg: &Config, config_path: &Path) -> Result<CoordinatorHandle> {
        self.reset().await?;
        // Mirror upstream ping-retry (zStackAdapter.ts start(): 3 attempts, SYS ping capabilities=1)
        self.ping().await?;
        let version = self.check_version().await?;
        // Unverified heuristic, not sourced from a TI spec we have on hand:
        // product_id 0 is assumed to mean Z-Stack Home 1.2 (CC2531) and >=1
        // Z-Stack 3.x (CC2652/CC1352). If this misdetects, BDB channel
        // configuration below is silently skipped or wrongly applied -- if
        // joins fail on a given adapter, check the "ZNP version:" log line
        // logged just above against that adapter's actual stack version.
        let znp_version = ZnpVersion::from_product_id(version.product_id);
        let is_zstack3 = znp_version == ZnpVersion::ZStack3;

        match self
            .determine_startup(znp_version, cfg, config_path)
            .await?
        {
            Some(decision) => {
                for warning in &decision.warnings {
                    warn!("ZNP startup: {warning}");
                }
                match decision.strategy {
                    StartupStrategy::Startup => {
                        info!("ZNP startup: resuming the existing network without rewriting NV");
                    }
                    StartupStrategy::RestoreBackup => {
                        return Err(Error::Znp(
                            "coordinator backup restore is not implemented; refusing to re-commission over an existing backup"
                                .into(),
                        ));
                    }
                    StartupStrategy::StartCommissioning => {
                        info!("ZNP startup: commissioning a new network");
                        self.write_nv_config(cfg).await?;
                        if is_zstack3 {
                            self.configure_channel_bdb(cfg).await?;
                        }
                    }
                }
            }
            None => {
                // Transitional fallback for firmware where the strategy probe is
                // unsupported. It is explicit in the log, unlike the previous
                // unconditional rewrite.
                warn!(
                    "ZNP startup: cannot determine adapter strategy from NV memory; using legacy NV write path"
                );
                self.write_nv_config(cfg).await?;
                if is_zstack3 {
                    self.configure_channel_bdb(cfg).await?;
                }
            }
        }
        self.register_endpoints().await?;
        self.start_network().await?;
        let device_info = self.get_device_info().await;
        info!("ZNP coordinator ready");

        let coord_info = CoordinatorInfo {
            ieee_addr: device_info.as_ref().map(|d| IeeeAddr(d.ieee_addr)),
            version: format!("{}.{}", version.major_rel, version.minor_rel),
            transport_rev: version.transport_rev,
        };

        let (coord_event_tx, coord_event_rx) = mpsc::channel::<CoordinatorEvent>(64);
        let transport_clone = self.transport.clone();
        let waiters = Arc::new(Mutex::new(ZclWaiters::default()));
        let pump_waiters = Arc::clone(&waiters);

        tokio::spawn(async move {
            event_pump(self.event_rx, coord_event_tx, pump_waiters).await;
        });

        Ok(CoordinatorHandle {
            inner: Arc::new(Mutex::new(ZnpHandle {
                transport: transport_clone,
                waiters,
            })),
            events: coord_event_rx,
            info: coord_info,
        })
    }

    // ── Initialisation steps ─────────────────────────────────────────────────

    async fn reset(&mut self) -> Result<()> {
        info!("Resetting ZNP coordinator (soft reset)…");
        self.transport.send(sys_reset_req(ResetType::Soft)).await?;

        // Wait for SYS_RESET_IND from the device
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            match tokio::time::timeout_at(deadline, self.event_rx.recv()).await {
                Ok(Some(ZnpEvent::ResetInd)) => {
                    info!("Received SYS_RESET_IND");
                    return Ok(());
                }
                Ok(Some(_)) => continue, // ignore other events during reset
                Ok(None) => return Err(Error::ChannelClosed),
                Err(_) => {
                    warn!("No SYS_RESET_IND received within timeout, continuing anyway");
                    return Ok(());
                }
            }
        }
    }

    async fn ping(&self) -> Result<()> {
        // Upstream tries 3 times; we try once then let the 10s timeout surface.
        // Failure here is non-fatal for backward compat with old firmware that
        // may not reply correctly, but we log it. Matches zStackAdapter.ts behaviour
        // of retrying ping before version fetch.
        for attempt in 0..3 {
            match self.transport.request(sys_ping()).await {
                Ok(_) => return Ok(()),
                Err(e) if attempt == 2 => return Err(e),
                Err(_) => tokio::time::sleep(Duration::from_millis(200)).await,
            }
        }
        Ok(())
    }

    async fn check_version(&self) -> Result<SysVersionRsp> {
        let rsp = self.transport.request(sys_version()).await?;
        match SysVersionRsp::parse(&rsp.data) {
            Some(v) => {
                info!(
                    "ZNP version: transport_rev={} product_id={} {}.{} hw_rev={}",
                    v.transport_rev, v.product_id, v.major_rel, v.minor_rel, v.hw_rev
                );
                Ok(v)
            }
            None => Err(Error::Znp("failed to parse SYS_VERSION response".into())),
        }
    }

    /// Read one NV item using OSAL_NV_LENGTH + chunked OSAL_NV_READ_EXT.
    async fn read_nv_item(&self, item_id: u16) -> Result<Option<Vec<u8>>> {
        let length_rsp = self.transport.request(sys_osal_nv_length(item_id)).await?;
        if length_rsp.data.len() < 2 {
            return Err(Error::Znp(format!(
                "OSAL_NV_LENGTH 0x{item_id:04X} returned {} bytes",
                length_rsp.data.len()
            )));
        }
        let length = u16::from_le_bytes([length_rsp.data[0], length_rsp.data[1]]) as usize;
        if length == 0 {
            return Ok(None);
        }
        let mut data = Vec::with_capacity(length);
        let mut offset: u16 = 0;
        while (offset as usize) < length {
            let rsp = self
                .transport
                .request(sys_osal_nv_read_ext(item_id, offset))
                .await?;
            if rsp.data.len() < 2 {
                return Err(Error::Znp(format!(
                    "OSAL_NV_READ_EXT 0x{item_id:04X} returned a short response"
                )));
            }
            let status = rsp.data[0];
            let chunk = rsp.data[1] as usize;
            if status != 0 {
                return Err(Error::Znp(format!(
                    "OSAL_NV_READ_EXT 0x{item_id:04X} failed with status {status}"
                )));
            }
            if chunk == 0 || rsp.data.len() < 2 + chunk {
                return Err(Error::Znp(format!(
                    "OSAL_NV_READ_EXT 0x{item_id:04X} returned an invalid chunk"
                )));
            }
            data.extend_from_slice(&rsp.data[2..2 + chunk]);
            offset = offset.saturating_add(chunk as u16);
        }
        data.truncate(length);
        Ok(Some(data))
    }

    async fn coordinator_extended_pan_id(&self, cfg: &Config) -> Option<[u8; 8]> {
        if cfg.advanced.ext_pan_id != [0xdd; 8] {
            return None;
        }
        let response = self.transport.request(sys_get_ext_addr()).await.ok()?;
        if response.data.len() < 8 {
            return None;
        }
        let mut address = [0u8; 8];
        address.copy_from_slice(&response.data[..8]);
        address.reverse();
        Some(address)
    }

    /// Read adapter state and return the source-backed startup decision.
    ///
    /// `Ok(None)` means the firmware did not answer the NV probe; the caller
    /// then uses the explicitly logged legacy path.
    async fn determine_startup(
        &self,
        version: ZnpVersion,
        cfg: &Config,
        config_path: &Path,
    ) -> Result<Option<strategy::Decision>> {
        let configured_id = if version == ZnpVersion::ZStack12 {
            nv::ZNP_HAS_CONFIGURED_ZSTACK1
        } else {
            nv::ZNP_HAS_CONFIGURED_ZSTACK3
        };
        let configured = match self.read_nv_item(configured_id).await {
            Ok(value) => value.map(|bytes| parse_has_configured(&bytes)),
            Err(error) => {
                info!("ZNP startup: NV probe unavailable ({error})");
                return Ok(None);
            }
        };
        let nib_bytes = self.read_nv_item(nv::NIB).await?;
        let nib = match nib_bytes {
            Some(bytes) => match parse_nib(&bytes) {
                Some(nib) if bytes.len() == 112 => Some(nib),
                _ => {
                    // Aligned-memory targets or future layouts are not parsed;
                    // do not claim a decision from misunderstood bytes.
                    info!(
                        "ZNP startup: unsupported NIB length {}; falling back",
                        bytes.len()
                    );
                    return Ok(None);
                }
            },
            None => None,
        };
        let read_key = |id: u16| async move {
            match self.read_nv_item(id).await? {
                Some(bytes) => parse_key_descriptor(&bytes)
                    .map(Some)
                    .ok_or_else(|| Error::Znp(format!("short key descriptor for NV 0x{id:04X}"))),
                None => Ok(None),
            }
        };
        let preconfigured_key = read_key(nv::PRECFGKEY).await?;
        let (active_key, alternate_key) = if version == ZnpVersion::ZStack12 {
            // Z-Stack 1.2 has no key-info entries; upstream uses the
            // preconfigured key as both active and alternate.
            (preconfigured_key, preconfigured_key)
        } else {
            (
                read_key(nv::NWK_ACTIVE_KEY_INFO).await?,
                read_key(nv::NWK_ALTERN_KEY_INFO).await?,
            )
        };

        let default_ext = self.coordinator_extended_pan_id(cfg).await;
        let has_default_extended_pan_id = default_ext.is_some();
        let extended_pan_id = default_ext.unwrap_or(cfg.advanced.ext_pan_id);
        let config_network = NetworkOptions {
            pan_id: cfg.advanced.pan_id,
            extended_pan_id,
            channel_mask: 1 << cfg.advanced.channel,
            network_key: cfg.advanced.network_key,
            network_key_distribute: false,
            has_default_extended_pan_id,
        };
        let adapter = AdapterNvState {
            has_configured: configured.unwrap_or(false),
            nib,
            preconfigured_key,
            active_key,
            alternate_key,
        };

        let backup_path = config_path
            .parent()
            .unwrap_or(Path::new("."))
            .join("coordinator_backup.json");
        let backup: Option<BackupNetwork> = load_backup(&backup_path)?;
        let decision =
            determine_strategy(version, &config_network, &adapter, backup.as_ref(), false)?;
        info!(
            "ZNP startup strategy: {:?} (configured={}, nib={}, backup={})",
            decision.strategy,
            adapter.has_configured,
            adapter.nib.is_some(),
            backup.is_some()
        );
        Ok(Some(decision))
    }

    /// Write network configuration to NVRAM before starting the network.
    async fn write_nv_config(&self, cfg: &Config) -> Result<()> {
        info!("Writing network configuration to NVRAM");

        // STARTUP_OPTION = 0x00 (no special startup – mirrors manager.ts updateCommissioningNvItems)
        self.nv_write(nv::STARTUP_OPTION, &[0x00]).await?;

        // Logical type: coordinator = 0x00
        self.nv_write(nv::LOGICAL_TYPE, &[0x00]).await?;

        // PAN ID
        self.nv_write(nv::PANID, &cfg.advanced.pan_id.to_le_bytes())
            .await?;

        // Extended PAN ID – stored reversed on-wire in some NV items, but
        // ZNP OSAL NV write expects little-endian as configured; we store as-is
        // to match current Rust database convention (compatibility preserved).
        self.nv_write(nv::EXTENDED_PAN_ID, &cfg.advanced.ext_pan_id)
            .await?;
        // APS_USE_EXT_PANID must mirror EXTENDED_PAN_ID (manager.ts)
        self.nv_write(nv::APS_USE_EXT_PANID, &cfg.advanced.ext_pan_id)
            .await?;

        // Channel list (bitmask)
        let channel_mask: u32 = 1 << cfg.advanced.channel;
        self.nv_write(nv::CHANLIST, &channel_mask.to_le_bytes())
            .await?;

        // Network key
        self.nv_write(nv::PRECFGKEY, &cfg.advanced.network_key)
            .await?;

        // Enable pre-configured key distribution
        self.nv_write(nv::PRECFGKEYS_ENABLE, &[0x01]).await?;

        // Enable ZDO direct callbacks (so we receive device announcements etc.)
        self.nv_write(nv::ZDO_DIRECT_CB, &[0x01]).await?;

        Ok(())
    }

    async fn nv_write(&self, item_id: u16, data: &[u8]) -> Result<()> {
        let rsp = self
            .transport
            .request(sys_osal_nv_write(item_id, data))
            .await?;
        if rsp.data.first().copied() != Some(0) {
            warn!(
                "NV write 0x{item_id:04X} returned non-zero status {:?}",
                rsp.data.first()
            );
        }
        Ok(())
    }

    async fn configure_channel_bdb(&self, cfg: &Config) -> Result<()> {
        let channel = cfg.advanced.channel;
        let channel_mask: u32 = 1 << channel;

        // Set primary channel via BDB (Z-Stack 3.0+)
        let rsp = self
            .transport
            .request(app_cnf_bdb_set_channel(channel_mask, true))
            .await?;
        if rsp.data.first().copied() != Some(0) {
            warn!(
                "BDB set primary channel returned non-zero status: {:?}",
                rsp.data
            );
        }

        // Clear secondary channel
        let rsp = self
            .transport
            .request(app_cnf_bdb_set_channel(0, false))
            .await?;
        if rsp.data.first().copied() != Some(0) {
            warn!(
                "BDB set secondary channel returned non-zero status: {:?}",
                rsp.data
            );
        }

        info!("Zigbee channel set to {channel}");
        Ok(())
    }

    async fn register_endpoints(&self) -> Result<()> {
        // Register endpoint 1 (HA profile) – receives ZCL cluster traffic
        let input_clusters: Vec<u16> = vec![
            0x0000, 0x0001, 0x0005, 0x0006, 0x0008, 0x0300, 0x0400, 0x0402, 0x0405, 0x0406, 0x0500,
            0x0B04,
        ];
        let output_clusters: Vec<u16> = vec![0x0006, 0x0008, 0x0300];

        let rsp = self
            .transport
            .request(af_register(
                1,
                0x0104,
                0x0005,
                &input_clusters,
                &output_clusters,
            ))
            .await?;
        if rsp.data.first().copied() != Some(0) {
            warn!("AF_REGISTER returned non-zero: {:?}", rsp.data);
        }
        Ok(())
    }

    async fn start_network(&self) -> Result<()> {
        let rsp = self.transport.request(zdo_startup_from_app(100)).await?;
        match rsp.data.first().copied() {
            Some(0) => info!("ZDO startup: new network formed"),
            Some(1) => info!("ZDO startup: rejoined existing network"),
            other => warn!("ZDO startup returned status {:?}", other),
        }
        // Allow the network to stabilize
        tokio::time::sleep(Duration::from_secs(1)).await;
        Ok(())
    }

    async fn get_device_info(&self) -> Option<DeviceInfo> {
        match self.transport.request(util_get_device_info()).await {
            Ok(rsp) => DeviceInfo::parse(&rsp.data),
            Err(e) => {
                warn!("Failed to get device info: {e}");
                None
            }
        }
    }
}

// ── Event pump (AREQ → CoordinatorEvent) ─────────────────────────────────────

async fn event_pump(
    mut znp_rx: mpsc::Receiver<ZnpEvent>,
    out: mpsc::Sender<CoordinatorEvent>,
    waiters: Arc<Mutex<ZclWaiters>>,
) {
    while let Some(ev) = znp_rx.recv().await {
        let coord_event = match ev {
            ZnpEvent::EndDeviceAnnceInd(data) => {
                EndDeviceAnnceInd::parse(&data).map(|d| CoordinatorEvent::DeviceJoined {
                    ieee_addr: IeeeAddr(d.ieee_addr),
                    nwk_addr: d.nwk_addr,
                })
            }
            ZnpEvent::LeaveInd(data) => {
                LeaveInd::parse(&data).map(|d| CoordinatorEvent::DeviceLeft {
                    ieee_addr: IeeeAddr(d.ieee_addr),
                })
            }
            ZnpEvent::AfDataConfirm(data) => {
                if let Some(confirm) = AfDataConfirm::parse(&data) {
                    waiters.lock().await.confirm(&confirm);
                }
                None
            }
            ZnpEvent::AfIncomingMsg(data) => {
                if let Some(incoming) = ZclIncoming::parse_af(&data) {
                    waiters.lock().await.deliver(&incoming);
                }
                AfIncomingMsg::parse(&data).map(|m| CoordinatorEvent::Message {
                    src_addr: m.src_addr,
                    src_ep: m.src_ep,
                    cluster_id: m.cluster_id,
                    link_quality: m.link_quality,
                    data: m.data,
                })
            }
            ZnpEvent::ActiveEpRsp(data) => {
                ActiveEpRsp::parse(&data).map(|r| CoordinatorEvent::ActiveEpRsp {
                    nwk_addr: r.nwk_addr,
                    endpoints: r.endpoints,
                })
            }
            ZnpEvent::SimpleDescRsp(data) => {
                SimpleDescRsp::parse(&data).map(|r| CoordinatorEvent::SimpleDescRsp {
                    nwk_addr: r.nwk_addr,
                    endpoint: r.endpoint,
                    profile_id: r.profile_id,
                    device_id: r.device_id,
                    input_clusters: r.input_clusters,
                    output_clusters: r.output_clusters,
                })
            }
            ZnpEvent::IeeeAddrRsp(data) => {
                IeeeAddrRsp::parse(&data).map(|r| CoordinatorEvent::AddressResolved {
                    ieee_addr: IeeeAddr(r.ieee_addr),
                    nwk_addr: r.nwk_addr,
                })
            }
            ZnpEvent::NodeDescRsp(data) => {
                NodeDescRsp::parse(&data).map(|r| CoordinatorEvent::NodeDescRsp {
                    nwk_addr: r.nwk_addr,
                    manufacturer_code: r.manufacturer_code,
                    logical_type: r.logical_type,
                })
            }
            ZnpEvent::BindRsp(data) => BindRsp::parse(&data).map(|r| CoordinatorEvent::BindRsp {
                nwk_addr: r.nwk_addr,
                status: r.status,
            }),
            ZnpEvent::TcDevInd(data) => {
                // TC_DEV_IND: SrcAddr(u16) + IEEEAddr(8) + ParentAddr(u16)
                if data.len() >= 10 {
                    let nwk_addr = u16::from_le_bytes([data[0], data[1]]);
                    let mut ieee = [0u8; 8];
                    ieee.copy_from_slice(&data[2..10]);
                    Some(CoordinatorEvent::AddressResolved {
                        ieee_addr: IeeeAddr(ieee),
                        nwk_addr,
                    })
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(e) = coord_event {
            if out.send(e).await.is_err() {
                break;
            }
        }
    }
}

// ── ZnpHandle (send-side) ─────────────────────────────────────────────────────

/// Transport-independent part of `request_zcl` (tested with a fake sender).
async fn request_zcl_flow<F, Fut>(
    waiters: &Mutex<ZclWaiters>,
    request: ZclRequest,
    timeout: Duration,
    attempts: u8,
    mut send: F,
) -> Result<ZclIncoming>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<ZnpFrame>>,
{
    let attempts = attempts.max(1);
    let mut last_error: Option<Error> = None;
    for attempt in 0..attempts {
        let (id, mut reply) = {
            let mut waiters = waiters.lock().await;
            waiters.register(request.clone(), timeout)
        };
        let rsp = match send().await {
            Ok(rsp) => rsp,
            Err(error) => {
                waiters.lock().await.remove(id);
                return Err(error);
            }
        };
        if rsp.data.first().copied() != Some(0) {
            waiters.lock().await.remove(id);
            return Err(Error::Znp(format!(
                "AF_DATA_REQUEST rejected with status {:?}",
                rsp.data.first()
            )));
        }
        match tokio::time::timeout(timeout, &mut reply).await {
            Ok(Ok(Ok(incoming))) => return Ok(incoming),
            Ok(Ok(Err(error))) => last_error = Some(error),
            Ok(Err(_)) => last_error = Some(Error::ChannelClosed),
            Err(_) => {
                let removed = waiters.lock().await.remove(id);
                if !removed {
                    match reply.await {
                        Ok(Ok(incoming)) => return Ok(incoming),
                        Ok(Err(error)) => last_error = Some(error),
                        Err(_) => last_error = Some(Error::ChannelClosed),
                    }
                } else {
                    last_error = Some(Error::Timeout);
                }
            }
        }
        if attempt + 1 < attempts {
            warn!(
                "ZCL request 0x{:04X}:{} cluster 0x{:04X} TSN {} attempt {}/{} failed, retrying",
                request.nwk_addr,
                request.endpoint,
                request.cluster_id,
                request.trans_seq,
                attempt + 1,
                attempts
            );
        }
    }
    Err(last_error.unwrap_or(Error::Timeout))
}

pub struct ZnpHandle {
    transport: ZnpTransport,
    waiters: Arc<Mutex<ZclWaiters>>,
}

impl ZnpHandle {
    pub async fn permit_join(&self, duration: u8) -> Result<()> {
        let rsp = self
            .transport
            .request(zdo_permit_join(0xFFFC, duration))
            .await?;
        if rsp.data.first().copied() != Some(0) {
            warn!("PERMIT_JOIN rsp: {:?}", rsp.data);
        }
        Ok(())
    }

    pub async fn request_node_desc(&self, nwk_addr: u16) -> Result<()> {
        self.transport
            .request(zdo_node_desc_req(nwk_addr, nwk_addr))
            .await?;
        Ok(())
    }

    pub async fn request_active_eps(&self, nwk_addr: u16) -> Result<()> {
        self.transport
            .request(zdo_active_ep_req(nwk_addr, nwk_addr))
            .await?;
        Ok(())
    }

    pub async fn request_simple_desc(&self, nwk_addr: u16, endpoint: u8) -> Result<()> {
        self.transport
            .request(zdo_simple_desc_req(nwk_addr, nwk_addr, endpoint))
            .await?;
        Ok(())
    }

    pub async fn request_ieee_addr(&self, nwk_addr: u16) -> Result<()> {
        self.transport.request(zdo_ieee_addr_req(nwk_addr)).await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn request_bind(
        &self,
        dst_addr: u16,
        src_ieee: [u8; 8],
        src_ep: u8,
        cluster_id: u16,
        dst_ieee: [u8; 8],
        dst_ep: u8,
    ) -> Result<()> {
        self.transport
            .request(zdo_bind_req(
                dst_addr, src_ieee, src_ep, cluster_id, dst_ieee, dst_ep,
            ))
            .await?;
        Ok(())
    }

    pub async fn send_zcl(
        &self,
        dst_addr: u16,
        dst_ep: u8,
        cluster_id: u16,
        trans_id: u8,
        payload: Vec<u8>,
    ) -> Result<()> {
        let rsp = self
            .transport
            .request(af_data_request(
                dst_addr, dst_ep, 1, cluster_id, trans_id, payload,
            ))
            .await?;
        if rsp.data.first().copied() != Some(0) {
            warn!("AF_DATA_REQUEST status: {:?}", rsp.data);
        }
        Ok(())
    }

    /// Send a ZCL frame and wait for the correlated device response.
    ///
    /// AF_DATA_REQUEST acceptance is not delivery and AF_DATA_CONFIRM success is
    /// not a ZCL response, so this waits for the actual response matched by
    /// address/endpoint/cluster/TSN/command. A failed AF confirmation, timeout
    /// or closed reply channel is returned as an error; timeouts are retried up
    /// to `attempts` with the same TSN.
    #[allow(clippy::too_many_arguments)]
    pub async fn request_zcl(
        &self,
        dst_addr: u16,
        dst_ep: u8,
        cluster_id: u16,
        trans_id: u8,
        payload: Vec<u8>,
        timeout: Duration,
        attempts: u8,
    ) -> Result<ZclIncoming> {
        self.request_zcl_expected(
            dst_addr, dst_ep, cluster_id, trans_id, payload, timeout, attempts, None,
        )
        .await
    }

    /// Like `request_zcl`, but for cluster-specific commands with a known
    /// response command id (for example Groups addGroup response 0x00).
    #[allow(clippy::too_many_arguments)]
    pub async fn request_zcl_expected(
        &self,
        dst_addr: u16,
        dst_ep: u8,
        cluster_id: u16,
        trans_id: u8,
        payload: Vec<u8>,
        timeout: Duration,
        attempts: u8,
        expected_response: Option<u8>,
    ) -> Result<ZclIncoming> {
        let request = match expected_response {
            Some(expected) => ZclRequest::from_payload_with_response(
                dst_addr,
                dst_ep,
                cluster_id,
                &payload,
                Some(expected),
            ),
            None => ZclRequest::from_payload(dst_addr, dst_ep, cluster_id, &payload),
        }
        .ok_or_else(|| Error::Zcl("cannot parse outgoing ZCL header".into()))?;
        if !request.expects_response() {
            return Err(Error::Zcl(
                "ZCL command does not expect a response; use send_zcl".into(),
            ));
        }
        request_zcl_flow(&self.waiters, request, timeout, attempts, || {
            self.transport.request(af_data_request(
                dst_addr,
                dst_ep,
                1,
                cluster_id,
                trans_id,
                payload.clone(),
            ))
        })
        .await
    }
}

#[cfg(test)]
mod transaction_integration_tests {
    use super::*;

    fn af_frame(payload: Vec<u8>) -> Vec<u8> {
        let mut raw = vec![0u8; 17];
        raw[2..4].copy_from_slice(&0x0402u16.to_le_bytes());
        raw[4..6].copy_from_slice(&0x1234u16.to_le_bytes());
        raw[6] = 1;
        raw[7] = 1;
        raw[9] = 200;
        raw[15] = 9;
        raw[16] = payload.len() as u8;
        raw.extend(payload);
        raw
    }

    fn read_request() -> ZclRequest {
        // global frame, client->server, TSN 7, read attributes
        ZclRequest::from_payload(0x1234, 1, 0x0402, &[0x00, 7, 0x00, 0x00, 0x00]).unwrap()
    }

    #[tokio::test]
    async fn event_pump_resolves_zcl_waiter_from_af_incoming() {
        let (znp_tx, znp_rx) = mpsc::channel(4);
        let (out_tx, _out_rx) = mpsc::channel(4);
        let waiters = Arc::new(Mutex::new(ZclWaiters::default()));
        let (_, reply) = {
            let mut waiters = waiters.lock().await;
            waiters.register(read_request(), Duration::from_secs(1))
        };
        tokio::spawn(event_pump(znp_rx, out_tx, Arc::clone(&waiters)));

        // readRsp: global, server->client, TSN 7, command 0x01
        znp_tx
            .send(ZnpEvent::AfIncomingMsg(af_frame(vec![
                0x18, 7, 0x01, 0x00, 0x00, 0x00, 0x20, 0x01,
            ])))
            .await
            .unwrap();

        let incoming = reply.await.unwrap().unwrap();
        assert_eq!(incoming.command, 0x01);
        assert_eq!(incoming.nwk_addr, 0x1234);
        assert_eq!(incoming.cluster_id, 0x0402);
        assert_eq!(waiters.lock().await.pending(), 0);
    }

    #[tokio::test]
    async fn event_pump_propagates_failed_af_confirm() {
        let (znp_tx, znp_rx) = mpsc::channel(4);
        let (out_tx, _out_rx) = mpsc::channel(4);
        let waiters = Arc::new(Mutex::new(ZclWaiters::default()));
        let (_, reply) = {
            let mut waiters = waiters.lock().await;
            waiters.register(read_request(), Duration::from_secs(1))
        };
        tokio::spawn(event_pump(znp_rx, out_tx, Arc::clone(&waiters)));

        znp_tx
            .send(ZnpEvent::AfDataConfirm(vec![0xE9, 1, 7]))
            .await
            .unwrap();

        let error = reply.await.unwrap().unwrap_err();
        assert!(error.to_string().contains("0xE9"));
        assert_eq!(waiters.lock().await.pending(), 0);
    }

    #[tokio::test]
    async fn successful_af_confirm_does_not_complete_the_command() {
        let (znp_tx, znp_rx) = mpsc::channel(4);
        let (out_tx, _out_rx) = mpsc::channel(4);
        let waiters = Arc::new(Mutex::new(ZclWaiters::default()));
        let (_, mut reply) = {
            let mut waiters = waiters.lock().await;
            waiters.register(read_request(), Duration::from_secs(1))
        };
        tokio::spawn(event_pump(znp_rx, out_tx, Arc::clone(&waiters)));

        znp_tx
            .send(ZnpEvent::AfDataConfirm(vec![0x00, 1, 7]))
            .await
            .unwrap();
        assert!(
            reply.try_recv().is_err(),
            "success confirm must not resolve"
        );
        assert_eq!(waiters.lock().await.pending(), 1);
    }
}

#[cfg(test)]
mod request_flow_tests {
    use super::*;

    fn read_request() -> ZclRequest {
        ZclRequest::from_payload(0x1234, 1, 0x0402, &[0x00, 7, 0x00, 0x00, 0x00]).unwrap()
    }

    fn success_frame() -> ZnpFrame {
        ZnpFrame::new(
            frame::FrameType::SRsp,
            frame::Subsystem::Af,
            0x00,
            vec![0x00],
        )
    }

    fn response() -> ZclIncoming {
        ZclIncoming {
            nwk_addr: 0x1234,
            endpoint: 1,
            cluster_id: 0x0402,
            trans_seq: 7,
            frame_type: 0x00,
            direction: 1,
            command: 0x01,
            data: vec![0x18, 7, 0x01, 0x00, 0x00, 0x00, 0x20, 0x01],
        }
    }

    #[tokio::test]
    async fn fake_transport_delivers_response_on_first_attempt() {
        let waiters = Arc::new(Mutex::new(ZclWaiters::default()));
        let mut calls = 0;
        let result = request_zcl_flow(&waiters, read_request(), Duration::from_secs(1), 3, || {
            calls += 1;
            let waiters = Arc::clone(&waiters);
            let response = response();
            async move {
                waiters.lock().await.deliver(&response);
                Ok(success_frame())
            }
        })
        .await
        .unwrap();
        assert_eq!(result.command, 0x01);
        assert_eq!(calls, 1);
        assert_eq!(waiters.lock().await.pending(), 0);
    }

    #[tokio::test]
    async fn fake_transport_times_out_and_retries_then_errors() {
        let waiters = Arc::new(Mutex::new(ZclWaiters::default()));
        let mut calls = 0;
        let result = request_zcl_flow(
            &waiters,
            read_request(),
            Duration::from_millis(1),
            2,
            || {
                calls += 1;
                async { Ok(success_frame()) }
            },
        )
        .await;
        assert!(matches!(result, Err(Error::Timeout)));
        assert_eq!(calls, 2);
        assert_eq!(waiters.lock().await.pending(), 0);
    }

    #[tokio::test]
    async fn fake_transport_propagates_af_request_rejection() {
        let waiters = Arc::new(Mutex::new(ZclWaiters::default()));
        let mut calls = 0;
        let result = request_zcl_flow(&waiters, read_request(), Duration::from_secs(1), 3, || {
            calls += 1;
            async {
                Ok(ZnpFrame::new(
                    frame::FrameType::SRsp,
                    frame::Subsystem::Af,
                    0x00,
                    vec![0x01],
                ))
            }
        })
        .await;
        assert!(result.unwrap_err().to_string().contains("rejected"));
        assert_eq!(calls, 1);
        assert_eq!(waiters.lock().await.pending(), 0);
    }

    #[tokio::test]
    async fn fake_transport_propagates_af_confirm_failure() {
        let waiters = Arc::new(Mutex::new(ZclWaiters::default()));
        let result = request_zcl_flow(&waiters, read_request(), Duration::from_secs(1), 3, || {
            let waiters = Arc::clone(&waiters);
            async move {
                waiters.lock().await.confirm(&AfDataConfirm {
                    status: 0xE9,
                    endpoint: 1,
                    trans_seq: 7,
                });
                Ok(success_frame())
            }
        })
        .await;
        assert!(result.unwrap_err().to_string().contains("0xE9"));
        assert_eq!(waiters.lock().await.pending(), 0);
    }
}

#[cfg(test)]
mod fake_serial_e2e_tests {
    use super::*;
    use futures::SinkExt;
    use tokio::io::DuplexStream;
    use tokio_stream::StreamExt;
    use tokio_util::codec::Framed;

    fn af_incoming(cluster: u16, src: u16, src_ep: u8, zcl: Vec<u8>) -> Vec<u8> {
        let mut raw = vec![0u8; 17];
        raw[2..4].copy_from_slice(&cluster.to_le_bytes());
        raw[4..6].copy_from_slice(&src.to_le_bytes());
        raw[6] = src_ep;
        raw[7] = 1;
        raw[9] = 200;
        raw[15] = 9;
        raw[16] = zcl.len() as u8;
        raw.extend(zcl);
        raw
    }

    fn fake_handle() -> (Framed<DuplexStream, frame::ZnpCodec>, ZnpHandle) {
        let (client, server) = tokio::io::duplex(4096);
        let (transport, events) =
            transport::spawn_test_transport(Framed::new(server, frame::ZnpCodec));
        let waiters = Arc::new(Mutex::new(transaction::ZclWaiters::default()));
        let pump_waiters = Arc::clone(&waiters);
        let (event_tx, _event_rx) = mpsc::channel(16);
        tokio::spawn(event_pump(events, event_tx, pump_waiters));
        (
            Framed::new(client, frame::ZnpCodec),
            ZnpHandle { transport, waiters },
        )
    }

    async fn spawn_fake_device<F>(
        mut wire: Framed<DuplexStream, frame::ZnpCodec>,
        response: F,
    ) -> tokio::task::JoinHandle<()>
    where
        F: FnOnce(u8) -> Vec<u8> + Send + 'static,
    {
        tokio::spawn(async move {
            let request = wire.next().await.unwrap().unwrap();
            assert_eq!(request.frame_type, frame::FrameType::SReq);
            assert_eq!(request.subsystem, frame::Subsystem::Af);
            let tsn = request.data[6];
            wire.send(frame::ZnpFrame::new(
                frame::FrameType::SRsp,
                frame::Subsystem::Af,
                0x01,
                vec![0x00],
            ))
            .await
            .unwrap();
            wire.send(frame::ZnpFrame::new(
                frame::FrameType::AReq,
                frame::Subsystem::Af,
                0x81,
                af_incoming(0x0402, 0x1234, 1, response(tsn)),
            ))
            .await
            .unwrap();
        })
    }

    #[tokio::test]
    async fn fake_serial_get_reads_temperature_response() {
        let (wire, handle) = fake_handle();
        let device = spawn_fake_device(wire, |tsn| {
            // global readRsp: attr 0x0000, status 0, int16 2250 = 22.50 C
            vec![0x18, tsn, 0x01, 0x00, 0x00, 0x00, 0x29, 0xCA, 0x08]
        })
        .await;
        let request = crate::zigbee::zcl::frame::read_attributes_payload(7, &[0x0000]);
        let response = handle
            .request_zcl(0x1234, 1, 0x0402, 7, request, Duration::from_secs(1), 1)
            .await
            .expect("get response over fake serial");
        assert_eq!(response.command, 0x01);
        let parsed = crate::zigbee::zcl::parse_message(0x0402, &response.data, None, None)
            .unwrap()
            .expect("temperature values");
        assert_eq!(parsed.values["temperature"], serde_json::json!(22.5));
        device.await.unwrap();
    }

    #[tokio::test]
    async fn fake_serial_reporting_read_returns_configuration() {
        let (wire, handle) = fake_handle();
        let device = spawn_fake_device(wire, |tsn| {
            // read reporting config rsp: attr 0, status 0, u16, min 10, max 300, change 50
            vec![
                0x18, tsn, 0x09, 0x00, 0x00, 0x00, 0x00, 0x21, 0x0A, 0x00, 0x2C, 0x01, 0x32, 0x00,
            ]
        })
        .await;
        let request = crate::zigbee::zcl::frame::read_reporting_config_payload(7, &[0x0000]);
        let response = handle
            .request_zcl(0x1234, 1, 0x0402, 7, request, Duration::from_secs(1), 1)
            .await
            .expect("reporting response over fake serial");
        assert_eq!(response.command, 0x09);
        let (_, offset) = crate::zigbee::zcl::frame::ZclFrameHeader::parse(&response.data).unwrap();
        let configs = crate::zigbee::zcl::parse_reporting_config_rsp(&response.data[offset..]);
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].minimum_interval, Some(10));
        assert_eq!(configs[0].maximum_interval, Some(300));
        device.await.unwrap();
    }
}
