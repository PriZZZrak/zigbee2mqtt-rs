pub mod znp;

use std::path::Path;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use crate::config::{AdapterType, Config};
use crate::error::Result;
use crate::zigbee::IeeeAddr;
use znp::transaction::ZclIncoming;
use znp::{ZnpCoordinator, ZnpHandle};

#[derive(Debug, Clone)]
pub enum CoordinatorEvent {
    DeviceJoined {
        ieee_addr: IeeeAddr,
        nwk_addr: u16,
    },
    DeviceLeft {
        ieee_addr: IeeeAddr,
    },
    /// IEEE↔NWK address resolved (from ZDO_IEEE_ADDR_RSP or TC_DEV_IND).
    AddressResolved {
        ieee_addr: IeeeAddr,
        nwk_addr: u16,
    },
    Message {
        src_addr: u16,
        src_ep: u8,
        cluster_id: u16,
        link_quality: u8,
        data: Vec<u8>,
    },
    ActiveEpRsp {
        nwk_addr: u16,
        endpoints: Vec<u8>,
    },
    SimpleDescRsp {
        nwk_addr: u16,
        endpoint: u8,
        profile_id: u16,
        device_id: u16,
        input_clusters: Vec<u16>,
        output_clusters: Vec<u16>,
    },
    /// Result of a ZDO_BIND_REQ (see `CoordinatorHandle::request_bind`).
    BindRsp {
        nwk_addr: u16,
        status: u8,
    },
    NodeDescRsp {
        nwk_addr: u16,
        manufacturer_code: u16,
        logical_type: u8,
    },
}

#[derive(Debug, Clone)]
pub struct CoordinatorInfo {
    pub ieee_addr: Option<IeeeAddr>,
    pub version: String,
    pub transport_rev: u8,
    /// Coordinator network parameters (`ZDO_EXT_NWK_INFO`). `None` when the
    /// firmware does not answer the request.
    pub channel: Option<u8>,
    pub pan_id: Option<u16>,
    pub extended_pan_id: Option<String>,
}

pub struct CoordinatorHandle {
    pub inner: Arc<Mutex<ZnpHandle>>,
    pub events: mpsc::Receiver<CoordinatorEvent>,
    pub info: CoordinatorInfo,
}

impl CoordinatorHandle {
    pub async fn permit_join(&self, duration: u8) -> Result<()> {
        self.inner.lock().await.permit_join(duration).await
    }

    pub async fn send_zcl(
        &self,
        dst_addr: u16,
        dst_ep: u8,
        cluster_id: u16,
        trans_id: u8,
        payload: Vec<u8>,
    ) -> Result<()> {
        self.inner
            .lock()
            .await
            .send_zcl(dst_addr, dst_ep, cluster_id, trans_id, payload)
            .await
    }

    /// Send a ZCL frame and wait for a specific response command.
    #[allow(clippy::too_many_arguments)]
    pub async fn request_zcl_expected(
        &self,
        dst_addr: u16,
        dst_ep: u8,
        cluster_id: u16,
        trans_id: u8,
        payload: Vec<u8>,
        timeout: std::time::Duration,
        attempts: u8,
        expected_response: Option<u8>,
    ) -> Result<ZclIncoming> {
        self.inner
            .lock()
            .await
            .request_zcl_expected(
                dst_addr,
                dst_ep,
                cluster_id,
                trans_id,
                payload,
                timeout,
                attempts,
                expected_response,
            )
            .await
    }

    /// Send a ZCL frame and wait for the device response (see ZnpHandle).
    #[allow(clippy::too_many_arguments)]
    pub async fn request_zcl(
        &self,
        dst_addr: u16,
        dst_ep: u8,
        cluster_id: u16,
        trans_id: u8,
        payload: Vec<u8>,
        timeout: std::time::Duration,
        attempts: u8,
    ) -> Result<ZclIncoming> {
        self.inner
            .lock()
            .await
            .request_zcl(
                dst_addr, dst_ep, cluster_id, trans_id, payload, timeout, attempts,
            )
            .await
    }

    pub async fn request_active_eps(&self, nwk_addr: u16) -> Result<()> {
        self.inner.lock().await.request_active_eps(nwk_addr).await
    }

    pub async fn request_simple_desc(&self, nwk_addr: u16, endpoint: u8) -> Result<()> {
        self.inner
            .lock()
            .await
            .request_simple_desc(nwk_addr, endpoint)
            .await
    }

    pub async fn request_ieee_addr(&self, nwk_addr: u16) -> Result<()> {
        self.inner.lock().await.request_ieee_addr(nwk_addr).await
    }

    pub async fn request_node_desc(&self, nwk_addr: u16) -> Result<()> {
        self.inner.lock().await.request_node_desc(nwk_addr).await
    }

    /// Bind `cluster_id` on the device at `dst_addr`'s `src_ep` to `dst_ieee`'s
    /// `dst_ep` (normally this coordinator's own address). See
    /// `znp::commands::zdo_bind_req` for why this is needed.
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
        self.inner
            .lock()
            .await
            .request_bind(dst_addr, src_ieee, src_ep, cluster_id, dst_ieee, dst_ep)
            .await
    }
}

pub async fn open_coordinator(cfg: &Config, config_path: &Path) -> Result<CoordinatorHandle> {
    match cfg.serial.adapter {
        AdapterType::Znp | AdapterType::Zstack | AdapterType::Auto => {
            let coord = ZnpCoordinator::open(&cfg.serial.port, cfg.serial.baudrate)?;
            coord.start(cfg, config_path).await
        }
        AdapterType::Ezsp => Err(crate::error::Error::Config(
            "EZSP adapter not yet supported".into(),
        )),
    }
}
