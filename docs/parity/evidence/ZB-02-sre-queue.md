# ZB-02 — SREQ serialization and SRSP matching

Rust implementation: `src/coordinator/znp/transport.rs`. Behavior: ADAPTER-003;
plan ZB-02.

## Source contract

Pinned herdsman `3cb89ad906bd9b2de8b3dfed20d5a21070a467d2`:

- `src/adapter/z-stack/adapter/zStackAdapter.ts:100–112` detects the firmware and
  creates a `Queue` with `concurrent = 16` for Z-Stack 3.x and `2` for legacy
  1.2 (override via adapter options).
- All request paths (`start`, `sendZclFrameToEndpoint`, `permitJoin`, NV writes)
  execute through `this.queue.execute(...)`; `src/utils/queue.ts` limits running
  jobs and preserves FIFO order.
- `waitFor` matches responses on address/endpoint/cluster/TSN/command; ZNP SREQ
  frames themselves carry no request identifier, only subsystem and command.
- Latest herdsman additionally matches on ZCL frame type (`75e47fe`, DRIFT-001).

Original tests: `test/adapter/z-stack/adapter.test.ts` (waiters, default
response, timeout) and queue tests. They still need to be executed as a reference
oracle for this Rust path.

## Problem

The Rust actor kept a single `pending: Option<PendingRequest>`. A second SREQ
overwrote it, so the first caller waited for its full 10 s timeout and an SRSP
could be delivered to the wrong caller when both requests shared subsystem/cmd1.

## Change

- Incoming SREQ requests are pushed to a FIFO `VecDeque`; exactly one is on the
  wire at a time. The next is sent only after a matching SRSP (or cancellation /
  send failure).
- `Cancel(id)` removes a queued request or clears the in-flight one and advances
  the queue, so a timed-out caller cannot block later operations.
- A cancelled queued caller observes a closed oneshot; the frame is never sent.
- The legacy rule is preserved: an SRSP whose command matches but whose
  subsystem differs (including `0x00` on old firmware) is delivered with a
  warning.
- The actor is generic over the byte stream so tests use an in-memory duplex
  instead of requiring a coordinator.

## Tests added

`src/coordinator/znp/transport.rs`:

- `sreq_requests_are_serialized_and_lose_no_response` — second SREQ stays off the
  wire until the first SRSP; both callers receive their own payload.
- `cancelled_queued_request_is_dropped_and_does_not_block_others` — cancellation
  drops the queued frame and the first request still completes.
- `mismatched_subsystem_response_is_still_delivered` — legacy compatibility.

All three pass on an in-memory duplex.

## Divergence and gaps (not verified)

- Rust serializes **all** SREQ frames (concurrency 1). Upstream permits 2/16
  concurrent jobs. This is a throughput/latency divergence, not proven
  payload-equivalent; it needs measurement on CC2531 and Dongle-P and an ADR
  decision before claiming parity.
- The 10 s client timeout and cancellation are unit-tested; no paused-clock test
  asserts the exact deadline.
- Serial EOF/reopen, PTY fragmentation, queue backpressure under load and real
  coordinator timing remain untested.
- No hardware-in-loop evidence; ADAPTER-003 stays `IN_PROGRESS`.
