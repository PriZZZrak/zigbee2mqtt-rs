// Generates the pinned ZCL/ZDO frame differential corpus from
// zigbee-herdsman 11.2.0. Run directly with Node (dist package).
import {createRequire} from "node:module";
import {writeFileSync} from "node:fs";
import {fileURLToPath} from "node:url";
const require = createRequire(import.meta.url);
const root = fileURLToPath(new URL("../../.upstream/zigbee2mqtt/node_modules/zigbee-herdsman/dist/zspec/zcl/index.js", import.meta.url));
const zcl = require(root);
const zdo = require(fileURLToPath(new URL("../../.upstream/zigbee2mqtt/node_modules/zigbee-herdsman/dist/zspec/zdo/index.js", import.meta.url)));
const hex = (buffer) => Buffer.from(buffer).toString("hex");
const cases = [];
let next = 0;
function add(entry) { cases.push({id: `${entry.op}-${next++}`, ...entry}); }
// A `frame_from_buffer` case is only a valid differential if the pinned
// upstream can decode the buffer and re-encode the same bytes. Duplicate
// command IDs (e.g. greenPower ID 4) resolve to a different command on decode
// than on create, and upstream itself throws INVALID_FIELD; those buffers are
// recorded as expected errors for both implementations instead of pretending
// the codecs disagree.
function decodeRoundTrip(clusterId, buffer) {
  try {
    const header = zcl.Header.fromBuffer(buffer);
    const frame = zcl.Frame.fromBuffer(clusterId, header, buffer, {});
    return hex(frame.toBuffer());
  } catch (error) {
    return undefined;
  }
}
function defaultValue(type) {
  switch (type) {
    case 0x08: case 0x09: case 0x0a: case 0x0b: case 0x0c: case 0x0d: case 0x0e: case 0x0f:
    case 0x10: case 0x18: case 0x19: case 0x1a: case 0x1b: case 0x1c: case 0x1d: case 0x1e: case 0x1f:
    case 0x20: case 0x21: case 0x22: case 0x23: case 0x24: case 0x25: case 0x26: case 0x27:
    case 0x28: case 0x29: case 0x2a: case 0x2b: case 0x2c: case 0x2d: case 0x2e: case 0x2f:
      return 0;
    case 0x10: case 0x18: return 0;
    case 0x30: case 0x31: case 0x38: case 0x39: case 0x3a: return 0;
    case 0x42: return "";
    default: return undefined;
  }
}
function payloadFor(command) {
  const payload = {};
  for (const parameter of command.parameters || []) {
    if (parameter.conditions && parameter.conditions.length) continue;
    const value = defaultValue(parameter.type);
    if (value !== undefined) payload[parameter.name] = value;
  }
  return payload;
}
const clusters = zcl.Clusters;
for (const [clusterName, cluster] of Object.entries(clusters)) {
  if (!cluster || cluster.ID === undefined) continue;
  for (const [commandName, command] of Object.entries(cluster.commands || {})) {
    for (const direction of [0, 1]) {
      try {
        const frame = zcl.Frame.create(
          commandName === "read" || commandName === "write" || commandName === "report" || commandName === "readRsp" ? 0 : 1,
          direction,
          true,
          cluster.manufacturerCode,
          1,
          commandName,
          clusterName,
          payloadFor(command),
          {},
          0,
        );
        const buffer = frame.toBuffer();
        add({op: "frame_to_buffer", cluster: clusterName, command: commandName, frameType: frame.header.frameControl.frameType, direction: frame.header.frameControl.direction, disableDefaultResponse: true, manufacturerCode: cluster.manufacturerCode, transactionSequenceNumber: 1, payload: payloadFor(command), reservedBits: 0, expected: hex(buffer)});
        const decoded = decodeRoundTrip(cluster.ID, buffer);
        if (decoded === undefined) {
          add({op: "frame_from_buffer", cluster: clusterName, command: commandName, buffer: hex(buffer), expectedError: true, upstreamDecodeError: true});
        } else {
          add({op: "frame_from_buffer", cluster: clusterName, command: commandName, buffer: hex(buffer), expected: decoded, upstreamRoundTrip: decoded === hex(buffer)});
        }
      } catch (error) { /* command requires a payload shape not representable by defaults */ }
    }
  }
  for (const [commandName, command] of Object.entries(cluster.commandsResponse || {})) {
    try {
      const frame = zcl.Frame.create(0, 1, true, cluster.manufacturerCode, 1, commandName, clusterName, payloadFor(command), {}, 0);
      add({op: "frame_to_buffer", cluster: clusterName, command: commandName, frameType: 0, direction: 1, disableDefaultResponse: true, manufacturerCode: cluster.manufacturerCode, transactionSequenceNumber: 1, payload: payloadFor(command), reservedBits: 0, expected: hex(frame.toBuffer())});
    } catch (error) {}
  }
}
// Global/foundation commands on a standard cluster.
for (const [command, payload] of [["read", [{attrId: 0, dataType: 0x20}]], ["write", [{attrId: 0, dataType: 0x20, attrData: 0}]], ["report", [{attrId: 0, dataType: 0x20, attrData: 0}]], ["defaultRsp", {cmdId: 0, statusCode: 0}]]) {
  try {
    const frame = zcl.Frame.create(0, 0, true, undefined, 1, command, "genOnOff", payload, {}, 0);
    add({op: "frame_to_buffer", cluster: "genOnOff", command, frameType: 0, direction: 0, disableDefaultResponse: true, transactionSequenceNumber: 1, payload, reservedBits: 0, expected: hex(frame.toBuffer())});
    const decoded = decodeRoundTrip(zcl.Clusters.genOnOff.ID, frame.toBuffer());
    add({op: "frame_from_buffer", cluster: "genOnOff", command, buffer: hex(frame.toBuffer()), expected: decoded ?? hex(frame.toBuffer()), expectedError: decoded === undefined});
  } catch (error) {}
}
// Unknown incoming cluster with a global report: must be readable as raw/diagnostic.
try {
  const frame = zcl.Frame.create(0, 0, true, undefined, 1, "report", "genOnOff", [{attrId: 0, dataType: 0x20, attrData: 0}], {}, 0);
  const bytes = frame.toBuffer();
  // Rewrite the cluster ID in the parsed path instead of frame bytes: Rust
  // receives the cluster id separately.
  add({op: "frame_from_buffer", cluster: "0x1234", clusterId: 0x1234, command: "report", buffer: hex(bytes), expected: hex(bytes), unknownCluster: true});
} catch (error) {}
// Corrupted / truncated frames: upstream throws; Rust must return an error,
// never panic or encode a guessed payload.
const truncated = cases.find((entry) => entry.op === "frame_from_buffer" && entry.buffer.length > 8);
if (truncated) {
  add({op: "frame_from_buffer", cluster: truncated.cluster, command: truncated.command, buffer: truncated.buffer.slice(0, -2), expectedError: true});
  add({op: "frame_from_buffer", cluster: truncated.cluster, command: truncated.command, buffer: "00", expectedError: true});
}
const zdoConstants = Object.fromEntries(Object.entries(zdo.ClusterId).filter(([key]) => /^\d+$/.test(key)).map(([id, name]) => [name, Number(id)]));
writeFileSync(fileURLToPath(new URL("../../crates/zigbee-apiarist/tests/fixtures/zcl-pinned.json", import.meta.url)), JSON.stringify({revision: "zigbee-herdsman 11.2.0", cases}, null, 0) + "\n");
writeFileSync(fileURLToPath(new URL("../../crates/zigbee-apiarist/tests/fixtures/zdo-pinned.json", import.meta.url)), JSON.stringify({revision: "zigbee-herdsman 11.2.0", constants: zdoConstants}, null, 2) + "\n");
console.log(JSON.stringify({cases: cases.length}));
