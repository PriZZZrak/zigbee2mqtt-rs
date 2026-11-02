# Evidence: Device Join / Interview (ZIGBEE-001)

## Goal
Parity for device join (EndDeviceAnnceInd), address resolution, interview (ActiveEp + SimpleDesc + Basic), retry, blocklist/passlist.

## Source pins
- zigbee2mqtt 344b6e03 — lib/zigbee.ts, lib/controller/controller.ts
- zigbee-herdsman b560461e — src/controller/model/device.ts: interview() L861ff, interviewInternal() L989ff, src/controller/controller.ts: handleDeviceAnnounce L890ff, src/adapter/z-stack/...
- fetched 2026-10-31T00:04Z CURRENT

## Upstream path
1. Controller handleDeviceAnnounce → DeviceInterview pending → device.interview()
2. interview(): state InProgress → interviewInternal(ignoreCache) → on error check interviewQuirks() → Successful else Failed
3. interviewInternal(): loop 6x updateNodeDescriptor (ZDO node descriptor req) → hasNodeDescriptor else fail → updateActiveEndpoints (ZDO activeEp req) → for each EP updateSimpleDescriptor → for each EP reads genBasic (modelId, manufName), power, etc. → quirks → binding/reporting → save
4. Quirks: Tuya, Control4, Xiaomi lumi, etc. — on fail still Successful if modelID matches
5. Join filtering: blocklist/passlist in settings (lib/util/settings.ts) — filtered before interview

## Rust mapping (2026-11-02)
- `src/bridge.rs: DeviceJoined` (EndDeviceAnnceInd), `AddressResolved` (IeeeAddrRsp/TcDevInd), `NodeDescRsp` (0x82) → store `manufacturer_id` + `powerSource` mapping, then `ActiveEpRsp` → store `interview_expected_eps`, `SimpleDescRsp` → push `EndpointDesc` → `interview_complete` only when all expected EPs received → `requestBind` for output clusters → ZCL read Basic 0x0004/0x0005/0x0007/0x4000
- `spawn_interview_retry`: 3 attempts x 20s, now `request_node_desc` first, fallback to `ActiveEp` (quirks)
- `device_allowed()` blocklist/passlist (d1179ff) + `Device.database_id` + `PersistentDatabase` wired (c79faa8/71e8f7d) — persisted via `sync_devices` + 500ms debounce

## Implementation 2026-11-02
- Added ZDO `NODE_DESC_REQ` 0x02 + `NodeDescRsp` (0x82) with `manufacturer_code/logical_type`, `ZnpEvent::NodeDescRsp`, `CoordinatorEvent::NodeDescRsp`, `ZnpHandle::request_node_desc`, `Device.manufacturer_id` + `interview_expected_eps`
- Fixed multi-EP interview: `ActiveEpRsp` stores expected list, `SimpleDescRsp` checks `all(expected EPs present)` before marking complete (was first EP)
- Added `Device.database_id` and `PersistentDatabase` debounced write (500ms) + final flush

## Tests
- `commands::tests::node_desc_req_format`, `parse_node_desc_rsp` (2 new)
- Existing: `spawn_interview_retry` not unit-tested; event pump mocked not available — plan unit test for Device interview_complete transition

## Hardware
- HARDWARE_PENDING — join/interview requires real device; simulated ZNP frames only test parsing

## Still not as original (2026-11-02)
- No `PowerDescriptor` read, no `SimpleDescriptor` for all EPs with full `clusters/binds` preservation beyond `inClusterList/outClusterList`
- No `genBasic` reads for `modelID/manufacturerName` retries (6 attempts) as in `interviewInternal` — we read Basic only once per EP
- No `interviewQuirks` for Tuya `TS*`/`_TZ*`, Control4 `0xabcd`, Xiaomi `lumi*` etc. — upstream would mark Successful despite failure, we mark Failed
- No `IAS enroll` or `binding/reporting` setup after interview (upstream does via `configure` extension)
- `interviewState` only `SUCCESSFUL/FAILED` (bool), not `Pending/InProgress` + `checkinInterval/gpSecurityKey` updates
- No `Device.save()` debounce granular per-endpoint, only per-device batching via `persist_database`
