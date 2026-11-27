#!/usr/bin/env node
// Verify a published converter Release against the pinned release descriptor.
// Usage: verify-release.js <release-dir> <pin.json> <expected-retrieval-base>
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");

const [dir, pinFile, expectedBase] = process.argv.slice(2);
if (!dir || !pinFile || !expectedBase) {
  console.error("usage: verify-release.js <release-dir> <pin.json> <expected-base>");
  process.exit(2);
}
const pin = JSON.parse(fs.readFileSync(pinFile, "utf8"));
const manifest = JSON.parse(fs.readFileSync(path.join(dir, "manifest.json"), "utf8"));
const sha256 = (bytes) => crypto.createHash("sha256").update(bytes).digest("hex");
const errors = [];
const check = (condition, message) => { if (!condition) errors.push(message); };

check(manifest.artifactId === pin.artifactId, `artifactId ${manifest.artifactId} != ${pin.artifactId}`);
check(manifest.host && manifest.host.abi === pin.hostAbi, "host.abi mismatch");
check(manifest.host && manifest.host.engine === pin.engine, "host.engine mismatch");
check(manifest.host && manifest.host.engineVersion === pin.engineVersion, "host.engineVersion mismatch");
check(manifest.upstream && manifest.upstream.zigbee2mqtt === pin.zigbee2mqtt, "z2m version mismatch");
check(manifest.upstream && manifest.upstream.zigbee2mqttCommit === pin.zigbee2mqttCommit, "z2m commit mismatch");
check(manifest.upstream && manifest.upstream.zigbeeHerdsman === pin.zigbeeHerdsman, "herdsman version mismatch");
check(manifest.upstream && manifest.upstream.zigbeeHerdsmanConverters === pin.zigbeeHerdsmanConverters, "converters version mismatch");
check(manifest.modules && manifest.modules.retrieval === expectedBase, `retrieval ${manifest.modules && manifest.modules.retrieval} != ${expectedBase}`);
check(manifest.compatibility && manifest.compatibility.minHostAbi === pin.hostAbi && manifest.compatibility.maxHostAbi === pin.hostAbi, "compatibility range mismatch");

const read = (name) => fs.readFileSync(path.join(dir, name));
const indexBytes = read("index.z2idx1");
check(indexBytes.length === manifest.index.bytes, "index size mismatch");
check(sha256(indexBytes) === manifest.index.sha256, "index sha256 mismatch");
const graphBytes = read("graph.json");
check(graphBytes.length === manifest.graph.bytes, "graph size mismatch");
check(sha256(graphBytes) === manifest.graph.sha256, "graph sha256 mismatch");
const indexJson = read("index.json");
check(indexJson.length === manifest.index.jsonBytes, "index.json size mismatch");
check(sha256(indexJson) === manifest.index.jsonSha256, "index.json sha256 mismatch");
const manifestBytes = read("manifest.json");

const sums = new Map();
for (const line of fs.readFileSync(path.join(dir, "SHA256SUMS"), "utf8").trim().split("\n")) {
  const match = line.match(/^([0-9a-f]{64})  (.+)$/);
  if (!match) { errors.push(`invalid SHA256SUMS line: ${line}`); continue; }
  sums.set(match[2], match[1]);
}
const expected = new Set(["manifest.json", "index.z2idx1", "index.json", "graph.json"]);
for (const module of manifest.modules.entries) {
  const asset = module.asset;
  check(Boolean(asset), `module ${module.id} has no asset`);
  check(!asset.includes("/") && !asset.includes(".."), `unsafe asset ${asset}`);
  check(!expected.has(asset), `duplicate asset ${asset}`);
  expected.add(asset);
  const bytes = read(asset);
  check(bytes.length === module.bytes, `module ${module.id} size mismatch`);
  check(sha256(bytes) === module.sha256, `module ${module.id} sha256 mismatch`);
  check(sums.get(asset) === module.sha256, `SHA256SUMS mismatch for ${asset}`);
}
for (const name of expected) {
  check(fs.existsSync(path.join(dir, name)), `missing release asset ${name}`);
  check(sums.has(name), `SHA256SUMS has no entry for ${name}`);
}
for (const name of sums.keys()) check(expected.has(name), `unexpected release asset ${name}`);
const actual = new Set(fs.readdirSync(dir).filter((name) => name !== "SHA256SUMS"));
for (const name of actual) check(expected.has(name), `unexpected file in release ${name}`);
check(actual.size === expected.size, `release has ${actual.size} assets, expected ${expected.size}`);

if (errors.length) {
  for (const error of errors) console.error(`release: ${error}`);
  process.exit(1);
}
console.log(JSON.stringify({
  ok: true,
  tag: pin.tag,
  artifactId: manifest.artifactId,
  abi: manifest.host.abi,
  modules: manifest.modules.entries.length,
  assets: actual.size + 1,
  manifestSha256: sha256(manifestBytes),
  indexSha256: manifest.index.sha256
}));
