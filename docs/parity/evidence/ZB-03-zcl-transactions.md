# ZB-03 — ZCL transaction completion

Rust implementation: `src/coordinator/znp/transaction.rs`,
`src/coordinator/znp/mod.rs` (`request_zcl`, event pump),
`src/coordinator/znp/commands.rs` (`AfDataConfirm`, incoming fields). Behavior:
ADAPTER-005; plan ZB-03.

## Source contract

Pinned herdsman `3cb89ad906bd9b2de8b3dfed20d5a21070a467d2`:

- `adapter/adapter.ts:123–137` (`zclWaitressValidator`): match network address,
  cluster, endpoint, transaction sequence number and command; a global
  `defaultRsp` (0x0B) also matches (optionally validated against the failed
  command id).
- `adapter/z-stack/adapter/zStackAdapter.ts:491–566`: registers the response
  waiter before sending the data request, waits for `dataRequest` confirmation,
  rejects on non-success confirmation/timeout, and retries/recovery follow.
- `znp/definition.ts:978–986`: `AF_DATA_CONFIRM` = status + endpoint + APS
  transaction id (no destination address). `incomingMsg` = cluster, source
  address/endpoint, destination endpoint, link quality, APS sequence and data.
- Latest herdsman also matches frame type (`75e47fe`, DRIFT-001).

## Rust change

- Added `ZclIncoming` parsing from `AF_INCOMING_MSG`: ZCL frame type, direction,
  TSN, command, plus network address, source endpoint and cluster.
- Added `ZclRequest` derived from the outgoing payload (TSN, command, frame type,
  expected response for read/write/configure-reporting, default-response flag).
- Added `ZclWaiters`: registers one-shot waiters, delivers only to matching
  address/endpoint/cluster/TSN/direction/command or accepted default response,
  fails on `AF_DATA_CONFIRM` status != 0, expires overdue waiters, and supports
  explicit removal.
- `ZNP` transport now emits `AfDataConfirm` (AF 0x80) and the event pump feeds
  Confirm/Incoming frames to the shared registry before forwarding device events.
- `ZnpHandle::request_zcl` / `CoordinatorHandle::request_zcl` register before
  sending, distinguish AF request acceptance, confirmation and device response,
  and retry a timed-out attempt up to `attempts` with the same TSN.
- A successful AF confirmation deliberately does **not** complete the command.

## Tests and evidence

`cargo test --locked` → **301 passed** (269 library, 2 binary, 30 compat).
`cargo clippy --all-targets --locked` → exit 0, 5 existing warnings.
`cargo build --release --locked` → success.

Transaction tests (10):

- matching read response resolves; success confirm does not;
- wrong address/endpoint/cluster/TSN/direction ignored;
- default response accepted and its status exposed;
- disabled default response not matched for a no-response command;
- failed AF confirm errors the waiter (status in message);
- expiry fails only overdue waiters;
- AF→ZCL field parsing;
- event-pump integration: `AfIncomingMsg` resolves, `AfDataConfirm` failure
  propagates, success confirm leaves the waiter pending.

## Divergences and gaps (not verified)

- No hardware/ZNP stream; `request_zcl`'s flow is tested with a fake sender
  (`request_flow_tests`), not against real serial I/O or timing.
- `AF_DATA_CONFIRM` carries no destination address, so confirmation correlates by
  TSN only; a late confirm from a previous retry can affect a new attempt with the
  same TSN. Upstream has the same wire limitation and uses queue/recovery logic.
- Retries are timeout/confirm-triggered only; route discovery, association
  removal/restore, `NWK_NO_ROUTE`/`MAC_NO_ACK` recovery and backoff are not ported.
- Default-response status is exposed but not automatically converted to an error
  (upstream returns the payload; app-level quirks decide).
- Frame-type matching from DRIFT-001 is not yet applied; matching uses the
  direction/command/default-rsp fields available in the pinned baseline.
- MQTT `get` now uses `request_zcl` for native reads; set/configure/reporting and
  upstream `convertGet` still use fire-and-forget or remain unimplemented.
- No HIL evidence; ADAPTER-005 stays `IN_PROGRESS`.
