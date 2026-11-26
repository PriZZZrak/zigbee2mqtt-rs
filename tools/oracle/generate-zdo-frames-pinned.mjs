// Generates ZDO request/response frame cases from pinned zigbee-herdsman 11.2.0.
import {createRequire} from "node:module";
import {writeFileSync} from "node:fs";
import {fileURLToPath} from "node:url";
const require = createRequire(import.meta.url);
const zdo = require(fileURLToPath(new URL("../../.upstream/zigbee2mqtt/node_modules/zigbee-herdsman/dist/zspec/zdo/index.js", import.meta.url)));
const C = zdo.ClusterId;
const hex = (buffer) => Buffer.from(buffer).toString("hex");
const cases = [];
function request(name, cluster, args, overhead = false) {
  cases.push({name, op: "build_request", clusterId: C[cluster], args, overhead, expected: hex(zdo.Buffalo.buildRequest(overhead, C[cluster], ...args))});
}
function response(name, cluster, buffer) {
  let parsed;
  try { parsed = zdo.Buffalo.readResponse(false, C[cluster], buffer); }
  catch (error) { parsed = {error: error.message}; }
  cases.push({name, op: "read_response", clusterId: C[cluster], buffer: buffer.toString("hex"), expected: parsed});
}
const SRC = "0x0017880104e45531";
const DST = "0x0017880104e45599";
request("bind-unicast", "BIND_REQUEST", [SRC, 1, 6, 3, DST, null, 2]);
request("bind-multicast", "BIND_REQUEST", [SRC, 1, 6, 1, null, 0x1234, null]);
request("unbind-unicast", "UNBIND_REQUEST", [SRC, 1, 6, 3, DST, null, 2]);
request("unbind-multicast", "UNBIND_REQUEST", [SRC, 1, 6, 1, null, 0x1234, null]);
request("nwk-addr-req", "NETWORK_ADDRESS_REQUEST", [SRC, true, 0]);
request("ieee-addr-req", "IEEE_ADDRESS_REQUEST", [0x1234, true, 0]);
request("node-desc-req", "NODE_DESCRIPTOR_REQUEST", [0x1234]);
request("power-desc-req", "POWER_DESCRIPTOR_REQUEST", [0x1234]);
request("active-ep-req", "ACTIVE_ENDPOINTS_REQUEST", [0x1234]);
request("simple-desc-req", "SIMPLE_DESCRIPTOR_REQUEST", [0x1234, 1]);
request("match-desc-req", "MATCH_DESCRIPTORS_REQUEST", [0x1234, 0x0104, [6, 8], [25]]);
request("leave-req", "LEAVE_REQUEST", [SRC, 0]);
request("permit-join-req", "PERMIT_JOINING_REQUEST", [60, 1, []]);
request("mgmt-lqi-req", "LQI_TABLE_REQUEST", [0]);
request("mgmt-bind-req", "BINDING_TABLE_REQUEST", [0]);
request("mgmt-rtg-req", "ROUTING_TABLE_REQUEST", [0]);
request("nwk-addr-req-overhead", "NETWORK_ADDRESS_REQUEST", [SRC, false, 0], true);
response("nwk-addr-rsp", "NETWORK_ADDRESS_RESPONSE", Buffer.from("003155e40401881700341202037b00cbcd", "hex"));
response("ieee-addr-rsp", "IEEE_ADDRESS_RESPONSE", Buffer.from("003155e40401881700341200000a02", "hex"));
response("active-ep-rsp", "ACTIVE_ENDPOINTS_RESPONSE", Buffer.from("00341203010203", "hex"));
response("simple-desc-rsp", "SIMPLE_DESCRIPTOR_RESPONSE", Buffer.from("00341208010401" + "7b00" + "0002d51d700201" + "ca14", "hex"));
response("node-desc-rsp", "NODE_DESCRIPTOR_RESPONSE", Buffer.from("00341222" + "20" + "0e" + "0a0a" + "7c" + "ff7e" + "1600" + "ff3c" + "00", "hex"));
response("binding-table-rsp", "BINDING_TABLE_RESPONSE", Buffer.from("000100013155e40401881700010600039955e4040188170002", "hex"));
response("bind-rsp", "BIND_RESPONSE", Buffer.from("00", "hex"));
response("unbind-rsp", "UNBIND_RESPONSE", Buffer.from("00", "hex"));
response("leave-rsp", "LEAVE_RESPONSE", Buffer.from("00", "hex"));
response("nwk-addr-rsp-error", "NETWORK_ADDRESS_RESPONSE", Buffer.from("01", "hex"));
response("active-ep-rsp-error", "ACTIVE_ENDPOINTS_RESPONSE", Buffer.from("80", "hex"));
writeFileSync(fileURLToPath(new URL("../../crates/zigbee-apiarist/tests/fixtures/zdo-frames-pinned.json", import.meta.url)), JSON.stringify({revision: "zigbee-herdsman 11.2.0", cases}, null, 2) + "\n");
console.log(JSON.stringify({cases: cases.length}));
