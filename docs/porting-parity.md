# Porting and Parity Verification

Normative procedure for every ported module. Read this before writing code.
`AGENTS.md` wins on scope; this document defines **how** parity is proven.

The goal is not "tests pass". The goal is evidence that the Rust function
observes the same behaviour as the pinned upstream TypeScript function on the
same inputs: return value, mutated state, emitted bytes/events, ordering,
timeouts and errors.

## 1. Oracle and pins

The oracle is the vendored upstream checkout in `.upstream/`, not memory and not
the latest internet revision.

Current pins (must match `.upstream/*/package.json`):

| Package | Version | Used for |
|---|---|---|
| `zigbee-herdsman` | 11.2.0 | ZNP, ZDO, ZCL, controller core |
| `zigbee2mqtt` | 2.14.3 | MQTT, HA, frontend, config, storage |
| `zigbee-herdsman-converters` | 26.125.0 | reference only; never executed in R1 |
| `zigbee2mqtt-windfront` | 2.15.1 | frontend protocol |
| `zigbee2mqtt-frontend` | 0.9.21 | historical frontend reference |

Rules:

- record the package version and git revision in every ported file header;
- if the vendored version changes, re-run the parity suite before touching code;
- never silently port from a different revision.

## 2. Before writing code

For the function/type being ported:

1. Read the upstream source file, its callers, and its upstream tests.
2. Read the exact revision (`git -C .upstream/<pkg> describe --tags --always`).
3. Decide the verification class:
   - pure function → differential fixture;
   - object/state machine → differential + state assertions;
   - runtime/protocol → transcript replay;
   - MQTT/HA/storage → broker/transcript comparison;
   - hardware-dependent → HIL row.
4. Add a row to the parity matrix (see §7) **before** implementation.
5. If the function is too large or crosses more than one upstream file, stop and
   ask the user to split it into smaller functions. Do not invent a simplified
   version.

## 3. Source-mapping comments

Every ported file starts with:

```rust
// Original: .upstream/zigbee-herdsman/src/adapter/z-stack/znp/znp.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).
```

Every ported function, type, field and constant carries an upstream anchor:

```rust
/// Upstream `onTransportData(chunk)` — znp.ts:58.
/// Upstream `WaitressMatcher` — znp.ts:29.
```

Rules:

- use the upstream name verbatim in the comment (camelCase stays camelCase);
- use Rust naming for the Rust identifier;
- add `file.ts:line` for functions and non-trivial types;
- if a member is intentionally not ported, say so and why
  (Node-only, test-only, unused in R1);
- do not write vague comments such as `// logic here`, `// model`,
  `// TODO: port later`.

## 4. Porting rules

- Mirror control flow and statement order, including early returns.
- Preserve error conditions, error classes/messages and timeouts.
- Preserve observable quirks; do not "clean up" upstream behaviour.
- No stub functions, no `todo!()`, no `unimplemented!()`, no placeholder
  branches.
- No hardcoded device names, values or scenarios in production code.
- No test-only branches in production code. Use an injected transport/clock
  only where upstream itself injects a dependency.
- Keep data catalogs (ZCL clusters, MT commands, status enums) explicitly marked
  as partial if they are partial; partial data is not parity.
- If a function cannot be completed in one pass, stop and ask for a split.

## 5. Required test layers

| Layer | Catches | Required for |
|---|---|---|
| Ported upstream unit test | known upstream cases | every ported module |
| Differential fixture test | Rust vs Node divergence | every pure function/object |
| Property/fuzz corpus | edge cases absent from upstream tests | buffers, codecs, parsers |
| Transcript replay | state/event/order divergence | ZNP, transport, ZCL routing |
| MQTT/HA transcript | topic/payload/retain/QoS/order divergence | bridge, HA, storage |
| HIL | hardware, timing, driver quirks | R1 acceptance |

A module is not "done" with only the first layer.

## 6. Differential harness

### 6.1 Layout

```
tools/oracle/                      # Node harness, committed
  run.mjs                          # reads NDJSON jobs, calls upstream, prints NDJSON
  generate.mjs                     # regenerates the committed fixtures
  packages/<module>.mjs            # one adapter per ported module
  package.json / package-lock.json
crates/<crate>/tests/fixtures/     # compact committed fixtures
  <module>.json
crates/<crate>/tests/differential_<module>.rs
```

Fixtures are committed. `.upstream/` is ignored and must never be a build-time
dependency of `cargo test`. Node is required only to regenerate fixtures or run
the live oracle; the Rust gate replays the committed JSON.

Initial modules: `frame`, `buffalo`, `buffalo_znp`, `zpi_object`, `zcl`,
`constants`, `definition` (see `crates/zigbee-apiarist/tests/differential_*.rs`).

Regenerate:

```bash
tsx tools/oracle/generate.mjs
# or, with local tooling:
cd tools/oracle && npm install && npm run generate
```

Run the live oracle on NDJSON jobs:

```bash
cd tools/oracle
npm run oracle < jobs.ndjson
```

### 6.2 Fixture schema

Use one self-contained JSON file per function or module:

```json
[
  {
    "id": "frame-to-buffer-srsp",
    "function": "Frame.toBuffer",
    "args": {"type": "SRSP", "subsystem": "SYS", "commandId": 3, "data": "0601"},
    "expected": "fe026103060167"
  },
  {
    "id": "frame-from-buffer-bad-fcs",
    "function": "Frame.fromBuffer",
    "args": {"length": 3, "fcsPosition": 7, "buffer": "fe0361080001 553f"},
    "error": "Invalid checksum"
  }
]
```

Canonicalisation (mandatory, otherwise comparison is meaningless):

- buffers as lowercase hex without separators;
- canonical JSON: object keys sorted, no insignificant whitespace;
- `undefined` and `null` are distinct and must be represented distinctly
  (`{"absent": true}` or an explicit marker);
- `BigInt` as a decimal string;
- floats compared bit-for-bit; NaN/Infinity represented explicitly;
- unordered maps sorted by key before comparison;
- errors compared by class and message, not just "it failed".

### 6.3 Node oracle skeleton

```js
// tools/oracle/run.mjs
// Reads NDJSON jobs from stdin, writes NDJSON results to stdout.
import {createInterface} from "node:readline";
import {loadModule} from "./packages/index.mjs";

const rl = createInterface({input: process.stdin, crlfDelay: Infinity});
for await (const line of rl) {
    if (!line.trim()) continue;
    const job = JSON.parse(line);
    const result = await loadModule(job).run(job);
    console.log(JSON.stringify(result));
}
```

Adapters import the pinned upstream source directly:

```js
// tools/oracle/packages/frame.mjs
import {Frame} from "../../../.upstream/zigbee-herdsman/src/adapter/z-stack/unpi/frame.ts";

export function run(job) {
    if (job.function === "Frame.toBuffer") {
        const frame = new Frame(job.args.type, job.args.subsystem, job.args.commandId,
            Buffer.from(job.args.data, "hex"));
        return {value: frame.toBuffer().toString("hex")};
    }
    // ...
}
```

Run with the same pinned Node version used by `engines`
(`^22.2.0 || ^24 || <=26.2`).

### 6.4 Rust driver skeleton

```rust
#[test]
fn frame_matches_upstream_fixtures() {
    let cases: Vec<FrameCase> = serde_json::from_str(include_str!(
        "fixtures/frame/frame.json"
    ))
    .expect("fixtures");

    for case in cases {
        let result = match case.function.as_str() {
            "Frame.toBuffer" => Ok(hex(&case.frame().to_buffer())),
            "Frame.fromBuffer" => Frame::from_buffer(
                case.args.length, case.args.fcs_position, &case.args.buffer,
            ).map(|frame| hex(&frame.to_buffer())).ok_or("Invalid checksum".to_string()),
            other => panic!("unknown function {other}"),
        };

        match (&result, &case.expected, &case.error) {
            (Ok(actual), Some(expected), None) => assert_eq!(actual, expected, "{}", case.id),
            (Err(actual), None, Some(expected)) => assert_eq!(actual, expected, "{}", case.id),
            other => panic!("fixture {}: unexpected result {other:?}", case.id),
        }
    }
}
```

### 6.5 Generate fixtures from upstream tests

Port upstream test inputs, not only hand-written happy paths. Extract:

- every `it(...)` input and assertion;
- invalid inputs and expected errors;
- boundary cases (empty buffer, max length, bad FCS, unknown cluster/command).

Commit a generator script if extraction is automated; keep the generator
deterministic and record the seed.

### 6.6 Fuzzing

Use `proptest` for buffers, lengths and structures. Two modes:

- **offline corpus**: generate N inputs, run the Node oracle once, commit the
  input/output pairs; `cargo test` replays them without Node;
- **nightly live oracle**: spawn the Node harness and compare thousands of
  cases; persist every failing input as a new fixture.

Never rely only on a live oracle for the default `cargo test` path.

## 7. Stateful and runtime parity

### 7.1 ZNP / transport replay

Compare a transcript, not a single return value:

1. Input chunks (including split boundaries).
2. Bytes written by the implementation, in order.
3. Emitted `received` objects, in order.
4. Errors/timeouts and their messages.

Upstream uses a mock serial binding; the Rust port uses an attached PTY pair or
an injected transport. Feed identical chunk sequences and compare
`Frame`/`ZpiObject` sequences. Do not compare wall-clock timestamps; inject a
clock or compare relative ordering.

### 7.2 MQTT / HA / storage

Capture a broker transcript from pinned Node z2m:

```
topic, payload, qos, retain, order
```

Replay the same inbound messages through the Rust bridge and diff the outbound
transcript. Cover:

- `bridge/state`, `bridge/info`, `bridge/devices`;
- `bridge/request/*` transactions and responses;
- `homeassistant/*` discovery and retained cleanup on rename/remove;
- `database.db` NDJSON round-trip, `configuration.yaml`, `state.json`;
- availability/last-seen/optimistic state.

### 7.3 HIL

Hardware rows stay `HARDWARE_PENDING` until executed on the actual router +
Dongle-P. A coordinator passed through to the dev container is enough for the
ZNP smoke probe (not for final R1 acceptance):

```yaml
# docker-compose example
devices:
  - "/dev/ttyACM0:/dev/ttyACM0"
group_add:
  - "dialout"
```

Hotplug is not propagated by Docker; restart the container after connecting.
Then run:

```bash
dmesg | tail -20
ls -l /dev/ttyACM0
Z2M_SERIAL_PORT=/dev/ttyACM0 cargo test -p zigbee-apiarist \
    --test hil_znp -- --ignored --nocapture
```

Record:

- hardware and firmware;
- binary hash;
- commands;
- serial/MQTT/dmesg evidence;
- comparison against the same scenario on pinned Node.

## 8. Parity matrix and CI gates

Maintain `docs/parity-matrix.md` (create from the R1 checklist) with:

| Function | Upstream file:line | Status | Evidence |
|---|---|---|---|
| `Znp::request` | `znp.ts:165` | PORTED | `znp` replay + fixtures |
| `AdapterTransport::get` | `transport.ts:140` | OMITTED | unused; Node-only modem query |
| `Zcl.Utils.getCluster` | `utils.ts:...` | PARTIAL | catalog subset; no parity claim |

Statuses: `PORTED`, `PARTIAL`, `OMITTED`, `BLOCKED`, `HARDWARE_PENDING`.
Never use `PORTED` without evidence.

CI gates:

1. `cargo test --workspace` runs all committed golden fixtures without Node.
2. `parity` job runs the Node oracle against the same fixtures and fails on any
   diff.
3. An exported ported function without a parity row or fixture fails review.
4. Fuzz/live-oracle runs nightly; failures become fixtures.
5. A module with a partial catalog cannot be reported as full parity.

## 9. Definition of done (per function)

- [ ] upstream source, callers and tests read at the pinned revision;
- [ ] upstream path + revision + `file.ts:line` anchors present;
- [ ] function fully implemented; no stubs, no model branches;
- [ ] ported upstream unit test passes;
- [ ] differential fixture compares value **and** error path;
- [ ] state/event order verified for stateful code;
- [ ] parity matrix row updated with evidence;
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` clean;
- [ ] intentional omissions documented with a reason.

## 10. Anti-patterns (will fail an audit)

- tests written to match the Rust implementation rather than upstream;
- hand-written "golden" values with no Node oracle behind them;
- hardcoded device names/values or scenario flags in production code;
- partial catalogs presented as complete;
- silent omission of functions, fields or error paths;
- claiming "byte-for-byte" without error/ordering parity;
- leaving `todo!()`, `unimplemented!()`, placeholder comments or model structs.

## 11. Environment notes

- `.upstream/` is gitignored; fixtures and harnesses must live in the repo.
- Node is installed persistently at `/config/.local/node-v24.22.2-linux-x64`
  (symlinked as `/config/.local/bin/node`), with global `tsx`. Do not download
  a fresh copy per session.
- `node --version` must satisfy upstream `engines`; the current pin is v24.22.2.
- For pure modules the oracle imports the vendored `.ts` sources via `tsx`; a
  full `pnpm install` is only needed when the imported module pulls runtime
  packages.
- The current environment has no `cc`; use a working linker for Rust
  verification, but never bake that workaround into project files.
