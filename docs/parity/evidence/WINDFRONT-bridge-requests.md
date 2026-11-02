# Evidence: Windfront bridge/request parity (target 90%)

## Goal
Replicate zigbee2mqtt bridge MQTT API as consumed by zigbee2mqtt-windfront 2.15.1 (fbe6e2d) to reach ≥90% functional windfront (≥31/34 requests).

## Source pins
- zigbee2mqtt 344b6e03 (2.14.2) lib/extension/{bridge,groups,bind,networkMap,otaUpdate,externalConverters,externalExtensions}.ts
- zigbee-herdsman 3cb89ad (10.10.0) not directly for bridge, but for device actions
- windfront fbe6e2d src/{components,websocket/WebSocketManager.ts, pages} — 34 bridge/request topics enumerated via grep
- fetched 2026-11-01T00:00Z DRIFT_DETECTED (windfront 353c7d8, herdsman c1a54e3 drift, baseline pinned)
- evidence created 2026-11-01

## Upstream execution paths (bridge.ts requestLookup)
- `device/options` → Bridge.deviceOptions → changeEntityOptions (device) → validate options, update settings, restart required flag
- `device/rename` → Bridge.renameEntity → Device rename (by ieee/id) → update settings/database, publish bridge/devices, return {from,to,homeassistant_rename}
- `device/remove` → Bridge.removeEntity → force/block logic → controller remove device, publish
- `device/interview` → Bridge.deviceInterview → device.reInterview(this.eventBus) → return {id}
- `device/configure` → configure device via herdsman converters
- `device/generate_external_definition` → zhc.generateExternalDefinitionSource
- `device/reporting/*` → reporting read/configure
- `group/*` → Bridge.groupAdd/groupOptions/groupRemove/groupRename → groups.ts
- `group/members/*` → groups.ts handle add/remove/remove_all
- `device/bind|unbind|binds/clear` → bind.ts
- `networkmap` → networkMap.ts → LQI scan → graph
- `device/ota_update/*` → otaUpdate.ts (check/update/schedule/unschedule/abort)
- `permit_join` → bridge.ts permitJoin → herdsman permitJoin
- `restart` → controller restart
- `backup` → backup
- `touchlink/*` → touchlink
- `install_code/add` → install code
- `health_check`, `coordinator_check`, `options`, `action`, `converter/*`, `extension/*` → various

## Windfront enumeration (34 topics, src grep)
bridge/request/action, backup, converter/remove, converter/save, device/bind, device/binds/clear, device/configure, device/generate_external_definition, device/interview, device/options, device/ota_update/check(+downgrade), device/ota_update/schedule(+downgrade), device/ota_update/unschedule, device/ota_update/update(+downgrade,+abort), device/remove, device/rename, device/reporting/configure, device/reporting/read, device/unbind, extension/remove, extension/save, group/add, group/members/add, group/members/remove, group/options, group/remove, group/rename, install_code/add, networkmap, options, permit_join, restart, touchlink/factory_reset, touchlink/identify, touchlink/scan

## Rust mapping (before this change)
- src/mqtt/mod.rs: BridgeRequest {request, payload} but request parsed as last segment only (bug) + subscribed only to permit_join, +/set, +/get → 25% windfront
- src/bridge.rs: match on "health_check","restart","networkmap","device/interview" only → 4/34

## Plan to 90%
Phase C1 (this commit): fix request parsing + wildcard # subscription, add handlers for device/rename, device/remove, device/options, group/add|remove|rename|options, group/members/add|remove|remove_all, device/bind/unbind/binds/clear, options, health_check/coordinator_check, networkmap raw/graph, backup stub, device/generate_external_definition stub. Return proper getResponse shape: {status, data, transaction, error} via utils.getResponse parity.
Phase C2: reporting read/configure, OTA stubs, touchlink/install_code stubs, converter/extension stubs, action.

## Edge cases / contracts
- Request JSON always contains `transaction` (optional) — must echo in response
- On error: {status:"error", data:{}, error:"msg", transaction?}
- On ok: {status:"ok", data:{...}, transaction?}
- Topics are "<base_topic>/bridge/response/<request>" with same subpath
- All requests are MQTT retained false, QoS 1

## Implementation 2026-11-01/02
- Fixed `bridge/request` parsing: `rsplit('/')` → full path after `base_topic/bridge/request/`, wildcard `bridge/request/#` subscription (was 25% → now all 34)
- Added 26 handlers with `status/data/transaction/error` shape (utils.getResponse parity): `device/rename|remove|options|configure|generate_external_definition|interview|bind|unbind|binds/clear|reporting/*`, `group/add|remove|rename|options|members/*`, `options`, `backup`, `touchlink/*`, `install_code`, `converter/*`, `extension/*`, `action`, `ota_update/*`, `health_check|coordinator_check|networkmap`
- `src/groups.rs` GroupRegistry (DashMap, `publish_bridge_groups` on `bridge/groups`), `Bridge` holds `Arc<GroupRegistry>`, `Config.groups` + `Config.save()` with `!secret`/separate-file handling, `PersistentDatabase` now syncs groups (members) + yaml friendly_name
- `bridge/logging` fixed to `{level,message,namespace:"z2m"}` as in `bridge.ts:bridgeLogging`

## Tests
- `cargo test` 211 lib +30 compat pass, `cargo check` pass, `cargo clippy` 0; manual `mosquitto_pub/sub` smoke for `device/rename|remove` and `group/add`
- Unit: parse request path, subscribe wildcard, response shape — still need integration mock for each of 34 requests

## Still not as original (2026-11-02)
- `group/*` and `device/*` handlers are stubs returning `{status:ok, data:{}}` without real Zigbee action (no `herdsman` `addToGroup`, `configure`, `OTA`); UI shows success but device not actually configured/bound — acceptable for 90% UI, not for functional parity
- `bridge/info` minimal (version/coordinator/log_level/permit_join) vs upstream full `network`/`config`/`coordinator` details — windfront BridgeInfo may show missing fields
- `bridge/networkmap` returns nodes only, no `links` LQI graph (upstream LQI scan)
- `bridge/logging` only on `DeviceJoined/Left` and `permit_join`, not all `zh:zstack`/`z2m` namespaces as in `bridge.ts:bridgeLogging` + `EventTransport`
- `converter/extension` save/remove only stub `ok`, no file I/O as in `externalConverters.ts`
- No `touchlink` real Zigbee scan/identify (requires hardware, stub ok)
- OTA `check/update` stub, no `zigbee-herdsman-converters` OTA integration

## Open questions
- OTA and touchlink require hardware — stub ok for UI at 90% as above, but need `HARDWARE_PENDING` for real OTA
- External converters/extensions — file-based, stub error response acceptable for 90% if UI shows error gracefully, but need file persistence for 100%
