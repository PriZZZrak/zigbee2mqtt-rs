// Oracle adapter for zigbee-herdsman/src/buffalo/buffalo.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

import {Buffalo} from "../../../.upstream/zigbee-herdsman/src/buffalo/buffalo.ts";

const hex = (buffer) => Buffer.from(buffer).toString("hex");
const unhex = (value) => Buffer.from(value, "hex");

const METHODS = {
    uint8: ["readUInt8", "writeUInt8"],
    uint16: ["readUInt16", "writeUInt16"],
    uint24: ["readUInt24", "writeUInt24"],
    uint32: ["readUInt32", "writeUInt32"],
    uint40: ["readUInt40", "writeUInt40"],
    uint48: ["readUInt48", "writeUInt48"],
    uint56: ["readUInt56", "writeUInt56"],
    uint64: ["readUInt64", "writeUInt64"],
    int8: ["readInt8", "writeInt8"],
    int16: ["readInt16", "writeInt16"],
    int24: ["readInt24", "writeInt24"],
    int32: ["readInt32", "writeInt32"],
    int40: ["readInt40", "writeInt40"],
    int48: ["readInt48", "writeInt48"],
    int56: ["readInt56", "writeInt56"],
    int64: ["readInt64", "writeInt64"],
    floatle: ["readFloatLE", "writeFloatLE"],
    doublele: ["readDoubleLE", "writeDoubleLE"],
    ieeeaddr: ["readIeeeAddr", "writeIeeeAddr"],
    buffer: ["readBuffer", "writeBuffer"],
};

function method(type, index) {
    const entry = METHODS[type];
    if (!entry) {
        throw new Error(`unknown buffalo type '${type}'`);
    }
    return entry[index];
}

export function handle(job) {
    if (job.op === "write") {
        const buffalo = new Buffalo(Buffer.alloc(250), 0);
        if (job.type === "buffer") {
            buffalo.writeBuffer(unhex(job.value), job.length);
        } else {
            buffalo[method(job.type, 1)](job.value);
        }
        return {
            value: hex(buffalo.getWritten()),
            position: buffalo.getPosition(),
        };
    }

    if (job.op === "read") {
        const buffalo = new Buffalo(unhex(job.buffer), 0);
        const value =
            job.type === "buffer"
                ? hex(buffalo.readBuffer(job.length))
                : buffalo[method(job.type, 0)]();
        return {
            value: typeof value === "bigint" ? {bigint: value.toString()} : value,
            position: buffalo.getPosition(),
        };
    }

    throw new Error(`unknown buffalo op '${job.op}'`);
}
