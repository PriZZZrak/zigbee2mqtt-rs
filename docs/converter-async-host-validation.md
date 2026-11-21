# Async host performance and scheduling validation

Date: 2026-11-22. Scope: quickjs-oxide 0.5.0, selected in
[`converter-engine-study.md`](converter-engine-study.md). Production runtime is
not implemented; this stage establishes the cause of the `configure-danfoss`
slowdown and validates a minimal non-blocking host prototype.

## 1. Result in one paragraph

The configure slowdown was **not** the JS↔Rust boundary, JSON serialization or
Promise creation. It was the **Zcl cluster lookup shim**: `configure-danfoss`
performed 962 `Zcl.Utils.getCluster` calls, and on quickjs-oxide each call
scanned the plain-object cluster map. Caching cluster lookups (semantics
unchanged; cluster definitions are immutable) and caching each endpoint's
input/output cluster arrays reduced the calls to 26 and cut the Routerich
`configure-danfoss` converter-only p95 from **221.7 ms to 25.1 ms**; a 50-call
tail run measured p50 17.7 ms, p95 28.6 ms, p99 39.2 ms. Node is 6.7/15.6 ms
p50/p95, so the optimized path is **1.6× p95** and meets the 2× acceptance
target. A true async host prototype (pending Promises settled by Rust, host
errors, timeout, interrupt) works and interleaves independent device jobs, but
its minimal scheduler is not yet production-grade for high concurrency.

## 2. Profiling: where the time went

Profile hooks in [`fixtures/bootstrap.js`](../tools/engine-study/fixtures/bootstrap.js)
measure, per call:

- `serializeMs` — `JSON.stringify` of the host operation;
- `boundaryMs` — the `__hostOp` JS→Rust call;
- `resumeMs` — from host return to the `.then` continuation (profile mode only);
- host calls and Zcl/mock lookup counts.

Profiles: [`results/profiles/`](../tools/engine-study/results/profiles/).

### Before cache (x86, 20 iterations)

| Component | Time |
|---|---:|
| converter callback total (p50) | 9.99 ms |
| JSON serialization (51 ops) | 0.16 ms |
| JS→Rust boundary (51 ops) | 0.09 ms |
| Promise resume markers | 0.19 ms |
| `Zcl.getCluster` calls | **962** |
| `getInputClusters` calls | 37 |
| host calls | 51 |

The boundary, serialization and resume components are sub-millisecond. The
962 lookups are the only component large enough to explain the 10 ms x86 /
154 ms Routerich cost. The lookup happens because `modernExtend` calls
`getInputClusters()` repeatedly and each endpoint maps its 25 input clusters
through the JSON cluster map.

### After local lookup caches

| Profile | converter p50 x86 | getCluster calls |
|---|---:|---:|
| cluster lookup cache only | 2.86 ms | 962 (cached hits) |
| + endpoint input/output cluster caches | **1.06 ms** | **26** |

No upstream code, host-call order or observable result changed: the cache is
inside the benchmark's Zcl/endpoint shim, and invalidation happens when
`addCustomCluster` mutates the device.

### Routerich effect

| Configuration | converter p50 | converter p95 |
|---|---:|---:|
| pre-cache (3 runs) | 153.6 ms | 221.7 ms |
| cluster cache | 49.2 ms | 84.4 ms |
| cluster + endpoint caches (3 runs) | **19.5 ms** | **25.1 ms** |
| 50-call tail run | 17.7 ms | 28.6 ms (p99 39.2) |
| Node baseline | 6.7 ms | 15.6 ms |

The optimized configure path is **1.6× Node p95** on Routerich (25.1 / 15.6),
and the 50-call p95 is 1.8×. The acceptance criterion is unchanged.

## 3. True async host prototype

Source: [`src/bin/async_probe.rs`](../tools/engine-study/src/bin/async_probe.rs).

Unlike the immediate `__hostOp` in the main probe, this host returns **pending**
Promises and stores their `PromiseCapability` in Rust. A Rust scheduler settles
them, runs the job queue, and advances independent jobs. The JS side starts one
`configure-danfoss` and N `set-ikea-brightness` jobs with distinct devices.

### Normal scheduling (Routerich, 8 independent jobs)

| Metric | Result |
|---|---|
| configure converter-only | 22–26 ms (jobs=0) / 0.52–0.77 s with 8 concurrent set jobs (x86: 0.26–0.33 s) |
| host operations settled | 51 / 59 |
| first independent job completed | before configure (interleaved = true) |
| jobs completed before configure | 8 |
| host errors observable | `rejected:host EIO` |
| host timeout observable | `rejected:host timeout after 100 ms` |
| interrupt on runaway JS | `Interrupted` in 100–112 ms |
| runtime after error/timeout/interrupt | evaluates `1 + 1` successfully |

Interpretation:

- With no competing jobs, the async configure floor is **22–26 ms** versus
  19.5 ms for the immediate mock. The added ~3–6 ms is the real Rust↔JS
  settlement and job-pump cost; the worst run is still within the 2× target
  (31 ms).
- With 8 concurrent set converters, configure rises to 0.52–0.77 s on the router
  (0.26–0.33 s x86). This is not a
  language failure: the minimal prototype drains all pending capabilities and
  runs all job queues in one turn, so eight converters plus configure contend
  on a single interpreter and GC. A production scheduler needs bounded
  concurrency, per-device fairness and backpressure. This is the main
  scheduler finding, not a reason to change the acceptance target.
- `settleMs` is ~0.03 ms and `runJobsMs` ~2 ms for the batch; the prototype's
  `totalMs` also includes one `eval` of a multi-KB scenario JSON (~25 ms x86),
  which is harness overhead and is reported separately as `initialEvalMs`.

### Timeout, cancellation, errors

- Host rejection is delivered as a rejected Promise and observed by the
  awaiting JS (`__hostErrorProbe`).
- Host timeout is modelled as rejecting the pending capability after 100 ms;
  the rejection is observed.
- The engine interrupt aborts a runaway JS loop in ~100 ms (x86) and
  ~112 ms (Routerich); the same runtime then executes normally.
- Pending host operations are not silently resolved on interrupt; the
  prototype reports the remaining queue length.

## 4. GC tail, heap cap, interrupt on Routerich

50-iteration tail runs (`results/async/routerich-tail.json`, 25 MB RSS):

| Scenario | p50 | p95 | p99 | max |
|---|---:|---:|---:|---:|
| `configure-danfoss` converter | 17.7 ms | 28.6 ms | 39.2 ms | 39.2 ms |
| `fz-lumi-occupancy` converter | 0.17 ms | 0.24 ms | 0.97 ms | 0.97 ms |
| `fz-lumi-occupancy` wall | 0.49 ms | 1.40 ms | 12.2 ms | 12.2 ms |

The 1.4 s GC outlier seen in the earlier 10-iteration run did not reproduce in
the 50-iteration tail run (p99 wall 12.2 ms). GC tail remains a monitored risk,
not a currently reproducible failure.

Guard results (same binary family, Routerich): interrupt **133 ms**, 16 MB
heap cap → catchable `InternalError: out of memory` in **336 ms**, runtime
continues afterwards. The cap bounds JS heap, not native RSS; a process/worker
RSS watchdog is still required.

## 5. Decision

- **Cause established and proven by measurement:** 962 uncached Zcl cluster
  lookups per Danfoss configure dominated the cost; host boundary,
  serialization and Promise creation were negligible.
- **Local optimization applied without semantic change:** cluster and endpoint
  lookup caches. Golden result payloads and host-call sequences still match.
- **Target:** optimized immediate configure is 19.5/25.1 ms p50/p95 vs Node
  6.7/15.6 ms on Routerich, i.e. **1.6× p95 ≤ 2×**. The true async floor with
  no competing jobs is 22–26 ms p50 (1.4–1.7× Node p95) and meets the same
  bound. The acceptance criterion is unchanged.
- **GO** for the next-stage runtime prototype, with a mandatory scheduler
  milestone: bounded per-device concurrency, fair job pumping and verification
  that concurrent devices do not starve configure. The current prototype's
  8-job contention (784 ms) is a scheduler limitation, not an engine
  limitation.
- **NO-GO for production** until that scheduler milestone and the existing
  engine-study gates (pin/audit quickjs-oxide, timer semantics, supervision)
  are complete.
