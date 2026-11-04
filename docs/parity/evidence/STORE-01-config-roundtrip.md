# STORE-01 — lossless configuration write

Rust implementation: `src/config.rs:save` (working tree after `07bb90f`).
Behavior: CONFIG-003; related CONFIG-004 and plan STORE-01.

## Source contract

Pinned app `344b6e03b43644c97154a72ca9eae17193c553a2`,
`lib/util/settings.ts`:

- `read()` (332–382) parses the whole YAML document into `Partial<Settings>`,
  resolves `!secret` values in five known paths (`mqtt.server`, `mqtt.user`,
  `mqtt.password`, `advanced.network_key`, `frontend.auth_token`) and expands
  `devices`/`groups` string-or-array file references into objects. Other fields
  are carried through unchanged.
- `getPersistedSettings()` (447–453) returns that document, explicitly excluding
  defaults merged at startup.
- `write()` (206–263) deep-copies the persisted document, updates referenced
  secret files and restores their references, writes `devices`/`groups` to the
  first configured file while excluding keys defined in later files, re-applies
  environment variables, and writes the main document.

Consequence: a rename or group change must not discard unknown sections,
unknown per-entity options, the literal `mqtt.user` key, or the original MQTT
URI form. Reference tests: `test/settings.test.ts`,
`test/settingsMigration.test.ts` (fixtures still need a full read for the cases
below before this row can become VERIFIED).

## Rust change

`Config::save` no longer serializes only the typed structs. It:

1. starts from the parsed original document and overlays only top-level sections
   that the runtime does not mutate, preserving their representation;
2. merges `devices`/`groups` per entity, using the original entry (or referenced
   file) as the base and applying only `Some` typed options, so unknown options
   survive;
3. reads referenced device/group files as the merge base, while retaining the
   existing first-file write and later-file exclusion behavior;
4. takes secret-file values from the resolved typed view, so a restored
   `!secret` reference is never written into the secret file as its own value.

## Tests added

`src/config.rs`:

- `save_preserves_unknown_configuration_and_entity_options`
- `save_preserves_device_options_in_referenced_file`

They assert preservation of unknown top-level sections, unknown serial/MQTT/
frontend options, per-device and per-group options, `mqtt.user`, an `mqtts://`
URI, and a device defined in a referenced file after changing friendly names.

## Known gaps (not verified)

- No Node reference round-trip yet; tests are Rust-local oracle checks.
- `!secret` write path has no regression test in this change.
- Environment-variable precedence (`ZIGBEE2MQTT_CONFIG_*`) is not implemented in
  Rust and is not claimed.
- `mqtt.user` is preserved on disk but still not used for authentication
  (CONFIG-004 remains open).
- Comment/formatting preservation is not provided by `serde_yml`; only semantic
  preservation is claimed.
- Multi-file device arrays, duplicate/invalid names and ENOSPC I/O failures are
  still open cases (plan STORE-01/03).
