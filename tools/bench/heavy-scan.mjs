// Ranks upstream definitions by static complexity to justify the benchmark
// scenario selection. This is discovery only; it does not execute converters.
//
// Usage: node tools/bench/heavy-scan.mjs [--package path] [--out path]
import { readdirSync, statSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const packageRoot = resolve(
  process.argv.includes("--package")
    ? process.argv[process.argv.indexOf("--package") + 1]
    : join(root, ".upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters"),
);
const out = resolve(
  process.argv.includes("--out")
    ? process.argv[process.argv.indexOf("--out") + 1]
    : join(here, "corpus/heavy-definitions.json"),
);
const require = createRequire(join(packageRoot, "package.json"));
const files = readdirSync(join(packageRoot, "dist/devices")).filter((file) => file.endsWith(".js"));

const ranked = [];
for (const file of files) {
  const size = statSync(join(packageRoot, "dist/devices", file)).size;
  const definitions = require(join(packageRoot, "dist/devices", file)).definitions ?? [];
  for (const definition of definitions) {
    const fromZigbee = definition.fromZigbee?.length ?? 0;
    const toZigbee = definition.toZigbee?.length ?? 0;
    const extend = definition.extend?.length ?? 0;
    const exposes = Array.isArray(definition.exposes) ? definition.exposes.length : 0;
    const datapoints = definition.meta?.tuyaDatapoints?.length ?? 0;
    const score =
      fromZigbee * 2 +
      toZigbee * 2 +
      extend * 4 +
      exposes +
      datapoints * 2 +
      (definition.configure ? 5 : 0) +
      (definition.onEvent ? 5 : 0) +
      size / 10000;
    ranked.push({
      file,
      model: definition.model,
      zigbeeModel: definition.zigbeeModel ?? null,
      fromZigbee,
      toZigbee,
      extend,
      exposes,
      datapoints,
      configure: !!definition.configure,
      onEvent: !!definition.onEvent,
      fileBytes: size,
      score,
    });
  }
}
ranked.sort((left, right) => right.score - left.score);
const result = {
  package: packageRoot,
  files: files.length,
  definitions: ranked.length,
  top: ranked.slice(0, 40),
};
writeFileSync(out, `${JSON.stringify(result, null, 2)}\n`);
console.log(`wrote ${out}: ${ranked.length} definitions, top ${result.top[0]?.model}`);
