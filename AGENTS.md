# AGENTS.md — zigbee2mqtt-rs: Rust architecture + upstream parity rules

> **Priority: mandatory instructions for every coding agent and every task.**
> This repository is a Rust reimplementation of the original Node.js/TypeScript Zigbee2MQTT ecosystem. The objective is **observable feature parity without regressions**, not a simplified Zigbee-to-MQTT demonstration. Preserve the existing Rust project architecture and legacy hardware support. Adapter support is intentionally staged: Z-Stack first, all upstream adapters later, with separate progress metrics. The original project's accumulated behavior, edge cases, compatibility guarantees, and tests are the specification.
>
## 0. Non-negotiable rules

1. **READ UPSTREAM BEFORE WRITING CODE.** Before any implementation, modification, refactor, optimization, dependency choice, or bug fix, locate and read the relevant *current* original implementation, its callers/callees, tests, types, documentation, and external dependencies. Reading README alone does not count.
2. **NEVER GUESS UPSTREAM BEHAVIOR.** If a behavior cannot be verified from source, tests, docs, or a controlled reference execution, mark it `UNKNOWN` and investigate. Do not silently invent semantics.
3. **NEVER CLAIM PARITY WITHOUT EVIDENCE.** Code compiling, unit tests passing, or matching an API signature is not sufficient. Matching observable behavior, including failures and edge cases, is required.
4. **KEEP UPSTREAM FRESH.** Fetch upstream before starting a task, again before accepting a parity claim, and check the tracked source SHA before publishing results. If fetching is impossible, record the failure and mark freshness `UNVERIFIED`; do not claim full/current compatibility.
5. **NO LOST FUNCTIONALITY.** All upstream adapters belong to the **long-term global parity target**, but implementation is staged. **Stage 1 is Z-Stack**, including mandatory CC2531 Z-Stack HA 1.2 and Texas Instruments CC2652P (SONOFF Zigbee 3.0 USB Dongle Plus / Dongle-P). Other upstream adapter families are future-stage work, **not permanently excluded**. Preserve all already-working Rust behavior and legacy corner cases.
6. **TRACK EVERYTHING.** Every newly discovered behavior or upstream change must have an item in the parity inventory. Every implemented feature must be linked to evidence and tests. Update the tracker in the same change as the implementation.
7. **PRESERVE USER-VISIBLE CONTRACTS.** MQTT topic names/payloads, discovery, config defaults and migrations, event ordering, error behavior, storage, CLI, and API semantics must remain compatible unless explicitly approved and documented.
8. **BE HONEST ABOUT PROGRESS.** An untested or stubbed capability contributes **0** to verified-completion percentage. Never mark a whole subsystem complete because its happy path works.
9. **DO NOT MODIFY THE REFERENCE COPY.** Treat upstream checkouts as read-only. Changes belong in the Rust repository. Never run destructive git operations on a user's uncommitted work.
10. **STOP IF BLOCKED.** If original sources/dependencies are missing, exact dependency versions are unknown, or compatibility evidence cannot be obtained, report the blocker and implement only clearly identified, independently verifiable work; never substitute assumptions.

## 1. Mission and definition of compatibility

The goal is a maintainable, **pure Rust** implementation targeting embedded ARM (notably aarch64 Raspberry Pi 3) that can replace the upstream Zigbee2MQTT deployment without changing the expectations of MQTT consumers, device owners, Home Assistant integrations, coordinator hardware, and operational tooling. Stage 1 must fully support Z-Stack coordinators (especially CC2531 and CC2652P SONOFF Dongle-P); the final target extends to every adapter family the pinned upstream supports. Use upstream behavior as the contract; do **not** automatically preserve implementation details that have no observable effect.

- Behavioral parity: identical or explicitly justified equivalent outputs for the same input, state, and configuration.
- Protocol parity: Zigbee adapter frames, command sequences, retries, timeouts, acknowledgements, message parsing, and supported adapter families.
- Integration parity: MQTT topics, JSON shapes/types, retained/QoS semantics, availability, bridge requests/responses/events, frontend APIs and Home Assistant discovery.
- Operational parity: startup/restart, shutdown, persistence, recovery, backup/restore, migrations, logging, configuration validation, errors, security, packaging, and upgrades.
- Catalog parity: the same supported device definitions, exposes, converter quirks, vendor-specific commands, OTA behavior, and overrides as the **selected upstream source snapshot**.

**Important:** Upstream is a moving target. Report parity against a precise pinned source snapshot and freshness against latest upstream separately. Never write “100% of Zigbee2MQTT” without specifying source SHAs and coverage scope.

## 2. Reference repositories and source-of-truth policy

Canonical upstreams (verify their URLs and structure on each refresh):

| Repository | Role |
| --- | --- |
| `https://github.com/Koenkk/zigbee2mqtt.git` | Application logic, extensions, MQTT, config, API, tests, startup and deployment |
| `https://github.com/Koenkk/zigbee-herdsman.git` | Zigbee stack, adapters, coordinator interaction, transport, ZCL/ZDO and network state |
| `https://github.com/Koenkk/zigbee-herdsman-converters.git` | Device definitions, fromZigbee/toZigbee converters, exposes, reporting, OTA integration |
| `https://github.com/Koenkk/zigbee2mqtt.io.git` | Public behavior/config/device documentation; supplement, not substitute for source |
| Upstream frontend package/repository, as referenced by the pinned Zigbee2MQTT package | UI/API contracts (discover the actual revision from upstream dependencies) |

The original app is TypeScript running on Node.js. Typical application code appears under `lib/`, with tests under `test/`; inspect the **actual current tree** rather than assuming paths remain stable. Inspect `package.json`, `pnpm-lock.yaml`, any `AGENTS.md`, build config, changelog, tests, and transitive package boundaries. Treat documentation and release notes as clues; pinned implementation and executable reference behavior resolve ambiguity. If code/tests/docs disagree, document the conflict and investigate rather than selecting the easiest answer.

### Recommended local layout (adapt if the repository already defines another)

```text
<rust-repo>/
  AGENTS.md
  src/ ...
  tests/ ...
  docs/parity/
    README.md
    inventory.csv
    upstream-lock.json
    upstream-changes.md
    decisions.md
    evidence/                   # Golden fixtures, traces, mappings
    reports/                    # Generated summaries
  .upstream/                    # Ignored, read-only working copies; no edits
    zigbee2mqtt/
    zigbee-herdsman/
    zigbee-herdsman-converters/
    zigbee2mqtt.io/
```

If these files do not exist, **bootstrap them as the first task**, before implementing functionality. Paths are recommendations, not claims about the current Rust repository.

## 2A. Existing Rust project contract — preserve unless the user explicitly approves a change

These are **established project decisions**, not optional recommendations. When a new parity requirement conflicts with one, **stop and ask the user** before changing it, weakening parity claims or introducing workaround behavior. Upstream describes *what the program does*; these constraints specify *how this Rust implementation must be built*. Neither silently overrides the other.

### Target, dependencies, build

- **Pure Rust, no C dependencies:** never add crates that require bundled C/C++/assembly crypto or native libraries. In particular `rumqttc` **MUST** specify `default-features = false`; inspect `cargo tree -e features` and actual lockfile/target dependency tree before claiming no native code. Do not infer that a feature flag alone proves it. Rust system linker use for aarch64 builds is not permission to link native application dependencies.
- Primary embedded target: **aarch64 Raspberry Pi 3**. Keep resource use, startup time and memory compatible with embedded constraints; preserve general native builds.
- Expected builds (adapt to the actual toolchain):

```bash
cargo build --release
cargo build --release --target aarch64-unknown-linux-gnu
cargo test
```

- ARM cross-compilation may require `gcc-aarch64-linux-gnu` and the pre-existing `.cargo/config.toml` linker configuration. Do not replace the toolchain or introduce C-based crates without user approval.

### Architecture — do not silently refactor

- **Single-threaded bridge event loop:** one Tokio task performs `select!` across coordinator events and MQTT commands. `DashMap` is already used for ergonomic interior mutability, **not** as a reason to introduce multithreaded business logic. Keep ordering, lifetimes and reentrancy semantics intact.
- **Library + thin binary:** `src/lib.rs` exports modules, `src/main.rs` handles CLI parsing, and `src/bridge.rs` lives in the library to support testing. Do not move bridge logic into the CLI or scatter it across independent event loops.
- **Static cluster handlers:** `handler_for()` returns `&'static dyn ClusterHandler`; avoid allocating a boxed handler for each message.
- **ZNP transport actor:** serial I/O runs in a spawned task; SREQ/SRSP request matching uses oneshot channels and a **10-second timeout**. Preserve the established compatibility behavior that a mismatched-subsystem SRSP can be delivered to the pending request instead of being discarded, especially for old firmware; add targeted regression tests.

### Required Z-Stack hardware in Stage 1

**Hardware A — CC2531 / Z-Stack HA 1.2:** `product_id = 0` means some `APP_CNF`/BDB commands are unavailable. Skip BDB channel configuration when `product_id < 1`; do not assume modern Z-Stack commands exist. SRSP frames with unknown subsystem values, including **`0x00`** error responses, must reach the request handling logic rather than being dropped (prevent false 10-second timeouts).

**Hardware B — Texas Instruments CC2652P / SONOFF Zigbee 3.0 USB Dongle Plus (Dongle-P):** first-class, mandatory Stage 1 coordinator. Identify the actual ZNP/Z-Stack firmware capabilities at runtime from protocol responses; **do not apply CC2531's `product_id=0` workarounds indiscriminately**, and do not assume all CC2652P firmware builds share commands, product IDs or behavior. Test network startup/restart, permit-join, interview, command/response, reporting, coordinator reconnect and database persistence on **both** hardware types. An emulator/fixture can validate protocol behavior but cannot replace actual hardware evidence for the hardware-verified metric. Do not confuse **Dongle-P (TI CC2652P / Z-Stack)** with **Dongle-E (Silicon Labs / Ember)**.

### Existing externally visible compatibility contracts

Preserve these known project expectations and verify each against the **pinned current upstream source** for relevant modes/versions; do not generalize beyond observed applicability:

| Surface | Existing contract / risk |
| --- | --- |
| Bridge state | JSON `{"state":"online"}`, **not** plain text `online` |
| IEEE address | Lowercase hex `0xec1bbdfffeaa66db` everywhere (config, topics, registry and discovery) |
| Home Assistant entity `unique_id` | Existing template `{ieee}_{type}_{base_topic}` (normally ends with `_zigbee2mqtt` when using that base topic); audit exact upstream entity-type variations before changing |
| Color | Nested object `{"color":{"x":...,"y":...}}`, not flat color keys |
| Optimistic set | Preserve immediately published expected state for operations currently using optimistic behavior; verify exact original conditions and exception paths before changing |
| Switch discovery | `payload_on` / `payload_off` are string values `"ON"` / `"OFF"`, not JSON objects |
| `database.db` | zigbee-herdsman NDJSON; **camelCase** keys such as `ieeeAddr`, `nwkAddr`, `manufName`, `epList`, and endpoint `inClusterList` / `outClusterList` |

Critical lifecycle rules: `IeeeAddr::as_hex()` is lowercase; devices pre-seeded from config/database use real persisted NWK addresses; new joins get NWK from `EndDeviceAnnceInd`; unknown NWK addresses initiate `ZDO_IEEE_ADDR_REQ`. Preserve these behaviors when changing bridge logic, import, onboarding or state.

### Existing file ownership and regression surfaces

| Path | Responsibility |
| --- | --- |
| `src/bridge.rs` | Bridge/event loop, Zigbee-to-MQTT wiring, command handling |
| `src/homeassistant.rs` | HA discovery and exact payload formats |
| `src/coordinator/znp/mod.rs` | Coordinator init sequences; firmware-dependent behavior |
| `src/coordinator/znp/transport.rs` | Serial transport and SREQ/SRSP pairing |
| `src/database.rs` | `database.db` NDJSON/camelCase import |
| `tests/z2m_compat.rs` | Zigbee2MQTT compatibility regressions; rerun after state/MQTT/HA changes |

These are known **historical** paths; verify the present tree before relying on them. The prior notes claimed **137 tests across 4 targets** but their stated counts (**107 library + 9 binary + 30 compatibility**) sum to **146**, not 137. **Do not present either as the current total**. Discover actual test targets and run `cargo test -- --list` / `cargo test`; record current counts and failures, and migrate tests only with behavioral equivalence. The test count discrepancy is not a reason to delete tests.

## 3. Mandatory upstream refresh — before EVERY coding task

**Do not write Rust code until this gate passes or its failure is explicitly recorded.**

1. Locate the reference checkouts; clone missing repositories into `.upstream/` or use preconfigured reference paths. Do not overwrite an existing checkout.
2. Run `git remote -v`, `git status --porcelain`, `git fetch --prune origin`, and record `git rev-parse HEAD` and `git rev-parse origin/HEAD` (resolve the remote default branch correctly). Use `git ls-remote --symref origin HEAD` where needed. Never assume `master`/`main` without checking.
3. For the app, inspect actual dependency versions and resolved package hashes from the **pinned** `package.json` and lockfile. Check out the matching versions/tags/commits of herdsman, converters, and frontend for **reference behavior**, rather than indiscriminately mixing latest versions. If lockfile versions do not map cleanly to git SHAs, pin package version/integrity and record the mapping uncertainty.
4. Separately fetch the latest default branch of each project to determine *upstream drift* and discover new features. Record both the compatibility baseline and latest upstream head. **Do not silently move the baseline on every fetch.**
5. Determine which paths/releases/commits have changed since the previously reviewed baseline using `git log --oneline OLD..NEW`, `git diff --stat OLD..NEW`, and targeted diffs. Look for changed behavior, dependency updates, breaking changes, new devices, and regression fixes.
6. Update `docs/parity/upstream-lock.json` and `docs/parity/upstream-changes.md`; create new inventory rows for newly discovered behavior. Timestamp all checks in UTC ISO-8601.
7. If no network access: use the last verified local snapshot, record `freshness_status: UNVERIFIED`, the last successful fetch timestamp, and the affected gap; ask for network access when needed. **No fabricated HEAD or dates.**

Read-only reference examples (replace paths/branches as actually discovered):

```bash
# Run inside a reference checkout; does not change files or branches.
git remote -v
git status --porcelain
git fetch --prune origin
git ls-remote --symref origin HEAD
git rev-parse HEAD
git log -1 --format='%H %cI %s'
git log --oneline --max-count=30 HEAD..origin/HEAD
# For known reviewed revisions:
git diff --stat <previous-reviewed-sha>..<new-sha>
git diff <previous-reviewed-sha>..<new-sha> -- <relevant-path>
```

`HEAD..origin/HEAD` requires that `origin/HEAD` exists; resolve the named default branch otherwise. Fetching is safe, but do not `reset --hard`, `clean -fd`, or auto-checkout in a dirty checkout. Do not use `git pull` as a substitute for a controlled review. If tags/branches are missing or the fetch fails, record it.

**Refresh cadence:** at the start of every agent session/task and before marking the task complete; CI should independently run a daily (or more frequent configured) upstream drift check and open a report/issue when there are new commits. CI should not auto-merge upstream behavior or change the parity baseline without review.

## 4. Mandatory code archaeology before implementation

For **each** feature/bug fix, produce a short research record in `docs/parity/evidence/<feature-id>.md` **before** implementation with:

1. Goal and exact externally observable contract; associated inventory ID(s).
2. Source pins: app SHA, `zigbee-herdsman` version/SHA, converters version/SHA, relevant frontend revision, plus fetch/check timestamp.
3. Precise original locations: `path:function/type:line-range` **at the pinned revision**; immutable commit URLs where practical.
4. Full execution path: entry point → config → controller/extension/event → herdsman → converters → persistence → MQTT/API output, including relevant callbacks, state, caches, timers, and teardown.
5. The original tests covering the feature and gaps they leave; inspect fixtures and mocks, not just test names.
6. Default values, optional/invalid inputs, exceptional paths, retries, concurrency, ordering, timing, persistence/restart behavior, security and performance-sensitive cases.
7. Rust mapping: modules/types/crates, boundary decisions, precise behavior to preserve, and any unavoidable deviation (with approval).
8. Golden fixtures or reference execution plan, with exact commands/environment; where hardware is required, mark `HARDWARE_REQUIRED` and define the test setup.
9. Open questions and known divergences; do not pretend they are resolved.

**Search is not reading.** `rg`/symbol search is for locating behavior; inspect full implementations, consumers, and tests. Where behavior lives in a dependency, recursively inspect that dependency at the resolved version. Read the project’s own `AGENTS.md` and contributing instructions, but these do not override this parity policy.

### Required investigation order

1. Identify the user-facing operation and test fixture.
2. Trace it downward through **all affected source modules** and dependencies.
3. Trace inputs/outputs and error cases upward to API/MQTT/UI.
4. Examine version history/blame/changelog for compatibility-driven quirks and bug fixes; do not “clean up” quirks without equivalent evidence.
5. Record the contract and tests in the inventory/research note.
6. **Only then** plan and write Rust code.

## 5. Functionality inventory: avoid silent scope loss

Maintain **one row per independently testable behavior**, not one row per file or broad subsystem. Start with these domains and add every item actually discovered in upstream:

- Runtime: startup, shutdown, CLI, process signals, reconnect and crash recovery, logging, health and diagnostics.
- Configuration: schema, validation, defaults, environment overrides, deprecations, migrations, generated configuration and security settings.
- MQTT: connections, reconnect, auth/TLS, base topic, state publish, subscribe, commands, bridge topics, requests/responses, retained/QoS, errors, birth/will/availability.
- Zigbee network: coordinator startup, permit join, interview, reinterview, pairing, leave, reset, network join and restoration, address changes and route failures.
- Adapter/transport: serial/TCP/socket selection, discovery, timeouts, vendor adapter protocols and supported coordinator families; firmware/version constraints.
- Zigbee stack: ZCL/ZDO, endpoint/group/bind/reporting, read/write/configure, manufacturer-specific behavior, default responses, transaction sequence and retries.
- Devices and converters: identify devices, exposes, fromZigbee/toZigbee conversion, reporting and calibration, options, external converters/device definitions, vendor quirks, per-model edge cases.
- Groups and scenes: membership, commands, binding and persistent state.
- Availability/state: online/offline detection, last seen, debounce, cache, optimistic state, debounce/repeat, transition semantics.
- OTA: image sources, availability checks, checksums, download, update progress, error/retry/cancel semantics and supported device types.
- Integrations: Home Assistant discovery, device/entity identifiers and lifecycle, other documented integrations' MQTT contracts.
- Frontend / API: HTTP/WebSocket/event behavior, auth, configuration management, endpoint payloads, streaming and error codes.
- Storage and migration: database format, backups, restore, legacy data/config loading, crash consistency and ownership of persisted state.
- Operations: Docker/start scripts, paths, volumes, permissions, service mode, upgrade/downgrade behavior, metrics and tracing.
- Security and robustness: malformed packets, MQTT injection, credentials, untrusted device data, races, resource exhaustion, disconnections.

**Device coverage must not be buried in a single “converters complete” row.** Track catalog totals plus per-definition/per-converter testability and new/changed device deltas. Support separate totals for devices, converters, adapters, and externally observable behaviors; do not equate these populations.

## 6. Parity tracker: schema, formulas and reporting

Create and maintain these source-controlled artifacts:

- `docs/parity/inventory.csv`: authoritative, diff-friendly feature inventory. Every unique `id` is an independently verifiable behavior.
- `docs/parity/upstream-lock.json`: precise source baseline, dependency provenance, last fetch and latest heads.
- `docs/parity/upstream-changes.md`: append-only (or chronologically maintained) review of new upstream commits/behavior and disposition.
- `docs/parity/evidence/<id>.md`: source trace, test scenarios and evidence for each feature.
- `docs/parity/README.md`: generated dashboard with **stage-1 and global** parity percentages, exclusions, timestamps, freshness and blockers; **never manually invent percentages**.
- `docs/parity/decisions.md`: explicitly approved deviations and scope exclusions, with justification.

### CSV columns (required)

```csv
id,domain,behavior,baseline_sha,upstream_refs,dependency_refs,stage,adapter_family,priority,weight,status,verified_cases,total_cases,test_refs,evidence_ref,owner,upstream_changed,notes
```

- `id`: stable (e.g. `MQTT-001`), no duplicates.
- `stage`: `1` for Z-Stack scope and shared/core behaviors necessary for Stage 1; `2+` for future adapter-specific behavior. This is a **planning label**, never a reason to delete global inventory rows. For globally shared behavior, use `1`.
- `adapter_family`: `shared`, `zstack`, or an actual upstream adapter family identifier discovered from source (do not invent names). If one behavior has family-specific differences, split it into separate independently testable rows.
- `upstream_refs`: original file/symbol/commit pointers; cannot be empty for a verified row.
- `dependency_refs`: linked converters/herdsman references when relevant.
- `priority`: `P0` critical / `P1` important / `P2` other; **not** a completeness override.
- `weight`: positive integer complexity/impact weight, default `1`, agreed before work; do not increase/decrease weights just to improve the percentage. Report raw unweighted coverage too.
- `status`: `DISCOVERED`, `RESEARCHED`, `IN_PROGRESS`, `IMPLEMENTED_UNVERIFIED`, `PARTIAL`, `VERIFIED`, `BLOCKED`, `REGRESSION`, or `OUT_OF_SCOPE_APPROVED`.
- `verified_cases`/`total_cases`: count of specifically enumerated, meaningful scenarios with passing parity evidence; `0 <= verified_cases <= total_cases`, `total_cases > 0` once researched. Tests may cover multiple edge cases only if enumerated; duplicate assertions do not inflate counts.
- `test_refs` and `evidence_ref`: Rust/reference/differential tests and attached fixture/log paths.
- `upstream_changed`: `yes` if the behavior has changed upstream since the pinned baseline and awaits review.

### Two independent parity KPIs (mandatory)

The user-approved roadmap is **staged, with TWO primary completion metrics**. Stage 1 covers Z-Stack and every shared/core behavior required for a drop-in Z-Stack deployment, explicitly including **CC2531 (Z-Stack HA 1.2)** and **TI CC2652P / SONOFF Dongle-P**. Later stages add other upstream adapter families. Do not falsely label a future adapter as out of scope merely because its implementation is deferred.

Define `global_rows` as all inventoried upstream behaviors except individually `OUT_OF_SCOPE_APPROVED` with explicit user approval; `stage1_rows` as `global_rows` where `stage == 1`. The baseline must include the full original ecosystem, even while Stage 1 is the active milestone. All shared Zigbee2MQTT functionality required for Z-Stack belongs in Stage 1, not just coordinator transport.

```text
stage1_verified_% = 100 * count(stage1_rows where status == VERIFIED) / count(stage1_rows)
global_verified_% = 100 * count(global_rows where status == VERIFIED) / count(global_rows)

stage1_weighted_% = 100 * sum(weight of VERIFIED stage1_rows) / sum(weight of stage1_rows)
global_weighted_% = 100 * sum(weight of VERIFIED global_rows) / sum(weight of global_rows)

stage1_scenarios_% = 100 * sum(verified_cases in enumerated stage1_rows)
                         / sum(total_cases in enumerated stage1_rows)
global_scenarios_% = 100 * sum(verified_cases in enumerated global_rows)
                         / sum(total_cases in enumerated global_rows)
```

Use `N/A` for zero denominators, never 100%. Report numerator and denominator for both KPIs and explicit inventory-audit completeness separately. Since the inventory can be incomplete, **neither metric implies complete upstream parity until an upstream-wide audit is finished**. Newly discovered upstream behavior increases denominators and may reduce completion. Report unmeasured scenarios and unreviewed upstream changes independently. Weights are positive stable estimates, never adjusted to boost percentage. Count only whole `VERIFIED` rows; unfinished, mocked-only, blocked, partial, regressed or implemented-without-evidence rows add zero to verified feature totals. `OUT_OF_SCOPE_APPROVED` requires explicit user approval and is reported separately; **future stage is not exclusion**.

For device catalogs and converters, produce separate inventory completeness and compatibility stats (count of device definitions, converter behaviors and real verified cases); avoid a single token row that claims full device coverage. Report adapter family status per hardware variant. Differentiate `compiled`, `simulated`, and `hardware-verified`. Track **Stage 1 and global** verified features, weighted verified features and scenario coverage, plus upstream drift.

### Status transitions and completion gate

`DISCOVERED → RESEARCHED → IN_PROGRESS → IMPLEMENTED_UNVERIFIED/PARTIAL → VERIFIED`.

Mark `VERIFIED` **only if all are true**:

- Upstream source and all relevant dependency code were read and pinned.
- Complete expected behavior and edge cases were recorded; no relevant unknowns remain.
- A Rust implementation exists without silently skipped paths or placeholders.
- Unit, integration and/or differential parity tests pass; protocol or hardware functionality also has hardware-in-loop evidence when required.
- Original equivalent scenarios are covered, including negative cases, restart/persistence, ordering and migration where relevant.
- Relevant CI gates pass and latest upstream drift for this behavior has been reviewed.
- Test references/evidence and inventory are updated in the same change.

For hardware-dependent rows, mocks alone cannot prove hardware parity. Record `HARDWARE_PENDING` in notes and use `IMPLEMENTED_UNVERIFIED`/`PARTIAL` until evidence is available. For a feature not supported for legitimate, expressly documented reasons, use `OUT_OF_SCOPE_APPROVED`; report its number and impact separately, and never call an excluded feature “ported”.

### Required dashboard contents

```text
Compatibility baseline: zigbee2mqtt <sha/tag> + resolved herdsman/converters versions
Latest upstream:        <head shas> (checked <UTC timestamp>)
Freshness:              CURRENT | DRIFT_DETECTED | UNVERIFIED
Discovered:             <N> eligible behaviors; <M> unmeasured rows
Stage 1 Z-Stack parity:<V1>/<N1> = <x.xx>% [CC2531 + CC2652P mandatory]
Global upstream parity:<Vall>/<Nall> = <x.xx>% [all adapter families]
Stage 1 / global weighted parity: <x.xx>% / <x.xx>%
Stage 1 / global scenarios: <S1>/<T1> and <Sall>/<Tall> [unmeasured rows]
Status breakdown:       VERIFIED / PARTIAL / BLOCKED / REGRESSION / etc.
Critical P0 gaps:       <IDs>
Adapter coverage:       each upstream family + CC2531 and CC2652P hardware evidence
Inventory audit:        <audited domains>/<upstream domains>, unknown gaps
Device definition parity:<verified>/<baseline catalog definitions>
Converter parity:       <verified>/<baseline converter behaviors>
Open upstream deltas:   <count + links>
Last test run:          <command, timestamp, result, environment>
Known exclusions:       <IDs, approval references>
```

If a generator script does not exist, add one (e.g. `scripts/parity_report.py` or Rust equivalent) that **validates** CSV schema, IDs, statuses, case counts, source pins and evidence links; computes metrics; generates the dashboard; and exits nonzero for malformed/stale or unsupported claims. No spreadsheet-style manually typed percentages. Put a regression check in CI; any unreviewed upstream change should report drift and optionally fail a dedicated freshness check, without conflating drift with a Rust test failure.

## 7. Test strategy: the original behavior is the oracle

Prefer a test pyramid with explicit parity comparisons:

1. **Reference fixture collection:** run the original pinned app (and pinned dependencies) in a reproducible controlled setup. Record exact inputs/config, MQTT traffic, adapter/Zigbee frames where safe, persisted files, timing tolerances and outputs.
2. **Differential tests:** feed equivalent stimuli to Node and Rust and compare normalized observable behavior. Use an **explicit whitelist** of nondeterministic fields (timestamps, random IDs, ordering only if original permits it); do not normalize away substantive mismatches.
3. **Golden tests:** version fixtures and verify byte-level or schema/semantic parity as the contract requires, including retained flags/QoS and payload field types.
4. **Property/fuzz tests:** malformed ZCL/ZDO/MQTT frames, corrupted database/config, boundary values, retries, state machines and resilience.
5. **Integration tests:** local MQTT broker, fake Zigbee coordinator/transport, reconnection, adapter variations, event ordering and persistence.
6. **Hardware-in-loop tests:** supported coordinator families and representative devices; verify join/interview/reporting/commands/OTA where safe. Never perform destructive actions on real hardware without explicit authorization and a controlled environment.
7. **Upgrade/migration tests:** load an upstream-produced config/database/backup, run Rust, confirm no data loss; test rollback/restart behavior.
8. **Non-regression tests:** any original bug fix or Rust bug report becomes a fixture/test in the parity suite.

Never use a mock that simply encodes the new Rust implementation's assumptions as proof of original compatibility. Preserve original quirks (including error outcomes) until a conscious, documented decision permits a deviation. Test published MQTT messages and user-visible payloads, not only internal Rust functions.

## 8. Rust implementation engineering rules

- Favor a layered design: transports/adapters → Zigbee protocol and network state → device definitions/converters → domain/application/controller → MQTT/API/frontend interfaces → persistence/operations. Adjust based on investigated actual upstream contracts.
- Separate pure converters and serialization from I/O for differential and golden testing.
- Use explicit typed state machines and errors; preserve error categories, externally visible output, and retry semantics. Avoid blanket `unwrap()`/`expect()` in runtime input paths.
- Carefully reason about asynchronous ordering: Node event-loop callbacks/promises are not automatically equivalent to Rust `async`/Tokio tasks. Track causal order, locks, cancellation, backpressure, timeouts, race conditions and shutdown.
- Validate behavior before optimizing. Do not replace mature original workarounds with “cleaner” logic without tests.
- Treat network packets, external converters, MQTT payloads, configuration and persistent data as untrusted input; apply bounds checks and secure credential handling.
- Choose crates after examining upstream requirements (protocol and behavior), not based only on crate popularity. Record dependency risks and licenses.
- Avoid inventing support for adapters or devices that have not been verified; clearly distinguish `compiled`, `simulated`, `tested with hardware`.
- Respect upstream licensing and attribution requirements, especially if copying/adapting code, fixtures, definitions or generated catalogs; review GPL-3.0 implications for redistributed derivatives with the project's legal owner.

## 9. Upstream drift and version management

There are **three distinct concepts**:

1. **Baseline**: approved Zigbee2MQTT commit/tag and the **actually resolved** versions of herdsman, converters and frontend used to define the current parity target.
2. **Latest upstream**: current default-branch heads or new releases checked on the network, which may differ from the baseline.
3. **Rust support**: verified inventory coverage against the baseline, plus a queue of uncovered upstream changes.

Do not confuse them. Upstream changes may require new inventory rows, changed expected fixtures, or reopening already verified rows. Every upstream delta gets an entry in `upstream-changes.md` with repository, old/new SHA, links, affected IDs, impact (`none`/`compatible`/`breaking`/`unknown`), review state, tests and disposition. If dependencies changed in upstream `package.json`/lockfile, re-evaluate their behavior as part of that delta.

### Baseline advancement policy

1. Fetch and record latest branches/releases.
2. Inventory and triage all new or modified behaviors and security fixes.
3. Explicitly choose the next baseline (approval required when project policy requires it).
4. Update pins together, keeping package dependency compatibility; do not mix arbitrary heads.
5. Re-run reference capture, differential and Rust regressions; reopen invalidated `VERIFIED` rows.
6. Regenerate metrics and publish a migration/delta summary.

**Freshness statuses:** `CURRENT` = last network check succeeded in the configured window and there are no unreviewed deltas relevant to the baseline; `DRIFT_DETECTED` = newer changes found/pending review; `UNVERIFIED` = network check missing/failed/older than the configured window. Default freshness window: **24 hours for automated CI, and mandatory fetch at each coding task**. A repository may be up to date locally while upstream parity still has pending deltas; reflect both in the report.

## 10. Mandatory task workflow and review gates

### Phase A — Discover (no code changes)

- [ ] Read this file and current repository-specific instructions.
- [ ] Inspect Rust project state, existing tests and inventory; protect uncommitted changes.
- [ ] Fetch/verify all relevant upstreams; record SHA, time, dependency versions, freshness.
- [ ] Check upstream changes since last review and create inventory/delta entries.
- [ ] Find and read original implementations **and** test cases down to actual dependencies.
- [ ] Write source trace and edge-case checklist in `docs/parity/evidence/<id>.md`.
- [ ] Set affected inventory rows to `RESEARCHED` and identify baselines/unknowns.

### Phase B — Plan (still no feature code)

- [ ] Specify observable acceptance criteria and strict parity test plan.
- [ ] Identify migrations, concurrency and hardware risks.
- [ ] Decide Rust boundaries and necessary crates; explain departures from original design.
- [ ] List fixtures and tests to add before or alongside implementation.

### Phase C — Implement

- [ ] Make small, reviewable Rust changes, preserving existing working behavior.
- [ ] Add unit/differential/golden/integration tests for positive and negative cases.
- [ ] Update errors, logs, config/migrations and documentation as behavior requires.
- [ ] Do not silently reduce the supported devices, adapters or existing surface.

### Phase D — Validate

- [ ] Run relevant Rust format, lint, build, tests, and sanitizer/fuzz checks where configured.
- [ ] Run corresponding original/reference tests or fixtures where feasible.
- [ ] Run differential tests and hardware checks if required for verification.
- [ ] Compare MQTT/API payloads, state transitions, failure handling and persistence.
- [ ] Re-fetch/check upstream and review new deltas before claiming completion.
- [ ] Set status to `VERIFIED` only if Section 6 gates all pass; otherwise reflect the real partial/blocking state.
- [ ] Regenerate parity report; ensure test evidence, IDs and pins are committed together.

### Phase E — Report (required in every agent response)

Report concisely:

```text
Task / inventory IDs:
Upstream baseline: app SHA; herdsman/converters version + SHA/integrity
Latest upstream check: UTC time, heads, CURRENT/DRIFT_DETECTED/UNVERIFIED
Original source read: paths + symbols; original tests read
Behavior implemented: precise summary
Tests: exact commands, passed/failed/skipped; hardware yes/no
Differences/unknowns: explicit list (or none)
Parity tracker: Stage 1 Z-Stack % and global upstream % before -> after,
                each numerator/denominator; weighted and scenario %;
                inventory audit completeness; device/adapter coverage if affected
New upstream deltas: links/IDs
Blockers and next highest-priority gap:
```

Never write “done”, “feature complete”, “100% compatible” or similar if evidence, freshness or test coverage is insufficient. When the task only involves analysis, update inventory/research notes and **do not** change verified progress without passing evidence.

## 11. Prioritization and definition of done

- **P0:** safety/security, storage compatibility, no data loss, Zigbee adapter functionality, network join/interview, stable MQTT contracts, configuration and restart/recovery.
- **P1:** common converter families and device operations, Home Assistant discovery, groups, reporting, availability, OTA, frontend/control API.
- **P2:** less common devices, niche features, polish, performance improvements that do not change behavior.

Priorities guide implementation order, **not scope deletion**. A feature is done only when its reference research, mapping, tests, evidence, upstream review and inventory update are all present. Reviewers should reject PRs that omit these elements or increase parity percentages through unverified labels.

## 12. Minimum initial bootstrap when no tracker exists

1. Enumerate actual upstream modules, public configuration options, API/MQTT contracts, test suites, **all upstream adapter families**, device definitions, converter categories and documented workflows from pinned revisions; include all in the global inventory even if deferred to later stages.
2. Populate `inventory.csv` with stable IDs, source references and initial statuses; do not claim the inventory is complete before the audit has covered the ecosystem.
3. Start all unverified rows as `DISCOVERED`/`RESEARCHED` with `verified_cases=0`; define scenarios and weights only from real analysis.
4. Record precise baseline and latest-head metadata in `upstream-lock.json`. Suggested JSON shape (example **schema only**, never fill with fictitious commits):

```json
{
  "baseline": {
    "zigbee2mqtt": {"repo": "https://github.com/Koenkk/zigbee2mqtt.git", "commit": null, "version": null},
    "zigbee-herdsman": {"repo": "https://github.com/Koenkk/zigbee-herdsman.git", "commit": null, "resolved_version": null, "lock_integrity": null},
    "zigbee-herdsman-converters": {"repo": "https://github.com/Koenkk/zigbee-herdsman-converters.git", "commit": null, "resolved_version": null, "lock_integrity": null},
    "frontend": {"package": null, "resolved_version": null, "commit": null, "lock_integrity": null}
  },
  "latest_heads": {},
  "last_successful_fetch_utc": null,
  "last_checked_utc": null,
  "freshness_status": "UNVERIFIED",
  "baseline_approved_at_utc": null
}
```

5. Build `scripts/parity_report.py` or equivalent with CI assertions, **both stage-1 and global denominators** and stage labels; generate the first dashboard **from actual rows only**. The initial percentage may be 0% or N/A. Never use imaginary totals or sample rows in a real report.
6. Select one narrow vertical slice (e.g. a concrete device message → converter → state → MQTT), trace it completely, capture reference fixtures, implement, test, and only then mark its inventory rows verified.

## 12A. Conflict-handling and approvals

**Ask the user before making an architectural or scope decision** when upstream compatibility appears to require: native/C dependencies, a new event-loop/concurrency topology, dropping the 10s ZNP request policy, changing static cluster handlers, removing CC2531 support, deprioritizing or excluding the mandatory CC2652P Dongle-P, changing existing MQTT/HA contracts, destructive database migrations, or excluding an upstream feature from global parity. Present the precise original behavior with source pointers, the conflicting local rule, options, and impact on both metrics. Until answered, mark affected rows `BLOCKED`, preserve existing behavior and continue independent research/tasks.

If upstream documentation and old notes disagree, **research current pinned code first**; ask only when a genuine design decision remains. Never silently resolve it by rewriting a fixed project rule. For configuration-specific upstream behavior, preserve applicability predicates rather than treating an example as universal.

## 13. Final instruction to the coding agent

**Your job is to port the accumulated behavior of Zigbee2MQTT, not to write what you think Zigbee2MQTT should do.** Every coding task begins with a verified current upstream fetch and deep source reading, and ends with reproducible evidence and an honest parity-tracker update. If you cannot demonstrate that a Rust feature behaves like the pinned original, the feature is **not verified**.
