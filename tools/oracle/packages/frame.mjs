// Oracle adapter for zigbee-herdsman/src/adapter/z-stack/unpi/frame.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

import {
    Frame,
    Constants,
} from "../../../.upstream/zigbee-herdsman/src/adapter/z-stack/unpi/index.ts";

const {Subsystem, Type} = Constants;
const hex = (buffer) => Buffer.from(buffer).toString("hex");
const unhex = (value) => Buffer.from(value, "hex");

export function handle(job) {
    if (job.op === "to_buffer") {
        const frame = new Frame(Type[job.type], Subsystem[job.subsystem], job.commandId, unhex(job.data));
        return {value: hex(frame.toBuffer())};
    }

    if (job.op === "from_buffer") {
        try {
            const frame = Frame.fromBuffer(job.length, job.fcsPosition, unhex(job.buffer));
            return {value: hex(frame.toBuffer())};
        } catch (error) {
            return {error: error.message};
        }
    }

    throw new Error(`unknown frame op '${job.op}'`);
}
