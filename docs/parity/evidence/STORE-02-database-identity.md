# STORE-02 — database record identity and fidelity

Rust implementation: `src/database.rs` (`sync_devices`, `sync_groups`),
`src/devices/mod.rs` (`set_database_id`), `src/bridge.rs`
(`persist_database`). Behavior: STORAGE-002, STORAGE-003, STORAGE-004; plan
STORE-02.

## Source contract

Pinned herdsman `3cb89ad906bd9b2de8b3dfed20d5a21070a467d2`:

- `src/controller/database.ts` inserts/updates/removes addressable records by a
  persistent numeric `id`, writing NDJSON after mutations.
- `src/controller/model/device.ts`, `endpoint.ts` and `group.ts` serialize entity
  state into those records. `Group` keeps its database `id` and group membership
  separately from the Zigbee `groupID`; groups are looked up by identity rather
  than re-created.
- `test/controller.test.ts` and related model tests cover database insert/lookup
  and group persistence (full fixtures still need review for edge cases).

Consequence: repeated synchronization must reuse the same numeric record for an
IEEE address or group, and metadata attached to that record must not be dropped
by a routine state update.

## Rust change

1. `sync_devices` now returns `(IeeeAddr, u32)` assignments. `persist_database`
   writes them back through `DeviceRegistry::set_database_id`, so a device that
   was not loaded from `database.db` keeps its allocated id instead of receiving
   a new one on every flush.
2. `sync_groups` captures existing group records before removing group entries.
   It reuses the prior record and its `id`/`meta`/unknown fields for the same
   `groupID`, and allocates only for genuinely new groups without recycling ids
   already in use.
3. Device rewrite preserves `epList` endpoints that are still present in the
   endpoint map (notably 242) instead of rebuilding the list from the live
   registry only.
4. A non-complete interview preserves a persisted in-progress `interviewState`
   instead of rewriting it to `FAILED`.
5. `swBuildId` is written from the live device (falling back to the old record),
   so a newly imported software build id is no longer dropped.

## Tests added

`src/database.rs`:

- `sync_groups_reuses_database_id_and_meta` — two syncs keep id 10, `meta` and
  members.
- `sync_devices_returns_ids_that_stay_stable_after_write_back` — first
  assignment avoids the coordinator id; after write-back the second sync reuses
  it and the record count stays stable.
- `sync_devices_preserves_ep_list_interview_state_and_sw_build_id` — endpoint
  242 remains in `epList`/`endpoints`, `IN_PROGRESS` is not downgraded, and the
  new `swBuildId` is serialized.

## Known gaps (not verified)

- No Node-produced database opened by Rust and re-opened by herdsman as a
  differential oracle yet; the tests are Rust-local.
- `type` is still inferred from `powerSource` for new devices; interview state
  beyond the preserved string, binds/reportings/meta for new endpoints, and full
  group/scene persistence are not addressed.
- Duplicate/malformed record ids, coordinator id collisions and unrelated
  entries are not covered.
- The 500 ms debounce can still lose an accepted update on abrupt shutdown
  (STORE-03).
