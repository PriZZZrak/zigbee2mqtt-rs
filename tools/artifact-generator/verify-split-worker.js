#!/usr/bin/env node
"use strict";
// Prepare one side (original module or emitted fragments) in a fresh process,
// so shared expose objects cannot leak between the two comparisons.
const fs = require("fs");
const path = require("path");
function arg(name) { const index = process.argv.indexOf(name); return index >= 0 ? process.argv[index + 1] : undefined; }
const input = path.resolve(arg("--converters"));
const stage = path.resolve(arg("--stage"));
const fileName = arg("--file");
const side = arg("--side");
const fragmentFiles = JSON.parse(fs.readFileSync(arg("--fragments"), "utf8"));
const definitions = side === "original"
  ? require(path.join(input, "dist", "devices", fileName)).definitions
  : fragmentFiles.flatMap((file) => require(path.join(stage, "dist", "devices", file)).definitions);
const zhc = require(path.join(side === "original" ? input : stage, "dist", "index.js"));
function norm(definition) {
  let exposes = definition.exposes;
  if (typeof exposes === "function") exposes = "function";
  else { try { exposes = JSON.parse(JSON.stringify(exposes)); } catch { exposes = String(exposes); } }
  return {
    model: definition.model, vendor: definition.vendor, description: definition.description,
    zigbeeModel: definition.zigbeeModel, fingerprint: definition.fingerprint, whiteLabel: definition.whiteLabel,
    meta: definition.meta, endpoint: definition.endpoint, version: definition.version, exposes,
    from: (definition.fromZigbee || []).map((c) => [c.cluster, c.type]),
    to: (definition.toZigbee || []).map((c) => [c.key, c.endpoints]),
    options: (definition.options || []).map((option) => option.name),
    configure: typeof definition.configure, onEvent: typeof definition.onEvent,
  };
}
const output = definitions.map((definition) => {
  try { return norm(zhc.prepareDefinition(definition)); }
  catch (error) { return { error: String(error.message || error) }; }
});
fs.writeFileSync(arg("--out"), JSON.stringify(output));
console.log(JSON.stringify({ side, definitions: definitions.length }));
