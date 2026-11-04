# CV-06/07 — dedicated converter actor with persistent Boa sessions

Rust implementation: `src/converters/external.rs`
(`ConverterJob`, `ConverterSession`, `runtime_sender`,
`convert_from_zigbee_session`, `TIMERS_JS`), `src/bridge.rs` dispatch.
Behavior: CONVERTER-018; plan CV-06/07; ADR-08 option 1.

## Architecture

`boa_engine::Context` is not `Send` (it owns `Rc<dyn JobQueue>` and
`Rc<dyn ModuleLoader>`). The bridge shares `Arc<ExternalConverter>` and runs on
the default multi-thread Tokio runtime, so contexts cannot live in the shared
struct or in a thread-local map.

Implemented option 1:

- a dedicated `z2m-converters` thread owns all Boa contexts and module caches;
- jobs cross the boundary through `std::sync::mpsc` and carry owned inputs;
- one session is cached per `(IEEE address, model)`; sessions are created lazily
  and reused for every frame from the same device;
- if the thread cannot start or is gone, conversion falls back to the previous
  per-call context (no silent device loss).

## Session semantics

- The CJS bootstrap/module loader is evaluated once per session, so module-level
  variables persist across frames.
- A stable `globalThis.__device` object is reused as `meta.device` and
  `msg.endpoint.getDevice()`; `Object.assign` refreshes its fields each frame.
  This makes upstream `globalStore` Maps keyed by the device object persist.
- Timer host functions are defined in JS (`setTimeout`, `clearTimeout`,
  `__runDueTimers`). Timers are stored on `globalThis` so Boa GC traces them.
  Due timers are executed after a frame conversion.
- The bridge's message dispatch now uses `convert_from_zigbee_session`.

## Tests and evidence

- `session_keeps_module_state_and_runs_due_timers`: first frame returns
  `frames=1, timerFired=0`; the `setTimeout(...,0)` callback runs after the frame;
  second frame returns `frames=2, timerFired=1`.
- `session_keeps_device_keyed_module_store`: a JS `Map` keyed by `meta.device`
  returns 1, 2, 3 over three frames.
- `async_from_zigbee_promises_are_awaited_across_frames`: an async converter
  returns 1 then 2 only if its Promise is awaited.
- `future_timer_is_pumped_and_resolves_async_converter`: an async converter awaits
  `setTimeout(...,1000)`; the fake clock pumps the timer and frames return 1, 2.
- Full suite: `cargo test --locked` → **341 passed** (309 library, 2 binary,
  30 compatibility); clippy exit 0 with five existing warnings; release build OK.

## Divergences and gaps (not verified)

- Timers are not wall-clock scheduled: the fake clock advances immediately to the
  next due timer during dispatch (max 60 s). Long or repeating timers beyond that
  window are not serviced between frames.
- Timer callbacks cannot emit `publish` results to Rust; callback output is
  discarded.
- Async converters that wait on network/external I/O still cannot progress.
- Sessions are never evicted; memory grows with the number of
  devices × definitions. Per-device memory on the router is unmeasured.
- Actor thread panic/shutdown is only observed as a closed channel; in-flight
  jobs have a 10 s receive timeout and then return `None`.
- No real published package, Node differential test or HIL; fixture only.
