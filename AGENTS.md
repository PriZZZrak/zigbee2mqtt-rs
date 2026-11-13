# AGENTS.md — zigbee2mqtt-rs (clean restart)

> Instructions for every coding agent. Read this file before touching code.
> If a question in §5 is still open, stop and ask the user before implementing
> that area.

## 1. Current state

- Clean restart after the first Rust implementation was archived on
  2026-11-06.
- Archived work:
  - branch `archive/boa-js-host-20261106` (commit `502601f`)
  - tag `archive-boa-js-host-20261106`
  - `https://github.com/PriZZZrak/zigbee2mqtt-rs/tree/archive/boa-js-host-20261106`
- `main` is the new implementation line (decision 2026-11-06).
- The test router currently runs the **Node.js Zigbee2MQTT**, not a Rust
  binary. Hardware-in-the-loop work requires an explicit redeploy and user
  coordination.
- The archive contains the previous code, parity inventory, HIL evidence and
  the original 467-line AGENTS file. Use it as history/evidence, but **do not
  copy its converter-host architecture**.

## 2. Mission and R1 decision

Build a fully working **core** Zigbee-to-MQTT bridge in pure Rust for the R1
hardware. R1 deliberately excludes the device-specific converter layer.

Resolved decisions (2026-11-06):

- **R1 scope:** ZNP + Zigbee stack + MQTT + Home Assistant + Windfront +
  storage/operations, **without** device-specific converters.
- **No `zigbee-herdsman-converters`** and **no JavaScript engine (Boa or
  other)** in R1.
- **Pure Rust, no C dependencies** remains mandatory.
- Development happens directly on `main`.

R1 hardware: Routerich AX3000 v1, OpenWrt 24.10.8 (aarch64), SONOFF Dongle-P
(TI CC2652P / Z-Stack).

R1 is a limited core milestone, not a claim of complete Zigbee2MQTT or device
catalog parity.

## 3. Lessons from v1 — non-negotiable rules

1. **Measure on the target before choosing architecture.** The router has two
   cores; x86 timings are irrelevant.
2. **No JavaScript/converter interpreter in the bridge.** R1 has no Boa at all.
3. **No per-device/per-session module evaluation.** Not applicable in R1, and
   forbidden if a JS layer is ever reconsidered.
4. **No process-wide lock shared with the event loop.** Short-lived
   synchronous access to static model registries (`Device`/`Group`, database)
   is allowed because upstream uses static maps; never hold such a lock across
   an `.await`.
5. **Bound every wait.** Mutexes, DB writes, MQTT publishes and coordinator
   requests need timeouts; on timeout, log and defer/fallback.
6. **Publish early, work later.** Responses go out before expensive background
   work; HA/device-list work runs in detached tasks.
7. **Keep one bridge event loop.** State/command ordering stays
   single-threaded; heavy work goes to workers and returns via channels.
8. **A stub that hangs is worse than an explicit unsupported path.**
9. **No parity claim without evidence.** Upstream source, tests and hardware
   evidence are required.
10. **No stubs, models or placeholders — ever.** It is STRICTLY FORBIDDEN to
    introduce stub functions, placeholder implementations, `todo!()`,
    `unimplemented!()` or comments such as `// logic here` in production code.
    Every ported function must preserve the observable behavior and functional
    semantics of the corresponding upstream TypeScript implementation.
    Behavioral equivalence includes, where applicable:

    - identical protocol messages, payloads, serialization formats and binary
      wire representations;
    - equivalent state transitions, side effects and persistence behavior;
    - correct event ordering, command-response relationships and asynchronous
      behavior;
    - equivalent error handling, edge cases, retries and timeout semantics;
    - compatibility with existing MQTT, Zigbee, Home Assistant and frontend
      contracts.

    Byte-for-byte equivalence is mandatory for externally observable binary
    data and serialized outputs where the upstream protocol or compatibility
    contract requires it, not for internal implementation details.
    Rust implementations may use different internal structures, algorithms,
    memory management and asynchronous mechanisms, provided behavioral
    equivalence is demonstrated through upstream tests, golden fixtures,
    differential testing or protocol transcripts.
    Do not simplify, omit or silently change upstream behavior merely to make
    the Rust implementation easier.
    If a function is too large to port safely in one pass, split the work into
    smaller, independently testable units while preserving the complete
    behavior. Do not introduce temporary stubs or incomplete production paths.
    If exact compatibility conflicts with project architecture or technical
    constraints, document the conflict and ask the user before proceeding.
11. **Re-commissioning a live network is FORBIDDEN.** Never run
    `StartCommissioning` or the restore-backup path (`beginRestore`) against a
    real coordinator that belongs to a live installation. Both delete the NIB
    and form a new random provisioning network; devices lose the network key
    and need physical re-pairing. Such paths may only be exercised against a
    PTY/fake ZNP, or on a coordinator the user explicitly dedicated to
    destructive testing. HIL evidence for these paths was captured once on
    2026-11-10 and must not be re-run.

## 4. R1 scope

“100% working” means the following surfaces are complete for the core, with no
placeholder responses, unbounded waits or deadlocks.

Q1 is resolved for the port: the controller model layer (Device/Endpoint/Group,
standard-ZCL state/commands) is ported as part of `controller/`. This is a
code-parity decision; R1 runtime still excludes `zigbee-herdsman-converters`
and vendor-specific quirks. Network identity, registry, interview, MQTT, HA
bridge entities, Windfront and storage are in R1.

- **ZNP / coordinator**
  - serial discovery/selection, ZNP framing, SREQ/SRSP matching, timeouts;
  - coordinator start/resume without NV rewrite, network parameters, channel;
  - permit join; soft reset/reconnect; firmware-capability detection.
- **Zigbee stack**
  - ZDO: device announce, IEEE/NWK resolution, node/active-endpoint/simple
    descriptors, interview, bind/unbind.
  - ZCL: read/write/configure-reporting, report/read responses, default
    response handling, groups, standard cluster encoders/decoders.
  - Network identity/descriptors/interview/bind are in R1. Generic
    standard-cluster state/commands are part of the controller model port; no
    model-specific quirks.
- **MQTT**
  - `bridge/state`, `info`, `devices`, `converters`, `groups`, `health`,
    `logging`, `definitions` where applicable;
  - `bridge/request/*`: `options`, `permit_join`, `device/rename`,
    `device/remove` (leave + block), `device/interview`, `health_check`,
    `networkmap`, `restart`;
  - device `set`/`get`, transactions, retained/QoS semantics, reconnect.
- **State and storage**
  - `database.db` NDJSON import/round-trip, `configuration.yaml` import/save,
    `state.json` cache, optimistic state, availability, last seen;
  - restart, `kill -9`/respawn and reboot recovery; no data loss.
- **Home Assistant**
  - bridge entities, device-registry discovery, availability, retained cleanup
    on rename/remove. Per-device cluster discovery is **pending** (not part of
    the initial R1 surface).
- **Frontend**
  - HTTP/WebSocket protocol required by unmodified Windfront;
  - bridge responses, device/state replay and command dispatch on the
    WebSocket bus; assets/static serving.
- **Operations**
  - CLI, config logging, OpenWrt service, aarch64 musl build, rollback-safe
    persistence, import/migration tooling.

### Out of R1

- `zigbee-herdsman-converters`, Boa/JS, vendor-specific exposes and quirks.
- Full device catalog and per-model configuration.
- Adapters other than Z-Stack (ember, deconz, zigate, zboss, zoh, ezsp).
- CC2531 hardware acceptance (R1 uses Dongle-P; CC2531 remains required for
  complete Stage 1 later).
- OTA flashing, scenes, full Windfront feature parity, live HA verification.

## 5. Decisions and remaining questions

Resolved on 2026-11-06:

- **Q2 — hardware acceptance:** **coordinator-only HIL is sufficient** for R1
  (Dongle-P start/resume, network, permit join, MQTT/HA/frontend/storage). A
  real device is not required for R1 acceptance.
- **Q3 — “100%” checklist:** the surface list in §4 is accepted as the R1
  definition of “100% working”.
- **Q1 — device layer boundary:** resolved for the port. The controller model
  layer (`device.ts`, `endpoint.ts`, `group.ts`) and generic standard-ZCL
  state/commands are ported as part of `controller/`. This does not add
  `zigbee-herdsman-converters` or a device catalog to R1; per-device HA
  discovery remains outside the initial R1 surface.
- **Q4 — converter boundary (2026-11-13):** the R1 exclusion is the converter
  *database* (`definitions/`, `devices/`, `converters/`, vendor quirks), not the
  shared converter host. Common files and loading/lookup mechanisms may and
  should be ported: `access`, `exposes`, `utils`, `modernExtend` pieces as
  needed, `findByDevice`/`findDefinition`/fingerprint matching, external
  definition registration, `prepareDefinition`, and the `fromZigbee`/
  `toZigbee` converter interfaces. With no database registered the lookup
  falls back to “not found”: unsupported devices stay definition-less and
  publish/receive follow their upstream fallback branches (e.g.
  `receive.ts:130-134`). `generateDefinition` (the “unknown device” synthesizer)
  is a shared mechanism, not the database, and may be ported later; until then
  the fallback is an explicit not-found, never a stub.

## 6. Technical constraints

- Pure Rust application; no C/C++/assembly crypto or native libraries.
  `rumqttc` keeps `default-features = false`; verify with `cargo tree -e
  features` and the lockfile.
- Target `aarch64-unknown-linux-musl` (static); Routerich AX3000 / OpenWrt
  24.10.8. Native Linux builds must also work.
- `src/lib.rs` + thin `src/main.rs`; bridge logic in the library.
- Dependency policy: `docs/dependency-map.md` is normative. Every upstream
  package has one Rust replacement and one wrapper module; third-party crates
  may only be imported inside their wrapper. Do not re-implement the same helper
  in two modules.
- Single bridge event loop; static cluster handlers; ZNP transport actor with
  request matching and timeouts.
- No `zigbee-herdsman-converters`, Boa or other JS runtime.
- Local parity toolchain (do not re-download): Node v24.22.2 and global `tsx`
  live under `/config/.local/node-v24.22.2-linux-x64`, exposed via
  `/config/.local/bin/{node,npm,npx,tsx}`. See `tools/oracle/README.md`.
- Do not commit user-owned files (`scripts/mqtt_log.sh`, `.opencode/bugs/`,
  `.opencode/issue/`, etc.).

## 7. Upstream and evidence policy

- The oracle for the core is `zigbee-herdsman` (ZNP/ZDO/ZCL/network/controller
  behaviour), `zigbee2mqtt` (MQTT/HA/frontend/config/storage) and the pinned
  Windfront revision.
- `zigbee-herdsman-converters` is **reference only** for understanding what the missing device layer would do; it is not executed or copied into R1.
- Read the relevant upstream code, callers, tests and dependency versions before
  implementing. Record the source revision.
- Every ported file must start with the upstream path and pinned revision, and
  each ported function/type must carry an upstream name plus a `file.ts:line`
  anchor so the next upstream bump can be diffed mechanically.
- The normative porting and parity procedure is `docs/porting-parity.md`.
  Read it before porting any function; a module without differential/transcript
  evidence is not `PORTED`.
- Preserve observable MQTT/HA/storage contracts; do not “clean up” quirks
  without evidence.
- Prefer golden fixtures and differential tests against the pinned Node
  implementation where feasible. Hardware rows remain `HARDWARE_PENDING` until
  real evidence exists.
- The archived parity inventory is historical; define a new R1 checklist rather
  than reusing global catalog percentages.
- Make link with lines on upstream name of function, variable or entity, in comment section

## 8. Workflow

1. Read this file and the user’s current instructions.
2. Inspect the archived evidence before designing new behaviour:
   ```bash
   git show archive/boa-js-host-20261106:AGENTS.md
   git show archive/boa-js-host-20261106:docs/parity/README.md
   ```
3. Answer §5 questions with the user before coding affected areas.
4. Write a short design/measurement note; measure on hardware when performance
   is part of the decision.
5. Implement in small slices. For every ported function add the upstream unit
   test, a differential fixture or transcript, and a parity-matrix row as
   required by `docs/porting-parity.md`.
6. Run `cargo fmt`, `cargo test`, `cargo clippy` and the parity/differential
   checks. Never claim hardware results without a real run.
7. Commit with a clear message; do not rewrite published history or force-push
   `main` without explicit approval.
8. Report: upstream read, changes, tests, unknowns, remaining decisions.

## 9. Definition of done for R1

On the actual router + Dongle-P:

- coordinator starts/resumes and network operations work;
- MQTT and Windfront stay responsive during startup, interview and persistence;
- bridge API requests return correct transactions and payloads, not stubs;
- config/database/state survive restart, kill/respawn and reboot;
- HA bridge entities and device-registry discovery/cleanup work (per-device
  cluster discovery is added after the initial controller model port);
- permit join, rename, remove/leave/block and options work end-to-end (fixtures
  are acceptable; coordinator-only HIL is the R1 acceptance);
- no unbounded wait or event-loop deadlock remains;
- exact hardware, binary hash, commands and evidence are recorded.

R1 is a core milestone. It does not establish device-catalog, complete Stage 1
or global Zigbee2MQTT parity.
