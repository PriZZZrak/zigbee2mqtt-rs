// Generates committed differential fixtures from the pinned upstream sources.
//
// Usage: cd tools/oracle && npm install && npm run generate
// Requires Node matching upstream engines and tsx.

import {writeFileSync, mkdirSync} from "node:fs";
import {fileURLToPath} from "node:url";
import {handle as frame} from "./packages/frame.mjs";
import {handle as buffalo} from "./packages/buffalo.mjs";
import {handle as buffalo_znp} from "./packages/buffalo_znp.mjs";
import {handle as constants} from "./packages/constants.mjs";
import {handle as zcl} from "./packages/zcl.mjs";
import {handle as zpi_object} from "./packages/zpi_object.mjs";

const HANDLERS = {frame, buffalo, buffalo_znp, zcl, zpi_object, constants};
const FIXTURES = fileURLToPath(new URL("../../crates/zigbee-apiarist/tests/fixtures/", import.meta.url));

const cases = {
    frame: [
        {
            id: "to-buffer-srsp",
            op: "to_buffer",
            type: "SRSP",
            subsystem: "SYS",
            commandId: 3,
            data: "0601",
        },
        {
            id: "to-buffer-version",
            op: "to_buffer",
            type: "SRSP",
            subsystem: "SYS",
            commandId: 2,
            data: "010203040501010101",
        },
        {
            id: "to-buffer-zdo-areq",
            op: "to_buffer",
            type: "AREQ",
            subsystem: "ZDO",
            commandId: 196,
            data: "d41700",
        },
        {
            id: "to-buffer-empty",
            op: "to_buffer",
            type: "SREQ",
            subsystem: "AF",
            commandId: 0,
            data: "",
        },
        {
            id: "from-buffer-zdo-areq",
            op: "from_buffer",
            length: 3,
            fcsPosition: 7,
            buffer: "fe0345c4d4170041",
        },
        {
            id: "from-buffer-bad-fcs",
            op: "from_buffer",
            length: 3,
            fcsPosition: 7,
            buffer: "fe0345c4d4170040",
        },
    ],
    buffalo: [
        {id: "write-uint8", op: "write", type: "uint8", value: 255},
        {id: "write-uint16", op: "write", type: "uint16", value: 1234},
        {id: "write-uint24", op: "write", type: "uint24", value: 0x123456},
        {id: "write-uint32", op: "write", type: "uint32", value: 0x12345678},
        {id: "write-int8", op: "write", type: "int8", value: -1},
        {id: "write-int16", op: "write", type: "int16", value: -2},
        {id: "write-int24", op: "write", type: "int24", value: -2},
        {id: "write-int32", op: "write", type: "int32", value: -123456},
        {id: "write-floatle", op: "write", type: "floatle", value: 1.5},
        {id: "write-doublele", op: "write", type: "doublele", value: -0.25},
        {id: "write-ieeeaddr", op: "write", type: "ieeeaddr", value: "0x00124b0000000001"},
        {id: "write-buffer", op: "write", type: "buffer", value: "010203", length: 3},
        {id: "read-uint16", op: "read", type: "uint16", buffer: "d204"},
        {id: "read-int24", op: "read", type: "int24", buffer: "feffff"},
        {id: "read-floatle", op: "read", type: "floatle", buffer: "0000c03f"},
        {id: "read-doublele", op: "read", type: "doublele", buffer: "000000000000d0bf"},
        {id: "read-ieeeaddr", op: "read", type: "ieeeaddr", buffer: "010000004b120000"},
        {id: "read-uint64", op: "read", type: "uint64", buffer: "ffffffffffffffff"},
        {id: "read-buffer", op: "read", type: "buffer", buffer: "01020304", length: 3},
    ],
    buffalo_znp: [
        {id: "read-uint16", op: "read", type: "UINT16", buffer: "d204"},
        {id: "read-int8", op: "read", type: "INT8", buffer: "ff"},
        {id: "read-ieeeaddr", op: "read", type: "IEEEADDR", buffer: "010000004b120000"},
        {id: "read-buffer", op: "read", type: "BUFFER", buffer: "01020304", options: {length: 3}},
        {id: "read-buffer8", op: "read", type: "BUFFER8", buffer: "0001020304050607"},
        {id: "read-list-uint8", op: "read", type: "LIST_UINT8", buffer: "010203", options: {length: 3}},
        {
            id: "read-list-uint16",
            op: "read",
            type: "LIST_UINT16",
            buffer: "010002000300",
            options: {length: 3},
        },
        {
            id: "read-list-network",
            op: "read",
            type: "LIST_NETWORK",
            buffer: "34120f214301",
            options: {length: 1},
        },
        {id: "read-buffer-missing-length", op: "read", type: "BUFFER", buffer: "0102"},
        {id: "write-uint8", op: "write", type: "UINT8", value: 255},
        {id: "write-uint16", op: "write", type: "UINT16", value: 1234},
        {id: "write-ieeeaddr", op: "write", type: "IEEEADDR", value: "0x00124b0000000001"},
        {id: "write-buffer", op: "write", type: "BUFFER", value: [1, 2, 3], options: {length: 3}},
        {
            id: "write-buffer-length-mismatch",
            op: "write",
            type: "BUFFER",
            value: [1, 2, 3],
            options: {length: 2},
        },
        {id: "write-buffer8", op: "write", type: "BUFFER8", value: [0, 1, 2, 3, 4, 5, 6, 7]},
        {id: "write-list-uint8", op: "write", type: "LIST_UINT8", value: [1, 2, 3]},
        {id: "write-list-uint16", op: "write", type: "LIST_UINT16", value: [1, 2, 3]},
        {id: "write-int8", op: "write", type: "INT8", value: -1},
        {id: "write-list-network", op: "write", type: "LIST_NETWORK", value: []},
    ],
    zpi_object: [
        {
            id: "create-version",
            op: "create_request",
            subsystem: "SYS",
            command: "version",
            payload: {},
        },
        {
            id: "create-osal-nv-read",
            op: "create_request",
            subsystem: "SYS",
            command: "osalNvRead",
            payload: {id: 1, offset: 2},
        },
        {
            id: "from-version",
            op: "from_frame",
            frameType: "SRSP",
            subsystem: "SYS",
            commandId: 2,
            data: "010203040501010101",
        },
        {
            id: "from-osal-nv-read",
            op: "from_frame",
            frameType: "SRSP",
            subsystem: "SYS",
            commandId: 8,
            data: "00020102",
        },
    ],
    zcl: [
        {id: "get-cluster-name", op: "get_cluster", key: "genBasic"},
        {id: "get-cluster-id", op: "get_cluster", key: 6},
        {id: "get-global-command", op: "get_global_command", key: "read"},
        {id: "get-foundation-command", op: "get_foundation_command", key: 0},
        {id: "get-cluster-command", op: "get_cluster_command", cluster: "genOnOff", key: "off"},
        {id: "get-cluster-command-toggle", op: "get_cluster_command", cluster: "genOnOff", key: "toggle"},
        {
            id: "frame-to-buffer-global-read",
            op: "frame_to_buffer",
            frameType: 0,
            direction: 0,
            disableDefaultResponse: true,
            manufacturerCode: null,
            transactionSequenceNumber: 1,
            command: "read",
            cluster: "genBasic",
            payload: [],
            reservedBits: 0,
        },
    ],
    constants: [
        ...Array.from({length: 256}, (_, code) => ({
            id: `status-${code}`,
            op: "status_description",
            code,
        })),
        {id: "channel-mask-11", op: "channel_mask", channels: [11]},
        {id: "channel-mask-15-20-25", op: "channel_mask", channels: [15, 20, 25]},
        {id: "channel-mask-0-26", op: "channel_mask", channels: [0, 26]},
        {id: "channel-mask-11-15-20-25", op: "channel_mask", channels: [11, 15, 20, 25]},
    ],
};

mkdirSync(FIXTURES, {recursive: true});

for (const [module, jobs] of Object.entries(cases)) {
    const fixtures = jobs.map((job) => {
        let result;
        try {
            result = HANDLERS[module](job);
        } catch (error) {
            result = {error: error.message};
        }
        if (result.error !== undefined) {
            return {...job, error: result.error};
        }
        const fixture = {...job, expected: result.value};
        if (result.position !== undefined) {
            fixture.position = result.position;
        }
        return fixture;
    });
    writeFileSync(`${FIXTURES}/${module}.json`, `${JSON.stringify(fixtures, null, 2)}\n`);
    process.stdout.write(`${module}: ${fixtures.length} fixtures\n`);
}
