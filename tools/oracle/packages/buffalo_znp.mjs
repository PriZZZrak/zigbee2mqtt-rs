// Oracle adapter for zigbee-herdsman/src/adapter/z-stack/znp/buffaloZnp.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

import BuffaloZnpModule from "../../../.upstream/zigbee-herdsman/src/adapter/z-stack/znp/buffaloZnp.ts";
import ParameterTypeModule from "../../../.upstream/zigbee-herdsman/src/adapter/z-stack/znp/parameterType.ts";

const BuffaloZnp = BuffaloZnpModule.default ?? BuffaloZnpModule;
const ParameterType = ParameterTypeModule.default ?? ParameterTypeModule;

const hex = (buffer) => Buffer.from(buffer).toString("hex");
const unhex = (value) => Buffer.from(value, "hex");
const normalize = (value) => {
    if (Buffer.isBuffer(value)) {
        return [...value];
    }
    if (Array.isArray(value)) {
        return value.map(normalize);
    }
    if (value && typeof value === "object") {
        return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, normalize(item)]));
    }
    return value;
};

const type = (name) => {
    const value = ParameterType[name];
    if (value === undefined) {
        throw new Error(`unknown parameter type '${name}'`);
    }
    return value;
};

export function handle(job) {
    if (job.op === "read") {
        const buffalo = new BuffaloZnp(unhex(job.buffer), 0);
        const value = buffalo.read(type(job.type), job.options ?? {});
        return {value: normalize(value), position: buffalo.getPosition()};
    }

    if (job.op === "write") {
        const buffalo = new BuffaloZnp(Buffer.alloc(250), 0);
        buffalo.write(type(job.type), job.value, job.options ?? {});
        return {value: hex(buffalo.getWritten()), position: buffalo.getPosition()};
    }

    throw new Error(`unknown buffalo_znp op '${job.op}'`);
}
