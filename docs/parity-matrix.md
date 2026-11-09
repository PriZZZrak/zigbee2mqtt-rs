# R1 parity matrix (initial)

Status is evidence-based. `PORTED` requires a ported upstream test **and** a
differential fixture or transcript. This file is a starting point, not a parity
claim.

Legend: `PORTED` / `PARTIAL` / `OMITTED` / `BLOCKED` / `HARDWARE_PENDING`.

| Surface | Upstream | Status | Evidence / gap |
|---|---|---|---|
| UNPI frame encode/decode + FCS | `adapter/z-stack/unpi/frame.ts` | PORTED | unit tests + `differential_frame` fixtures |
| Adapter transport (serial/TCP) | `adapter/transport.ts` | PARTIAL | PTY unit tests; TCP/HIL and transcript replay missing |
| ZNP framing + request/waitress | `adapter/z-stack/znp/znp.ts` | PARTIAL | PTY request tests; no full upstream znp transcript |
| ZNP bootloader skip | `znp.ts:117` | PARTIAL | code path only; HIL pending |
| `zStackAdapter` | `adapter/z-stack/adapter/zStackAdapter.ts` | PARTIAL | start/ZDO/events/ZCL/interPAN/network-params/backup HIL; endDeviceAnnce debounce pending |
| `zStackAdapter` (ZCL/queue/start) | `zStackAdapter.ts` | BLOCKED | not ported yet |
| Adapter manager | `adapter/z-stack/adapter/manager.ts` | PARTIAL | full port; startup path HIL `Resumed`, restore/commissioning paths pending |
| Adapter NV memory | `adapter-nv-memory.ts` | PORTED | wire transcript + HIL `init` |
| Adapter backup | `adapter-backup.ts` | PARTIAL | `createBackup` HIL matched router backup; `restoreBackup` not HIL yet |
| Z-Stack structs | `adapter/z-stack/structs/*` | PORTED | buffer-backed Struct/Table + `differential_structs` fixtures |
| Z-Stack constants | `adapter/z-stack/constants/*` | PORTED | generated from upstream + `differential_constants`/`generated_constants` |
| ZNP MT command catalog | `znp/definition.ts` | PORTED | generated 241 commands + `differential_definition` |
| Buffalo primitives | `buffalo/buffalo.ts` | PORTED | unit tests + `differential_buffalo` fixtures |
| BuffaloZnp | `znp/buffaloZnp.ts` | PORTED | `differential_buffalo_znp` fixtures |
| ZpiObject | `znp/zpiObject.ts` | PORTED | `differential_zpi_object` fixtures |
| ZCL codecs (Buffalo/Foundation/Frame/Utils) | `zspec/zcl/*` | PARTIAL | `differential_zcl` fixtures for lookup/header; cluster catalog subset |
| ZDO codecs | `zspec/zdo/*` | PARTIAL | many unit ports; response catalog subset |
| Controller / Device / Endpoint / Group | `controller/*` | BLOCKED | not ported |
| Green Power | `controller/greenPower.ts` | BLOCKED | not ported |
| MQTT bridge + `bridge/request/*` | `zigbee2mqtt/lib/mqtt.ts`, `extension/*` | BLOCKED | not ported |
| Home Assistant discovery | `extension/homeassistant.ts` | BLOCKED | not ported |
| Frontend / Windfront protocol | `extension/frontend.ts` | BLOCKED | not ported |
| Storage (`database.db`, `state.json`, config) | `controller/database.ts`, `lib/state.ts`, `util/settings.ts` | PARTIAL | database + settings validated against router data; state/bridge missing |
| Backup metadata (`toUnifiedBackup`) | `utils/backup.ts` | PARTIAL | round-trip validated on real router backup; legacy path partial |
| HIL (coordinator in container) | — | PORTED | Dongle Plus `10c4:ea60`, SYS version + NV init, see `docs/hil/2026-11-09-dongle-plus.md` |
| Upstream z2m oracle (container) | `zigbee2mqtt.14.3` | PORTED | resumed network, 3 devices, MQTT transcripts, see `docs/hil/2026-11-09-z2m-run.md` |
| HIL (router + Dongle-P) | — | HARDWARE_PENDING | no Routerich/OpenWrt run recorded |

## Next evidence to add

1. `Frame`, `Buffalo`, `Zcl.Utils`, `Zdo` differential fixtures against
   `zigbee-herdsman@11.2.0`.
2. ZNP transcript replay (chunks → writes/received/errors).
3. MQTT bridge transcript once the controller slice exists.
4. HIL record for start/resume, permit join, reboot recovery.
