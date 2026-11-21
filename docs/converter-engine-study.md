# Converter engine feasibility and selection study

Date: 2026-11-21. Status: study complete; **quickjs-oxide selected**, with a
conditional GO for the next-stage runtime prototype and a NO-GO for production
against the current performance gate.

This document records an empirical comparison of pure-Rust JavaScript engines
against the pinned upstream converter package. It is not a runtime
implementation. Prototype sources live in
[`tools/engine-study/`](../tools/engine-study/); raw measurements live in
[`tools/engine-study/results/`](../tools/engine-study/results/).

## 1. Scope and question

The post-R1 converter runtime must execute the pinned
`zigbee-herdsman-converters` JavaScript without a C dependency and without
rewriting converter logic. This study answers:

1. Which pure-Rust engines can execute the **real** upstream package
   (not synthetic JavaScript) through a CommonJS loader?
2. Do they support the language/async/host surface the converters need?
3. Can a host bound runaway scripts, cap memory and recover?
4. What do they cost on the Routerich AX3000 compared with the Node.js
   baseline from [`converter-benchmark-baseline.md`](converter-benchmark-baseline.md)?
5. Is the next stage a GO or NO-GO, and with which engine?

Out of scope: a production runtime, MQTT/ZNP integration, the full device
catalog, and any C or C++ engine. `rquickjs`/QuickJS-ng is therefore excluded by
the pure-Rust constraint; it is not a candidate in this study.

## 2. Candidates

| Engine | Version | Pure Rust | Embedding/CJS | Guard features | Screening |
|---|---|---:|---|---|---|
| **quickjs-oxide** | 0.5.0 (2026-11-18) | yes, `unsafe_code = forbid` | Runtime/Context API, host functions, modules, heap limits, interrupts | heap byte limit, interrupt, catchable OOM, recovery | **selected** |
| **Boa** | 0.22.0 (2026-10-20) | yes | Context/jobs API, custom CJS loader | loop/recursion/stack limits; no interrupt; no heap limit | fallback/reference |
| **Kataan** | 0.0.10 (2026-11-07) | yes | `Interp`/`Ctx` host functions, snapshot/JIT | cooperative interrupt and size limits; no heap byte limit; 65,535-register compiler cap | **rejected** |
| quickjs-oxide-cli | 0.5.0 | yes | CLI only | n/a | not an embedding target |
| Nova VM | 1.0.0 | yes | VM internals; no stable CJS/host embedding surface | n/a | not feasible in this stage |
| `rquickjs` / QuickJS-ng | — | **no (C)** | mature | — | excluded by AGENTS §6 |

The long list of crates named “JavaScript engine” is mostly FFI (V8/JSC/
QuickJS/Duktape), TypeScript tooling, or abandoned interpreters. Only the
three above were viable enough to execute the upstream package.

quickjs-oxide is young (released four days before this study), so the
recommendation includes a pinning/maturity gate. It is nevertheless the only
candidate that combines real upstream compatibility with heap limits and
interrupts.

## 3. Prototype method

The same prototype shape was used for each engine:

- a **CommonJS loader written in JavaScript** (`bootstrap.js`) with module
  cache, relative/package resolution, and `new Function` wrappers;
- upstream package `26.125.0`, `zigbee-herdsman` `11.2.0`, zigbee2mqtt
  `3c841fdf`;
- the real `models-index.json` mapping for definition selection;
- a JSON snapshot of the real `Zcl` enums/clusters (`fixtures/zcl.json`,
  276 KB) and `Zcl.Utils` lookup with unknown-cluster synthesis;
- Node shims: minimal `Buffer`, `TextEncoder`/`TextDecoder`, `assert`,
  `semver`, `path`, `util`, `fs`, `process`, `console`;
- a fixed clock (`2026-11-20T12:00:00Z`) and fixed transaction-id ranges;
- JS device/endpoint mocks with Promise-based `read/write/command/bind/
  configureReporting`, call recording and a `__hostOp` Rust round-trip;
- the same 36 converter scenarios as `tools/bench/corpus/golden.json`
  (excluding the `package-load` pseudo-scenario), including the real external
  `L101Ze_z2m.js`;
- Rust host functions: `__readFile`, `__fileExists`, `__now`, `__log`,
  `__hostOp`.

Execution used a single upstream call implementation for all engines. Scenario
results were compared with the Node golden corpus for `result`, `published` and
the host-call sequence. A run is only “compatible” if the real upstream code
produced the same observable output.

Commands and build notes: [`tools/engine-study/README.md`](../tools/engine-study/README.md).

## 4. Functional compatibility with real upstream code

36 scenarios: IKEA on/off, brightness and color; Lumi occupancy; Tuya
datapoints and M9Pro; Danfoss/Lytko thermostats; multi-endpoint TS0002; two
`configure` paths with sequential host I/O; `onEvent`; heavy `prepareDefinition`
(yokis 81 extends, Icon2); the external L101 converter.

| Engine | Pass | Result/published parity | Host-call parity | Failures |
|---|---:|---|---|---|
| quickjs-oxide 0.5.0 | **36/36** | exact | exact (ops/endpoints/attributes) | none |
| Boa 0.22.0 | **36/36** | exact | exact | none |
| Kataan 0.0.10 | 33/36 | exact on passing cases | exact on passing cases | `fz-ts0002-ep2-onoff`: `RangeError: too many registers`; `set-ts0002-ep2-onoff` and `fz-tuya-m9pro-multi-dp`: `TypeError: find is not a function` |

The Kataan failures are in the real `devices/tuya.js` file. The compiler uses
`u16` registers and refuses a function that exceeds 65,535 registers
(`nbvm.rs`, `TOO_MANY_REGISTERS`); the Tuya converter functions are large
enough to hit that limit. This is not fixable by a loader shim without editing
upstream converter code, which the project forbids.

Language probes (classes, optional chaining, nullish, spread, destructuring,
generators, async generators, Proxy/Reflect, typed arrays, BigInt, named
groups/lookbehind, `new Function`, async/await, microtask order) passed on all
three engines once the loader supplied a `Buffer` shim.

### Required compatibility shims

Confirmed necessary for the real package:

1. **CJS loader** with `module.exports`, cache, cycles, `require` relative and
   package subpaths (`zigbee-herdsman-converters/...`,
   `zigbee-herdsman/dist/...`).
2. **`models-index.json` selection**. Picking by `definition.model` alone is
   wrong: e.g. `TS0002` maps to `_TZ3000_h1ipgkwn` index 172. The prototype had
   to mirror the index to match converter selection and host calls.
3. **Zcl shim**: `DataType`, `ManufacturerCode`, `BuffaloZclDataType`, `Status`,
   `Direction`, `FrameType`, `StructuredIndicatorType`, `Clusters`,
   `getCluster` with unknown-cluster synthesis, and
   `getTimeClusterAttributes` (fixed clock, local offset, DST sentinels).
4. **Node builtins**: `Buffer` (Uint8Array subclass), `TextEncoder`/`Decoder`,
   `assert` (including `.default`), minimal `semver` (`gt/gte/lt/valid`),
   `path`, `util`, `fs.existsSync/readFileSync`.
5. **Timers**: `setTimeout(...).unref()` and `clearTimeout`; the study stub
   returns a timer object but does not execute callbacks. A production host
   needs a real timer queue.
6. **Host boundary**: Promise-based endpoint methods and a JS↔Rust host call
   for every operation; `meta.publish`, `postProcessConvertedFromZigbeeMessage`
   and `prepareDefinition` must stay upstream JS.

## 5. Runaway, memory and recovery

Probe: [`guard_probe.rs`](../tools/engine-study/src/bin/guard_probe.rs),
results in [`results/guard/`](../tools/engine-study/results/guard/).

| Property | Boa 0.22 | Kataan 0.0.10 | quickjs-oxide 0.5.0 |
|---|---|---|---|
| Infinite loop | loop-iteration limit: error after **162 ms** (10M on x86) | host `Interrupt` from another thread: **100 ms** | interrupt: **100 ms x86 / 133 ms router** |
| Recursion | recursion limit: **0.4 ms** | call-depth `RangeError` | stack limit (`RangeError`/`InternalError`) |
| Long builtin (regex/sort/repeat) | **not interruptible**; the probe was killed after 15 min | interrupt checks back-edges/call entry; regex has a step budget | interrupt checks execution; a hard deadline is enforceable |
| Heap cap | none | no total heap byte cap (string/array/JSON/BigInt caps only) | `HeapLimits { max_bytes }`: 16 MB OOM in **27 ms x86 / 336 ms router**, catchable |
| Recovery | context continues after a runtime-limit error | reuse after `Interrupt::clear()` works | same runtime continues after interrupt and catchable OOM |
| Hard OOM | OS kill | OS kill (Routerich killed it during package load) | heap limit prevents reachable-memory OOM; process isolation still recommended |

Engine-level limits are necessary but not sufficient: a production runtime
should still run converters in a supervised worker/process with an RSS/AS
limit and a watchdog, and recreate the realm on failure. quickjs-oxide is the
only candidate that provides both an interrupt and a byte-level heap cap in
the embedding API.

## 6. Performance

Environment: x86_64 (Node v24 diagnostic) and Routerich AX3000, OpenWrt
24.10.8, aarch64, Node service left running. Engines are static aarch64-musl
builds; router runs used `nice -n 19`. Raw data:
[`results/x86/`](../tools/engine-study/results/x86/),
[`results/routerich/`](../tools/engine-study/results/routerich/).

### Cold start and memory

| Engine | x86 cold root / prepare | Router cold root / prepare | x86 maxRSS (36 scenarios) | Router maxRSS |
|---|---:|---:|---:|---:|
| Node.js baseline | 0.69–0.80 s / ~0.1 ms + file load | 2.54–3.66 s / device file 0.08–1.93 s | 86–141 MB | 51–74 MB |
| **quickjs-oxide** | **0.22–0.46 s / 47–63 ms** | **1.87–2.49 s / 0.56–0.60 s** | **81–84 MB** | **57 MB** |
| Boa | 0.49–0.55 s / 0.23–0.29 s | 4.7–5.3 s / 2.3 s (first definition) | 94 MB | 93 MB (full corpus, 1 iteration) |
| Kataan | 3.6–3.9 s / 0.55–0.64 s | **OOM-killed during package load** | 634 MB | not measurable |

Node values are the converter benchmark baseline (package root and first
device file); they are not a like-for-like total, because Node loads the
package with V8 snapshots while the prototypes parse and evaluate JS. The
relevant result is that quickjs-oxide is the only engine whose cold path and
RSS are in the same range as Node on the router.

Kataan's 634 MB on x86 and OOM on the router is a hard failure: the router has
497 MB total and less than 100 MB available while Node is running. Boa fits
when definitions are split into groups (39–80 MB per group), but a single
process loading all 36 scenarios exceeded the remaining memory at 20
iterations (OOM at ~85 MB). A production runtime must lazy-load only paired
devices and isolate/restart the realm.

### Warm converter-only latency (Routerich, ms)

Node numbers are the current `converterStats` baseline; engine numbers are the
prototype `converterMs` (upstream callback only). Medians; qjs = quickjs-oxide.

| Scenario | Node p50/p95 | Boa p50/p95 | qjs p50/p95 |
|---|---:|---:|---:|
| `fz-ikea-onoff` | 0.070 / 0.124 | 0.233 / 0.511 | 0.086 / 0.138 |
| `fz-ikea-brightness` | 0.042 / 0.113 | 0.342 / 2.736 | 0.099 / 0.305 |
| `fz-tuya-m9pro-multi-dp` | 0.162 / 0.380 | 0.275 / 0.914 | 0.068 / 0.424 |
| `set-ikea-onoff` | 0.432 / 1.412 | 1.551 / 3.802 | 0.496 / 2.298 |
| `set-ikea-brightness` | 0.440 / 1.505 | 2.136 / 4.219 | 0.493 / 1.244 |
| `configure-danfoss` | 6.723 / 15.552 | **1110.9 / 1213.7** | **153.6 / 221.7** |
| `configure-l101z` | 0.526 / 2.029 | 0.707 / 2.160¹ | 2.091 / 5.659 |
| `fz-external-l101-temp` | 0.081 / 0.100 | 0.450 / 1.400 | 0.097 / 0.183 |

¹ Boa `configure-l101z` is from the 30-iteration grouped run; other Boa values
use the same grouped run.

- **Reports and sets:** quickjs-oxide is at or near Node converter parity
  (0.08–0.40 ms p50). Boa is 2–6× slower on reports and 2–4× on sets.
- **Configure with 51–54 sequential host calls:** both interpreters are far
  slower than V8. quickjs-oxide is ~10–14× Node p95 on Danfoss and ~2.8× on
the simpler Lytko configure; Boa is ~80× on Danfoss. This is the
  dominant performance risk.
- **p95 outliers:** quickjs-oxide showed a 1.4 s p95 for `fz-lumi-occupancy`
  in one 10-iteration router run (GC pause); p50 stayed 0.31 ms. This needs a
  GC/heap-limit tuning pass.

### x86 converter-only medians

| Scenario | Boa p50 | Kataan p50 | qjs p50 |
|---|---:|---:|---:|
| `fz-ikea-onoff` | 0.025 | 0.014 | **0.004** |
| `set-ikea-brightness` | 0.153 | 0.090 | **0.033** |
| `configure-danfoss` | 137.5 | 148.5 | **11.0** |
| `configure-l101z` | 0.707 | 0.153 | 0.144 |
| `fz-external-l101-temp` | 0.042 | 0.039 | **0.009** |

Kataan is fast on the scenarios it can compile, but that advantage is
irrelevant while three real Tuya scenarios fail and the router run OOMs.

## 7. Verdict

### Engine selection

**Select quickjs-oxide 0.5.0** as the engine for the Converter Runtime.

Reasons:

- only engine with 36/36 real-upstream scenarios and exact output/host-call
  parity;
- warm report/set performance at or near Node converter parity on the router;
- cold start (2.0–2.6 s) and RSS (57 MB) in the Node range on the router;
- built-in interrupt (~133 ms on router) and 16 MB heap cap (catchable OOM in
  ~336 ms) with recovery in the same runtime;
- pure Rust, no C, `unsafe_code = forbid`.

Rejected:

- **Kataan: NO-GO.** Hard compiler limit on real Tuya code, 634 MB RSS on
  x86, and OOM during package load on the router.
- **Boa: NO-GO as primary.** Functionally correct, but 2–6× slower on
  reports/sets and ~1.1 s per Danfoss configure on the router; no interrupt
  and no heap cap. Keep it as a reference/secondary implementation.

### GO / NO-GO

- **GO** for a focused next-stage **prototype** of the runtime on
  quickjs-oxide, because the engine is viable, functionally compatible and can
  be bounded.
- **NO-GO for production** until the scheduler and remaining gates are
  complete. The follow-up
  [`converter-async-host-validation.md`](converter-async-host-validation.md)
  found that the Danfoss slowdown was a study-shim Zcl lookup cost (962
  uncached `getCluster` calls); after semantic-preserving lookup caches the
  Routerich configure p95 is **25.1 ms vs Node 15.6 ms (1.6×)** and the true
  async floor is 22–26 ms. Reports are at parity; the remaining production
  risk is scheduler fairness under concurrent devices, not the engine's
  language support.
- Do not start the production implementation before:
  1. the scheduler prototype is replaced by bounded, fair per-device
     scheduling and re-measured under concurrent load; the configure path is
     already within the p95 gate after the lookup-cache fix;
  2. the quickjs-oxide 0.5.0 pin is vendored/audited and the build is
     reproducible for aarch64-musl;
  3. timer execution, `setTimeout(...).unref()` semantics, Zcl/time shims and
     `models-index` selection are covered by differential tests;
  4. the realm/process supervision policy (heap limit, RSS watchdog, restart)
     is exercised on the router.

## 8. Reproduction

See [`tools/engine-study/README.md`](../tools/engine-study/README.md). Short
version (host build; static aarch64-musl is produced with the Rust `rust-lld`
and Zig as the C compiler/linker):

```bash
cd tools/engine-study
cargo build --release --features qjs --bin converter_probe
cargo build --release --features boa --bin converter_probe
cargo build --release --features kataan --bin converter_probe
cargo build --release --features qjs --bin guard_probe

./target/release/converter_probe --iterations 20 > results/x86/quickjs.json
./target/release/guard_probe
```

The router commands and the exact package/`herdsman`/fixture paths are in the
same README. Raw runs used for this report:
`results/x86/{boa,kataan,quickjs-oxide}-run{1,2,3}.json`,
`results/routerich/quickjs-oxide-run{1,2,3}.json` and the Boa grouped runs.

## 9. Evidence limits

- The prototype is not the production host: device/endpoint mocks, a simplified
  Zcl shim and a JS loader add overhead that is included in wall time but
  excluded from `converterMs`.
- Timer callbacks are not executed by the study stub; scenarios that depend on
  timers need the production timer queue.
- No MQTT/ZNP/network or bridge event loop is included. This is a converter
  engine study, not an integration benchmark.
- Router runs share the live Node service and its memory pressure. Kataan's
  OOM is partly an environment constraint, but its 634 MB x86 footprint and
  compiler failures are independent of the router.
- The golden corpus and Node results were not modified by this study.
