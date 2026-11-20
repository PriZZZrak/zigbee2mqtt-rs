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

import { invokeConverter, makeEndpoint } from "./call.mjs";
import { installFixedClock } from "./clock.mjs";
import { MockHost } from "./mocks.mjs";
import { scenarios } from "./scenarios.mjs";
import { canonicalJson, stats } from "./stats.mjs";
import { createUpstream } from "./upstream.mjs";

installFixedClock();

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");

function arg(name, fallback) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : fallback;
}
const quick = process.argv.includes("--quick");
const overrideIterations = Number(arg("--iterations", "0")) || 0;
const profile = process.argv.includes("--profile");
const generateGolden = process.argv.includes("--generate-golden");
const verifyGolden = process.argv.includes("--verify-golden");
const force = process.argv.includes("--force");
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

function createScenarioContext(scenario) {
  const host = new MockHost({
    delayMs: scenario.hostDelayMs ?? 0,
    readValues: scenario.readValues ?? {},
    zcl: upstream.zcl(),
  });
  const { device, endpoint } = makeEndpoint(scenario, host, upstream.zcl());
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
  const context = scenario.__context;
  if (context) resetScenarioContext(context, scenario);
  const host = context?.host ?? new MockHost({
    delayMs: scenario.hostDelayMs ?? 0,
    readValues: scenario.readValues ?? {},
    zcl: upstream.zcl(),
  });
  const { device, endpoint } = context ?? makeEndpoint(scenario, host, upstream.zcl());
  return invokeConverter({
    upstream,
    scenario,
    prepared,
    host,
    device,
    endpoint,
    iteration,
  });
}

// The upstream transaction store is keyed by `ieeeAddr-endpoint` and is not
// cleared by clearGlobalStore(). With the frozen benchmark clock its entries
// never expire, so every scenario needs its own transaction-id range to avoid
// false duplicate detection between scenarios.
const scenarioSequenceBase = new Map(
  scenarios.map((scenario, index) => [scenario.id, (index + 1) * 10000]),
);

function sequenceBase(scenario) {
  return scenarioSequenceBase.get(scenario.id) ?? 1;
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
      converterStats: stats([elapsed]),
      overheadStats: stats([0]),
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
      converterStats: stats(timings),
      overheadStats: stats(timings.map(() => 0)),
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
    sample = await callOnce(scenario, prepared, sequenceBase(scenario) + index);
  }

  upstream.clearStore();
  const timings = [];
  const converterTimings = [];
  const cpuBefore = process.cpuUsage();
  const rssBefore = process.memoryUsage().rss;
  const profiler = profileSession();
  await startProfile(profiler);
  let hostWaitMs = 0;
  let stateDrift = false;
  let driftBaseline = null;
  for (let index = 0; index < iterations; index++) {
    const started = performance.now();
    const current = await callOnce(scenario, prepared, sequenceBase(scenario) + 1000 + index);
    const totalMs = performance.now() - started;
    timings.push(totalMs);
    converterTimings.push(current.converterMs);
    hostWaitMs += current.waitMs;
    // Compare measured calls with each other; a one-call warmup can legitimately
    // differ from later calls because converters cache per-device state.
    driftBaseline = driftBaseline ?? current;
    if (!stateDrift && JSON.stringify(current.result) !== JSON.stringify(driftBaseline.result)) {
      stateDrift = true;
    }
    if (!stateDrift && JSON.stringify(current.published) !== JSON.stringify(driftBaseline.published)) {
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
    // `stats` is the legacy harness-total metric kept for comparability.
    stats: stats(timings),
    converterStats: stats(converterTimings),
    overheadStats: stats(timings.map((value, index) => value - converterTimings[index])),
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

if (generateGolden && verifyGolden) {
  console.error("--generate-golden and --verify-golden are mutually exclusive");
  process.exit(1);
}

let goldenReference = null;
if (verifyGolden) {
  const goldenPath = join(outDir, "golden.json");
  if (!existsSync(goldenPath)) {
    console.error(`golden file not found: ${goldenPath}`);
    process.exit(1);
  }
  goldenReference = JSON.parse(readFileSync(goldenPath, "utf8"));
  if (!Array.isArray(goldenReference) || goldenReference.length === 0) {
    console.error(`golden file is empty or malformed: ${goldenPath}`);
    process.exit(1);
  }
}

if (generateGolden && existsSync(join(outDir, "golden.json")) && !force) {
  console.error(
    `refusing to overwrite existing golden file: ${join(outDir, "golden.json")} (pass --force)`,
  );
  process.exit(1);
}

let selected;
if (verifyGolden) {
  selected = [];
  for (const entry of goldenReference) {
    if (filter && !entry.id.includes(filter)) continue;
    const scenario = scenarios.find((candidate) => candidate.id === entry.id);
    if (!scenario) {
      console.error(`golden case has no matching scenario: ${entry.id}`);
      process.exitCode = 1;
      continue;
    }
    selected.push(scenario);
  }
} else {
  selected = scenarios.filter((scenario) => !filter || scenario.id.includes(filter));
}
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
      // Mean aliases with the audit-requested names; `stats`/`converterStats`
      // retain the full p50/p95/p99 distributions.
      harnessTotalMs: rest.stats?.meanMs ?? null,
      converterOnlyMs: rest.converterStats?.meanMs ?? null,
      ...rest,
    };
    cases.push(record);
    measurements.push({
      id: scenario.id,
      kind: scenario.kind,
      harnessTotalMs: rest.stats?.meanMs ?? null,
      converterOnlyMs: rest.converterStats?.meanMs ?? null,
      stats: rest.stats,
      converterStats: rest.converterStats,
      overheadStats: rest.overheadStats,
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
const failedCases = cases.filter((entry) => entry.error);
if (failedCases.length > 0) {
  process.exitCode = 1;
}

function serializable(value) {
  return JSON.parse(JSON.stringify(value));
}

function describeMismatch(reference, actual) {
  const keys = new Set([...Object.keys(reference), ...Object.keys(actual)]);
  for (const key of keys) {
    const left = canonicalJson(reference[key]);
    const right = canonicalJson(actual[key]);
    if (left !== right) {
      const clip = (text) => (text === undefined ? "undefined" : text.slice(0, 300));
      return `${key}: expected ${clip(left)} actual ${clip(right)}`;
    }
  }
  return "unknown difference";
}

if (verifyGolden) {
  const actualById = new Map(golden.map((entry) => [entry.id, serializable(entry)]));
  const driftById = new Map(cases.map((entry) => [entry.id, Boolean(entry.stateDrift)]));
  let mismatches = 0;
  let checked = 0;
  for (const reference of goldenReference) {
    if (filter && !reference.id.includes(filter)) continue;
    checked += 1;
    const actual = actualById.get(reference.id);
    if (!actual) {
      console.error(`FAIL ${reference.id}: scenario did not run`);
      mismatches += 1;
      continue;
    }
    if (driftById.get(reference.id)) {
      console.error(`FAIL ${reference.id}: state drift during repeated calls`);
      mismatches += 1;
    }
    if (canonicalJson(actual) !== canonicalJson(serializable(reference))) {
      console.error(`FAIL ${reference.id}: ${describeMismatch(serializable(reference), actual)}`);
      mismatches += 1;
    }
  }
  if (mismatches > 0 || failedCases.length > 0) {
    console.error(
      `golden verify FAILED: ${mismatches} mismatch(es), ${failedCases.length} scenario error(s) of ${checked} checked`,
    );
    process.exitCode = 1;
  } else {
    console.log(`golden verify OK: ${checked} case(s) match ${join(outDir, "golden.json")}`);
  }
} else if (generateGolden) {
  if (failedCases.length > 0) {
    console.error(`golden generation FAILED: ${failedCases.length} scenario error(s); file not written`);
    process.exitCode = 1;
  } else {
    writeFileSync(join(outDir, "golden.json"), `${JSON.stringify(golden, null, 2)}\n`);
    console.log(`wrote ${outDir}/golden.json (${golden.length} cases)`);
  }
} else {
  writeFileSync(join(outDir, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  writeFileSync(join(outDir, "cases.json"), `${JSON.stringify(cases, null, 2)}\n`);
  writeFileSync(join(outDir, "measurements.json"), `${JSON.stringify(measurements, null, 2)}\n`);
  console.log(`wrote ${outDir}/manifest.json, cases.json, measurements.json`);
  if (failedCases.length > 0) {
    console.error(`${failedCases.length} scenario error(s)`);
    process.exitCode = 1;
  }
}
