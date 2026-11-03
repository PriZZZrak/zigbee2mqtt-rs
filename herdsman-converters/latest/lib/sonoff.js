"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.firmwareSupportFeaturesVersion = exports.withConditionalEnumValues = exports.withConditionalExpose = exports.readAttributesInBatches = exports.parseTimeToSecondsSinceMidnight = exports.formatSecondsToTimeSinceMidnight = exports.toUInt16LEBytes = exports.toUInt32LEBytes = exports.zclArrayUInt32FzConvert = exports.zclArrayValueToBytes = exports.readUInt16LE = exports.readUInt40LE = exports.readUInt32LE = exports.signedInt32MilliToValue = exports.toBigEndianUInt32 = exports.parseSWVZFRawZclCommand = exports.shiftUtcSecondsByOffsetMonths = exports.formatUtcSecondsToIsoWithOffset = exports.getRuntimeLocalOffsetSeconds = exports.parseIsoWithOffsetToUtcSeconds = exports.deviceLocal2000ToUTCSeconds = exports.utcToDeviceLocal2000Seconds = exports.YEAR_2000_IN_UTC = void 0;
const exposes = __importStar(require("./exposes"));
const logger_1 = require("./logger");
const utils = __importStar(require("./utils"));
const NS = "zhc:sonoff";
/**
 * Unix timestamp in seconds for `2000-01-01T00:00:00Z`.
 * Sonoff irrigation devices encode local datetimes relative to this base.
 */
exports.YEAR_2000_IN_UTC = Math.floor(Date.UTC(2000, 0, 1) / 1000);
/**
 * Convert Unix UTC seconds to device local-time seconds (2000 base).
 */
const utcToDeviceLocal2000Seconds = (utcSeconds, offsetSeconds) => {
    return utcSeconds + offsetSeconds - exports.YEAR_2000_IN_UTC;
};
exports.utcToDeviceLocal2000Seconds = utcToDeviceLocal2000Seconds;
/**
 * Convert device local-time seconds (2000 base) back to Unix UTC seconds.
 */
const deviceLocal2000ToUTCSeconds = (deviceSeconds, offsetSeconds) => {
    return deviceSeconds - offsetSeconds + exports.YEAR_2000_IN_UTC;
};
exports.deviceLocal2000ToUTCSeconds = deviceLocal2000ToUTCSeconds;
/**
 * Parse ISO 8601 datetime (must include `Z` or `±HH:mm`) to Unix UTC seconds.
 * Returns `undefined` when format is invalid or parsed value is negative.
 */
const parseIsoWithOffsetToUtcSeconds = (value) => {
    if (!/(Z|[+-]\d{2}:\d{2})$/.test(value)) {
        return;
    }
    const timeMs = new Date(value).getTime();
    if (Number.isNaN(timeMs)) {
        return;
    }
    const seconds = Math.floor(timeMs / 1000);
    if (seconds < 0) {
        return;
    }
    return seconds;
};
exports.parseIsoWithOffsetToUtcSeconds = parseIsoWithOffsetToUtcSeconds;
/**
 * Get runtime local timezone offset in seconds for the specified UTC timestamp.
 */
const getRuntimeLocalOffsetSeconds = (utcSeconds) => {
    const date = new Date(utcSeconds * 1000);
    return -date.getTimezoneOffset() * 60;
};
exports.getRuntimeLocalOffsetSeconds = getRuntimeLocalOffsetSeconds;
/**
 * Format Unix UTC seconds to ISO 8601 with the specified timezone offset.
 * Falls back to the runtime local timezone offset when not provided.
 */
const formatUtcSecondsToIsoWithOffset = (utcSeconds, offsetSeconds) => {
    const resolvedOffsetSeconds = typeof offsetSeconds === "number" && Number.isFinite(offsetSeconds) ? offsetSeconds : (0, exports.getRuntimeLocalOffsetSeconds)(utcSeconds);
    const sign = resolvedOffsetSeconds >= 0 ? "+" : "-";
    const offsetMinutesAbs = Math.abs(Math.floor(resolvedOffsetSeconds / 60));
    const offsetHours = Math.floor(offsetMinutesAbs / 60);
    const offsetMinutes = offsetMinutesAbs % 60;
    const localMs = utcSeconds * 1000 + resolvedOffsetSeconds * 1000;
    const localDate = new Date(localMs);
    return (`${localDate.getUTCFullYear()}-${String(localDate.getUTCMonth() + 1).padStart(2, "0")}-${String(localDate.getUTCDate()).padStart(2, "0")}T` +
        `${String(localDate.getUTCHours()).padStart(2, "0")}:${String(localDate.getUTCMinutes()).padStart(2, "0")}:${String(localDate.getUTCSeconds()).padStart(2, "0")}` +
        `${sign}${String(offsetHours).padStart(2, "0")}:${String(offsetMinutes).padStart(2, "0")}`);
};
exports.formatUtcSecondsToIsoWithOffset = formatUtcSecondsToIsoWithOffset;
/**
 * Shift UTC seconds by local calendar months under a fixed UTC -> local offset.
 */
const shiftUtcSecondsByOffsetMonths = (utcSeconds, monthDelta, offsetSeconds = 0) => {
    const localMs = utcSeconds * 1000 + offsetSeconds * 1000;
    const localDate = new Date(localMs);
    const shiftedLocalMs = Date.UTC(localDate.getUTCFullYear(), localDate.getUTCMonth() + monthDelta, localDate.getUTCDate(), localDate.getUTCHours(), localDate.getUTCMinutes(), localDate.getUTCSeconds(), localDate.getUTCMilliseconds());
    return Math.floor((shiftedLocalMs - offsetSeconds * 1000) / 1000);
};
exports.shiftUtcSecondsByOffsetMonths = shiftUtcSecondsByOffsetMonths;
/**
 * Extract the ZCL command id and payload from a raw SWV-ZN/ZF frame.
 * Handles both standard and manufacturer-specific headers.
 */
const parseSWVZFRawZclCommand = (buffer) => {
    if (buffer.length < 3) {
        return;
    }
    const frameControl = buffer[0];
    const hasManufacturerCode = (frameControl & 0b100) !== 0;
    const zclHeaderLength = hasManufacturerCode ? 5 : 3;
    if (buffer.length < zclHeaderLength) {
        return;
    }
    return {
        commandId: buffer[zclHeaderLength - 1],
        payload: buffer.subarray(zclHeaderLength),
    };
};
exports.parseSWVZFRawZclCommand = parseSWVZFRawZclCommand;
/**
 * Swap the byte order of a 32-bit value reported as a big-endian unsigned integer.
 */
const toBigEndianUInt32 = (rawValue) => {
    return (((rawValue & 0xff) << 24) | ((rawValue & 0xff00) << 8) | ((rawValue >>> 8) & 0xff00) | ((rawValue >>> 24) & 0xff)) >>> 0;
};
exports.toBigEndianUInt32 = toBigEndianUInt32;
/**
 * Decode a 32-bit milli-value that is exposed through a UINT32 ZCL attribute
 * but uses two's-complement encoding when the reported value is below zero.
 */
const signedInt32MilliToValue = (value) => (value > 0x7fffffff ? value - 0x100000000 : value) / 1000;
exports.signedInt32MilliToValue = signedInt32MilliToValue;
/**
 * Read an unsigned 32-bit little-endian integer from an array-like byte buffer.
 */
const readUInt32LE = (data, index) => {
    return ((data[index] ?? 0) | ((data[index + 1] ?? 0) << 8) | ((data[index + 2] ?? 0) << 16) | ((data[index + 3] ?? 0) << 24)) >>> 0;
};
exports.readUInt32LE = readUInt32LE;
/**
 * Read an unsigned 40-bit little-endian integer from an array-like byte buffer.
 * Multiplication avoids JavaScript's 32-bit bitwise truncation.
 */
const readUInt40LE = (data, index) => {
    return ((data[index] ?? 0) +
        (data[index + 1] ?? 0) * 0x100 +
        (data[index + 2] ?? 0) * 0x10000 +
        (data[index + 3] ?? 0) * 0x1000000 +
        (data[index + 4] ?? 0) * 0x100000000);
};
exports.readUInt40LE = readUInt40LE;
/**
 * Read an unsigned 16-bit little-endian integer from an array-like byte buffer.
 */
const readUInt16LE = (data, index) => {
    return (data[index] ?? 0) | ((data[index + 1] ?? 0) << 8);
};
exports.readUInt16LE = readUInt16LE;
/**
 * Normalize Zigbee-herdsman ZCL array values to a plain byte array.
 * Depending on the decoder path, arrays may arrive as `Uint8Array`, `number[]`,
 * or an object with an `elements` field.
 */
const zclArrayValueToBytes = (value) => {
    if (value instanceof Uint8Array) {
        return Array.from(value);
    }
    if (Array.isArray(value)) {
        return value.map((item) => Number(item) & 0xff);
    }
    if (value !== null && typeof value === "object" && "elements" in value) {
        const elements = value.elements;
        if (elements instanceof Uint8Array) {
            return Array.from(elements);
        }
        if (Array.isArray(elements)) {
            return elements.map((item) => Number(item) & 0xff);
        }
    }
};
exports.zclArrayValueToBytes = zclArrayValueToBytes;
/**
 * Decode a Sonoff private attribute whose value is a ZCL array containing one
 * unsigned 32-bit little-endian integer.
 */
const zclArrayUInt32FzConvert = (name, attributeKey) => {
    return (model, msg, publish, options, meta) => {
        if (!(attributeKey in msg.data)) {
            return;
        }
        const bytes = (0, exports.zclArrayValueToBytes)(msg.data[attributeKey]);
        if (bytes === undefined || bytes.length < 4) {
            logger_1.logger.warning(`${attributeKey} payload is not a uint32 ZCL array value`, NS);
            return;
        }
        return { [name]: (0, exports.readUInt32LE)(bytes, 0) };
    };
};
exports.zclArrayUInt32FzConvert = zclArrayUInt32FzConvert;
/**
 * Encode an unsigned 32-bit integer as little-endian bytes.
 */
const toUInt32LEBytes = (value) => {
    const uint32Value = value >>> 0;
    return [uint32Value & 0xff, (uint32Value >> 8) & 0xff, (uint32Value >> 16) & 0xff, (uint32Value >> 24) & 0xff];
};
exports.toUInt32LEBytes = toUInt32LEBytes;
/**
 * Encode an unsigned 16-bit integer as little-endian bytes.
 */
const toUInt16LEBytes = (value) => {
    const uint16Value = value & 0xffff;
    return [uint16Value & 0xff, (uint16Value >> 8) & 0xff];
};
exports.toUInt16LEBytes = toUInt16LEBytes;
/**
 * Format a time-of-day value stored as seconds since midnight.
 * Uses `HH:mm` when there are no seconds, otherwise `HH:mm:ss`.
 */
const formatSecondsToTimeSinceMidnight = (seconds) => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const remainingSeconds = seconds % 60;
    const value = `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}`;
    return remainingSeconds === 0 ? value : `${value}:${String(remainingSeconds).padStart(2, "0")}`;
};
exports.formatSecondsToTimeSinceMidnight = formatSecondsToTimeSinceMidnight;
/**
 * Parse `HH:mm` or `HH:mm:ss` into seconds since midnight.
 * Throws when the format is invalid or points outside the current day.
 */
const parseTimeToSecondsSinceMidnight = (time, field = "value") => {
    const match = time.match(/^([01]\d|2[0-3]):([0-5]\d)(?::([0-5]\d))?$|^24:00(?::00)?$/);
    if (!match) {
        throw new Error(`Invalid ${field}, expected time from midnight (e.g. 08:30 or 18:00)`);
    }
    if (time.startsWith("24:")) {
        throw new Error(`Invalid ${field}, 24:00 is not supported. Use 00:00 of the next day instead.`);
    }
    return Number(match[1]) * 3600 + Number(match[2]) * 60 + Number(match[3] ?? 0);
};
exports.parseTimeToSecondsSinceMidnight = parseTimeToSecondsSinceMidnight;
/**
 * Read attributes in batches of four; retry failed batches one attribute at a time.
 * Log individual failures and continue so unsupported attributes do not stop configuration.
 * @param cluster Cluster name or ID (e.g. 0xfc11).
 * @param attributes Attribute names or IDs to read.
 * @param endpoint Device endpoint to read from.
 * @param options Optional Zigbee read options, also used for retries.
 */
const readAttributesInBatches = async (cluster, attributes, endpoint, options) => {
    const readAttributes = async (batch) => {
        try {
            await endpoint.read(cluster, batch, options);
        }
        catch (error) {
            if (batch.length === 1) {
                const attribute = typeof batch[0] === "number" ? `0x${batch[0].toString(16)}` : batch[0];
                logger_1.logger.error(`Failed to read cluster ${cluster} attribute ${attribute}: ${error}`, NS);
                return;
            }
            for (const attribute of batch) {
                await readAttributes([attribute]);
            }
        }
    };
    for (let i = 0; i < attributes.length; i += 4) {
        await readAttributes(attributes.slice(i, i + 4));
    }
};
exports.readAttributesInBatches = readAttributesInBatches;
/** Gate real-device exposes by the predicate; retain dummy exposes for definition exports unless explicitly disabled. */
const withConditionalExpose = (extend, predicate, { exposeOnDummy = true } = {}) => {
    const originalExposes = extend.exposes ?? [];
    const expose = (device, options) => {
        if (utils.isDummyDevice(device) ? !exposeOnDummy : !predicate(device))
            return [];
        return originalExposes.flatMap((item) => (typeof item === "function" ? item(device, options) : [item]));
    };
    return { ...extend, exposes: [expose] };
};
exports.withConditionalExpose = withConditionalExpose;
// Expose an enum's values only while the device supports them: `gatedValues` are exposed only while `isSupported`
// returns true (e.g. a firmware version floor), every other value is always exposed. The wrapped extend keeps its full
// lookup, so reported values and writes are unaffected. Dummy devices retain all values for definition exports and device docs.
const withConditionalEnumValues = (extend, gatedValues, isSupported) => {
    const expose = (device, options) => {
        const items = (extend.exposes ?? []).flatMap((item) => (typeof item === "function" ? item(device, options) : [item]));
        if (utils.isDummyDevice(device) || isSupported(device))
            return items;
        return items.map((item) => {
            if (!(item instanceof exposes.Enum))
                return item;
            const reduced = item.clone();
            reduced.values = item.values.filter((value) => !gatedValues.includes(value));
            return reduced;
        });
    };
    return { ...extend, exposes: [expose] };
};
exports.withConditionalEnumValues = withConditionalEnumValues;
/**
 * Checks whether a device firmware version supports a feature.
 * Missing firmware or a mismatched optional model is treated as unsupported.
 * @param targetVersion Firmware version threshold.
 * @param type Whether the current version must be lower than, or equal to/higher than, the threshold.
 * @returns Whether the feature should be exposed.
 */
const firmwareSupportFeaturesVersion = (targetVersion, type, options) => {
    const targetParts = targetVersion.split(".").map(Number);
    return (device) => {
        if (options?.modelID !== undefined && device.modelID !== options.modelID)
            return false;
        if (!device.softwareBuildID)
            return false;
        const currentParts = device.softwareBuildID.split(".").map(Number);
        const length = Math.max(currentParts.length, targetParts.length);
        for (let i = 0; i < length; i++) {
            const currentPart = Number.isFinite(currentParts[i]) ? currentParts[i] : 0;
            const targetPart = Number.isFinite(targetParts[i]) ? targetParts[i] : 0;
            if (currentPart < targetPart)
                return type === "lower";
            if (currentPart > targetPart)
                return type === "higher";
        }
        return type === "higher";
    };
};
exports.firmwareSupportFeaturesVersion = firmwareSupportFeaturesVersion;
//# sourceMappingURL=sonoff.js.map