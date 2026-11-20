// Node.js baseline benchmark for zigbee-herdsman-converters.
//
// This runs the real upstream converter code. It does not reimplement any
// conversion; mocks only replace the Zigbee/host boundary.
//
// Usage:
//   node tools/bench/runner.mjs [--out tools/bench/corpus] [--quick]
//     [--package .upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters]
//     [--external tools/bench/fixtures/external/L101Ze_z2m.js]
//     [--filter substring] [--label x86]

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { Session } from "node:inspector";

import { createDeviceAndEndpoint, MockHost, normalizeValue } from "./mocks.mjs";
import { scenarios } from "./scenarios.mjs";
import { createUpstream } from "./upstream.mjs";

// Deterministic clock: some modernExtend configure paths embed the current
// time (genTime via customTimeResponse/writeTimeDaily), which would otherwise
// make the golden corpus change on every run.
const FIXED_NOW_MS = Date.UTC(2026, 10, 20, 12, 0, 0);
const RealDate = Date;
class FixedDate extends RealDate {
  constructor(...args) {
    if (args.length === 0) {
      super(FIXED_NOW_MS);
    } else {
      super(...args);
    }
  }
  static now() {
    return FIXED_NOW_MS;
  }
}
globalThis.Date = FixedDate;

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");

function arg(name, fallback) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : fallback;
}
const quick = process.argv.includes("--quick");
const overrideIterations = Number(arg("--iterations", "0")) || 0;
const profile = process.argv.includes("--profile");
const filter = arg("--filter", "");
const label = arg("--label", `${process.platform}-${process.arch}`);
const packageRoot = resolve(
  arg("--package", join(root, ".upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters")),
);
const externalPath = resolve(
  arg("--external", join(root, "tools/bench/fixtures/external/L101Ze_z2m.js")),
);
const outDir = resolve(arg("--out", join(root, "tools/bench/corpus")));

if (!existsSync(packageRoot)) {
  console.error(`converter package not found: ${packageRoot}`);
  process.exit(1);
}

const upstream = createUpstream({ packageRoot, externalPath });

function percentile(values, fraction) {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const index = Math.min(sorted.length - 1, Math.max(0, Math.ceil(fraction * sorted.length) - 1));
  return sorted[index];
}

function stats(values) {
  if (values.length === 0) return { count: 0 };
  const total = values.reduce((sum, value) => sum + value, 0);
  return {
    count: values.length,
    meanMs: total / values.length,
    p50Ms: percentile(values, 0.5),
    p95Ms: percentile(values, 0.95),
    p99Ms: percentile(values, 0.99),
    minMs: Math.min(...values),
    maxMs: Math.max(...values),
    totalMs: total,
    opsPerSecond: total > 0 ? (values.length * 1000) / total : 0,
  };
}

function profileSession() {
  if (!profile) return null;
  const session = new Session();
  session.connect();
  const post = (method) =>
    new Promise((resolve, reject) => {
      session.post(method, (error, result) => (error ? reject(error) : resolve(result)));
    });
  return { session, post };
}

async function startProfile(profiler) {
  if (!profiler) return;
  await profiler.post("Profiler.enable");
  await profiler.post("Profiler.start");
}

async function stopProfile(profiler, id) {
  if (!profiler) return;
  const { profile: data } = await profiler.post("Profiler.stop");
  const dir = join(outDir, "profiles");
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, `${id}.cpuprofile`), JSON.stringify(data));
}

function clone(value) {
  return value === undefined ? undefined : structuredClone(value);
}

function logger() {
  const noop = () => {};
  return { debug: noop, info: noop, warn: noop, error: noop };
}

function makeMeta(scenario, device, endpoint, definition, state, options, published) {
  return {
    device,
    logger: logger(),
    state: clone(state ?? {}),
    deviceExposesChanged: () => {},
    endpoint_name: scenario.endpointName,
    options: clone(options ?? {}),
    message: clone(scenario.message ?? {}),
    membersState: {},
    mapped: definition,
    publish: async (payload) => published.push(normalizeValue(payload)),
  };
}

function makeEndpoint(device, scenario, host) {
  const { device: built, endpoint } = createDeviceAndEndpoint(
    {
      ...scenario,
      device: scenario.device
        ? {
            ...scenario.device,
            endpoints:
              scenario.device.endpoints ??
              [{ id: 1, inputClusters: [], outputClusters: [] }],
          }
        : { endpoints: [{ id: 1, inputClusters: [], outputClusters: [] }] },
    },
    host,
    upstream.zcl(),
  );
  // Device constructor in mocks creates endpoints; ensure the selected one is
  // present even for endpointId=2.
  return { device: built, endpoint };
}

function createScenarioContext(scenario) {
  const host = new MockHost({
    delayMs: scenario.hostDelayMs ?? 0,
    readValues: scenario.readValues ?? {},
    zcl: upstream.zcl(),
  });
  const { device, endpoint } = makeEndpoint(deviceSpec(scenario), scenario, host);
  return { host, device, endpoint };
}

function resetScenarioContext(context, scenario) {
  const { host, device } = context;
  host.calls = [];
  host.waitMs = 0;
  host.readValues = { ...(scenario.readValues ?? {}) };
  device.meta = {};
  device.customClusters = {};
  device.customReadResponse = undefined;
  for (const endpoint of device.endpoints) {
    endpoint.binds = [];
    endpoint.configuredReportings = [];
    endpoint.meta = {};
  }
}

async function callOnce(scenario, prepared, iteration) {
  const definition = prepared.definition;
  const context = scenario.__context;
  if (context) resetScenarioContext(context, scenario);
  const host = context?.host ?? new MockHost({
    delayMs: scenario.hostDelayMs ?? 0,
    readValues: scenario.readValues ?? {},
    zcl: upstream.zcl(),
  });
  const { device, endpoint } = context ?? makeEndpoint(deviceSpec(scenario), scenario, host);
  const state = { ...(scenario.state ?? {}) };
  const options = { ...(scenario.options ?? {}) };
  const published = [];
  const meta = makeMeta(scenario, device, endpoint, definition, state, options, published);

  if (scenario.kind === "fromZigbee") {
    const type = scenario.msg.type ?? "attributeReport";
    const converters = definition.fromZigbee.filter((candidate) => {
      const types = Array.isArray(candidate.type) ? candidate.type : [candidate.type];
      return candidate.cluster === scenario.msg.cluster && types.includes(type);
    });
    if (converters.length === 0) {
      throw new Error(
        `no fromZigbee converter for ${definition.model}:${scenario.msg.cluster}/${type}`,
      );
    }
    const msg = {
      ...clone(scenario.msg),
      device,
      endpoint,
      meta: {
        ...(scenario.msg.meta ?? {}),
        zclTransactionSequenceNumber:
          (scenario.msg.meta?.zclTransactionSequenceNumber ?? 1) + iteration,
      },
    };
    const publish = async (payload) => {
      upstream.pkg.postProcessConvertedFromZigbeeMessage(definition, payload, options, device);
      published.push(normalizeValue(payload));
    };
    const convertMeta = {
      device,
      logger: logger(),
      state: clone(state),
      deviceExposesChanged: () => {},
    };
    let merged = {};
    for (const converter of converters) {
      const converted = await converter.convert(definition, msg, publish, options, convertMeta);
      if (converted) merged = { ...merged, ...converted };
    }
    if (Object.keys(merged).length > 0) await publish(merged);
    return {
      result: normalizeValue(Object.keys(merged).length > 0 ? merged : null),
      published,
      calls: host.calls,
      waitMs: host.waitMs,
    };
  }

  if (scenario.kind === "set" || scenario.kind === "get") {
    const converters = definition.toZigbee.filter((candidate) => {
      const keys = Array.isArray(candidate.key) ? candidate.key : [candidate.key];
      return keys.includes(scenario.key);
    });
    const converter =
      converters.find((candidate) => candidate.key !== undefined) ??
      definition.toZigbee.find((candidate) => candidate.key === undefined);
    if (!converter) {
      throw new Error(`no toZigbee converter for ${definition.model}:${scenario.key}`);
    }
    const result =
      scenario.kind === "set"
        ? await converter.convertSet(endpoint, scenario.key, scenario.value, meta)
        : await converter.convertGet(endpoint, scenario.key, meta);
    return {
      result: normalizeValue(result ?? null),
      published,
      calls: host.calls,
      waitMs: host.waitMs,
    };
  }

  if (scenario.kind === "configure") {
    if (!definition.configure) throw new Error(`no configure for ${definition.model}`);
    const coordinatorDevice = {
      ...device,
      ieeeAddr: "0x00124b0000000000",
      networkAddress: 0,
    };
    const coordinatorEndpoint = makeEndpoint(
      {
        ...scenario,
        device: { endpoints: [{ id: 1, inputClusters: [], outputClusters: [] }] },
      },
      scenario,
      host,
    ).endpoint;
    coordinatorEndpoint.deviceIeeeAddress = coordinatorDevice.ieeeAddr;
    await definition.configure(device, coordinatorEndpoint, definition);
    return { result: null, published, calls: host.calls, waitMs: host.waitMs };
  }

  if (scenario.kind === "onEvent") {
    if (!definition.onEvent) throw new Error(`no onEvent for ${definition.model}`);
    const event = {
      type: scenario.eventType,
      data: {
        device,
        state: clone(state),
        options: clone(options),
        deviceExposesChanged: () => {},
      },
    };
    if (scenario.eventType === "deviceInterview") event.data.status = "successful";
    await definition.onEvent(event);
    return { result: null, published, calls: host.calls, waitMs: host.waitMs };
  }

  throw new Error(`unknown scenario kind ${scenario.kind}`);
}

function deviceSpec(scenario) {
  return {
    ...scenario,
    device: scenario.device
      ? {
          ...scenario.device,
          endpoints:
            scenario.device.endpoints ?? [{ id: 1, inputClusters: [], outputClusters: [] }],
        }
      : { endpoints: [{ id: 1, inputClusters: [], outputClusters: [] }] },
  };
}

async function runScenario(scenario) {
  const iterations = overrideIterations || (quick
    ? Math.min(5, scenario.iterations ?? 20)
    : (scenario.iterations ?? 100));

  if (scenario.kind === "package") {
    const started = performance.now();
    const loadMs = upstream.loadRoot();
    const elapsed = performance.now() - started;
    return {
      scenario,
      cold: { loadMs: elapsed, reportedLoadMs: loadMs },
      stats: stats([elapsed]),
      cpu: null,
      memory: {
        rssBefore: null,
        rssAfter: process.memoryUsage().rss,
        maxRssBytes: process.resourceUsage().maxRSS * 1024,
        rssDelta: null,
      },
      eventLoop: { blockingUpperBoundMs: elapsed, measured: false, note: "cold package load" },
      hostWaitMs: 0,
      stateDrift: false,
      expected: null,
      expectedHostCalls: [],
    };
  }

  if (scenario.kind === "prepare") {
    upstream.loadRoot();
    const selected = upstream.selectRaw(scenario.definition);
    const raw = selected.raw;
    const firstStarted = performance.now();
    upstream.prepareRaw(raw);
    const prepareMs = performance.now() - firstStarted;
    for (let index = 0; index < 5; index++) upstream.prepareRaw(raw);
    const timings = [];
    const cpuBefore = process.cpuUsage();
    const rssBefore = process.memoryUsage().rss;
    for (let index = 0; index < iterations; index++) {
      const started = performance.now();
      upstream.prepareRaw(raw);
      timings.push(performance.now() - started);
    }
    const cpu = process.cpuUsage(cpuBefore);
    return {
      scenario,
      definition: {
        file: selected.file,
        index: selected.index,
        source: selected.source,
        model: raw.model,
        vendor: raw.vendor,
        loadMs: selected.loadMs,
        prepareMs,
      },
      stats: stats(timings),
      cpu: { userMs: cpu.user / 1000, systemMs: cpu.system / 1000, totalMs: (cpu.user + cpu.system) / 1000 },
      memory: {
        rssBefore,
        rssAfter: process.memoryUsage().rss,
        maxRssBytes: process.resourceUsage().maxRSS * 1024,
        rssDelta: process.memoryUsage().rss - rssBefore,
      },
      eventLoop: {
        blockingUpperBoundMs: timings.length > 0 ? Math.max(...timings) : 0,
        measured: false,
        note: "prepareDefinition batch",
      },
      hostWaitMs: 0,
      stateDrift: false,
      expected: { model: raw.model, vendor: raw.vendor, zigbeeModel: raw.zigbeeModel ?? null },
      expectedHostCalls: [],
    };
  }

  const prepared = upstream.prepare(scenario.definition);
  const context = createScenarioContext(scenario);
  scenario.__context = context;
  const warmup = Math.min(10, Math.max(1, Math.floor(iterations / 5)));
  let sample = null;
  for (let index = 0; index < warmup; index++) {
    sample = await callOnce(scenario, prepared, index);
  }

  upstream.clearStore();
  const timings = [];
  const cpuBefore = process.cpuUsage();
  const rssBefore = process.memoryUsage().rss;
  const profiler = profileSession();
  await startProfile(profiler);
  let hostWaitMs = 0;
  let stateDrift = false;
  for (let index = 0; index < iterations; index++) {
    const started = performance.now();
    const current = await callOnce(scenario, prepared, 1000 + index);
    timings.push(performance.now() - started);
    hostWaitMs += current.waitMs;
    if (!stateDrift && sample && JSON.stringify(current.result) !== JSON.stringify(sample.result)) {
      stateDrift = true;
    }
    if (!stateDrift && sample && JSON.stringify(current.published) !== JSON.stringify(sample.published)) {
      stateDrift = true;
    }
    sample = sample ?? current;
  }
  const cpu = process.cpuUsage(cpuBefore);
  await stopProfile(profiler, scenario.id);
  const rssAfter = process.memoryUsage().rss;
  const maxRssBytes = process.resourceUsage().maxRSS * 1024;

  delete scenario.__context;
  return {
    scenario,
    definition: {
      file: prepared.file,
      index: prepared.index,
      source: prepared.source,
      model: prepared.definition.model,
      vendor: prepared.definition.vendor,
      loadMs: prepared.loadMs,
      prepareMs: prepared.prepareMs,
    },
    stats: stats(timings),
    cpu: {
      userMs: cpu.user / 1000,
      systemMs: cpu.system / 1000,
      totalMs: (cpu.user + cpu.system) / 1000,
    },
    memory: {
      rssBefore,
      rssAfter,
      maxRssBytes,
      rssDelta: rssAfter - rssBefore,
    },
    eventLoop: {
      blockingUpperBoundMs: timings.length > 0 ? Math.max(...timings) : 0,
      measured: false,
      note: "isolated converter call; use the bridge integration run for event-loop delay",
    },
    hostWaitMs,
    stateDrift,
    expected: sample ? { result: sample.result, published: sample.published } : null,
    expectedHostCalls: sample ? sample.calls : [],
  };
}

function gitHead(path) {
  try {
    return execFileSync("git", ["-C", path, "rev-parse", "HEAD"], { encoding: "utf8" }).trim();
  } catch {
    return null;
  }
}

const manifest = {
  generatedAt: new Date().toISOString(),
  label,
  node: process.version,
  platform: process.platform,
  arch: process.arch,
  hostname: process.env.HOSTNAME ?? null,
  converterVersion: upstream.packageJson.version,
  converterRoot: packageRoot,
  upstreamZigbee2mqtt: gitHead(join(root, ".upstream/zigbee2mqtt")),
  upstreamConverters: gitHead(packageRoot),
  externalConverter: {
    path: externalPath,
    sha256: createHash("sha256").update(readFileSync(externalPath)).digest("hex"),
  },
  command: process.argv.join(" "),
};

const selected = scenarios.filter((scenario) => !filter || scenario.id.includes(filter));
if (selected.length === 0) {
  console.error(`no scenarios match '${filter}'`);
  process.exit(1);
}

mkdirSync(outDir, { recursive: true });
const cases = [];
const measurements = [];
for (const scenario of selected) {
  process.stderr.write(`bench ${scenario.id} ... `);
  try {
    const result = await runScenario(scenario);
    const { scenario: _scenario, ...rest } = result;
    const record = {
      id: scenario.id,
      kind: scenario.kind,
      description: scenario.description,
      input: {
        definition: scenario.definition,
        device: scenario.device,
        endpointId: scenario.endpointId,
        key: scenario.key,
        value: scenario.value,
        message: scenario.message,
        endpointName: scenario.endpointName,
        msg: scenario.msg,
        eventType: scenario.eventType,
        options: scenario.options,
        state: scenario.state,
      },
      ...rest,
    };
    cases.push(record);
    measurements.push({
      id: scenario.id,
      kind: scenario.kind,
      stats: rest.stats,
      cpu: rest.cpu,
      memory: rest.memory,
      eventLoop: rest.eventLoop,
      hostWaitMs: rest.hostWaitMs,
      stateDrift: rest.stateDrift,
      definition: rest.definition,
    });
    process.stderr.write(`${rest.stats.p50Ms?.toFixed(3) ?? "?"} ms\n`);
  } catch (error) {
    process.stderr.write(`ERROR ${error.message}\n`);
    cases.push({ id: scenario.id, kind: scenario.kind, error: String(error.stack ?? error) });
    measurements.push({ id: scenario.id, kind: scenario.kind, error: String(error.message ?? error) });
  }
}

const golden = cases.map((entry) => ({
  id: entry.id,
  kind: entry.kind,
  description: entry.description,
  input: entry.input,
  expected: entry.expected,
  expectedHostCalls: entry.expectedHostCalls,
  error: entry.error,
}));
writeFileSync(join(outDir, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
writeFileSync(join(outDir, "cases.json"), `${JSON.stringify(cases, null, 2)}\n`);
writeFileSync(join(outDir, "golden.json"), `${JSON.stringify(golden, null, 2)}\n`);
writeFileSync(join(outDir, "measurements.json"), `${JSON.stringify(measurements, null, 2)}\n`);
console.log(`wrote ${outDir}/manifest.json, cases.json, golden.json, measurements.json`);
