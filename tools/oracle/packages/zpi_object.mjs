// Oracle adapter for zigbee-herdsman/src/adapter/z-stack/znp/zpiObject.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

import {ZpiObject} from "../../../.upstream/zigbee-herdsman/src/adapter/z-stack/znp/zpiObject.ts";
import {Frame, Constants} from "../../../.upstream/zigbee-herdsman/src/adapter/z-stack/unpi/index.ts";

const {Subsystem, Type} = Constants;

const normalize = (value) => {
    if (Buffer.isBuffer(value)) {
        return [...value];
    }
    if (Array.isArray(value)) {
        return value.map(normalize);
    }
    if (value && typeof value === "object") {
        return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, normalize(item)]));
    }
    return value;
};

const command = (cmd) => ({ID: cmd.ID, name: cmd.name});

export function handle(job) {
    if (job.op === "create_request") {
        const object = ZpiObject.createRequest(Subsystem[job.subsystem], job.command, job.payload ?? {});
        return {
            value: object.unpiFrame.toBuffer().toString("hex"),
            type: object.type,
            subsystem: object.subsystem,
            command: command(object.command),
        };
    }

    if (job.op === "from_frame") {
        const frame = new Frame(
            Type[job.frameType],
            Subsystem[job.subsystem],
            job.commandId,
            Buffer.from(job.data ?? "", "hex"),
        );
        const object = ZpiObject.fromUnpiFrame(frame);
        return {
            value: normalize(object.payload),
            type: object.type,
            subsystem: object.subsystem,
            command: command(object.command),
        };
    }

    throw new Error(`unknown zpi_object op '${job.op}'`);
}
