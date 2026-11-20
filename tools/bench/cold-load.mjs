// Measures cold package-root load in fresh Node processes.
//
// Usage: node tools/bench/cold-load.mjs [runs] [--package path] [--label name]
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");
const firstArgument = process.argv[2];
const runs = firstArgument && !firstArgument.startsWith("--") ? Number(firstArgument) : 5;
const packageRoot = resolve(
  process.argv.includes("--package")
    ? process.argv[process.argv.indexOf("--package") + 1]
    : join(root, ".upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters"),
);
const label = process.argv.includes("--label")
  ? process.argv[process.argv.indexOf("--label") + 1]
  : `${process.platform}-${process.arch}`;
const runner = join(here, "runner.mjs");
const values = [];
for (let index = 0; index < runs; index++) {
  const out = mkdtempSync(join(tmpdir(), "z2m-cold-"));
  execFileSync(
    process.execPath,
    [
      runner,
      "--filter",
      "package-load",
      "--out",
      out,
      "--package",
      packageRoot,
      "--external",
      join(here, "fixtures/external/L101Ze_z2m.js"),
    ],
    { stdio: "ignore" },
  );
  const cases = JSON.parse(readFileSync(join(out, "cases.json"), "utf8"));
  values.push(cases[0].cold.loadMs);
}
const sorted = [...values].sort((a, b) => a - b);
const result = {
  label,
  runs,
  valuesMs: values,
  minMs: sorted[0],
  medianMs: sorted[Math.floor(sorted.length / 2)],
  maxMs: sorted[sorted.length - 1],
};
const output = join(here, "corpus", `cold-load-${label}.json`);
writeFileSync(output, `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify(result, null, 2));
