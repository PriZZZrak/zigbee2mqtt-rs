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
const zigbee_herdsman_1 = require("zigbee-herdsman");
const exposes = __importStar(require("../lib/exposes"));
const m = __importStar(require("../lib/modernExtend"));
const tuya = __importStar(require("../lib/tuya"));
const e = exposes.presets;
const ea = exposes.access;
exports.definitions = [
    {
        fingerprint: tuya.fingerprint("TS0225", ["_TZ3210_eep3fewj"]),
        model: "TS0225_EKAZA",
        vendor: "Ekaza",
        description: "24 GHz presence sensor",
        extend: [
            m.iasZoneAlarm({ zoneType: "occupancy", zoneAttributes: ["alarm_1"] }),
            // Keep this before tuyaBase so the generic DP converter does not intercept distance writes.
            m.numeric({
                name: "detection_distance",
                cluster: "manuSpecificTuya2",
                attribute: { ID: 0xe00b, type: zigbee_herdsman_1.Zcl.DataType.UINT16 },
                description: "Maximum detection distance",
                unit: "m",
                valueMin: 1,
                valueMax: 6,
                valueStep: 1,
                reporting: false,
            }),
            tuya.modernExtend.tuyaBase({ dp: true }),
        ],
        exposes: [
            e.numeric("illuminance", ea.STATE).withDescription("Raw illuminance reported by the sensor"),
            e
                .numeric("presence_delay", ea.STATE_SET)
                .withUnit("s")
                .withValueMin(1)
                .withValueMax(300)
                .withValueStep(1)
                .withDescription("Delay before reporting absence after presence is no longer detected"),
        ],
        meta: {
            tuyaSendCommand: "sendData",
            tuyaDatapoints: [
                [101, "presence_delay", tuya.valueConverter.raw],
                [104, "illuminance", tuya.valueConverter.raw],
            ],
        },
        configure: async (device) => {
            // IAS enrollment is handled by zigbee-herdsman during interview.
            await device.getEndpoint(1).read("ssIasZone", ["zoneStatus"]);
        },
    },
];
//# sourceMappingURL=ekaza.js.map