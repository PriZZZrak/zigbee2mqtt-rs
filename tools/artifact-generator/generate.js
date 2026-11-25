#!/usr/bin/env node
"use strict";
// Deterministic converter artifact generator (phases 1-2 research/build tool).
// It reads the pinned zigbee-herdsman-converters dist tree and writes:
//   index.json, index.z2idx1, graph.json, manifest.json
//   dist/{index.js,converters,lib}, dist/devices/* (modules; split tuya)
// The generated package contains shared JS + index + manifest only; device
// modules are separate retrieval artifacts and are not part of the opkg.

const fs = require("fs");
const path = require("path");
const crypto = require("crypto");
const { createRequire } = require("module");

function arg(name, fallback) {
  const index = process.argv.indexOf(name);
  return index >= 0 && process.argv[index + 1] !== undefined ? process.argv[index + 1] : fallback;
}
function sha256(buffer) { return crypto.createHash("sha256").update(buffer).digest("hex"); }
function stat(file) { return fs.statSync(file).size; }
function write(file, data) { fs.mkdirSync(path.dirname(file), { recursive: true }); fs.writeFileSync(file, data); }
function jsonStable(value) { return JSON.stringify(value); }
function treeHash(entries) {
  return sha256(entries.map((entry) => `${entry.path}\0${entry.sha256}\n`).join(""));
}
function rel(root, file) { return path.relative(root, file).split(path.sep).join("/"); }

const input = path.resolve(arg("--converters", ".upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters"));
// pnpm keeps some converter-only dependencies in the virtual store.
const nodePath = [process.env.NODE_PATH, path.dirname(input), path.join(path.dirname(input), ".pnpm", "node_modules")].filter(Boolean).join(path.delimiter);
process.env.NODE_PATH = nodePath;
require("module").Module._initPaths();
const out = path.resolve(arg("--out", "target/converter-artifact"));
const artifactId = arg("--artifact-id", "z2hc-26.125.0-zh-11.2.0-abi1");
const z2mVersion = arg("--z2m-version", "2.14.3");
const z2mCommit = arg("--z2m-commit", "3c841fdfd66c92239921ef84fde8f039c64e1662");
const zhVersion = arg("--zh-version", "11.2.0");
const convertersVersion = arg("--converters-version", "26.125.0");
const nodeBaseline = arg("--node", "22.23.4");
const hostAbi = Number(arg("--host-abi", "1"));
const engine = arg("--engine", "quickjs-oxide");
const engineVersion = arg("--engine-version", "0.5.0");
const zclCatalog = path.resolve(arg("--zcl-catalog", "crates/zigbee-apiarist/src/zspec/zcl/definition/clusters.json"));
const splitTarget = Number(arg("--split-target", "8"));
const behaviorProbe = arg("--behavior-probe", "");
const allowUnverifiedSplit = arg("--allow-unverified-split", "0") === "1";
const splitFiles = new Set(arg("--split-files", "tuya.js").split(",").map((value) => value.trim()).filter(Boolean));
const retrievalUrl = arg("--retrieval-url", "https://artifact.example/z2hc/");
const splitMinBytes = Number(arg("--split-min-bytes", "300000"));
const babelPath = arg("--babel-parser", process.env.BABEL_PARSER || "");
let babel;
function parser() {
  if (babel) return babel;
  if (babelPath) { babel = require(path.resolve(babelPath)); return babel; }
  try {
    const requireFrom = createRequire(path.join(input, "package.json"));
    babel = requireFrom("@babel/parser");
  } catch (error) {
    throw new Error("AST parser unavailable: pass --babel-parser /path/to/@babel/parser");
  }
  return babel;
}

const dist = path.join(input, "dist");
const devicesDir = path.join(dist, "devices");
const modelsIndex = JSON.parse(fs.readFileSync(path.join(dist, "models-index.json"), "utf8"));

// ---- load definitions ----------------------------------------------------
const files = fs.readdirSync(devicesDir).filter((name) => name.endsWith(".js") && name !== "index.js").sort();
const fileDefs = new Map();
const definitionByFileIndex = new Map();
for (const name of files) {
  const full = path.join(devicesDir, name);
  const module = require(full);
  const definitions = module.definitions || module.default || [];
  if (!Array.isArray(definitions)) throw new Error(`definitions not an array: ${name}`);
  fileDefs.set(name, definitions);
  definitions.forEach((definition, index) => {
    if (!definition || typeof definition !== "object") throw new Error(`invalid definition ${name}:${index}`);
    if (!definition.model) throw new Error(`definition without model ${name}:${index}`);
    definitionByFileIndex.set(`${name}:${index}`, { definition, fileName: name, index });
  });
}

function regexToJson(value) {
  if (value instanceof RegExp) return { source: value.source, flags: value.flags };
  return undefined;
}
function fingerprintToJson(fingerprint) {
  if (!fingerprint || typeof fingerprint !== "object") throw new Error("invalid fingerprint");
  const out = {};
  for (const key of ["applicationVersion", "manufacturerID", "type", "dateCode", "hardwareVersion", "manufacturerName", "modelID", "powerSource", "softwareBuildID", "stackVersion", "zclVersion", "priority"]) {
    if (fingerprint[key] !== undefined) out[key] = fingerprint[key];
  }
  out.priority = fingerprint.priority === undefined ? 0 : fingerprint.priority;
  const ieee = regexToJson(fingerprint.ieeeAddr);
  if (ieee) out.ieeeAddr = ieee;
  if (fingerprint.endpoints) {
    out.endpoints = fingerprint.endpoints.map((endpoint) => {
      const result = {};
      for (const key of ["ID", "profileID", "deviceID", "inputClusters", "outputClusters"]) {
        if (endpoint[key] !== undefined) result[key] = endpoint[key];
      }
      return result;
    });
  }
  return out;
}
function definitionToJson(entry) {
  const definition = entry.definition;
  return {
    file: entry.fileName,
    index: entry.index,
    model: definition.model,
    vendor: definition.vendor || "",
    description: definition.description || "",
    version: definition.version || "0.0.0",
    zigbeeModel: definition.zigbeeModel ? (Array.isArray(definition.zigbeeModel) ? definition.zigbeeModel : [definition.zigbeeModel]) : [],
    fingerprints: (definition.fingerprint || []).map(fingerprintToJson),
    whiteLabel: (definition.whiteLabel || []).map((white) => ({
      model: white.model,
      vendor: white.vendor || undefined,
      description: white.description || undefined,
      fingerprints: (white.fingerprint || []).map(fingerprintToJson),
    })),
  };
}

// Definitions are ordered by file path, then definition index. Candidate order
// below comes from the pinned models-index, preserving upstream tie-breaking.
const definitionEntries = [];
for (const name of files) {
  const definitions = fileDefs.get(name);
  definitions.forEach((definition, index) => definitionEntries.push({ definition, fileName: name, index }));
}
const definitionIdByFileIndex = new Map();
definitionEntries.forEach((entry, id) => definitionIdByFileIndex.set(`${entry.fileName}:${entry.index}`, id));

// ---- split tuya ----------------------------------------------------------
function topLevelBindings(program, before) {
  const names = new Set();
  function addPattern(pattern) {
    if (!pattern) return;
    switch (pattern.type) {
      case "Identifier": names.add(pattern.name); break;
      case "ObjectPattern":
        for (const property of pattern.properties) {
          if (property.type === "RestElement") addPattern(property.argument);
          else addPattern(property.value);
        }
        break;
      case "ArrayPattern": for (const element of pattern.elements) addPattern(element); break;
      case "AssignmentPattern": addPattern(pattern.left); break;
      case "RestElement": addPattern(pattern.argument); break;
      default: break;
    }
  }
  for (const node of program.body) {
    if (node.start >= before) break;
    if (node.type === "VariableDeclaration") for (const declaration of node.declarations) addPattern(declaration.id);
    if ((node.type === "FunctionDeclaration" || node.type === "ClassDeclaration") && node.id) names.add(node.id.name);
  }
  return [...names].sort();
}
function findDefinitionsArray(ast) {
  for (const node of ast.program.body) {
    if (node.type !== "ExpressionStatement") continue;
    const expression = node.expression;
    if (expression.type !== "AssignmentExpression" || expression.operator !== "=") continue;
    const left = expression.left;
    if (left.type !== "MemberExpression" || left.object.type !== "Identifier" || left.object.name !== "exports" || left.property.type !== "Identifier" || left.property.name !== "definitions") continue;
    if (expression.right.type !== "ArrayExpression") continue;
    return { assignment: expression, array: expression.right };
  }
  throw new Error("exports.definitions assignment not found");
}
function splitFile(fileName, targetGroups) {
  const file = path.join(devicesDir, fileName);
  const source = fs.readFileSync(file, "utf8");
  const ast = parser().parse(source, { sourceType: "script", errorRecovery: false, ranges: false });
  const { assignment, array } = findDefinitionsArray(ast);
  const elements = array.elements;
  if (elements.some((element) => !element || !element.start || element.end === undefined)) throw new Error("non-literal definition element");
  const preamble = source.slice(0, assignment.start);
  const names = topLevelBindings(ast.program, assignment.start);
  const sizes = elements.map((element) => element.end - element.start);
  const total = sizes.reduce((sum, value) => sum + value, 0);
  const groups = Math.max(1, Math.min(targetGroups, elements.length));
  const target = total / groups;
  const partitions = [];
  let current = [], currentSize = 0;
  for (let index = 0; index < elements.length; index++) {
    current.push(index); currentSize += sizes[index];
    if (currentSize >= target && partitions.length < groups - 1) { partitions.push(current); current = []; currentSize = 0; }
  }
  if (current.length) partitions.push(current);
  return { source, elements, preamble, names, partitions };
}
function emitSplit(fileName, partitions, fileOutDir) {
  const split = splitFile(fileName, partitions);
  const stem = fileName.replace(/\.js$/, "");
  const commonName = `${stem}-common.js`;
  const commonSource = `${split.preamble}\nmodule.exports = {${split.names.join(",")}};\n`;
  const commonPath = path.join(fileOutDir, commonName);
  write(commonPath, commonSource);
  const fragments = [];
  split.partitions.forEach((indices, part) => {
    const fragmentName = `${stem}-${part}.js`;
    const body = indices.map((index) => {
      const element = split.elements[index];
      return split.source.slice(element.start, element.end);
    }).join(",");
    const source = `"use strict";\nconst {${split.names.join(",")}} = require("./${commonName}");\nexports.definitions = [${body}];\n`;
    write(path.join(fileOutDir, fragmentName), source);
    fragments.push({ file: fragmentName, source, indices, common: commonName, commonSource });
  });
  return { fileName, fragments, commonName, commonSource, elementCount: split.elements.length, preambleBytes: split.preamble.length };
}

// Split verification: exact definition text and prepared-definition equality.
function verifySplit(originalName, emitted, stageDir) {
  const fragmentFile = path.join(stageDir, "split-verify-fragments.json");
  const originalFile = path.join(stageDir, "split-verify-original.json");
  const fragmentsOut = path.join(stageDir, "split-verify-fragments-out.json");
  write(fragmentFile, JSON.stringify(emitted.fragments.map((fragment) => fragment.file)));
  const spawn = require("child_process").spawnSync;
  const run = (side, outFile) => spawn(process.execPath, [
    path.join(__dirname, "verify-split-worker.js"),
    "--converters", input,
    "--stage", stageDir,
    "--file", originalName,
    "--fragments", fragmentFile,
    "--side", side,
    "--out", outFile,
  ], { encoding: "utf8", env: process.env });
  const originalRun = run("original", originalFile);
  const fragmentsRun = run("fragments", fragmentsOut);
  let result;
  if (originalRun.status !== 0 || fragmentsRun.status !== 0) {
    result = { ok: false, reason: `verifier exited ${originalRun.status}/${fragmentsRun.status}: ${(originalRun.stderr || fragmentsRun.stderr || "").trim()}` };
  } else {
    const original = JSON.parse(fs.readFileSync(originalFile, "utf8"));
    const fragments = JSON.parse(fs.readFileSync(fragmentsOut, "utf8"));
    let differences = 0; const samples = [];
    const length = Math.max(original.length, fragments.length);
    for (let index = 0; index < length; index++) {
      if (JSON.stringify(original[index]) !== JSON.stringify(fragments[index])) {
        differences++;
        if (samples.length < 3) samples.push({ index, model: original[index] && original[index].model });
      }
    }
    result = { ok: differences === 0 && original.length === fragments.length, compared: original.length, fragments: fragments.length, differences, samples };
  }
  for (const file of [fragmentFile, originalFile, fragmentsOut]) fs.rmSync(file, { force: true });
  return result.ok ? result : { ...result, reason: result.reason || `${result.differences} prepared differences` };
}

// ---- stage shared tree and modules ---------------------------------------
fs.rmSync(out, { recursive: true, force: true });
const distOut = path.join(out, "dist");
const devicesOut = path.join(distOut, "devices");
fs.mkdirSync(devicesOut, { recursive: true });
// Verification-only module resolution: the staged dist requires the pinned
// top-level node_modules (zigbee-herdsman etc.). This symlink is removed before
// the artifact is considered complete and is never part of the manifest.
const verificationNodeModules = path.join(out, "node_modules");
if (!fs.existsSync(verificationNodeModules)) fs.symlinkSync(path.dirname(input), verificationNodeModules, "dir");
fs.copyFileSync(path.join(dist, "index.js"), path.join(distOut, "index.js"));
fs.copyFileSync(path.join(dist, "models-index.json"), path.join(distOut, "models-index.json"));
for (const directory of ["converters", "lib"]) {
  fs.cpSync(path.join(dist, directory), path.join(distOut, directory), { recursive: true });
  for (const entry of fs.readdirSync(path.join(distOut, directory), { withFileTypes: true })) {
    if (entry.isFile() && (entry.name.endsWith(".map") || entry.name.endsWith(".d.ts"))) fs.rmSync(path.join(distOut, directory, entry.name));
  }
}

// Modules: all device files except a successfully split file; fragments replace it.
const splitResults = {};
const replacedFiles = new Set();
function shouldSplit(fileName) {
  return stat(path.join(devicesDir, fileName)) >= splitMinBytes;
}
const splitCandidates = files.filter((fileName) => shouldSplit(fileName) && splitFiles.has(fileName));
for (const fileName of splitCandidates) {
  let emitted;
  try {
    const split = splitFile(fileName, splitTarget);
    emitted = emitSplit(fileName, splitTarget, devicesOut);
  } catch (error) {
    splitResults[fileName] = { status: "fallback", reason: `not split: ${error.message}` };
    continue;
  }
  // Verification uses the same stage layout as the artifact.
  const verification = verifySplit(fileName, emitted, out);
  let behavior = { ok: true, skipped: true, reason: "behavior probe not configured" };
  if (verification.ok && behaviorProbe) {
    const child = require("child_process").spawnSync(process.execPath, [
      path.join(__dirname, "verify-split-behavior.js"),
      "--converters", input,
      "--stage", out,
      "--file", fileName,
      "--probe", path.resolve(behaviorProbe),
      "--workdir", path.join(out, ".behavior-verify"),
    ], { encoding: "utf8", env: process.env, timeout: 180000 });
    try { behavior = JSON.parse((child.stdout || "").trim().split("\n").pop()); }
    catch (error) { behavior = { ok: false, reason: `behavior verifier output: ${(child.stdout || child.stderr || "").slice(0, 500)}` }; }
    fs.rmSync(path.join(out, ".behavior-verify"), { recursive: true, force: true });
  }
  const behaviorProven = (behavior.ok && behavior.skipped !== true) || allowUnverifiedSplit;
  if (verification.ok && behaviorProven) {
    replacedFiles.add(fileName);
    splitResults[fileName] = { status: "split", common: emitted.commonName, fragments: emitted.fragments.map((f) => ({ file: f.file, definitions: f.indices.length })), verification, behavior };
  } else {
    for (const fragment of emitted.fragments) fs.rmSync(path.join(devicesOut, fragment.file));
    fs.rmSync(path.join(devicesOut, emitted.commonName));
    splitResults[fileName] = {
      status: "fallback",
      reason: verification.reason || (behavior.skipped ? "behavioral differential required for split" : `${verification.differences} prepared differences`),
      behavior,
    };
    if (behavior.skipped && allowUnverifiedSplit) {
      splitResults[fileName].reason = "unverified split allowed explicitly";
    }
  }
}
for (const fileName of files) {
  if (replacedFiles.has(fileName)) continue;
  fs.copyFileSync(path.join(devicesDir, fileName), path.join(devicesOut, fileName));
}

// Map original (file,index) to artifact (file,index).
function artifactLocation(fileName, index) {
  const split = splitResults[fileName];
  if (split && split.status === "split") {
    let offset = 0;
    for (const fragment of split.fragments) {
      const module = require(path.join(devicesOut, fragment.file));
      const count = (module.definitions || []).length;
      if (index < offset + count) return { file: fragment.file, index: index - offset };
      offset += count;
    }
  }
  return { file: fileName, index };
}

// Rewrite the upstream model index to artifact locations (split files use
// fragment names/local indices). This is what lets normal resolution load only
// the selected module/fragment.
const artifactIndexLookup = {};
for (const entry of definitionEntries) {
  const location = artifactLocation(entry.fileName, entry.index);
  artifactIndexLookup[`${entry.fileName}:${entry.index}`] = [location.file, location.index];
}
const artifactModelsIndex = {};
for (const key of Object.keys(modelsIndex).sort()) {
  artifactModelsIndex[key] = modelsIndex[key].map(([fileName, index]) => artifactIndexLookup[`${fileName}:${index}`]);
}
write(path.join(distOut, "models-index.json"), JSON.stringify(artifactModelsIndex));

// ---- dependency graph ----------------------------------------------------
function parseRequires(file) {
  const source = fs.readFileSync(file, "utf8");
  const ast = parser().parse(source, { sourceType: "script", errorRecovery: false });
  const requires = [];
  function walk(node) {
    if (!node || typeof node !== "object") return;
    if (node.type === "CallExpression" && node.callee.type === "Identifier" && node.callee.name === "require" && node.arguments.length === 1 && node.arguments[0].type === "StringLiteral") {
      requires.push(node.arguments[0].value);
    }
    for (const key of Object.keys(node)) {
      const value = node[key];
      if (Array.isArray(value)) for (const child of value) walk(child);
      else if (value && typeof value === "object" && value.type) walk(value);
    }
  }
  walk(ast.program);
  return [...new Set(requires)].sort();
}
function resolveRelative(fromFile, spec) {
  if (!spec.startsWith(".")) return null;
  const candidate = path.resolve(path.dirname(fromFile), spec);
  for (const test of [candidate, `${candidate}.js`, path.join(candidate, "index.js"), `${candidate}.json`]) {
    if (fs.existsSync(test) && fs.statSync(test).isFile()) return test;
  }
  return null;
}
const graph = { nodes: {}, edges: {}, external: {} };
for (const fileName of fs.readdirSync(devicesOut).filter((name) => name.endsWith(".js")).sort()) {
  const full = path.join(devicesOut, fileName);
  const requires = parseRequires(full);
  graph.nodes[fileName] = { bytes: stat(full), sha256: sha256(fs.readFileSync(full)) };
  graph.edges[fileName] = [];
  for (const spec of requires) {
    const resolved = resolveRelative(full, spec);
    if (resolved && resolved.startsWith(distOut)) graph.edges[fileName].push(rel(distOut, resolved));
    else graph.external[spec] = (graph.external[spec] || 0) + 1;
  }
}
for (const directory of ["converters", "lib"]) {
  for (const fileName of fs.readdirSync(path.join(distOut, directory)).filter((name) => name.endsWith(".js")).sort()) {
    const key = `${directory}/${fileName}`;
    const full = path.join(distOut, key);
    const requires = parseRequires(full);
    graph.nodes[key] = { bytes: stat(full), sha256: sha256(fs.readFileSync(full)) };
    graph.edges[key] = [];
    for (const spec of requires) {
      const resolved = resolveRelative(full, spec);
      if (resolved && resolved.startsWith(distOut)) graph.edges[key].push(rel(distOut, resolved));
      else graph.external[spec] = (graph.external[spec] || 0) + 1;
    }
  }
}
graph.nodes["index.js"] = { bytes: stat(path.join(distOut, "index.js")), sha256: sha256(fs.readFileSync(path.join(distOut, "index.js"))) };
graph.edges["index.js"] = parseRequires(path.join(distOut, "index.js")).map((spec) => resolveRelative(path.join(distOut, "index.js"), spec)).filter((file) => file && file.startsWith(distOut)).map((file) => rel(distOut, file));

// ---- build index model ---------------------------------------------------
const definitions = [];
const definitionIdByOriginal = new Map();
for (const entry of definitionEntries) {
  const location = artifactLocation(entry.fileName, entry.index);
  const json = definitionToJson(entry);
  const id = definitions.length;
  definitions.push({
    file: location.file,
    index: location.index,
    model: json.model,
    vendor: json.vendor,
    description: json.description,
    version: json.version,
    zigbeeModel: json.zigbeeModel,
    fingerprints: json.fingerprints,
    whiteLabel: json.whiteLabel,
    originalFile: entry.fileName,
    originalIndex: entry.index,
  });
  definitionIdByOriginal.set(`${entry.fileName}:${entry.index}`, id);
}
// File table is every module in the artifact.
const moduleFiles = fs.readdirSync(devicesOut).filter((name) => name.endsWith(".js")).sort();
const fileRecords = moduleFiles.map((fileName) => {
  const full = path.join(devicesOut, fileName);
  const bytes = fs.readFileSync(full);
  return { path: `devices/${fileName}`, bytes: bytes.length, sha256: sha256(bytes) };
});
const fileId = new Map(fileRecords.map((record, index) => [record.path.replace(/^devices\//, ""), index]));
for (const definition of definitions) {
  if (!fileId.has(definition.file)) throw new Error(`definition file missing: ${definition.file}`);
  definition.fileId = fileId.get(definition.file);
}
// Candidate order is exactly the pinned models-index order.
const keys = {};
for (const key of Object.keys(modelsIndex).sort()) {
  const ids = [];
  for (const [fileName, index] of modelsIndex[key]) {
    const id = definitionIdByOriginal.get(`${fileName}:${index}`);
    if (id === undefined) throw new Error(`models-index entry missing: ${key} ${fileName}:${index}`);
    if (!ids.includes(id)) ids.push(id);
  }
  keys[key] = ids;
}

// ---- JSON index ----------------------------------------------------------
const indexModel = {
  format: "z2idx1",
  version: 1,
  normalization: "lowercase; normalizeModelID strips NUL suffix and trims",
  files: fileRecords,
  definitions: definitions.map((definition) => ({
    file: definition.fileId,
    index: definition.index,
    model: definition.model,
    vendor: definition.vendor,
    description: definition.description,
    version: definition.version,
    zigbeeModel: definition.zigbeeModel,
    fingerprints: definition.fingerprints,
    whiteLabel: definition.whiteLabel,
  })),
  keys,
};
const indexJson = jsonStable(indexModel);
write(path.join(out, "index.json"), indexJson);

// ---- binary index --------------------------------------------------------
function binaryWriter() {
  const chunks = [];
  return {
    u8(value) { const b = Buffer.alloc(1); b.writeUInt8(value); chunks.push(b); },
    u16(value) { const b = Buffer.alloc(2); b.writeUInt16LE(value); chunks.push(b); },
    u32(value) { const b = Buffer.alloc(4); b.writeUInt32LE(value >>> 0); chunks.push(b); },
    i32(value) { const b = Buffer.alloc(4); b.writeInt32LE(value); chunks.push(b); },
    u64(value) { const b = Buffer.alloc(8); b.writeBigUInt64LE(BigInt(value)); chunks.push(b); },
    bytes(value) { chunks.push(Buffer.from(value)); },
    result() { return Buffer.concat(chunks); },
  };
}
const stringValues = new Set();
function collectStrings(value) {
  if (typeof value === "string") stringValues.add(value);
  else if (Array.isArray(value)) for (const item of value) collectStrings(item);
  else if (value && typeof value === "object") {
    for (const [key, item] of Object.entries(value)) { stringValues.add(key); collectStrings(item); }
  }
}
collectStrings(indexModel);
// Remove non-string sentinels.
const strings = [...stringValues].sort();
const stringId = new Map(strings.map((value, index) => [value, index]));
const NONE = 0xffffffff;
function sid(value) {
  if (value === undefined || value === null) return NONE;
  const id = stringId.get(String(value));
  if (id === undefined) throw new Error(`string missing from table: ${JSON.stringify(value)}`);
  return id;
}
function writeFingerprint(writer, fingerprint) {
  writer.u32(sid(fingerprint.modelID));
  writer.u32(sid(fingerprint.manufacturerName));
  writer.u32(fingerprint.manufacturerID === undefined ? NONE : fingerprint.manufacturerID);
  writer.u32(sid(fingerprint.type));
  writer.u32(sid(fingerprint.powerSource));
  writer.u32(fingerprint.applicationVersion === undefined ? NONE : fingerprint.applicationVersion);
  writer.u32(fingerprint.zclVersion === undefined ? NONE : fingerprint.zclVersion);
  writer.u32(fingerprint.stackVersion === undefined ? NONE : fingerprint.stackVersion);
  writer.u32(fingerprint.hardwareVersion === undefined ? NONE : fingerprint.hardwareVersion);
  writer.u32(sid(fingerprint.dateCode));
  writer.u32(sid(fingerprint.softwareBuildID));
  writer.i32(fingerprint.priority === undefined ? 0 : fingerprint.priority);
  writer.u32(fingerprint.ieeeAddr ? sid(fingerprint.ieeeAddr.source) : NONE);
  writer.u32(fingerprint.ieeeAddr ? sid(fingerprint.ieeeAddr.flags) : NONE);
  const endpoints = fingerprint.endpoints;
  if (endpoints === undefined) writer.u32(NONE);
  else writer.u32(endpoints.length);
  for (const endpoint of endpoints || []) {
    writer.u32(endpoint.ID === undefined ? NONE : endpoint.ID);
    writer.u32(endpoint.profileID === undefined ? NONE : endpoint.profileID);
    writer.u32(endpoint.deviceID === undefined ? NONE : endpoint.deviceID);
    if (endpoint.inputClusters === undefined) writer.u16(0xffff);
    else { writer.u16(endpoint.inputClusters.length); for (const cluster of endpoint.inputClusters) writer.u16(cluster); }
    if (endpoint.outputClusters === undefined) writer.u16(0xffff);
    else { writer.u16(endpoint.outputClusters.length); for (const cluster of endpoint.outputClusters) writer.u16(cluster); }
  }
}
const writer = binaryWriter();
writer.bytes(Buffer.from("Z2IDX1", "ascii"));
writer.u16(1); writer.u8(0); writer.u8(0);
writer.u32(strings.length); writer.u32(fileRecords.length); writer.u32(definitions.length); writer.u32(Object.keys(keys).length); 
writer.u32(Object.values(keys).reduce((sum, ids) => sum + ids.length, 0));
for (const value of strings) { const bytes = Buffer.from(value, "utf8"); writer.u32(bytes.length); writer.bytes(bytes); }
for (const file of fileRecords) { writer.u32(sid(file.path)); writer.u64(file.bytes); writer.bytes(Buffer.from(file.sha256, "hex")); }
for (const definition of indexModel.definitions) {
  writer.u32(definition.file); writer.u32(definition.index);
  writer.u32(sid(definition.model)); writer.u32(sid(definition.vendor)); writer.u32(sid(definition.description)); writer.u32(sid(definition.version));
  writer.u16(definition.zigbeeModel.length); for (const model of definition.zigbeeModel) writer.u32(sid(model));
  writer.u16(definition.fingerprints.length); for (const fingerprint of definition.fingerprints) writeFingerprint(writer, fingerprint);
  writer.u16(definition.whiteLabel.length);
  for (const white of definition.whiteLabel) {
    writer.u32(sid(white.model)); writer.u32(sid(white.vendor)); writer.u32(sid(white.description));
    writer.u16(white.fingerprints.length); for (const fingerprint of white.fingerprints) writeFingerprint(writer, fingerprint);
  }
}
for (const key of Object.keys(keys).sort()) {
  writer.u32(sid(key)); const ids = keys[key]; writer.u32(ids.length); for (const id of ids) writer.u32(id);
}
const binary = writer.result();
write(path.join(out, "index.z2idx1"), binary);

// ---- manifest and graph --------------------------------------------------
const graphJson = jsonStable(graph);
write(path.join(out, "graph.json"), graphJson);
const sharedRoot = distOut;
const sharedFiles = [];
function collectShared(directory) {
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const full = path.join(directory, entry.name);
    if (entry.isDirectory()) collectShared(full);
    else if (entry.isFile() && entry.name.endsWith(".js")) {
      const bytes = fs.readFileSync(full);
      sharedFiles.push({ path: rel(sharedRoot, full), bytes: bytes.length, sha256: sha256(bytes) });
    }
  }
}
collectShared(path.join(distOut, "converters"));
collectShared(path.join(distOut, "lib"));
for (const fileName of ["index.js"]) {
  const bytes = fs.readFileSync(path.join(distOut, fileName));
  sharedFiles.push({ path: fileName, bytes: bytes.length, sha256: sha256(bytes) });
}
sharedFiles.sort((a, b) => a.path.localeCompare(b.path));
const moduleRecords = [];
for (const fileName of moduleFiles) {
  const bytes = fs.readFileSync(path.join(devicesOut, fileName));
  const count = (require(path.join(devicesOut, fileName)).definitions || []).length;
  const requires = (graph.edges[fileName] || [])
    .filter((dependency) => dependency.startsWith("devices/"))
    .map((dependency) => dependency.slice("devices/".length));
  moduleRecords.push({ id: fileName.replace(/\.js$/, ""), file: `dist/devices/${fileName}`, bytes: bytes.length, sha256: sha256(bytes), definitions: count, requires });
}
const splitManifest = {};
for (const [fileName, result] of Object.entries(splitResults)) {
  splitManifest[fileName] = {
    status: result.status,
    common: result.common || null,
    fragments: result.fragments || [],
    verification: result.verification || null,
    behavior: result.behavior || null,
    reason: result.reason || null,
  };
}
const zclBytes = fs.readFileSync(zclCatalog);
const manifest = {
  manifestSchema: 1,
  artifactId,
  upstream: { zigbee2mqtt: z2mVersion, zigbee2mqttCommit: z2mCommit, zigbeeHerdsman: zhVersion, zigbeeHerdsmanConverters: convertersVersion, nodeBaseline },
  host: { abi: hostAbi, engine, engineVersion, zclCatalogSha256: sha256(zclBytes), requiredFeatures: ["cjs", "timers", "host-zcl-v1", "async-host"] },
  index: { file: "index.z2idx1", format: "z2idx1", version: 1, bytes: binary.length, sha256: sha256(binary), jsonBytes: Buffer.byteLength(indexJson), jsonSha256: sha256(indexJson), lookupKeys: Object.keys(keys).length, definitions: definitions.length, candidates: Object.values(keys).reduce((sum, ids) => sum + ids.length, 0) },
  graph: { file: "graph.json", bytes: graphJson.length, sha256: sha256(graphJson) },
  shared: { root: "dist", files: sharedFiles, bytes: sharedFiles.reduce((sum, file) => sum + file.bytes, 0), sha256: treeHash(sharedFiles) },
  modules: { format: "cjs", packageIncluded: false, retrieval: retrievalUrl, entries: moduleRecords },
  split: splitManifest,
  cache: { layout: "{artifactId}/generations/<generation>/", activePointer: "active.json", rollbackPointer: "lkg.json", pairedOffline: true, verify: ["sha256", "host.abi", "requiredFeatures"] },
  compatibility: { minHostAbi: hostAbi, maxHostAbi: hostAbi, unknownInbound: "raw-diagnostic", unknownOutbound: "error" },
  signature: { type: "sha256", hashOnly: true, value: null, note: "hash of shared/index/module artifacts only; not a cryptographic signature" },
};
write(path.join(out, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n");

fs.rmSync(verificationNodeModules, { force: true, recursive: true });
console.log(JSON.stringify({
  artifactId,
  definitions: definitions.length,
  keys: Object.keys(keys).length,
  candidates: Object.values(keys).reduce((sum, ids) => sum + ids.length, 0),
  indexJsonBytes: indexJson.length,
  indexBinaryBytes: binary.length,
  sharedBytes: manifest.shared.bytes,
  modules: moduleRecords.length,
  split: splitManifest,
}, null, 2));
