#!/usr/bin/env node
// Stage flat GitHub release assets from a verified artifact directory.
// Usage: stage-release-assets.js <artifact-dir> <out-dir>
// Copies manifest/index/graph and every module under its flat `asset` name,
// verifying size and sha256 against the manifest before writing.
const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");

const artifact = process.argv[2];
const out = process.argv[3];
if (!artifact || !out) {
  console.error("usage: stage-release-assets.js <artifact-dir> <out-dir>");
  process.exit(2);
}
const manifest = JSON.parse(fs.readFileSync(path.join(artifact, "manifest.json"), "utf8"));
fs.mkdirSync(out, { recursive: true });
const sha256 = (bytes) => crypto.createHash("sha256").update(bytes).digest("hex");
const copied = [];
for (const name of ["manifest.json", "index.z2idx1", "index.json", "graph.json"]) {
  const source = path.join(artifact, name);
  if (!fs.existsSync(source)) throw new Error(`missing ${name}`);
  fs.copyFileSync(source, path.join(out, name));
  copied.push(name);
}
for (const module of manifest.modules.entries) {
  const asset = module.asset;
  if (!asset || asset.includes("/") || asset.includes("..")) throw new Error(`unsafe asset ${asset}`);
  const bytes = fs.readFileSync(path.join(artifact, module.file));
  if (bytes.length !== module.bytes) throw new Error(`size mismatch ${module.id}`);
  if (sha256(bytes) !== module.sha256) throw new Error(`sha256 mismatch ${module.id}`);
  const target = path.join(out, asset);
  if (fs.existsSync(target)) throw new Error(`duplicate asset ${asset}`);
  fs.writeFileSync(target, bytes);
  copied.push(asset);
}
if (copied.length > 1000) throw new Error(`too many release assets: ${copied.length}`);
process.stdout.write(JSON.stringify({ assets: copied.length, files: copied }) + "\n");
