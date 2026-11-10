// Original: .upstream/zigbee-herdsman/src/controller/database.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::utils::logger::logger;

const NS: &str = "zh:controller:database";

/// Upstream `DatabaseEntry` — a raw JSON object with a numeric `id` and `type`.
pub type DatabaseEntry = Value;

/// Upstream `DeviceType` — tstype.ts:38.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Coordinator,
    Router,
    EndDevice,
    Unknown,
    GreenPower,
}

impl DeviceType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Coordinator => "Coordinator",
            Self::Router => "Router",
            Self::EndDevice => "EndDevice",
            Self::Unknown => "Unknown",
            Self::GreenPower => "GreenPower",
        }
    }
}

/// Upstream `EntityType` — tstype.ts:40.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    DeviceType(DeviceType),
    Group,
}

impl EntityType {
    fn as_str(self) -> &'static str {
        match self {
            Self::DeviceType(device_type) => device_type.as_str(),
            Self::Group => "Group",
        }
    }
}

/// Upstream `Database`: NDJSON file with numeric ids.
///
/// Entries keep insertion order like the upstream `Map` (it is observable in
/// `write()` and `getEntriesIterator`).
#[derive(Debug)]
pub struct Database {
    entries: Vec<(u64, DatabaseEntry)>,
    path: PathBuf,
    max_id: u64,
}

impl Database {
    /// Upstream `Database.open(path)` — database.ts:26.
    pub fn open(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        let mut entries: Vec<(u64, DatabaseEntry)> = Vec::new();
        let mut max_id = 0u64;

        if let Ok(file) = std::fs::read_to_string(&path) {
            for row in file.split('\n') {
                if row.is_empty() {
                    continue;
                }
                match serde_json::from_str::<DatabaseEntry>(row) {
                    Ok(json) => {
                        if let Some(id) = json_id(&json) {
                            if let Some(existing) = entries.iter_mut().find(|(key, _)| *key == id) {
                                existing.1 = json;
                            } else {
                                entries.push((id, json));
                            }
                            if id > max_id {
                                max_id = id;
                            }
                        }
                    }
                    Err(error) => {
                        logger().error(&format!("Corrupted database line, ignoring. {error}"), NS);
                    }
                }
            }
        }

        Self {
            entries,
            path,
            max_id,
        }
    }

    /// Upstream `getEntriesIterator(type)` — database.ts:57.
    pub fn get_entries_iterator<'a>(
        &'a self,
        types: &'a [EntityType],
    ) -> impl Iterator<Item = &'a DatabaseEntry> + 'a {
        self.entries
            .iter()
            .map(|(_, entry)| entry)
            .filter(move |entry| {
                entry
                    .get("type")
                    .and_then(Value::as_str)
                    .is_some_and(|kind| types.iter().any(|candidate| candidate.as_str() == kind))
            })
    }

    /// Upstream `insert(databaseEntry)` — database.ts:65.
    pub fn insert(&mut self, database_entry: DatabaseEntry) -> Result<(), String> {
        let id = json_id(&database_entry).ok_or_else(|| {
            format!(
                "DatabaseEntry with ID '{}' already exists",
                id_display(&database_entry)
            )
        })?;
        if self.entries.iter().any(|(key, _)| *key == id) {
            return Err(format!("DatabaseEntry with ID '{id}' already exists"));
        }
        self.entries.push((id, database_entry));
        self.write()
    }

    /// Upstream `update(databaseEntry, write)` — database.ts:73.
    pub fn update(&mut self, database_entry: DatabaseEntry, write: bool) -> Result<(), String> {
        let id = json_id(&database_entry).ok_or_else(|| {
            format!(
                "DatabaseEntry with ID '{}' does not exist",
                id_display(&database_entry)
            )
        })?;
        let Some(entry) = self.entries.iter_mut().find(|(key, _)| *key == id) else {
            return Err(format!("DatabaseEntry with ID '{id}' does not exist"));
        };
        entry.1 = database_entry;
        if write {
            self.write()?;
        }
        Ok(())
    }

    /// Upstream `remove(id)` — database.ts:83.
    pub fn remove(&mut self, id: u64) -> Result<(), String> {
        let Some(position) = self.entries.iter().position(|(key, _)| *key == id) else {
            return Err(format!("DatabaseEntry with ID '{id}' does not exist"));
        };
        self.entries.remove(position);
        self.write()
    }

    /// Upstream `has(id)` — database.ts:91.
    pub fn has(&self, id: u64) -> bool {
        self.entries.iter().any(|(key, _)| *key == id)
    }

    /// Upstream `newID()` — database.ts:95.
    pub fn new_id(&mut self) -> u64 {
        self.max_id += 1;
        self.max_id
    }

    /// Upstream `write()` — database.ts:100.
    pub fn write(&self) -> Result<(), String> {
        logger().debug(
            &format!("Writing database to '{}'", self.path.display()),
            NS,
        );
        let mut lines = String::new();
        for (_, entry) in &self.entries {
            lines.push_str(&serde_json::to_string(entry).unwrap_or_default());
            lines.push('\n');
        }

        let tmp_path = PathBuf::from(format!("{}.tmp", self.path.display()));

        // If there already exists a database.db.tmp, rename it away.
        if tmp_path.exists() {
            let date = crate::utils::datetime::now_iso8601();
            let date_tmp_path =
                PathBuf::from(format!("{}.{}", tmp_path.display(), date.replace(':', "-")));
            if std::fs::rename(&tmp_path, &date_tmp_path).is_ok() {
                logger().warning(
                    &format!(
                        "Found '{}' when writing database, indicating past write failure; renamed it to '{}'",
                        tmp_path.display(),
                        date_tmp_path.display()
                    ),
                    NS,
                );
            }
        }

        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_path)
            .map_err(|error| error.to_string())?;
        // Remove the last newline, no effect if empty string.
        let content = lines.strip_suffix('\n').unwrap_or(&lines);
        {
            use std::io::Write;
            let mut file = file;
            file.write_all(content.as_bytes())
                .map_err(|error| error.to_string())?;
            // Ensure the file is on disk (upstream fsyncSync).
            file.sync_all().map_err(|error| error.to_string())?;
        }
        std::fs::rename(&tmp_path, &self.path).map_err(|error| error.to_string())
    }

    /// Upstream `clear()` — database.ts:132.
    pub fn clear(&mut self) -> Result<(), String> {
        logger().debug(
            &format!("Clearing database '{}'...", self.path.display()),
            NS,
        );
        self.entries.clear();
        self.write()
    }

    /// Not upstream: convenience used by tooling/tests to inspect every entry.
    pub fn all_entries(&self) -> impl Iterator<Item = &DatabaseEntry> {
        self.entries.iter().map(|(_, entry)| entry)
    }

    /// Not upstream: convenience used by tooling/tests.
    pub fn get(&self, id: u64) -> Option<&DatabaseEntry> {
        self.entries
            .iter()
            .find(|(key, _)| *key == id)
            .map(|(_, entry)| entry)
    }

    /// Not upstream: convenience used by tooling/tests.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Upstream `Number.isFinite(json.id)` — accepts any finite number.
fn json_id(json: &Value) -> Option<u64> {
    let id = json.get("id")?;
    let value = id.as_f64()?;
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    Some(value as u64)
}

fn id_display(entry: &Value) -> String {
    entry
        .get("id")
        .map(|value| value.to_string())
        .unwrap_or_else(|| "undefined".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "z2m-db-{name}-{}",
            crate::utils::datetime::now_ms()
        ));
        std::fs::create_dir_all(&dir).expect("mkdir");
        dir.join("database.db")
    }

    #[test]
    fn open_missing_is_empty_and_new_id_starts_at_one() {
        let path = unique_path("missing");
        let mut database = Database::open(&path);
        assert!(!database.has(1));
        assert_eq!(database.new_id(), 1);
        assert_eq!(database.new_id(), 2);
    }

    #[test]
    fn insert_update_remove_and_reload() {
        let path = unique_path("crud");
        let mut database = Database::open(&path);
        let first = serde_json::json!({"id": 5, "type": "Device", "ieeeAddr": "0x01"});
        let second = serde_json::json!({"id": 2, "type": "Group", "groupID": 1});
        database.insert(first).expect("insert");
        database.insert(second).expect("insert");

        assert!(database
            .insert(serde_json::json!({"id": 5, "type": "Device"}))
            .is_err());
        assert!(database
            .update(serde_json::json!({"id": 99, "type": "Device"}), false)
            .is_err());
        assert!(database.remove(99).is_err());

        database
            .update(
                serde_json::json!({"id": 5, "type": "Device", "ieeeAddr": "0x02"}),
                true,
            )
            .expect("update");

        let reopened = Database::open(&path);
        // Upstream Map preserves insertion order, not id order.
        let ids: Vec<u64> = reopened.all_entries().filter_map(json_id).collect();
        assert_eq!(ids, vec![5, 2]);
        assert_eq!(
            reopened.get(5).and_then(|entry| entry.get("ieeeAddr")),
            Some(&serde_json::json!("0x02"))
        );

        database.remove(2).expect("remove");
        let reopened = Database::open(&path);
        assert_eq!(reopened.all_entries().count(), 1);

        // Upstream `newID` keeps increasing across the loaded entries.
        let mut database = reopened;
        assert_eq!(database.new_id(), 6);
    }

    #[test]
    fn clear_keeps_max_id() {
        let path = unique_path("clear");
        // `maxId` is only derived on open/newID, not on insert.
        std::fs::write(&path, "{\"id\":7,\"type\":\"Device\"}\n").expect("write");
        let mut database = Database::open(&path);
        database.clear().expect("clear");
        assert!(!database.has(7));
        assert_eq!(database.new_id(), 8);
    }

    #[test]
    fn entries_filter_matches_upstream_types() {
        let path = unique_path("filter");
        let mut database = Database::open(&path);
        database
            .insert(serde_json::json!({"id": 1, "type": "Router"}))
            .expect("insert");
        database
            .insert(serde_json::json!({"id": 2, "type": "Group"}))
            .expect("insert");

        let devices: Vec<_> = database
            .get_entries_iterator(&[EntityType::DeviceType(DeviceType::Router)])
            .collect();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0]["id"], serde_json::json!(1));

        let none: Vec<_> = database.get_entries_iterator(&[]).collect();
        assert!(none.is_empty());
    }

    #[test]
    fn corrupted_lines_are_ignored() {
        let path = unique_path("corrupt");
        std::fs::write(
            &path,
            "{\"id\":1,\"type\":\"Device\"}\nnot-json\n{\"id\":2,\"type\":\"Group\"}\n",
        )
        .expect("write");
        let database = Database::open(&path);
        assert_eq!(database.all_entries().count(), 2);
        assert!(database.has(1));
        assert!(database.has(2));
    }
}
