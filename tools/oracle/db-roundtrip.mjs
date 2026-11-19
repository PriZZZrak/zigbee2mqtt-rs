#!/usr/bin/env node
// Verifies that a Node-written `database.db` survives
// Node -> Rust model -> Node byte-for-byte.
//
// Usage: node tools/oracle/db-roundtrip.mjs <database.db>
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const input = process.argv[2];
if (!input) {
  console.error("usage: node tools/oracle/db-roundtrip.mjs <database.db>");
  process.exit(1);
}

const output = join(dirname(input), "database-rust-roundtrip.db");
const cargo = spawnSync(
  "cargo",
  ["run", "-q", "-p", "zigbee-apiarist", "--example", "db_roundtrip", "--", input, output],
  { cwd: root, env: process.env, stdio: "inherit" },
);
if (cargo.status !== 0) process.exit(cargo.status ?? 1);

const require = createRequire(import.meta.url);
const { Database } = require(
  join(root, ".upstream/zigbee2mqtt/node_modules/zigbee-herdsman/dist/controller/database.js"),
);
Database.open(output).write();

const original = readFileSync(input);
const rewritten = readFileSync(output);
if (!original.equals(rewritten)) {
  console.error(`MISMATCH: ${input} (${original.length} B) != ${output} (${rewritten.length} B)`);
  process.exit(1);
}
console.log(`OK: ${original.length} bytes preserved through Node -> Rust -> Node`);
