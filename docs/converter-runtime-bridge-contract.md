# Converter runtime <-> Rust bridge integration contract

Status: proposed contract for the offline bridge integration milestone.
Runtime implementation: `crates/converter-runtime`; scheduler report:
[`converter-runtime-scheduler-prototype.md`](converter-runtime-scheduler-prototype.md).

This contract does not wire Zigbee, MQTT or HA. It defines how the bridge will
use the autonomous runtime without blocking its Tokio event loop.

## 1. Ownership

- The bridge owns one `ConverterRuntime` per converter realm (built-in and
  external are separate realms in production; the prototype shares one).
- The runtime owns the single quickjs-oxide worker thread, its JS heap, timer
  queue and host transport queues.
- The bridge owns the Zigbee/MQTT side and is the only component that executes
  physical operations.

## 2. Submission and results

```text
bridge ── try_submit(JobRequest{device, scenario}) ──▶ runtime
       ◀── JobId / Backpressure / Closed             (non-blocking)

bridge host transport ◀── HostRequest{correlation, job, payload} ── runtime
       ── respond(correlation, HostResult) ──▶ runtime
```

- `submit` is non-blocking and returns `JobId` immediately or
  `SubmitError::{Backpressure,Closed}`. It must never be called from a Tokio
  task while holding a lock that the worker needs.
- Results are delivered asynchronously. The current prototype exposes
  `wait(id, timeout)` for tests; the production bridge contract is an
  **event channel** (`JobEvent::{Started,WaitingHost,Completed,Failed,Cancelled}`)
  or a callback, never a blocking `wait` on the Tokio reactor.
- A job result contains normalized `result`, `published`, host calls and
  metrics. The bridge must not infer completion from MQTT publishes alone.

## 3. Backpressure

- Admission is bounded per device and globally; backpressure is a normal
  outcome, not an error to retry in a busy loop.
- On `Backpressure`, the bridge keeps the request in its own bounded work
  queue and retries after a scheduler event, or returns an explicit
  MQTT/bridge error. It must not spawn an unbounded retry task.
- Host-request and response queues are bounded. A host operation that cannot
  be admitted fails with an explicit host error; the bridge must not silently
  drop the physical operation.
- Duplicate/late responses are ignored by correlation ID and counted; a response
  after a job's terminal state is not an error.

## 4. Device lifecycle

- `device` is the ordering key. At most one JS job per device is active;
  queued work is FIFO. Different devices are scheduled round-robin.
- On device removal: cancel queued/running jobs, clear job-tagged timers, wait
  for terminal outcomes, then remove the device from the bridge index.
- On rename: the device key and queued jobs must be updated atomically by the
  bridge; in-flight jobs retain their original key until terminal.
- On external-converter reload: use a separate external realm or
  `restart()` with the new package; do not mutate a live realm while jobs are
  active.

## 5. Host transport

- Every operation has a correlation ID and a bounded payload
  (`max_host_payload_bytes`).
- The bridge resolves the Promise only when the real ZNP/device response
  arrives; timeouts, cancellations and transport errors are delivered as
  rejected Promises.
- Timeout policy: job timeout bounds total work; host timeout bounds one
  operation. The bridge must avoid reporting a timeout as a successful
  command.
- The shared JS interrupt is runtime-wide: a job timeout or synchronous
  cancellation can abort other active jobs on the same realm. The bridge must
  surface this as a scheduler event and recreate the realm if needed.

## 6. Shutdown and recovery

- `shutdown()` is bounded: it clears timers/queues, interrupts the worker,
  joins it and stops the supervisor. The bridge must have an outer timeout and
  must not block its event loop.
- `health()` reports `Running`, `RssExceeded`, `Unresponsive`, `Stopped`.
  `rss_bytes()` is process RSS; JS heap is reported separately in job metrics.
- `restart()` recreates worker/supervisor and transport channels. The bridge
  must reacquire `host_receiver()` and re-register host transport wiring.
- An OS service supervisor remains required for process death/panic; in-process
  restart is not a substitute.

## 7. Metrics / observability

Per job: queue wait, eval, JS execution, Promise scheduling, host wait, total,
host calls, GC collections/max pause/live heap. Runtime: submitted/completed/
failed/cancelled, late responses, host timeouts, request/response backpressure,
payload rejections, RSS.

These are the bridge's scheduler SLO inputs. The single-call Node baseline is
not a valid comparison for concurrent bridge workloads; use the Node scheduler
workloads in `tools/bench/stress.mjs`.

## 8. Non-goals of this contract

- Zigbee/MQTT/HA command semantics.
- The full device catalog or per-device HA discovery.
- Production timers beyond the upstream-needed API surface.
