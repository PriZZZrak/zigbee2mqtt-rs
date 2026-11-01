# Evidence: database.db portability (STORAGE-001)

## Goal
1:1 file portability between original zigbee-herdsman database and Rust fork — file written by one must be readable by the other without data loss, including Coordinator/Router/EndDevice/GreenPower/Group, all fields, and atomic write semantics.

## Source pins
- zigbee2mqtt 344b6e03 (2.14.2) uses herdsman Database via `lib/zigbee.ts: data.joinPath("database.db")`
- zigbee-herdsman 3cb89ad (10.10.0) `src/controller/database.ts:19-122` — open/read split("\n")/JSON.parse per line, Map<id, DatabaseEntry>, write via `.tmp` + `renameSync(tmp, dateTmp)` if exists + `openSync/writeFileSync/fsync/close/renameSync(tmp, path)`
- zigbee-herdsman `src/controller/model/device.ts:596-699` — fromDatabaseEntry/toDatabaseEntry fields: `id, type, ieeeAddr, nwkAddr, manufId, manufName, powerSource, modelId, epList, endpoints{profId,epId,devId,inClusterList,outClusterList,clusters,binds,configuredReportings,meta}, appVersion, stackVersion, hwVersion, dateCode, swBuildId, zclVersion, interviewCompleted, interviewState, meta, lastSeen, checkinInterval, gpSecurityKey, scheduledOta`
- Group: `src/controller/model/group.ts: toDatabaseEntry` — `id, type:"Group", groupID, members, meta`
- fetched 2026-11-01T00:00Z DRIFT_DETECTED (latest heads newer)

## Upstream execution path
- `Database.open(path)` → if exists, readFileSync, split("\n"), ignore empty, JSON.parse, if `Number.isFinite(json.id)` store, track maxId, on parse error `logger.error Corrupted database line, ignoring`
- `getEntriesIterator(type[])` → filter by entry.type
- `insert(entry)` → throw if exists, set, write()
- `update(entry, write)` → throw if not exists, set, conditional write
- `remove(id)` → delete, write
- `newID()` → ++maxId
- `write()` → lines = JSON.stringify(entry)+"\n" for each entry, handle existing `.tmp` rename to `.tmp.<ISOdate>`, open `.tmp`, write `lines.slice(0,-1)` (no trailing newline if empty? actually removes last "\n"), fsync, close, rename to `path`
- Device load: `Device.loadFromDatabaseIfNecessary()` iterates `getEntriesIterator(["Coordinator",...])`, `Device.fromDatabaseEntry(entry)` maps fields, `interviewState` migration from `interviewCompleted` if missing.

## Rust mapping (before)
- `src/database.rs: load_database()` reads NDJSON, deserializes `DbEntry {device_type, ieee_addr, nwk_addr, manuf_name, power_source, model_id, sw_build_id, endpoints{profId,epId,devId,inClusterList,outClusterList}, interview_completed}` — missing `id, manufId, epList, appVersion/stackVersion/hwVersion/dateCode/zclVersion, interviewState, meta, lastSeen, clusters/binds/configuredReportings`
- Skips `Coordinator` and `Group` (`ieee_addr.is_empty()`), no Group handling
- No write path — in-memory `DeviceRegistry` only, no `Database.write()`, no atomic tmp+fsync
- `interview_completed` bool only, not `interviewState` enum.

## Gaps
- Missing fields → round-trip loses `id` (critical for update/remove), `manufId`, `epList`, version fields, `meta`, `lastSeen`, `interviewState`
- No Group persistence
- No atomic write — risk of corruption on power loss (vs upstream fsync)
- No handling of `interviewCompleted` ↔ `interviewState` migration (upstream keeps both for downgrade)
- No preservation of unknown extra fields (`[s:string]:any`)

## Implementation 2026-11-01 (c79faa8)
- Added `PersistentDatabase` with `Map<u32, Value>` + `open`/`newID`/`insert`/`update`/`remove`/`write` (tmp+fsync+rename, dated .tmp on collision) and `sync_devices` preserving Coordinator/Group
- Extended `DbEntry` to `id, manufId, epList, interviewState, lastSeen, flatten extra` and `Device.database_id`
- `Device` now maps `interviewState`↔`interviewCompleted` both ways
- Tests 17 (was 12): round-trip, interviewState both fields, Group preserved

## Implementation 2026-11-02 (wired + groups)
- `Bridge` now holds `Arc<Mutex<PersistentDatabase>>` opened from `find_database(config_path)` or `database.db` next to config; `persist_database()` called after `DeviceJoined`, `DeviceLeft`, `NodeDescRsp` (manufId), `SimpleDescRsp` (interview complete), `device/rename`, `device/remove` and `group/*`
- `Bridge::new` now loads groups from `PersistentDatabase::load_groups()` into `GroupRegistry` (preserving `friendly_name` via extra field) and `persist_database()` now calls `sync_devices` (with preservation of `meta/lastSeen/appVersion` etc. via `orig_by_ieee` merge) + `sync_groups` (writes `id, type Group, groupID, friendly_name, members, meta`)
- `src/groups.rs` added `GroupRegistry` with `inner/by_name/next_id` pub(crate) for DB sync, `src/database.rs` added `sync_groups`/`load_groups` and `json` import
- `Device.database_id` preserved across restarts, `sync_devices` now merges original `meta/lastSeen/appVersion/stackVersion/hwVersion/dateCode/zclVersion` instead of resetting

## Implementation 2026-11-02 (yaml groups)
- Added `groups: HashMap<String, GroupConfig>` to `Config` + `GroupConfig {friendly_name}`; `Config.save()` atomic write via `.yaml.tmp`
- `Bridge::new` now overlays `cfg.groups` onto `GroupRegistry` (friendly_name authoritative from yaml, members from DB), `persist_groups_config()` updates `configuration.yaml` on `group/add|remove|rename|members/*` plus `persist_database` for DB members
- `database.db` + `configuration.yaml` now both portable: groups survive DB↔yaml round-trip, device `friendly_name` already via `devices` map

## Still not as original (2026-11-02, after clusters preservation)
- `Coordinator` entry still recreated with defaults if missing, not fully preserving original `manufId/epList/meta` when file has no Coordinator (rare, but now preserved if orig exists)
- `checkinInterval`, `gpSecurityKey`, `scheduledOta` now preserved from orig, but not updated from live Device (upstream updates these on interview, we keep orig value)
- `appVersion/stackVersion/hwVersion/dateCode/zclVersion` now preserved, but not refreshed from live `read` (upstream updates on SimpleDesc/DeviceAnnounce)
- `endpoints[].clusters/binds/configuredReportings/meta` now preserved per-endpoint from orig, but new endpoints from Device still get `{} / []` defaults (upstream would have actual ZCL clusters)
- No `Device.save()` debounce / `update(..., write:false)` batching as in `Entity.database.update` — we write immediately on every join/leave/rename
- `interviewState` mapping only `SUCCESSFUL`/`FAILED`, not `Pending`/`InProgress` + quirks for Tuya `TS*` etc.
- No handling of `GreenPower` type entries (filtered, upstream stores with `type GreenPower`)
- `Group` `friendly_name` duplicated in DB extra field for portability — upstream stores only in yaml, we store in both (acceptable, but file diff vs upstream when round-tripping pure DB)
- `configuration.yaml` groups now written via `Config.save()` with `serde_yml` dump — loses comments/formatting and `!secret`/`!env` handling vs `yaml.writeIfChanged` with `js-yaml` (upstream preserves `!secret` refs via `updateIfChanged`)
- No `yaml` handling for `devices` separate file (`devices.yaml` via `actual[type]` string) as in `settings.write()`

## Implementation 2026-11-02 (batching, memory)
- `Bridge::persist_database()` now debounced: in-memory `sync_devices`+`sync_groups` immediately, disk `write()` via `db_write_pending` flag + `tokio::spawn` 500ms delayed single write, final flush on shutdown (`run` end) — mirrors `Entity.database.update(write:false)` / `update(write:true)` batching, saves memory/I/O (was 2-3 fsync per interview for 2-EP thermostat, now 1)

## Still not as original (2026-11-02, after batching)
- `Coordinator` still defaults if missing
- `endpoints[].clusters/binds` now preserved, but new endpoints still defaults
- `checkinInterval/gpSecurityKey/scheduledOta` preserved but not live-updated
- `appVersion` etc. preserved but not refreshed
- `interviewState` only SUCCESSFUL/FAILED
- `GreenPower` filtered
- `Group` duplicate friendly_name
- `configuration.yaml` loses comments/`!secret`
- No `devices.yaml` separate file
- Batching window 500ms vs upstream immediate `write()` for `update(write:true)` — crash within 500ms may lose last batch (upstream would have fsync immediately)

## Hardware
- No hardware required — file I/O only, differential test with Node `Database.open` fixture.
