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
exports.definitions = void 0;
const logger_1 = require("../lib/logger");
const m = __importStar(require("../lib/modernExtend"));
const tuya = __importStar(require("../lib/tuya"));
const NS = "zhc:simon";
/**
 * After pair, Simon i7 on/off switches often report a temporary S2100-100x modelID.
 * Neutral (router + mains) vs no-neutral (end device) remaps to the retail SKU.
 * Behavior copied from a production Z2M external converter; maintainers may prefer fingerprint-only.
 */
function remapNeutralNonNeutral(neutralModel, nonNeutralModel) {
    return (device) => {
        logger_1.logger.info(`Detection ${device.ieeeAddr} type=${device.type} power=${device.powerSource} model=${device.modelID}`, NS);
        if (device.powerSource === "Mains (single phase)" && device.type === "Router") {
            device.modelID = neutralModel;
            device.save();
        }
        else if ((device.powerSource === "Unknown" || device.powerSource == null) && device.type === "EndDevice") {
            device.modelID = nonNeutralModel;
            device.save();
        }
    };
}
exports.definitions = [
    // Existing dimmers — unchanged
    {
        zigbeeModel: ["SM0502"],
        model: "SM0502",
        vendor: "SIMON",
        description: "i7 2-gang smart dimming switch",
        extend: [m.deviceEndpoints({ endpoints: { left: 1, right: 2 } }), m.light({ endpointNames: ["left", "right"] })],
    },
    {
        zigbeeModel: ["SM0501"],
        model: "SM0501",
        vendor: "SIMON",
        description: "3 gang smart dimming switch",
        extend: [m.deviceEndpoints({ endpoints: { left: 1, center: 3, right: 4 } }), m.light({ endpointNames: ["left", "center", "right"] })],
    },
    // Pairing fingerprints (temporary modelID before SKU remap)
    {
        zigbeeModel: ["S2100-1001"],
        model: "S2100-1001",
        vendor: "SIMON",
        description: "i7 1-gang smart switch (pairing)",
        extend: [
            m.deviceEndpoints({ endpoints: { switch: 1 } }),
            m.onOff({ powerOnBehavior: false, endpointNames: ["switch"], configureReporting: false }),
        ],
        meta: { multiEndpoint: true },
        configure: remapNeutralNonNeutral("70E838TY", "70E850TY"),
    },
    {
        zigbeeModel: ["S2100-1002"],
        model: "S2100-1002",
        vendor: "SIMON",
        description: "i7 2-gang smart switch (pairing)",
        extend: [
            m.deviceEndpoints({ endpoints: { left: 1, right: 2 } }),
            m.onOff({ powerOnBehavior: false, endpointNames: ["left", "right"], configureReporting: false }),
        ],
        meta: { multiEndpoint: true },
        configure: remapNeutralNonNeutral("70E839TY", "70E851TY"),
    },
    {
        zigbeeModel: ["S2100-1003"],
        model: "S2100-1003",
        vendor: "SIMON",
        description: "i7 3-gang smart switch (pairing)",
        extend: [
            m.deviceEndpoints({ endpoints: { left: 1, middle: 2, right: 3 } }),
            m.onOff({ powerOnBehavior: false, endpointNames: ["left", "middle", "right"], configureReporting: false }),
        ],
        meta: { multiEndpoint: true },
        configure: remapNeutralNonNeutral("70E840TY", "70E852TY"),
    },
    // 1-gang neutral
    {
        zigbeeModel: ["70E838TY"],
        model: "70E838TY",
        vendor: "SIMON",
        description: "i7 1-gang smart switch (neutral)",
        extend: [
            m.deviceEndpoints({ endpoints: { switch: 1 } }),
            m.onOff({ powerOnBehavior: false, endpointNames: ["switch"] }),
            tuya.modernExtend.tuyaLedIndicator(),
        ],
        meta: { multiEndpoint: true },
    },
    // 1-gang no-neutral — modelID often null; match Tuya manufacturerName
    {
        zigbeeModel: ["70E850TY"],
        fingerprint: [{ manufacturerName: "_TZ2000_s93al43ewlu7wzwi" }],
        model: "70E850TY",
        vendor: "SIMON",
        description: "i7 1-gang smart switch (no neutral)",
        extend: [
            m.deviceEndpoints({ endpoints: { switch: 1 } }),
            m.onOff({ powerOnBehavior: false, endpointNames: ["switch"], configureReporting: false }),
            tuya.modernExtend.tuyaLedIndicator(),
        ],
        meta: { multiEndpoint: true },
    },
    // 2-gang neutral
    {
        zigbeeModel: ["70E839TY"],
        fingerprint: [{ manufacturerName: "_TZ2000_vvxwtxzf96vvarzj" }, { manufacturerName: "_TZ2000_mqovtiqe8m6av6po" }],
        model: "70E839TY",
        vendor: "SIMON",
        description: "i7 2-gang smart switch (neutral)",
        extend: [
            m.deviceEndpoints({ endpoints: { left: 1, right: 2 } }),
            m.onOff({ powerOnBehavior: false, endpointNames: ["left", "right"] }),
            tuya.modernExtend.tuyaLedIndicator(),
        ],
        meta: { multiEndpoint: true },
    },
    // 2-gang no-neutral
    {
        zigbeeModel: ["70E851TY"],
        fingerprint: [{ manufacturerName: "_TZ2000_zzk5p7jx29mq7nym" }],
        model: "70E851TY",
        vendor: "SIMON",
        description: "i7 2-gang smart switch (no neutral)",
        extend: [
            m.deviceEndpoints({ endpoints: { left: 1, right: 2 } }),
            m.onOff({ powerOnBehavior: false, endpointNames: ["left", "right"], configureReporting: false }),
            tuya.modernExtend.tuyaLedIndicator(),
        ],
        meta: { multiEndpoint: true },
    },
    // 3-gang neutral
    {
        zigbeeModel: ["70E840TY"],
        fingerprint: [{ manufacturerName: "_TZ2000_8auyu8pgbug5fzl5" }],
        model: "70E840TY",
        vendor: "SIMON",
        description: "i7 3-gang smart switch (neutral)",
        extend: [
            m.deviceEndpoints({ endpoints: { left: 1, middle: 2, right: 3 } }),
            m.onOff({ powerOnBehavior: false, endpointNames: ["left", "middle", "right"] }),
            tuya.modernExtend.tuyaLedIndicator(),
        ],
        meta: { multiEndpoint: true },
    },
    // 3-gang no-neutral
    {
        zigbeeModel: ["70E852TY"],
        fingerprint: [{ manufacturerName: "_TZ2000_p4vghfngci5te7ol" }],
        model: "70E852TY",
        vendor: "SIMON",
        description: "i7 3-gang smart switch (no neutral)",
        extend: [
            m.deviceEndpoints({ endpoints: { left: 1, middle: 2, right: 3 } }),
            m.onOff({ powerOnBehavior: false, endpointNames: ["left", "middle", "right"], configureReporting: false }),
            tuya.modernExtend.tuyaLedIndicator(),
        ],
        meta: { multiEndpoint: true },
    },
];
//# sourceMappingURL=simon.js.map