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
| ZCL codecs (Buffalo/Foundation/Frame/Utils) | `zspec/zcl/*` | PORTED | `differential_zcl` fixtures for lookup/header/commands; generated catalog covers all 129 upstream clusters, attributes, commands and parameter conditions; `Arc<ClusterInner>` handles + startup warmup keep lookups cheap (router measurements in `docs/zcl-catalog-performance.md`) |
| ZDO codecs | `zspec/zdo/*` | PORTED | full response catalog incl. status-only responses, non-success short-buffer guard, unknown-cluster panic parity |
| `controller/database.ts` | `controller/database.ts` | PORTED | CRUD/order/clear semantics + reload/corruption tests |
| Controller helpers (request/queue/install codes/zclFrameConverter/zclTSN) | `controller/helpers/*` | PORTED | upstream request-queue semantics, full install-code vectors, converter tests |
| `controller/touchlink.ts` | `controller/touchlink.ts` | PORTED | scan/identify/factory-reset frames + lock/restore tests |
| Controller Entity / ZigbeeEntity + ControllerAdapter boundary | `controller/model/entity.ts`, `zigbeeEntity.ts` | PORTED | injection roundtrip + trait-object test |
| Controller models (Device/Endpoint/Group) | `controller/model/{device,endpoint,group}.ts` | PARTIAL | ported: CRUD/registries, ZCL read/write/command/report/config-reporting, interview, `onZclData`, binds, groups, DB round-trip; unit tests in each file. Gaps: OTA (omitted), device/model-specific HA discovery, no Node differential fixture or transcript yet |
| Device OTA methods (`checkOta`/`updateOta`/`abortOta`/`scheduleOta`/`unscheduleOta`/`findMatchingOtaImage`/`#waitForOtaCommand`/`#notifyOta`) | `controller/model/device.ts:1495-1899` | OMITTED | R1 excludes OTA (user decision); `controller/helpers/ota.ts` needs HTTPS + SHA-512 and no HTTP/TLS/hash dependency may be added. Fields `#scheduledOta`/`#otaInProgress`/`#otaAbortController` and the `scheduledOta` DB field are omitted consistently |
| ZCL cluster catalog | `zspec/zcl/definition/cluster.ts` | PORTED | generated `definition/clusters.json` from upstream v11.2.0: all 129 clusters with attributes, commands, responses, defaults, limits and conditions; `differential_zcl::zcl_catalog_covers_all_upstream_clusters` checks every entry by name and ID |
| `controller.test.ts` | `test/controller.test.ts` | PORTED | 311 / 311 upstream `it` cases are mapped 1:1 in `docs/controller-test-parity.md` (suite `tests/controller_suite.rs`, 220 tests, plus controller/green-power unit tests). Request-queue cases use the public queue/test-support accessors; the rxOnCap case assembles the spec-valid commissioning frame bytes because upstream mocks `Frame.fromBuffer` (no GPD 0xe0 writer). |
| `controller/events.ts` | `controller/events.ts` | PORTED | typed payloads + `ControllerEvent` enum |
| `controller/controller.ts` | `controller/controller.ts` | PORTED | options validation, start/stop, adapter event pump, permit-join/timers, backup/database-save, `sendRaw`, install codes, ZDO/ZCL routing, unknown-device identification, Xiaomi re-transmit handling, green power device creation/removal, message/lastSeen events; runtime adapter boundary `ControllerRuntimeAdapter`; per-device event forwarding; `ZclStatusError` -> `defaultResponse`; 22 controller tests ported from controller.test.ts (ZCL report/default-rsp status/raw data, unknown cluster raw data, default-response disabled/broadcast/headerless, Xiaomi re-transmit, unknown-device identification + de-dupe, ZDO network/announce/no-op, sendRaw ZDO/group/broadcast/endpoint/custom cluster/inter-PAN, permitJoin, green power join/decommission/invalid/IEEE/response/channel-request edge cases) |
| `controller/index.ts` | `controller/index.ts` | PORTED | re-exports `Controller` |
| Z-Stack controller runtime wrapper | `adapter/z-stack/adapter/zStackAdapter.ts` (controller surface) | PORTED | `ZStackRuntimeAdapter` implements `ControllerRuntimeAdapter` (model/green power/touchlink/runtime) and forwards broadcast adapter events; event-mapping test. Scripted PTY coordinator runs `ZStackAdapter.start` + `Controller.start/stop` with captured UNPI frames |
| Wire differential (no hardware) | `adapter/z-stack/*` + `controller/model/endpoint.ts` path | PORTED | `tests/wire_capture.rs`: scripted TCP coordinator records bytes to file; `ZStackAdapter` broadcast/group/permitJoin/interPAN/retry/ZDO and `Endpoint` command paths match upstream `ZpiObject.createRequest` goldens. `Controller.start/stop` transcript is byte-for-byte identical to the upstream Node run; ZCL incoming responses match by TSN (`tools/oracle/wire_controller.mjs`, 38 frames, `tests/fixtures/controller_start_wire.hex`); see `docs/wire-differential.md` |
| `adapter/z-stack/adapter.test.ts` | `test/adapter/z-stack/adapter.test.ts` | PORTED | `tests/adapter_zstack_suite.rs`: serial-wire ZNP mock mirroring `ZnpRequestMockBuilder`, upstream NV fixtures (1.2/3.0.x/3.x.0, aligned/unaligned/extended), upstream backup JSON; 130 tests cover commissioning, restore/create/legacy backup failures, diagnostics, LED/power, permit join, ZCL send/retry/route-discovery/timeout, groups/broadcast/inter-PAN, incoming messages + chunked `dataRetrieve`, device events, `waitFor` and all ZDO payload cases. See `docs/wire-differential.md`; per-case mapping in `docs/adapter-test-parity.md` |
| ZCL frame/codec tests | `test/zcl.test.ts`, `test/zspec/zcl/utils.test.ts` | PORTED | `tests/zcl_test.rs`: 147 real tests with exact upstream vectors, covering all 230 upstream cases (191 single `it` + 39 `it.each` rows; per-case mapping in `docs/zcl-test-parity.md`); no tautological/placeholder assertions. `tests/zspec/zcl/utils_test.rs` (54 tests: processAttributeWrite/PreRead/PostRead, parameter write/read, defaults/restrictions for both attribute and parameter variants), `tests/zcl_cluster_roundtrip.rs` (every catalog cluster serializes/parses a command and read frame) |
| `controller/greenPower.ts` | `controller/greenPower.ts` | PORTED | pairing/commissioning option codecs, security-key encryption, notification decryption/re-parse, commissioning/decommissioning/channel-request handling, `permitJoin`, `deviceJoined`/`deviceLeave`; upstream unit cases ported (option vectors, GPP/MIC omission, FULLENCR decrypt, MOES commissioning vector) |
| `controller/helpers/ota.ts` | `controller/helpers/ota.ts` | OMITTED | R1 excludes OTA flashing (user decision); no HTTP/TLS or SHA-512 dependency |
| Green Power | `controller/greenPower.ts` | PORTED | see row above |
| GPD frame codec (`GPD_FRAME` read/write) | `zspec/zcl/buffaloZcl.ts` | PORTED | commissioning/channel/custom writes, commissioning/channel/attribute-report/read-rsp parses with `payloadSize` positioning |
| MQTT bridge + `bridge/request/*` | `zigbee2mqtt/lib/mqtt.ts`, `extension/*` | BLOCKED | not ported |
| Home Assistant discovery | `extension/homeassistant.ts` | BLOCKED | not ported |
| Frontend / Windfront protocol | `extension/frontend.ts` | BLOCKED | not ported |
| Storage (`database.db`, `state.json`, config) | `controller/database.ts`, `lib/state.ts`, `util/settings.ts` | PARTIAL | database + settings validated against router data; `settings.apply`/`reRead`/`write`, the Ajv schema subset and `state.ts` (`lib/state.rs` port with load/save/merge/filter) are PORTED; bridge missing |
| z2m entity model (`Device`/`Group`, no converters) | `lib/model/device.ts`, `lib/model/group.ts` | PARTIAL | definition-independent surface ported (`ID`, `name`, `options`, `ensureInSettings`, endpoint lookup, group members); definition resolution via the shared converter host, exposes are out of R1 |
| Converter host (no database) | `zigbee-herdsman-converters` shared modules | PARTIAL | `access`, `Expose`/`Exposes`, `Definition`, separate crate `crates/zigbee-converters`; only the converter surface required by zigbee2mqtt is ported, not the whole package; registry with `findByDevice`/fingerprint/white-label/external registration and callable `fromZigbee`/`toZigbee` interfaces; empty registry → upstream not-found fallback; `generateDefinition` pending; registry wired into device definition resolution |
| Bridge Zigbee layer | `lib/zigbee.ts` | PARTIAL | start/stop, Z-Stack adapter wiring, permits, pass/block lists, device/group lookups, entity resolution, controller-event forwarding (message/join/interview/leave/last-seen/announce), `resolveDefinition` via the converter host; touchlink is out of R1 |
| Receive extension | `lib/extension/receive.ts` | PARTIAL | definition-less fallback (`Skipping message, still interviewing` + `publishLastSeen`), matching `fromZigbee` converters, `genOta`/`genTime`/`genBasic`/`genPollCtrl` ignore list, per-device debounce (flush on conflict) and throttle, elapsed, state-cache updates; `postProcessConvertedFromZigbeeMessage` calibration pending |
| Publish extension | `lib/extension/publish.ts` | PARTIAL | get/set topic parsing, JSON/state payloads, HA duplicate-state suppression, converter matching (key/endpoints), optimistic state + `filtered_optimistic`, set/get dispatch, scene-change event, `Cannot publish to unsupported device`; group `membersState` meta pending |
| Extension base | `lib/extension/extension.ts` | PORTED | shared `ExtensionContext` (zigbee/mqtt/state/publishEntityState/eventBus/logger/enableDisable/restart/addExtension), async `start`/`stop(self: Arc<Self>)`, listener removal on stop, `adjustMessageBeforePublish` |
| Availability | `lib/extension/availability.ts` | PORTED | enabled/per-device overrides, active/passive timeouts, per-device timeout/jitter/backoff/pause_on_backoff_gt, timers, ping queue with retries and backoff, availability publish (retain qos1) with dedupe, reconnect state retrieval (state/brightness/color), rename/remove/leave/announce/lastSeen/group-members events, irreversible stop |
| Bind | `lib/extension/bind.ts` | PORTED | bind/unbind/binds/clear requests (validation, default_bind_group, endpoint/group/group-id targets), cluster candidate matching, responses and logs; setupReporting with color-capability conditions; disableUnnecessaryReportings; group-members reporting setup; POLL_ON_MESSAGE polling with manufacturer filters and debounce |
| Bridge | `lib/extension/bridge.ts` | PARTIAL | start/stop logging transport, bridge/info|devices|groups|definitions, request dispatch (device+group options/remove/rename/interview, permitting, reporting configure/read, group add, options, restart, backup ZIP, install code, health/coordinator check); touchlink/generate_external_definition/action handlers OMITTED (AGENTS §5 Q5); info.os.node_version is "rust" (no Node runtime) |
| Configure | `lib/extension/configure.ts` | PORTED | startup router pass (10s spacing), deviceJoined/interview/lastSeenChanged/reconfigure/MQTT request triggers, force path, `meta.configured` versioning, 3-attempt limit, in-flight guard, response payloads, exposes/devices event |
| Frontend (protocol/API) | `lib/extension/frontend.ts` | PORTED | WebSocket server (path, auth token, retained/device-state replay, inbound topic dispatch, entity-state/published broadcast, offline on stop, 404 for non-API paths); static assets deferred to OpenWrt packaging; TLS refused (no C crypto provider) |
| Groups | `lib/extension/groups.ts` | PORTED | state-change optimistic fan-out (device↔group, dedupe, off_state/all-members checks, endpoint suffixes), MQTT group/members add/remove/remove_all with validation, endpoint operations, responses, GroupMembersChanged payloads |
| Health | `lib/extension/health.ts` | PORTED | interval timer, `bridge/health` payload (OS load/memory, process uptime/rss, mqtt stats, per-device messages/mps/leave/nwk changes), `reset_on_check`, retain+qos1 |
| Home Assistant | `lib/extension/homeassistant.ts` | PARTIAL | discovery infrastructure (retained configs, dedupe, old-config cleanup handler, availability, device payload, unique/object ids, retained clearing on remove/rename, MQTT status republish, scenes/last_seen/OTA, group member unique ids, exposes/options/scenes/group-member events), exposes numeric/binary/enum/switch/light/lock/cover/fan/climate/text/composite/list, full bridge entity set, device_automation triggers + action republish, experimental event entities, color adjust hook; overrideHaDiscoveryPayload callback, per-object discovery filtering, regex wildcard action matcher; remaining gap: group expose aggregation by type |
| Network map | `lib/extension/networkMap.ts` | PORTED | networkmap request, raw/graphviz/plantuml renderers (colors, escaping, relative last-seen, routes), LQI+routing-table scan with retry and 1s spacing, node/link topology incl. disabled/GreenPower filtering and Xiaomi 0-EUI64 fallback |
| OnEvent | `lib/extension/onEvent.ts` | PORTED | start/stop, deviceJoined/leave/announce/interview/networkAddressChanged/optionsChanged events, `startCalled` gating, disabled devices skipped; global vendor Legrand hook out of the minimal converter surface |
| External JS/converters/extensions | `lib/extension/external*.ts` | OMITTED | user JS evaluation forbidden (AGENTS §5 Q5) |
| OTA update | `lib/extension/otaUpdate.ts` | OMITTED | OTA out of R1 (AGENTS §4/§5) |
| Controller publish pipeline | `lib/controller.ts` | PARTIAL | `publishEntityState` (cache_state, retain/qos/retention tracking, include_device_information, last_seen, linkquality, filtered_attributes, json/attribute outputs, PublishEntityState event) and `iteratePayloadAttributeOutput`; lifecycle/extensions wiring pending; MQTT v5 message-expiry is tracked but not sent (rumqttc built without v5) |
| Backup metadata (`toUnifiedBackup`/`fromUnifiedBackup`/`fromLegacyBackup`) | `utils/backup.ts` | PORTED | unified round-trip against the real router backup; legacy parser unit test |
| HIL (coordinator in container) | — | PORTED | Dongle Plus `10c4:ea60`, SYS version + NV init, see `docs/hil/2026-11-09-dongle-plus.md` |
| Upstream z2m oracle (container) | `zigbee2mqtt.14.3` | PORTED | resumed network, 3 devices, MQTT transcripts, see `docs/hil/2026-11-09-z2m-run.md` |
| HIL (router + Dongle-P) | — | HARDWARE_PENDING | no Routerich/OpenWrt run recorded |

## Controller model layer — status and remaining gaps

Ported in `crates/zigbee-apiarist/src/controller/model/{device,endpoint,group}.rs`
against zigbee-herdsman v11.2.0 (c1a54e3d), with the ZCL/ZDO prerequisites:
`Frame` stores owned `Cluster`/`Command` (custom clusters via
`create_with_definitions`), parameters carry owned conditions, and the catalog
now covers all 129 upstream ZCL clusters generated from v11.2.0.

Ownership model (AGENTS §3.4): upstream static maps translate to
`RwLock<HashMap<...>>` registries with short synchronous access; models resolve
each other through the registry (Endpoint stores the IEEE and calls
`Device::by_ieee_addr`), so no `Arc` cycle and no lock across `.await`.
`pendingRequests` is a `tokio::sync::Mutex<RequestQueue>`; upstream queues a
promise, so the port enqueues and awaits the response without holding the lock.

Known gaps (not parity claims):

- **OTA** is omitted per the user decision (see the matrix row above).
- No Node differential fixture/transcript yet for the model layer; evidence is
  the ported upstream unit cases (device.test.ts) plus frame-level assertions
  through a recording `FakeAdapter`.
- `serde_json::Map` keeps keys sorted, so object-iteration order is normalised
  (array-index keys ascending first, like JavaScript; remaining keys sorted
  instead of insertion order). Only visible in multi-attribute write/read-rsp
  payload ordering.
- Green Power `removeFromNetwork` derives `srcID` from the exact EUI64 integer
  instead of the JavaScript double.

## Controller orchestration — status and remaining gaps

Ported in `crates/zigbee-apiarist/src/controller/controller.rs` against
zigbee-herdsman v11.2.0 (c1a54e3d). The adapter is injected into `start()`
instead of being constructed from transport options; the full adapter surface
is the `ControllerRuntimeAdapter` trait (super-traits `ControllerAdapter`,
`GreenPowerAdapter`, `TouchlinkAdapter`). `ZStackRuntimeAdapter`
(`adapter/z_stack/runtime_adapter.rs`) implements that trait for the existing
Z-Stack adapter and forwards its broadcast events into the controller channel.

Known deviations:

- `AbortSignal` is expressed by dropping the start future. Rust futures are
  cancellable by drop, so no separate abort handle is needed; the upstream
  `throwIfAborted()` checkpoints are covered by not polling the future.
- The `zdoResponse` event carries `clusterId`/`response` through the adapter
  event channel instead of an `EventEmitter`; the controller dispatch behaviour
  and ordering are identical (single consumer loop).
- `Frame::try_from_buffer` converts codec panics into `FrameParseError`
  (`Status` for `ZclStatusError` cases, `Other` otherwise). `INVALID_FIELD`
  (0x85) and unknown command/global-command lookups (0x81) map to the upstream
  status, so `defaultResponse` is populated for those cases. With the full
  129-cluster catalog, unknown numeric cluster ids now map to
  `UNSUPPORTED_CLUSTER` (0xc3). Other parse failures remain non-status errors
  without a default response, matching upstream (only `ZclStatusError` sets
  `defaultResponse`).

Coordinator-only HIL smoke with `ZStackRuntimeAdapter` is **not yet recorded**.
Per AGENTS §1/§6 the test router currently runs the Node.js Zigbee2MQTT, so a
hardware run needs an explicit redeploy and user coordination; no destructive
operation or recommissioning was attempted.

Remaining ledger item: **`controller/helpers/ota.ts` needs HTTPS and SHA-512**.
The crate has no HTTP/TLS or hashing dependency and `docs/dependency-map.md`
declares none; that is why OTA stays out of R1 rather than being stubbed.

## Next evidence to add

1. `Frame`, `Buffalo`, `Zcl.Utils`, `Zdo` differential fixtures against
   `zigbee-herdsman@11.2.0`.
2. ZNP transcript replay (chunks → writes/received/errors).
3. MQTT bridge transcript once the controller slice exists.
4. HIL record for start/resume, permit join, reboot recovery.
