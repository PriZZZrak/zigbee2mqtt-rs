#!/usr/bin/env node
// EVID-04 Node reference driver (pinned zigbee-herdsman-converters).
//
// Reads a fixture JSON object on stdin and prints JSON:
//   - mode "fromZigbee" (default): resolves the definition through the upstream
//     models index, prepares modern extends exactly like upstream and runs the
//     matching fromZigbee converters for the synthetic message.
//   - mode "convertSet": runs the matching toZigbee converter with a recording
//     entity stub and prints {state, ops}.
//
// Environment: HC_CONVERTERS_DIST must point at the converters `dist` directory
// of the app-resolved package (`.upstream/zigbee2mqtt/node_modules/.../dist`).
"use strict";

const fs = require("node:fs");
const path = require("node:path");
const {createRequire} = require("node:module");

function fail(message) {
    console.error(`evid04-node-reference: ${message}`);
    process.exit(2);
}

const dist = process.env.HC_CONVERTERS_DIST;
if (!dist) fail("HC_CONVERTERS_DIST is not set");
const requireFromConverters = createRequire(path.join(dist, "..", "package.json"));

function normalize(value) {
    if (value === null || value === undefined) return value;
    if (value instanceof Uint8Array || Array.isArray(value)) {
        return Array.from(value, normalize);
    }
    if (typeof value === "object") {
        const out = {};
        for (const [key, item] of Object.entries(value)) out[key] = normalize(item);
        return out;
    }
    return value;
}

function readInput() {
    return new Promise((resolve, reject) => {
        let raw = "";
        process.stdin.setEncoding("utf8");
        process.stdin.on("data", (chunk) => {
            raw += chunk;
        });
        process.stdin.on("error", reject);
        process.stdin.on("end", () => {
            try {
                resolve(JSON.parse(raw));
            } catch (error) {
                reject(new Error(`invalid fixture JSON: ${error.message}`));
            }
        });
    });
}

function selectDefinition(input) {
    const index = JSON.parse(fs.readFileSync(path.join(dist, "models-index.json"), "utf8"));
    const model = String(input.model || "")
        .toLowerCase()
        .replace(/\0(.|\n)*$/g, "")
        .trim();
    const candidates = index[model === "" ? "null" : model];
    if (!Array.isArray(candidates) || candidates.length === 0) {
        fail(`no models-index candidates for '${model}'`);
    }
    const load = ([file, at]) => {
        const definitions = require(path.join(dist, "devices", file)).definitions;
        const definition = definitions[at];
        if (!definition) fail(`missing definition ${file}[${at}]`);
        return {file, at, definition};
    };
    if (candidates.length === 1) return load(candidates[0]);

    // Mirror the generated registry's fingerprint selection: highest priority
    // match wins; ties keep candidate order; otherwise the first candidate.
    const registryPath = process.env.EVID04_REGISTRY;
    if (!registryPath || !fs.existsSync(registryPath)) {
        return load(candidates[0]);
    }
    const registry = JSON.parse(fs.readFileSync(registryPath, "utf8"));
    let best = null;
    for (const candidate of candidates) {
        for (const entry of registry.fingerprints || []) {
            if (entry.file !== candidate[0] || entry.index !== candidate[1]) continue;
            const fp = entry.fingerprint || {};
            const matches =
                (fp.modelID === undefined || fp.modelID === input.model) &&
                (fp.manufacturerName === undefined ||
                    fp.manufacturerName === input.manufacturer_name) &&
                (fp.manufacturerID === undefined ||
                    fp.manufacturerID === input.manufacturer_id) &&
                (fp.powerSource === undefined || fp.powerSource === input.power_source) &&
                (fp.type === undefined || fp.type === input.device_type);
            if (!matches) continue;
            const priority = fp.priority || 0;
            if (!best || priority > best.priority) best = {candidate, priority};
        }
    }
    return load(best ? best.candidate : candidates[0]);
}

async function main() {
    const input = await readInput();
    if (typeof input.model !== "string") fail("fixture is missing 'model'");
    const clusterId = Number(input.cluster_id);
    if (!Number.isInteger(clusterId)) fail("fixture is missing a valid 'cluster_id'");
    const endpoint = Number.isInteger(input.endpoint) ? input.endpoint : 1;
    const msgType = input.msg_type || "attributeReport";
    const {Clusters} = requireFromConverters("zigbee-herdsman/dist/zspec/zcl/definition/cluster.js");
    const clusters = requireFromConverters("zigbee-herdsman/dist/zspec/zcl/definition/cluster.js");
    const clusterName = Object.values(Clusters).find((cluster) => cluster.ID === clusterId)?.name;
    if (!clusterName) fail(`unknown cluster id ${clusterId}`);

    const selected = selectDefinition(input);
    if (process.env.EVID04_DEBUG) console.error(`selected ${selected.file}[${selected.at}]`);
    const zhc = require(path.join(dist, "index.js"));
    const definition = await zhc.prepareDefinition(selected.definition);
    const meta = {
        state: input.state && typeof input.state === "object" ? input.state : {},
        device: {
            modelID: input.model,
            manufacturerName: input.manufacturer_name,
            ieeeAddr: input.ieee_addr || "0x00124b0000000000",
            endpoints: [{ID: 1}, {ID: 2}, {ID: 3}, {ID: 4}],
            getEndpoint: (id) => ({ID: id}),
        },
        options: {},
        message: {},
        endpoint_name: input.endpoint_name,
    };

    const mode = input.mode || "fromZigbee";
    if (mode === "fromZigbee") {
        let data;
        if (input.data && typeof input.data === "object") {
            data = normalize(input.data);
        } else {
            if (typeof input.attribute !== "string") fail("fixture is missing 'attribute' or 'data'");
            if (input.value === undefined) fail("fixture is missing 'value'");
            data = {[input.attribute]: input.value};
        }
        const msg = {
            cluster: clusterName,
            type: msgType,
            data,
            endpoint: {ID: endpoint},
            linkquality: input.linkquality === undefined ? 0 : input.linkquality,
            groupID: 0,
        };
        msg.endpoint.constructor = {name: "Endpoint"};
        Object.assign(msg.endpoint, {
            command: () => Promise.resolve(),
            read: () => Promise.resolve(),
            write: () => Promise.resolve(),
        });
        meta.device.constructor = {name: "Device"};
        msg.device = meta.device;
        const out = {};
        let matched = false;
        for (const converter of definition.fromZigbee || []) {
            const types = Array.isArray(converter.type) ? converter.type : [converter.type];
            if (converter.cluster !== clusterName && String(converter.cluster) !== String(clusterId)) continue;
            if (!types.includes(msgType)) continue;
            matched = true;
            try {
                const result = await converter.convert(definition, msg, (publish) => Object.assign(out, publish), {}, meta);
                if (result && typeof result === "object") Object.assign(out, result);
            } catch (error) {
                fail(`converter threw: ${(error && error.stack) || error}`);
            }
        }
        if (!matched) fail(`no fromZigbee converter matched ${clusterName}/${msgType}`);
        process.stdout.write(`${JSON.stringify(normalize(out))}\n`);
        return;
    }

    if (mode === "convertSet") {
        const key = input.key;
        if (typeof key !== "string") fail("convertSet fixture is missing 'key'");
        if (input.value === undefined) fail("convertSet fixture is missing 'value'");
        const clusterIdOf = (cluster) => {
            if (cluster && cluster.ID !== undefined) return cluster.ID;
            if (typeof cluster === "number") return cluster;
            return clusters.Clusters[cluster] ? clusters.Clusters[cluster].ID : cluster;
        };
        const ops = [];
        const optionsOf = (options) => ({
            manufacturerCode: options && options.manufacturerCode !== undefined ? options.manufacturerCode : null,
            disableDefaultResponse: !!(options && options.disableDefaultResponse),
        });
        const entity = {
            ID: endpoint,
            deviceIeeeAddress: meta.device.ieeeAddr,
            getDevice: () => meta.device,
            getEndpoint: () => entity,
            write: (cluster, attributes, options) => {
                ops.push({op: "write", cluster: clusterIdOf(cluster), attributes: normalize(attributes) || {}, options: optionsOf(options)});
                return Promise.resolve();
            },
            command: (cluster, command, payload, options) => {
                const cl = clusterIdOf(cluster);
                const definitionCluster = Object.values(clusters.Clusters).find((c) => c.ID === cl);
                const commandId =
                    typeof command === "number"
                        ? command
                        : definitionCluster && definitionCluster.commands[command]
                          ? definitionCluster.commands[command].ID
                          : command;
                ops.push({op: "command", cluster: cl, command: commandId, payload: normalize(payload) || {}, options: optionsOf(options)});
                return Promise.resolve();
            },
            read: (cluster, attributes) => {
                ops.push({op: "read", cluster: clusterIdOf(cluster), attributes: normalize(Array.isArray(attributes) ? attributes : [attributes])});
                return Promise.resolve();
            },
            configureReporting: () => Promise.resolve(),
            bind: () => Promise.resolve(),
        };
        let state = {};
        let handled = false;
        for (const converter of definition.toZigbee || []) {
            const keys = Array.isArray(converter.key) ? converter.key : [converter.key];
            if (!keys.includes(key) || typeof converter.convertSet !== "function") continue;
            handled = true;
            try {
                const result = await converter.convertSet(entity, key, input.value, meta);
                if (result && result.state && typeof result.state === "object") Object.assign(state, result.state);
            } catch (error) {
                fail(`convertSet threw: ${(error && error.stack) || error}`);
            }
            break;
        }
        if (!handled) fail(`no toZigbee converter handles '${key}'`);
        process.stdout.write(`${JSON.stringify({state: normalize(state), ops})}\n`);
        return;
    }

    fail(`unknown mode '${mode}'`);
}

main().catch((error) => {
    fail((error && error.stack) || String(error));
});
