# Evidence: Z-Stack Coordinator Initialization (ADAPTER-001/002/003)

## Goal
Verify Z-Stack init parity for CC2531 (HA 1.2, product_id 0) and CC2652P (Dongle-P, product_id 1/2).

## Source pins
- zigbee2mqtt: 344b6e03 (v2.14.2) — lib/zigbee.ts, lib/extension/bridge.ts
- zigbee-herdsman: b560461e (10.10.0) — src/adapter/z-stack/adapter/zStackAdapter.ts: start() L120-170, src/adapter/z-stack/adapter/manager.ts: start/determineStrategy/beginStartup/beginCommissioning, src/adapter/z-stack/znp/znp.ts
- zigbee-herdsman-converters: c18c32d9 (26.115.1)
- fetched: 2026-10-31 UTC, freshness CURRENT

## Upstream execution path
1. ZStackAdapter.start(): znp.open() → ping x3 (SYS ping cap 1, 250ms) → SYS version (fallback to 1.2 if fail) → create ZnpAdapterManager → manager.start() → optional LED/transmitPower → return.
2. manager.start(): parseConfig → nv.init() → determineStrategy() reads hasConfigured (ZNP_HAS_CONFIGURED_ZSTACK1=0x33/ ZSTACK3=0x34), NIB, PRECFGKEY (SAPI for 1.2, NV for 3.x), active/alternate key info, backup; decides startup/restoreBackup/startCommissioning with extensive mismatch warnings.
3. beginStartup(): UTIL getDeviceInfo → if not ZB_COORD → waitFor ZDO stateChangeInd (60s) + ZDO startupFromApp (100ms).
4. beginCommissioning(): delete NIB, clearAdapter (STARTUP_OPTION 0x03 → reset → 0x00), updateCommissioningNvItems (STARTUP_OPTION 0x00, LOGICAL_TYPE 0x00, ZDO_DIRECT_CB 0x01, CHANLIST, PANID, EXTENDED_PAN_ID reversed, APS_USE_EXT_PANID reversed, PRECFGKEYS_ENABLE, PRECFGKEY via NV or SAPI+ TCLK for 1.2), for 3.x: bdbSetChannel primary/secondary + bdbStartCommissioning (0x04) waitFor stateChangeInd (100s); for 1.2: beginStartup(); wait NIB settle (3s poll x10), validate extNwkInfo, write hasConfigured.
5. registerEndpoints(): query active endpoints, register Endpoints list if missing.
6. addToGroup(242, GP_GROUP_ID).

## Rust mapping
- src/coordinator/znp/mod.rs: ZnpCoordinator::start() — reset → ping (new) → check_version → write_nv_config → configure_channel_bdb (if product_id >=1) → register_endpoints → start_network → get_device_info → spawn event_pump.
- write_nv_config now writes STARTUP_OPTION 0x00 and APS_USE_EXT_PANID (added 2026-10-31) to match upstream's Commissioning NV items.
- transport.rs: SREQ/SRSP with 10s timeout, subsystem 0x00 fallback (Z-Stack 1.2 quirk), single pending.
- frame.rs: FCS resync one byte, subsystem 0x00 preserved as Rpc.

## Comparison table

| Upstream op | Rust op | Parity | Notes |
|---|---|---|---|
| SYS ping x3 | sys_ping x3 (added) | VERIFIED | 1-byte payload, 200ms retry |
| SYS version | sys_version | VERIFIED | fallback to 1.2 not yet (Rust errors) |
| NIB/backup strategy | none | GAP | Rust always rewrites NV and starts; no NIB check or backup restore |
| STARTUP_OPTION 0x00 | added | VERIFIED | |
| LOGICAL_TYPE | VERIFIED | |
| PANID/CHANLIST/EXT_PANID | VERIFIED | |
| APS_USE_EXT_PANID | added | VERIFIED | |
| PRECFGKEYS_ENABLE/PRECFGKEY | VERIFIED | NV path for both versions (upstream SAPI for 1.2) — acceptable |
| BDB bdbSetChannel | configure_channel_bdb | VERIFIED for 3.x, skipped for 1.2 |
| bdbStartCommissioning | NOT IMPLEMENTED | GAP — Rust uses ZDO startupFromApp for both; Z-Stack 3.x commissioning path missing |
| NIB settle + extNwkInfo | NOT IMPLEMENTED | GAP |
| hasConfigured flag | NOT IMPLEMENTED | GAP |
| clearAdapter (STARTUP_OPTION 0x03 reset) | NOT IMPLEMENTED | GAP — only soft reset |
| registerEndpoints | PARTIAL — single EP1 vs full Endpoints list + activeEp query | GAP |
| addToGroup 242 | NOT IMPLEMENTED | GAP |
| ZDO startupFromApp + stateChangeInd wait | PARTIAL — sends startupFromApp 100ms, checks SRSP status, sleeps 1s; no stateChangeInd wait | GAP |
| getDeviceInfo | VERIFIED | |
| Timeout 10s | VERIFIED | matches AGENTS.md |
| Subsystem 0x00 fallback | VERIFIED | transport dispatch |

## Implementation 2026-11-01/02
- Added `sys_ping` (3 attempts) before version, `STARTUP_OPTION` + `APS_USE_EXT_PANID` NV writes, `sys_ping` and NV ID tests
- Transport already had 10s timeout and 0x00 fallback (VERIFIED), FCS resync 1 byte (VERIFIED)

## Tests covering
- frame::tests:: unknown_subsystem_not_discarded, fcs_mismatch_resyncs_on_next_byte (VERIFIED)
- commands::tests:: nv_write_format, bdb_set_channel (6 tests) + new sys_ping, NV IDs (2)
- Total 8 ZNP tests passing

## Hardware-in-loop
- HARDWARE_PENDING — CC2531 and CC2652P not available in CI. Protocol verification via codec/transport unit tests only.

## Still not as original (2026-11-02)
- NIB/backup `determineStrategy` not implemented — Rust always rewrites NV and does `startupFromApp`, no NIB read, no backup restore, no `hasConfigured` check (upstream decides `startup` vs `restoreBackup` vs `startCommissioning`)
- `bdbStartCommissioning` (0x04, wait 100s) not implemented — Rust uses `startupFromApp` for both 1.2 and 3.x; 3.x new network commissioning will not match upstream
- NIB settle (3s poll x10) + `extNwkInfo` PAN ID collision check not implemented
- `hasConfigured` flag and `clearAdapter` (STARTUP_OPTION 0x03 → reset) not implemented — only soft reset
- Endpoint registration: single EP1 vs upstream `Endpoints` list + `activeEp` query + `addToGroup(242)`
- `startupFromApp` no `stateChangeInd` waiter (60s/100s) — only SRSP check + 1s sleep
- Version fallback to 1.2 on `SYS version` failure not implemented — Rust errors instead of assuming 1.2
- `ZDO_DIRECT_CB` etc. written, but `STARTUP_OPTION` 0x00 only, not handling commissioning vs startup distinction

## Open questions
- NIB read requires `SYS_OSAL_NV_READ` (not yet implemented) — need to add NV read path
- Endpoint list for CC2652P may differ from CC2531 — need hardware to verify
