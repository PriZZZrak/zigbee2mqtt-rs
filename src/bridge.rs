/// The main bridge -- ties coordinator, MQTT, device registry, and ZCL together.
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use std::path::PathBuf;

use crate::config::{Config, OutputMode};
use crate::converters::external::ExternalConverter;
use crate::coordinator::znp::ZnpHandle;
use crate::coordinator::{open_coordinator, CoordinatorEvent, CoordinatorHandle};
use crate::database::{self, PersistentDatabase};
use crate::devices::{Device, DeviceRegistry};
use crate::error::Result;
use crate::groups::GroupRegistry;
use crate::homeassistant;
use crate::mqtt::attribute_output::flatten_attribute_output;
use crate::mqtt::{MqttBridge, MqttCommand};
use crate::zigbee::zcl;
use crate::zigbee::zcl::clusters::color;
use crate::zigbee::zcl::clusters::level;
use crate::zigbee::zcl::clusters::on_off;
use crate::zigbee::{EndpointDesc, IeeeAddr};

/// De-duplicates IEEE-address lookups for unknown network addresses.
///
/// Mirrors herdsman's `unknownDevices` set: a failed lookup stays suppressed for
/// the rest of the process so a non-responding device cannot spam the
/// coordinator; a successful resolution clears the key.
#[derive(Default)]
struct UnknownDeviceTracker {
    requested: HashSet<u16>,
}

impl UnknownDeviceTracker {
    fn should_identify(&mut self, nwk_addr: u16) -> bool {
        self.requested.insert(nwk_addr)
    }

    fn identified(&mut self, nwk_addr: u16) {
        self.requested.remove(&nwk_addr);
    }
}

type ReportingFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// Transport seam for the reporting bridge requests, enabling a fake coordinator
/// in tests without a serial port.
pub(crate) trait ReportingIo {
    fn coordinator_ieee(&self) -> Option<IeeeAddr>;
    fn read<'a>(
        &'a self,
        nwk_addr: u16,
        endpoint: u8,
        cluster: u16,
        tsn: u8,
        payload: Vec<u8>,
    ) -> ReportingFuture<'a, crate::error::Result<crate::coordinator::znp::transaction::ZclIncoming>>;
    #[allow(clippy::too_many_arguments)]
    fn bind<'a>(
        &'a self,
        nwk_addr: u16,
        source_ieee: [u8; 8],
        endpoint: u8,
        cluster: u16,
        destination_ieee: [u8; 8],
    ) -> ReportingFuture<'a, crate::error::Result<()>>;
}

impl ReportingIo for CoordinatorHandle {
    fn coordinator_ieee(&self) -> Option<IeeeAddr> {
        self.info.ieee_addr
    }

    fn read<'a>(
        &'a self,
        nwk_addr: u16,
        endpoint: u8,
        cluster: u16,
        tsn: u8,
        payload: Vec<u8>,
    ) -> ReportingFuture<'a, crate::error::Result<crate::coordinator::znp::transaction::ZclIncoming>>
    {
        Box::pin(async move {
            self.request_zcl(
                nwk_addr,
                endpoint,
                cluster,
                tsn,
                payload,
                Duration::from_secs(3),
                2,
            )
            .await
        })
    }

    fn bind<'a>(
        &'a self,
        nwk_addr: u16,
        source_ieee: [u8; 8],
        endpoint: u8,
        cluster: u16,
        destination_ieee: [u8; 8],
    ) -> ReportingFuture<'a, crate::error::Result<()>> {
        Box::pin(async move {
            self.request_bind(
                nwk_addr,
                source_ieee,
                endpoint,
                cluster,
                destination_ieee,
                1,
            )
            .await
        })
    }
}

pub struct Bridge {
    cfg: Config,
    config_path: PathBuf,
    devices: Arc<DeviceRegistry>,
    groups: Arc<GroupRegistry>,
    database: Arc<Mutex<PersistentDatabase>>,
    db_write_pending: Arc<Mutex<bool>>,
    external: Arc<ExternalConverter>,
    ota: Arc<Mutex<crate::ota::OtaManager>>,
}

/// Resolve when the service manager or user asks the bridge to stop.
///
/// Without this, OpenWrt's procd sends SIGTERM and the process exits before the
/// post-loop offline publication and database flush can run.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut terminate = match signal(SignalKind::terminate()) {
            Ok(signal) => signal,
            Err(error) => {
                warn!("Cannot install SIGTERM handler: {error}");
                let _ = tokio::signal::ctrl_c().await;
                return;
            }
        };
        tokio::select! {
            _ = terminate.recv() => {}
            _ = tokio::signal::ctrl_c() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

impl Bridge {
    fn device_allowed(cfg: &Config, ieee: &str) -> bool {
        let ieee = ieee.to_lowercase();
        if cfg.blocklist.iter().any(|b| b.to_lowercase() == ieee) {
            return false;
        }
        if !cfg.passlist.is_empty() && !cfg.passlist.iter().any(|p| p.to_lowercase() == ieee) {
            return false;
        }
        true
    }

    pub fn new(cfg: Config, config_path: PathBuf) -> Self {
        let db_path = database::find_database(&config_path).unwrap_or_else(|| {
            config_path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("database.db")
        });
        let db = PersistentDatabase::open(&db_path);
        let groups = GroupRegistry::new();
        // Load groups from database into registry
        for g in db.load_groups() {
            // Preserve friendly_name if stored, otherwise keep as is
            groups.inner.insert(g.id, g.clone());
            groups.by_name.insert(g.friendly_name.clone(), g.id);
            // Ensure next_id is beyond max group id
            let cur = groups.next_id.load(std::sync::atomic::Ordering::Relaxed);
            if g.id >= cur {
                groups
                    .next_id
                    .store(g.id + 1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        // Overlay groups from configuration.yaml (friendly_name authoritative)
        for (id_str, gcfg) in &cfg.groups {
            if let Ok(gid) = id_str.parse::<u16>() {
                if let Some(mut g) = groups.inner.get_mut(&gid) {
                    if let Some(fname) = &gcfg.friendly_name {
                        // Update friendly_name from config if different
                        let old = g.friendly_name.clone();
                        if &old != fname {
                            groups.by_name.remove(&old);
                            g.friendly_name = fname.clone();
                            groups.by_name.insert(fname.clone(), gid);
                        }
                    }
                } else if let Some(fname) = &gcfg.friendly_name {
                    // Group in config but not in DB — create it
                    let g = crate::groups::Group {
                        id: gid,
                        friendly_name: fname.clone(),
                        members: Vec::new(),
                    };
                    groups.by_name.insert(fname.clone(), gid);
                    groups.inner.insert(gid, g);
                    let cur = groups.next_id.load(std::sync::atomic::Ordering::Relaxed);
                    if gid >= cur {
                        groups
                            .next_id
                            .store(gid + 1, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }
        }
        let external = ExternalConverter::new(
            "/etc/zigbee2mqtt-rs/herdsman-converters",
            "https://prizzzrak.github.io/zigbee2mqtt-rs/herdsman-converters/latest",
        );
        let _ = external.ensure_dir();
        Self {
            cfg,
            config_path,
            devices: Arc::new(DeviceRegistry::new()),
            groups: Arc::new(groups),
            database: Arc::new(Mutex::new(db)),
            db_write_pending: Arc::new(Mutex::new(false)),
            external: Arc::new(external),
            ota: Arc::new(Mutex::new(crate::ota::OtaManager::default())),
        }
    }

    /// Flush any debounced database update immediately and surface failures.
    ///
    /// Called on shutdown so an accepted join/rename isn't lost while the
    /// 500 ms write task is still sleeping. A failed write keeps the dirty
    /// state in memory and is reported as an error, not swallowed.
    async fn flush_database(&self) {
        let result = {
            let db = self.database.lock().await;
            db.write()
        };
        match result {
            Ok(()) => {
                let mut pending = self.db_write_pending.lock().await;
                *pending = false;
            }
            Err(error) => error!("Failed to flush database.db: {error}"),
        }
    }

    /// Publish HA discovery using upstream exposes when available, otherwise
    /// the cluster-inference fallback.
    async fn publish_device_discovery(
        &self,
        mqtt: &MqttBridge,
        device: &Device,
        base_topic: &str,
        coordinator_ieee: &str,
    ) {
        let info = crate::converters::external::ExternalDeviceInfo::from_device(device);
        let model = device.model.clone().unwrap_or_default();
        match self.external.exposes(&model, &info) {
            Some(exposes) if !exposes.is_empty() => {
                homeassistant::publish_exposes_discovery(
                    mqtt,
                    device,
                    &exposes,
                    base_topic,
                    coordinator_ieee,
                )
                .await;
            }
            _ => {
                homeassistant::publish_discovery(mqtt, device, base_topic, coordinator_ieee).await;
            }
        }
    }

    async fn persist_database(&self, coordinator_ieee: Option<IeeeAddr>) {
        // Batching: update in-memory immediately, debounce disk write to save memory/I/O (upstream Entity.database.update(write:false))
        {
            let devices = self.devices.all_devices();
            let groups = self.groups.all();
            let assigned = {
                let mut db = self.database.lock().await;
                let assigned = db.sync_devices(&devices, coordinator_ieee);
                db.sync_groups(&groups);
                assigned
            };
            // Keep the assigned ids on the live entities so the next
            // synchronization does not allocate a fresh record for them.
            for (ieee, id) in assigned {
                self.devices.set_database_id(&ieee, id);
            }
        }
        let mut pending = self.db_write_pending.lock().await;
        if *pending {
            return;
        }
        *pending = true;
        drop(pending);
        let db_clone = Arc::clone(&self.database);
        let pending_clone = Arc::clone(&self.db_write_pending);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            {
                let mut p = pending_clone.lock().await;
                *p = false;
            }
            let db = db_clone.lock().await;
            if let Err(e) = db.write() {
                // Keep the in-memory records; the next mutation or the shutdown
                // flush retries the write. Never report success to the caller.
                error!("Failed to write database.db (batched): {e}");
            }
        });
    }

    async fn persist_groups_config(&self) {
        // Read current config file, update groups section, write back
        let path = &self.config_path;
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warn!(
                    "Cannot read config for groups persist {}: {e}",
                    path.display()
                );
                return;
            }
        };
        let mut cfg: crate::config::Config = match serde_yml::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                warn!("Cannot parse config for groups persist: {e}");
                return;
            }
        };
        cfg.groups.clear();
        for g in self.groups.all() {
            cfg.groups.insert(
                g.id.to_string(),
                crate::config::GroupConfig {
                    friendly_name: Some(g.friendly_name.clone()),
                },
            );
        }
        if let Err(e) = cfg.save(path) {
            warn!("Failed to save groups to config {}: {e}", path.display());
        }
    }

    pub async fn run(self) -> Result<()> {
        // 1. Import device database from existing zigbee2mqtt install
        self.import_database();

        // 2. Apply device configs (friendly names override database)
        self.apply_device_configs();

        // 3. Log device summary
        let all = self.devices.all_devices();
        info!(
            "Device registry: {} devices ({} interviewed)",
            all.len(),
            all.iter().filter(|d| d.interview_complete).count()
        );
        for dev in &all {
            info!(
                "  {} ({}) nwk=0x{:04X} eps={} interviewed={} model={:?}",
                dev.friendly_name,
                dev.ieee_addr,
                dev.nwk_addr,
                dev.endpoints.len(),
                dev.interview_complete,
                dev.model.as_deref().unwrap_or("-"),
            );
        }
        drop(all);

        // 4. Connect MQTT
        let (mqtt, mut mqtt_rx) = MqttBridge::connect(&self.cfg.mqtt).await?;
        let _ = mqtt.publish_bridge_extensions().await;
        info!("MQTT connected (base_topic={})", self.cfg.mqtt.base_topic);

        // 5. Open coordinator
        let mut coord = open_coordinator(&self.cfg, &self.config_path).await?;
        info!("Coordinator ready");

        // 6. Publish retained MQTT state
        self.republish_mqtt_state(&mqtt, &coord).await;

        // 7. Permit join if configured
        if self.cfg.permit_join {
            coord.permit_join(254).await?;
            info!("Permit join enabled (254 s)");
        }
        info!("Startup complete, entering main loop");
        // Warm registry cache (gh-pages, 24h)
        let _ = self.external.fetch_registry().await;

        let devices = Arc::clone(&self.devices);
        let groups = Arc::clone(&self.groups);
        let external = Arc::clone(&self.external);
        let base_topic = self.cfg.mqtt.base_topic.clone();
        let ha_enabled = self.cfg.homeassistant.enabled();
        let device_configs = self.cfg.devices.clone();
        let coordinator_ieee = coord.info.ieee_addr.map(|a| a.as_hex()).unwrap_or_default();

        // 7. Main event loop
        let mut trans_id: u8 = 0;
        let mut unknown_devices = UnknownDeviceTracker::default();

        loop {
            tokio::select! {
                // -- Coordinator events --
                event = coord.events.recv() => {
                    match event {
                        None => {
                            error!("Coordinator event channel closed");
                            break;
                        }
                        Some(CoordinatorEvent::DeviceJoined { ieee_addr, nwk_addr }) => {
                            let ieee = ieee_addr;
                            unknown_devices.identified(nwk_addr);
                            if !Self::device_allowed(&self.cfg, &ieee.as_hex()) {
                                info!("Device {ieee} is blocked/filtered, skipping");
                                continue;
                            }
                            info!("Device joined: {ieee} (0x{nwk_addr:04X})");

                            if devices.get_by_ieee(&ieee).is_none() {
                                let mut dev = Device::new(ieee, nwk_addr);
                                // Apply friendly name from config
                                if let Some(cfg) = device_configs.get(&ieee.as_hex()) {
                                    if let Some(ref name) = cfg.friendly_name {
                                        dev.friendly_name = name.clone();
                                    }
                                    dev.disabled = cfg.disabled.unwrap_or(false);
                                }
                                devices.add(dev);
                            } else {
                                devices.update_nwk_addr(&ieee, nwk_addr);
                            }
                            self.persist_database(coord.info.ieee_addr).await;

                            if let Err(e) = mqtt.publish_bridge_log("info", &format!("Device joined: {ieee}")).await {
                                warn!("Failed to publish bridge log: {e}");
                            }
                            Self::spawn_interview_retry(Arc::clone(&devices), Arc::clone(&coord.inner), ieee, nwk_addr);
                        }

                        Some(CoordinatorEvent::DeviceLeft { ieee_addr, .. }) => {
                            let ieee = ieee_addr;
                            info!("Device left: {ieee}");
                            devices.remove_by_ieee(&ieee);
                            self.persist_database(coord.info.ieee_addr).await;
                            if let Err(e) = mqtt.publish_bridge_log("info", &format!("Device left: {ieee}")).await {
                                warn!("Failed to publish bridge log: {e}");
                            }
                            Self::publish_device_list_static(&devices, &mqtt).await;
                        }

                        Some(CoordinatorEvent::AddressResolved { ieee_addr, nwk_addr }) => {
                            let ieee = ieee_addr;
                            debug!("Address resolved: {ieee} -> 0x{nwk_addr:04X}");
                            unknown_devices.identified(nwk_addr);

                            let known = devices.get_by_ieee(&ieee).is_some();
                            if known {
                                info!("Linking {ieee} to NWK 0x{nwk_addr:04X}");
                                devices.update_nwk_addr(&ieee, nwk_addr);
                            } else if Self::device_allowed(&self.cfg, &ieee.as_hex()) {
                                // Unknown IEEE -- create a new device entry
                                let mut dev = Device::new(ieee, nwk_addr);
                                if let Some(cfg) = device_configs.get(&ieee.as_hex()) {
                                    if let Some(ref name) = cfg.friendly_name {
                                        dev.friendly_name = name.clone();
                                    }
                                }
                                devices.add(dev);
                            }

                            // Trigger interview if not done yet
                            if devices.get_by_ieee(&ieee).is_some_and(|d| !d.interview_complete) {
                                Self::spawn_interview_retry(Arc::clone(&devices), Arc::clone(&coord.inner), ieee, nwk_addr);
                            }
                        }

                        Some(CoordinatorEvent::NodeDescRsp {
                            nwk_addr,
                            manufacturer_code,
                            logical_type,
                        }) => {
                            debug!("NodeDesc 0x{nwk_addr:04X} manuf=0x{manufacturer_code:04X} type={logical_type}");
                            if let Some(mut dev) = devices.get_mut_by_nwk(nwk_addr) {
                                dev.manufacturer_id = Some(manufacturer_code);
                                // Map logicalType 0=Coord,1=Router,2=EndDevice
                                if dev.power_source.is_none() {
                                    dev.power_source = Some(match logical_type {
                                        1 => "Mains (single phase)".into(),
                                        2 => "Battery".into(),
                                        _ => "Unknown".into(),
                                    });
                                }
                                drop(dev);
                                self.persist_database(coord.info.ieee_addr).await;
                                // Trigger ActiveEp after NodeDesc (upstream interview order)
                                if let Err(e) = coord.request_active_eps(nwk_addr).await {
                                    warn!("Failed to request active endpoints after NodeDesc: {e}");
                                }
                            }
                        }

                        Some(CoordinatorEvent::ActiveEpRsp { nwk_addr, endpoints }) => {
                            debug!("Active EPs for 0x{nwk_addr:04X}: {endpoints:?}");
                            if let Some(mut dev) = devices.get_mut_by_nwk(nwk_addr) {
                                dev.interview_expected_eps = Some(endpoints.clone());
                            }
                            for ep in endpoints {
                                if let Err(e) = coord.request_simple_desc(nwk_addr, ep).await {
                                    warn!("Failed to request simple descriptor: {e}");
                                }
                            }
                        }

                        Some(CoordinatorEvent::BindRsp { nwk_addr, status }) => {
                            if status == 0 {
                                debug!("Bind succeeded for 0x{nwk_addr:04X}");
                            } else {
                                warn!("Bind failed for 0x{nwk_addr:04X}: status=0x{status:02X}");
                            }
                        }

                        Some(CoordinatorEvent::SimpleDescRsp {
                            nwk_addr, endpoint, profile_id, device_id,
                            input_clusters, output_clusters
                        }) => {
                            debug!("SimpleDesc 0x{nwk_addr:04X} ep={endpoint} clusters={input_clusters:?}");
                            let ep_desc = EndpointDesc {
                                endpoint,
                                profile_id,
                                device_id,
                                input_clusters: input_clusters.clone(),
                                output_clusters: output_clusters.clone(),
                            };

                            let mut interview_just_completed = false;
                            let mut src_ieee: Option<IeeeAddr> = None;
                            if let Some(mut dev) = devices.get_mut_by_nwk(nwk_addr) {
                                dev.endpoints.retain(|e| e.endpoint != endpoint);
                                dev.endpoints.push(ep_desc);
                                let expected_done = dev
                                    .interview_expected_eps
                                    .as_ref()
                                    .map(|exp| exp.iter().all(|ep| dev.endpoints.iter().any(|e| &e.endpoint == ep)))
                                    .unwrap_or(!dev.endpoints.is_empty());
                                if !dev.interview_complete && expected_done {
                                    dev.interview_complete = true;
                                    dev.interview_expected_eps = None;
                                    interview_just_completed = true;
                                    info!("Interview complete for {}", dev.display_name());
                                }
                                src_ieee = Some(dev.ieee_addr);
                                if interview_just_completed {
                                    self.persist_database(coord.info.ieee_addr).await;
                                }
                            }

                            // Bind this endpoint's output clusters to the coordinator so
                            // devices whose relevant clusters are *output* clusters (e.g.
                            // remote controls -- button presses are genOnOff/genLevelCtrl/
                            // genScenes commands) have somewhere to unicast their commands.
                            // Without this, such devices join fine but produce no Zigbee
                            // traffic at all when used.
                            if let Some(dst_ieee) = coord.info.ieee_addr {
                                if let Some(src_ieee) = src_ieee {
                                    for &cluster_id in &output_clusters {
                                        if let Err(e) = coord
                                            .request_bind(nwk_addr, src_ieee.0, endpoint, cluster_id, dst_ieee.0, 1)
                                            .await
                                        {
                                            warn!("Failed to request bind for 0x{nwk_addr:04X} ep={endpoint} cluster=0x{cluster_id:04X}: {e}");
                                        }
                                    }
                                }
                            } else if !output_clusters.is_empty() {
                                warn!("Cannot bind 0x{nwk_addr:04X} ep={endpoint}: coordinator IEEE address unknown");
                            }

                            // Request basic cluster attributes (manufacturer, model)
                            if input_clusters.contains(&0x0000) {
                                trans_id = trans_id.wrapping_add(1);
                                let payload = crate::zigbee::zcl::frame::read_attributes_payload(
                                    trans_id,
                                    &[0x0004, 0x0005, 0x0007, 0x4000],
                                );
                                if let Err(e) = coord.send_zcl(nwk_addr, endpoint, 0x0000, trans_id, payload).await {
                                    warn!("Failed to send ZCL read attributes: {e}");
                                }
                            }

                            // Request IAS Zone ZoneType, so zone status reports can be
                            // classified correctly (contact/occupancy/smoke/water_leak/...)
                            // instead of always assumed to be a door/contact sensor.
                            if input_clusters.contains(&0x0500) {
                                trans_id = trans_id.wrapping_add(1);
                                let payload = crate::zigbee::zcl::frame::read_attributes_payload(
                                    trans_id,
                                    &[0x0001],
                                );
                                if let Err(e) = coord.send_zcl(nwk_addr, endpoint, 0x0500, trans_id, payload).await {
                                    warn!("Failed to send ZCL read attributes (IAS Zone type): {e}");
                                }
                            }

                            // Request Power Configuration battery attributes, so the
                            // battery/battery_voltage state keys (and the HA battery
                            // sensor discovered for this device) actually get a value
                            // instead of waiting on an unsolicited report that most
                            // devices never send without being asked.
                            if input_clusters.contains(&0x0001) {
                                trans_id = trans_id.wrapping_add(1);
                                let payload = crate::zigbee::zcl::frame::read_attributes_payload(
                                    trans_id,
                                    &[0x0020, 0x0021],
                                );
                                if let Err(e) = coord.send_zcl(nwk_addr, endpoint, 0x0001, trans_id, payload).await {
                                    warn!("Failed to send ZCL read attributes (Power Configuration): {e}");
                                }
                            }

                            Self::publish_device_list_static(&devices, &mqtt).await;

                            // Publish HA discovery after interview
                            if ha_enabled && interview_just_completed {
                                if let Some(dev) = devices.get_by_nwk(nwk_addr) {
                                    self.publish_device_discovery(&mqtt, &dev, &base_topic, &coordinator_ieee).await;
                                }
                            }
                        }

                        Some(CoordinatorEvent::Message {
                            src_addr, src_ep, cluster_id, link_quality, data
                        }) => {
                            debug!("AF msg from 0x{src_addr:04X} ep={src_ep} cluster=0x{cluster_id:04X} lqi={link_quality}");

                            // Unknown NWK? Request IEEE address once per runtime session.
                            // A failed lookup stays suppressed (upstream keeps the nwk
                            // in its unknownDevices set) until a successful resolution or
                            // a fresh announce clears it.
                            if devices.get_by_nwk(src_addr).is_none()
                                && unknown_devices.should_identify(src_addr)
                            {
                                debug!("Unknown NWK 0x{src_addr:04X}, requesting IEEE address");
                                if let Err(e) = coord.request_ieee_addr(src_addr).await {
                                    warn!("Failed to request IEEE address: {e}");
                                }
                            }

                            if cluster_id == 0x0000 {
                                Self::handle_basic_cluster_response(&devices, src_addr, &data);
                            }
                            if cluster_id == 0x0500 {
                                Self::handle_ias_zone_type_response(&devices, src_addr, &data);
                            }
                            if cluster_id == crate::zigbee::zcl::clusters::ota::CLUSTER_ID {
                                if let Some(ieee) =
                                    devices.get_by_nwk(src_addr).map(|device| device.ieee_addr)
                                {
                                    let tsn = if data.first().is_some_and(|control| control & 0x04 != 0) {
                                        data.get(3).copied().unwrap_or(0)
                                    } else {
                                        data.get(1).copied().unwrap_or(0)
                                    };
                                    let action = self
                                        .ota
                                        .lock()
                                        .await
                                        .handle_zcl(&ieee.as_hex(), &data, tsn);
                                    if let Some(action) = action {
                                        if let Err(error) = coord
                                            .send_zcl(
                                                src_addr,
                                                src_ep,
                                                crate::zigbee::zcl::clusters::ota::CLUSTER_ID,
                                                tsn,
                                                action.response,
                                            )
                                            .await
                                        {
                                            warn!("OTA response send failed: {error}");
                                        }
                                        if let Some(progress) = action.progress {
                                            if let Some(mut device) = devices.get_mut_by_nwk(src_addr) {
                                                device.state.insert(
                                                    "update".into(),
                                                    ota_update_state(progress, action.completed),
                                                );
                                                let state = serde_json::Value::Object(device.state.clone());
                                                let name = device.friendly_name.clone();
                                                drop(device);
                                                Self::publish_state(
                                                    &mqtt,
                                                    self.cfg.advanced.output,
                                                    &name,
                                                    &state,
                                                )
                                                .await;
                                            }
                                        }
                                    }
                                }
                            }

                            let zone_type = devices.get_by_nwk(src_addr).and_then(|d| d.zone_type);
                            let held_arrow_direction = devices
                                .get_by_nwk(src_addr)
                                .and_then(|d| d.arrow_hold_direction.clone());

                            // Native parse and upstream definition conversion run for
                            // the same frame; their values are merged once and published
                            // once, with the device definition taking precedence.
                            let mut values = serde_json::Map::new();
                            let mut arrow_update = None;
                            let mut parse_error = None;
                            match zcl::parse_message(
                                cluster_id,
                                &data,
                                zone_type,
                                held_arrow_direction.as_deref(),
                            ) {
                                Ok(Some(mut zcl_msg)) => {
                                    if cluster_id == 0x0000 {
                                        for key in BASIC_CLUSTER_METADATA_KEYS {
                                            zcl_msg.values.remove(key);
                                        }
                                    }
                                    arrow_update = zcl_msg.arrow_hold_direction;
                                    values = zcl_msg.values;
                                }
                                Ok(None) => {}
                                Err(error) => parse_error = Some(error),
                            }

                            let mut external_values = None;
                            if let Ok((header, offset)) =
                                zcl::frame::ZclFrameHeader::parse(&data)
                            {
                                if header.frame_type == zcl::frame::FrameType::Global {
                                    let msg_type = match header.command_id {
                                        zcl::frame::global::REPORT_ATTRIBUTES => {
                                            Some("attributeReport")
                                        }
                                        zcl::frame::global::READ_ATTRIBUTES_RSP => {
                                            Some("readResponse")
                                        }
                                        _ => None,
                                    };
                                    if let Some(msg_type) = msg_type {
                                        let reports: Vec<(u16, serde_json::Value)> =
                                            if msg_type == "readResponse" {
                                                zcl::parse_read_attr_rsp(&data[offset..])
                                            } else {
                                                zcl::attribute::AttributeReport::parse_all(
                                                    &data[offset..],
                                                )
                                            }
                                            .into_iter()
                                            .filter_map(|report| {
                                                attribute_json(&report.value)
                                                    .map(|value| (report.attr_id, value))
                                            })
                                            .collect();
                                        if !reports.is_empty() {
                                            let prepared = devices
                                                .get_by_nwk(src_addr)
                                                .map(|dev| {
                                                    let info = crate::converters::external::ExternalDeviceInfo::from_device(&dev);
                                                    (
                                                        dev.model.clone().unwrap_or_default(),
                                                        info,
                                                        dev.state.clone(),
                                                    )
                                                });
                                            if let Some((model, info, state)) = prepared {
                                                if external
                                                    .ensure_device_files(&model, &info)
                                                    .await
                                                    .is_some()
                                                {
                                                    let message =
                                                        crate::converters::external::IncomingMessageInfo {
                                                            endpoint: src_ep,
                                                            linkquality: link_quality,
                                                            group_id: 0,
                                                        };
                                                    if let Some(converted) =
                                                        external.convert_from_zigbee_session(
                                                            &model,
                                                            cluster_id,
                                                            msg_type,
                                                            &reports,
                                                            &info,
                                                            &message,
                                                            &state,
                                                        )
                                                    {
                                                        external_values = Some(converted);
                                                    }
                                                } else {
                                                    debug!("external converter: no candidates for model {model:?}");
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            let external_used = external_values.is_some();
                            values = merge_converted_values(values, external_values);
                            if !values.is_empty() {
                                let suffix = devices.get_by_nwk(src_addr).and_then(|d| {
                                    if d.endpoints.len() <= 1 {
                                        return None;
                                    }
                                    match src_ep {
                                        1 => Some("l1".to_string()),
                                        2 => Some("l2".to_string()),
                                        3 => Some("button".to_string()),
                                        _ => None,
                                    }
                                });
                                if let Some(suffix) = &suffix {
                                    let keys: Vec<String> = values.keys().cloned().collect();
                                    let mut suffixed = serde_json::Map::new();
                                    for key in keys {
                                        if let Some(value) = values.remove(&key) {
                                            suffixed.insert(format!("{key}_{suffix}"), value);
                                        }
                                    }
                                    values = suffixed;
                                }
                                if let Some(mut dev) = devices.get_mut_by_nwk(src_addr) {
                                    if let Some(direction_update) = arrow_update {
                                        dev.arrow_hold_direction = direction_update;
                                    }
                                    dev.merge_state(values);
                                    dev.state.insert("linkquality".into(), json!(link_quality));
                                    dev.state.insert("last_seen".into(), json!(now_iso8601()));
                                    let state = serde_json::Value::Object(dev.state.clone());
                                    let name = dev.friendly_name.clone();
                                    Self::strip_transient_action_fields(&mut dev.state);
                                    drop(dev);
                                    Self::publish_state(
                                        &mqtt,
                                        self.cfg.advanced.output,
                                        &name,
                                        &state,
                                    )
                                    .await;
                                    let _ = mqtt.publish_availability(&name, true).await;
                                }
                                if external_used {
                                    debug!("external converter handled cluster 0x{cluster_id:04X} for 0x{src_addr:04X}");
                                }
                            } else if let Some(error) = parse_error {
                                warn!("ZCL parse error: {error}");
                            }
                        }
                    }
                }

                // -- MQTT commands --
                cmd = mqtt_rx.recv() => {
                    match cmd {
                        None => break,
                        Some(MqttCommand::Connected) => {
                            info!("MQTT session restored, republishing retained state");
                            self.republish_mqtt_state(&mqtt, &coord).await;
                            let _ = mqtt.publish_bridge_extensions().await;
                        }
                        Some(MqttCommand::PermitJoin { duration }) => {
                            info!("Permit join: {duration}s");
                            if let Err(e) = coord.permit_join(duration).await {
                                warn!("Failed to set permit join: {e}");
                            }
                            if let Err(e) = mqtt.publish_bridge_log("info", &format!("Permit join: {duration}s")).await {
                                warn!("Failed to publish bridge log: {e}");
                            }
                            if duration > 0 {
                                let mqtt_clone = mqtt.clone();
                                tokio::spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_secs(duration as u64)).await;
                                    let _ = mqtt_clone.publish_bridge_log("info", "Permit join disabled").await;
                                });
                            }
                        }
                        Some(MqttCommand::SetDevice { friendly_name, payload }) => {
                            info!("MQTT set: {friendly_name} -> {payload}");
                            Self::handle_set(
                                &devices, &coord, &mqtt, self.cfg.advanced.output, &mut trans_id,
                                &friendly_name, &payload,
                            ).await;
                        }
                        Some(MqttCommand::GetDevice { friendly_name, payload }) => {
                            debug!("MQTT get: {friendly_name}");
                            Self::handle_get(
                                &devices, &self.external, &coord, &mqtt,
                                self.cfg.advanced.output, &mut trans_id,
                                &friendly_name, &payload,
                            ).await;
                        }
                        Some(MqttCommand::BridgeRequest { request, payload }) => {
                            let transaction = payload.get("transaction").and_then(|v| v.as_str()).map(|s| s.to_string());
                            let mk_resp = |status: &str, data: serde_json::Value, err: Option<String>| {
                                let mut obj = json!({"status": status, "data": data});
                                if let Some(t) = &transaction { obj["transaction"] = json!(t); }
                                if let Some(e) = err { obj["error"] = json!(e); }
                                obj
                            };
                            match request.as_str() {
                                "health_check" | "coordinator_check" => {
                                    let health = json!({
                                        "response_time": std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .map(|d| d.as_millis() as i64)
                                            .unwrap_or(0),
                                        "devices": devices.all_devices().len(),
                                    });
                                    let _ = mqtt.publish_bridge_health(&health).await;
                                    let resp = mk_resp("ok", health, None);
                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                }
                                "restart" => {
                                    let _ = mqtt.publish_bridge_log("info", "Restarting").await;
                                    let resp = mk_resp("ok", json!({}), None);
                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                }
                                "networkmap" => {
                                    let _ = mqtt.publish_bridge_networkmap(&devices).await;
                                    // windfront expects bridge/response/networkmap with graph
                                    let nodes: Vec<_> = devices.all_devices().into_iter().map(|d| json!({"ieeeAddr": d.ieee_addr.as_hex(), "friendlyName": d.friendly_name})).collect();
                                    let resp = mk_resp("ok", json!({"nodes": nodes, "links": []}), None);
                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                }
                                "device/interview" => {
                                    let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                    let found = if let Some(d) = devices.find_by_name(id) {
                                        Some((d.ieee_addr, d.nwk_addr))
                                    } else if let Some(ieee) = crate::zigbee::IeeeAddr::from_hex(id) {
                                        devices.get_by_ieee(&ieee).map(|r| (r.ieee_addr, r.nwk_addr))
                                    } else { None };
                                    if let Some((ieee, nwk)) = found {
                                        Self::spawn_interview_retry(Arc::clone(&devices), Arc::clone(&coord.inner), ieee, nwk);
                                        let resp = mk_resp("ok", json!({"id": id}), None);
                                        let _ = mqtt.publish_bridge_response("device/interview", &resp).await;
                                    } else {
                                        let resp = mk_resp("error", json!({}), Some("device not found".into()));
                                        let _ = mqtt.publish_bridge_response("device/interview", &resp).await;
                                    }
                                }
                                "device/rename" => {
                                    let from = payload.get("from").and_then(|v| v.as_str()).unwrap_or("");
                                    let to = payload.get("to").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                                    if from.is_empty() || to.is_empty() {
                                        let resp = mk_resp("error", json!({}), Some("Invalid payload".into()));
                                        let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                    } else if let Some(ieee) = devices.find_by_name(from).map(|d| d.ieee_addr).or_else(|| crate::zigbee::IeeeAddr::from_hex(from)) {
                                        let old_device = devices.get_by_ieee(&ieee).map(|device| device.clone());
                                        devices.rename(&ieee, to.clone());
                                        let ha_rename = self.cfg.homeassistant.enabled();
                                        if let (true, Some(old_device)) = (ha_rename, old_device) {
                                            let info = crate::converters::external::ExternalDeviceInfo::from_device(&old_device);
                                            let model = old_device.model.clone().unwrap_or_default();
                                            let exposes = self.external.exposes(&model, &info).unwrap_or_default();
                                            homeassistant::cleanup_discovery(&mqtt, &old_device, &exposes).await;
                                            if let Some(new_device) = devices.get_by_ieee(&ieee).map(|device| device.clone()) {
                                                self.publish_device_discovery(&mqtt, &new_device, &base_topic, &coordinator_ieee).await;
                                            }
                                        }
                                        Self::publish_device_list_static(&devices, &mqtt).await;
                                        self.persist_database(coord.info.ieee_addr).await;
                                        let resp = mk_resp("ok", json!({"from": from, "to": to, "homeassistant_rename": ha_rename}), None);
                                        let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                    } else {
                                        let resp = mk_resp("error", json!({}), Some("entity not found".into()));
                                        let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                    }
                                }
                                "group/rename" => {
                                    let from = payload.get("from").and_then(|v| v.as_str()).unwrap_or("");
                                    let to = payload.get("to").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                                    if from.is_empty() || to.is_empty() {
                                        let resp = mk_resp("error", json!({}), Some("Invalid payload".into()));
                                        let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                    } else if groups.rename(from, to.clone()).is_some() {
                                        let _ = mqtt.publish_bridge_groups(&groups).await;
                                        self.persist_database(coord.info.ieee_addr).await;
                                        self.persist_groups_config().await;
                                        let resp = mk_resp("ok", json!({"from": from, "to": to}), None);
                                        let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                    } else {
                                        let resp = mk_resp("error", json!({}), Some("group not found".into()));
                                        let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                    }
                                }
                                "device/remove" => {
                                    let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or(payload.as_str().unwrap_or(""));
                                    let ieee_opt = crate::zigbee::IeeeAddr::from_hex(id).or_else(|| devices.find_by_name(id).map(|d| d.ieee_addr));
                                    if let Some(ieee) = ieee_opt {
                                        if self.cfg.homeassistant.enabled() {
                                            if let Some(device) = devices.get_by_ieee(&ieee).map(|device| device.clone()) {
                                                let info = crate::converters::external::ExternalDeviceInfo::from_device(&device);
                                                let model = device.model.clone().unwrap_or_default();
                                                let exposes = self.external.exposes(&model, &info).unwrap_or_default();
                                                homeassistant::cleanup_discovery(&mqtt, &device, &exposes).await;
                                            }
                                        }
                                        devices.remove_by_ieee(&ieee);
                                        Self::publish_device_list_static(&devices, &mqtt).await;
                                        self.persist_database(coord.info.ieee_addr).await;
                                        let resp = mk_resp("ok", json!({"id": id}), None);
                                        let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                    } else {
                                        let resp = mk_resp("error", json!({}), Some("entity not found".into()));
                                        let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                    }
                                }
                                "group/remove" => {
                                    let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or(payload.as_str().unwrap_or(""));
                                    let gid: u16 = id.parse().unwrap_or_else(|_| groups.get_by_name(id).map(|g| g.id).unwrap_or(0));
                                    if groups.remove_by_id(gid).is_some() || groups.get_by_name(id).and_then(|g| groups.remove_by_id(g.id)).is_some() {
                                        let _ = mqtt.publish_bridge_groups(&groups).await;
                                        self.persist_database(coord.info.ieee_addr).await;
                                        self.persist_groups_config().await;
                                        let resp = mk_resp("ok", json!({"id": id}), None);
                                        let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                    } else {
                                        let resp = mk_resp("error", json!({}), Some("group not found".into()));
                                        let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                    }
                                }
                                "device/options" | "group/options" | "options" | "bridge/options" => {
                                    // Stub: accept options, return ok
                                    let resp = mk_resp("ok", json!({}), None);
                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                }
                                "group/add" => {
                                    let friendly = payload.get("friendly_name").and_then(|v| v.as_str()).unwrap_or("group").to_string();
                                    // Check duplicate
                                    if groups.get_by_name(&friendly).is_some() {
                                        let resp = mk_resp("error", json!({}), Some("group already exists".into()));
                                        let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                    } else {
                                        let g = groups.add(friendly.clone());
                                        let _ = mqtt.publish_bridge_groups(&groups).await;
                                        self.persist_database(coord.info.ieee_addr).await;
                                        self.persist_groups_config().await;
                                        let resp = mk_resp("ok", json!({"friendly_name": friendly, "id": g.id}), None);
                                        let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                    }
                                }
                                "group/members/add" => {
                                    let group_key = payload.get("group").and_then(|v| v.as_str()).or_else(|| payload.get("id").and_then(|v| v.as_str())).unwrap_or("");
                                    let device_key = payload.get("device").and_then(|v| v.as_str()).unwrap_or("");
                                    let endpoint: u8 = payload.get("endpoint").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
                                    let group = group_key.parse::<u16>().ok().and_then(|id| groups.get_by_id(id))
                                        .or_else(|| groups.get_by_name(group_key));
                                    let device = find_device_for_id(&devices, device_key);
                                    match (group, device) {
                                        (Some(group), Some(device)) => {
                                            trans_id = trans_id.wrapping_add(1);
                                            let tsn = trans_id;
                                            let command = crate::zigbee::zcl::clusters::groups::add_group_payload(tsn, group.id, &group.friendly_name);
                                            match send_group_command(&coord, device.nwk_addr, endpoint, tsn, command, Some(crate::zigbee::zcl::clusters::groups::RSP_ADD_GROUP)).await {
                                                Ok(response) if response.is_success() => {
                                                    groups.add_member(group.id, device.ieee_addr.as_hex(), endpoint);
                                                    let _ = mqtt.publish_bridge_groups(&groups).await;
                                                    self.persist_database(coord.info.ieee_addr).await;
                                                    self.persist_groups_config().await;
                                                    let resp = mk_resp("ok", response.as_json(), None);
                                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                                }
                                                Ok(response) => {
                                                    let resp = mk_resp("error", response.as_json(), Some(format!("addGroup status {}", response.status)));
                                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                                }
                                                Err(error) => {
                                                    let resp = mk_resp("error", json!({}), Some(error));
                                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                                }
                                            }
                                        }
                                        (None, _) => {
                                            let resp = mk_resp("error", json!({}), Some("group not found".into()));
                                            let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                        }
                                        (_, None) => {
                                            let resp = mk_resp("error", json!({}), Some("device not found".into()));
                                            let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                        }
                                    }
                                }
                                "group/members/remove" => {
                                    let group_key = payload.get("group").and_then(|v| v.as_str()).unwrap_or("");
                                    let device_key = payload.get("device").and_then(|v| v.as_str()).unwrap_or("");
                                    let endpoint: u8 = payload.get("endpoint").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
                                    let group = group_key.parse::<u16>().ok().and_then(|id| groups.get_by_id(id))
                                        .or_else(|| groups.get_by_name(group_key));
                                    let device = find_device_for_id(&devices, device_key);
                                    match (group, device) {
                                        (Some(group), Some(device)) => {
                                            trans_id = trans_id.wrapping_add(1);
                                            let tsn = trans_id;
                                            let command = crate::zigbee::zcl::clusters::groups::remove_group_payload(tsn, group.id);
                                            match send_group_command(&coord, device.nwk_addr, endpoint, tsn, command, Some(crate::zigbee::zcl::clusters::groups::RSP_REMOVE_GROUP)).await {
                                                Ok(response) if response.is_success() => {
                                                    groups.remove_member(group.id, &device.ieee_addr.as_hex(), endpoint);
                                                    let _ = mqtt.publish_bridge_groups(&groups).await;
                                                    self.persist_database(coord.info.ieee_addr).await;
                                                    self.persist_groups_config().await;
                                                    let resp = mk_resp("ok", response.as_json(), None);
                                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                                }
                                                Ok(response) => {
                                                    let resp = mk_resp("error", response.as_json(), Some(format!("removeGroup status {}", response.status)));
                                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                                }
                                                Err(error) => {
                                                    let resp = mk_resp("error", json!({}), Some(error));
                                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                                }
                                            }
                                        }
                                        (None, _) => {
                                            let resp = mk_resp("error", json!({}), Some("group not found".into()));
                                            let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                        }
                                        (_, None) => {
                                            let resp = mk_resp("error", json!({}), Some("device not found".into()));
                                            let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                        }
                                    }
                                }
                                "group/members/remove_all" => {
                                    let device_key = payload.get("device").and_then(|v| v.as_str()).unwrap_or("");
                                    let endpoint: u8 = payload.get("endpoint").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
                                    match find_device_for_id(&devices, device_key) {
                                        Some(device) => {
                                            trans_id = trans_id.wrapping_add(1);
                                            let tsn = trans_id;
                                            let command = crate::zigbee::zcl::clusters::groups::remove_all_groups_payload(tsn);
                                            match send_group_command(&coord, device.nwk_addr, endpoint, tsn, command, None).await {
                                                Ok(response) if response.is_success() => {
                                                    for group in groups.all() {
                                                        groups.remove_member(group.id, &device.ieee_addr.as_hex(), endpoint);
                                                    }
                                                    let _ = mqtt.publish_bridge_groups(&groups).await;
                                                    self.persist_database(coord.info.ieee_addr).await;
                                                    self.persist_groups_config().await;
                                                    let resp = mk_resp("ok", json!({"device": device.friendly_name, "endpoint": endpoint}), None);
                                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                                }
                                                Ok(response) => {
                                                    let resp = mk_resp("error", response.as_json(), Some(format!("removeAllGroups status {}", response.status)));
                                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                                }
                                                Err(error) => {
                                                    let resp = mk_resp("error", json!({}), Some(error));
                                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                                }
                                            }
                                        }
                                        None => {
                                            let resp = mk_resp("error", json!({}), Some("device not found".into()));
                                            let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                        }
                                    }
                                }
                                "device/bind" | "device/unbind" | "device/binds/clear" => {
                                    let resp = mk_resp("ok", json!({}), None);
                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                }
                                "device/configure" | "device/generate_external_definition" => {
                                    let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("");
                                    let resp = mk_resp("ok", json!({"id": id, "source": ""}), None);
                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                }
                                "device/reporting/read" | "device/reporting/configure" => {
                                    let result = Self::handle_reporting(
                                        &devices, &self.external, &coord, &self.database,
                                        &request, &payload, &mut trans_id,
                                    ).await;
                                    let resp = reporting_mqtt_response(result, transaction.as_deref());
                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                }
                                "backup" => {
                                    let resp = mk_resp("ok", json!({"zip": ""}), None);
                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                }
                                "install_code/add" | "touchlink/scan" | "touchlink/identify" | "touchlink/factory_reset" => {
                                    let resp = mk_resp("ok", json!({}), None);
                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                }
                                "converter/save" | "converter/remove" | "extension/save" | "extension/remove" | "action" => {
                                    let resp = mk_resp("ok", json!({}), None);
                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                }
                                _ if request.starts_with("device/ota_update/") => {
                                    let base_dir = self
                                        .config_path
                                        .parent()
                                        .unwrap_or_else(|| std::path::Path::new("."));
                                    match handle_ota_request(
                                        &self.ota, base_dir, &devices, &request, &payload,
                                    ).await {
                                        Ok(outcome) => {
                                            if let Some((ieee, update)) = outcome.state_update {
                                                if let Some(address) = IeeeAddr::from_hex(&ieee) {
                                                    if let Some(mut device) = devices.get_mut_by_ieee(&address) {
                                                        device.state.insert("update".into(), update);
                                                        let state = serde_json::Value::Object(device.state.clone());
                                                        let name = device.friendly_name.clone();
                                                        drop(device);
                                                        Self::publish_state(
                                                            &mqtt,
                                                            self.cfg.advanced.output,
                                                            &name,
                                                            &state,
                                                        )
                                                        .await;
                                                    }
                                                }
                                            }
                                            let resp = mk_resp("ok", outcome.data, None);
                                            let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                        }
                                        Err(error) => {
                                            let resp = mk_resp("error", json!({}), Some(error));
                                            let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                        }
                                    }
                                }
                                _ => {
                                    warn!("Unsupported bridge request: {request}");
                                    let resp = mk_resp("error", json!({}), Some(format!("Unsupported {request}")));
                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                }
                            }
                        }
                    }
                }
                _ = shutdown_signal() => {
                    info!("Shutdown signal received, stopping bridge");
                    break;
                }
            }
        }

        if let Err(e) = mqtt.publish_bridge_state(false).await {
            warn!("Failed to publish offline state: {e}");
        }
        self.flush_database().await;
        Ok(())
    }

    /// Import devices from an existing zigbee2mqtt database.db file.
    fn import_database(&self) {
        if let Some(db_path) = database::find_database(&self.config_path) {
            let (devices, _coord_ieee) = database::load_database(&db_path, &self.cfg.devices);
            for dev in devices {
                if Self::device_allowed(&self.cfg, &dev.ieee_addr.as_hex()) {
                    self.devices.add(dev);
                }
            }
        }
    }

    /// Apply friendly names from config, overriding database values.
    /// Also pre-seeds stub entries for devices in config but not in database.
    fn apply_device_configs(&self) {
        for (ieee_str, cfg) in &self.cfg.devices {
            if let Some(ieee) = IeeeAddr::from_hex(ieee_str) {
                let name = cfg.friendly_name.clone().unwrap_or_else(|| ieee.as_hex());
                let disabled = cfg.disabled.unwrap_or(false);

                if let Some(mut dev) = self.devices.get_mut_by_ieee(&ieee) {
                    dev.disabled = disabled;
                    drop(dev);
                    self.devices.rename(&ieee, name);
                } else {
                    // Pre-seed: create a stub device with NWK=0 (updated on join)
                    let mut dev = Device::new(ieee, 0);
                    dev.friendly_name = name;
                    dev.disabled = disabled;
                    self.devices.add(dev);
                }
            }
        }
    }

    async fn publish_bridge_info(&self, mqtt: &MqttBridge, coord: &CoordinatorHandle) {
        let coord_ieee = coord.info.ieee_addr.map(|a| a.as_hex()).unwrap_or_default();

        let info = json!({
            "version": env!("CARGO_PKG_VERSION"),
            "coordinator": {
                "ieee_address": coord_ieee,
                "type": "z-Stack",
                "meta": {
                    "revision": coord.info.transport_rev,
                    "version": coord.info.version,
                }
            },
            "log_level": self.cfg.advanced.log_level,
            "permit_join": self.cfg.permit_join,
            "config": {},
        });

        if let Err(e) = mqtt.publish_bridge_info(&info).await {
            warn!("Failed to publish bridge info: {e}");
        }
    }

    async fn republish_mqtt_state(&self, mqtt: &MqttBridge, coord: &CoordinatorHandle) {
        if let Err(e) = mqtt.publish_bridge_state(true).await {
            warn!("Failed to publish online state: {e}");
        }

        self.publish_bridge_info(mqtt, coord).await;

        let coordinator_ieee_str = coord.info.ieee_addr.map(|a| a.as_hex()).unwrap_or_default();

        for dev in self.devices.all_devices() {
            let state = serde_json::Value::Object(dev.state.clone());
            Self::publish_state(mqtt, self.cfg.advanced.output, &dev.friendly_name, &state).await;

            if self.cfg.homeassistant.enabled() && dev.interview_complete {
                self.publish_device_discovery(
                    mqtt,
                    &dev,
                    &self.cfg.mqtt.base_topic,
                    &coordinator_ieee_str,
                )
                .await;
            }
        }

        self.publish_device_list(mqtt).await;
        let _ = mqtt.publish_bridge_groups(&self.groups).await;
        let _ = mqtt.publish_bridge_extensions().await;
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_reporting(
        devices: &DeviceRegistry,
        external: &ExternalConverter,
        io: &dyn ReportingIo,
        database: &Arc<Mutex<PersistentDatabase>>,
        request: &str,
        payload: &serde_json::Value,
        trans_id: &mut u8,
    ) -> std::result::Result<serde_json::Value, String> {
        let id = payload
            .get("id")
            .and_then(|id| id.as_str())
            .ok_or_else(|| "missing id".to_string())?;
        let endpoint = payload
            .get("endpoint")
            .and_then(|endpoint| endpoint.as_u64())
            .ok_or_else(|| "missing endpoint".to_string())? as u8;
        let cluster = payload
            .get("cluster")
            .and_then(|cluster| cluster.as_u64())
            .ok_or_else(|| "missing cluster".to_string())? as u16;
        let device =
            find_device_for_id(devices, id).ok_or_else(|| format!("unknown device {id}"))?;
        if !device
            .endpoints
            .iter()
            .any(|candidate| candidate.endpoint == endpoint)
        {
            return Err(format!("device {id} has no endpoint {endpoint}"));
        }

        if request.ends_with("/read") {
            let configs = payload
                .get("configs")
                .and_then(|configs| configs.as_array())
                .ok_or_else(|| "missing configs".to_string())?;
            let mut attributes = Vec::new();
            for config in configs {
                match config.get("attribute") {
                    Some(serde_json::Value::Number(number)) => {
                        if let Some(attribute) = number.as_u64() {
                            attributes.push(attribute as u16);
                        }
                    }
                    Some(serde_json::Value::String(name)) => {
                        if let Some(attribute) = external.attribute_id(cluster, name) {
                            attributes.push(attribute);
                        } else {
                            return Err(format!("unknown attribute {name} for cluster {cluster}"));
                        }
                    }
                    _ => return Err("config is missing an attribute".to_string()),
                }
            }
            if attributes.is_empty() {
                return Err("no attributes to read".to_string());
            }
            *trans_id = trans_id.wrapping_add(1);
            let tsn = *trans_id;
            let request_payload =
                crate::zigbee::zcl::frame::read_reporting_config_payload(tsn, &attributes);
            let response = io
                .read(device.nwk_addr, endpoint, cluster, tsn, request_payload)
                .await
                .map_err(|error| format!("read reporting config failed: {error}"))?;
            let (header, offset) = crate::zigbee::zcl::frame::ZclFrameHeader::parse(&response.data)
                .map_err(|error| format!("invalid ZCL response: {error}"))?;
            if header.command_id != crate::zigbee::zcl::frame::global::READ_REPORTING_CONFIG_RSP {
                return Err(format!(
                    "unexpected ZCL command 0x{:02X} for reporting read",
                    header.command_id
                ));
            }
            let configs = crate::zigbee::zcl::parse_reporting_config_rsp(&response.data[offset..]);
            for config in &configs {
                if config.status != 0 {
                    continue;
                }
                let record = json!({
                    "cluster": cluster,
                    "attrId": config.attribute,
                    "minRepIntval": config.minimum_interval.unwrap_or(0),
                    "maxRepIntval": config.maximum_interval.unwrap_or(0),
                    "repChange": reportable_change_value(
                        config.data_type.unwrap_or(0),
                        config.reportable_change.as_deref().unwrap_or(&[]),
                    ),
                });
                let mut db = database.lock().await;
                if !db.set_configured_reporting(&device.ieee_addr.as_hex(), endpoint, record) {
                    warn!("Reporting read: cannot persist config for {id} endpoint {endpoint}");
                } else if let Err(error) = db.write() {
                    warn!("Reporting read: cannot write database: {error}");
                }
            }
            let configs: Vec<serde_json::Value> = configs
                .into_iter()
                .map(|config| {
                    json!({
                        "attribute": config.attribute,
                        "attribute_name": external.attribute_name(cluster, config.attribute),
                        "status": config.status,
                        "data_type": config.data_type,
                        "minimum_report_interval": config.minimum_interval,
                        "maximum_report_interval": config.maximum_interval,
                        "reportable_change": config.reportable_change.map(hex::encode),
                    })
                })
                .collect();
            return Ok(json!({
                "id": id,
                "endpoint": endpoint,
                "cluster": cluster,
                "configs": configs,
            }));
        }

        // device/reporting/configure
        let attribute = payload
            .get("attribute")
            .and_then(|attribute| {
                attribute.as_u64().map(|value| value as u16).or_else(|| {
                    attribute
                        .as_str()
                        .and_then(|name| external.attribute_id(cluster, name))
                })
            })
            .ok_or_else(|| "missing or unknown attribute".to_string())?;
        let minimum = payload
            .get("minimum_report_interval")
            .and_then(|value| value.as_u64())
            .ok_or_else(|| "missing minimum_report_interval".to_string())?
            as u16;
        let maximum = payload
            .get("maximum_report_interval")
            .and_then(|value| value.as_u64())
            .ok_or_else(|| "missing maximum_report_interval".to_string())?
            as u16;
        let reportable_change = payload
            .get("reportable_change")
            .and_then(|value| value.as_f64());

        // The bridge payload does not carry a data type; read the current
        // reporting configuration first so the command can be encoded correctly.
        *trans_id = trans_id.wrapping_add(1);
        let read_tsn = *trans_id;
        let read_payload =
            crate::zigbee::zcl::frame::read_reporting_config_payload(read_tsn, &[attribute]);
        let read_response = io
            .read(device.nwk_addr, endpoint, cluster, read_tsn, read_payload)
            .await
            .map_err(|error| format!("cannot discover reporting data type: {error}"))?;
        let (_, offset) = crate::zigbee::zcl::frame::ZclFrameHeader::parse(&read_response.data)
            .map_err(|error| format!("invalid ZCL response: {error}"))?;
        let data_type = crate::zigbee::zcl::parse_reporting_config_rsp(&read_response.data[offset..])
            .into_iter()
            .find(|config| config.attribute == attribute && config.status == 0)
            .and_then(|config| config.data_type)
            .ok_or_else(|| {
                "cannot determine attribute data type; configure reporting is not supported for this attribute yet"
                    .to_string()
            })?;
        let change = encode_reportable_change(data_type, reportable_change)
            .ok_or_else(|| format!("unsupported data type 0x{data_type:02X} for reporting"))?;

        if let Some(coordinator_ieee) = io.coordinator_ieee() {
            if let Err(error) = io
                .bind(
                    device.nwk_addr,
                    device.ieee_addr.0,
                    endpoint,
                    cluster,
                    coordinator_ieee.0,
                )
                .await
            {
                return Err(format!("cannot bind reporting cluster: {error}"));
            }
        } else {
            return Err("coordinator IEEE address unknown; cannot bind reporting".to_string());
        }

        let mut command_payload = crate::zigbee::zcl::frame::ZclFrameHeader::encode_global(
            (*trans_id).wrapping_add(1),
            crate::zigbee::zcl::frame::global::CONFIGURE_REPORTING,
        );
        command_payload.push(0x00); // direction
        command_payload.extend_from_slice(&attribute.to_le_bytes());
        command_payload.push(data_type);
        command_payload.extend_from_slice(&minimum.to_le_bytes());
        command_payload.extend_from_slice(&maximum.to_le_bytes());
        command_payload.extend_from_slice(&change);
        *trans_id = trans_id.wrapping_add(1);
        let tsn = *trans_id;
        let response = io
            .read(device.nwk_addr, endpoint, cluster, tsn, command_payload)
            .await
            .map_err(|error| format!("configure reporting failed: {error}"))?;
        let (header, offset) = crate::zigbee::zcl::frame::ZclFrameHeader::parse(&response.data)
            .map_err(|error| format!("invalid ZCL response: {error}"))?;
        if header.command_id != crate::zigbee::zcl::frame::global::CONFIGURE_REPORTING_RSP {
            return Err(format!(
                "unexpected ZCL command 0x{:02X} for configure reporting",
                header.command_id
            ));
        }
        let status = response.data.get(offset).copied().unwrap_or(0xFF);
        if status != 0 {
            return Err(format!("configure reporting status 0x{status:02X}"));
        }
        {
            let record = json!({
                "cluster": cluster,
                "attrId": attribute,
                "minRepIntval": minimum,
                "maxRepIntval": maximum,
                "repChange": reportable_change.unwrap_or(0.0),
            });
            let mut db = database.lock().await;
            if !db.set_configured_reporting(&device.ieee_addr.as_hex(), endpoint, record) {
                warn!("Reporting configure: cannot persist config for {id} endpoint {endpoint}");
            } else if let Err(error) = db.write() {
                warn!("Reporting configure: cannot write database: {error}");
            }
        }
        Ok(json!({
            "id": id,
            "endpoint": endpoint,
            "cluster": cluster,
            "attribute": attribute,
            "minimum_report_interval": minimum,
            "maximum_report_interval": maximum,
            "reportable_change": reportable_change,
        }))
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_get(
        devices: &DeviceRegistry,
        external: &ExternalConverter,
        coord: &CoordinatorHandle,
        mqtt: &MqttBridge,
        output: OutputMode,
        trans_id: &mut u8,
        name: &str,
        payload: &serde_json::Value,
    ) {
        const DEFAULT_KEYS: &[&str] = &[
            "state",
            "brightness",
            "color_temp",
            "color",
            "temperature",
            "humidity",
            "illuminance",
            "battery",
            "battery_voltage",
            "occupancy",
            "contact",
        ];

        let Some(dev) = devices.find_by_name(name) else {
            warn!("Get command for unknown device: {name}");
            return;
        };
        if dev.nwk_addr == 0 || !dev.interview_complete {
            warn!(
                "Get for {name} skipped: device not available (nwk=0x{:04X}, interviewed={})",
                dev.nwk_addr, dev.interview_complete
            );
            return;
        }
        let nwk_addr = dev.nwk_addr;
        let endpoints = dev.endpoints.clone();
        let zone_type = dev.zone_type;
        let model = dev.model.clone().unwrap_or_default();
        let info = crate::converters::external::ExternalDeviceInfo::from_device(&dev);
        let cached_state = dev.state.clone();
        drop(dev);

        let requested: Vec<String> = payload
            .as_object()
            .map(|object| object.keys().cloned().collect())
            .unwrap_or_default();
        let wanted: Vec<&str> = if requested.is_empty() {
            DEFAULT_KEYS.to_vec()
        } else {
            requested.iter().map(String::as_str).collect()
        };

        let targets = build_get_targets(&endpoints, &wanted);
        if targets.is_empty() {
            debug!("Get for {name}: no readable attributes for the requested keys");
            return;
        }

        let inner = Arc::clone(&coord.inner);
        let (mut merged, mut failed) = execute_get_reads(
            name,
            &targets,
            trans_id,
            zone_type,
            move |endpoint, cluster_id, tsn, payload| {
                let inner = Arc::clone(&inner);
                async move {
                    inner
                        .lock()
                        .await
                        .request_zcl(
                            nwk_addr,
                            endpoint,
                            cluster_id,
                            tsn,
                            payload,
                            GET_ZCL_TIMEOUT,
                            GET_ZCL_ATTEMPTS,
                        )
                        .await
                }
            },
        )
        .await;

        // Fallback for keys the native map does not cover: execute the upstream
        // definition's convertGet and perform the reads it requests.
        if !requested.is_empty() {
            let converter_keys: Vec<String> = requested
                .iter()
                .filter(|key| read_target(key).is_none())
                .cloned()
                .collect();
            if !converter_keys.is_empty() {
                let reads = external.convert_get(&model, &converter_keys, &info, &cached_state);
                for read in reads {
                    let Some(endpoint) = find_ep_with_cluster(&endpoints, read.cluster_id) else {
                        debug!(
                            "Get for {name}: convertGet requested cluster 0x{:04X} not on any endpoint",
                            read.cluster_id
                        );
                        continue;
                    };
                    *trans_id = trans_id.wrapping_add(1);
                    let tsn = *trans_id;
                    let request =
                        crate::zigbee::zcl::frame::read_attributes_payload(tsn, &read.attributes);
                    match coord
                        .request_zcl(
                            nwk_addr,
                            endpoint,
                            read.cluster_id,
                            tsn,
                            request,
                            GET_ZCL_TIMEOUT,
                            GET_ZCL_ATTEMPTS,
                        )
                        .await
                    {
                        Ok(response) => {
                            match crate::zigbee::zcl::frame::ZclFrameHeader::parse(&response.data) {
                                Ok((_, offset)) => {
                                    let reports = crate::zigbee::zcl::parse_read_attr_rsp(
                                        &response.data[offset..],
                                    );
                                    let pairs: Vec<(u16, serde_json::Value)> = reports
                                        .iter()
                                        .filter_map(|report| {
                                            attribute_json(&report.value)
                                                .map(|value| (report.attr_id, value))
                                        })
                                        .collect();
                                    if let Some(values) = external.convert_from_zigbee(
                                        &model,
                                        read.cluster_id,
                                        "readResponse",
                                        &pairs,
                                        &info,
                                        &cached_state,
                                    ) {
                                        for (key, value) in values {
                                            merged.insert(key, value);
                                        }
                                    }
                                }
                                Err(error) => {
                                    failed += 1;
                                    warn!("Get for {name}: convertGet response header invalid: {error}");
                                }
                            }
                        }
                        Err(error) => {
                            failed += 1;
                            warn!(
                                "Get for {name}: convertGet read cluster 0x{:04X} failed: {error}",
                                read.cluster_id
                            );
                        }
                    }
                }
            }
        }

        if merged.is_empty() {
            // Do not republish cached state as if the device had answered.
            warn!("Get for {name}: no device response (failed reads: {failed})");
            return;
        }
        if let Some(mut dev) = devices.get_mut_by_nwk(nwk_addr) {
            for (key, value) in merged {
                dev.state.insert(key, value);
            }
            dev.state.insert("last_seen".into(), json!(now_iso8601()));
            let state = serde_json::Value::Object(dev.state.clone());
            let friendly_name = dev.friendly_name.clone();
            drop(dev);
            Self::publish_state(mqtt, output, &friendly_name, &state).await;
            let _ = mqtt.publish_availability(&friendly_name, true).await;
        }
    }

    async fn handle_set(
        devices: &DeviceRegistry,
        coord: &CoordinatorHandle,
        mqtt: &MqttBridge,
        output: OutputMode,
        trans_id: &mut u8,
        name: &str,
        payload: &serde_json::Value,
    ) {
        let dev = match devices.find_by_name(name) {
            Some(d) => d,
            None => {
                warn!("Set command for unknown device: {name}");
                return;
            }
        };

        if dev.nwk_addr == 0 || !dev.interview_complete {
            warn!(
                "Device {name} not yet available on network (nwk=0x{:04X}, interviewed={})",
                dev.nwk_addr, dev.interview_complete
            );
            return;
        }

        let nwk_addr = dev.nwk_addr;
        let endpoints = dev.endpoints.clone();

        // Build optimistic state from the command payload
        let mut optimistic = serde_json::Map::new();

        // Handle state (on/off)
        if let Some(state_val) = payload.get("state") {
            let state_str = state_val.as_str().unwrap_or("");
            if let Some(ep) = find_ep_with_cluster(&endpoints, 0x0006) {
                if let Some(zcl_payload) = on_off::set_state_payload(*trans_id, state_str) {
                    *trans_id = trans_id.wrapping_add(1);
                    if let Err(e) = coord
                        .send_zcl(nwk_addr, ep, 0x0006, *trans_id, zcl_payload)
                        .await
                    {
                        warn!("Failed to send on/off command: {e}");
                    }
                    optimistic.insert("state".into(), json!(state_str.to_uppercase()));
                }
            }
        }

        // Handle brightness
        if let Some(brightness) = payload.get("brightness").and_then(|v| v.as_u64()) {
            if let Some(ep) = find_ep_with_cluster(&endpoints, 0x0008) {
                let lvl = brightness.min(254) as u8;
                let transition = transition_time(payload);
                let zcl_payload = level::move_to_level_payload(*trans_id, lvl, transition);
                *trans_id = trans_id.wrapping_add(1);
                if let Err(e) = coord
                    .send_zcl(nwk_addr, ep, 0x0008, *trans_id, zcl_payload)
                    .await
                {
                    warn!("Failed to send level command: {e}");
                }
                optimistic.insert("brightness".into(), json!(lvl));
                // Move-to-level-with-on/off implies state ON
                if !optimistic.contains_key("state") {
                    optimistic.insert("state".into(), json!("ON"));
                }
            }
        }

        // Handle color_temp
        if let Some(ct) = payload.get("color_temp").and_then(|v| v.as_u64()) {
            if let Some(ep) = find_ep_with_cluster(&endpoints, 0x0300) {
                let transition = transition_time(payload);
                let zcl_payload =
                    color::move_to_color_temp_payload(*trans_id, ct as u16, transition);
                *trans_id = trans_id.wrapping_add(1);
                if let Err(e) = coord
                    .send_zcl(nwk_addr, ep, 0x0300, *trans_id, zcl_payload)
                    .await
                {
                    warn!("Failed to send color temp command: {e}");
                }
                optimistic.insert("color_temp".into(), json!(ct));
                optimistic.insert("color_mode".into(), json!("color_temp"));
            }
        }

        // Handle color object
        if let Some(color_obj) = payload.get("color").and_then(|v| v.as_object()) {
            if let Some(ep) = find_ep_with_cluster(&endpoints, 0x0300) {
                let transition = transition_time(payload);

                if let (Some(x), Some(y)) = (
                    color_obj.get("x").and_then(|v| v.as_f64()),
                    color_obj.get("y").and_then(|v| v.as_f64()),
                ) {
                    let zcl_payload = color::move_to_color_xy_payload(*trans_id, x, y, transition);
                    *trans_id = trans_id.wrapping_add(1);
                    if let Err(e) = coord
                        .send_zcl(nwk_addr, ep, 0x0300, *trans_id, zcl_payload)
                        .await
                    {
                        warn!("Failed to send color XY command: {e}");
                    }
                    optimistic.insert("color".into(), json!({"x": x, "y": y}));
                    optimistic.insert("color_mode".into(), json!("xy"));
                } else if let (Some(h), Some(s)) = (
                    color_obj.get("hue").and_then(|v| v.as_f64()),
                    color_obj.get("saturation").and_then(|v| v.as_f64()),
                ) {
                    let zcl_hue = ((h / 360.0) * 254.0).round() as u8;
                    let zcl_sat = ((s / 100.0) * 254.0).round() as u8;
                    let zcl_payload =
                        color::move_to_hue_sat_payload(*trans_id, zcl_hue, zcl_sat, transition);
                    *trans_id = trans_id.wrapping_add(1);
                    if let Err(e) = coord
                        .send_zcl(nwk_addr, ep, 0x0300, *trans_id, zcl_payload)
                        .await
                    {
                        warn!("Failed to send color HS command: {e}");
                    }
                    optimistic.insert("color".into(), json!({"hue": h, "saturation": s}));
                    optimistic.insert("color_mode".into(), json!("hs"));
                }
            }
        }

        // Handle sensor_type (Lytko thermostat, manuf 0x7777)
        if let Some(st) = payload.get("sensor_type").and_then(|v| v.as_str()) {
            let types = [
                "3.3 кОм",
                "5 кОм",
                "6.8 кОм",
                "10 кОм",
                "12 кОм",
                "14.8 кОм",
                "15 кОм",
                "20 кОм",
                "33 кОм",
                "47 кОм",
            ];
            if let Some(idx) = types.iter().position(|t| *t == st) {
                if let Some(ep) = find_ep_with_cluster(&endpoints, 0x0201) {
                    let zcl_payload = zcl::write::write_attributes_payload(
                        *trans_id,
                        Some(0x7777),
                        &[(0xFF00, 0x30, &[idx as u8])],
                    );
                    *trans_id = trans_id.wrapping_add(1);
                    if let Err(e) = coord
                        .send_zcl(nwk_addr, ep, 0x0201, *trans_id, zcl_payload)
                        .await
                    {
                        warn!("Failed to write sensor_type: {e}");
                    }
                    optimistic.insert("sensor_type".into(), json!(st));
                }
            }
        }

        // Handle hysteresis (Lytko thermostat, manuf 0x7777)
        if let Some(hyst) = payload.get("hysteresis").and_then(|v| v.as_f64()) {
            let mut raw = (hyst * 10.0 / 5.0).round() as i64 * 5;
            raw = raw.clamp(5, 30);
            if let Some(ep) = find_ep_with_cluster(&endpoints, 0x0201) {
                let zcl_payload = zcl::write::write_attributes_payload(
                    *trans_id,
                    Some(0x7777),
                    &[(0xFF01, 0x20, &[raw as u8])],
                );
                *trans_id = trans_id.wrapping_add(1);
                if let Err(e) = coord
                    .send_zcl(nwk_addr, ep, 0x0201, *trans_id, zcl_payload)
                    .await
                {
                    warn!("Failed to write hysteresis: {e}");
                }
                optimistic.insert("hysteresis".into(), json!(raw as f64 / 10.0));
            }
        }

        // Handle correction (Lytko thermostat, manuf 0x7777)
        if let Some(corr) = payload.get("correction").and_then(|v| v.as_f64()) {
            let mut deg = corr.round() as i64;
            deg = deg.clamp(-5, 5);
            let raw = (deg * 10) as i8;
            if let Some(ep) = find_ep_with_cluster(&endpoints, 0x0201) {
                let zcl_payload = zcl::write::write_attributes_payload(
                    *trans_id,
                    Some(0x7777),
                    &[(0xFF02, 0x28, &[raw as u8])],
                );
                *trans_id = trans_id.wrapping_add(1);
                if let Err(e) = coord
                    .send_zcl(nwk_addr, ep, 0x0201, *trans_id, zcl_payload)
                    .await
                {
                    warn!("Failed to write correction: {e}");
                }
                optimistic.insert("correction".into(), json!(deg));
            }
        }

        // Handle switch_type (Lytko switch channel)
        if let Some(st) = payload.get("switch_type").and_then(|v| v.as_str()) {
            let types = ["Кнопка", "Выключатель", "Переключатель"];
            if let Some(idx) = types.iter().position(|t| *t == st) {
                if let Some(ep) = find_ep_with_cluster(&endpoints, 0x0007) {
                    let zcl_payload = zcl::write::write_attributes_payload(
                        *trans_id,
                        None,
                        &[(0x0000, 0x30, &[idx as u8])],
                    );
                    *trans_id = trans_id.wrapping_add(1);
                    if let Err(e) = coord
                        .send_zcl(nwk_addr, ep, 0x0007, *trans_id, zcl_payload)
                        .await
                    {
                        warn!("Failed to write switch_type: {e}");
                    }
                    optimistic.insert("switch_type".into(), json!(st));
                }
            }
        }

        // Handle decoupled (Lytko switch channel, manuf 0x7777)
        if let Some(dec) = payload.get("decoupled") {
            let on = match dec {
                serde_json::Value::Bool(b) => *b,
                serde_json::Value::String(s) => s.to_uppercase() == "ON",
                _ => false,
            };
            if let Some(ep) = find_ep_with_cluster(&endpoints, 0x0007) {
                let zcl_payload = zcl::write::write_attributes_payload(
                    *trans_id,
                    Some(0x7777),
                    &[(0xFF10, 0x10, &[if on { 1u8 } else { 0u8 }])],
                );
                *trans_id = trans_id.wrapping_add(1);
                if let Err(e) = coord
                    .send_zcl(nwk_addr, ep, 0x0007, *trans_id, zcl_payload)
                    .await
                {
                    warn!("Failed to write decoupled: {e}");
                }
                optimistic.insert("decoupled".into(), json!(if on { "ON" } else { "OFF" }));
            }
        }

        // Handle recovery (Lytko, manuf 0x7777, EP1, hvacUserInterfaceCfg 0x0204)
        if let Some(rec) = payload.get("recovery") {
            let on = match rec {
                serde_json::Value::Bool(b) => *b,
                serde_json::Value::String(s) => s.to_uppercase() == "ON",
                _ => false,
            };
            if on {
                if let Some(ep) = endpoints.iter().map(|e| e.endpoint).min() {
                    let zcl_payload = zcl::write::write_attributes_payload(
                        *trans_id,
                        Some(0x7777),
                        &[(0x4901, 0x30, &[1u8])],
                    );
                    *trans_id = trans_id.wrapping_add(1);
                    if let Err(e) = coord
                        .send_zcl(nwk_addr, ep, 0x0204, *trans_id, zcl_payload)
                        .await
                    {
                        warn!("Failed to write recovery: {e}");
                    }
                    optimistic.insert("recovery".into(), json!("OFF"));
                }
            }
        }

        // Publish optimistic state immediately (z2m behavior)
        if !optimistic.is_empty() {
            // Merge with existing device state
            if let Some(mut dev) = devices.get_mut_by_nwk(nwk_addr) {
                dev.merge_state(optimistic.clone());
                dev.state.insert("last_seen".into(), json!(now_iso8601()));
            }
            // Publish the full merged state
            if let Some(dev) = devices.find_by_name(name) {
                let full_state = serde_json::Value::Object(dev.state);
                Self::publish_state(mqtt, output, name, &full_state).await;
            }
        }
    }

    /// Remove momentary button-press fields (`action` and `action_*`) from a
    /// device's *retained* state. Real device state (brightness, battery,
    /// contact, ...) should keep accumulating via `merge_state`, but an
    /// `action` is a one-off event -- without this, a stale `action` (and
    /// its companion `action_*` keys) from one button press would linger and
    /// get republished, mixed in, on every later unrelated event for that
    /// device. Call this *after* building the JSON to publish for the
    /// current message, so the action still appears in that one publish.
    fn strip_transient_action_fields(state: &mut serde_json::Map<String, serde_json::Value>) {
        state.retain(|k, _| k != "action" && !k.starts_with("action_"));
    }

    /// Publish a device's state per `advanced.output` (z2m's
    /// `Controller.publishEntityState`): the merged JSON message, each state
    /// key as its own flattened raw-value subtopic, or both. All state
    /// publishes should go through this rather than calling
    /// `mqtt.publish_device_state` directly, so every call site respects the
    /// configured output mode uniformly.
    async fn publish_state(
        mqtt: &MqttBridge,
        output: OutputMode,
        friendly_name: &str,
        state: &serde_json::Value,
    ) {
        let (publish_json, attributes) = Self::plan_state_publishes(output, state);

        if publish_json {
            if let Err(e) = mqtt.publish_device_state(friendly_name, state).await {
                warn!("Failed to publish device state: {e}");
            }
        }
        for (key_path, value) in attributes {
            if let Err(e) = mqtt
                .publish_device_attribute(friendly_name, &key_path, &value)
                .await
            {
                warn!("Failed to publish attribute {key_path} for {friendly_name}: {e}");
            }
        }
    }

    /// Decides *what* `publish_state` should publish for a given output mode,
    /// split out from the actual MQTT I/O above purely so this branching is
    /// unit-testable without a live broker connection: returns whether the
    /// merged-JSON message should be published, and the flattened
    /// `(key_path, value)` attribute pairs (empty unless `output` is
    /// `Attribute`/`AttributeAndJson`).
    fn plan_state_publishes(
        output: OutputMode,
        state: &serde_json::Value,
    ) -> (bool, Vec<(String, String)>) {
        let publish_json = matches!(output, OutputMode::Json | OutputMode::AttributeAndJson);
        let attributes = if matches!(output, OutputMode::Attribute | OutputMode::AttributeAndJson) {
            state
                .as_object()
                .map(flatten_attribute_output)
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        (publish_json, attributes)
    }

    /// Handle basic cluster (0x0000) Read Attributes Response to update device metadata.
    fn handle_basic_cluster_response(devices: &DeviceRegistry, src_addr: u16, data: &[u8]) {
        if let Ok(Some(zcl_msg)) = zcl::parse_message(0x0000, data, None, None) {
            if let Some(mut dev) = devices.get_mut_by_nwk(src_addr) {
                for (key, value) in &zcl_msg.values {
                    match key.as_str() {
                        "manufacturer" => {
                            if let Some(s) = value.as_str() {
                                dev.manufacturer = Some(s.to_string());
                            }
                        }
                        "model" => {
                            if let Some(s) = value.as_str() {
                                dev.model = Some(s.to_string());
                            }
                        }
                        "power_source" => {
                            if let Some(s) = value.as_str() {
                                dev.power_source = Some(s.to_string());
                            }
                        }
                        "sw_build_id" => {
                            if let Some(s) = value.as_str() {
                                dev.sw_build_id = Some(s.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Handle IAS Zone (0x0500) Read Attributes Response to learn ZoneType,
    /// which determines how zone status reports are classified (contact,
    /// occupancy, smoke, water_leak, ...). No-op for any other ZCL command.
    fn handle_ias_zone_type_response(devices: &DeviceRegistry, src_addr: u16, data: &[u8]) {
        if let Ok(Some(zone_type)) = zcl::extract_ias_zone_type(data) {
            if let Some(mut dev) = devices.get_mut_by_nwk(src_addr) {
                dev.zone_type = Some(zone_type);
            }
        }
    }

    async fn publish_device_list(&self, mqtt: &MqttBridge) {
        Self::publish_device_list_static(&self.devices, mqtt).await;
    }

    async fn publish_device_list_static(devices: &DeviceRegistry, mqtt: &MqttBridge) {
        let list: Vec<_> = devices
            .all_devices()
            .iter()
            .map(|d| d.to_z2m_device_json())
            .collect();
        if let Err(e) = mqtt.publish_bridge_devices(&json!(list)).await {
            warn!("Failed to publish device list: {e}");
        }
    }

    /// Spawn a background retry loop for a device's ZDO interview.
    ///
    /// A sleepy battery-powered end device (e.g. a remote control) may be
    /// asleep when the initial ZDO_ACTIVE_EP_REQ goes out over the air and
    /// simply never see it -- the SREQ/SRSP exchange with the coordinator's
    /// own radio still succeeds (it only confirms the request was queued),
    /// so nothing here times out on its own. Without a retry, the device's
    /// `interview_complete` flag would then never flip true. This polls
    /// `interview_complete` and re-sends the request a bounded number of
    /// times with a delay generous enough for such devices to wake up and
    /// respond, stopping as soon as the real ActiveEpRsp/SimpleDescRsp event
    /// handling (in the main loop) marks the interview complete.
    fn spawn_interview_retry(
        devices: Arc<DeviceRegistry>,
        znp: Arc<Mutex<ZnpHandle>>,
        ieee: IeeeAddr,
        nwk_addr: u16,
    ) {
        const MAX_ATTEMPTS: u32 = 3;
        const RETRY_DELAY: Duration = Duration::from_secs(20);

        tokio::spawn(async move {
            for attempt in 1..=MAX_ATTEMPTS {
                if devices
                    .get_by_ieee(&ieee)
                    .is_some_and(|d| d.interview_complete)
                {
                    return;
                }
                // Upstream interview starts with Node Descriptor, then ActiveEp
                if let Err(e) = znp.lock().await.request_node_desc(nwk_addr).await {
                    warn!(
                        "Interview attempt {attempt}/{MAX_ATTEMPTS} for {ieee}: \
                         failed to request node descriptor: {e}"
                    );
                    // Fallback to ActiveEp if NodeDesc fails (quirks)
                    if let Err(e2) = znp.lock().await.request_active_eps(nwk_addr).await {
                        warn!(
                            "Interview attempt {attempt}/{MAX_ATTEMPTS} for {ieee}: \
                             failed to request active endpoints: {e2}"
                        );
                    }
                }
                tokio::time::sleep(RETRY_DELAY).await;
                if devices
                    .get_by_ieee(&ieee)
                    .is_some_and(|d| d.interview_complete)
                {
                    return;
                }
            }
            warn!(
                "Interview for {ieee} did not complete after {MAX_ATTEMPTS} attempts -- \
                 device may be asleep or out of range. It will retry automatically on its \
                 next join/announce."
            );
        });
    }
}

/// Keys BasicCluster::process_reports emits, mirroring the dedicated Device
/// fields handle_basic_cluster_response() populates from the same data.
const BASIC_CLUSTER_METADATA_KEYS: [&str; 4] =
    ["manufacturer", "model", "power_source", "sw_build_id"];

/// Merge native and upstream-converter values for one frame.
///
/// The device definition is authoritative for keys it produces; native values
/// remain for keys the definition does not cover. Callers publish the result
/// exactly once.
fn merge_converted_values(
    native: serde_json::Map<String, serde_json::Value>,
    external: Option<serde_json::Map<String, serde_json::Value>>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut merged = native;
    if let Some(external) = external {
        for (key, value) in external {
            merged.insert(key, value);
        }
    }
    merged
}

const GET_ZCL_TIMEOUT: Duration = Duration::from_secs(3);
const GET_ZCL_ATTEMPTS: u8 = 2;

/// Execute one read per endpoint/cluster and merge the parsed responses.
///
/// The read closure is the seam used by the broker-free test; production passes
/// the coordinator's ZCL transaction service.
async fn execute_get_reads<F, Fut>(
    name: &str,
    targets: &std::collections::BTreeMap<(u8, u16), Vec<u16>>,
    trans_id: &mut u8,
    zone_type: Option<u16>,
    mut read: F,
) -> (serde_json::Map<String, serde_json::Value>, usize)
where
    F: FnMut(u8, u16, u8, Vec<u8>) -> Fut,
    Fut: std::future::Future<
        Output = crate::error::Result<crate::coordinator::znp::transaction::ZclIncoming>,
    >,
{
    let mut merged = serde_json::Map::new();
    let mut failed = 0;
    for (&(endpoint, cluster_id), attributes) in targets {
        *trans_id = trans_id.wrapping_add(1);
        let tsn = *trans_id;
        let payload = crate::zigbee::zcl::frame::read_attributes_payload(tsn, attributes);
        match read(endpoint, cluster_id, tsn, payload).await {
            Ok(response) => {
                match zcl::parse_message(cluster_id, &response.data, zone_type, None) {
                    Ok(Some(message)) => {
                        for (key, value) in message.values {
                            merged.insert(key, value);
                        }
                    }
                    Ok(None) => {
                        debug!("Get for {name}: cluster 0x{cluster_id:04X} response had no mapped values");
                    }
                    Err(error) => {
                        failed += 1;
                        warn!("Get for {name}: cannot parse cluster 0x{cluster_id:04X}: {error}");
                    }
                }
            }
            Err(error) => {
                failed += 1;
                warn!("Get for {name}: read cluster 0x{cluster_id:04X} failed: {error}");
            }
        }
    }
    (merged, failed)
}

#[derive(Debug)]
struct OtaRequestOutcome {
    data: serde_json::Value,
    state_update: Option<(String, serde_json::Value)>,
}

fn ota_update_state(progress: f64, completed: bool) -> serde_json::Value {
    json!({
        "state": if completed { "idle" } else { "updating" },
        "progress": (progress * 100.0).round() as i64,
    })
}

fn select_index_from_payload(
    base_dir: &std::path::Path,
    device: &Device,
    payload: &serde_json::Value,
) -> std::result::Result<Option<(crate::ota::SelectedImage, Vec<u8>)>, String> {
    let Some(index) = payload.get("index").and_then(|value| value.as_str()) else {
        return Ok(None);
    };
    let index_path = if std::path::Path::new(index).is_absolute() {
        std::path::PathBuf::from(index)
    } else {
        base_dir.join(index)
    };
    let manufacturer_code = payload
        .get("manufacturer_code")
        .and_then(|value| value.as_u64())
        .map(|value| value as u16)
        .or(device.manufacturer_id)
        .ok_or_else(|| "index selection requires a manufacturer code".to_string())?;
    let image_type = payload
        .get("image_type")
        .and_then(|value| value.as_u64())
        .map(|value| value as u16)
        .ok_or_else(|| "index selection requires image_type".to_string())?;
    let file_version = payload
        .get("file_version")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as u32;
    let hardware_version = payload
        .get("hardware_version")
        .and_then(|value| value.as_u64())
        .map(|value| value as u16);
    let identity = crate::ota::OtaQueryIdentity {
        manufacturer_code,
        image_type,
        file_version,
        hardware_version,
        model_id: device.model.as_deref(),
        manufacturer_name: device.manufacturer.as_deref(),
    };
    let (selected, bytes) = crate::ota::select_local_image(&index_path, base_dir, &identity)?;
    Ok(Some((selected, bytes)))
}

async fn handle_ota_request(
    ota: &Mutex<crate::ota::OtaManager>,
    base_dir: &std::path::Path,
    devices: &DeviceRegistry,
    request: &str,
    payload: &serde_json::Value,
) -> std::result::Result<OtaRequestOutcome, String> {
    let suffix = request
        .strip_prefix("device/ota_update/")
        .ok_or_else(|| "invalid OTA request".to_string())?;
    let id = payload
        .get("id")
        .and_then(|value| value.as_str())
        .or_else(|| payload.as_str())
        .unwrap_or("");
    let device =
        find_device_for_id(devices, id).ok_or_else(|| format!("Device '{id}' does not exist"))?;
    let expected_sha512 = payload.get("sha512").and_then(|value| value.as_str());
    let image_bytes = payload
        .get("hex")
        .and_then(|hex| hex.get("data"))
        .and_then(|data| data.as_str())
        .and_then(|data| hex::decode(data).ok());
    match suffix {
        "check" | "check/downgrade" => {
            let (info, source, release_notes) =
                match select_index_from_payload(base_dir, &device, payload)? {
                    Some((selected, _bytes)) => {
                        (selected.info, Some(selected.url), selected.release_notes)
                    }
                    None => {
                        let bytes = image_bytes
                            .ok_or_else(|| "missing or invalid hex image".to_string())?;
                        let info = ota.lock().await.check_checked(&bytes, expected_sha512)?;
                        (info, None, None)
                    }
                };
            Ok(OtaRequestOutcome {
                data: json!({
                    "id": id,
                    "update_available": true,
                    "downgrade": suffix.ends_with("downgrade"),
                    "source": source,
                    "release_notes": release_notes,
                    "manufacturer_code": info.manufacturer_code,
                    "image_type": info.image_type,
                    "file_version": info.file_version,
                    "image_size": info.size,
                }),
                state_update: None,
            })
        }
        "update" | "update/downgrade" => {
            let (info, source, release_notes) =
                match select_index_from_payload(base_dir, &device, payload)? {
                    Some((selected, bytes)) => {
                        let info = ota.lock().await.start(&device.ieee_addr.as_hex(), &bytes)?;
                        (info, Some(selected.url), selected.release_notes)
                    }
                    None => {
                        let bytes = image_bytes
                            .ok_or_else(|| "missing or invalid hex image".to_string())?;
                        let info = ota.lock().await.start_checked(
                            &device.ieee_addr.as_hex(),
                            &bytes,
                            expected_sha512,
                        )?;
                        (info, None, None)
                    }
                };
            Ok(OtaRequestOutcome {
                data: json!({
                    "id": id,
                    "manufacturer_code": info.manufacturer_code,
                    "image_type": info.image_type,
                    "file_version": info.file_version,
                    "image_size": info.size,
                    "source": source,
                    "release_notes": release_notes,
                }),
                state_update: Some((device.ieee_addr.as_hex(), ota_update_state(0.0, false))),
            })
        }
        "update/abort" | "unschedule" => {
            let aborted = ota.lock().await.abort(&device.ieee_addr.as_hex());
            Ok(OtaRequestOutcome {
                data: json!({"id": id, "aborted": aborted}),
                state_update: Some((device.ieee_addr.as_hex(), ota_update_state(1.0, true))),
            })
        }
        _ => Err(format!("Unsupported OTA request {suffix}")),
    }
}

async fn send_group_command(
    coord: &CoordinatorHandle,
    nwk_addr: u16,
    endpoint: u8,
    tsn: u8,
    payload: Vec<u8>,
    expected_response: Option<u8>,
) -> std::result::Result<crate::zigbee::zcl::clusters::groups::GroupResponse, String> {
    let response = coord
        .request_zcl_expected(
            nwk_addr,
            endpoint,
            crate::zigbee::zcl::clusters::groups::CLUSTER_ID,
            tsn,
            payload,
            Duration::from_secs(3),
            2,
            expected_response,
        )
        .await
        .map_err(|error| format!("group command failed: {error}"))?;
    crate::zigbee::zcl::clusters::groups::parse_response(&response.data)
        .ok_or_else(|| "invalid groups cluster response".to_string())
}

/// Build the MQTT bridge response envelope for reporting requests.
fn reporting_mqtt_response(
    result: std::result::Result<serde_json::Value, String>,
    transaction: Option<&str>,
) -> serde_json::Value {
    let mut response = match result {
        Ok(data) => json!({"status": "ok", "data": data}),
        Err(error) => json!({"status": "error", "data": {}, "error": error}),
    };
    if let Some(transaction) = transaction {
        response["transaction"] = json!(transaction);
    }
    response
}

fn reportable_change_value(data_type: u8, bytes: &[u8]) -> serde_json::Value {
    match data_type {
        0x39 if bytes.len() >= 4 => {
            json!(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64)
        }
        0x3A if bytes.len() >= 8 => json!(f64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]
        ])),
        _ => {
            let mut raw = [0u8; 8];
            for (index, byte) in bytes.iter().take(8).enumerate() {
                raw[index] = *byte;
            }
            json!(u64::from_le_bytes(raw))
        }
    }
}

fn find_device_for_id(devices: &DeviceRegistry, id: &str) -> Option<Device> {
    devices.find_by_name(id).or_else(|| {
        IeeeAddr::from_hex(id)
            .and_then(|address| devices.get_by_ieee(&address).map(|dev| dev.clone()))
    })
}

fn encode_reportable_change(data_type: u8, value: Option<f64>) -> Option<Vec<u8>> {
    let size = crate::zigbee::zcl::reportable_change_size(data_type)?;
    // Reportable change is only present for analog types in ZCL. For discrete
    // types return an empty change field. Half-precision floats are not
    // encoded rather than writing an incorrect value.
    let analog = matches!(data_type, 0x20..=0x2B | 0x38..=0x3A);
    if !analog {
        return Some(Vec::new());
    }
    if data_type == 0x38 {
        return None;
    }
    let value = value.unwrap_or(0.0);
    let mut bytes = Vec::with_capacity(size);
    match data_type {
        0x39 => bytes.extend_from_slice(&(value as f32).to_le_bytes()),
        0x3A => bytes.extend_from_slice(&value.to_le_bytes()),
        _ => {
            let raw = value as i64;
            let raw_bytes = raw.to_le_bytes();
            bytes.extend_from_slice(&raw_bytes[..size]);
        }
    }
    Some(bytes)
}

/// Combine requested keys into one attribute read per endpoint/cluster.
fn build_get_targets(
    endpoints: &[EndpointDesc],
    wanted: &[&str],
) -> std::collections::BTreeMap<(u8, u16), Vec<u16>> {
    let mut targets: std::collections::BTreeMap<(u8, u16), Vec<u16>> =
        std::collections::BTreeMap::new();
    for key in wanted {
        let Some((cluster_id, attributes)) = read_target(key) else {
            continue;
        };
        let Some(endpoint) = find_ep_with_cluster(endpoints, cluster_id) else {
            continue;
        };
        let entry = targets.entry((endpoint, cluster_id)).or_default();
        for attribute in attributes {
            if !entry.contains(attribute) {
                entry.push(*attribute);
            }
        }
    }
    targets
}

/// Attributes to read for a native state key.
fn read_target(key: &str) -> Option<(u16, &'static [u16])> {
    Some(match key {
        "state" => (0x0006, &[0x0000][..]),
        "brightness" => (0x0008, &[0x0000][..]),
        "color" => (0x0300, &[0x0003, 0x0004, 0x0007, 0x0008][..]),
        "color_temp" => (0x0300, &[0x0007][..]),
        "temperature" => (0x0402, &[0x0000][..]),
        "humidity" => (0x0405, &[0x0000][..]),
        "illuminance" => (0x0400, &[0x0000][..]),
        "battery" => (0x0001, &[0x0020][..]),
        "battery_voltage" => (0x0001, &[0x0021][..]),
        "occupancy" => (0x0406, &[0x0000][..]),
        "contact" | "tamper" => (0x0500, &[0x0002][..]),
        _ => return None,
    })
}

fn find_ep_with_cluster(endpoints: &[EndpointDesc], cluster_id: u16) -> Option<u8> {
    endpoints
        .iter()
        .find(|e| e.input_clusters.contains(&cluster_id))
        .map(|e| e.endpoint)
}

fn transition_time(payload: &serde_json::Value) -> u16 {
    payload
        .get("transition")
        .and_then(|v| v.as_f64())
        .map(|s| (s.max(0.0) * 10.0).min(65535.0) as u16)
        .unwrap_or(0)
}

/// Convert a parsed ZCL attribute value to JSON for the external-converter
/// bridge. `Bytes`/`Invalid` have no meaningful JSON form for converters and
/// are skipped (documented in CONVERTER-external.md).
fn attribute_json(
    value: &crate::zigbee::zcl::attribute::AttributeValue,
) -> Option<serde_json::Value> {
    use crate::zigbee::zcl::attribute::AttributeValue as V;
    match value {
        V::Bool(b) => Some(serde_json::Value::Bool(*b)),
        V::U8(n) => Some(json!(*n)),
        V::U16(n) => Some(json!(*n)),
        V::U24(n) => Some(json!(*n)),
        V::U32(n) => Some(json!(*n)),
        V::I8(n) => Some(json!(*n)),
        V::I16(n) => Some(json!(*n)),
        V::I24(n) => Some(json!(*n)),
        V::I32(n) => Some(json!(*n)),
        V::Float(f) => Some(json!(*f as f64)),
        V::Str(s) => Some(serde_json::Value::String(s.clone())),
        V::Bytes | V::Invalid => None,
    }
}

fn now_iso8601() -> String {
    // Simple UTC timestamp without external crate
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    // Convert to components
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Days since epoch to Y-M-D (simplified leap year handling)
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}+00:00")
}

fn days_to_ymd(days_since_epoch: u64) -> (u64, u64, u64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days_since_epoch + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ieee_addr_valid() {
        let addr = IeeeAddr::from_hex("0x00158D0001020304").unwrap();
        assert_eq!(addr.as_hex(), "0x00158d0001020304");
    }

    #[test]
    fn parse_ieee_addr_lowercase() {
        let addr = IeeeAddr::from_hex("0xec1bbdfffeaa66db").unwrap();
        assert_eq!(addr.as_hex().to_lowercase(), "0xec1bbdfffeaa66db");
    }

    #[test]
    fn parse_ieee_addr_invalid() {
        assert!(IeeeAddr::from_hex("0x1234").is_none());
        assert!(IeeeAddr::from_hex("not_hex").is_none());
    }

    #[test]
    fn now_iso8601_format() {
        let ts = now_iso8601();
        // Should be like "2024-01-15T12:34:56+00:00"
        assert!(ts.contains('T'));
        assert!(ts.ends_with("+00:00"));
        assert_eq!(ts.len(), 25);
    }

    #[test]
    fn strip_transient_action_fields_removes_only_action_keys() {
        let mut state = serde_json::Map::new();
        state.insert("action".into(), json!("brightness_up_click"));
        state.insert("action_duration".into(), json!(1.5));
        state.insert("brightness".into(), json!(200));
        state.insert("battery".into(), json!(80));
        state.insert("state".into(), json!("ON"));

        Bridge::strip_transient_action_fields(&mut state);

        assert!(!state.contains_key("action"));
        assert!(!state.contains_key("action_duration"));
        assert_eq!(state.get("brightness"), Some(&json!(200)));
        assert_eq!(state.get("battery"), Some(&json!(80)));
        assert_eq!(state.get("state"), Some(&json!("ON")));
    }

    #[test]
    fn output_json_publishes_only_merged_state() {
        let state = json!({"state": "ON", "brightness": 200});
        let (publish_json, attributes) = Bridge::plan_state_publishes(OutputMode::Json, &state);
        assert!(publish_json);
        assert!(attributes.is_empty());
    }

    #[test]
    fn output_attribute_publishes_only_flattened_attributes() {
        let state = json!({"state": "ON", "brightness": 200});
        let (publish_json, attributes) =
            Bridge::plan_state_publishes(OutputMode::Attribute, &state);
        assert!(!publish_json);
        assert_eq!(attributes.len(), 2);
        assert!(attributes.contains(&("state".to_string(), "ON".to_string())));
        assert!(attributes.contains(&("brightness".to_string(), "200".to_string())));
    }

    #[test]
    fn output_attribute_and_json_publishes_both() {
        let state = json!({"state": "ON", "brightness": 200});
        let (publish_json, attributes) =
            Bridge::plan_state_publishes(OutputMode::AttributeAndJson, &state);
        assert!(publish_json);
        assert_eq!(attributes.len(), 2);
    }

    /// Reproduces the exact per-message sequence in the `CoordinatorEvent::
    /// Message` handler (merge_state -> clone into `state` -> strip transient
    /// action fields from retained `dev.state` -> plan_state_publishes) for
    /// two consecutive button clicks reporting the *same* action value, in
    /// `AttributeAndJson` mode. Guards against the flattened `action`
    /// attribute topic silently going stale/missing on repeat presses.
    #[test]
    fn repeated_identical_action_is_flattened_every_click() {
        let mut dev = Device::new(IeeeAddr::from_hex("0x00158d0001020304").unwrap(), 0x1234);

        for _ in 0..2 {
            let mut values = serde_json::Map::new();
            values.insert("action".into(), json!("toggle"));
            dev.merge_state(values);
            dev.state.insert("linkquality".into(), json!(200));

            let state = serde_json::Value::Object(dev.state.clone());
            Bridge::strip_transient_action_fields(&mut dev.state);

            let (publish_json, attributes) =
                Bridge::plan_state_publishes(OutputMode::AttributeAndJson, &state);
            assert!(publish_json);
            assert!(
                attributes.contains(&("action".to_string(), "toggle".to_string())),
                "action attribute missing from flattened publish: {attributes:?}"
            );
            assert!(!dev.state.contains_key("action"));
        }
    }

    #[test]
    fn days_to_ymd_epoch() {
        let (y, m, d) = days_to_ymd(0);
        assert_eq!((y, m, d), (1970, 1, 1));
    }

    #[test]
    fn days_to_ymd_known_date() {
        // 2024-01-15 is 19737 days since epoch
        let (y, m, d) = days_to_ymd(19737);
        assert_eq!((y, m, d), (2024, 1, 15));
    }

    #[test]
    fn transition_time_from_payload() {
        let p = json!({"state": "ON", "transition": 1.5});
        assert_eq!(transition_time(&p), 15);
    }

    #[test]
    fn transition_time_default() {
        let p = json!({"state": "ON"});
        assert_eq!(transition_time(&p), 0);
    }

    #[test]
    fn find_ep_with_existing_cluster() {
        let eps = vec![
            EndpointDesc {
                endpoint: 1,
                profile_id: 0x0104,
                device_id: 0,
                input_clusters: vec![0x0000, 0x0006],
                output_clusters: vec![],
            },
            EndpointDesc {
                endpoint: 2,
                profile_id: 0x0104,
                device_id: 0,
                input_clusters: vec![0x0402],
                output_clusters: vec![],
            },
        ];
        assert_eq!(find_ep_with_cluster(&eps, 0x0006), Some(1));
        assert_eq!(find_ep_with_cluster(&eps, 0x0402), Some(2));
        assert_eq!(find_ep_with_cluster(&eps, 0x9999), None);
    }

    #[test]
    fn unknown_device_tracker_dedups_until_identified() {
        let mut tracker = UnknownDeviceTracker::default();
        assert!(tracker.should_identify(0x1234));
        assert!(!tracker.should_identify(0x1234));
        assert!(tracker.should_identify(0x5678));
        // A failed lookup stays suppressed; a successful resolution allows a
        // later re-identification (for example after another NWK change).
        tracker.identified(0x1234);
        assert!(tracker.should_identify(0x1234));
    }

    #[test]
    fn read_targets_cover_native_keys() {
        assert_eq!(read_target("state"), Some((0x0006, &[0x0000][..])));
        assert_eq!(read_target("temperature"), Some((0x0402, &[0x0000][..])));
        assert_eq!(read_target("contact"), Some((0x0500, &[0x0002][..])));
        assert_eq!(read_target("unknown_key"), None);
    }

    #[test]
    fn build_get_targets_combines_and_deduplicates() {
        let endpoints = vec![
            EndpointDesc {
                endpoint: 1,
                profile_id: 0x0104,
                device_id: 0,
                input_clusters: vec![0x0006, 0x0300],
                output_clusters: vec![],
            },
            EndpointDesc {
                endpoint: 2,
                profile_id: 0x0104,
                device_id: 0,
                input_clusters: vec![0x0402],
                output_clusters: vec![],
            },
        ];
        let targets = build_get_targets(
            &endpoints,
            &["state", "color", "color_temp", "temperature", "bogus"],
        );
        assert_eq!(targets.get(&(1, 0x0006)), Some(&vec![0x0000]));
        assert_eq!(
            targets.get(&(1, 0x0300)),
            Some(&vec![0x0003, 0x0004, 0x0007, 0x0008])
        );
        assert_eq!(targets.get(&(2, 0x0402)), Some(&vec![0x0000]));
        assert_eq!(targets.len(), 3);
    }

    #[test]
    fn build_get_targets_skips_unavailable_clusters() {
        let endpoints = vec![EndpointDesc {
            endpoint: 1,
            profile_id: 0x0104,
            device_id: 0,
            input_clusters: vec![0x0006],
            output_clusters: vec![],
        }];
        let targets = build_get_targets(&endpoints, &["temperature", "state"]);
        assert_eq!(targets.len(), 1);
        assert!(targets.contains_key(&(1, 0x0006)));
    }

    #[test]
    fn codecs_reportable_change_for_analog_and_discrete_types() {
        assert_eq!(
            encode_reportable_change(0x21, Some(100.0)),
            Some(vec![0x64, 0x00])
        );
        assert_eq!(encode_reportable_change(0x20, Some(1.0)), Some(vec![0x01]));
        assert_eq!(encode_reportable_change(0x10, Some(1.0)), Some(Vec::new()));
        assert_eq!(
            encode_reportable_change(0x39, Some(1.5)),
            Some(1.5f32.to_le_bytes().to_vec())
        );
        assert_eq!(encode_reportable_change(0x42, Some(1.0)), None);
    }

    #[tokio::test]
    async fn get_pipeline_reads_parses_and_merges_device_response() {
        let endpoints = vec![EndpointDesc {
            endpoint: 1,
            profile_id: 0x0104,
            device_id: 0,
            input_clusters: vec![0x0402],
            output_clusters: vec![],
        }];
        let targets = build_get_targets(&endpoints, &["temperature"]);
        let mut trans_id = 0;
        let (merged, failed) = execute_get_reads(
            "sensor",
            &targets,
            &mut trans_id,
            None,
            |_endpoint, _cluster, tsn, _payload| async move {
                Ok(crate::coordinator::znp::transaction::ZclIncoming {
                    nwk_addr: 0x1234,
                    endpoint: 1,
                    cluster_id: 0x0402,
                    trans_seq: tsn,
                    frame_type: 0x00,
                    direction: 1,
                    command: 0x01,
                    // readRsp: attr 0x0000, status 0, int16 2250 = 22.50 C
                    data: vec![0x18, tsn, 0x01, 0x00, 0x00, 0x00, 0x29, 0xCA, 0x08],
                })
            },
        )
        .await;
        assert_eq!(failed, 0);
        assert_eq!(trans_id, 1);
        assert_eq!(merged.get("temperature"), Some(&json!(22.5)));
    }

    #[tokio::test]
    async fn get_pipeline_counts_failed_reads_without_fake_state() {
        let endpoints = vec![EndpointDesc {
            endpoint: 1,
            profile_id: 0x0104,
            device_id: 0,
            input_clusters: vec![0x0402],
            output_clusters: vec![],
        }];
        let targets = build_get_targets(&endpoints, &["temperature"]);
        let mut trans_id = 0;
        let (merged, failed) = execute_get_reads(
            "sensor",
            &targets,
            &mut trans_id,
            None,
            |_endpoint, _cluster, _tsn, _payload| async { Err(crate::error::Error::Timeout) },
        )
        .await;
        assert_eq!(failed, 1);
        assert!(merged.is_empty());
    }

    #[test]
    fn reporting_change_values_decode_integer_and_float() {
        assert_eq!(reportable_change_value(0x21, &[0x64, 0x00]), json!(100));
        assert_eq!(reportable_change_value(0x20, &[0x01]), json!(1));
        assert_eq!(
            reportable_change_value(0x39, &1.5f32.to_le_bytes()),
            json!(1.5)
        );
        assert_eq!(reportable_change_value(0x42, &[]), json!(0));
    }

    #[test]
    fn unified_dispatch_prefers_converter_values_and_keeps_native_extras() {
        let native = serde_json::Map::from_iter([
            ("state".to_string(), json!("ON")),
            ("temperature".to_string(), json!(1)),
        ]);
        let external = serde_json::Map::from_iter([
            ("temperature".to_string(), json!(2)),
            ("action".to_string(), json!("on")),
        ]);
        let merged = merge_converted_values(native, Some(external));
        assert_eq!(merged["state"], "ON");
        assert_eq!(merged["temperature"], 2);
        assert_eq!(merged["action"], "on");
        assert_eq!(merged.len(), 3);
    }

    #[test]
    fn unified_dispatch_without_converter_keeps_native_values() {
        let native = serde_json::Map::from_iter([("state".to_string(), json!("OFF"))]);
        let merged = merge_converted_values(native, None);
        assert_eq!(merged["state"], "OFF");
    }

    struct FakeReporting {
        ieee: Option<IeeeAddr>,
        requests: std::sync::Mutex<Vec<(u8, u16, u8, u8)>>,
        binds: std::sync::atomic::AtomicUsize,
    }

    impl FakeReporting {
        fn new() -> Self {
            Self {
                ieee: IeeeAddr::from_hex("0x00124b00120144ae"),
                requests: std::sync::Mutex::new(Vec::new()),
                binds: std::sync::atomic::AtomicUsize::new(0),
            }
        }
    }

    impl ReportingIo for FakeReporting {
        fn coordinator_ieee(&self) -> Option<IeeeAddr> {
            self.ieee
        }

        fn read<'a>(
            &'a self,
            _nwk_addr: u16,
            endpoint: u8,
            cluster: u16,
            tsn: u8,
            payload: Vec<u8>,
        ) -> ReportingFuture<
            'a,
            crate::error::Result<crate::coordinator::znp::transaction::ZclIncoming>,
        > {
            let command = payload.get(2).copied().unwrap_or(0);
            self.requests
                .lock()
                .unwrap()
                .push((endpoint, cluster, tsn, command));
            let data = if command == 0x08 {
                // read reporting config response: attr 0, status 0, u16,
                // min 10, max 300, change 50
                vec![
                    0x18, tsn, 0x09, 0x00, 0x00, 0x00, 0x00, 0x21, 0x0A, 0x00, 0x2C, 0x01, 0x32,
                    0x00,
                ]
            } else {
                // configure reporting response: success
                vec![0x18, tsn, 0x07, 0x00]
            };
            Box::pin(async move {
                Ok(crate::coordinator::znp::transaction::ZclIncoming {
                    nwk_addr: 0x1234,
                    endpoint,
                    cluster_id: cluster,
                    trans_seq: tsn,
                    frame_type: 0x00,
                    direction: 1,
                    command: data[2],
                    data,
                })
            })
        }

        fn bind<'a>(
            &'a self,
            _nwk_addr: u16,
            _source_ieee: [u8; 8],
            _endpoint: u8,
            _cluster: u16,
            _destination_ieee: [u8; 8],
        ) -> ReportingFuture<'a, crate::error::Result<()>> {
            self.binds
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Box::pin(async { Ok(()) })
        }
    }

    fn reporting_device() -> (
        DeviceRegistry,
        Arc<Mutex<PersistentDatabase>>,
        std::path::PathBuf,
    ) {
        let dir = std::env::temp_dir().join(format!(
            "z2m_reporting_e2e_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("database.db");
        std::fs::write(
            &path,
            r#"{"id":2,"type":"Router","ieeeAddr":"0xec1bbdfffeaa66db","nwkAddr":4660,"endpoints":{"1":{"profId":260,"epId":1,"devId":0,"inClusterList":[1026],"outClusterList":[],"clusters":{},"binds":[],"configuredReportings":[],"meta":{}}},"meta":{}}"#,
        )
        .unwrap();
        let devices = DeviceRegistry::new();
        let ieee = IeeeAddr::from_hex("0xec1bbdfffeaa66db").unwrap();
        let mut device = Device::new(ieee, 0x1234);
        device.interview_complete = true;
        device.endpoints.push(EndpointDesc {
            endpoint: 1,
            profile_id: 0x0104,
            device_id: 0,
            input_clusters: vec![1026],
            output_clusters: vec![],
        });
        devices.add(device);
        let db = Arc::new(Mutex::new(PersistentDatabase::open(&path)));
        (devices, db, dir)
    }

    #[tokio::test]
    async fn reporting_read_e2e_with_fake_coordinator_persists_and_replies() {
        let (devices, db, dir) = reporting_device();
        let external = ExternalConverter::new(dir.join("cache"), "https://example.invalid");
        let fake = FakeReporting::new();
        let mut trans_id = 0;
        let mqtt_topic = "zigbee2mqtt/bridge/request/device/reporting/read";
        let mqtt_payload = serde_json::to_vec(&json!({
            "id": "0xec1bbdfffeaa66db",
            "endpoint": 1,
            "cluster": 1026,
            "configs": [{"attribute": 0}],
        }))
        .unwrap();
        let (request, payload) =
            crate::mqtt::parse_bridge_request("zigbee2mqtt", mqtt_topic, &mqtt_payload)
                .expect("MQTT bridge request");
        let result = Bridge::handle_reporting(
            &devices,
            &external,
            &fake,
            &db,
            &request,
            &payload,
            &mut trans_id,
        )
        .await;
        let data = result.expect("reporting read should succeed");
        assert_eq!(data["configs"][0]["attribute"], 0);
        assert_eq!(data["configs"][0]["data_type"], 0x21);
        assert_eq!(data["configs"][0]["minimum_report_interval"], 10);
        assert_eq!(data["configs"][0]["maximum_report_interval"], 300);
        assert_eq!(data["configs"][0]["reportable_change"], "3200");
        assert_eq!(
            fake.requests.lock().unwrap().as_slice(),
            &[(1, 1026, 1, 0x08)]
        );

        let db = db.lock().await;
        let reports = db.configured_reportings("0xec1bbdfffeaa66db", 1);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0]["minRepIntval"], 10);
        assert_eq!(reports[0]["maxRepIntval"], 300);

        let reply = reporting_mqtt_response(Ok(data), Some("tx-1"));
        assert_eq!(reply["status"], "ok");
        assert_eq!(reply["transaction"], "tx-1");
        drop(db);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn reporting_configure_e2e_with_fake_coordinator_binds_and_persists() {
        let (devices, db, dir) = reporting_device();
        let external = ExternalConverter::new(dir.join("cache"), "https://example.invalid");
        let fake = FakeReporting::new();
        let mut trans_id = 0;
        let payload = json!({
            "id": "0xec1bbdfffeaa66db",
            "endpoint": 1,
            "cluster": 1026,
            "attribute": 0,
            "minimum_report_interval": 5,
            "maximum_report_interval": 600,
            "reportable_change": 100,
        });
        let result = Bridge::handle_reporting(
            &devices,
            &external,
            &fake,
            &db,
            "device/reporting/configure",
            &payload,
            &mut trans_id,
        )
        .await;
        let data = result.expect("reporting configure should succeed");
        assert_eq!(data["minimum_report_interval"], 5);
        assert_eq!(data["maximum_report_interval"], 600);
        assert_eq!(fake.binds.load(std::sync::atomic::Ordering::Relaxed), 1);
        let commands: Vec<u8> = fake
            .requests
            .lock()
            .unwrap()
            .iter()
            .map(|(_, _, _, command)| *command)
            .collect();
        assert_eq!(commands, vec![0x08, 0x06]);

        let db = db.lock().await;
        let reports = db.configured_reportings("0xec1bbdfffeaa66db", 1);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0]["minRepIntval"], 5);
        assert_eq!(reports[0]["maxRepIntval"], 600);
        assert_eq!(reports[0]["repChange"], 100.0);
        drop(db);

        let reply = reporting_mqtt_response(Err("bad payload".into()), None);
        assert_eq!(reply["status"], "error");
        assert_eq!(reply["error"], "bad payload");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn broker_free_mqtt_get_request_end_to_end() {
        let command = crate::mqtt::parse_device_command(
            "zigbee2mqtt",
            "zigbee2mqtt/bulb/get",
            br#"{"temperature":""}"#,
        )
        .expect("get command");
        let (name, payload) = match command {
            MqttCommand::GetDevice {
                friendly_name,
                payload,
            } => (friendly_name, payload),
            other => panic!("unexpected command: {other:?}"),
        };
        assert_eq!(name, "bulb");

        let devices = DeviceRegistry::new();
        let mut device = Device::new(IeeeAddr::from_hex("0xec1bbdfffeaa66db").unwrap(), 0x1234);
        device.friendly_name = name;
        device.interview_complete = true;
        device.endpoints.push(EndpointDesc {
            endpoint: 1,
            profile_id: 0x0104,
            device_id: 0,
            input_clusters: vec![0x0402],
            output_clusters: vec![],
        });
        devices.add(device);

        let requested: Vec<String> = payload
            .as_object()
            .map(|object| object.keys().cloned().collect())
            .unwrap_or_default();
        let wanted: Vec<&str> = requested.iter().map(String::as_str).collect();
        let endpoints = devices.find_by_name("bulb").unwrap().endpoints.clone();
        let targets = build_get_targets(&endpoints, &wanted);
        let mut trans_id = 0;
        let (merged, failed) = execute_get_reads(
            "bulb",
            &targets,
            &mut trans_id,
            None,
            |_endpoint, _cluster, tsn, _payload| async move {
                Ok(crate::coordinator::znp::transaction::ZclIncoming {
                    nwk_addr: 0x1234,
                    endpoint: 1,
                    cluster_id: 0x0402,
                    trans_seq: tsn,
                    frame_type: 0x00,
                    direction: 1,
                    command: 0x01,
                    data: vec![0x18, tsn, 0x01, 0x00, 0x00, 0x00, 0x29, 0xCA, 0x08],
                })
            },
        )
        .await;
        assert_eq!(failed, 0);
        assert_eq!(merged.get("temperature"), Some(&json!(22.5)));
    }

    fn ota_fixture_image() -> Vec<u8> {
        let size = 64usize;
        let mut bytes = vec![0u8; size];
        bytes[0..4].copy_from_slice(&0x0BEE_F11Eu32.to_le_bytes());
        bytes[4..6].copy_from_slice(&0x0100u16.to_le_bytes());
        bytes[6..8].copy_from_slice(&56u16.to_le_bytes());
        bytes[10..12].copy_from_slice(&0x1037u16.to_le_bytes());
        bytes[12..14].copy_from_slice(&1u16.to_le_bytes());
        bytes[14..18].copy_from_slice(&0x01020304u32.to_le_bytes());
        bytes[18..20].copy_from_slice(&2u16.to_le_bytes());
        bytes[20..36].copy_from_slice(b"rust-ota-fixture");
        bytes[52..56].copy_from_slice(&(size as u32).to_le_bytes());
        bytes
    }

    fn ota_test_devices() -> DeviceRegistry {
        let devices = DeviceRegistry::new();
        let mut device = Device::new(IeeeAddr::from_hex("0xec1bbdfffeaa66db").unwrap(), 0x1234);
        device.friendly_name = "ota_device".into();
        device.interview_complete = true;
        devices.add(device);
        devices
    }

    #[tokio::test]
    async fn ota_bridge_check_and_update_requests_end_to_end() {
        let devices = ota_test_devices();
        let ota = Mutex::new(crate::ota::OtaManager::default());
        let fixture = ota_fixture_image();
        let image = hex::encode(&fixture);
        let payload = json!({
            "id": "0xec1bbdfffeaa66db",
            "hex": {"data": image, "file_name": "fixture.ota"},
            "sha512": crate::ota::sha512_hex(&fixture),
        });

        let check = handle_ota_request(
            &ota,
            std::path::Path::new("."),
            &devices,
            "device/ota_update/check",
            &payload,
        )
        .await
        .expect("check");
        assert_eq!(check.data["update_available"], true);
        assert_eq!(check.data["manufacturer_code"], 0x1037);
        assert_eq!(check.data["image_size"], 64);
        assert!(check.state_update.is_none());

        let update = handle_ota_request(
            &ota,
            std::path::Path::new("."),
            &devices,
            "device/ota_update/update",
            &payload,
        )
        .await
        .expect("update");
        assert_eq!(update.data["file_version"], 0x01020304);
        let (ieee, state) = update.state_update.expect("update state");
        assert_eq!(ieee, "0xec1bbdfffeaa66db");
        assert_eq!(state["state"], "updating");
        assert_eq!(state["progress"], 0);

        let abort = handle_ota_request(
            &ota,
            std::path::Path::new("."),
            &devices,
            "device/ota_update/update/abort",
            &payload,
        )
        .await
        .expect("abort");
        assert_eq!(abort.data["aborted"], true);
        assert_eq!(abort.state_update.unwrap().1["state"], "idle");
    }

    #[tokio::test]
    async fn ota_bridge_request_error_paths() {
        let devices = ota_test_devices();
        let ota = Mutex::new(crate::ota::OtaManager::default());

        let unknown = json!({"id": "0xffffffffffffffff", "hex": {"data": "00"}});
        assert!(handle_ota_request(
            &ota,
            std::path::Path::new("."),
            &devices,
            "device/ota_update/update",
            &unknown
        )
        .await
        .unwrap_err()
        .contains("does not exist"));

        let invalid = json!({"id": "0xec1bbdfffeaa66db", "hex": {"data": "zzzz"}});
        assert!(handle_ota_request(
            &ota,
            std::path::Path::new("."),
            &devices,
            "device/ota_update/check",
            &invalid
        )
        .await
        .unwrap_err()
        .contains("invalid hex"));

        let unsupported = json!({"id": "0xec1bbdfffeaa66db"});
        assert!(handle_ota_request(
            &ota,
            std::path::Path::new("."),
            &devices,
            "device/ota_update/schedule",
            &unsupported
        )
        .await
        .unwrap_err()
        .contains("Unsupported"));
    }

    #[test]
    fn ota_progress_payload_matches_state_contract() {
        assert_eq!(
            ota_update_state(0.5, false),
            json!({"state": "updating", "progress": 50})
        );
        assert_eq!(
            ota_update_state(1.0, true),
            json!({"state": "idle", "progress": 100})
        );
    }

    #[tokio::test]
    async fn ota_bridge_local_index_metadata_and_checksum() {
        let dir = std::env::temp_dir().join(format!(
            "z2m_ota_bridge_index_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let image = ota_fixture_image();
        std::fs::write(dir.join("fixture.ota"), &image).unwrap();
        let index = json!([{
            "fileName": "fixture.ota",
            "fileVersion": 0x01020304u32,
            "url": "fixture.ota",
            "sha512": crate::ota::sha512_hex(&image),
            "manufacturerCode": 0x1037,
            "imageType": 1,
            "releaseNotes": "bridge release",
        }]);
        std::fs::write(dir.join("index.json"), serde_json::to_vec(&index).unwrap()).unwrap();

        let devices = ota_test_devices();
        let ota = Mutex::new(crate::ota::OtaManager::default());
        let payload = json!({
            "id": "0xec1bbdfffeaa66db",
            "index": "index.json",
            "manufacturer_code": 0x1037,
            "image_type": 1,
            "file_version": 0x01020303u32,
        });
        let check = handle_ota_request(&ota, &dir, &devices, "device/ota_update/check", &payload)
            .await
            .expect("index check");
        assert_eq!(check.data["source"], "fixture.ota");
        assert_eq!(check.data["release_notes"], "bridge release");
        assert_eq!(check.data["file_version"], 0x01020304);

        let update = handle_ota_request(&ota, &dir, &devices, "device/ota_update/update", &payload)
            .await
            .expect("index update");
        assert_eq!(update.data["source"], "fixture.ota");
        assert_eq!(update.state_update.unwrap().1["progress"], 0);

        let mut broken = index.clone();
        broken[0]["sha512"] = json!("00");
        std::fs::write(dir.join("index.json"), serde_json::to_vec(&broken).unwrap()).unwrap();
        let error = handle_ota_request(&ota, &dir, &devices, "device/ota_update/check", &payload)
            .await
            .unwrap_err();
        assert!(error.contains("checksum"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
