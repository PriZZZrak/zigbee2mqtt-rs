// Dumps the pinned upstream ZCL cluster catalog as JSON for the Rust loader.
import fs from "node:fs";
import path from "node:path";
import {Clusters} from "../../.upstream/zigbee2mqtt/node_modules/.pnpm/zigbee-herdsman@11.2.0_supports-color@7.2.0/node_modules/zigbee-herdsman/dist/zspec/zcl/definition/cluster.js";

const output = path.join(
    "crates/zigbee-apiarist/src/zspec/zcl/definition/clusters.json",
);
fs.writeFileSync(output, `${JSON.stringify(Clusters, null, 2)}\n`);
console.log(JSON.stringify({output, clusters: Object.keys(Clusters).length}));
