#!/usr/bin/env node
// Production gates for a generated converter artifact.
// Usage: check-manifest-gates.js <artifact-dir>
const fs = require("node:fs");
const path = require("node:path");

const dir = process.argv[2];
if (!dir) {
  console.error("usage: check-manifest-gates.js <artifact-dir>");
  process.exit(2);
}
const manifest = JSON.parse(fs.readFileSync(path.join(dir, "manifest.json"), "utf8"));
const graph = JSON.parse(fs.readFileSync(path.join(dir, "graph.json"), "utf8"));
const errors = [];

if (manifest.manifestSchema !== 1) errors.push("manifestSchema must be 1");
const abi = manifest.host && manifest.host.abi;
if (!Number.isInteger(abi) || abi < 1) errors.push("host.abi missing");
if (!manifest.host || manifest.host.engine !== "quickjs-oxide") errors.push("unexpected engine");
if (!manifest.modules || !Array.isArray(manifest.modules.entries)) errors.push("modules.entries missing");
const retrieval = manifest.modules && manifest.modules.retrieval;
if (typeof retrieval !== "string" || !retrieval.startsWith("https://") || !retrieval.endsWith("/")) {
  errors.push("modules.retrieval must be an HTTPS base ending in /");
}

const ids = new Set();
const assets = new Set();
for (const module of manifest.modules.entries || []) ids.add(module.id);
for (const module of manifest.modules.entries || []) {
  if (!module.id || !module.file || !module.sha256) errors.push(`module ${module.id || "?"} missing fields`);
  if (!module.asset) errors.push(`module ${module.id} has no flat release asset name`);
  else {
    if (module.asset.includes("/") || module.asset.includes("..")) errors.push(`unsafe asset ${module.asset}`);
    if (assets.has(module.asset)) errors.push(`duplicate asset ${module.asset}`);
    assets.add(module.asset);
  }
  const file = path.join(dir, module.file);
  if (!fs.existsSync(file)) errors.push(`missing file ${module.file}`);
  else {
    const bytes = fs.readFileSync(file);
    if (bytes.length !== module.bytes) errors.push(`size mismatch ${module.file}`);
  }
  for (const dependency of module.requires || []) {
    if (!ids.has(dependency.replace(/\.js$/, ""))) {
      errors.push(`unresolved dependency ${module.id} -> ${dependency}`);
    }
  }
}

for (const [file, split] of Object.entries(manifest.split || {})) {
  if (split.status === "split") {
    if (!split.behavior || split.behavior.ok !== true) {
      errors.push(`split ${file}: behavior gate missing or failed (production forbids unverified splits)`);
    } else if (split.behavior.skipped === true || (split.behavior.compared | 0) === 0) {
      errors.push(`split ${file}: behavior gate skipped`);
    }
    if (!split.verification || split.verification.ok !== true) errors.push(`split ${file}: verification failed`);
  } else if (split.status === "fallback") {
    if (!split.reason) errors.push(`split ${file}: fallback without a reason`);
  } else {
    errors.push(`split ${file}: unknown status ${split.status}`);
  }
}

if (assets.size + 4 > 1000) errors.push(`too many release assets (${assets.size + 4} > 1000)`);

if (errors.length) {
  for (const error of errors) console.error(`gate: ${error}`);
  process.exit(1);
}
console.log(JSON.stringify({ ok: true, modules: ids.size, assets: assets.size + 4, retrieval }));
