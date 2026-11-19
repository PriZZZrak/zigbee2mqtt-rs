#!/usr/bin/env node
// Regenerates the canonical Node `database.db` fixture used by the Rust
// byte-parity tests (crates/zigbee-apiarist/tests/database_roundtrip.rs).
//
// Uses the pinned zigbee-herdsman `Database` writer, so the file is exactly
// what upstream produces (compact JSON, insertion order, no trailing newline).
//
// Usage: node tools/oracle/db-fixture.mjs [output]
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const outPath = process.argv[2] ?? join(root, "crates/zigbee-apiarist/tests/fixtures/database-node.golden");
const require = createRequire(import.meta.url);
const { Database } = require(
  join(root, ".upstream/zigbee2mqtt/node_modules/zigbee-herdsman/dist/controller/database.js"),
);

const IEE_A = "0x00124b0000000001";
const IEE_B = "0x00124b0000000002";
const IEE_C = "0x00124b0000000003";

// Coordinator: optional fields that upstream leaves undefined are omitted.
const coordinator = {
  id: 1,
  type: "Coordinator",
  ieeeAddr: IEE_A,
  nwkAddr: 0,
  manufId: 7,
  epList: [1],
  endpoints: {
    1: {
      profId: 260,
      epId: 1,
      devId: 5,
      inClusterList: [],
      outClusterList: [],
      clusters: {},
      binds: [],
      configuredReportings: [],
      meta: {},
    },
  },
  interviewCompleted: true,
  interviewState: "SUCCESSFUL",
  meta: {},
};

// EndDevice with every optional field present, endpoint/group binds and
// reportings with and without a manufacturer code.
const endDevice = {
  id: 2,
  type: "EndDevice",
  ieeeAddr: IEE_B,
  nwkAddr: 0x1234,
  manufId: 0x1037,
  manufName: "_TZE200_test",
  powerSource: "Battery",
  modelId: "TS0601",
  epList: [1, 2],
  endpoints: {
    1: {
      profId: 260,
      epId: 1,
      devId: 81,
      inClusterList: [0, 3, 4, 5],
      outClusterList: [10, 25],
      clusters: {
        genBasic: {
          attributes: {
            zclVersion: 3,
            appVersion: 1,
            modelId: "TS0601",
            manufacturerName: "_TZE200_test",
            powerSource: 3,
            swBuildId: "1.0.0",
            // Values that JSON distinguishes: string, number, bool, null,
            // array, object, and an integral float written by JS as `1`.
            testNumber: 1,
            testFloat: 1.5,
            testFlag: true,
            testNull: null,
            testArray: [1, "two", false],
            testObject: { a: 1, b: "two" },
          },
        },
      },
      binds: [
        { cluster: 6, type: "endpoint", deviceIeeeAddress: IEE_A, endpointID: 1 },
        { cluster: 8, groupID: 42, type: "group" },
      ],
      configuredReportings: [
        { cluster: 0, attrId: 4, minRepIntval: 60, maxRepIntval: 3600, repChange: 1, manufacturerCode: 0x1037 },
        { cluster: 6, attrId: 0, minRepIntval: 0, maxRepIntval: 300, repChange: 0.5 },
      ],
      meta: { configured: 1 },
    },
    2: {
      profId: 260,
      epId: 2,
      devId: 81,
      inClusterList: [0],
      outClusterList: [],
      clusters: {},
      binds: [],
      configuredReportings: [],
      meta: {},
    },
  },
  appVersion: 1,
  stackVersion: 2,
  hwVersion: 3,
  dateCode: "20260101",
  swBuildId: "1.0.0",
  zclVersion: 3,
  interviewCompleted: true,
  interviewState: "SUCCESSFUL",
  meta: { configured: 1 },
  lastSeen: 1760000000000,
  checkinInterval: 3600,
  gpSecurityKey: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
};

// Router without optional fields (coordinator-like omission), and a group bind.
const router = {
  id: 3,
  type: "Router",
  ieeeAddr: IEE_C,
  nwkAddr: 0x5678,
  manufId: 0x1234,
  epList: [1],
  endpoints: {
    1: {
      profId: 260,
      epId: 1,
      devId: 769,
      inClusterList: [0, 3, 4, 5, 513],
      outClusterList: [],
      clusters: {},
      binds: [{ cluster: 513, groupID: 7, type: "group" }],
      configuredReportings: [],
      meta: {},
    },
  },
  interviewCompleted: false,
  interviewState: "FAILED",
  meta: {},
};

// Group entry: members use the group-specific `deviceIeeeAddr` key upstream.
const group = {
  id: 4,
  type: "Group",
  groupID: 42,
  members: [{ deviceIeeeAddr: IEE_B, endpointID: 1 }],
  meta: {},
};

const dir = mkdtempSync(join(tmpdir(), "z2m-db-fixture-"));
const path = join(dir, "database.db");
const database = Database.open(path);
for (const entry of [coordinator, endDevice, router, group]) {
  database.insert(entry);
}
const content = readFileSync(path, "utf8");
writeFileSync(outPath, content);
console.log(`wrote ${outPath} (${Buffer.byteLength(content)} bytes)`);
