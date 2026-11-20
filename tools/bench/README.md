# Converter benchmark harness (Node.js baseline)

This harness measures the **original upstream** `zigbee2mqtt` +
`zigbee-herdsman-converters` converter path. It does not reimplement or modify
converter logic. It exists to establish the performance and functional baseline
for a future Rust converter runtime.

## Upstream path

Pinned source: zigbee2mqtt `3c841fdf` (2.14.3), converters `26.125.0`.

- `lib/extension/receive.ts:136-198` — select `fromZigbee` by cluster/type,
  build `msg`/`meta`, call `convert`, merge payloads, `publish`.
- `lib/extension/publish.ts:210-289` — build `tz` meta, call
  `convertSet`/`convertGet`, optimistic state, group members.
- `lib/extension/configure.ts:86-150` — call `definition.configure`.
- `lib/extension/onEvent.ts:74-86` — call `definition.onEvent`.
- `node_modules/zigbee-herdsman-converters/dist/index.js:342` —
  `prepareDefinition` (extends, exposes, options, generic toZigbee).

## What is mocked

Only the Zigbee/host boundary is mocked:

- `Endpoint.read/write/command/bind/configureReporting/commandResponse`,
  `getClusterAttributeValue`, `getDevice`, `getInputClusters`,
  `supportsInputCluster`;
- `Device` fields, `getEndpoint`, `addCustomCluster`, `save`, `meta`,
  `customClusters`, `customReadResponse`;
- `meta.device/state/options/message/publish/deviceExposesChanged`;
- optional deterministic host delay (`hostDelayMs`) and read values.

Every host call (operation, endpoint, cluster, attributes/command, options) is
recorded. `expectedHostCalls` in the corpus is the golden replay contract for
the future Rust runtime.

## Run

```bash
# x86 diagnostic, full corpus
node tools/bench/runner.mjs --out tools/bench/corpus

# one scenario / higher iteration count / CPU profile
node tools/bench/runner.mjs --filter configure-danfoss --iterations 200 \
  --profile --out /tmp/bench-config

# cold package load in fresh processes
node tools/bench/cold-load.mjs 5 --label linux-x64

# static complexity ranking of definitions
node tools/bench/heavy-scan.mjs

# parse a V8 CPU profile into a top-self-time summary
node tools/bench/profile.mjs <profile.cpuprofile> 20
```

On the Routerich AX3000, use the installed package and a staged external file:

```bash
node tools/bench/runner.mjs \
  --package /opt/zigbee2mqtt/node_modules/zigbee-herdsman-converters \
  --external /tmp/z2m-bench/fixtures/external/L101Ze_z2m.js \
  --out /tmp/z2m-bench/corpus-router --label routerich-openwrt
```

The benchmark does not open the serial port and does not touch the running
Zigbee2MQTT service. It was run with `nice -n 19` while the service stayed
running (the service was not stopped).

## Measurement semantics

- **totalMs/p50/p95/p99** — one converter call (including JS and mocked host
  calls), measured with `performance.now()` around repeated calls.
- **cpu** — `process.cpuUsage()` over the measured batch (user + system).
- **hostWaitMs** — time spent in the explicit mock host delay; subtract it to
  approximate active JS/host-boundary time.
- **blockingUpperBoundMs** — max one-call duration; isolated harness cannot
  measure the bridge event loop, so integration event-loop latency must be
  measured separately.
- **RSS/maxRSS** — `process.memoryUsage().rss` and
  `process.resourceUsage().maxRSS`.
- **cold load** — package root in fresh processes (`cold-load.mjs`) and first
  device-file require in a scenario. Device-file loads are cached per process.
- **batch isolation** — one device/endpoint per scenario; `meta`, binds,
  reportings and recorded calls are reset per iteration; the converter module
  cache (upstream `require` semantics) is shared; `clearGlobalStore()` runs
  between scenarios. Short scenarios use 100 iterations (20% warmup); configure
  uses 30–50; the host-delay scenario uses 20.
- **router runs** — three full runs; table uses median p50/p95 and max p99.
  p99 is retained as an outlier signal, not smoothed away.

## Corpus layout

`tools/bench/corpus/`:

- `manifest.json` — upstream revisions, Node/platform, external converter hash.
- `golden.json` — scenario inputs, expected results and expected host calls.
- `cases.json` — golden data plus measurements.
- `measurements.json` — metrics only.
- `heavy-definitions.json` — static complexity ranking (401 files / 4619
  definitions at the pinned version).
- `cold-load-linux-x64.json`.

`tools/bench/results/routerich-openwrt/` — three router runs, cold-load
distribution, CPU profiles and summaries. `tools/bench/results/linux-x64/`
holds the x86 CPU profile summaries.

## Scenario coverage

| Category | Scenarios |
|---|---|
| simple on/off | `fz-ikea-onoff`, `set-ikea-onoff`, `get-ikea-onoff` |
| brightness/color | `fz-ikea-brightness`, `fz-ikea-color-temp`, `fz-ikea-color-xy`, `set-ikea-brightness`, `set-ikea-color-temp` |
| Tuya datapoints | `fz-tuya-dp-temperature`, `fz-tuya-dp-multi`, `set-tuya-temperature-unit`, `fz-tuya-m9pro-multi-dp` |
| thermostat schedule | `get-danfoss-schedule`, `fz-danfoss-temperature`, `set-l101-setpoint` |
| multi-endpoint | `fz-ts0002-ep2-onoff`, `set-ts0002-ep2-onoff` |
| complex fromZigbee | `fz-danfoss-temperature`, `fz-tuya-m9pro-multi-dp` |
| configure with I/O | `configure-l101z`, `configure-danfoss`, `configure-external-l101` |
| onEvent/timers | `onEvent-daewoo-start`, `onEvent-daewoo-announce` |
| modernExtend preparation | `prepare-ikea-modern-extend`, `prepare-danfoss-thermostat`, `prepare-tuya-sensor`, `prepare-heavy-*` |
| real external converter | `prepare-external-l101`, `fz-external-l101-temp`, `set-external-l101-mode`, `get-external-l101-sensor`, `configure-external-l101` |
| host I/O separation | `set-ikea-brightness-host-delay` |

## Caveats

- Router Node is v22.23.4; x86 diagnostic Node is v24.22.2. The pinned converter
  package and harness are the same.
- The router ran with the live Node service and background load (`loadavg`
  around 1.5–2.2 at the time of the runs); p99 values are noisy and are reported
  as observed.
- The harness itself (mock construction, `structuredClone`, JSON normalization)
  contributes measurable overhead for sub-millisecond scenarios; CPU profiles
  show it explicitly. The future Rust harness must use the same golden inputs
  and expected calls, and should report its own harness overhead separately.
- No WAN/Zigbee network latency is included; host I/O is deterministic or an
  explicit `hostDelayMs`.
- No secrets from `ref/configuration.yaml` are used; device/endpoint data are
  synthetic. The external fixture is the user's `L101Ze_z2m.js` (SHA256 in the
  manifest).
