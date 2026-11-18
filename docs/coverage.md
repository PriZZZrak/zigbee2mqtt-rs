# Coverage and gates (CP-11)

Reproducible line/function/region coverage for the R1 **software gate**. HIL is a
separate acceptance step and is never implied by these numbers.

## How to reproduce

```bash
PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin:$PATH" \
CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=/tmp/opencode/zigcc-cov \
./tools/coverage.sh
```

`tools/coverage.sh` builds every workspace test binary with
`-Cinstrument-coverage`, runs them with `LLVM_PROFILE_FILE`, merges the profiles
with `llvm-profdata` and reports with `llvm-cov`. The script reports the
dependency packages (`zigbee-apiarist`, `zigbee-converters`) and `z2m-rs`
separately and deduplicates shared source paths; combining instrumented binaries
from different generations in one `llvm-cov` invocation drops newer mappings, so
the split is required for a correct union.

Artifacts (not committed):

- `target/coverage/summary.txt` — llvm-cov per-file report + TOTAL
- `target/coverage/lcov.info` — lcov export
- `target/coverage/summary_deps.txt`, `summary_z2m.txt` — per-pass inputs

## Run metadata

| Item | Value |
|---|---|
| Date | 2026-11-18 |
| Commit | `a4023a5` (`test(fuzz): deterministic UNPI corpus and buffer/codec properties; close CP-10`) |
| Toolchain | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| Coverage tool | rustup `llvm-tools-preview` 1.99.1 (`llvm-cov`, `llvm-profdata`) |
| Linker | `/tmp/opencode/zigcc-cov` (Zig cc wrapper; forwards `-u` to `-Wl,-u`) |
| Included crates | `zigbee-apiarist`, `zigbee-converters`, `z2m-rs` |
| Excluded from report | `tests/`, `examples/`, `benches/`, registry/rustlib, `target/`, external C/asm |
| Excluded from execution | 19 `#[ignore]` tests: 12 HIL ZNP (`hil_znp.rs`, requires `Z2M_SERIAL_PORT`), 1 real-broker test (`mqtt_broker.rs`, requires `MQTT_TEST_SERVER`), 6 upstream stub-file markers (ember/zboss/zigate/buffalo titles) |

## Result

```text
TOTAL  regions 72.96%   functions 74.23% (2552/3438)   lines 72.73% (25502/35066)
```

| Crate | Lines | Functions |
|---|---:|---:|
| `zigbee-apiarist` | 67.85% (13215/19477) | 71.24% |
| `zigbee-converters` | 79.81% (336/421) | 78.18% |
| `z2m-rs` | 78.80% (11948/15163) | 77.37% |

The overall percentage is intentionally **not** the acceptance criterion. CP-06
closed every `code`/`slice` row and CP-07 closed every adapter row with named
tests; coverage is supporting evidence, not a replacement for those scenarios.

## Uncovered critical branches

Low-line files and their classification (from `summary.txt`):

| File | Lines | Why it is acceptable / where it is covered |
|---|---:|---|
| `adapter/transport.rs` | 58.66% | real serial/TCP error paths and hotplug/timing branches are HIL (`docs/hil/2026-11-1*-*`); loopback/split tests cover framing |
| `extension/on_event.rs` | 56.15% | failure/hang handlers are asserted; remaining branches are converter callbacks |
| `adapter/z_stack/runtime_adapter.rs` | 10.74% | production wiring to the Z-Stack adapter; the same operations are exercised through `adapter_zstack_suite` (130 tests) and the CP-07 recording runtime adapter |
| `controller/model/device.rs` / `endpoint.rs` | 44.65% / 39.94% | model operations are asserted via device/endpoint/group suites; uncovered branches are ZDO/ZCL interview paths that need a coordinator (CP-08/HIL) |
| `controller/controller.rs` (apiarist) | 34.06% | start/stop orchestration and coordinator-only paths; z2m controller lifecycle suite covers the wrapper |
| `controller/touchlink.rs`, `green_power.rs` | 0% / 13.81% | touchlink/GP are out of R1 acceptance (AGENTS §4); typing/traits are compiled |
| `z2m-rs/mqtt.rs` | 23.27% | reconnect/retained paths are covered by the in-process broker transcript and reconnect suite; TLS/client branches are exercised in `mqtt_reconnect` |
| `z2m-rs/controller.rs`, `zigbee.rs` | 49.83% / 39.20% | startup/abort/error paths are covered by `controller_startup`; live coordinator paths are HIL |
| definition/status tables | 16-19% | data tables; values are exercised through differential constants fixtures |

Error/retry/cancellation/persistence/reconnect specifically:

- retry/timeouts: `znp_suite` (timeouts, long timeout, no-reply), `differential_transport`, `network_map` LQI retry.
- cancellation: `controller_startup` abort/drop, `EventBus` panic isolation.
- persistence: `persistence_errors` (I/O failures), `settings`/`state` round-trips, CP-04 kill -9/reboot HIL.
- reconnect: `mqtt_reconnect` (7 tests) + broker transcript (`docs/fixtures/mqtt-transcript.json`).

## Gates

| Gate | Command | Status |
|---|---|---|
| Format | `cargo fmt --check` | repo has pre-existing formatting drift; changed files are formatted |
| Lints | `cargo clippy --workspace --all-targets` | run in CP-12 final acceptance |
| Workspace tests | `cargo test --workspace` | 33+ suites green (apiarist alone: 33 suites / 1015 tests) |
| Map checker | `node tools/docs/check-parity-maps.mjs` | `OK: 11 maps checked, 0 errors` |
| Node oracle | `node tools/oracle/generate.mjs` | regenerates 8 committed fixture sets |
| Model/ MQTT transcripts | `node tools/oracle/model-transcript.mjs`, `node tools/oracle/mqtt-transcript.mjs` | both report match |
| Broker integrations | `cargo test -p z2m-rs --test mqtt_reconnect --test mqtt_broker` | reconnect green; real broker ignored unless `MQTT_TEST_SERVER` |
| Fuzz/live oracle | `node tools/oracle/fuzz-corpus.mjs` (live), `cargo test -p zigbee-apiarist --test fuzz_boundaries` (offline replay) | 120-case corpus, 3 tests green |
| HIL | packaged install/reboot, crash recovery, coordinator start | `docs/hil/2026-11-17-*`, separate from the software gate |

## Software vs HIL split

- Software gate: workspace tests, differential fixtures, Node transcripts,
  in-process MQTT broker, fuzz corpus, coverage report.
- HIL acceptance (CP-12): Routerich AX3000 + Dongle-P start/resume, permit join,
  MQTT/Windfront responsiveness, packaged install/reboot and crash recovery.
  HIL evidence is recorded in `docs/hil/` and is not inferred from coverage.
