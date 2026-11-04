//! Offline migration preflight and snapshot tooling.
//!
//! These operations never open the serial port or the coordinator: they inspect
//! configuration, the herdsman database and referenced files, then optionally
//! copy them to an immutable, timestamped snapshot for rollback.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::config::Config;
use crate::database::find_database;
use crate::error::{Error, Result};

/// Non-cryptographic integrity checksum. It detects accidental truncation or
/// modification; it is not a signature and does not protect against a malicious
/// editor (documented as `fnv1a64` in the manifest).
fn checksum(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn checksum_string(bytes: &[u8]) -> String {
    format!("fnv1a64:{:016x}", checksum(bytes))
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct NetworkSummary {
    #[serde(rename = "pan_id")]
    pub pan_id: u16,
    #[serde(rename = "ext_pan_id")]
    pub ext_pan_id: String,
    pub channel: u8,
    /// The key itself is deliberately never included in the report.
    pub network_key_configured: bool,
    pub adapter: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DatabaseSummary {
    pub path: Option<String>,
    pub exists: bool,
    pub total_lines: usize,
    pub valid_lines: usize,
    pub invalid_lines: Vec<usize>,
    pub devices: usize,
    pub interviewed: usize,
    pub groups: usize,
    pub coordinator_ieee: Option<String>,
    pub duplicate_ids: Vec<u32>,
    pub duplicate_ieee: Vec<String>,
    pub models: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileStatus {
    pub path: String,
    pub exists: bool,
    pub bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Inspection {
    pub config_path: String,
    pub config_valid: bool,
    pub config_error: Option<String>,
    pub network: NetworkSummary,
    pub database: DatabaseSummary,
    pub referenced_files: Vec<FileStatus>,
    pub coordinator_backup: Option<FileStatus>,
    pub state_file: Option<FileStatus>,
    /// Checks that cannot be performed offline and must not be reported as done.
    pub unperformed_checks: Vec<String>,
    pub warnings: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotFile {
    pub source: String,
    pub snapshot: String,
    pub bytes: u64,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub directory: String,
    pub created_unix_seconds: u64,
    pub tool_version: String,
    pub files: Vec<SnapshotFile>,
    pub warnings: Vec<String>,
}

fn file_status(path: &Path) -> FileStatus {
    let metadata = std::fs::metadata(path).ok();
    FileStatus {
        path: path.display().to_string(),
        exists: metadata.is_some(),
        bytes: metadata.map(|m| m.len()),
    }
}

fn secret_file(value: &str) -> Option<PathBuf> {
    let rest = value.trim().strip_prefix("!secret")?.trim();
    let mut parts = rest.splitn(2, char::is_whitespace);
    let file = parts.next()?.trim();
    let _key = parts.next()?.trim();
    if file.is_empty() {
        return None;
    }
    let mut name = file.to_string();
    if !name.ends_with(".yaml") && !name.ends_with(".yml") {
        name.push_str(".yaml");
    }
    Some(PathBuf::from(name))
}

fn referenced_files(config_path: &Path) -> Vec<PathBuf> {
    let config_dir = config_path.parent().unwrap_or(Path::new("."));
    let mut files = Vec::new();
    let Ok(content) = std::fs::read_to_string(config_path) else {
        return files;
    };
    let raw: serde_json::Value = serde_yml::from_str(&content).unwrap_or(serde_json::Value::Null);
    for key in ["devices", "groups"] {
        if let Some(value) = raw.get(key) {
            if let Some(single) = value.as_str() {
                files.push(config_dir.join(single));
            } else if let Some(array) = value.as_array() {
                for file in array.iter().filter_map(|entry| entry.as_str()) {
                    files.push(config_dir.join(file));
                }
            }
        }
    }
    for (namespace, key) in [
        ("mqtt", "server"),
        ("mqtt", "user"),
        ("mqtt", "password"),
        ("advanced", "network_key"),
        ("frontend", "auth_token"),
    ] {
        if let Some(value) = raw
            .get(namespace)
            .and_then(|section| section.get(key))
            .and_then(|value| value.as_str())
        {
            if let Some(file) = secret_file(value) {
                files.push(config_dir.join(file));
            }
        }
    }
    files.sort();
    files.dedup();
    files
}

fn analyze_database(path: Option<&Path>) -> DatabaseSummary {
    let Some(path) = path else {
        return DatabaseSummary::default();
    };
    let mut summary = DatabaseSummary {
        path: Some(path.display().to_string()),
        exists: path.exists(),
        ..DatabaseSummary::default()
    };
    let Ok(content) = std::fs::read_to_string(path) else {
        return summary;
    };
    let mut id_counts: HashMap<u32, usize> = HashMap::new();
    let mut ieee_counts: HashMap<String, usize> = HashMap::new();
    for (index, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        summary.total_lines += 1;
        let value: serde_json::Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => {
                summary.invalid_lines.push(index + 1);
                continue;
            }
        };
        summary.valid_lines += 1;
        if let Some(id) = value.get("id").and_then(|x| x.as_u64()) {
            *id_counts.entry(id as u32).or_default() += 1;
        }
        if let Some(ieee) = value.get("ieeeAddr").and_then(|x| x.as_str()) {
            if !ieee.is_empty() {
                *ieee_counts.entry(ieee.to_lowercase()).or_default() += 1;
            }
        }
        let entry_type = value
            .get("type")
            .and_then(|x| x.as_str())
            .unwrap_or_default();
        match entry_type {
            "Coordinator" => {
                summary.coordinator_ieee = value
                    .get("ieeeAddr")
                    .and_then(|x| x.as_str())
                    .map(str::to_string);
            }
            "Group" => summary.groups += 1,
            _ => {
                summary.devices += 1;
                let interviewed = value
                    .get("interviewState")
                    .and_then(|x| x.as_str())
                    .map(|state| state == "SUCCESSFUL")
                    .or_else(|| value.get("interviewCompleted").and_then(|x| x.as_bool()))
                    .unwrap_or(false);
                if interviewed {
                    summary.interviewed += 1;
                }
                if let Some(model) = value.get("modelId").and_then(|x| x.as_str()) {
                    *summary.models.entry(model.to_string()).or_default() += 1;
                }
            }
        }
    }
    summary.duplicate_ids = id_counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(id, _)| id)
        .collect();
    summary.duplicate_ids.sort_unstable();
    summary.duplicate_ieee = ieee_counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(ieee, _)| ieee)
        .collect();
    summary.duplicate_ieee.sort();
    summary
}

/// Build a read-only migration report. Never opens the coordinator.
pub fn inspect(config_path: &Path) -> Inspection {
    let mut warnings = Vec::new();
    let mut blockers = Vec::new();
    let (config, config_error) = match Config::load(config_path) {
        Ok(config) => {
            if let Err(error) = config.validate() {
                blockers.push(format!("configuration validation failed: {error}"));
            }
            (Some(config), None)
        }
        Err(error) => {
            blockers.push(format!("cannot load configuration: {error}"));
            (None, Some(error.to_string()))
        }
    };

    let network = config
        .as_ref()
        .map(|config| NetworkSummary {
            pan_id: config.advanced.pan_id,
            ext_pan_id: hex::encode(config.advanced.ext_pan_id),
            channel: config.advanced.channel,
            network_key_configured: config.advanced.network_key.iter().any(|byte| *byte != 0),
            adapter: format!("{:?}", config.serial.adapter).to_lowercase(),
        })
        .unwrap_or_default();

    let database_path = find_database(config_path);
    let database = analyze_database(database_path.as_deref());
    if database_path.is_none() {
        warnings.push(
            "no database.db found; this looks like a fresh install, not a network migration"
                .to_string(),
        );
    }
    if !database.invalid_lines.is_empty() {
        blockers.push(format!(
            "database contains {} unparseable line(s): {:?}",
            database.invalid_lines.len(),
            database.invalid_lines
        ));
    }
    if !database.duplicate_ids.is_empty() {
        blockers.push(format!(
            "database contains duplicate record ids: {:?}",
            database.duplicate_ids
        ));
    }
    if !database.duplicate_ieee.is_empty() {
        blockers.push(format!(
            "database contains duplicate IEEE addresses: {:?}",
            database.duplicate_ieee
        ));
    }
    if database.exists && database.coordinator_ieee.is_none() {
        warnings.push("database has no Coordinator record".to_string());
    }

    let referenced: Vec<FileStatus> = referenced_files(config_path)
        .iter()
        .map(|path| file_status(path))
        .collect();
    for file in &referenced {
        if !file.exists {
            blockers.push(format!(
                "referenced configuration file is missing: {}",
                file.path
            ));
        }
    }

    let config_dir = config_path.parent().unwrap_or(Path::new("."));
    let data_dir = database_path
        .as_deref()
        .and_then(Path::parent)
        .unwrap_or(config_dir);
    let mut backup_candidates = vec![data_dir.join("coordinator_backup.json")];
    if data_dir != config_dir {
        backup_candidates.push(config_dir.join("coordinator_backup.json"));
    }
    let coordinator_backup = backup_candidates
        .into_iter()
        .find(|path| path.exists())
        .map(|path| file_status(&path));
    if coordinator_backup.is_none() {
        warnings.push(
            "no coordinator_backup.json found; database.db alone cannot restore coordinator NV"
                .to_string(),
        );
    }

    let state_file = [data_dir.join("state.json"), config_dir.join("state.json")]
        .into_iter()
        .find(|path| path.exists())
        .map(|path| file_status(&path));

    Inspection {
        config_path: config_path.display().to_string(),
        config_valid: config_error.is_none(),
        config_error,
        network,
        database,
        referenced_files: referenced,
        coordinator_backup,
        state_file,
        unperformed_checks: vec![
            "coordinator firmware/version, NV layout and backup version were not read (no serial access)"
                .to_string(),
            "configured network key/PAN/channel were not compared with coordinator NV"
                .to_string(),
            "device definitions and converter execution were not evaluated".to_string(),
        ],
        warnings,
        blockers,
    }
}

fn permission_error(path: &Path, mode: u32) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
    }
    Ok(())
}

fn collect_snapshot_files(config_path: &Path, database_path: Option<&Path>) -> Vec<PathBuf> {
    let config_dir = config_path.parent().unwrap_or(Path::new("."));
    let mut files = vec![config_path.to_path_buf()];
    files.extend(referenced_files(config_path));
    if let Some(database) = database_path {
        files.push(database.to_path_buf());
        if let Some(parent) = database.parent() {
            files.push(parent.join("coordinator_backup.json"));
            files.push(parent.join("state.json"));
        }
    }
    files.push(config_dir.join("coordinator_backup.json"));
    files.push(config_dir.join("state.json"));
    files.retain(|path| path.exists() && path.is_file());
    files.sort();
    files.dedup();
    files
}

/// Copy migration inputs into a new timestamped directory.
///
/// The destination must be outside the application's configuration directory;
/// this prevents a snapshot from being written into the live data directory by
/// accident. The manifest is written last, so a directory without a manifest is
/// an incomplete snapshot.
pub fn create_snapshot(config_path: &Path, destination: &Path) -> Result<Snapshot> {
    if !config_path.exists() {
        return Err(Error::Config(format!(
            "configuration file does not exist: {}",
            config_path.display()
        )));
    }
    let config_dir = std::path::absolute(config_path.parent().unwrap_or(Path::new(".")))?;
    let destination = std::path::absolute(destination)?;
    if destination.starts_with(&config_dir) {
        return Err(Error::Config(format!(
            "snapshot destination must be outside the application directory {}",
            config_dir.display()
        )));
    }
    std::fs::create_dir_all(&destination)?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut snapshot_dir = destination.join(format!("snapshot-{now}-{}", std::process::id()));
    let mut suffix = 1;
    while snapshot_dir.exists() {
        snapshot_dir = destination.join(format!("snapshot-{now}-{}-{suffix}", std::process::id()));
        suffix += 1;
    }
    std::fs::create_dir(&snapshot_dir)?;
    permission_error(&snapshot_dir, 0o700)?;

    let database_path = find_database(config_path);
    let sources = collect_snapshot_files(config_path, database_path.as_deref());
    let mut files = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        let name = source
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("file");
        let relative = format!("{index:02}-{name}");
        let target = snapshot_dir.join(&relative);
        let bytes = std::fs::read(source)?;
        std::fs::write(&target, &bytes)?;
        permission_error(&target, 0o600)?;
        // Verify the copy instead of claiming success after a partial write.
        let copied = std::fs::read(&target)?;
        let expected = checksum_string(&bytes);
        if checksum_string(&copied) != expected {
            return Err(Error::Config(format!(
                "snapshot copy verification failed for {}",
                source.display()
            )));
        }
        files.push(SnapshotFile {
            source: source.display().to_string(),
            snapshot: relative,
            bytes: bytes.len() as u64,
            checksum: expected,
        });
    }

    let snapshot = Snapshot {
        directory: snapshot_dir.display().to_string(),
        created_unix_seconds: now,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        files,
        warnings: vec![
            "checksums are FNV-1a integrity checks, not cryptographic signatures".to_string(),
            "coordinator NV was not read; coordinator_backup.json is copied only if present"
                .to_string(),
        ],
    };
    let manifest = serde_json::to_vec_pretty(&snapshot)?;
    let manifest_path = snapshot_dir.join("manifest.json");
    std::fs::write(&manifest_path, manifest)?;
    permission_error(&manifest_path, 0o600)?;
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("z2m_migration_{name}_{}_{id}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_config(dir: &Path) -> PathBuf {
        let path = dir.join("configuration.yaml");
        std::fs::write(
            &path,
            "serial:\n  port: /dev/ttyUSB0\nmqtt:\n  server: localhost\n  base_topic: zigbee2mqtt\n",
        )
        .unwrap();
        path
    }

    #[test]
    fn checksum_matches_known_fnv1a_vector() {
        assert_eq!(checksum(b"hello"), 0xa430_d846_80aa_bd0b);
        assert_eq!(checksum_string(b"hello"), "fnv1a64:a430d84680aabd0b");
    }

    #[test]
    fn inspect_reports_health_and_blockers_without_touching_files() {
        let dir = temp_dir("inspect");
        let config = write_config(&dir);
        let db = dir.join("database.db");
        std::fs::write(
            &db,
            concat!(
                r#"{"id":1,"type":"Coordinator","ieeeAddr":"0x00124b00120144ae"}"#,
                "\n",
                r#"{"id":2,"type":"Router","ieeeAddr":"0xec1bbdfffeaa66db","modelId":"TRADFRI bulb","interviewCompleted":true}"#,
                "\n",
                r#"{"id":2,"type":"Group","groupID":1,"members":[]}"#,
                "\n",
                "not-json\n"
            ),
        )
        .unwrap();
        let before = std::fs::read(&db).unwrap();

        let report = inspect(&config);

        assert!(report.config_valid);
        assert_eq!(report.database.devices, 1);
        assert_eq!(report.database.groups, 1);
        assert_eq!(report.database.interviewed, 1);
        assert_eq!(
            report.database.coordinator_ieee.as_deref(),
            Some("0x00124b00120144ae")
        );
        assert_eq!(report.database.duplicate_ids, vec![2]);
        assert_eq!(report.database.invalid_lines, vec![4]);
        assert!(report
            .blockers
            .iter()
            .any(|b| b.contains("duplicate record ids")));
        assert!(report.blockers.iter().any(|b| b.contains("unparseable")));
        assert_eq!(
            std::fs::read(&db).unwrap(),
            before,
            "inspect must be read-only"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn inspect_flags_missing_referenced_file() {
        let dir = temp_dir("missing_ref");
        let config = dir.join("configuration.yaml");
        std::fs::write(
            &config,
            "devices: missing.yaml\nserial:\n  port: /dev/ttyUSB0\nmqtt:\n  server: localhost\n  base_topic: zigbee2mqtt\n",
        )
        .unwrap();
        let report = inspect(&config);
        assert!(report
            .blockers
            .iter()
            .any(|blocker| blocker.contains("missing.yaml")));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn snapshot_copies_inputs_with_checksums_and_unique_directories() {
        let dir = temp_dir("snapshot");
        let app = dir.join("app");
        std::fs::create_dir_all(&app).unwrap();
        let config = app.join("configuration.yaml");
        std::fs::write(
            &config,
            "devices: devices.yaml\nmqtt:\n  server: localhost\n  base_topic: zigbee2mqtt\n  password: '!secret secret password'\n",
        )
        .unwrap();
        std::fs::write(
            app.join("devices.yaml"),
            "0xec1bbdfffeaa66db:\n  friendly_name: bulb\n",
        )
        .unwrap();
        std::fs::write(app.join("secret.yaml"), "password: hunter2\n").unwrap();
        std::fs::write(
            app.join("database.db"),
            "{\"id\":1,\"type\":\"Coordinator\"}\n",
        )
        .unwrap();
        std::fs::write(app.join("coordinator_backup.json"), "{}").unwrap();
        std::fs::write(app.join("state.json"), "{}").unwrap();
        let destination = dir.join("snapshots");

        let first = create_snapshot(&config, &destination).unwrap();
        let second = create_snapshot(&config, &destination).unwrap();

        assert_ne!(first.directory, second.directory);
        assert!(
            first.files.len() >= 5,
            "config, devices, secret, db and backup"
        );
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(Path::new(&first.directory).join("manifest.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            manifest["files"].as_array().unwrap().len(),
            first.files.len()
        );
        for file in &first.files {
            let copied = std::fs::read(Path::new(&first.directory).join(&file.snapshot)).unwrap();
            assert_eq!(checksum_string(&copied), file.checksum);
            let original = std::fs::read(&file.source).unwrap();
            assert_eq!(copied, original);
        }
        assert!(first
            .files
            .iter()
            .any(|file| file.source.ends_with("secret.yaml")));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(Path::new(&first.directory).join("manifest.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn snapshot_refuses_destination_inside_application_directory() {
        let dir = temp_dir("snapshot_inside");
        let config = write_config(&dir);
        let error = create_snapshot(&config, &dir.join("backups")).unwrap_err();
        assert!(error
            .to_string()
            .contains("outside the application directory"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
