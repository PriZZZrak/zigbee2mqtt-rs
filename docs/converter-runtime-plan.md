# Converter runtime plan (post-R1)

Status: proposed for external audit; Node.js baseline measured 2026-11-20.
Scope: external converters first, then the full upstream device database.
This document is a plan, not an implementation claim. The measured baseline is
[`converter-benchmark-baseline.md`](converter-benchmark-baseline.md).

## 1. Position relative to R1

R1 (the core milestone) is closed. Its scope deliberately excluded
`zigbee-herdsman-converters`, JavaScript execution, vendor quirks and the
device catalog (AGENTS §4/§5 Q4/Q5). The converter work therefore starts
**after** R1 and does not change the R1 definition of done.

Current R1 state in this tree:

- ZNP/Z-Stack, ZDO/ZCL, controller model, MQTT bridge API, HA discovery,
  Windfront protocol, storage and operations are ported and tested;
- `crates/zigbee-converters` contains only the shared host surface
  (`Definition`, `Expose`, access, registry, lookup), not the device database;
  the registry is empty at runtime, so devices are definition-less;
- there is no JavaScript runtime, no `external_converters` loader and no
  `bridge/request/converter/*` or `bridge/converters` implementation in R1;
- the test router runs the Node.js Zigbee2MQTT service; the Rust binary is a
  HIL artifact.

## 2. What is ported and what is executed as upstream JS

The converter package is **not translated to Rust**. The pinned upstream JS is
shipped as an artifact and executed by an embedded engine.

| Component | Treatment |
|---|---|
| `dist/converters/*.js` (`fromZigbee`, `toZigbee`, `actions`) | shipped and executed as JS |
| `dist/lib/*.js` (`exposes`, `modernExtend`, `reporting`, `utils`, `tuya`, `store`, `legacy`, …) | shipped and executed as JS; shared modules loaded once |
| `dist/devices/*.js` | shipped and executed as JS; lazy-loaded per used file |
| `dist/models-index.json` | build-time input for a compact Rust index (no runtime JS lookup) |
| `indexer`/`findByDevice` behaviour | reproduced in the Rust index |
| `generateDefinition` | JS if/when needed; not ported |
| CJS loader, `require` resolution, `Buffer`, `assert`, `util`, `path`, `semver`, `iconv-lite`, host shims | Rust/JS bootstrap infrastructure |
| `msg.endpoint.read/write/command`, `reporting.bind`, `meta.device`, `publish`, timers | Rust host API bridged to JS |
| `Definition`/`Exposes`/`Fingerprint` Rust structs | thin data boundary for bridge/HA, populated from JS |

Reason: reimplementing `lib/` and `converters/` would duplicate thousands of
vendor-specific functions, diverge from upstream, and still require a JS engine
for external converters.

## 3. Why the archived Boa host was slow

Evidence from `archive/boa-js-host-20261106`:

- one Boa context per `(IEEE, model)` session; shared modules reloaded per
  session;
- definitions/exposes for the device list generated synchronously and waited
  for by the bridge event loop;
- **~6–9 s** to generate definitions on the router and **30–40 s** before the
  first command after startup (cold sessions);
- a frame waited up to **500 ms** for a session; command handling waited
  longer; MQTT/frontend stalled.

The new design removes the causes, not just the engine.

## 4. Target architecture

```
Bridge event loop (single Tokio task)
 ├─ Zigbee / MQTT / storage / HA / frontend      (unchanged)
 ├─ ConverterDispatcher (Rust)
 │   ├─ Rust definition index (models-index)
 │   ├─ prepared-definition cache (metadata + JS handler ids)
 │   └─ per-device ordered job queue
 └─ Converter worker (dedicated thread)
     ├─ built-in realm: shared modules once + lazy device files
     ├─ external realm: user JS, isolated, atomically replaceable
     ├─ CJS loader + host shims
     └─ host API: read/write/command/bind/timers/state/publish
```

Rules:

1. One shared realm for the built-in database, not one context per device.
2. Shared modules are loaded once; a device file is loaded once on first use.
3. Rust resolves model/fingerprint/whiteLabel from the generated index.
4. Only paired devices' definitions/exposes are prepared; not all 5k.
5. Conversion is asynchronous. Reports are queued per device and published
   when ready; the bridge loop is never blocked on JS warmup.
6. Commands may wait a bounded time (target 1 s) and return a real error on
   timeout, never a false success.
7. External JS runs in a separate realm with an atomic reload and
   last-known-good fallback.
8. Per-call limits, watchdog and cancellation; no unbounded waits.

## 5. Performance strategy

- **Rust index**: parse `models-index.json` once at build time into a compact
  index (model, zigbeeModel, fingerprints, whiteLabel, file/index, static
  metadata). No JS for lookup.
- **Shared module cache**: `converters/fromZigbee`, `toZigbee`, `lib/exposes`,
  `modernExtend`, `reporting`, `utils` (~650 KB total) evaluated once.
- **Lazy device files**: 439 files / 11.7 MB total; only files for paired
  devices are loaded. Largest file (`tuya.js`, 1.4 MB) is loaded in the
  background.
- **Warmup after interview**: enqueue definition preparation immediately;
  reports before readiness are queued, not dropped and not blocking.
- **Prepared definition cache**: merge extends/exposes once per definition;
  cache Rust metadata and JS function handles.
- **Reuse JS objects**: per-device `msg.meta`, `device`, `options`, `state`
  objects are reused/updated instead of rebuilt per frame.
- **Precomputed dispatch**: `(definition, cluster, type)` → converter list in
  Rust.
- **Bytecode (engine permitting)**: precompile shared + device files at
  artifact build to remove parser cost on the router.

## 5a. Measured Node baseline (2026-11-20)

The complete report is [`converter-benchmark-baseline.md`](converter-benchmark-baseline.md);
the harness and corpus are in [`../tools/bench/`](../tools/bench/).

Key measured facts on Routerich AX3000 (Node v22.23.4, converters 26.125.0,
Node service left running):

- package root cold load: **2.54–3.66 s** (median 2.56 s in fresh processes);
- first device-file load: 0.075 s (`lytko.js`) to **1.93 s** (`tuya.js`),
  0.69 s for `ikea.js`, 0.89 s for `lumi.js`;
- warm `fromZigbee`/`set`/`get`: p50 **0.2–0.9 ms**, p95 **0.5–4.3 ms**;
- `configure-danfoss` (54 sequential host calls): p50 **6–9 ms**, p95
  **29–38 ms** at load average 2.3–3.3, p99 up to 139 ms;
- heavy `prepareDefinition` (81-extend yokis / Icon2): p50 0.6–0.8 ms,
  p95 5–8 ms;
- external `L101Ze_z2m.js`: prepare p50 0.12 ms; conversions 0.3–0.6 ms;
- harness maxRSS on the router: **51–74 MB**; x86 86–141 MB.

The cold root/device-file times are large enough that a runtime must not load
them on the bridge request path. The measured acceptance target is in the
baseline report: per-scenario Rust p95 ≤ 2× Node p95 on the same corpus,
functional equality first, and no bridge stall > 50 ms p99. These are proposed
thresholds, finalized after the engine spike.

## 6. Engine choice

Engine is treated as a replaceable component behind a small host boundary.
The measured comparison is in
[`converter-engine-study.md`](converter-engine-study.md); the follow-up async
host study is in
[`converter-async-host-validation.md`](converter-async-host-validation.md);
the scheduler prototype is in
[`converter-runtime-scheduler-prototype.md`](converter-runtime-scheduler-prototype.md)
and [`../crates/converter-runtime/`](../crates/converter-runtime/); bridge
contract in [`converter-runtime-bridge-contract.md`](converter-runtime-bridge-contract.md); engine
prototypes are in [`../tools/engine-study/`](../tools/engine-study/).

| Engine | Pure Rust | Real upstream corpus | Guard features | Role |
|---|---|---:|---|---|
| **quickjs-oxide 0.5.0** | yes | **36/36** | interrupt + heap byte limit + catchable OOM + recovery | **selected** |
| Boa 0.22.0 | yes | 36/36 | loop/recursion limits; no interrupt or heap limit | reference/fallback |
| Kataan 0.0.10 | yes | 33/36 (`tuya.js` compiler/`find` failures) | interrupt; no heap byte limit | **rejected** |
| quickjs-oxide-cli / Nova VM | yes | not an embedding target | n/a | not feasible here |
| QuickJS-ng (`rquickjs`) | no (C) | n/a | n/a | excluded by AGENTS §6 |

Decision: **quickjs-oxide 0.5.0** is the only pure-Rust engine that passed all
36 real-upstream scenarios and stayed in the Node range for cold start and RSS
on Routerich. It is **GO for the next-stage prototype**, but **NO-GO for
production** until the Promise-heavy `configure` path meets the p95 gate:
quickjs-oxide measured ~154 ms per Danfoss configure versus Node 6.7 ms p50 /
15.6 ms p95, and showed a 1.4 s GC p95 in one Lumi run. Reports and sets are
already at or near Node converter parity.

### Spike gates

Use the same `tools/bench/corpus/golden.json` and host-call expectations. The
quickjs-oxide prototype already matches result/published/host-call sequences
for all 36 scenarios; functional equality is a prerequisite for the runtime.

Proposed provisional gates (replace with the measured baseline after the
spike; the Node baseline is in `converter-benchmark-baseline.md`):

- per-scenario Rust **converter-only** p95 ≤ 2× Node `converterStats.p95` on
  Routerich (median of three runs); harness-total numbers are not the gate;
- the converter-level corpus must pass; the separate MQTT/ZNP integration
  benchmark is a later stage, and stress results are informational;
- cold package/definition work must not block `bridge/state online`; if on the
  request path, ≤ 2× Node cold load;
- worker RSS ≤ 1.5× Node harness maxRSS (target ≤ 110 MB) and bounded;
- no conversion blocks the bridge event loop > 50 ms p99.

The selected engine must not proceed to production if it misses the p95 gate.
First optimize/batch the host API and tune GC; do not switch to a C engine or
Node sidecar without explicit user approval.

## 7. Host API and module loader

Required host surface (driven by real converter usage):

- CJS `require`: relative files, package subpaths (`zigbee-herdsman-converters/...`),
  root package, `zigbee-herdsman`/ZCL shims;
- Node shims: `assert`, `buffer` (`Buffer`), `util` (inspect/format),
  `path`, `fs` (debug append only), `semver`, `fast-deep-equal`,
  `iconv-lite` where used;
- `msg`: cluster name, type, data, linkquality, endpoint; manufacturer/custom
  cluster and TSN;
- `msg.endpoint`: `read`, `write`, `command`, `bind`, `getClusterAttributeValue`,
  `ID`, `deviceIeeeAddress`, device lookup;
- `meta`: `device` (`ieeeAddr`, `modelID`, `manufacturerName`, endpoints,
  `getEndpoint`, `save`, custom clusters), `state`, `options`,
  `deviceExposesChanged`, `logger`, `publish`;
- `toZigbee` `convertSet`/`convertGet` and `ToTarget` (device/group);
- `configure`, `onEvent`, `reporting.bind`/`configure`;
- timers (`setTimeout`/`clearTimeout`), Promises and async host I/O.

Async endpoint I/O is a host request/response: JS receives a Promise; the
worker forwards the request to the bridge, and the bridge resolves it when the
ZNP response arrives. Per-operation timeout and cancellation are mandatory.

## 8. External converter milestone

1. `converter/save` validates name/code, writes atomically, evaluates in a
   scratch realm with limits, extracts definitions, updates the external
   index overlay and swaps the external realm.
2. `converter/remove` removes the file/definitions, re-resolves devices and
   publishes updates.
3. Retained `bridge/converters` and `bridge/devices` updated; responses do not
   wait for expensive device-list regeneration.
4. Device resolution prefers external definitions; `source: "external"`.
5. Reports and `set`/`get` work, including async read/write and
   `reporting.bind`.
6. `.js`, `.cjs` and `.mjs` support (or an explicit, tested limitation).
7. Restart reloads external files; invalid files fail loudly and leave the
   previous active set untouched.

First real target: `L101Ze_z2m.js` (Lytko L101Ze.v2 family, 28,750 bytes).

## 9. Full database

- Pin and package the upstream converter version (currently 26.125.0) as a
  versioned artifact (separate opkg or data-dir cache) with checksums and
  license notices.
- Build-time Node script generates the Rust index and verifies every
  `models-index` coordinate against a real device file.
- Runtime loads shared modules once and device files on demand; no all-device
  evaluation.
- Optional bytecode compilation at artifact build time (engine permitting).
- Host ABI version is part of the artifact identity; incompatible external
  converters fail validation, not silently misbehave.

## 10. Reliability and security

- external JS has no `fs`/network/process access; only the host API;
- loop/time/memory limits per call; watchdog interrupts runaway code;
- worker panic is contained; supervisor restarts it and fails pending jobs
  with a real error;
- one bad converter cannot kill the frame queue;
- device leave/shutdown cancels pending conversions and timers;
- no double conversion: JS is authoritative for devices with a definition;
  native fallback remains for definition-less devices.

## 11. Testing and acceptance

- Unit: CJS loader, requires, Buffer, timers, Promises, errors, limits,
  cancellation.
- Differential vs Node: identical `msg`/`meta` for the real external
  converter; compare publish payloads, state and ZCL frames.
- Definition payload/exposes equality against upstream.
- Reload/save/remove/restart tests.
- HIL on Routerich + Dongle-P: Lytko reports, set/get, configure, MQTT/HA/
  Windfront responsiveness, restart.
- Performance report with cold/warm/p50/p95/p99/RSS on the router.

## 12. Phases

1. Feasibility spike: Boa vs Kataan on real external converter; report.
2. Rust index + CJS loader + shared realm + lazy device files.
3. External converter save/load/remove + differential + HIL.
4. Generic host API: configure/onEvent/read/write/command/bind/timers.
5. Full database artifact, packaging, lazy loading.
6. Performance/HIL acceptance and documentation.

## 13. Risks / decisions

- Pure-Rust constraint vs QuickJS-ng/Node sidecar if the spike misses gates.
- Boa has no public bytecode serialization; cold parse must be hidden by
  warmup. Kataan has it but is young.
- Router overlay is nearly full (Node installed); converter artifact packaging
  must account for space, or use a dedicated opkg after Node removal.
- External converter compatibility with Node builtins; shims need differential
  tests.
- Memory budget on a 497 MB router with Node currently using ~97 MB RSS.
- `external_converters` and `bridge/converters` are not implemented in R1;
  they are part of this plan.

## 14. Verification log

Re-check of the R1 tree before tagging (2026-11-20):

```text
cargo test --workspace                  80 suites, 1577 passed, 0 failed, 14 ignored
cargo clippy --workspace --all-targets  exit 0 (warnings only)
cargo fmt --all -- --check              exit 1: 134 pre-existing files; changed files formatted
node tools/docs/check-parity-maps.mjs   OK: 11 maps, 0 errors, 2 warnings
model transcript                        16/16 operations match
MQTT transcript                         element-wise match; offline retained received by a new subscriber
DB oracle                               Node -> Rust -> Node, 7081 bytes preserved
database_roundtrip (--include-ignored)  8 passed (Node golden, real DB, model, Node rewrite)
yaml_dump                               3 passed (js-yaml golden, idempotence, write_if_changed)
fuzz_boundaries                         3 passed
```

Coverage (`docs/coverage.md`, code revision `aa21502`):

```text
stable:  regions 73.33%, functions 75.06% (2609/3476), lines 73.11% (26007/35573)
nightly: branches 72.56% (3593/4952), lines 71.94% (24405/33925),
         functions 74.40% (2622/3524)
``` The test router remains on the Node.js service; R1 HIL
evidence is in `docs/hil/2026-11-18-*` and `docs/hil/2026-11-19-*`.
