# Converter engine study (prototype)

This directory contains the **feasibility prototypes** for the post-R1
converter engine study. It is not a runtime and must not be shipped as one.

See [`../../docs/converter-engine-study.md`](../../docs/converter-engine-study.md)
for the comparison, measurements and GO/NO-GO decision.

## What is here

| File | Purpose |
|---|---|
| `src/bin/feature_probe.rs` | Language/feature probe (`fixtures/features.js`) |
| `src/bin/converter_probe.rs` | Loads the real upstream package through the JS CJS loader and runs the golden scenarios |
| `src/bin/guard_probe.rs` | Infinite loop / heap limit / recovery probes |
| `src/bin/async_probe.rs` | Pending-Promise host scheduler, interleaving, host error/timeout/interrupt |
| `fixtures/bootstrap.js` | CJS loader, Node shims, Zcl shim, device/endpoint mocks, scenario runner |
| `fixtures/scenarios.json` | The 36 converter scenarios from `tools/bench/scenarios.mjs` |
| `fixtures/zcl.json` | JSON snapshot of upstream `Zcl` enums/clusters/consts |
| `fixtures/features.js` | Language surface probe |
| `results/` | Raw x86 and Routerich runs, guard, async and profile results |

The upstream package is not vendored here; the probes read the pinned install
from `.upstream/` (x86) or `/opt/zigbee2mqtt/node_modules/...` (router).

## Engine features

Cargo features select one engine:

```bash
cargo build --release --features qjs    --bin converter_probe   # quickjs-oxide 0.5
cargo build --release --features boa    --bin converter_probe   # Boa 0.22
cargo build --release --features kataan --bin converter_probe   # Kataan 0.0.10
```

The same binary name is overwritten by each build; copy it aside when comparing
engines (`cp target/release/converter_probe /tmp/converter_probe_qjs`, etc.).

## Run (x86 diagnostic)

From the repository root:

```bash
tools/engine-study/target/release/converter_probe \
  --iterations 20 \
  --package .upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters \
  --herdsman .upstream/zigbee2mqtt/node_modules/zigbee-herdsman \
  --scenarios tools/engine-study/fixtures/scenarios.json \
  --zcl tools/engine-study/fixtures/zcl.json \
  --bootstrap tools/engine-study/fixtures/bootstrap.js \
  --external tools/bench/fixtures/external/L101Ze_z2m.js \
  > /tmp/engine.json
```

`--filter <substring>` selects scenarios. `--iterations N` sets the batch size;
the first definition preparation is reported separately as `prepareMs`.

Feature probe:

```bash
target/release/feature_probe fixtures/features.js
```

Guard probe:

```bash
target/release/guard_probe
```

Async host scheduler (quickjs-oxide only):

```bash
target/release/async_probe --jobs 8
```

Host-operation profiles (`serializeMs`, `boundaryMs`, `resumeMs`, Zcl/mock
lookup counts):

```bash
target/release/converter_probe --filter configure-danfoss --iterations 20 \
  --profile-host
```

The async probe starts one `configure-danfoss` plus N independent set jobs,
returns pending Promises to Rust, settles them from the scheduler and reports
interleaving, host errors, timeout and interrupt recovery. The current
scheduler is a study prototype, not a production scheduler.

## Run (Routerich AX3000)

The router binary must be `aarch64-unknown-linux-musl`. Stage the fixtures and
the upstream package (already installed for the Node service):

```bash
ssh root@router 'mkdir -p /tmp/engine-study/fixtures/external'
tar -C tools/engine-study/fixtures -cf - bootstrap.js scenarios.json zcl.json \
  | ssh root@router 'tar -C /tmp/engine-study/fixtures -xf -'
ssh root@router 'cat > /tmp/engine-study/fixtures/external/L101Ze_z2m.js' \
  < tools/bench/fixtures/external/L101Ze_z2m.js
ssh root@router 'cat > /tmp/engine-study/converter_probe_qjs' \
  < target/aarch64-unknown-linux-musl/release/converter_probe
ssh root@router 'chmod +x /tmp/engine-study/converter_probe_qjs'

ssh root@router 'cd /tmp/engine-study &&
  nice -n 19 ./converter_probe_qjs --iterations 10 \
    --package /opt/zigbee2mqtt/node_modules/zigbee-herdsman-converters \
    --herdsman /opt/zigbee2mqtt/node_modules/zigbee-herdsman \
    --scenarios /tmp/engine-study/fixtures/scenarios.json \
    --zcl /tmp/engine-study/fixtures/zcl.json \
    --bootstrap /tmp/engine-study/fixtures/bootstrap.js \
    --external /tmp/engine-study/fixtures/external/L101Ze_z2m.js'
```

The Node service is not stopped. Router runs are `nice -n 19`; the live
service and its memory pressure affect results (see the study report).

## Build notes

The project container has Rust but no system C linker; the probes were built
with Zig as `cc` and the Rust toolchain's `rust-lld` for the static musl
targets:

```bash
# host (build scripts/proc macros)
PATH=/tmp:$PATH RUSTFLAGS='-C linker=/tmp/cc' cargo build --release ...

# aarch64-musl
RUSTLLD=$(ls ~/.rustup/toolchains/*/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld | head -1)
PATH=/tmp:$PATH \
  CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=/tmp/cc \
  CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=$RUSTLLD \
  cargo build --release --target aarch64-unknown-linux-musl ...
```

`/tmp/cc` was `zig cc`; `/tmp/cc-aarch64` (`zig cc -target aarch64-linux-musl`)
cannot link the Rust static target because it rejects
`--fix-cortex-a53-843419`, hence `rust-lld` for the final link. These are
environment workarounds, not part of the runtime design.

## Limits

- The bootstrap is a study shim: simplified Zcl, minimal Node builtins,
  non-executing timer callbacks, synthetic devices.
- It calls the real upstream converters; it is not a substitute for the
  production host, loader or bridge integration.
- `converterMs` excludes loader/mock overhead; `wallMs` includes it.
- `bootstrap.js` contains study-only lookup caches used by the async
  performance experiment. They preserve upstream semantics (cluster
  definitions are immutable and caches are invalidated on
  `addCustomCluster`), but production should use a Rust-side Zcl index
  instead.
- Do not reuse the prototype as production code. The study report lists the
  gates that a production runtime must satisfy.
