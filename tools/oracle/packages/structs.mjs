// Oracle adapter for zigbee-herdsman/src/adapter/z-stack/structs
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

import * as StructsModule from "../../../.upstream/zigbee-herdsman/src/adapter/z-stack/structs/index.ts";

const Structs = StructsModule.default ?? StructsModule;

const hex = (buffer) => Buffer.from(buffer).toString("hex");

export function handle(job) {
    if (job.op === "nib_to_json") {
        const nib = Structs.nib();
        nib.extendedPANID = Buffer.alloc(8, 0x01);
        nib.nwkLogicalChannel = 21;
        const object = nib.toJSON();
        return {
            value: {
                extendedPANID: hex(object.extendedPANID),
                nwkLogicalChannel: object.nwkLogicalChannel,
            },
        };
    }

    if (job.op === "nib_serialize") {
        const nib = Structs.nib(Buffer.from(job.source, "hex"));
        return {
            value: {
                unaligned: hex(nib.serialize("unaligned", true, 0)),
                aligned: hex(nib.serialize("aligned", true, 0)),
                alignedLength: nib.getLength("aligned"),
                alignedLengthUnpadded: nib.getLength("aligned", false),
                unalignedLength: nib.getLength("unaligned"),
            },
        };
    }

    if (job.op === "security_table_serialize") {
        const table = Structs.securityManagerTable(job.capacity);
        return {
            value: {
                unaligned: hex(table.serialize("unaligned")),
                aligned: hex(table.serialize("aligned")),
                free: table.free.length,
                used: table.used.length,
            },
        };
    }

    if (job.op === "security_table_occupancy") {
        const table = Structs.securityManagerTable(job.capacity);
        table.free[6].ami = 0xfffe;
        table.free[6].authenticationOption = 0x00;
        table.free[7].ami = 0x0000;
        table.free[7].authenticationOption = 0x00;
        return {value: {free: table.free.length, used: table.used.length}};
    }

    if (job.op === "nwk_table_serialize") {
        const table = Structs.nwkSecMaterialDescriptorTable(job.capacity);
        return {
            value: {
                unaligned: hex(table.serialize("unaligned")),
                aligned: hex(table.serialize("aligned")),
                free: table.free.length,
                used: table.used.length,
                capacity: table.capacity,
                entries: table.entries.length,
            },
        };
    }

    if (job.op === "invalid_nib_length") {
        const nib = Structs.nib();
        nib.extendedPANID = Buffer.alloc(job.length, 0x01);
        return {value: null};
    }

    if (job.op === "invalid_nib_source") {
        Structs.nib(Buffer.from(job.source, "hex"));
        return {value: null};
    }

    if (job.op === "table_entries_mismatch") {
        Structs.securityManagerTable([Buffer.alloc(12, 0), Buffer.alloc(17, 0)], "unaligned");
        return {value: null};
    }

    if (job.op === "table_buffer_indivisible") {
        Structs.securityManagerTable(Buffer.alloc(job.length, 0));
        return {value: null};
    }

    throw new Error(`unknown structs op '${job.op}'`);
}
