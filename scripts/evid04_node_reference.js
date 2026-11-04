#!/usr/bin/env node
// EVID-04 Node reference driver (pinned zigbee-herdsman-converters).
//
// Reads a fixture JSON object on stdin, resolves the definition through the
// upstream models index exactly like src/index.ts getFromIndex/findDefinition,
// runs the matching fromZigbee converter for the synthetic message and prints
// the converted state as JSON.
//
// Environment: HC_CONVERTERS_DIST must point at the converters `dist` directory
// of the pinned checkout (`.../zigbee-herdsman-converters/dist`).
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

let raw = "";
process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk) => {
    raw += chunk;
});
process.stdin.on("end", () => {
    let input;
    try {
        input = JSON.parse(raw);
    } catch (error) {
        fail(`invalid fixture JSON: ${error.message}`);
    }

    const model = input.model;
    if (typeof model !== "string") fail("fixture is missing 'model'");
    const clusterId = Number(input.cluster_id);
    if (!Number.isInteger(clusterId)) fail("fixture is missing a valid 'cluster_id'");
    const msgType = input.msg_type || "attributeReport";
    const attribute = input.attribute;
    if (typeof attribute !== "string") fail("fixture is missing 'attribute'");
    if (input.value === undefined) fail("fixture is missing 'value'");

    const modelsIndex = JSON.parse(
        fs.readFileSync(path.join(dist, "models-index.json"), "utf8"),
    );
    const lookupKey = model.toLowerCase().replace(/\0(.|\n)*$/g, "").trim();
    const candidates = modelsIndex[lookupKey];
    if (!Array.isArray(candidates) || candidates.length === 0) {
        fail(`no models-index candidates for '${lookupKey}'`);
    }
    const [file, index] = candidates[0];
    const definitions = require(path.join(dist, "devices", file)).definitions;
    const definition = definitions[index];
    if (!definition) fail(`missing definition ${file}[${index}]`);

    // Resolve herdsman from the pinned converters package, not this repository.
    const requireFromConverters = createRequire(path.join(dist, "..", "package.json"));
    const {Clusters} = requireFromConverters(
        "zigbee-herdsman/dist/zspec/zcl/definition/cluster.js",
    );
    const clusterName = Object.values(Clusters).find((cluster) => cluster.ID === clusterId)?.name;
    if (!clusterName) fail(`unknown cluster id ${clusterId}`);

    const msg = {
        cluster: clusterName,
        type: msgType,
        data: {[attribute]: input.value},
        endpoint: {ID: 1},
        linkquality: input.linkquality === undefined ? 0 : input.linkquality,
        groupID: 0,
    };
    const meta = {
        state: input.state && typeof input.state === "object" ? input.state : {},
        device: {
            modelID: model,
            manufacturerName: input.manufacturer_name,
            ieeeAddr: input.ieee_addr || "0x00124b0000000000",
            endpoints: [{ID: 1}],
            getEndpoint: (id) => ({ID: id}),
        },
        options: {},
        message: {},
    };

    const out = {};
    let matched = false;
    for (const converter of definition.fromZigbee || []) {
        const types = Array.isArray(converter.type) ? converter.type : [converter.type];
        if (converter.cluster !== clusterName && String(converter.cluster) !== String(clusterId)) continue;
        if (!types.includes(msgType)) continue;
        matched = true;
        try {
            const result = converter.convert(definition, msg, (publish) => Object.assign(out, publish), {}, meta);
            if (result && typeof result === "object") Object.assign(out, result);
        } catch (error) {
            fail(`converter threw: ${(error && error.stack) || error}`);
        }
    }
    if (!matched) fail(`no fromZigbee converter matched cluster ${clusterName}/${msgType}`);

    process.stdout.write(`${JSON.stringify(out)}\n`);
});
