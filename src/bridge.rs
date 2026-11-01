/// The main bridge -- ties coordinator, MQTT, device registry, and ZCL together.
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use std::path::PathBuf;

use crate::config::{Config, OutputMode};
use crate::coordinator::znp::ZnpHandle;
use crate::coordinator::{open_coordinator, CoordinatorEvent, CoordinatorHandle};
use crate::database::{self, PersistentDatabase};
use crate::devices::{Device, DeviceRegistry};
use crate::groups::GroupRegistry;
use crate::error::Result;
use crate::homeassistant;
use crate::mqtt::attribute_output::flatten_attribute_output;
use crate::mqtt::{MqttBridge, MqttCommand};
use crate::zigbee::zcl;
use crate::zigbee::zcl::clusters::color;
use crate::zigbee::zcl::clusters::level;
use crate::zigbee::zcl::clusters::on_off;
use crate::zigbee::{EndpointDesc, IeeeAddr};

pub struct Bridge {
    cfg: Config,
    config_path: PathBuf,
    devices: Arc<DeviceRegistry>,
    groups: Arc<GroupRegistry>,
    database: Arc<Mutex<PersistentDatabase>>,
    db_write_pending: Arc<Mutex<bool>>,
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
        let db_path = database::find_database(&config_path)
            .unwrap_or_else(|| config_path.parent().unwrap_or(std::path::Path::new(".")).join("database.db"));
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
                groups.next_id.store(g.id + 1, std::sync::atomic::Ordering::Relaxed);
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
                    let g = crate::groups::Group { id: gid, friendly_name: fname.clone(), members: Vec::new() };
                    groups.by_name.insert(fname.clone(), gid);
                    groups.inner.insert(gid, g);
                    let cur = groups.next_id.load(std::sync::atomic::Ordering::Relaxed);
                    if gid >= cur {
                        groups.next_id.store(gid + 1, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }
        }
        Self {
            cfg,
            config_path,
            devices: Arc::new(DeviceRegistry::new()),
            groups: Arc::new(groups),
            database: Arc::new(Mutex::new(db)),
            db_write_pending: Arc::new(Mutex::new(false)),
        }
    }

    async fn persist_database(&self, coordinator_ieee: Option<IeeeAddr>) {
        // Batching: update in-memory immediately, debounce disk write to save memory/I/O (upstream Entity.database.update(write:false))
        {
            let devices = self.devices.all_devices();
            let groups = self.groups.all();
            let mut db = self.database.lock().await;
            db.sync_devices(&devices, coordinator_ieee);
            db.sync_groups(&groups);
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
                warn!("Failed to write database.db (batched): {e}");
            }
        });
    }

    async fn persist_groups_config(&self) {
        // Read current config file, update groups section, write back
        let path = &self.config_path;
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                warn!("Cannot read config for groups persist {}: {e}", path.display());
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
            cfg.groups.insert(g.id.to_string(), crate::config::GroupConfig { friendly_name: Some(g.friendly_name.clone()) });
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
        let mut coord = open_coordinator(&self.cfg).await?;
        info!("Coordinator ready");

        // 6. Publish retained MQTT state
        self.republish_mqtt_state(&mqtt, &coord).await;

        // 7. Permit join if configured
        if self.cfg.permit_join {
            coord.permit_join(254).await?;
            info!("Permit join enabled (254 s)");
        }
        info!("Startup complete, entering main loop");

        let devices = Arc::clone(&self.devices);
        let groups = Arc::clone(&self.groups);
        let base_topic = self.cfg.mqtt.base_topic.clone();
        let ha_enabled = self.cfg.homeassistant.enabled();
        let device_configs = self.cfg.devices.clone();
        let coordinator_ieee = coord.info.ieee_addr.map(|a| a.as_hex()).unwrap_or_default();

        // 7. Main event loop
        let mut trans_id: u8 = 0;

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
                                    homeassistant::publish_discovery(&mqtt, &dev, &base_topic, &coordinator_ieee).await;
                                }
                            }
                        }

                        Some(CoordinatorEvent::Message {
                            src_addr, src_ep, cluster_id, link_quality, data
                        }) => {
                            debug!("AF msg from 0x{src_addr:04X} ep={src_ep} cluster=0x{cluster_id:04X} lqi={link_quality}");

                            // Unknown NWK? Request IEEE address to link it.
                            if devices.get_by_nwk(src_addr).is_none() {
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

                            let zone_type = devices.get_by_nwk(src_addr).and_then(|d| d.zone_type);
                            let held_arrow_direction = devices
                                .get_by_nwk(src_addr)
                                .and_then(|d| d.arrow_hold_direction.clone());
                            match zcl::parse_message(cluster_id, &data, zone_type, held_arrow_direction.as_deref()) {
                                Ok(Some(mut zcl_msg)) => {
                                    if cluster_id == 0x0000 {
                                        for key in BASIC_CLUSTER_METADATA_KEYS {
                                            zcl_msg.values.remove(key);
                                        }
                                    }
                                    let suffix = devices.get_by_nwk(src_addr).and_then(|d| {
                                        if d.endpoints.len() <= 1 { return None; }
                                        match src_ep {
                                            1 => Some("l1".to_string()),
                                            2 => Some("l2".to_string()),
                                            3 => Some("button".to_string()),
                                            _ => None,
                                        }
                                    });
                                    if let Some(suf) = &suffix {
                                        let keys: Vec<String> = zcl_msg.values.keys().cloned().collect();
                                        let mut new_values = serde_json::Map::new();
                                        for k in keys {
                                            if let Some(v) = zcl_msg.values.remove(&k) {
                                                new_values.insert(format!("{}_{}", k, suf), v);
                                            }
                                        }
                                        zcl_msg.values = new_values;
                                    }
                                    if let Some(mut dev) = devices.get_mut_by_nwk(src_addr) {
                                        if let Some(direction_update) = zcl_msg.arrow_hold_direction {
                                            dev.arrow_hold_direction = direction_update;
                                        }
                                        dev.merge_state(zcl_msg.values.clone());
                                        dev.state.insert("linkquality".into(), json!(link_quality));
                                        dev.state.insert("last_seen".into(), json!(now_iso8601()));
                                        let state = serde_json::Value::Object(dev.state.clone());
                                        let name = dev.friendly_name.clone();
                                        Self::strip_transient_action_fields(&mut dev.state);
                                        drop(dev);
                                        Self::publish_state(&mqtt, self.cfg.advanced.output, &name, &state).await;
                                        let _ = mqtt.publish_availability(&name, true).await;
                                    }
                                }
                                Ok(None) => {}
                                Err(e) => warn!("ZCL parse error: {e}"),
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
                        Some(MqttCommand::GetDevice { friendly_name, .. }) => {
                            debug!("MQTT get: {friendly_name}");
                            if let Some(dev) = devices.find_by_name(&friendly_name) {
                                let state = serde_json::Value::Object(dev.state);
                                Self::publish_state(&mqtt, self.cfg.advanced.output, &friendly_name, &state).await;
                                let _ = mqtt.publish_availability(&friendly_name, true).await;
                            } else {
                                warn!("Get command for unknown device: {friendly_name}");
                            }
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
                                        devices.rename(&ieee, to.clone());
                                        Self::publish_device_list_static(&devices, &mqtt).await;
                                        self.persist_database(coord.info.ieee_addr).await;
                                        let resp = mk_resp("ok", json!({"from": from, "to": to, "homeassistant_rename": false}), None);
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
                                    let group_id = payload.get("group").and_then(|v| v.as_str()).or_else(|| payload.get("id").and_then(|v| v.as_str())).unwrap_or("");
                                    let device = payload.get("device").and_then(|v| v.as_str()).unwrap_or("");
                                    let endpoint: u8 = payload.get("endpoint").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
                                    let gid: u16 = group_id.parse().unwrap_or_else(|_| groups.get_by_name(group_id).map(|g| g.id).unwrap_or(0));
                                    if groups.add_member(gid, device.to_string(), endpoint) {
                                        let _ = mqtt.publish_bridge_groups(&groups).await;
                                        self.persist_database(coord.info.ieee_addr).await;
                                        self.persist_groups_config().await;
                                        let resp = mk_resp("ok", json!({}), None);
                                        let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                    } else {
                                        let resp = mk_resp("error", json!({}), Some("group not found".into()));
                                        let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                    }
                                }
                                "group/members/remove" => {
                                    let group_id = payload.get("group").and_then(|v| v.as_str()).unwrap_or("");
                                    let device = payload.get("device").and_then(|v| v.as_str()).unwrap_or("");
                                    let endpoint: u8 = payload.get("endpoint").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
                                    let gid: u16 = group_id.parse().unwrap_or_else(|_| groups.get_by_name(group_id).map(|g| g.id).unwrap_or(0));
                                    if groups.remove_member(gid, device, endpoint) {
                                        let _ = mqtt.publish_bridge_groups(&groups).await;
                                        self.persist_database(coord.info.ieee_addr).await;
                                        self.persist_groups_config().await;
                                        let resp = mk_resp("ok", json!({}), None);
                                        let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                    } else {
                                        let resp = mk_resp("error", json!({}), Some("group not found".into()));
                                        let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                    }
                                }
                                "group/members/remove_all" => {
                                    let group_id = payload.get("group").and_then(|v| v.as_str()).unwrap_or("");
                                    let gid: u16 = group_id.parse().unwrap_or_else(|_| groups.get_by_name(group_id).map(|g| g.id).unwrap_or(0));
                                    if groups.clear_members(gid) {
                                        let _ = mqtt.publish_bridge_groups(&groups).await;
                                        self.persist_database(coord.info.ieee_addr).await;
                                        self.persist_groups_config().await;
                                        let resp = mk_resp("ok", json!({}), None);
                                        let _ = mqtt.publish_bridge_response(&request, &resp).await;
                                    } else {
                                        let resp = mk_resp("error", json!({}), Some("group not found".into()));
                                        let _ = mqtt.publish_bridge_response(&request, &resp).await;
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
                                    let resp = mk_resp("ok", json!({}), None);
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
                                    let resp = mk_resp("ok", json!({}), None);
                                    let _ = mqtt.publish_bridge_response(&request, &resp).await;
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
            }
        }

        if let Err(e) = mqtt.publish_bridge_state(false).await {
            warn!("Failed to publish offline state: {e}");
        }
        // Flush any pending batched database write
        {
            let db = self.database.lock().await;
            let _ = db.write();
        }
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
                homeassistant::publish_discovery(
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
            let types = ["3.3 кОм", "5 кОм", "6.8 кОм", "10 кОм", "12 кОм", "14.8 кОм", "15 кОм", "20 кОм", "33 кОм", "47 кОм"];
            if let Some(idx) = types.iter().position(|t| *t == st) {
                if let Some(ep) = find_ep_with_cluster(&endpoints, 0x0201) {
                    let zcl_payload = zcl::write::write_attributes_payload(*trans_id, Some(0x7777), &[(0xFF00, 0x30, &[idx as u8])]);
                    *trans_id = trans_id.wrapping_add(1);
                    if let Err(e) = coord.send_zcl(nwk_addr, ep, 0x0201, *trans_id, zcl_payload).await {
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
                let zcl_payload = zcl::write::write_attributes_payload(*trans_id, Some(0x7777), &[(0xFF01, 0x20, &[raw as u8])]);
                *trans_id = trans_id.wrapping_add(1);
                if let Err(e) = coord.send_zcl(nwk_addr, ep, 0x0201, *trans_id, zcl_payload).await {
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
                let zcl_payload = zcl::write::write_attributes_payload(*trans_id, Some(0x7777), &[(0xFF02, 0x28, &[raw as u8])]);
                *trans_id = trans_id.wrapping_add(1);
                if let Err(e) = coord.send_zcl(nwk_addr, ep, 0x0201, *trans_id, zcl_payload).await {
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
                    let zcl_payload = zcl::write::write_attributes_payload(*trans_id, None, &[(0x0000, 0x30, &[idx as u8])]);
                    *trans_id = trans_id.wrapping_add(1);
                    if let Err(e) = coord.send_zcl(nwk_addr, ep, 0x0007, *trans_id, zcl_payload).await {
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
                let zcl_payload = zcl::write::write_attributes_payload(*trans_id, Some(0x7777), &[(0xFF10, 0x10, &[if on { 1u8 } else { 0u8 }])]);
                *trans_id = trans_id.wrapping_add(1);
                if let Err(e) = coord.send_zcl(nwk_addr, ep, 0x0007, *trans_id, zcl_payload).await {
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
                    let zcl_payload = zcl::write::write_attributes_payload(*trans_id, Some(0x7777), &[(0x4901, 0x30, &[1u8])]);
                    *trans_id = trans_id.wrapping_add(1);
                    if let Err(e) = coord.send_zcl(nwk_addr, ep, 0x0204, *trans_id, zcl_payload).await {
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
}
