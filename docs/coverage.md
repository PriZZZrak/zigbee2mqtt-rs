# Coverage and gates (CP-11)

Reproducible line/function/region coverage for the R1 **software gate**, plus a
nightly branch-coverage run because stable LLVM source coverage has no branch
metric. HIL is a separate acceptance step and is never implied by these numbers.

## How to reproduce

```bash
PATH="$HOME/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin:$PATH" \
CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=/tmp/opencode/zigcc-cov \
./tools/coverage.sh
```

`tools/coverage.sh` builds every workspace test binary with
`-Cinstrument-coverage`, runs them with `LLVM_PROFILE_FILE`, merges the profiles
with `llvm-profdata` and reports with `llvm-cov`. `TOOLCHAIN` selects the rustup
toolchain and `COVERAGE_OPTIONS` adds instrumentation flags; the branch run uses
`TOOLCHAIN=nightly COVERAGE_OPTIONS="-Zcoverage-options=branch"`. The script reports the
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
| Commit | `6dbbd7c` (`fix(audit): close CP-02…CP-06 gaps found in the re-review`) |
| Toolchain | `rustc 1.99.1 (58cf8f9c3 2026-10-13)` |
| Coverage tool | rustup `llvm-tools-preview` 1.99.1 (`llvm-cov`, `llvm-profdata`) |
| Linker | `/tmp/opencode/zigcc-cov` (Zig cc wrapper; forwards `-u` to `-Wl,-u`) |
| Included crates | `zigbee-apiarist`, `zigbee-converters`, `z2m-rs` |
| Excluded from report | `tests/`, `examples/`, `benches/`, registry/rustlib, `target/`, external C/asm |
| Excluded from execution | 12 `#[ignore]` tests: 11 HIL ZNP (`hil_znp.rs`, requires `Z2M_SERIAL_PORT`) and 1 real-broker test (`mqtt_broker.rs`, requires `MQTT_TEST_SERVER`); 7 further markers live in upstream stub-file modules that cargo does not collect as test targets |

## Result

```text
TOTAL  regions 73.17%   functions 74.75% (2573/3442)   lines 72.94% (25628/35134)
```

| Crate | Lines | Functions |
|---|---:|---:|
| `zigbee-apiarist` | 67.85% (13215/19477) | 71.24% (1251/1756) |
| `zigbee-converters` | 79.81% (336/421) | 78.18% (43/55) |
| `z2m-rs` | 79.27% (12074/15231) | 78.47% (1276/1626) |

## Branch coverage (nightly)

LLVM source-based coverage does not emit branch regions on stable rustc, so the
branch metric is recorded with the nightly toolchain and the same script:

```bash
TOOLCHAIN=nightly COVERAGE_OPTIONS="-Zcoverage-options=branch" \
OUT=target/coverage-branch \
CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=/tmp/opencode/zigcc-cov \
./tools/coverage.sh
```

| Item | Value |
|---|---|
| Date | 2026-11-18 |
| Commit | `6dbbd7c` |
| Toolchain | `rustc 1.102.0-nightly (59cbdee2e 2026-11-17)` |
| Tool | rustup nightly `llvm-tools-preview` (`llvm-cov`, `llvm-profdata`) |
| Result | `TOTAL regions 71.69% functions 74.10% (2586/3490) lines 71.76% (24038/33499) branches 73.11% (3470/4746)` |

| Crate | Lines | Functions | Branches |
|---|---:|---:|---:|
| `zigbee-apiarist` | 66.61% (12450/18692) | 70.04% (1258/1796) | 78.40% (1695/2162) |
| `zigbee-converters` | 79.46% (321/404) | 77.19% (44/57) | 63.75% (51/80) |
| `z2m-rs` | 78.24% (11263/14396) | 78.53% (1280/1630) | 68.85% (1724/2504) |

The absolute values differ slightly from the stable run because the compiler
and coverage mapping differ between toolchains; use each table only within its
own row. The stable run stays the primary software gate (it is the pinned
toolchain); the nightly run adds the branch metric the audit asked for. The
nightly toolchain is not committed or required to build the project.

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
| `z2m-rs/mqtt.rs` | 29.20% | reconnect/retained paths are covered by the in-process broker transcript and reconnect suite; TLS/client branches are exercised in `mqtt_reconnect` |
| `z2m-rs/controller.rs`, `zigbee.rs` | 47.97% / 39.37% | startup/abort/error paths are covered by `controller_startup`; live coordinator paths are HIL |
| definition/status tables | 16-19% | data tables; values are exercised through differential constants fixtures |

Error/retry/cancellation/persistence/reconnect specifically:

- retry/timeouts: `znp_suite` (timeouts, long timeout, no-reply), `differential_transport`, `network_map` LQI retry.
- cancellation: `controller_startup` abort/drop, `EventBus` panic isolation.
- persistence: `persistence_errors` (I/O failures), `settings`/`state` round-trips, CP-04 kill -9/reboot HIL.
- reconnect: `mqtt_reconnect` (7 tests) + broker transcript (`docs/fixtures/mqtt-transcript.json`).

## Gates

| Gate | Command | Status |
|---|---|---|
| Format | `cargo fmt --all -- --check` | **fails on 134 pre-existing files** (repo-wide drift); every Rust file changed for this audit passes `rustfmt --check`, repo-wide reformat is a separate change |
| Lints | `cargo clippy --workspace --all-targets` | exit 0 (warnings only) |
| Workspace tests | `cargo test --workspace` | 78 suites / 1568 passed, 0 failed, 12 ignored (final run on `6dbbd7c`) |
| Map checker | `node tools/docs/check-parity-maps.mjs` | `OK: 11 maps checked, 0 errors` |
| Node oracle | `node tools/oracle/generate.mjs` | regenerates 8 committed fixture sets |
| Model/ MQTT transcripts | `node tools/oracle/model-transcript.mjs`, `node tools/oracle/mqtt-transcript.mjs` | both report match; MQTT compares publishes/subscriptions element-wise (order + duplicates) and asserts a new subscriber receives retained `bridge/state offline` |
| Broker integrations | `cargo test -p z2m-rs --test mqtt_reconnect --test mqtt_broker` | reconnect green; real broker ignored unless `MQTT_TEST_SERVER` |
| Fuzz/live oracle | `node tools/oracle/fuzz-corpus.mjs` (live), `cargo test -p zigbee-apiarist --test fuzz_boundaries` (offline replay) | 120-case corpus, 3 tests green |
| HIL | packaged install/reboot, crash recovery, coordinator start | `docs/hil/2026-11-17-*` and `docs/hil/2026-11-18-*`, separate from the software gate |

## Software vs HIL split

- Software gate: workspace tests, differential fixtures, Node transcripts,
  in-process MQTT broker, fuzz corpus, coverage report.
- HIL acceptance (CP-12): Routerich AX3000 + Dongle-P start/resume, permit join,
  MQTT/Windfront responsiveness, packaged install/reboot and crash recovery.
  HIL evidence is recorded in `docs/hil/` and is not inferred from coverage.
