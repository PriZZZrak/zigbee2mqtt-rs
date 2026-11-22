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
| c1 | 57.4 ms | 91.2 ms | 1.59× |
| c4 | 56.4 ms | 109.0 ms | 1.93× |

### Routerich AX3000 (same host-delay model)

| Workload | Node configure | Rust runtime | Ratio |
|---|---:|---:|---:|
| c1 | 255.4 ms | 131.8 ms | 0.52× |
| c4 | 112.2 ms | 139.4 ms | 1.24× |

With zero host delay Node's mock promises settle in JS microtasks while the
runtime crosses the Rust boundary; those runs are retained for diagnosis
(Node c1/c4 50.3/25.2 ms, runtime 34.7/67.6 ms on Routerich) but are not the
acceptance comparison. The comparable async-host path is within 2× Node on
both targets; Routerich c4 is 1.24× on the median of three runs (individual
runs ranged 127–195 ms; c1 median 131.8 ms, range 95–195 ms; best observed
95 ms c1 / 111 ms c4). Variance comes from the live Node service sharing the
router; the median is the reported figure.

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

## 7a. Hardening round (2026-11-22)

The audit findings were addressed in the same crate:

- **P0 timeout slot leak fixed.** All terminal paths (`success`, `failure`,
  `timeout`, `cancel`, `shutdown`) go through one `finish_job`: it removes the
  active-device slot, pending host capabilities, generation guard, admission
  credit and live state, then publishes the outcome. Regression: a configure
  whose host operations time out is followed by a successful report on the
  same device (`timeout-reuse`).
- **P0 non-blocking admission.** The bridge-facing command channel is a
  bounded `sync_channel` and `submit` uses `try_send`; it never waits for a
  worker acknowledgement. A mutexed admission counter enforces per-device and
  global caps before enqueue; full/disconnected returns `Backpressure`/`Closed`
  immediately. Host responses use a separate queue, and a wake command keeps
  the worker responsive.
- **P1 cancellation safety.** Each start gets a generation token. `__hostOp`,
  `__runtimeDone` and `__runtimeFailed` carry/check the token against the
  active job map; a cancelled/stale JS continuation cannot send new host
  operations or publish a result. Already-issued physical operations are not
  recalled; their late responses are counted and ignored. `cancel-reuse`
  cancels a running configure and immediately completes a report on the same
  device.
- **P0 non-blocking response delivery.** `respond()` now returns
  `RespondStatus::{Delivered,Backpressure,Closed}` and uses `try_send`; a full
  response queue never blocks the caller. On backpressure the bridge retains
  the already-completed physical result and retries the same correlation ID;
  the pending job remains waiting until it arrives or times out. Duplicate and
  late responses are counted/ignored.
- **P0 bounded shutdown.** `shutdown_with_timeout()` places the `JoinHandle`
  in a helper thread and waits only for a configured deadline; it returns
  `ShutdownStatus::{Stopped,TimedOut,AlreadyStopped}`. A timed-out worker is
  explicitly detached and marked `Unresponsive`; the method does **not** claim
  it was stopped. OS/service supervision remains the final boundary. `Drop`
  uses a short bounded wait, and `restart()` stops the old worker before
  creating the new one; a timed-out restart returns `TimedOut` and does not
  create a second runtime.
- **P1 admission race.** The admitted id is registered **before** the Submit
  command is sent; a failed send removes it and releases admission. A worker
  that finishes immediately therefore removes an existing entry rather than
  leaving a stale one.
- **P1 file sandbox.** `__readFile`/`__fileExists` canonicalize paths and
  allow only the package root, herdsman root and the external converter's
  directory/exact file. `..`, absolute escapes and symlinks resolving outside
  the roots are rejected; tests cover all three.
- **P1 timer ownership.** Timer entries carry job id and device key
  (`__deviceKey` injected by the scheduler). Job cancellation removes its
  timers; `cancel_device()` removes queued/running jobs and all device timers;
  `restart()`/shutdown clear the queue. Timer callbacks that outlive a job can
  publish JS state but cannot issue new host operations (generation guard).
  Callback errors are logged and do not kill the worker.
- **P1 host value bridge.** Recursive JSON→JS conversion now builds real
  arrays/objects and uses `Int`/`Float`/`BigInt` without truncating i64/u64
  values; unit tests cover 2^32, 2^53+1 and `u64::MAX`. No array/object becomes
  `undefined`.
- **Timeouts under load.** The worker drains bounded command/response batches,
  then calls `check_timeouts()` on every turn before starting more work; a
  continuous command stream cannot starve timeouts.
- **Long JS and memory.** A single watchdog thread arms the qjs interrupt at
  the earliest job deadline. An infinite JS loop is cancelled after the job
  timeout (`JS execution timeout`) and the same device can start a new job;
  a shared-JS timeout cancels all active jobs (one interpreter, documented).
  Metrics/outcomes are retention-capped; states are removed on finish.
- **Timers:** `setTimeout`/`clearTimeout`/`setInterval`/`clearInterval`,
  `.ref()`, `.unref()`, `.hasRef()`, `.close()` are backed by a Rust timer
  queue. Callbacks run on the worker and may outlive the converter call; a
  pending 10 s timer does not block shutdown. Entries carry both the creating
  job and the device key injected by the scheduler, so cancelling a job removes
  its timers and `cancel_device()` removes every timer owned by a removed
  device, including `onEvent` timers that outlived their call. Restart/shutdown
  clears the whole queue; a recreated realm starts with an empty timer set.
- **Bounded host transport:** host-request and response queues are bounded
  (`max_pending_host_ops`, `max_pending_responses`), payload size
  (`max_host_payload_bytes`) is enforced, and duplicate/late correlation IDs
  are counted and ignored.
- **Cancellation contract:** `cancel_request` returns `Accepted`,
  `AlreadyTerminal`, `Unknown` or `Overloaded` separately from the terminal
  state. Admission-tracked jobs can be cancelled before the worker has
  created their live state. A running synchronous JS slice triggers the shared
  interrupt; queued/waiting jobs use the normal loop.
- **Supervision:** a supervisor thread samples process RSS and worker
  heartbeat, exposes `health()`/`rss_bytes()`, and interrupts the worker on
  RSS/heartbeat violation. `restart()` recreates worker and supervisor with
  the same config; the bridge must reacquire `host_receiver()`.
- **Tests:** four unit tests (number bridge, object/array bridge, bounded
  admission, metric retention), two ignored integration regressions
  (`timeout_then_same_device_job_completes`,
  `cancel_then_same_device_job_completes`), twenty autonomous
  timer/lifecycle tests in `tests/timers.rs` and one offline bridge transport
  test in `tests/bridge_offline.rs` (fake package, no ignored) pass; the
  JS-loop watchdog is exercised by `runtime_probe --mode js-timeout`.
- **Soak (Routerich, 32 MB JS heap cap):** 60 s mixed reports/sets with host
  drops/errors/timeouts and cancellations: **4,083 completed, 508 failed,
  1,523 cancelled, 329 host timeouts, 0 late responses**, process RSS
  **42.4 MB** before shutdown (peak 54.2 MB), live JS heap 20.4 MB (peak
  23.6 MB), max GC pause 132 ms, no OOM or worker death. A 5 s x86 soak
  completed 83 jobs (26 failed, 37 cancelled) with 66 MB RSS before shutdown.

Raw hardening results are in
`tools/bench/results/scheduler/hardening/` (x86) and
`tools/bench/results/scheduler/runtime-router/*-hardened.json` plus
`soak-32mb-60s.json`.

### RSS vs JS heap measurement

The earlier "12.3 MB RSS / 23.6 MB heap" pair was not measured at the same
point: the RSS value was read **after** `runtime.shutdown()` released the
worker, while the heap value was the maximum across completed jobs. The
hardening probe now reports:

- `rssBeforeShutdownKb` — process `VmRSS` sampled immediately before shutdown,
  while the worker and its heap are alive;
- `rssPeakKb` — `VmHWM` high-water mark;
- `heapLiveBytes` — qjs live JS heap from the last completed job;
- `heapLivePeakBytes` — maximum live heap across jobs;
- `rssKb` — post-shutdown RSS, retained only for diagnostics and never compared
  with heap.

The consistent Routerich sample is **42.4 MB process RSS / 20.4 MB JS live
heap** before shutdown (peak 54.2 MB RSS / 23.6 MB heap). RSS includes the
binary, loader, Zcl snapshot and native allocator; heap is JS-only, so RSS
must be larger. The same methodology applies to x86 runs.

### Restart memory strategy

`restart()` is **sequential**: it performs a bounded shutdown of the old worker
before creating the new runtime, so old and new JS heaps never coexist. On
Routerich a restart that includes package load measured **15.8 MB RSS before
and after**, peak **17.2 MB**, and the next timer job completed. On x86 the
same probe measured 20.6 → 30.2 MB RSS, peak 30.2 MB. The cost is restart
latency (Routerich ~2.4–2.7 s including package/bootstrap); the bridge must perform
reload off the request path. A timed-out old worker is detached, not overlapped
with a new one, and must be contained by the OS supervisor.

Probe: `crates/converter-runtime/src/bin/restart_probe.rs`;
raw result is part of the scheduler results set.

## 8. Limits and next milestone

Not yet production:

- Timers are runtime-global; job/device attribution uses the best-effort
  `__timerJobId`/`__timerDevice` set when the call starts. A timer created from
  an async continuation inherits the tags of the call that scheduled it; a
  timer created inside another timer callback inherits that callback's tags.
  A timer that outlives its job cannot issue new host operations because the
  generation guard requires an active job; it can publish JS state. Device
  leave/removal removes all timers carrying that device key.
- Timer callbacks run on the shared worker; a runaway timer callback is covered
  by the same shared interrupt policy.
- The JS watchdog uses the one runtime-wide interrupt handle: a single job's
  timeout or synchronous cancellation aborts all active jobs on the shared
  interpreter. Production must accept the documented shared-timeout policy or
  isolate realms.
- A shutdown that times out detaches the worker thread; it is not force-killed
  in-process. The OS service supervisor must terminate the process and restart
  the runtime.
- Supervision is in-process (RSS + heartbeat + restart); a crash that kills
  the whole process still needs an OS service supervisor.
- `restart()` replaces the transport channels; bridge code must reacquire
  `host_receiver()` and re-deliver its host transport wiring.
- Definition/device modules are loaded lazily inside the worker; cold eval is
  visible in `evalMs` and must not run on the bridge request path.
- Per-device fairness is validated at the scheduler level; the production
  bridge still needs backpressure integration and shutdown draining.
- Under the full 36-scenario corpus at `max-concurrent 4`, the Danfoss
  configure is queued behind many independent devices (queue wait ~4.7 s,
  total ~5.3 s; host wait ~535 ms) because it is submitted later in the batch.
  This is expected fair round-robin behaviour, not a slot leak. Production
  should bound concurrency, prioritise configure work, or use per-device
  queues with request deadlines. The comparable 1-configure + 8-set workloads
  above are the performance reference.

**Decision:** GO for offline bridge integration (contract + harness) and for
the next production-scheduler milestone. The runtime is demonstrably correct
on the golden corpus and meets the comparable async-host target (≤2× Node);
timers, bounded host transport, cancellation contract and in-process
supervision are implemented. NO-GO for deployment/HIL until the production
bridge dispatcher, event-channel delivery, OS-level supervision/restart and
the production device lifecycle contract are implemented.

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
