// Oracle adapter for zigbee-herdsman/src/adapter/z-stack/constants
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

import {Utils} from "../../../.upstream/zigbee-herdsman/src/adapter/z-stack/constants/index.ts";

export function handle(job) {
    if (job.op === "status_description") {
        return {value: Utils.statusDescription(job.code)};
    }

    if (job.op === "channel_mask") {
        return {value: Utils.getChannelMask(job.channels)};
    }

    throw new Error(`unknown constants op '${job.op}'`);
}
