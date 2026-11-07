// Original: .upstream/zigbee2mqtt/lib/util/settingsMigration.ts
// TODO: port the migration rules.
//
// Upstream mutates the persisted settings module and writes backup/notes files.
// The Rust port takes the settings document explicitly and returns the notes
// that upstream writes to `migration-X-to-Y.log`.

use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct MigrationOutcome {
    pub notes: Vec<String>,
    pub errors: Vec<String>,
}

/// Upstream: `migrateIfNecessary()`.
///
/// `current_version` is the settable `settings.CURRENT_VERSION` used by the
/// tests to stop after a given version. `state` is the `state.json` document
/// used by the v4 -> v5 migration.
pub fn migrate_if_necessary(
    settings: &mut Value,
    current_version: u32,
    state: Option<&mut Value>,
) -> Result<MigrationOutcome, String> {
    todo!()
}
