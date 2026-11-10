# R1 parity matrix (initial)

Status is evidence-based. `PORTED` requires a ported upstream test **and** a
differential fixture or transcript. This file is a starting point, not a parity
claim.

Legend: `PORTED` / `PARTIAL` / `OMITTED` / `BLOCKED` / `HARDWARE_PENDING`.

| Surface | Upstream | Status | Evidence / gap |
|---|---|---|---|
| UNPI frame encode/decode + FCS | `adapter/z-stack/unpi/frame.ts` | PORTED | unit tests + `differential_frame` fixtures |
| Adapter transport (serial/TCP) | `adapter/transport.ts` | PORTED | serial HIL on Dongle Plus, PTY unit tests and TCP loopback transcript (`differential_transport`) |
| ZNP framing + request/waitress | `adapter/z-stack/znp/znp.ts` | PORTED | PTY transcript: SREQ/SRSP status, AREQ by transid, ZDO AREQ by eui64, received stream, timeouts (`differential_znp`) |
| ZNP bootloader skip | `znp.ts:117` | PARTIAL | ping + CC2530/CC2531 magic-byte path covered by PTY transcript; CC2652 DTR/RTS path needs serial hardware |
| `zStackAdapter` | `adapter/z-stack/adapter/zStackAdapter.ts` | PORTED | start/stop/queue/ZDO/events/ZCL endpoint+group+broadcast+chunked incomingMsgExt/interPAN/network-params/backup/waitFor/endDeviceAnnce debounce; endpoint 239 Shelly profile fixed; PTY chunk test + HIL |
| Adapter manager | `adapter/z-stack/adapter/manager.ts` | PORTED | startup HIL `Resumed`; PTY tests cover `startup`/`restoreBackup`/`startCommissioning` strategy selection; destructive HIL validated once (2026-11-10) — **re-run forbidden** (AGENTS §3.11) |
| Adapter NV memory | `adapter-nv-memory.ts` | PORTED | wire transcript + HIL `init` |
| Adapter backup | `adapter-backup.ts` | PORTED | `createBackup` matched router backup; `restoreBackup` validated once (2026-11-10) — **re-run forbidden** (AGENTS §3.11) |
| Z-Stack structs | `adapter/z-stack/structs/*` | PORTED | buffer-backed Struct/Table + `differential_structs` fixtures |
| Z-Stack constants | `adapter/z-stack/constants/*` | PORTED | generated from upstream + `differential_constants`/`generated_constants` |
| ZNP MT command catalog | `znp/definition.ts` | PORTED | generated 241 commands + `differential_definition` |
| Buffalo primitives | `buffalo/buffalo.ts` | PORTED | unit tests + `differential_buffalo` fixtures |
| BuffaloZnp | `znp/buffaloZnp.ts` | PORTED | `differential_buffalo_znp` fixtures |
| ZpiObject | `znp/zpiObject.ts` | PORTED | `differential_zpi_object` fixtures |
| ZCL codecs (Buffalo/Foundation/Frame/Utils) | `zspec/zcl/*` | PARTIAL | `differential_zcl` fixtures for lookup/header; cluster catalog subset |
| ZDO codecs | `zspec/zdo/*` | PORTED | full response catalog incl. status-only responses, non-success short-buffer guard, unknown-cluster panic parity |
| `controller/database.ts` | `controller/database.ts` | PORTED | CRUD/order/clear semantics + reload/corruption tests |
| Controller helpers (request/queue/install codes/zclFrameConverter/zclTSN) | `controller/helpers/*` | PORTED | upstream request-queue semantics, full install-code vectors, converter tests |
| `controller/touchlink.ts` | `controller/touchlink.ts` | PORTED | scan/identify/factory-reset frames + lock/restore tests |
| Controller Entity / ZigbeeEntity + ControllerAdapter boundary | `controller/model/entity.ts`, `zigbeeEntity.ts` | PORTED | injection roundtrip + trait-object test |
| Controller models (Device/Endpoint/Group) | `controller/model/{device,endpoint,group}.ts` | BLOCKED | not ported; prerequisites in "Blocked: controller model layer" below |
| `controller/controller.ts` | `controller/controller.ts` | BLOCKED | not ported yet |
| `controller/greenPower.ts` | `controller/greenPower.ts` | BLOCKED | not ported yet |
| `controller/helpers/ota.ts` | `controller/helpers/ota.ts` | BLOCKED | not ported yet |
| Green Power | `controller/greenPower.ts` | BLOCKED | not ported |
| MQTT bridge + `bridge/request/*` | `zigbee2mqtt/lib/mqtt.ts`, `extension/*` | BLOCKED | not ported |
| Home Assistant discovery | `extension/homeassistant.ts` | BLOCKED | not ported |
| Frontend / Windfront protocol | `extension/frontend.ts` | BLOCKED | not ported |
| Storage (`database.db`, `state.json`, config) | `controller/database.ts`, `lib/state.ts`, `util/settings.ts` | PARTIAL | database + settings validated against router data; state/bridge missing |
| Backup metadata (`toUnifiedBackup`/`fromUnifiedBackup`/`fromLegacyBackup`) | `utils/backup.ts` | PORTED | unified round-trip against the real router backup; legacy parser unit test |
| HIL (coordinator in container) | — | PORTED | Dongle Plus `10c4:ea60`, SYS version + NV init, see `docs/hil/2026-11-09-dongle-plus.md` |
| Upstream z2m oracle (container) | `zigbee2mqtt.14.3` | PORTED | resumed network, 3 devices, MQTT transcripts, see `docs/hil/2026-11-09-z2m-run.md` |
| HIL (router + Dongle-P) | — | HARDWARE_PENDING | no Routerich/OpenWrt run recorded |

## Controller model layer — remaining blocker

The ZCL prerequisites are done: `Frame` stores owned `Cluster`/`Command`
(custom clusters supported via `create_with_definitions`), parameters carry
owned conditions, and the catalog now covers genGroups/genTime/genPollCtrl/
ssIasZone/genOta/touchlink plus full ZCL/ZDO statuses and the Foundation table.

Ownership model chosen (AGENTS §3.4): upstream static maps translate to
`RwLock<HashMap<...>>` registries with short synchronous access; models resolve
each other through the registry (Endpoint stores the IEEE and calls
`Device::by_ieee_addr`), so no `Arc` cycle and no lock across `.await`.

Remaining blocker: **`controller/helpers/ota.ts` needs HTTPS and SHA-512**.
The crate has no HTTP/TLS or hashing dependency and `docs/dependency-map.md`
declares none. `Device`'s OTA methods (checkOta/updateOta/abortOta/scheduleOta/
unscheduleOta and matching) cannot be ported completely without deciding how to
fetch `https://raw.githubusercontent.com/Koenkk/zigbee-OTA/...` and verify
SHA-512. Local-file OTA indexes/images are already portable without new crates.

## Next evidence to add

1. `Frame`, `Buffalo`, `Zcl.Utils`, `Zdo` differential fixtures against
   `zigbee-herdsman@11.2.0`.
2. ZNP transcript replay (chunks → writes/received/errors).
3. MQTT bridge transcript once the controller slice exists.
4. HIL record for start/resume, permit join, reboot recovery.
