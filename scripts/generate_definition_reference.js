#!/usr/bin/env node
// Generated-definition reference: run the pinned zhc generator with full ZCL
// cluster names (unlike the app's test mocks, which expose a partial map).
//
// Reads a device fixture on stdin, prints the `bridge/devices` definition
// payload (`getDefinitionPayload` semantics + linkquality expose).
"use strict";

const path = require("node:path");
const {createRequire} = require("node:module");

function fail(message) {
    console.error(`generated-definition-reference: ${message}`);
    process.exit(2);
}

const appRoot = process.env.Z2M_APP || ".upstream/zigbee2mqtt";
const convertersDist = process.env.HC_CONVERTERS_DIST;
if (!convertersDist) fail("HC_CONVERTERS_DIST is not set");

const requireFromApp = createRequire(path.join(path.resolve(appRoot), "package.json"));
const {Zcl} = requireFromApp("zigbee-herdsman");
const zhc = require(path.resolve(convertersDist, "index.js"));
const {generateDefinition} = require(path.resolve(convertersDist, "lib/generateDefinition.js"));

const clusterNames = {};
for (const cluster of Object.values(Zcl.Clusters)) {
    clusterNames[cluster.ID] = cluster.name;
}

function endpoint(entry) {
    const inputClusters = entry.input_clusters || [];
    const outputClusters = entry.output_clusters || [];
    return {
        ID: entry.id,
        profileID: 260,
        deviceID: 0,
        inputClusters,
        outputClusters,
        getInputClusters: () => inputClusters.map((id) => ({ID: id, name: clusterNames[id] || String(id)})),
        getOutputClusters: () => outputClusters.map((id) => ({ID: id, name: clusterNames[id] || String(id)})),
        getClusterAttributeValue: () => undefined,
        supportsInputCluster: (name) => inputClusters.some((id) => clusterNames[id] === name),
        supportsOutputCluster: (name) => outputClusters.some((id) => clusterNames[id] === name),
    };
}

const LINKQUALITY = {
    access: 1,
    category: "diagnostic",
    description: "Link quality (signal strength)",
    label: "Linkquality",
    name: "linkquality",
    property: "linkquality",
    type: "numeric",
    unit: "lqi",
    value_max: 255,
    value_min: 0,
};

function normalize(value) {
    return JSON.parse(JSON.stringify(value, (key, item) => (typeof item === "function" ? undefined : item)));
}

async function main() {
    let raw = "";
    for await (const chunk of process.stdin) raw += chunk;
    const fixture = JSON.parse(raw);
    const input = fixture.device || fixture;
    const device = {
        type: input.device_type || "EndDevice",
        modelID: input.model_id || "",
        manufacturerName: input.manufacturer_name || "",
        ieeeAddr: input.ieee_addr || "0x00124b0000000000",
        powerSource: input.power_source,
        interviewCompleted: true,
        endpoints: (input.endpoints || []).map(endpoint),
    };
    device.getEndpoint = (id) => device.endpoints.find((entry) => entry.ID === id);
    device.meta = {};

    const {definition} = await generateDefinition(device);
    const prepared = zhc.prepareDefinition(definition);
    let exposes = prepared.exposes;
    if (typeof exposes === "function") exposes = exposes(device, {});
    if (!Array.isArray(exposes)) exposes = [];
    exposes = exposes.filter((entry) => entry !== null && entry !== undefined);
    const payload = {
        source: "generated",
        model: prepared.model,
        vendor: prepared.vendor,
        description: prepared.description,
        exposes: normalize(exposes),
        supports_ota: !!prepared.ota,
        options: normalize(prepared.options || []),
        version: prepared.version || "0.0.0",
    };
    payload.exposes.push(LINKQUALITY);
    process.stdout.write(`${JSON.stringify({definition: payload, model_id: input.model_id || ""})}\n`);
}

main().catch((error) => fail(String((error && error.stack) || error)));
