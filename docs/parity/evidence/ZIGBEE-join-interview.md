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

## Rust mapping
- src/bridge.rs: DeviceJoined (EndDeviceAnnceInd), AddressResolved (IeeeAddrRsp/TcDevInd), ActiveEpRsp → requestSimpleDesc, SimpleDescRsp → push EndpointDesc → interview_complete=true (on first EP) → requestBind for output clusters → ZCL read Basic 0x0004/0x0005/0x0007/0x4000
- spawn_interview_retry: 3 attempts x 20s polling interview_complete
- device_allowed() blocklist/passlist (added d1179ff) — matches upstream settings
- interview_complete stored per Device, persisted via database (not yet NV backup)

## Gaps
- No node descriptor (ZDO 0x02) — upstream requires it before endpoints
- No power source / modelID quirks — Tuya/Control4 etc. not handled
- Interview marked complete on first endpoint, not all — 2-ep thermostat incomplete
- No ZDO simple desc for all endpoints tracking (expected count)
- No retry for Basic read / no IAS enroll
- No persistence of interviewState (only bool)

## Tests
- Existing: spawn_interview_retry not unit-tested; event pump mocked not available
- Plan: add unit test for Device interview_complete transition, blocklist filter, endpoint suffix logic

## Hardware
- HARDWARE_PENDING — join/interview requires real device; simulated ZNP frames only test parsing

## Next
- Implement node descriptor ZDO (0x02) and store manufacturerID/type
- Track expected endpoint count, mark complete after all SimpleDescRsp
- Add quirks for Tuya modelID TS* (minimal)
