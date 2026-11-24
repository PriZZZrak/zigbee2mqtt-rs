#!/usr/bin/env node
"use strict";
// Behavioral differential for a tuya split: run representative fromZigbee,
// set, configure and onEvent scenarios through the real converter runtime on
// the original package and on the staged split package. Timing fields are
// ignored; result/published/host-call order must match.
const fs = require("fs");
const path = require("path");
const cp = require("child_process");
function arg(name, fallback) { const i = process.argv.indexOf(name); return i >= 0 ? process.argv[i + 1] : fallback; }
const input = path.resolve(arg("--converters"));
const stage = path.resolve(arg("--stage"));
const file = arg("--file", "tuya.js");
const probe = path.resolve(arg("--probe"));
const work = path.resolve(arg("--workdir", "/tmp/z2m-split-behavior"));
if (file !== "tuya.js") {
  console.log(JSON.stringify({ ok: true, skipped: true, reason: "no behavioral corpus for this module" }));
  process.exit(0);
}
const map = {};
for (const fragment of fs.readdirSync(path.join(stage, "dist", "devices")).filter((name) => /^tuya-\d+\.js$/.test(name)).sort()) {
  const defs = require(path.join(stage, "dist", "devices", fragment)).definitions;
  for (let index = 0; index < defs.length; index++) map[defs[index].model] = { file: fragment, index };
}
const goldenPath = path.resolve("tools/bench/corpus/scenarios.json");
const scenarios = fs.existsSync(goldenPath) ? JSON.parse(fs.readFileSync(goldenPath, "utf8")) : [];
const byId = Object.fromEntries(scenarios.map((scenario) => [scenario.id, scenario]));
function clone(id, file) { const value = JSON.parse(JSON.stringify(byId[id])); value.definition.file = file; return value; }
function scenario(kind, model, file, extra) {
  const value = {
    id: `${kind}-${model}`,
    kind,
    definition: { source: "package", file, model },
    device: { ieeeAddr: "0x0017880104e45510", networkAddress: 4096, modelID: model, manufacturerName: "Bench", genBasic: { swBuildId: "1.2.3" }, endpoints: [{ id: 1, inputClusters: [0, 3, 4, 5, 6, 8, 61184], outputClusters: [25] }] },
    endpointId: 1, state: {}, options: {},
  };
  return Object.assign(value, extra || {});
}
const original = [], split = [];
for (const id of ["fz-ts0002-ep2-onoff", "set-ts0002-ep2-onoff", "fz-tuya-m9pro-multi-dp"]) {
  if (!byId[id]) continue;
  const model = byId[id].definition.model;
  if (!map[model]) continue;
  original.push(clone(id, "tuya.js"));
  split.push(clone(id, map[model].file));
}
for (const model of ["CTL-Mini-DTP-TYZ/AC", "MG-DIM02Z"]) {
  if (!map[model]) continue;
  original.push(scenario("configure", model, "tuya.js"));
  split.push(scenario("configure", model, map[model].file));
  original.push(scenario("onEvent", model, "tuya.js", { eventType: "start" }));
  split.push(scenario("onEvent", model, map[model].file, { eventType: "start" }));
}
if (!original.length) { console.log(JSON.stringify({ ok: true, skipped: true, reason: "no representative tuya scenarios" })); process.exit(0); }
fs.mkdirSync(work, { recursive: true });
const origFile = path.join(work, "original.json"), splitFile = path.join(work, "split.json");
fs.writeFileSync(origFile, JSON.stringify(original)); fs.writeFileSync(splitFile, JSON.stringify(split));
function run(packageRoot, scenariosFile) {
  const herdman = path.join(path.dirname(input), "zigbee-herdsman");
  const result = cp.spawnSync(probe, ["--mode", "all", "--warmup", "0", "--jobs", "1", "--max-concurrent", "1", "--timeout-ms", "30000", "--package", packageRoot, "--herdsman", herdman, "--scenarios", scenariosFile], { encoding: "utf8", timeout: 120000 });
  if (result.status !== 0) throw new Error(`probe failed: ${result.status} ${(result.stderr || "").slice(0, 500)}`);
  try { return JSON.parse(result.stdout.slice(result.stdout.indexOf("\n{") + 1)); }
  catch (error) { throw new Error(`probe JSON: ${result.stdout.slice(0, 300)}`); }
}
let a, b;
try { a = run(input, origFile); b = run(stage, splitFile); }
catch (error) { console.log(JSON.stringify({ ok: false, reason: error.message })); process.exit(0); }
function functional(result) {
  const body = result.result || {};
  return { id: result.scenarioId, state: result.state, result: body.result, published: body.published, calls: body.calls };
}
const differences = [];
const ai = Object.fromEntries((a.cases || []).map((caseValue) => [caseValue.scenarioId, caseValue]));
for (const caseValue of b.cases || []) {
  const left = ai[caseValue.scenarioId];
  if (!left) { differences.push({ id: caseValue.scenarioId, reason: "missing original" }); continue; }
  if (JSON.stringify(functional(left)) !== JSON.stringify(functional(caseValue))) differences.push({ id: caseValue.scenarioId, original: functional(left), split: functional(caseValue) });
}
const ok = differences.length === 0 && (a.failed || 0) === (b.failed || 0);
console.log(JSON.stringify({ ok, compared: original.length, originalFailed: a.failed, splitFailed: b.failed, differences: differences.slice(0, 5) }));
process.exit(0);
