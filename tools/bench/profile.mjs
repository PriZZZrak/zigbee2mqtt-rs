// Summarizes a V8 `.cpuprofile` by self time.
//
// Usage: node tools/bench/profile.mjs <profile.cpuprofile> [top]
import { readFileSync, writeFileSync } from "node:fs";

const file = process.argv[2];
const top = Number(process.argv[3] ?? 20);
if (!file) {
  console.error("usage: node tools/bench/profile.mjs <profile.cpuprofile> [top]");
  process.exit(1);
}
const profile = JSON.parse(readFileSync(file, "utf8"));
const nodes = new Map(profile.nodes.map((node) => [node.id, node]));
const self = new Map();
let total = 0;
for (let index = 0; index < profile.samples.length; index++) {
  const id = profile.samples[index];
  const delta = profile.timeDeltas[index] ?? 0;
  const node = nodes.get(id);
  if (!node) continue;
  total += delta;
  const key = `${node.callFrame.functionName || "(anonymous)"} ${node.callFrame.url || ""}`;
  self.set(key, (self.get(key) ?? 0) + delta);
}
const rows = [...self.entries()]
  .sort((left, right) => right[1] - left[1])
  .slice(0, top)
  .map(([key, microseconds]) => ({
    function: key,
    selfMs: microseconds / 1000,
    percent: total > 0 ? (microseconds / total) * 100 : 0,
  }));
const summary = {
  file,
  totalMs: total / 1000,
  top: rows,
};
writeFileSync(file.replace(/\.cpuprofile$/, ".summary.json"), `${JSON.stringify(summary, null, 2)}\n`);
console.log(JSON.stringify(summary, null, 2));
