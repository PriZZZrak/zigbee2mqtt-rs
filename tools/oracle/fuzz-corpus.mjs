#!/usr/bin/env node
// Deterministic malformed-input corpus for UNPI frames (audit CP-10).
//
// Generates a fixed-seed corpus, runs the pinned upstream `Frame.fromBuffer`
// and writes `crates/zigbee-apiarist/tests/fixtures/fuzz_unpi.json`. The Rust
// test replays the same buffers offline and compares accept/reject plus the
// parsed fields, so a Node live-oracle run is only needed to refresh the corpus.
//
// Usage: node tools/oracle/fuzz-corpus.mjs
import { createRequire } from "node:module";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const upstream = join(root, ".upstream/zigbee2mqtt/node_modules/zigbee-herdsman/dist");
const require = createRequire(import.meta.url);
const { Frame } = require(join(upstream, "adapter/z-stack/unpi/frame.js"));

let seed = 0x5eed1234;
function random() {
  seed = (seed * 1664525 + 1013904223) >>> 0;
  return seed / 0xffffffff;
}
function byte() {
  return Math.floor(random() * 256);
}

const cases = [];
for (let id = 0; id < 120; id += 1) {
  const dataLength = 2 + Math.floor(random() * 18);
  const buffer = Buffer.alloc(dataLength);
  // A UNPI frame needs at least length + cmd0 + cmd1 + fcs.
  buffer[0] = 0xfe;
  for (let index = 1; index < dataLength; index += 1) {
    buffer[index] = byte();
  }
  // Rust models Type/Subsystem as enums, so the corpus keeps cmd0 valid and
  // randomises only the remaining fields (unknown subsystems are a documented
  // out-of-contract input for the typed parser).
  const type = Math.floor(random() * 4);
  const subsystems = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 21];
  const subsystem = subsystems[Math.floor(random() * subsystems.length)];
  buffer[2] = (type << 5) | subsystem;
  const fcsPosition = 3 + Math.floor(random() * Math.max(1, dataLength - 4));
  const length = fcsPosition - 2;
  // Half of the corpus gets a valid checksum, half is left random.
  if (id % 2 === 0) {
    buffer[fcsPosition] = Frame.calculateChecksum(buffer.subarray(1, fcsPosition));
  }
  try {
    const frame = Frame.fromBuffer(length, fcsPosition, buffer);
    cases.push({
      id: `frame-${id}`,
      buffer: buffer.toString("hex"),
      length,
      fcsPosition,
      ok: true,
      type: frame.type,
      subsystem: frame.subsystem,
      commandID: frame.commandID,
      data: Buffer.from(frame.data).toString("hex"),
    });
  } catch (error) {
    cases.push({
      id: `frame-${id}`,
      buffer: buffer.toString("hex"),
      length,
      fcsPosition,
      ok: false,
      error: error.message,
    });
  }
}

const path = join(root, "crates/zigbee-apiarist/tests/fixtures/fuzz_unpi.json");
mkdirSync(dirname(path), { recursive: true });
writeFileSync(path, `${JSON.stringify(cases, null, 2)}\n`);
console.log(`wrote ${cases.length} UNPI fuzz cases (${cases.filter((c) => c.ok).length} valid)`);
