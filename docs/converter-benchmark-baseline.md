# Node.js converter performance baseline (2026-11-20)

This report records the **upstream Node.js baseline** for the post-R1
converter runtime. It does not implement or tune the Rust runtime. The
benchmark harness and golden corpus are in [`tools/bench/`](../tools/bench/);
methodology and scenario rationale are in
[`tools/bench/README.md`](../tools/bench/README.md).

## Environment and pins

| Item | x86 diagnostic | Router target |
|---|---|---|
| Hardware | benchmark container | Routerich AX3000 v1, OpenWrt 24.10.8, aarch64 |
| CPU/RAM | host | 2 cores class, 497 MB RAM |
| Node.js | v24.22.2 | v22.23.4 |
| zigbee2mqtt | `3c841fdf` (2.14.3) | same pinned source |
| zigbee-herdsman-converters | `26.125.0` | `26.125.0` (installed package) |
| External converter | `L101Ze_z2m.js`, 28,750 B, SHA256 `25393d40…4907` | same file |
| Service state | — | Node service kept running; benchmark `nice -n 19` |

No serial port or Zigbee network is used. The harness only calls the real
upstream converter functions with deterministic mocks.

## Method

- Real upstream `fromZigbee`, `toZigbee`, `configure`, `onEvent`,
  `prepareDefinition`; no converter logic is reimplemented.
- One device/endpoint per scenario; per-iteration reset of state, bindings,
  reportings and host calls. Module cache follows upstream `require`
  semantics; global store is cleared between scenarios.
- 37 scenarios; 100 iterations for short operations (20% warmup), 30–50 for
  configure/onEvent, 20 for the host-delay scenario.
- Total time is `performance.now()` around one converter call. CPU is
  `process.cpuUsage()` over the batch. `hostWaitMs` is explicit mock I/O wait.
  RSS/maxRSS from `process.memoryUsage()`/`resourceUsage()`.
- Cold load is measured in fresh processes; device-file load is the first
  require in a process.
- The harness installs a fixed clock (`2026-11-20T12:00:00Z`) so configure
  paths that embed the current `genTime` value produce a stable golden
  payload. With it, `golden.json` is byte-identical across repeated x86 runs
  and between x86 and the router.
- Router table uses the median of three full runs; p99 is the maximum observed
  across those runs (outliers retained, not smoothed).

## Upstream call path and scenario selection

The benchmark calls the same functions as the application:

- `lib/extension/receive.ts:136-198` — filter `fromZigbee`, build
  `msg`/`meta`, call `convert`, merge payloads, `publish`.
- `lib/extension/publish.ts:210-289` — `convertSet`/`convertGet`, meta,
  optimistic state.
- `lib/extension/configure.ts:86-150` — `definition.configure`.
- `lib/extension/onEvent.ts:74-86` — `definition.onEvent`.
- `converters/dist/index.js:342` — `prepareDefinition`.

The 37 scenarios cover simple on/off, brightness/color, Tuya datapoints,
thermostat schedule, multi-endpoint, complex fromZigbee, configure with
sequential I/O, onEvent/timers, modernExtend preparation, the real external
`L101Ze_z2m.js`, and an explicit host-delay case. The full list and rationale
are in [`tools/bench/README.md`](../tools/bench/README.md).

`tools/bench/heavy-scan.mjs` ranked all 4,619 definitions in 401 device files
by static complexity (fromZigbee/toZigbee/extend/exposes/Tuya datapoints/configure/
onEvent/file size). Top entries:

| Definition | File | Score | Why selected |
|---|---|---:|---|
| `MTV300E-UP` | `yokis.js` | 338.6 | 81 extends — heavy `prepareDefinition` |
| `M9Pro` | `tuya.js` | 323.5 | 48 Tuya datapoints — heavy DP dispatch |
| `Icon2` | `danfoss.js` | 319.2 | 9 fz/22 tz/configure — heavy thermostat |
| `Ensystec-Leak-Protect-Zigbee` | `ensystec.js` | 312.9 | 77 extends |
| `F3-Pro` | `tuya.js` | 309.5 | 53 Tuya datapoints |

## Cold load and device files

Package root (converters + `fromZigbee`/`toZigbee`/`lib` shared modules):

| Target | Values (3–5 fresh processes) | Median |
|---|---|---:|
| x86 | 686, 695, 708, 744, 800 ms | 708 ms |
| Router | 2,543 / 2,559 / 3,659 ms | 2,559 ms |

First device-file require on the router (the x86 diagnostic run is under
variable background load and is not used for acceptance):

| Definition file | Router |
|---|---:|
| `ikea.js` (`LED1545G12`) | 693 ms |
| `lumi.js` (`RTCGQ01LM`) | 886 ms |
| `avatto.js` (`ZWSH16`) | 213 ms |
| `tuya.js` (`TS0002`/`M9Pro`) | 1,929 ms |
| `danfoss.js` (`014G2461`) | 152 ms |
| `lytko.js` (`L101Z-SBI`) | 75 ms |
| `daewoo.js` (`WKE502Z`) | 13 ms |
| `yokis.js` (`MTV300E-UP`, 81 extends) | 127 ms |
| external `L101Ze_z2m.js` | 68 ms |

This is the main cold-start risk: the Node package root alone costs ~2.6 s on
the router, and `tuya.js` adds ~2.3 s. Warm converter calls are sub-millisecond
to a few milliseconds.

## Scenario results

Full machine-readable data:
[`tools/bench/results/routerich-openwrt/measurements.json`](../tools/bench/results/routerich-openwrt/measurements.json)
and [`tools/bench/corpus/measurements.json`](../tools/bench/corpus/measurements.json).

| Сценарий | x86 p50 | x86 p95 | Router p50 | Router p95 | Router p99 max |
|---|---:|---:|---:|---:|---:|
| prepare-ikea-modern-extend | 0.012 | 0.034 | 0.130 | 0.673 | 11.036 |
| prepare-danfoss-thermostat | 0.042 | 0.058 | 0.492 | 2.488 | 53.434 |
| prepare-tuya-sensor | 0.019 | 0.030 | 0.217 | 0.589 | 8.791 |
| fz-ikea-onoff | 0.030 | 0.054 | 0.502 | 1.648 | 15.676 |
| fz-ikea-brightness | 0.018 | 0.039 | 0.492 | 1.150 | 6.449 |
| fz-ikea-color-temp | 0.024 | 0.054 | 0.644 | 1.370 | 3.466 |
| fz-ikea-color-xy | 0.024 | 0.046 | 0.670 | 4.335 | 8.728 |
| set-ikea-onoff | 0.039 | 0.073 | 0.766 | 1.771 | 7.429 |
| set-ikea-brightness | 0.033 | 0.066 | 0.841 | 2.662 | 16.070 |
| set-ikea-color-temp | 0.021 | 0.041 | 0.843 | 5.745 | 15.671 |
| get-ikea-onoff | 0.007 | 0.017 | 0.304 | 1.310 | 4.347 |
| set-ikea-brightness-host-delay | 2.162 | 2.255 | 3.798 | 8.535 | 22.557 |
| fz-lumi-occupancy | 0.018 | 0.044 | 0.638 | 1.560 | 17.122 |
| fz-lumi-occupancy-clear | 0.009 | 0.018 | 0.346 | 0.902 | 5.826 |
| fz-l101-thermostat | 0.015 | 0.025 | 0.565 | 2.783 | 15.246 |
| set-l101-setpoint | 0.010 | 0.021 | 0.393 | 1.287 | 4.414 |
| get-l101-sensor-type | 0.008 | 0.017 | 0.363 | 1.648 | 9.213 |
| fz-tuya-dp-temperature | 0.018 | 0.033 | 0.655 | 1.461 | 6.347 |
| fz-tuya-dp-multi | 0.023 | 0.046 | 0.742 | 2.154 | 11.194 |
| set-tuya-temperature-unit | 0.019 | 0.042 | 0.609 | 1.639 | 4.249 |
| fz-ts0002-ep2-onoff | 0.017 | 0.020 | 0.512 | 1.150 | 13.317 |
| set-ts0002-ep2-onoff | 0.012 | 0.022 | 0.361 | 1.558 | 5.177 |
| fz-danfoss-temperature | 0.034 | 0.072 | 0.764 | 3.947 | 21.598 |
| get-danfoss-schedule | 0.013 | 0.031 | 0.422 | 1.285 | 9.721 |
| configure-l101z | 0.030 | 0.095 | 0.841 | 1.727 | 8.703 |
| configure-danfoss | 0.318 | 0.498 | 7.541 | 33.748 | 138.930 |
| onEvent-daewoo-start | 0.011 | 0.017 | 0.213 | 1.037 | 3.757 |
| onEvent-daewoo-announce | 0.019 | 0.028 | 0.434 | 0.947 | 8.346 |
| prepare-external-l101 | 0.011 | 0.032 | 0.134 | 0.562 | 28.354 |
| fz-external-l101-temp | 0.024 | 0.050 | 0.520 | 4.783 | 19.000 |
| set-external-l101-mode | 0.014 | 0.028 | 0.280 | 0.694 | 8.816 |
| get-external-l101-sensor | 0.012 | 0.022 | 0.263 | 3.674 | 24.604 |
| configure-external-l101 | 0.032 | 0.056 | 0.501 | 4.099 | 8.039 |
| prepare-heavy-yokis-81-extends | 0.057 | 0.074 | 0.844 | 10.581 | 17.519 |
| prepare-heavy-icon2-thermostat | 0.104 | 0.138 | 0.812 | 1.547 | 5.494 |
| fz-tuya-m9pro-multi-dp | 0.038 | 0.091 | 0.779 | 3.448 | 26.200 |
The cold package-root load is reported in the separate table above (fresh
processes); it is not part of the per-converter table.

## Heavy operations and profiles

Router CPU profiles (500 configure calls, 1,000 Tuya calls):

- `configure-danfoss`: `setupAttributes` 13.6% self time, `configure`
  (`modernExtend`) 11.7%, GC 8.2%, mock `read` 7.1%, `getInputClusters` 4.9%,
  `getEndpointsWithCluster` 3.9%. The profile shows that configuration is
  dominated by sequential host calls and `setupAttributes` loops, not by a
  single expensive ZCL operation.
- `fz-tuya-m9pro-multi-dp`: harness `callOnce`/`runScenario` and
  `structuredClone` are prominent; converter work is
  `hasAlreadyProcessedMessage` 2.8%, `tuya.js` `convert` 2.3% and `from` 1.9%.
  For very short conversions the harness overhead is material; the Rust
  harness must report its own overhead separately.

Profiles and summaries:
[`tools/bench/results/routerich-openwrt/profiles/`](../tools/bench/results/routerich-openwrt/profiles/),
[`tools/bench/results/linux-x64/profiles/`](../tools/bench/results/linux-x64/profiles/).

## Memory and event loop

- Router maxRSS: **51–74 MB** across scenarios (package root 51 MB; heavy
  Tuya/yokis scenarios up to 74 MB).
- x86 maxRSS: **86–141 MB** (V8/heap and larger address space).
- The isolated harness does not measure the bridge event loop; per-call
  `blockingUpperBoundMs` is the relevant upper bound. Event-loop/queue latency
  during startup, interview and persistence must be measured in the bridge
  integration run (CP-05/CP-12), not inferred from this harness.
- p99 spikes on the router (10–100 ms in heavy cases) coincide with background
  Node service load; p50/p95 medians are the stable comparison points.

## Acceptance criteria for the Rust runtime

These are proposals to be finalized after the engine spike, using this exact
corpus and the same golden inputs/host calls.

1. **Functional:** all 37 `golden.json` cases produce the same normalized
   payloads, state and host-call sequence; no converter exception/timeout;
   `stateDrift=false`.
2. **Steady state:** for every computationally significant scenario, Rust
   **p95 ≤ 2× Node p95** on Routerich (median of three runs), excluding
   explicit host wait. Baseline examples: `fz-*` p95 1–4 ms, `set-*` p95 1–4 ms,
   `configure-danfoss` p95 ~29–38 ms (last run round, load average 2.3–3.3),
   `onEvent` p95 ~0.6–2.2 ms.
3. **Cold use:** package/definition preparation may run in the background; it
   must not block `bridge/state online` or MQTT/frontend. If measured on the
   request path, Rust first use ≤ 2× Node cold load (root ~2.6 s, largest
   device file ~1.9 s).
4. **Memory:** converter worker RSS ≤ 1.5× Node harness maxRSS (target
   ≤ 110 MB; router baseline 74 MB) and no unbounded growth across scenarios.
5. **Responsiveness:** no single conversion blocks the bridge event loop
   > 50 ms p99; command/health request latency stays within the existing R1
   HIL bounds during conversion/warmup.
6. **Engine decision:** Boa/Kataan/other may be selected only after running
   this corpus and the functional differential. Claims from engine benchmarks
   are not acceptance evidence.

## Gaps and caveats

- The Routerich Node service was not stopped; background load causes p99
  variance. The three timing runs reported here ran at load average 2.3–3.3
  and were `nice -n 19`; earlier rounds at lower load produced p95 values
  about 30–40% lower for `configure-danfoss`, so treat p95 as an upper
  envelope rather than a best case.
- Node versions differ (x86 v24, router v22); the pinned converter package and
  scenario corpus are identical. The router's opkg installation carries no git
  metadata, so its manifest records `null` for upstream SHAs; its
  `converterVersion` is 26.125.0 and the app is zigbee2mqtt 2.14.3. The local
  pinned checkout used for the x86 run is `3c841fdf`.
- The harness measures converter compute plus its own mock/JSON overhead; CPU
  profiles expose this. Future Rust results must use the same corpus and report
  harness overhead separately.
- No physical Zigbee I/O latency is included; host responses are deterministic
  except `hostDelayMs`.
- 37 scenarios cover the requested categories and the statically heaviest
  definitions found by `heavy-scan.mjs`; they do not execute all 4,619
  definitions. Full-catalog loading remains a separate cold-start measurement.
- No secrets from `ref/configuration.yaml`; device data are synthetic. The
  external fixture is the user's `L101Ze_z2m.js`, identified by hash in the
  manifest.
