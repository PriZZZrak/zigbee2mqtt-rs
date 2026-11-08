// Original: .upstream/zigbee-herdsman/src/controller/database.ts

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Upstream `DatabaseEntry` (raw JSON object with a numeric `id`).
pub type DatabaseEntry = Value;

/// Upstream `Database`: NDJSON file with numeric ids.
#[derive(Debug)]
pub struct Database {
    entries: BTreeMap<u64, DatabaseEntry>,
    path: PathBuf,
    max_id: u64,
}

impl Database {
    /// Upstream `Database.open(path)`.
    pub fn open(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        let mut entries = BTreeMap::new();
        let mut max_id = 0;
        if let Ok(content) = std::fs::read_to_string(&path) {
            for row in content.lines().filter(|row| !row.is_empty()) {
                let Ok(json) = serde_json::from_str::<DatabaseEntry>(row) else {
                    continue;
                };
                if let Some(id) = json.get("id").and_then(Value::as_u64) {
                    max_id = max_id.max(id);
                    entries.insert(id, json);
                }
            }
        }
        Self {
            entries,
            path,
            max_id,
        }
    }

    /// Upstream `getEntriesIterator(types)`.
    pub fn entries<'a>(
        &'a self,
        types: &'a [&'a str],
    ) -> impl Iterator<Item = &'a DatabaseEntry> + 'a {
        self.entries.values().filter(move |entry| {
            types.is_empty()
                || entry
                    .get("type")
                    .and_then(Value::as_str)
                    .is_some_and(|kind| types.contains(&kind))
        })
    }

    pub fn all_entries(&self) -> impl Iterator<Item = &DatabaseEntry> {
        self.entries.values()
    }

    /// Upstream `insert(databaseEntry)`.
    pub fn insert(&mut self, entry: DatabaseEntry) -> Result<(), String> {
        let id = entry
            .get("id")
            .and_then(Value::as_u64)
            .ok_or_else(|| "DatabaseEntry has no numeric id".to_string())?;
        if self.entries.contains_key(&id) {
            return Err(format!("DatabaseEntry with ID '{id}' already exists"));
        }
        self.entries.insert(id, entry);
        self.write()
    }

    /// Upstream `update(databaseEntry, write)`.
    pub fn update(&mut self, entry: DatabaseEntry, write: bool) -> Result<(), String> {
        let id = entry
            .get("id")
            .and_then(Value::as_u64)
            .ok_or_else(|| "DatabaseEntry has no numeric id".to_string())?;
        if !self.entries.contains_key(&id) {
            return Err(format!("DatabaseEntry with ID '{id}' does not exist"));
        }
        self.entries.insert(id, entry);
        if write {
            self.write()?;
        }
        Ok(())
    }

    /// Upstream `remove(id)`.
    pub fn remove(&mut self, id: u64) -> Result<(), String> {
        if self.entries.remove(&id).is_none() {
            return Err(format!("DatabaseEntry with ID '{id}' does not exist"));
        }
        self.write()
    }

    pub fn has(&self, id: u64) -> bool {
        self.entries.contains_key(&id)
    }

    pub fn get(&self, id: u64) -> Option<&DatabaseEntry> {
        self.entries.get(&id)
    }

    /// Upstream `newID()`.
    pub fn new_id(&mut self) -> u64 {
        self.max_id += 1;
        self.max_id
    }

    /// Upstream `clear()`.
    pub fn clear(&mut self) -> Result<(), String> {
        self.entries.clear();
        self.max_id = 0;
        self.write()
    }

    /// Upstream `write()`: atomic tmp file + rename (best effort).
    pub fn write(&self) -> Result<(), String> {
        let lines = self
            .entries
            .values()
            .map(|entry| serde_json::to_string(entry).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n");
        let tmp_path = self.path.with_extension(format!(
            "{}.tmp",
            self.path
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or("db")
        ));
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        std::fs::write(&tmp_path, lines).map_err(|error| error.to_string())?;
        // fsync to survive power loss (upstream fsyncSync).
        if let Ok(file) = std::fs::File::open(&tmp_path) {
            let _ = file.sync_all();
        }
        std::fs::rename(&tmp_path, &self.path).map_err(|error| error.to_string())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
