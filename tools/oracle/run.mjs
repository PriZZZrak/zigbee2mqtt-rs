// Differential oracle runner.
//
// Reads NDJSON jobs from stdin, dispatches them to the pinned upstream
// implementation and writes NDJSON results to stdout. Used to detect
// divergence between the Rust port and zigbee-herdsman v11.2.0.

import {createInterface} from "node:readline";
import {handle as frame} from "./packages/frame.mjs";
import {handle as buffalo} from "./packages/buffalo.mjs";
import {handle as zcl} from "./packages/zcl.mjs";

const HANDLERS = {frame, buffalo, zcl};

const rl = createInterface({input: process.stdin, crlfDelay: Infinity});
for await (const line of rl) {
    if (!line.trim()) {
        continue;
    }

    const job = JSON.parse(line);
    let result;
    try {
        const handler = HANDLERS[job.module];
        if (!handler) {
            throw new Error(`unknown oracle module '${job.module}'`);
        }
        result = handler(job);
    } catch (error) {
        result = {error: error.message};
    }

    process.stdout.write(`${JSON.stringify({id: job.id, ...result})}\n`);
}
