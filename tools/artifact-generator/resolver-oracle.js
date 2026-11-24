#!/usr/bin/env node
"use strict";
// Generate resolver oracle cases from pinned upstream findByDevice().
const fs = require("fs");
const path = require("path");
function arg(name, fallback) { const i = process.argv.indexOf(name); return i >= 0 ? process.argv[i + 1] : fallback; }
const input = path.resolve(arg("--converters", ".upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters"));
const indexFile = path.resolve(arg("--index", "/tmp/opencode/artifact-test/index.json"));
const outFile = path.resolve(arg("--out", "/tmp/opencode/resolver-oracle.json"));
process.env.NODE_PATH = [path.dirname(input), path.join(path.dirname(input), ".pnpm", "node_modules")].join(path.delimiter);
require("module").Module._initPaths();
const zhc = require(path.join(input, "dist", "index.js"));
const index = JSON.parse(fs.readFileSync(indexFile, "utf8"));
const knownFiles = new Map();
for (const file of fs.readdirSync(path.join(input, "dist", "devices"))) {
  if (file.endsWith(".js") && file !== "index.js") {
    const mod = require(path.join(input, "dist", "devices", file));
    (mod.definitions || mod.default || []).forEach((definition, i) => knownFiles.set(`${definition.model}\0${file}\0${i}`, { file, index: i, definition }));
  }
}
// Find each index definition's original source using model + file/index mapping.
const sourceByArtifact = new Map();
for (const definition of index.definitions) {
  const key = `${definition.model}\0${definition.file}\0${definition.index}`;
  // The artifact may split files; locate by model and original file metadata
  // captured by the generator is not in the JSON. Models are unique upstream.
  let found = null;
  for (const candidate of knownFiles.values()) if (candidate.definition.model === definition.model) { found = candidate; break; }
  if (!found) throw new Error(`source definition not found: ${definition.model}`);
  sourceByArtifact.set(definition.model, found.definition);
}
function endpointProfile(endpoint) {
  return {
    ID: endpoint.ID,
    profileID: endpoint.profileID,
    deviceID: endpoint.deviceID,
    inputClusters: endpoint.inputClusters || [],
    outputClusters: endpoint.outputClusters || [],
  };
}
function profileFromFingerprint(fingerprint, fallbackModel) {
  const profile = {
    ieeeAddr: "0x00124b0024c16b95",
    type: fingerprint.type || "EndDevice",
    modelID: fingerprint.modelID !== undefined ? fingerprint.modelID : fallbackModel,
    manufacturerName: fingerprint.manufacturerName,
    manufacturerID: fingerprint.manufacturerID,
    powerSource: fingerprint.powerSource,
    applicationVersion: fingerprint.applicationVersion,
    stackVersion: fingerprint.stackVersion,
    hardwareVersion: fingerprint.hardwareVersion,
    zclVersion: fingerprint.zclVersion,
    dateCode: fingerprint.dateCode,
    softwareBuildID: fingerprint.softwareBuildID,
    endpoints: fingerprint.endpoints ? fingerprint.endpoints.map(endpointProfile) : [{ ID: 1, inputClusters: [], outputClusters: [] }],
  };
  for (const key of Object.keys(profile)) if (profile[key] === undefined) delete profile[key];
  return profile;
}
function normalizeProfile(profile) {
  const result = { ...profile };
  result.type = result.type || "EndDevice";
  result.endpoints = result.endpoints || [{ ID: 1, inputClusters: [], outputClusters: [] }];
  return result;
}
function deviceObject(profile) {
  const endpoints = (profile.endpoints || []).map((endpoint) => ({ ...endpoint }));
  return {
    ieeeAddr: profile.ieeeAddr,
    type: profile.type,
    modelID: profile.modelID,
    manufacturerName: profile.manufacturerName,
    manufacturerID: profile.manufacturerID,
    powerSource: profile.powerSource,
    applicationVersion: profile.applicationVersion,
    stackVersion: profile.stackVersion,
    hardwareVersion: profile.hardwareVersion,
    zclVersion: profile.zclVersion,
    dateCode: profile.dateCode,
    softwareBuildID: profile.softwareBuildID,
    endpoints,
    getEndpoint(id) { return endpoints.find((endpoint) => endpoint.ID === id); },
  };
}
function expected(definition) {
  if (!definition) return null;
  return { model: definition.model, vendor: definition.vendor, description: definition.description, version: definition.version };
}
(async () => {
  const seen = new Set();
  const cases = [];
  async function add(id, profile) {
    const normalized = normalizeProfile(profile);
    const key = JSON.stringify(normalized);
    if (seen.has(key)) return;
    seen.add(key);
    const device = deviceObject(normalized);
    let found;
    try { found = await zhc.findByDevice(device); } catch (error) { found = null; }
    cases.push({ id, profile: normalized, expected: expected(found) });
  }
  // 1. Every lookup key with a minimal model-only profile.
  for (const key of Object.keys(index.keys).sort()) {
    await add(`key:${key}`, { type: "EndDevice", modelID: key === "null" ? undefined : key, endpoints: [{ ID: 1 }] });
  }
  // 2. Every definition fingerprint.
  for (const definition of index.definitions) {
    const fallback = definition.zigbeeModel[0];
    for (const [fingerprintIndex, fingerprint] of definition.fingerprints.entries()) {
      await add(`fingerprint:${definition.model}:${fingerprintIndex}`, profileFromFingerprint(fingerprint, fallback));
      // Additional-field mutation: force a mismatch on a present field.
      if (fingerprint.manufacturerName !== undefined) await add(`mutate:manufacturer:${definition.model}:${fingerprintIndex}`, { ...profileFromFingerprint(fingerprint, fallback), manufacturerName: "__oracle_mismatch__" });
      if (fingerprint.modelID !== undefined) await add(`mutate:model:${definition.model}:${fingerprintIndex}`, { ...profileFromFingerprint(fingerprint, fallback), modelID: "__oracle_mismatch__" });
      if (fingerprint.type !== undefined) await add(`mutate:type:${definition.model}:${fingerprintIndex}`, { ...profileFromFingerprint(fingerprint, fallback), type: fingerprint.type === "Router" ? "EndDevice" : "Router" });
    }
  }
  // 3. White-label fingerprints.
  for (const definition of index.definitions) {
    for (const [whiteIndex, white] of definition.whiteLabel.entries()) {
      for (const [fingerprintIndex, fingerprint] of white.fingerprints.entries()) {
        await add(`whitelabel:${definition.model}:${white.model}:${whiteIndex}:${fingerprintIndex}`, profileFromFingerprint(fingerprint, definition.zigbeeModel[0]));
      }
    }
  }
  // 4. Explicit null model and additional device fields.
  await add("null:model", { type: "EndDevice", modelID: undefined });
  await add("extra:fields", { type: "Router", modelID: "TS0601", manufacturerName: "_TZE200_locansqn", manufacturerID: 4417, powerSource: "Battery", applicationVersion: 72, stackVersion: 0, hardwareVersion: 1, zclVersion: 3, dateCode: "", softwareBuildID: "", endpoints: [{ ID: 1, inputClusters: [0, 4, 5, 61184], outputClusters: [25, 10] }] });
  fs.writeFileSync(outFile, JSON.stringify({ cases }, null, 0));
  console.log(JSON.stringify({ cases: cases.length, bytes: fs.statSync(outFile).size, file: outFile }));
})().catch((error) => { console.error(error); process.exit(1); });
