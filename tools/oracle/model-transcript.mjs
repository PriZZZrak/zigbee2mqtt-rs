#!/usr/bin/env node
// Node-vs-Rust model-layer transcript (audit CP-08).
//
// Runs the same Device/Endpoint/Group operations with the pinned upstream
// zigbee-herdsman model classes and the Rust `zigbee-apiarist` example, compares
// the JSON transcripts and writes `docs/fixtures/model-transcript.json`.
//
// Usage: node tools/oracle/model-transcript.mjs
import { createRequire } from "node:module";
import { spawn } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const upstream = join(root, ".upstream/zigbee2mqtt/node_modules/zigbee-herdsman/dist");
const require = createRequire(import.meta.url);

const entity = require(join(upstream, "controller/model/entity.js")).default;
let idCounter = 0;
entity.database = {
  newID: () => ++idCounter,
  insert: () => {},
  update: () => {},
  remove: () => {},
  has: () => false,
  getEntriesIterator: () => [],
  get: () => undefined,
};
const Device = require(join(upstream, "controller/model/device.js")).default;
const Group = require(join(upstream, "controller/model/group.js")).default;

const IEEE = "0x0017880104e45519";
const transcript = [];

function record(op, fn) {
  try {
    transcript.push({ op, value: fn() });
  } catch (error) {
    transcript.push({ op, error: error.message });
  }
}

const device = Device.create("Router", IEEE, 1234, undefined, "Test", "Mains (single phase)", "TEST", "SUCCESSFUL", undefined);
record("create-device", () => ({
  type: device.type,
  ieeeAddr: device.ieeeAddr,
  networkAddress: device.networkAddress,
  manufacturerName: device.manufacturerName,
  modelID: device.modelID,
  interviewState: device.interviewState,
}));
record("create-duplicate-device", () => Device.create("Router", IEEE, 1235, undefined, "Test", "Mains (single phase)", "TEST", "SUCCESSFUL", undefined));

const endpoint1 = device.createEndpoint(1);
const endpoint2 = device.createEndpoint(2);
record("endpoint1", () => ({
  ID: endpoint1.ID,
  deviceIeeeAddress: endpoint1.deviceIeeeAddress,
  deviceNetworkAddress: endpoint1.deviceNetworkAddress,
  profileID: endpoint1.profileID ?? null,
  deviceID: endpoint1.deviceID ?? null,
  inputClusters: endpoint1.inputClusters,
  outputClusters: endpoint1.outputClusters,
}));
record("endpoint-count", () => device.endpoints.length);
record("create-duplicate-endpoint", () => device.createEndpoint(1));
record("get-endpoint", () => device.getEndpoint(2).ID);
endpoint2.inputClusters = [6, 8];
endpoint2.outputClusters = [25];
record("endpoint-clusters", () => ({ input: endpoint2.inputClusters, output: endpoint2.outputClusters }));

const group = Group.create(1);
record("create-group", () => ({ groupID: group.groupID, members: group.members.length }));
record("create-duplicate-group", () => Group.create(1));
record("create-invalid-group-low", () => Group.create(0));
record("create-invalid-group-high", () => Group.create(0xfff8));
group.addMember(endpoint1);
group.addMember(endpoint1);
record("group-add-duplicate", () => group.members.map((member) => member.ID));
group.addMember(endpoint2);
record("group-members", () => group.members.map((member) => [member.deviceIeeeAddress, member.ID]));
record("group-has-member", () => group.hasMember(endpoint2));
group.removeMember(endpoint1);
record("group-remove", () => group.members.map((member) => member.ID));
record("group-by-id", () => Group.byGroupID(1).members.length);

function runRust() {
  return new Promise((resolve, reject) => {
    const child = spawn(
      "cargo",
      ["run", "-q", "-p", "zigbee-apiarist", "--example", "model_transcript"],
      { cwd: root, env: process.env, stdio: ["ignore", "pipe", "pipe"] },
    );
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => (stdout += chunk));
    child.stderr.on("data", (chunk) => (stderr += chunk));
    const timer = setTimeout(() => child.kill("SIGKILL"), 600_000);
    child.on("exit", (code) => {
      clearTimeout(timer);
      if (code !== 0) {
        reject(new Error(`rust example failed (${code}): ${stderr}`));
        return;
      }
      // The logger writes to stdout before the JSON transcript.
      const start = stdout.indexOf("[\n");
      if (start < 0) {
        reject(new Error(`no JSON transcript in rust output: ${stdout}`));
        return;
      }
      resolve(JSON.parse(stdout.slice(start)));
    });
  });
}

const rustTranscript = await runRust();
const nodeTranscript = transcript;
const comparison = {
  match: JSON.stringify(nodeTranscript) === JSON.stringify(rustTranscript),
  node: nodeTranscript,
  rust: rustTranscript,
};
mkdirSync(join(root, "docs/fixtures"), { recursive: true });
writeFileSync(
  join(root, "docs/fixtures/model-transcript.json"),
  `${JSON.stringify(comparison, null, 2)}\n`,
);

console.log(`node ops: ${nodeTranscript.length}, rust ops: ${rustTranscript.length}`);
for (let index = 0; index < Math.max(nodeTranscript.length, rustTranscript.length); index += 1) {
  const node = nodeTranscript[index];
  const rust = rustTranscript[index];
  if (JSON.stringify(node) !== JSON.stringify(rust)) {
    console.log(`MISMATCH at ${index}:`);
    console.log(`  node: ${JSON.stringify(node)}`);
    console.log(`  rust: ${JSON.stringify(rust)}`);
    process.exit(1);
  }
}
console.log("OK: Device/Endpoint/Group transcripts match");
