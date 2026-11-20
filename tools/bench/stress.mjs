// Concurrency / stress benchmark for the upstream converter call path.
//
// Separate from runner.mjs: this models many devices, a bounded dispatch
// queue, concurrent reports/sets and pending host Promises. Results are
// written to their own file/directory and must not be mixed with the
// single-call baseline.
//
// Usage:
//   node tools/bench/stress.mjs [--out tools/bench/results/<host>/stress/run1]
//     [--package ...] [--external ...] [--label host] [--quick] [--filter name]

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { invokeConverter, makeEndpoint } from "./call.mjs";
import { installFixedClock } from "./clock.mjs";
import { MockHost } from "./mocks.mjs";
import { scenarios } from "./scenarios.mjs";
import { stats } from "./stats.mjs";
import { createUpstream } from "./upstream.mjs";

installFixedClock();

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");

function arg(name, fallback) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : fallback;
}

const quick = process.argv.includes("--quick");
const filter = arg("--filter", "");
const label = arg("--label", `${process.platform}-${process.arch}`);
const packageRoot = resolve(
  arg("--package", join(root, ".upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters")),
);
const externalPath = resolve(
  arg("--external", join(root, "tools/bench/fixtures/external/L101Ze_z2m.js")),
);
const outDir = resolve(arg("--out", join(root, "tools/bench/results", label, "stress")));

if (!existsSync(packageRoot)) {
  console.error(`converter package not found: ${packageRoot}`);
  process.exit(1);
}

const upstream = createUpstream({ packageRoot, externalPath });

function template(id) {
  const scenario = scenarios.find((candidate) => candidate.id === id);
  if (!scenario) throw new Error(`scenario template not found: ${id}`);
  return scenario;
}

const templates = {
  onoff: template("fz-ikea-onoff"),
  brightness: template("fz-ikea-brightness"),
  occupancy: template("fz-lumi-occupancy"),
  tuyaMulti: template("fz-tuya-m9pro-multi-dp"),
  setBrightness: template("set-ikea-brightness"),
};

const preparedCache = new Map();
function preparedFor(scenario) {
  const key = `${scenario.definition.source}|${scenario.definition.file}|${scenario.definition.model}`;
  if (!preparedCache.has(key)) preparedCache.set(key, upstream.prepare(scenario.definition));
  return preparedCache.get(key);
}

function deviceAddress(index) {
  return `0x00124b${(0x1000 + index).toString(16).padStart(10, "0").slice(-10)}`;
}

function buildDevice(poolScenario, index, delayMs) {
  const scenario = {
    ...poolScenario,
    device: {
      ...poolScenario.device,
      ieeeAddr: deviceAddress(index),
      networkAddress: 0x1000 + index,
    },
    hostDelayMs: delayMs,
  };
  const host = new MockHost({ delayMs, readValues: {}, zcl: upstream.zcl() });
  const { device, endpoint } = makeEndpoint(scenario, host, upstream.zcl());
  return { scenario, host, device, endpoint, prepared: preparedFor(poolScenario) };
}

function resetDevice(entry) {
  entry.host.calls = [];
  entry.host.waitMs = 0;
  entry.device.meta = {};
  entry.device.customClusters = {};
  entry.device.customReadResponse = undefined;
  for (const endpoint of entry.device.endpoints) {
    endpoint.binds = [];
    endpoint.configuredReportings = [];
    endpoint.meta = {};
  }
}

let transactionSequence = 1;

async function execute(entry) {
  resetDevice(entry);
  return invokeConverter({
    upstream,
    scenario: entry.scenario,
    prepared: entry.prepared,
    host: entry.host,
    device: entry.device,
    endpoint: entry.endpoint,
    iteration: transactionSequence++,
  });
}

function buildPool({ devices, delayMs, kind }) {
  const reportTemplates = [templates.onoff, templates.brightness, templates.occupancy, templates.tuyaMulti];
  const pool = [];
  for (let index = 0; index < devices; index++) {
    const opKind = kind === "mixed" ? (index % 2 === 0 ? "set" : "report") : kind;
    const baseScenario =
      opKind === "set" ? templates.setBrightness : reportTemplates[index % reportTemplates.length];
    const entry = buildDevice(baseScenario, index, delayMs);
    entry.opKind = opKind;
    pool.push(entry);
  }
  return pool;
}

async function runWorkload(config) {
  const { name, operations, concurrency, devices, delayMs, kind } = config;
  upstream.clearStore();
  const pool = buildPool({ devices, delayMs, kind });
  const warmupCount = Math.min(delayMs > 0 ? 20 : 100, Math.ceil(operations / 4));
  for (let index = 0; index < warmupCount; index++) {
    await execute(pool[index % pool.length]);
  }

  const submittedAt = new Float64Array(operations);
  const queue = [];
  for (let index = 0; index < operations; index++) {
    submittedAt[index] = performance.now();
    queue.push(index);
  }

  const latency = [];
  const queueWait = [];
  const service = [];
  const converterLatency = [];
  let cursor = 0;
  const cpuBefore = process.cpuUsage();
  const rssBefore = process.memoryUsage().rss;
  const wallStart = performance.now();

  async function worker() {
    while (true) {
      const index = cursor++;
      if (index >= operations) return;
      const entry = pool[index % pool.length];
      const start = performance.now();
      queueWait.push(start - submittedAt[index]);
      const result = await execute(entry);
      const end = performance.now();
      latency.push(end - submittedAt[index]);
      service.push(end - start);
      converterLatency.push(result.converterMs);
    }
  }

  await Promise.all(Array.from({ length: Math.min(concurrency, operations) }, () => worker()));
  const wallMs = performance.now() - wallStart;
  const cpu = process.cpuUsage(cpuBefore);
  const rssAfter = process.memoryUsage().rss;

  return {
    name,
    kind,
    devices,
    concurrency,
    operations,
    hostDelayMs: delayMs,
    wallMs,
    throughputOpsPerSec: wallMs > 0 ? (operations * 1000) / wallMs : 0,
    queueWaitMs: stats(queueWait),
    serviceMs: stats(service),
    latencyMs: stats(latency),
    converterMs: stats(converterLatency),
    cpu: {
      userMs: cpu.user / 1000,
      systemMs: cpu.system / 1000,
      totalMs: (cpu.user + cpu.system) / 1000,
    },
    memory: {
      rssBefore,
      rssAfter,
      maxRssBytes: process.resourceUsage().maxRSS * 1024,
      rssDelta: rssAfter - rssBefore,
    },
  };
}

const workloadDefinitions = [
  {
    name: "reports-1-device-c1",
    kind: "report",
    operations: 2000,
    concurrency: 1,
    devices: 1,
    delayMs: 0,
  },
  {
    name: "reports-16-devices-c8",
    kind: "report",
    operations: 4000,
    concurrency: 8,
    devices: 16,
    delayMs: 0,
  },
  {
    name: "sets-16-devices-c1",
    kind: "set",
    operations: 1000,
    concurrency: 1,
    devices: 16,
    delayMs: 1,
  },
  {
    name: "sets-16-devices-c8",
    kind: "set",
    operations: 1000,
    concurrency: 8,
    devices: 16,
    delayMs: 1,
  },
  {
    name: "mixed-32-devices-c16",
    kind: "mixed",
    operations: 3000,
    concurrency: 16,
    devices: 32,
    delayMs: 1,
  },
  {
    name: "queue-backlog-sets-c4",
    kind: "set",
    operations: 2000,
    concurrency: 4,
    devices: 32,
    delayMs: 5,
  },
  {
    name: "pending-sets-c64-delay20",
    kind: "set",
    operations: 200,
    concurrency: 64,
    devices: 32,
    delayMs: 20,
  },
];

const selected = workloadDefinitions
  .filter((workload) => !filter || workload.name.includes(filter))
  .map((workload) =>
    quick
      ? {
          ...workload,
          operations: Math.max(20, Math.ceil(workload.operations / 10)),
          concurrency: Math.min(workload.concurrency, 8),
          devices: Math.min(workload.devices, 8),
        }
      : workload,
  );

if (selected.length === 0) {
  console.error(`no workloads match '${filter}'`);
  process.exit(1);
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
  benchmark: "concurrency-stress",
  node: process.version,
  platform: process.platform,
  arch: process.arch,
  converterVersion: upstream.packageJson.version,
  converterRoot: packageRoot,
  upstreamZigbee2mqtt: gitHead(join(root, ".upstream/zigbee2mqtt")),
  upstreamConverters: gitHead(packageRoot),
  externalConverter: {
    path: externalPath,
    sha256: createHash("sha256").update(readFileSync(externalPath)).digest("hex"),
  },
  fixedClock: "2026-11-20T12:00:00Z",
  command: process.argv.join(" "),
};

mkdirSync(outDir, { recursive: true });
const results = [];
for (const workload of selected) {
  process.stderr.write(`stress ${workload.name} ... `);
  try {
    const result = await runWorkload(workload);
    results.push(result);
    process.stderr.write(
      `${result.throughputOpsPerSec.toFixed(0)} ops/s, p95 ${result.latencyMs.p95Ms.toFixed(2)} ms\n`,
    );
  } catch (error) {
    process.stderr.write(`ERROR ${error.message}\n`);
    results.push({ name: workload.name, error: String(error.stack ?? error) });
    process.exitCode = 1;
  }
}

writeFileSync(join(outDir, "stress.json"), `${JSON.stringify({ manifest, results }, null, 2)}\n`);
console.log(`wrote ${outDir}/stress.json`);
