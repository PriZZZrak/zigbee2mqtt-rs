use crate::config::DeviceConfig;
use crate::devices::Device;
use crate::groups::Group;
use crate::zigbee::{EndpointDesc, IeeeAddr};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DbEntry {
    #[serde(default)]
    id: Option<u32>,
    #[serde(default, rename = "type")]
    device_type: String,
    #[serde(default)]
    ieee_addr: String,
    #[serde(default)]
    nwk_addr: u16,
    #[serde(default)]
    manuf_id: Option<u16>,
    #[serde(default)]
    manuf_name: Option<String>,
    #[serde(default)]
    power_source: Option<String>,
    #[serde(default)]
    model_id: Option<String>,
    #[serde(default)]
    sw_build_id: Option<String>,
    #[serde(default)]
    ep_list: Option<Vec<u8>>,
    #[serde(default)]
    endpoints: HashMap<String, DbEndpoint>,
    #[serde(default)]
    interview_completed: Option<bool>,
    #[serde(default)]
    interview_state: Option<String>,
    #[serde(default)]
    last_seen: Option<u64>,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DbEndpoint {
    #[serde(default)]
    prof_id: u16,
    #[serde(default)]
    ep_id: u8,
    #[serde(default)]
    dev_id: u16,
    #[serde(default)]
    in_cluster_list: Vec<u16>,
    #[serde(default)]
    out_cluster_list: Vec<u16>,
    #[serde(default)]
    clusters: serde_json::Value,
    #[serde(default)]
    binds: serde_json::Value,
    #[serde(default)]
    configured_reportings: serde_json::Value,
    #[serde(default)]
    meta: serde_json::Value,
}

pub fn load_database(
    path: &Path,
    device_configs: &HashMap<String, DeviceConfig>,
) -> (Vec<Device>, Option<IeeeAddr>) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            warn!("Cannot read database file {}: {e}", path.display());
            return (vec![], None);
        }
    };
    let mut devices = Vec::new();
    let mut coordinator_ieee = None;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let entry: DbEntry = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(e) => {
                debug!("Skipping unparseable database line: {e}");
                continue;
            }
        };
        if entry.device_type == "Coordinator" {
            if let Some(ieee) = parse_ieee(&entry.ieee_addr) {
                coordinator_ieee = Some(ieee);
            }
            continue;
        }
        if entry.device_type == "Group" {
            continue;
        }
        if entry.ieee_addr.is_empty() {
            continue;
        }
        let ieee = match parse_ieee(&entry.ieee_addr) {
            Some(a) => a,
            None => {
                debug!("Skipping entry with invalid IEEE: {}", entry.ieee_addr);
                continue;
            }
        };
        let mut dev = Device::new(ieee, entry.nwk_addr);
        let ieee_hex = ieee.as_hex();
        if let Some(cfg) = device_configs.get(&ieee_hex) {
            if let Some(ref name) = cfg.friendly_name {
                dev.friendly_name = name.clone();
            }
            dev.disabled = cfg.disabled.unwrap_or(false);
        }
        dev.database_id = entry.id;
        dev.manufacturer = entry.manuf_name;
        dev.power_source = entry.power_source;
        dev.model = entry.model_id;
        dev.sw_build_id = entry.sw_build_id;
        // interviewState takes precedence, fallback to interviewCompleted
        dev.interview_complete = entry
            .interview_state
            .as_deref()
            .map(|s| s == "SUCCESSFUL")
            .unwrap_or(entry.interview_completed.unwrap_or(false));
        if let Some(mid) = entry.manuf_id {
            dev.manufacturer_id = Some(mid);
        }
        for (ep_key, ep_data) in &entry.endpoints {
            let ep_id = ep_key.parse::<u8>().unwrap_or(ep_data.ep_id);
            dev.endpoints.push(EndpointDesc {
                endpoint: ep_id,
                profile_id: ep_data.prof_id,
                device_id: ep_data.dev_id,
                input_clusters: ep_data.in_cluster_list.clone(),
                output_clusters: ep_data.out_cluster_list.clone(),
            });
        }
        debug!(
            "Imported device {} ({}), NWK=0x{:04X}, {} endpoints, interviewed={}",
            dev.friendly_name,
            ieee_hex,
            dev.nwk_addr,
            dev.endpoints.len(),
            dev.interview_complete
        );
        devices.push(dev);
    }
    info!("Imported {} devices from {}", devices.len(), path.display());
    (devices, coordinator_ieee)
}

fn parse_ieee(s: &str) -> Option<IeeeAddr> {
    IeeeAddr::from_hex(s)
}

pub fn find_database(config_path: &Path) -> Option<PathBuf> {
    let dir = config_path.parent().unwrap_or(Path::new("."));
    let candidates = [
        dir.join("database.db"),
        dir.join("data/database.db"),
        Path::new("/opt/zigbee2mqtt/data/database.db").to_path_buf(),
        Path::new("/var/lib/zigbee2mqtt/database.db").to_path_buf(),
    ];
    for path in &candidates {
        if path.exists() {
            info!("Found zigbee2mqtt database: {}", path.display());
            return Some(path.clone());
        }
    }
    None
}

// ── Persistent Database with atomic write (portable both ways) ───────────────

pub struct PersistentDatabase {
    path: PathBuf,
    entries: HashMap<u32, serde_json::Value>,
    max_id: u32,
}

impl PersistentDatabase {
    pub fn open(path: &Path) -> Self {
        let mut entries = HashMap::new();
        let mut max_id = 0;
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                match serde_json::from_str::<serde_json::Value>(line) {
                    Ok(v) => {
                        if let Some(id) = v.get("id").and_then(|x| x.as_u64()) {
                            let id = id as u32;
                            entries.insert(id, v);
                            if id > max_id {
                                max_id = id;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Corrupted database line, ignoring. {}", e);
                    }
                }
            }
        }
        Self {
            path: path.to_path_buf(),
            entries,
            max_id,
        }
    }

    pub fn new_id(&mut self) -> u32 {
        self.max_id += 1;
        self.max_id
    }

    pub fn insert(&mut self, entry: serde_json::Value) {
        if let Some(id) = entry.get("id").and_then(|x| x.as_u64()).map(|x| x as u32) {
            if self.entries.contains_key(&id) {
                panic!("DatabaseEntry with ID '{}' already exists", id);
            }
            self.entries.insert(id, entry);
            if id > self.max_id {
                self.max_id = id;
            }
        }
    }

    pub fn update(&mut self, entry: serde_json::Value, write: bool) {
        if let Some(id) = entry.get("id").and_then(|x| x.as_u64()).map(|x| x as u32) {
            if !self.entries.contains_key(&id) {
                panic!("DatabaseEntry with ID '{}' does not exist", id);
            }
            self.entries.insert(id, entry);
            if write {
                self.write().unwrap();
            }
        }
    }

    pub fn remove(&mut self, id: u32) {
        if self.entries.remove(&id).is_some() {
            let _ = self.write();
        } else {
            panic!("DatabaseEntry with ID '{}' does not exist", id);
        }
    }

    pub fn write(&self) -> std::io::Result<()> {
        let mut lines = String::new();
        for entry in self.entries.values() {
            lines.push_str(&serde_json::to_string(entry).unwrap());
            lines.push('\n');
        }
        let tmp = format!("{}.tmp", self.path.display());
        let tmp_path = Path::new(&tmp);
        // handle existing tmp
        if tmp_path.exists() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let dated = format!("{}.{}", tmp, now);
            let _ = std::fs::rename(tmp_path, &dated);
            tracing::warn!("Found '{}' when writing database, indicating past write failure; renamed it to '{}'", tmp, dated);
        }
        // write tmp
        use std::io::Write;
        let mut f = std::fs::File::create(tmp_path)?;
        if !lines.is_empty() {
            // remove last newline to match upstream's slice(0,-1)
            let to_write = &lines[..lines.len() - 1];
            f.write_all(to_write.as_bytes())?;
        }
        f.sync_all()?;
        drop(f);
        std::fs::rename(tmp_path, &self.path)?;
        // Make the rename durable across power loss. Directory fsync is not
        // supported on every filesystem, so a refusal is reported rather than
        // turning an otherwise complete write into a failure.
        if let Some(parent) = self.path.parent() {
            match std::fs::File::open(parent) {
                Ok(directory) => {
                    if let Err(error) = directory.sync_all() {
                        warn!(
                            "Cannot fsync database directory {}: {error}",
                            parent.display()
                        );
                    }
                }
                Err(error) => warn!(
                    "Cannot open database directory {} for fsync: {error}",
                    parent.display()
                ),
            }
        }
        Ok(())
    }

    /// Synchronize device records and return the id assigned to each device.
    ///
    /// Callers must persist the returned ids in their device registry. Without
    /// this write-back a device that did not come from database.db would be
    /// assigned a fresh id on every synchronization, churning records instead
    /// of keeping a stable identity (upstream keeps `databaseID` on the entity).
    pub fn sync_devices(
        &mut self,
        devices: &[Device],
        coordinator_ieee: Option<IeeeAddr>,
    ) -> Vec<(IeeeAddr, u32)> {
        // Preserve Coordinator, Group and GreenPower entries, update/insert devices
        let mut assigned = Vec::with_capacity(devices.len());
        let mut to_keep: HashMap<u32, serde_json::Value> = HashMap::new();
        for (id, v) in &self.entries {
            if let Some(t) = v.get("type").and_then(|x| x.as_str()) {
                if t == "Coordinator" || t == "Group" || t == "GreenPower" {
                    to_keep.insert(*id, v.clone());
                }
            }
        }
        // Rebuild device entries from Device objects, preserving ids and extra fields
        let mut used_ids = std::collections::HashSet::new();
        // Build ieee -> original entry map for preservation
        let mut orig_by_ieee: HashMap<String, serde_json::Value> = HashMap::new();
        for v in self.entries.values() {
            if let Some(ieee) = v.get("ieeeAddr").and_then(|x| x.as_str()) {
                orig_by_ieee.insert(ieee.to_lowercase(), v.clone());
            }
        }
        for dev in devices {
            let id = dev.database_id.unwrap_or_else(|| {
                // find unused id
                let mut nid = self.max_id + 1;
                while self.entries.contains_key(&nid) || used_ids.contains(&nid) {
                    nid += 1;
                }
                nid
            });
            used_ids.insert(id);
            assigned.push((dev.ieee_addr, id));
            if id > self.max_id {
                self.max_id = id;
            }
            let mut ep_list: Vec<u8> = dev.endpoints.iter().map(|e| e.endpoint).collect();
            let mut endpoints = serde_json::Map::new();
            for ep in &dev.endpoints {
                endpoints.insert(
                    ep.endpoint.to_string(),
                    serde_json::json!({
                        "profId": ep.profile_id,
                        "epId": ep.endpoint,
                        "devId": ep.device_id,
                        "inClusterList": ep.input_clusters,
                        "outClusterList": ep.output_clusters,
                        "clusters": {},
                        "binds": [],
                        "configuredReportings": [],
                        "meta": {}
                    }),
                );
            }
            // Preserve extra fields from original if exists
            let orig = orig_by_ieee.get(&dev.ieee_addr.as_hex());
            let meta = orig
                .and_then(|v| v.get("meta"))
                .cloned()
                .unwrap_or(json!({}));
            let last_seen = orig
                .and_then(|v| v.get("lastSeen"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let app_version = orig
                .and_then(|v| v.get("appVersion"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let stack_version = orig
                .and_then(|v| v.get("stackVersion"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let hw_version = orig
                .and_then(|v| v.get("hwVersion"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let date_code = orig
                .and_then(|v| v.get("dateCode"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let zcl_version = orig
                .and_then(|v| v.get("zclVersion"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let checkin_interval = orig
                .and_then(|v| v.get("checkinInterval"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let gp_security_key = orig
                .and_then(|v| v.get("gpSecurityKey"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let scheduled_ota = orig
                .and_then(|v| v.get("scheduledOta"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            // Preserve original type if exists (Coordinator/Router/EndDevice/Unknown/GreenPower)
            let orig_type = orig
                .and_then(|v| {
                    v.get("type")
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| {
                    if dev.power_source.as_deref() == Some("Battery") {
                        "EndDevice".to_string()
                    } else {
                        "Router".to_string()
                    }
                });
            // Preserve original endpoints' clusters/binds if exists for that endpoint
            let mut endpoints_preserved = serde_json::Map::new();
            for ep in &dev.endpoints {
                let ep_key = ep.endpoint.to_string();
                let orig_ep = orig.and_then(|v| v.get("endpoints").and_then(|e| e.get(&ep_key)));
                let clusters = orig_ep
                    .and_then(|e| e.get("clusters"))
                    .cloned()
                    .unwrap_or(json!({}));
                let binds = orig_ep
                    .and_then(|e| e.get("binds"))
                    .cloned()
                    .unwrap_or(json!([]));
                let configured = orig_ep
                    .and_then(|e| e.get("configuredReportings"))
                    .cloned()
                    .unwrap_or(json!([]));
                let ep_meta = orig_ep
                    .and_then(|e| e.get("meta"))
                    .cloned()
                    .unwrap_or(json!({}));
                endpoints_preserved.insert(
                    ep_key,
                    json!({
                        "profId": ep.profile_id,
                        "epId": ep.endpoint,
                        "devId": ep.device_id,
                        "inClusterList": ep.input_clusters,
                        "outClusterList": ep.output_clusters,
                        "clusters": clusters,
                        "binds": binds,
                        "configuredReportings": configured,
                        "meta": ep_meta
                    }),
                );
            }
            // If orig had endpoints not in current Device.endpoints (e.g., 242), preserve them
            if let Some(orig_eps) =
                orig.and_then(|v| v.get("endpoints").and_then(|e| e.as_object()))
            {
                for (k, v) in orig_eps {
                    if !endpoints_preserved.contains_key(k) {
                        endpoints_preserved.insert(k.clone(), v.clone());
                    }
                }
            }
            // Keep the persisted endpoint list in sync with the preserved endpoint
            // map. Rebuilding it only from the live registry silently dropped
            // endpoints (notably 242) that are intentionally kept in the record.
            if let Some(orig_list) = orig
                .and_then(|v| v.get("epList"))
                .and_then(|x| x.as_array())
            {
                for endpoint in orig_list.iter().filter_map(|x| x.as_u64()).map(|x| x as u8) {
                    if !ep_list.contains(&endpoint) {
                        ep_list.push(endpoint);
                    }
                }
            }
            // An in-progress interview must not be rewritten as FAILED: preserve
            // the persisted state until the interview actually completes.
            let interview_state = if dev.interview_complete {
                "SUCCESSFUL".to_string()
            } else {
                orig.and_then(|v| v.get("interviewState"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("FAILED")
                    .to_string()
            };
            let mut entry = serde_json::json!({
                "id": id,
                "type": orig_type,
                "ieeeAddr": dev.ieee_addr.as_hex(),
                "nwkAddr": dev.nwk_addr,
                "manufId": dev.manufacturer_id.unwrap_or_else(|| orig.and_then(|v| v.get("manufId").and_then(|x| x.as_u64()).map(|x| x as u16)).unwrap_or(0)),
                "manufName": dev.manufacturer.clone().or_else(|| orig.and_then(|v| v.get("manufName").and_then(|x| x.as_str()).map(|s| s.to_string()))),
                "powerSource": dev.power_source.clone().or_else(|| orig.and_then(|v| v.get("powerSource").and_then(|x| x.as_str()).map(|s| s.to_string()))),
                "modelId": dev.model.clone().or_else(|| orig.and_then(|v| v.get("modelId").and_then(|x| x.as_str()).map(|s| s.to_string()))),
                "epList": ep_list,
                "endpoints": endpoints_preserved,
                "interviewCompleted": dev.interview_complete,
                "interviewState": interview_state,
                "meta": meta,
                "lastSeen": last_seen
            });
            // Preserve version fields if orig has them
            if let Some(sw_build_id) = dev.sw_build_id.clone().or_else(|| {
                orig.and_then(|v| {
                    v.get("swBuildId")
                        .and_then(|x| x.as_str())
                        .map(str::to_string)
                })
            }) {
                entry["swBuildId"] = json!(sw_build_id);
            }
            if !app_version.is_null() {
                entry["appVersion"] = app_version;
            }
            if !stack_version.is_null() {
                entry["stackVersion"] = stack_version;
            }
            if !hw_version.is_null() {
                entry["hwVersion"] = hw_version;
            }
            if !date_code.is_null() {
                entry["dateCode"] = date_code;
            }
            if !zcl_version.is_null() {
                entry["zclVersion"] = zcl_version;
            }
            if !checkin_interval.is_null() {
                entry["checkinInterval"] = checkin_interval;
            }
            if !gp_security_key.is_null() {
                entry["gpSecurityKey"] = gp_security_key;
            }
            if !scheduled_ota.is_null() {
                entry["scheduledOta"] = scheduled_ota;
            }
            // Preserve any other extra fields not explicitly handled
            if let Some(orig) = orig {
                for (k, v) in orig.as_object().unwrap() {
                    if !entry.as_object().unwrap().contains_key(k) {
                        entry[k] = v.clone();
                    }
                }
            }
            to_keep.insert(id, entry);
        }
        // Ensure Coordinator exists if we have one
        if let Some(coord) = coordinator_ieee {
            let has_coord = to_keep
                .values()
                .any(|v| v.get("type").and_then(|x| x.as_str()) == Some("Coordinator"));
            if !has_coord {
                let cid = 1;
                if !to_keep.contains_key(&cid) {
                    to_keep.insert(cid, serde_json::json!({
                        "id": cid,
                        "type": "Coordinator",
                        "ieeeAddr": coord.as_hex(),
                        "nwkAddr": 0,
                        "manufId": 0,
                        "epList": [1],
                        "endpoints": {"1": {"profId": 260, "epId": 1, "devId": 5, "inClusterList": [], "outClusterList": [], "clusters": {}, "binds": [], "configuredReportings": [], "meta": {}}},
                        "interviewCompleted": true,
                        "interviewState": "SUCCESSFUL",
                        "meta": {}
                    }));
                    if cid > self.max_id {
                        self.max_id = cid;
                    }
                }
            }
        }
        self.entries = to_keep;
        assigned
    }

    pub fn sync_groups(&mut self, groups: &[Group]) {
        // Capture group identities before removing the records. The previous
        // implementation retained only non-group entries and then searched for
        // the old groupID, so it could never find one and allocated a new id on
        // every synchronization.
        let mut previous: HashMap<u16, serde_json::Value> = HashMap::new();
        for value in self.entries.values() {
            if value.get("type").and_then(|x| x.as_str()) != Some("Group") {
                continue;
            }
            if let Some(group_id) = value
                .get("groupID")
                .and_then(|x| x.as_u64())
                .map(|x| x as u16)
            {
                previous.entry(group_id).or_insert_with(|| value.clone());
            }
        }
        self.entries
            .retain(|_, v| v.get("type").and_then(|x| x.as_str()) != Some("Group"));

        let mut used_ids: std::collections::HashSet<u32> = self.entries.keys().copied().collect();
        for g in groups {
            // Reuse the whole record when possible so `meta` and unknown fields
            // survive a rename or membership change.
            let mut entry = previous
                .get(&g.id)
                .cloned()
                .unwrap_or_else(|| json!({"meta": {}}));
            let id = entry
                .get("id")
                .and_then(|x| x.as_u64())
                .map(|x| x as u32)
                .filter(|id| !used_ids.contains(id))
                .unwrap_or_else(|| {
                    let mut nid = self.max_id + 1;
                    while used_ids.contains(&nid) {
                        nid += 1;
                    }
                    nid
                });
            used_ids.insert(id);
            if id > self.max_id {
                self.max_id = id;
            }
            let members: Vec<_> = g
                .members
                .iter()
                .map(|m| serde_json::json!({"ieeeAddr": m.ieee_address, "endpoint": m.endpoint}))
                .collect();
            entry["id"] = json!(id);
            entry["type"] = json!("Group");
            entry["groupID"] = json!(g.id);
            entry["members"] = json!(members);
            if !entry.get("meta").is_some_and(|meta| meta.is_object()) {
                entry["meta"] = json!({});
            }
            self.entries.insert(id, entry);
        }
    }

    pub fn load_groups(&self) -> Vec<Group> {
        let mut groups = Vec::new();
        for v in self.entries.values() {
            if v.get("type").and_then(|x| x.as_str()) != Some("Group") {
                continue;
            }
            if let Some(gid) = v.get("groupID").and_then(|x| x.as_u64()).map(|x| x as u16) {
                let members = v
                    .get("members")
                    .and_then(|x| x.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|m| {
                                let ieee = m.get("ieeeAddr").and_then(|x| x.as_str())?.to_string();
                                let ep =
                                    m.get("endpoint").and_then(|x| x.as_u64()).unwrap_or(1) as u8;
                                Some(crate::groups::GroupMember {
                                    ieee_address: ieee,
                                    endpoint: ep,
                                })
                            })
                            .collect()
                    })
                    .unwrap_or(Vec::new());
                // Upstream stores friendly_name only in configuration.yaml, not in DB — use placeholder, Bridge will overlay from Config
                let fname = format!("group_{}", gid);
                let mut g = Group {
                    id: gid,
                    friendly_name: fname,
                    members: Vec::new(),
                };
                g.members = members;
                groups.push(g);
            }
        }
        groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const SAMPLE_DB: &str = r#"{"id":1,"type":"Coordinator","ieeeAddr":"0x00124b00120144ae","nwkAddr":0,"manufId":4169,"epList":[1,242],"endpoints":{"1":{"profId":260,"epId":1,"devId":101,"inClusterList":[0,3,6,8],"outClusterList":[0,3,4,5,6,8],"clusters":{},"binds":[],"configuredReportings":[],"meta":{}}},"interviewCompleted":true,"meta":{}}
{"id":2,"type":"Router","ieeeAddr":"0xec1bbdfffeaa66db","nwkAddr":44718,"manufId":4476,"manufName":"IKEA of Sweden","powerSource":"Mains (single phase)","modelId":"TRADFRI bulb E27 CWS 806lm","epList":[1],"endpoints":{"1":{"profId":260,"epId":1,"devId":512,"inClusterList":[0,3,4,5,6,8,768,4096,64636],"outClusterList":[5,25,32,4096],"clusters":{},"binds":[],"configuredReportings":[],"meta":{}}},"appVersion":1,"stackVersion":6,"hwVersion":1,"dateCode":"20210331","swBuildId":"2.3.093","zclVersion":3,"interviewCompleted":true,"meta":{},"lastSeen":1710612345000}
{"id":3,"type":"EndDevice","ieeeAddr":"0xcc86ecfffe9fd1b1","nwkAddr":17895,"manufName":"Xiaomi","powerSource":"Battery","modelId":"lumi.sensor_ht.agl02","epList":[1],"endpoints":{"1":{"profId":260,"epId":1,"devId":770,"inClusterList":[0,1,3,1026,1029],"outClusterList":[],"clusters":{},"binds":[],"configuredReportings":[],"meta":{}}},"interviewCompleted":true,"meta":{}}"#;

    fn sample_configs() -> HashMap<String, DeviceConfig> {
        let mut m = HashMap::new();
        m.insert(
            "0xec1bbdfffeaa66db".to_string(),
            DeviceConfig {
                friendly_name: Some("living_room_bulb".to_string()),
                ..Default::default()
            },
        );
        m.insert(
            "0xcc86ecfffe9fd1b1".to_string(),
            DeviceConfig {
                friendly_name: Some("bedroom_sensor".to_string()),
                ..Default::default()
            },
        );
        m
    }

    fn load_sample() -> (Vec<Device>, Option<IeeeAddr>) {
        use std::sync::atomic::{AtomicU32, Ordering};
        static CTR: AtomicU32 = AtomicU32::new(0);
        let n = CTR.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("z2m_test_db_{n}"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("database.db");
        std::fs::write(&path, SAMPLE_DB).unwrap();
        let configs = sample_configs();
        let result = load_database(&path, &configs);
        std::fs::remove_dir_all(&dir).ok();
        result
    }

    #[test]
    fn loads_devices_not_coordinator() {
        let (devices, coord) = load_sample();
        assert_eq!(devices.len(), 2);
        assert!(coord.is_some());
    }

    #[test]
    fn coordinator_ieee_extracted() {
        let (_, coord) = load_sample();
        assert_eq!(coord.unwrap().as_hex(), "0x00124b00120144ae");
    }

    #[test]
    fn device_ieee_and_nwk() {
        let (devices, _) = load_sample();
        let bulb = devices
            .iter()
            .find(|d| d.friendly_name == "living_room_bulb")
            .unwrap();
        assert_eq!(bulb.ieee_addr.as_hex(), "0xec1bbdfffeaa66db");
        assert_eq!(bulb.nwk_addr, 44718);
    }

    #[test]
    fn friendly_names_from_config() {
        let (devices, _) = load_sample();
        let names: Vec<&str> = devices.iter().map(|d| d.friendly_name.as_str()).collect();
        assert!(names.contains(&"living_room_bulb"));
        assert!(names.contains(&"bedroom_sensor"));
    }

    #[test]
    fn manufacturer_and_model_imported() {
        let (devices, _) = load_sample();
        let bulb = devices
            .iter()
            .find(|d| d.friendly_name == "living_room_bulb")
            .unwrap();
        assert_eq!(bulb.manufacturer.as_deref(), Some("IKEA of Sweden"));
        assert_eq!(bulb.model.as_deref(), Some("TRADFRI bulb E27 CWS 806lm"));
        assert_eq!(bulb.sw_build_id.as_deref(), Some("2.3.093"));
    }

    #[test]
    fn endpoints_imported() {
        let (devices, _) = load_sample();
        let bulb = devices
            .iter()
            .find(|d| d.friendly_name == "living_room_bulb")
            .unwrap();
        assert_eq!(bulb.endpoints.len(), 1);
        let ep = &bulb.endpoints[0];
        assert_eq!(ep.endpoint, 1);
        assert_eq!(ep.profile_id, 260);
        assert!(ep.input_clusters.contains(&0x0006));
        assert!(ep.input_clusters.contains(&0x0008));
        assert!(ep.input_clusters.contains(&0x0300));
    }

    #[test]
    fn sensor_endpoints() {
        let (devices, _) = load_sample();
        let sensor = devices
            .iter()
            .find(|d| d.friendly_name == "bedroom_sensor")
            .unwrap();
        let ep = &sensor.endpoints[0];
        assert!(ep.input_clusters.contains(&0x0402));
        assert!(ep.input_clusters.contains(&0x0405));
        assert!(ep.input_clusters.contains(&0x0001));
    }

    #[test]
    fn interview_state() {
        let (devices, _) = load_sample();
        assert!(devices.iter().all(|d| d.interview_complete));
    }

    #[test]
    fn power_source_imported() {
        let (devices, _) = load_sample();
        let sensor = devices
            .iter()
            .find(|d| d.friendly_name == "bedroom_sensor")
            .unwrap();
        assert_eq!(sensor.power_source.as_deref(), Some("Battery"));
        assert_eq!(sensor.device_type(), "EndDevice");
    }

    #[test]
    fn device_type_from_power_source() {
        let (devices, _) = load_sample();
        let bulb = devices
            .iter()
            .find(|d| d.friendly_name == "living_room_bulb")
            .unwrap();
        assert_eq!(bulb.power_source.as_deref(), Some("Mains (single phase)"));
        assert_eq!(bulb.device_type(), "Router");
    }

    #[test]
    fn parse_ieee_valid() {
        let addr = parse_ieee("0xec1bbdfffeaa66db").unwrap();
        assert_eq!(addr.as_hex(), "0xec1bbdfffeaa66db");
    }

    #[test]
    fn parse_ieee_invalid() {
        assert!(parse_ieee("0x1234").is_none());
        assert!(parse_ieee("not_hex").is_none());
    }

    fn temp_db(name: &str, content: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static CTR: AtomicU32 = AtomicU32::new(0);
        let n = CTR.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("z2m_sync_{name}_{n}"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("database.db");
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn sync_groups_reuses_database_id_and_meta() {
        use crate::groups::{Group, GroupMember};
        let path = temp_db(
            "groups",
            r#"{"id":10,"type":"Group","groupID":1,"members":[],"meta":{"custom":true}}"#,
        );
        let mut db = PersistentDatabase::open(&path);
        let group = Group {
            id: 1,
            friendly_name: "group_1".to_string(),
            members: vec![GroupMember {
                ieee_address: "0xec1bbdfffeaa66db".to_string(),
                endpoint: 1,
            }],
        };

        db.sync_groups(std::slice::from_ref(&group));
        db.sync_groups(std::slice::from_ref(&group));

        assert_eq!(db.entries.len(), 1);
        let entry = db
            .entries
            .get(&10)
            .expect("original group id must be reused");
        assert_eq!(entry["groupID"], 1);
        assert_eq!(entry["meta"]["custom"], true);
        assert_eq!(entry["members"][0]["ieeeAddr"], "0xec1bbdfffeaa66db");
        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    #[test]
    fn sync_devices_returns_ids_that_stay_stable_after_write_back() {
        let path = temp_db(
            "devices",
            r#"{"id":1,"type":"Coordinator","ieeeAddr":"0x00124b00120144ae","nwkAddr":0,"epList":[1],"endpoints":{},"interviewCompleted":true,"meta":{}}"#,
        );
        let mut db = PersistentDatabase::open(&path);
        let mut device = Device::new(parse_ieee("0xec1bbdfffeaa66db").unwrap(), 0x1234);

        let first = db.sync_devices(std::slice::from_ref(&device), None);
        assert_eq!(first.len(), 1);
        let (_, id) = first[0];
        assert!(id > 1, "must not reuse the coordinator record id");

        // Simulate the bridge persisting the assigned id on the live entity.
        device.database_id = Some(id);
        let second = db.sync_devices(std::slice::from_ref(&device), None);
        assert_eq!(second[0].1, id);
        assert_eq!(db.entries.len(), 2);
        assert_eq!(
            db.entries.get(&id).unwrap()["ieeeAddr"],
            "0xec1bbdfffeaa66db"
        );
        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    #[test]
    fn sync_devices_preserves_ep_list_interview_state_and_sw_build_id() {
        let path = temp_db(
            "fidelity",
            r#"{"id":2,"type":"EndDevice","ieeeAddr":"0xcc86ecfffe9fd1b1","nwkAddr":1,"epList":[1,242],"endpoints":{"1":{"profId":260,"epId":1,"devId":0,"inClusterList":[0,1026],"outClusterList":[],"clusters":{},"binds":[],"configuredReportings":[],"meta":{}},"242":{"profId":41440,"epId":242,"devId":97,"inClusterList":[],"outClusterList":[],"clusters":{},"binds":[],"configuredReportings":[],"meta":{}}},"interviewState":"IN_PROGRESS","swBuildId":"1.2.3","meta":{}}"#,
        );
        let mut db = PersistentDatabase::open(&path);
        let mut device = Device::new(parse_ieee("0xcc86ecfffe9fd1b1").unwrap(), 2);
        device.database_id = Some(2);
        device.sw_build_id = Some("9.9.9".to_string());
        device.endpoints.push(EndpointDesc {
            endpoint: 1,
            profile_id: 260,
            device_id: 0,
            input_clusters: vec![0, 1026],
            output_clusters: vec![],
        });

        db.sync_devices(std::slice::from_ref(&device), None);

        let entry = db.entries.get(&2).unwrap();
        assert_eq!(entry["swBuildId"], "9.9.9");
        assert_eq!(entry["interviewState"], "IN_PROGRESS");
        let ep_list: Vec<u64> = entry["epList"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_u64().unwrap())
            .collect();
        assert!(
            ep_list.contains(&242),
            "preserved endpoint missing from epList"
        );
        assert!(entry["endpoints"].get("242").is_some());
        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    #[test]
    fn empty_database_returns_empty() {
        let dir = std::env::temp_dir().join(format!("z2m_empty_db_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("database.db");
        std::fs::write(&path, "").unwrap();
        let (devices, coord) = load_database(&path, &HashMap::new());
        std::fs::remove_dir_all(&dir).ok();
        assert!(devices.is_empty());
        assert!(coord.is_none());
    }

    #[test]
    fn corrupted_lines_skipped() {
        let db = "not json\n{\"id\":1,\"type\":\"Router\",\"ieeeAddr\":\"0xec1bbdfffeaa66db\",\"nwkAddr\":100,\"endpoints\":{},\"interviewCompleted\":true}\n{broken";
        let dir = std::env::temp_dir().join(format!("z2m_corrupt_db_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("database.db");
        std::fs::write(&path, db).unwrap();
        let (devices, _) = load_database(&path, &HashMap::new());
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(devices.len(), 1);
    }

    #[test]
    fn round_trip_preserves_compatible_format() {
        let dir = std::env::temp_dir().join(format!("z2m_roundtrip_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("database.db");
        std::fs::write(&path, SAMPLE_DB).unwrap();
        let mut db = PersistentDatabase::open(&path);
        // Sync with loaded devices and write
        let (devices, coord) = load_database(&path, &HashMap::new());
        db.sync_devices(&devices, coord);
        db.write().unwrap();
        // Re-open and verify can be read by original logic (load_database)
        let (devices2, _) = load_database(&path, &HashMap::new());
        assert_eq!(devices2.len(), 2);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn interview_state_both_fields_written() {
        let dir = std::env::temp_dir().join(format!("z2m_interview_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("database.db");
        let mut db = PersistentDatabase::open(&path);
        let dev = Device::new(
            crate::zigbee::IeeeAddr::from_hex("0xec1bbdfffeaa66db").unwrap(),
            0x1234,
        );
        // dev.interview_complete false by default -> should write FAILED
        db.sync_devices(std::slice::from_ref(&dev), None);
        db.write().unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"interviewCompleted\":false"));
        assert!(content.contains("\"interviewState\":\"FAILED\""));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn group_entry_preserved_on_sync() {
        let dir = std::env::temp_dir().join(format!("z2m_group_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("database.db");
        let group_line = r#"{"id":10,"type":"Group","groupID":1,"members":[],"meta":{}}"#;
        let content = format!("{}\n{}", SAMPLE_DB.lines().next().unwrap(), group_line);
        std::fs::write(&path, &content).unwrap();
        let mut db = PersistentDatabase::open(&path);
        assert!(db.entries.contains_key(&10));
        let (devices, coord) = load_database(&path, &HashMap::new());
        db.sync_devices(&devices, coord);
        assert!(db.entries.contains_key(&10), "Group should be preserved");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn write_is_atomic_and_replaces_stale_tmp() {
        let path = temp_db("atomic", "");
        let mut db = PersistentDatabase::open(&path);
        let device = Device::new(parse_ieee("0xec1bbdfffeaa66db").unwrap(), 0x1234);
        let assigned = db.sync_devices(std::slice::from_ref(&device), None);
        // Simulate a crashed previous write.
        std::fs::write(format!("{}.tmp", path.display()), "partial").unwrap();

        db.write().unwrap();

        assert!(
            !Path::new(&format!("{}.tmp", path.display())).exists(),
            "temporary file must be renamed or replaced"
        );
        let reopened = PersistentDatabase::open(&path);
        assert_eq!(
            reopened.entries.get(&assigned[0].1).unwrap()["ieeeAddr"],
            "0xec1bbdfffeaa66db"
        );
        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }
}
