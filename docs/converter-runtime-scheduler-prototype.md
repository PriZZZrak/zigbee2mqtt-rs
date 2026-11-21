# Converter runtime scheduler prototype

Date: 2026-11-22. Status: minimal runtime prototype works; **GO** for the next
milestone (bounded production scheduler + timers), **NO-GO for production
integration** until the remaining gates below.

This stage implements a real Rust crate, not another study script:
[`crates/converter-runtime`](../crates/converter-runtime). It is not wired to
Zigbee, MQTT, Home Assistant or the bridge. It executes the pinned upstream
package through quickjs-oxide and validates the scheduler against the Node
golden corpus and comparable concurrent Node workloads.

## 1. Architecture

```
bridge/test caller
   │ submit(JobRequest{device, scenario})      respond(correlation, result)
   ▼                                                    │
bounded scheduler (per-device FIFO, round-robin ready devices)
   │                                                    │
   ▼                                                    │
single quickjs-oxide worker thread                      │
   ├─ CJS loader + Zcl/Node shims (shared for package and external)
   ├─ __startRuntimeJob → runScenario → upstream converters
   ├─ __hostOp(op) → pending Promise + HostRequest ──────┘
   └─ __runtimeDone / __runtimeFailed → completion queue
```

- One JS runtime/context. Built-in and external converters use the same
  `runScenario` and host boundary; the definition `source` selects the package
  or the external file.
- `PendingHosts` is keyed by a monotonically increasing correlation ID. A
  response is applied only if the ID is still pending; late responses are
  counted and ignored.
- `HostRequest` carries `correlation_id`, `job_id`, device and the operation
  JSON. The test responder simulates real host I/O; no Zigbee transport is
  involved.
- Per-device ordering: at most one active job per device; queued jobs are FIFO.
- Fairness: ready devices are round-robin; `max_concurrent_jobs` bounds the
  JS jobs executing at once.
- Backpressure: per-device and global queue caps; `submit` returns
  `Backpressure` instead of growing without bound.
- Lifecycle states: `Pending`, `Running`, `Waiting` (host op outstanding),
  `Completed`, `Failed`, `Cancelled`. `ConverterRuntime::state(id)` exposes
  the live state. Job and host timeouts, cancellation and shutdown are
  implemented; late responses are rejected.
- Metrics per job: queue wait, eval/start, JS converter time, Promise
  scheduling, host wait, total; plus GC collections/max pause/live heap and
  runtime-wide late responses/host timeouts.

Public API: `RuntimeConfig`, `ConverterRuntime::{submit, wait, cancel, respond,
metrics, host_receiver, shutdown}`, `JobOutcome`, `JobMetrics`, `HostResult`.

## 2. Why the earlier async prototype showed 0.52–0.77 s

The earlier `async_probe` mixed three costs in one number:

1. **cold `eval`/definition load** — the first configure of a device file
   costs ~240 ms x86 (router ~2 s) because the device module is parsed and
   evaluated; this is outside the converter callback but inside the prototype's
   wall measurement;
2. **host-boundary wall time** — 51 sequential host operations; with the study
   host this is a few ms, but concurrent jobs inflate it because the JS thread
   and host pump are shared;
3. **GC** — quickjs-oxide reported 3 collections and a max pause of ~13 ms x86
   for configure; the full-corpus router run reported max pause ~82 ms.

In the runtime prototype, `evalMs` is separated. Warm configure (device module
already loaded) is **23.1 ms total**, of which 20.7 ms is the converter
callback, 3.2 ms host wait, 1.2 ms eval/start, with 3 GCs and 12.6 ms max pause.
With 8 concurrent jobs and the same warm state, total rises to 39.4 ms (host
wait 19.3 ms, JS 36.6 ms). The 0.5–0.8 s figure therefore is **cold load plus
contention**, not a 500 ms scheduler lock.

Batching all pending capabilities at once was not the cause of the converter
cost; it was replaced by actual response-driven settlement. The per-job Promise
cost is visible as `promiseSchedMs` (21.5 ms for configure) and is included in
the JS callback wall.

Settlement order is not a dependency: every response is matched by correlation
ID, and a stale ID is counted as `lateResponses` and ignored. This was
exercised by the late-response test; responses for different jobs may be
delivered in any order.

## 3. Golden parity

`runtime_probe --mode all` submits all 36 golden scenarios with distinct
scheduler device keys, `max-concurrent 4`, and a test responder that answers
every host operation:

- Routerich: **36/36 Completed**, 0 failed, 0 late responses, **exact
  result/published/host-call parity** with
  `tools/bench/corpus/golden.json`.
- x86: same 36/36 parity.
- The earlier 51/54 and 0/6 call differences were fixed by (a) per-job call
  logs instead of one shared JS array and (b) recording `addCustomCluster` on
  the job state.

The runtime preserves sequential awaits inside `configure`, `convertSet`,
`convertGet` and `onEvent`: the converter is a single async JS function; a host
response resolves exactly one pending correlation and the job resumes in its
original order.

## 4. Scheduler reference: Node concurrent workloads

Added to `tools/bench/stress.mjs`:

- `scheduler-configure-c1` / `-c4` (no host delay);
- `scheduler-configure-c1-delay1` / `-c4-delay1` (1 ms mock host delay; this is
  the comparable async-host model).

The Node runner warms the configure/set definitions before measuring, exactly
like the Rust runtime's warmup. Single-call Node numbers from the old baseline
are **not** used for this comparison.

### x86 (runtime probe warm, 1 ms host delay, 8 set jobs)

| Workload | Node configure | Rust runtime | Ratio |
|---|---:|---:|---:|
| c1 | 57.4 ms | 79.9 ms | 1.39× |
| c4 | 56.4 ms | 89.1 ms | 1.58× |

### Routerich AX3000 (same host-delay model)

| Workload | Node configure | Rust runtime | Ratio |
|---|---:|---:|---:|
| c1 | 255.4 ms | 106.8 ms | 0.42× |
| c4 | 112.2 ms | 140.1 ms | 1.25× |

With zero host delay Node's mock promises settle in JS microtasks while the
runtime crosses the Rust boundary; those runs are retained for diagnosis
(Node c1/c4 50.3/25.2 ms, runtime 34.7/67.6 ms on Routerich) but are not the
acceptance comparison. The comparable async-host path is within 2× Node on
both targets; Routerich c4 is 1.25×.

## 5. Lifecycle, backpressure and failure tests

| Test | Result |
|---|---|
| same device, 4 jobs, c4 | serialized; queue waits 0 / 5.7 / 13.6 / 20.6 ms; all Completed |
| queue cap 2, 10 same-device jobs | 3 accepted, 7 `Backpressure` |
| host error | job Failed (`host EIO`), later script works |
| host drop + 100 ms host timeout | job Failed (`host operation timeout`), 5 host timeouts observed |
| late response (200 ms) after 50 ms host timeout | job Failed; `lateResponses=1`; response ignored |
| cancel running configure | `Cancelled`; pending map cleared; subsequent work unaffected |
| shutdown | cancels queued/running jobs, joins worker, drops runtime |

## 6. GC, heap and shutdown

- 16 MB heap cap remains catchable OOM (engine study, Routerich: ~336 ms).
- Runtime default heap cap is 128 MB; job metrics expose qjs `GcStats`
  (`collections`, `max_pause`, `live_bytes`).
- Configure (warm): 3 GCs, max pause 12.6 ms x86 / 16.2 ms Routerich, live heap
  ~5.6 MB.
- Full 36-scenario Routerich run: max pause 82–103 ms, live heap ~21 MB.
- Worker shutdown is explicit; job outcomes are notified before the JS context
  is dropped. Pending responses after shutdown are impossible by channel
  closure.
- Worker startup (package index + bootstrap) measured **2.39 s** on Routerich
  and ~5.1 s on the loaded x86 container; it is a cold path and must stay off
  the bridge request path.

## 7. Reproducible build and pin

`Cargo.lock` pins `quickjs-oxide 0.5.0`, `quickjs-oxide-core 0.5.0`,
`quickjs-oxide-host 0.5.0` and `quickjs-oxide-regexp 0.5.0`. The crate is pure
Rust with no C artifact.

A reproducible static aarch64-musl build was produced with the Rust
`rust-lld` linker (the container has no system C compiler; Zig is only the
build-script C compiler):

```bash
RUSTLLD=$(ls ~/.rustup/toolchains/*/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld | head -1)
PATH=/tmp:$PATH \
  CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=/tmp/cc \
  CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=$RUSTLLD \
  cargo build --release --target aarch64-unknown-linux-musl \
    -p converter-runtime --bin runtime_probe
```

The router binary is statically linked; the Node service was not stopped and
no Zigbee operation was performed. The probe reads `/opt/zigbee2mqtt` only.

## 8. Limits and next milestone

Not yet production:

- `submit` currently waits for a worker ack (bounded); the bridge needs a
  non-blocking submission/ack protocol or an explicit bounded async API.
- Timers are still the study stub (`.unref()` objects, no callback execution).
  A real timer queue is a mandatory next milestone.
- Host response payloads are converted only for primitive JSON in this
  prototype; object/array responses and read values need the full JS value
  bridge.
- No supervision process/RSS watchdog is integrated (the engine heap cap is
  not an RSS cap).
- Definition/device modules are loaded lazily inside the worker; cold eval is
  visible in `evalMs` and must not run on the bridge request path.
- Per-device fairness is validated at the scheduler level; the production
  bridge still needs backpressure integration and shutdown draining.
- Under the full 36-scenario corpus at `max-concurrent 4`, the Danfoss
  configure measures ~675 ms because its host waits interleave with many other
  heavy jobs on one JS thread (host wait ~624 ms). This is a scheduler policy
  finding: production should bound concurrency or prioritise configure work.
  The comparable 1-configure + 8-set workloads above are the reference.

**Decision:** GO for the next-stage production scheduler/timer milestone. The
runtime is demonstrably correct on the golden corpus and meets the comparable
async-host target (≤2× Node); NO-GO for deployment until timers, non-blocking
submission, full host value bridge, RSS supervision and bridge integration are
implemented.

## 9. Sources and raw results

- Runtime: `crates/converter-runtime/src/lib.rs`,
  `src/bin/runtime_probe.rs`, `assets/bootstrap.js`, `assets/zcl.json`.
- Cargo pin: `crates/converter-runtime/Cargo.toml`, workspace `Cargo.lock`.
- Node scheduler workloads: `tools/bench/stress.mjs`.
- Raw results: `tools/bench/results/scheduler/` (Node x86/router, runtime
  x86/router, all-scenario parity, GC/tail).
- Engine study: `docs/converter-engine-study.md`; async profiling:
  `docs/converter-async-host-validation.md`.

Reproduction (from the repository root, with the package paths above):

```bash
cargo run -p converter-runtime --bin runtime_probe -- --mode all \
  --max-concurrent 4 --timeout-ms 60000
cargo run -p converter-runtime --bin runtime_probe -- --mode mix \
  --jobs 8 --max-concurrent 4 --host-delay-ms 1 --timeout-ms 30000
```

The probe prints per-job metrics and state JSON on stdout; use `--host-mode
drop|error|late`, `--queue-cap`, `--total-cap`, `--cancel-after-ms` for the
failure tests.
