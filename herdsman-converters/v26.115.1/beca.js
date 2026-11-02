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
const exposes = __importStar(require("../lib/exposes"));
const tuya = __importStar(require("../lib/tuya"));
const e = exposes.presets;
const ea = exposes.access;
exports.definitions = [
    {
        fingerprint: tuya.fingerprint("TS0601", ["_TZE204_6ewjlefg"]),
        model: "BVRF-L001",
        vendor: "Beca",
        description: "VRF/VRV central air-conditioning thermostat",
        extend: [tuya.modernExtend.tuyaBase({ dp: true })],
        exposes: [
            e.binary("state", ea.STATE_SET, "ON", "OFF").withDescription("Turn the thermostat on or off independently of the operating mode"),
            e
                .climate()
                .withSystemMode(["cool", "heat", "fan_only", "dry"], ea.STATE_SET)
                .withFanMode(["auto", "low", "medium", "high"], ea.STATE_SET)
                .withSetpoint("current_heating_setpoint", 16, 32, 1, ea.STATE_SET)
                .withLocalTemperature(ea.STATE),
            e.child_lock(),
        ],
        meta: {
            tuyaDatapoints: [
                [1, "state", tuya.valueConverter.onOff],
                [
                    2,
                    "system_mode",
                    tuya.valueConverterBasic.lookup({
                        cool: tuya.enum(0),
                        heat: tuya.enum(1),
                        fan_only: tuya.enum(2),
                        dry: tuya.enum(3),
                    }),
                ],
                [
                    16,
                    "current_heating_setpoint",
                    {
                        from: (value) => value / 10,
                        to: (value) => {
                            if (!Number.isInteger(value) || value < 16 || value > 32) {
                                throw new Error(`current_heating_setpoint must be an integer from 16 to 32 °C: ${value}`);
                            }
                            return value * 10;
                        },
                    },
                ],
                [24, "local_temperature", tuya.valueConverter.divideBy10FromOnly],
                [40, "child_lock", tuya.valueConverter.lockUnlock],
                [
                    49,
                    "fan_mode",
                    tuya.valueConverterBasic.lookup({
                        auto: tuya.enum(0),
                        low: tuya.enum(1),
                        medium: tuya.enum(2),
                        high: tuya.enum(3),
                    }),
                ],
            ],
        },
    },
];
//# sourceMappingURL=beca.js.map