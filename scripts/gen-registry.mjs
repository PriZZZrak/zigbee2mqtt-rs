/**
 * Build the on-demand converter registry on top of upstream models-index.json.
 *
 * Upstream resolution (src/index.ts @ pinned converters ref):
 *   1. getFromIndex(modelID): lookup MODELS_INDEX[modelID.toLowerCase()],
 *      fallback normalizeModelID() (strip NUL bytes + trim). Keys may be "null".
 *   2. Single candidate WITH zigbeeModel -> return it.
 *   3. Else fingerprint match wins (highest `priority`).
 *   4. Else first candidate whose zigbeeModel equals raw or normalized modelID.
 *   5. findByDevice() then applies whiteLabel fingerprint overrides.
 *
 * This script MUST NOT invent its own matching: it copies upstream's
 * models-index.json verbatim into `lookup`, then derives:
 *   - `models`:     model.toLowerCase() -> {file, index, vendor, model}
 *                     (+ whiteLabel models with whiteLabel:true + parent pointer)
 *   - `fingerprints`: [{file, index, model, vendor, fingerprint}]
 *                     (+ whiteLabel fingerprint entries)
 *   - `registry`:   legacy flat map (kept for the current Rust client):
 *                     exact/fuzzy keys -> file, but ONLY unambiguous entries:
 *                     zigbeeModel/fingerprint.modelID keys are included only
 *                     when their lookup entry has exactly ONE candidate.
 *                     "null" keys are never included.
 *
 * Any validation/smoke failure exits non-zero -> workflow publish is gated.
 */
import fs from "node:fs";
import path from "node:path";
import {createRequire} from "node:module";

const dist = process.argv[2] || "hc/dist";
const ref = process.argv[3] || "unknown";
const outDir = "herdsman-converters";

function fail(msg) {
    console.error(`ERROR: ${msg}`);
    process.exit(1);
}

// --- 1. Upstream index is the primary source (never regex) ---
const indexPath = path.join(dist, "models-index.json");
if (!fs.existsSync(indexPath)) {
    fail(`missing upstream index ${indexPath} (pnpm build must run node dist/indexer.js first)`);
}
const upstreamLookup = JSON.parse(fs.readFileSync(indexPath, "utf8"));
if (typeof upstreamLookup !== "object" || upstreamLookup === null) {
    fail(`upstream index ${indexPath} is not an object`);
}

const deviceDir = path.join(dist, "devices");
if (!fs.existsSync(deviceDir)) {
    fail(`missing device dir ${deviceDir}`);
}

// require() dist (CJS) files the same way upstream getDefinitions() imports them
const requireFromDist = createRequire(path.join(path.resolve(deviceDir), "dummy.cjs"));
const files = fs.readdirSync(deviceDir).filter((f) => f.endsWith(".js") && f !== "index.js");
if (files.length < 300) {
    fail(`expected 300+ device JS files in ${deviceDir}, got ${files.length}`);
}

// --- 2. Load every definition with its [file, index] coordinates ---
const definitionsByFile = new Map(); // file -> definitions[]
for (const f of files) {
    let mod;
    try {
        mod = requireFromDist(`./${f}`);
    } catch (e) {
        fail(`cannot require ${f}: ${e.message}`);
    }
    if (!Array.isArray(mod?.definitions)) {
        fail(`${f} does not export a definitions array`);
    }
    definitionsByFile.set(f, mod.definitions);
}

const models = {}; // model.lower -> {file, index, vendor, model, whiteLabel?}
const fingerprints = []; // {file, index, model, vendor, fingerprint}
let totalDefinitions = 0;
for (const [file, definitions] of definitionsByFile) {
    for (let i = 0; i < definitions.length; i++) {
        const d = definitions[i];
        totalDefinitions++;
        if (typeof d?.model !== "string" || typeof d?.vendor !== "string") {
            fail(`${file}[${i}] missing string model/vendor`);
        }
        const key = d.model.toLowerCase();
        if (models[key]) {
            fail(`duplicate model ${d.model} (${file}[${i}] vs ${models[key].file}[${models[key].index}])`);
        }
        models[key] = {file, index: i, vendor: d.vendor, model: d.model};
        for (const fp of d.fingerprint ?? []) {
            fingerprints.push({file, index: i, model: d.model, vendor: d.vendor, fingerprint: fp});
        }
        for (const wl of d.whiteLabel ?? []) {
            if ("fingerprint" in wl && Array.isArray(wl.fingerprint)) {
                for (const fp of wl.fingerprint) {
                    fingerprints.push({
                        file,
                        index: i,
                        model: wl.model,
                        vendor: wl.vendor ?? d.vendor,
                        whiteLabelOf: d.model,
                        fingerprint: fp,
                    });
                }
                const wlKey = String(wl.model).toLowerCase();
                if (!models[wlKey]) {
                    models[wlKey] = {
                        file,
                        index: i,
                        vendor: wl.vendor ?? d.vendor,
                        model: wl.model,
                        whiteLabel: true,
                        parent: d.model,
                    };
                }
            }
        }
    }
}

// --- 3. Validate every upstream lookup entry resolves to a real definition ---
for (const [lookupKey, candidates] of Object.entries(upstreamLookup)) {
    if (lookupKey === "null") continue;
    if (!Array.isArray(candidates) || candidates.length === 0) {
        fail(`lookup key ${JSON.stringify(lookupKey)} has no candidates`);
    }
    for (const [file, idx] of candidates) {
        const defs = definitionsByFile.get(file);
        if (!defs) fail(`lookup key ${JSON.stringify(lookupKey)} points at unknown file ${file}`);
        if (!defs[idx]) fail(`lookup key ${JSON.stringify(lookupKey)} points at ${file}[${idx}] which does not exist`);
    }
}

// --- 4. Legacy flat registry (compat only): unambiguous entries, never "null" ---
const registry = {};
const addUnique = (k, v) => {
    if (k === "null" || !k) return;
    if (!(k in registry)) registry[k] = v;
};
for (const [key, entry] of Object.entries(models)) {
    addUnique(key, entry.file);
    addUnique(`${entry.vendor}/${entry.model}`, entry.file);
    addUnique(`${entry.vendor.toLowerCase()}/${entry.model}`, entry.file);
    addUnique(`${entry.vendor.toLowerCase()}/${key}`, entry.file);
}
for (const [lookupKey, candidates] of Object.entries(upstreamLookup)) {
    if (lookupKey === "null") continue;
    // Mirror upstream single-candidate fast path ONLY when truly unambiguous.
    // Multi-candidate keys (incl. fingerprint-only "null"-style collisions)
    // stay resolvable exclusively through `lookup` + `fingerprints`.
    if (candidates.length === 1) {
        addUnique(lookupKey, candidates[0][0]);
    }
}

// --- 5. Smoke tests: publication is impossible without passing these ---
const lookupKeys = Object.keys(upstreamLookup);
if (lookupKeys.length < 5000) fail(`smoke: expected 5000+ lookup keys, got ${lookupKeys.length}`);
if (Object.keys(models).length < 3000) fail(`smoke: expected 3000+ models, got ${Object.keys(models).length}`);
if (fingerprints.length === 0) fail("smoke: expected non-empty fingerprints list");
const multi = lookupKeys.filter((k) => k !== "null" && upstreamLookup[k].length > 1);
if (multi.length === 0) fail("smoke: expected at least one multi-candidate lookup key");
for (const v of Object.values(registry)) {
    if (!definitionsByFile.has(v)) fail(`smoke: registry points at missing file ${v}`);
}
// Every definition file must contribute at least one model.
for (const f of files) {
    if (!Object.values(models).some((m) => m.file === f)) fail(`smoke: ${f} contributed no models`);
}

fs.mkdirSync(path.join(outDir, ref), {recursive: true});
const out = {
    ref,
    generated_at: new Date().toISOString(),
    lookup: upstreamLookup,
    models,
    fingerprints,
    registry,
};
fs.writeFileSync(path.join(outDir, ref, "registry.json"), JSON.stringify(out, null, 2));
console.log(
    `registry ${ref}: ${lookupKeys.length} lookup keys (${multi.length} multi-candidate), ` +
        `${Object.keys(models).length} models, ${fingerprints.length} fingerprints, ` +
        `${Object.keys(registry).length} legacy keys, ${totalDefinitions} definitions from ${files.length} files`,
);

// --- 6. ZCL enums for the Rust bridge (real values, never hand-copied) ---
// Converters reference Zcl.ManufacturerCode.* and cluster/attribute IDs at
// load time; the Rust CJS stub serves these exact values.
// NOTE: zspec lives in the zigbee-herdsman package (a converters dependency),
// NOT in converters dist — requiring ../zspec from dist/devices fails.
const requireHc = createRequire(path.join(path.resolve(dist, ".."), "package.json"));
const {Clusters} = requireHc("zigbee-herdsman/dist/zspec/zcl/definition/cluster.js");
const {ManufacturerCode} = requireHc("zigbee-herdsman/dist/zspec/zcl/definition/manufacturerCode.js");
const {DataType} = requireHc("zigbee-herdsman/dist/zspec/zcl/definition/enums.js");
const manufacturerCodes = {};
for (const [k, v] of Object.entries(ManufacturerCode)) {
    if (typeof v === "number") manufacturerCodes[k] = v;
}
if (Object.keys(manufacturerCodes).length < 100) fail(`smoke: expected 100+ manufacturer codes`);
if (manufacturerCodes.TEXAS_INSTRUMENTS !== 0x0007) fail("smoke: TEXAS_INSTRUMENTS != 0x0007");
const dataTypes = {};
for (const [k, v] of Object.entries(DataType)) {
    if (typeof v === "number") dataTypes[k] = v;
}
if (Object.keys(dataTypes).length < 20) fail(`smoke: expected 20+ data types`);
const clustersById = {};
const attributesByCluster = {};
for (const [name, cl] of Object.entries(Clusters)) {
    clustersById[String(cl.ID)] = name;
    const attrs = {};
    for (const [attrName, attr] of Object.entries(cl.attributes ?? {})) {
        attrs[String(attr.ID)] = attrName;
    }
    attributesByCluster[name] = attrs;
}
if (Object.keys(clustersById).length < 50) fail(`smoke: expected 50+ clusters`);
if (clustersById["513"] !== "hvacThermostat") fail("smoke: cluster 513 != hvacThermostat");
fs.writeFileSync(
    path.join(outDir, ref, "zcl.json"),
    JSON.stringify({ref, manufacturerCodes, dataTypes, clusters: clustersById, attributes: attributesByCluster}, null, 2),
);
// Attribute/command data types for converter-initiated writes and commands.
// Rust refuses to guess types, so this table must come from the pinned
// herdsman definition rather than from format guesses.
const clusterTypes = {};
for (const [name, cl] of Object.entries(Clusters)) {
    const attributes = {};
    for (const [attrName, attr] of Object.entries(cl.attributes ?? {})) {
        if (typeof attr.type !== "number") continue;
        attributes[String(attr.ID)] = {name: attrName, type: attr.type};
    }
    const commands = {};
    for (const [cmdName, cmd] of Object.entries(cl.commands ?? {})) {
        const parameters = (cmd.parameters ?? [])
            .filter((p) => typeof p.type === "number")
            .map((p) => ({name: p.name, type: p.type}));
        if (parameters.length !== (cmd.parameters ?? []).length) continue;
        commands[String(cmd.ID)] = {name: cmdName, parameters};
    }
    if (Object.keys(attributes).length === 0 && Object.keys(commands).length === 0) continue;
    clusterTypes[String(cl.ID)] = {name, attributes, commands};
}
if (Object.keys(clusterTypes).length < 50) fail(`smoke: expected 50+ typed clusters`);
if (clusterTypes["6"]?.commands?.["66"]?.parameters?.length !== 3) {
    fail("smoke: genOnOff.onWithTimedOff must have 3 typed parameters");
}
fs.writeFileSync(
    path.join(outDir, ref, "zcl-commands.json"),
    JSON.stringify({ref, clusters: clusterTypes}, null, 2),
);
console.log(
    `zcl-types ${ref}: ${Object.keys(clusterTypes).length} typed clusters ` +
        `(${Object.keys(clusterTypes["6"]?.attributes ?? {}).length} genOnOff attributes)`,
);
console.log(`zcl ${ref}: ${Object.keys(clustersById).length} clusters, ${Object.keys(manufacturerCodes).length} manufacturer codes`);
