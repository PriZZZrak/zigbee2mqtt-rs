#!/usr/bin/env node
// Storage reference driver (pinned zigbee-herdsman + z2m-style config).
//
//   node scripts/storage_reference.js --db <database.db> --config <config.yaml>
//   node scripts/storage_reference.js --canonicalize <in.db> <out.db>
//
// Prints the normalized device view (same shape as the Rust storage candidate):
// identity, interview state, persisted endpoints and config friendly names.
"use strict";

const fs = require("node:fs");
const path = require("node:path");
const {createRequire} = require("node:module");

function fail(message) {
    console.error(`storage-reference: ${message}`);
    process.exit(2);
}

const appRoot = process.env.Z2M_APP || ".upstream/zigbee2mqtt";
const requireFromApp = createRequire(path.join(path.resolve(appRoot), "package.json"));
const {Database} = requireFromApp("zigbee-herdsman/dist/controller/database.js");
const yaml = requireFromApp("js-yaml");

function parseArgs(argv) {
    const options = {};
    for (let index = 0; index < argv.length; index += 1) {
        const arg = argv[index];
        if (arg === "--canonicalize") {
            options.canonicalize = [argv[index + 1], argv[index + 2]];
            index += 2;
        } else if (arg === "--db" || arg === "--config") {
            options[arg.slice(2)] = argv[index + 1];
            index += 1;
        } else {
            fail(`unknown argument ${arg}`);
        }
    }
    return options;
}

function normalize(dbPath, configPath) {
    const database = Database.open(dbPath);
    const config = configPath && fs.existsSync(configPath)
        ? yaml.load(fs.readFileSync(configPath, "utf8")) || {}
        : {};
    const configured = config.devices || {};
    const devices = [];
    for (const entry of database.entries.values()) {
        if (entry.type === "Coordinator" || entry.type === "Group" || entry.type === "GreenPower") continue;
        if (!entry.ieeeAddr) continue;
        const deviceConfig = configured[entry.ieeeAddr] || {};
        const endpoints = Object.values(entry.endpoints || {})
            .map((endpoint) => ({
                id: endpoint.epId,
                profileId: endpoint.profId,
                deviceId: endpoint.devId,
                inputClusters: endpoint.inClusterList || [],
                outputClusters: endpoint.outClusterList || [],
            }))
            .sort((left, right) => left.id - right.id);
        devices.push({
            ieeeAddr: String(entry.ieeeAddr).toLowerCase(),
            nwkAddr: entry.nwkAddr,
            friendlyName: deviceConfig.friendly_name || String(entry.ieeeAddr).toLowerCase(),
            disabled: !!deviceConfig.disabled,
            manufacturerName: entry.manufName ?? null,
            manufacturerId: entry.manufId ?? null,
            powerSource: entry.powerSource ?? null,
            modelId: entry.modelId ?? null,
            swBuildId: entry.swBuildId ?? null,
            interviewComplete: entry.interviewState
                ? entry.interviewState === "SUCCESSFUL"
                : !!entry.interviewCompleted,
            endpoints,
        });
    }
    devices.sort((left, right) => left.ieeeAddr.localeCompare(right.ieeeAddr));
    return devices;
}

const options = parseArgs(process.argv.slice(2));
if (options.canonicalize) {
    const [input, output] = options.canonicalize;
    if (!input || !output) fail("--canonicalize needs <in> <out>");
    fs.copyFileSync(input, output);
    // Upstream `Database.write()` is what actually emits the canonical NDJSON.
    Database.open(output).write();
    console.error(`storage-reference: wrote ${output}`);
} else {
    if (!options.db) fail("--db is required");
    process.stdout.write(`${JSON.stringify(normalize(options.db, options.config), null, 2)}\n`);
}
