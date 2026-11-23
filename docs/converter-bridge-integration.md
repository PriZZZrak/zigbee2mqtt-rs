# Converter runtime ↔ Rust bridge (offline integration)

Status: implemented for the offline stage; production host transport and HIL are
the next milestone.
Runtime: `crates/converter-runtime`; bridge adapter:
`crates/z2m-rs/src/converter/`; tests:
`crates/z2m-rs/tests/converter_bridge.rs`; probe:
`crates/z2m-rs/src/bin/converter_bridge_probe.rs`.

This stage executes the pinned upstream converter JavaScript through the real
`converter-runtime` worker and the real Rust controller model. Zigbee, MQTT and
Home Assistant transports stay offline: no coordinator, broker or HA API is
contacted.

## 1. Architecture

```text
Controller / Zigbee / Device / DefinitionRegistry   (existing Rust model)
        │  ordinary zigbee_converters::Definition
        ▼
Receive / Publish / Configure / OnEvent             (existing extensions)
        │  definition closures
        ▼
ConverterDispatcher (z2m-rs)
  ├─ oneshot per job, resolved by the async JobEvent channel
  ├─ bounded runtime admission and per-device ordering
  └─ host-request loop → ConverterTransport
                              ├─ OfflineMockTransport (this stage)
                              └─ Endpoint/ZCL transport (next stage)
        │
        ▼
converter-runtime worker (quickjs-oxide)
```

- One JS worker per dispatcher; built-in and external definitions use the same
  adapter and the same execution path. The source only selects definition
  lookup (`package` or `external`).
- `prepare()` evaluates a definition in JS and returns ordinary
  `zigbee_converters::Definition` metadata plus adapter closures. The existing
  `DefinitionRegistry` stores it with `register_shared`; `Receive`, `Publish`,
  `Configure` and `OnEvent` are unchanged and keep upstream error/publish
  semantics.
- `Zigbee::set_converter_dispatcher()` installs the dispatcher;
  `Controller::start()` calls `prepare_converter_definitions()` after the
  coordinator starts and before the device list/extensions are built.
  `Controller::stop()` performs a bounded dispatcher shutdown.
- Without a dispatcher the bridge behaves exactly as R1 (no JS, empty
  registry), so existing Node/Rust golden behaviour is preserved.

## 2. Definition adapter

`prepare` runs with `kind=prepare` and `__metadata=true` and returns model,
vendor, description, zigbeeModel, endpoint map, version, `fromZigbee`
cluster/type lists, `toZigbee` keys, and whether `configure`/`onEvent` exist.
The adapter builds:

- one `FromZigbeeConverter` per cluster/type entry;
- one `ToZigbeeConverter` per key list, with `convertSet`/`convertGet`;
- `Definition.configure` when present;
- one `onEvent` handler when present.

Each closure serializes a normal runtime scenario from the existing
`ConvertData`/`ToMeta`/`OnEventEvent`, awaits the dispatcher future, and maps the
JS result back into `Option<Value>`, `ToZigbeeResult`, or `Result<(), String>`.
Published payloads from `toZigbee` are delivered through the existing
`meta.publish` callback.

The device endpoints/clusters in the scenario come from the real
`zhDevice`/`zhEndpoint` model, not from a separate device mock.

## 3. Host transport and ZCL binding

`ConverterTransport` is the only physical-operation boundary. The offline
implementation:

- resolves the operation (`read`, `write`, `command`, `commandResponse`,
  `bind`, `configureReporting`) from `HostRequest.operation`;
- validates known clusters/attributes/commands with the existing
  `zigbee_apiarist::zspec::zcl::utils` catalog;
- returns deterministic read values keyed by device/cluster/attribute;
- records every operation for assertions;
- never sends a ZNP frame.

The Rust ZCL catalog is intentionally partial. The offline transport therefore
treats clusters absent from the catalog as opaque-but-recorded instead of
rejecting real converter traffic. The production transport will call the
existing `Endpoint::read/write/command/bind/configure_reporting` methods, which
already encode/decode through the same ZCL layer and then reach the adapter.

`HostResult` delivery goes through `respond()`:
`Delivered`, `Backpressure` or `Closed`. On backpressure the completed result is
retained and retried with the same correlation ID; the physical operation is
never executed twice (verified by the delayed-read test, which asserts one
transport execution).

## 4. Concurrency and lifecycle

- `ConverterRuntime::take_events()` exposes `JobEvent::{Started,Finished}`.
  The dispatcher forwards events with a dedicated thread into a Tokio channel
  and resolves per-job `oneshot`s. No bridge task calls `wait()`.
- Admission backpressure is returned explicitly (`converter runtime
  backpressure`); the Tokio worker is never blocked.
- `cancel_device()` is non-blocking and uses the runtime's shared device-cancel
  set, which the worker drains even when the command queue is full. Pending
  requests receive the Cancelled outcome.
- Job and host timeouts are bounded. A late `HostResult` after a timeout is
  ignored by correlation ID and counted (`hostTimeouts`, `lateResponses`).
- `restart()` aborts forwarding tasks, fails pending requests explicitly,
  shuts down the old worker with a deadline and creates a fresh realm with a new
  generation. A timed-out stop returns `TimedOut` and creates no second worker.
- `shutdown()` fails pending requests, stops the worker and aborts the
  forwarding tasks; the bridge stop path is bounded.
- Timers are owned by job + device (runtime unit tests), so device leave removes
  onEvent timers as well as queued/running jobs.

## 5. Tests

`crates/z2m-rs/tests/converter_bridge.rs` (9 Tokio tests, fake package, no
ignored):

1. mixed reports and sets through the real Receive/Publish extensions;
2. configure through the real Configure extension;
3. sequential configure per device (overlap detector);
4. per-device order with four concurrent devices;
5. restart fails pending work, bumps generation and serves new jobs;
6. bridge stop and device leave are bounded/non-blocking;
7. admission backpressure surfaces explicitly;
8. delayed read → host timeout + late response + single execution;
9. production Zigbee prepare hook registers definitions.

Runtime unit/regression tests additionally cover the event channel, event
backpressure, device cancellation after shutdown, timer ownership and the
existing 36-scenario golden corpus.

## 6. Measurements

Probe:
`cargo run --release -p z2m-rs --features converter-probe --bin converter_bridge_probe -- --mode report --iterations 100 --concurrent 1`
(raw results in `tools/bench/results/scheduler/bridge/`).

`dispatchOverhead` = outer submit→event→oneshot time minus the runtime's
`JobMetrics.total_ms`; `jsExec` is the runtime JS execution metric. All values
are milliseconds.

### Routerich AX3000 (aarch64, Node service left running, nice 19)

| Workload | outer p50 / p95 | worker total p50 | JS p50 | dispatch overhead p50 / p95 |
|---|---:|---:|---:|---:|
| report c1 | 2.29 / 4.48 | 1.69 | 0.16 | 0.42 / 1.76 |
| set c1 | 3.81 / 41.03 | 3.23 | 1.28 | 0.52 / 3.65 |
| configure c1 | 4.51 / 13.84 | 3.78 | 1.89 | 0.37 / 2.43 |
| report c4 | 9.74 / 29.27 | 5.34 | 0.16 | 4.51 / 18.69 |

### x86 release/debug probe (same corpus/package)

| Workload | outer p50 / p95 | JS p50 | dispatch overhead p50 / p95 |
|---|---:|---:|---:|
| report c1 | 1.62 / 2.11 | 0.15 | 0.16 / 0.23 |
| set c1 | 2.21 / 3.19 | 0.72 | 0.18 / 0.23 |
| configure c1 | 2.38 / 3.03 | 0.95 | 0.15 / 0.21 |
| report c4 | 5.92 / 7.95 | 0.14 | 2.61 / 5.10 |

Single-device bridge dispatch overhead is well below 1 ms p50 on Routerich;
under four concurrent devices the event-loop/oneshot path contributes ~4.5 ms
p50 (18.7 ms p95) while JS remains ~0.2 ms. The 36-scenario golden parity still
passes (`bad 0`, 36/36) with the new bootstrap flags.

## 7. Limits of this stage

- Offline host transport only; the real `Endpoint`/ZNP transport is not wired.
- Group `toZigbee` execution returns an explicit error; groups remain on the
  native path.
- JS `exposes`/HA discovery metadata is not converted yet; per-device HA
  discovery is still pending.
- Definition lookup relies on the upstream `models-index.json` key
  (`modelID`/`zigbeeModel`). Devices that report a model absent from the index
  remain definition-less until the full Rust index/fallback is ported.
- The shared realm keeps the documented runtime-wide interrupt/timeout policy.
- No real Zigbee, MQTT or HA operation is exercised; no HIL claim is made.
- The static `z2m-rs` binary now links the pure-Rust JS runtime; OpenWrt
  package-size measurement and optional feature gating for R1 builds remain
  packaging work. The probe binary is opt-in via `--features converter-probe`.

## 8. GO / NO-GO

**GO** for the next stage: replace `OfflineMockTransport` with a transport that
calls the existing `Endpoint` ZCL/ZDO methods, then run device-level HIL.

**NO-GO for production enablement** until:

1. the real host transport is implemented and HIL-verified;
2. JS exposes/HA discovery and group converters are handled;
3. model-index fallback/fingerprint resolution matches upstream;
4. OS supervision and the documented shared-timeout policy are accepted;
5. MQTT/HA/frontend contracts are verified with the JS definitions enabled.
