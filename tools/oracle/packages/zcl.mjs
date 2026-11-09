// Oracle adapter for zigbee-herdsman/src/zspec/zcl
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

import {
    Frame,
    Utils,
} from "../../../.upstream/zigbee-herdsman/src/zspec/zcl/index.ts";

const hex = (buffer) => Buffer.from(buffer).toString("hex");

function pick(definition) {
    return {ID: definition.ID, name: definition.name};
}

export function handle(job) {
    if (job.op === "get_cluster") {
        return {value: pick(Utils.getCluster(job.key))};
    }

    if (job.op === "get_global_command") {
        return {value: pick(Utils.getGlobalCommand(job.key))};
    }

    if (job.op === "get_foundation_command") {
        return {value: pick(Utils.getFoundationCommand(job.key))};
    }

    if (job.op === "get_cluster_command") {
        const cluster = Utils.getCluster(job.cluster);
        return {value: pick(Utils.getClusterCommand(cluster, job.key))};
    }

    if (job.op === "frame_to_buffer") {
        const frame = Frame.create(
            job.frameType,
            job.direction,
            job.disableDefaultResponse,
            job.manufacturerCode ?? undefined,
            job.transactionSequenceNumber,
            job.command,
            job.cluster,
            job.payload,
            {},
            job.reservedBits ?? 0,
        );
        return {value: hex(frame.toBuffer())};
    }

    throw new Error(`unknown zcl op '${job.op}'`);
}
