// Runs the pinned upstream Controller against a scripted TCP coordinator and
// writes every outgoing UNPI request frame as an ordered hex transcript.
//
// Usage: tsx tools/oracle/wire_controller.mjs /tmp/upstream-start.hex

import fs from "node:fs";
import net from "node:net";
import os from "node:os";
import path from "node:path";

import {Controller} from "../../.upstream/zigbee2mqtt/node_modules/.pnpm/zigbee-herdsman@11.2.0_supports-color@7.2.0/node_modules/zigbee-herdsman/dist/controller/controller.js";
import {NvItemsIds} from "../../.upstream/zigbee2mqtt/node_modules/.pnpm/zigbee-herdsman@11.2.0_supports-color@7.2.0/node_modules/zigbee-herdsman/dist/adapter/z-stack/constants/common.js";
import * as Structs from "../../.upstream/zigbee2mqtt/node_modules/.pnpm/zigbee-herdsman@11.2.0_supports-color@7.2.0/node_modules/zigbee-herdsman/dist/adapter/z-stack/structs/index.js";
import {Constants, Frame} from "../../.upstream/zigbee2mqtt/node_modules/.pnpm/zigbee-herdsman@11.2.0_supports-color@7.2.0/node_modules/zigbee-herdsman/dist/adapter/z-stack/unpi/index.js";
import {ZpiObject} from "../../.upstream/zigbee2mqtt/node_modules/.pnpm/zigbee-herdsman@11.2.0_supports-color@7.2.0/node_modules/zigbee-herdsman/dist/adapter/z-stack/znp/zpiObject.js";

const {Subsystem, Type} = Constants;
const NETWORK_KEY = [1, 3, 5, 7, 9, 11, 13, 15, 0, 2, 4, 6, 8, 10, 12, 13];
const EXTENDED_PAN_ID = [0x00, 0x12, 0x4b, 0x00, 0x24, 0xc2, 0xd4, 0x00];
const PAN_ID = 0x1a62;
const CHANNEL = 11;

function nibBytes() {
    const value = Structs.nib();
    value.nwkPanId = PAN_ID;
    value.channelList = 1 << CHANNEL;
    value.nwkLogicalChannel = CHANNEL;
    value.extendedPANID = Buffer.from(EXTENDED_PAN_ID);
    value.SecurityLevel = 5;
    value.nwkUpdateId = 0;
    return value.serialize("unaligned", true, 0);
}

function nwkKey() {
    const value = Structs.nwkKeyDescriptor();
    value.keySeqNum = 0;
    value.key = Buffer.from(NETWORK_KEY);
    return value.serialize("unaligned", true, 0);
}

function items() {
    const map = new Map();
    map.set(NvItemsIds.ZNP_HAS_CONFIGURED_ZSTACK3, Buffer.from([0x55]));
    map.set(NvItemsIds.NIB, nibBytes());
    map.set(NvItemsIds.PRECFGKEY, Buffer.from(NETWORK_KEY));
    map.set(NvItemsIds.NWK_ACTIVE_KEY_INFO, nwkKey());
    map.set(NvItemsIds.NWK_ALTERN_KEY_INFO, nwkKey());
    const nwkKeyItem = Buffer.concat([Buffer.from([0]), Buffer.from(NETWORK_KEY), Buffer.alloc(4)]);
    map.set(NvItemsIds.NWKKEY, nwkKeyItem);
    return map;
}

function response(subsystem, commandId, data) {
    return new Frame(Type.SRSP, subsystem, commandId, Buffer.from(data)).toBuffer();
}

const frames = [];

const server = net.createServer((socket) => {
    let pending = Buffer.alloc(0);
    socket.on("data", (chunk) => {
        pending = Buffer.concat([pending, chunk]);
        while (pending.length >= 5) {
            const length = pending[1];
            const frameLength = 4 + length + 1;
            if (pending.length < frameLength) {
                break;
            }
            const raw = pending.subarray(0, frameLength);
            pending = pending.subarray(frameLength);
            frames.push(raw.toString("hex"));
            const frame = Frame.fromBuffer(length, 4 + length, raw);
            const object = ZpiObject.fromUnpiFrame(frame);
            const name = object.command.name;
            const payload = object.payload;

            let data = null;
            let extra = null;
            switch (name) {
                case "ping":
                    data = [0x01, 0x00];
                    break;
                case "version":
                    data = [2, 1, 2, 7, 1, 0x51, 0x0b, 0x35, 0x01];
                    break;
                case "getDeviceInfo":
                    data = [
                        0x00,
                        0xd4, 0xc2, 0x24, 0x00, 0x4b, 0x12, 0x00, 0x00,
                        0x00, 0x00, 0x05, 9, 0,
                    ];
                    break;
                case "osalNvLength": {
                    const value = items().get(payload.id) ?? Buffer.alloc(0);
                    data = [value.length & 0xff, (value.length >> 8) & 0xff];
                    break;
                }
                case "osalNvReadExt": {
                    const value = items().get(payload.id) ?? Buffer.alloc(0);
                    const chunk = value.subarray(payload.offset, Math.min(payload.offset + 240, value.length));
                    data = [0x00, chunk.length, ...chunk];
                    break;
                }
                case "osalNvWriteExt":
                    data = [0x00];
                    break;
                case "register":
                case "dataRequest":
                case "dataRequestExt":
                case "mgmtPermitJoinReq":
                case "ledControl":
                    data = [0x00];
                    break;
                case "extFindGroup":
                    data = [0x00, payload.groupid & 0xff, (payload.groupid >> 8) & 0xff, 0x00];
                    break;
                case "extNwkInfo": {
                    data = [
                        0x00, 0x00, 0x09, 0x62, 0x1a, 0x00, 0x00,
                        ...EXTENDED_PAN_ID,
                        0, 0, 0, 0, 0, 0, 0, 0,
                        CHANNEL,
                    ];
                    break;
                }
                case "activeEpReq":
                    data = [0x00];
                    extra = new Frame(Type.AREQ, Subsystem.ZDO, 133, Buffer.from([0, 0, 0, 0, 0, 2, 1, 242])).toBuffer();
                    break;
                case "simpleDescReq":
                    data = [0x00];
                    extra = new Frame(Type.AREQ, Subsystem.ZDO, 132, Buffer.from([
                        0, 0, 0, 0, 0, 5, 1, 4, 1, 5, 0, 0, 0, 0,
                    ])).toBuffer();
                    break;
                default:
                    throw new Error(`unexpected ZNP request '${name}'`);
            }

            socket.write(response(frame.subsystem, frame.commandID, data));
            if (extra) {
                socket.write(extra);
            }
        }
    });
});

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "z2m-upstream-wire-"));
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const port = server.address().port;

const controller = new Controller({
    network: {
        panID: PAN_ID,
        extendedPanID: EXTENDED_PAN_ID,
        channelList: [CHANNEL],
        networkKey: NETWORK_KEY,
        networkKeyDistribute: false,
    },
    transport: {adapter: "zstack", path: `tcp://127.0.0.1:${port}`},
    databasePath: path.join(tmp, "database.db"),
    databaseBackupPath: "",
    backupPath: "",
    adapter: {},
});

await controller.start();
await controller.stop();
server.close();

const output = process.argv[2] ?? path.join(tmp, "upstream-start.hex");
fs.writeFileSync(output, `${frames.join("\n")}\n`);
console.log(JSON.stringify({output, frames: frames.length}));
