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

## Plan
- Rust `Database` struct mirroring upstream: `Map<u32, Value> + path + maxId`, `open`, `getEntriesIterator`, `insert/update/remove/newID/write` with tmp+fsync+rename, corrupted line handling via tracing::error
- `DbEntry` with `#[serde(flatten)] extra: HashMap<String, Value>` to preserve unknown fields, plus explicit `id, type, ieeeAddr, nwkAddr, manufId, manufName, powerSource, modelId, epList, endpoints, appVersion, stackVersion, hwVersion, dateCode, swBuildId, zclVersion, interviewCompleted, interviewState, meta, lastSeen`
- Device ↔ DatabaseEntry conversion preserving all, mapping `interview_complete` ↔ `interviewState`/`interviewCompleted` both
- Group ↔ DatabaseEntry similarly, `GroupRegistry` ↔ database
- Tests: golden fixtures from upstream `test/controller.test.ts` database examples, corrupted line test, round-trip `open→insert→write→open` and `load_database` vs original file, NDJSON no trailing newline case

## Hardware
- No hardware required — file I/O only, differential test with Node `Database.open` fixture.
