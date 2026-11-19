#!/usr/bin/env node
// Regenerates the js-yaml config fixture used by
// crates/z2m-rs/tests/yaml_dump.rs. The JSON input is committed too so the
// comparison is reproducible without Node at test time.
//
// Usage: node tools/oracle/yaml-fixture.mjs
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const require = createRequire(import.meta.url);
const yaml = require(join(root, ".upstream/zigbee2mqtt/node_modules/js-yaml"));

const fixtureDir = join(root, "crates/z2m-rs/tests/fixtures");
const inputPath = join(fixtureDir, "yaml-dump.json");
const outputPath = join(fixtureDir, "yaml-dump.golden");

const long = "word ".repeat(30).trim();
const data = {
  homeassistant: { enabled: false },
  mqtt: { base_topic: "zigbee2mqtt", server: "mqtt://localhost" },
  advanced: {
    log_output: ["console", "file"],
    log_level: "warning",
    log_directory: "/tmp/log/zigbee2mqtt/%TIMESTAMP%",
    last_seen: "ISO_8601",
    nested: { deeper: [1, 2, { three: "four" }] },
  },
  frontend: { enabled: true, port: 8080, host: "127.0.0.1" },
  devices: {
    "0x00124b0000000001": { friendly_name: "Test lamp" },
    "0x00124b0000000002": { friendly_name: "Test sensor" },
  },
  groups: { "1": { friendly_name: "Test group" } },
  empty_map: {},
  empty_list: [],
  null_value: null,
  scalars: {
    empty: "",
    yes: "yes",
    no: "no",
    on: "on",
    off: "off",
    y: "y",
    n: "n",
    number: "123",
    hex: "0x10",
    date: "2026-01-01",
    time: "12:34:56",
    exponent: "1e3",
    colon: "a: b",
    hash: "a # b",
    url: "http://localhost/a#b",
    quote: "a'b",
    double_quote: 'a"b',
    tab: "a\tb",
    multiline: "line1\nline2",
    long,
  },
};

writeFileSync(inputPath, `${JSON.stringify(data, null, 2)}\n`);
writeFileSync(outputPath, yaml.dump(data));
console.log(`wrote ${inputPath} and ${outputPath}`);
