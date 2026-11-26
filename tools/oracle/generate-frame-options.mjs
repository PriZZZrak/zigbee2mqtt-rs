// Generates ZCL frame cases that exercise command options, manufacturer codes,
// endpoints-independent headers and custom clusters. Pinned to
// zigbee-herdsman 11.2.0; the Rust transport builds the same frames through the
// real Endpoint/ZCL code.
import {createRequire} from "node:module";
import {writeFileSync} from "node:fs";
import {fileURLToPath} from "node:url";
const require = createRequire(import.meta.url);
const zcl = require(fileURLToPath(new URL("../../.upstream/zigbee2mqtt/node_modules/zigbee-herdsman/dist/zspec/zcl/index.js", import.meta.url)));
const hex = (buffer) => Buffer.from(buffer).toString("hex");
const cases = [];
function add(name, options) {
  const frame = zcl.Frame.create(
    options.frameType,
    options.direction,
    options.disableDefaultResponse,
    options.manufacturerCode ?? undefined,
    options.transactionSequenceNumber,
    options.command,
    options.cluster,
    options.payload,
    options.customClusters ?? {},
    options.reservedBits ?? 0,
  );
  cases.push({name, ...options, manufacturerCode: options.manufacturerCode ?? null, reservedBits: options.reservedBits ?? 0, expected: hex(frame.toBuffer())});
}
add("onOff-on-default-response", {frameType: 1, direction: 0, disableDefaultResponse: false, transactionSequenceNumber: 0x2a, command: "on", cluster: "genOnOff", payload: {}});
add("level-move-to-level", {frameType: 1, direction: 0, disableDefaultResponse: true, transactionSequenceNumber: 7, command: "moveToLevelWithOnOff", cluster: "genLevelCtrl", payload: {level: 128, transtime: 5}});
add("identify-query-response", {frameType: 1, direction: 1, disableDefaultResponse: false, transactionSequenceNumber: 0x31, command: "identifyQueryRsp", cluster: "genIdentify", payload: {timeout: 60}});
add("manufacturer-cluster-command", {frameType: 1, direction: 0, disableDefaultResponse: true, manufacturerCode: 0x1217, transactionSequenceNumber: 0x12, command: "clearBindingTable", cluster: "manuSpecificAmazonWWAH", payload: {}});
add("manufacturer-write-attribute", {frameType: 0, direction: 0, disableDefaultResponse: true, manufacturerCode: 0x1234, transactionSequenceNumber: 0x05, command: "write", cluster: "genBasic", payload: [{attrId: 0x4000, dataType: 0x42, attrData: "abc"}]});
add("read-attribute", {frameType: 0, direction: 0, disableDefaultResponse: true, transactionSequenceNumber: 0x0b, command: "read", cluster: "genOnOff", payload: [{attrId: 0x0000}]});
add("configure-reporting", {frameType: 0, direction: 0, disableDefaultResponse: true, transactionSequenceNumber: 0x0c, command: "configReport", cluster: "genOnOff", payload: [{direction: 0, attrId: 0x0000, dataType: 0x10, minRepIntval: 1, maxRepIntval: 60, repChange: 0}]});
add("read-response", {frameType: 0, direction: 1, disableDefaultResponse: true, transactionSequenceNumber: 0x0d, command: "readRsp", cluster: "genOnOff", payload: [{attrId: 0x0000, status: 0, dataType: 0x10, attrData: 1}]});
add("reserved-bits", {frameType: 1, direction: 0, disableDefaultResponse: true, transactionSequenceNumber: 0x0e, command: "toggle", cluster: "genOnOff", payload: {}, reservedBits: 5});
const customCluster = {
  name: "customCluster",
  ID: 0xfc00,
  manufacturerCode: 0xabcd,
  attributes: {},
  commands: {setValue: {name: "setValue", ID: 0x01, parameters: [{name: "value", type: zcl.DataType.UINT8}]}},
  commandsResponse: {},
};
add("custom-cluster-command", {
  frameType: 1,
  direction: 0,
  disableDefaultResponse: true,
  manufacturerCode: 0xabcd,
  transactionSequenceNumber: 0x11,
  command: "setValue",
  cluster: "customCluster",
  payload: {value: 42},
  customCluster: {name: customCluster.name, ID: customCluster.ID, manufacturerCode: customCluster.manufacturerCode, command: {name: "setValue", ID: 0x01, type: zcl.DataType.UINT8}},
  customClusters: {[customCluster.name]: customCluster},
});
writeFileSync(fileURLToPath(new URL("../../crates/zigbee-apiarist/tests/fixtures/frame-options-pinned.json", import.meta.url)), JSON.stringify({revision: "zigbee-herdsman 11.2.0", cases}, null, 2) + "\n");
console.log(JSON.stringify({cases: cases.length}));
