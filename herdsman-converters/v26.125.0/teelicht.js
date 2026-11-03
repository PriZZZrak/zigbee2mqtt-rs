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
const m = __importStar(require("../lib/modernExtend"));
exports.definitions = [
    {
        zigbeeModel: ["teelicht-c6"],
        model: "teelicht",
        vendor: "teelicht",
        description: "Rechargeable tealight that flickers like a real candle",
        extend: [
            m.deviceAddCustomCluster("teelichtFlicker", {
                name: "teelichtFlicker",
                ID: 0xfc00,
                attributes: {
                    flickerDepth: { name: "flickerDepth", ID: 0x0000, type: zigbee_herdsman_1.Zcl.DataType.UINT8, write: true, max: 100 },
                    flickerSpeed: { name: "flickerSpeed", ID: 0x0001, type: zigbee_herdsman_1.Zcl.DataType.UINT8, write: true, min: 3, max: 40 },
                    flickerEffect: { name: "flickerEffect", ID: 0x0002, type: zigbee_herdsman_1.Zcl.DataType.ENUM8, write: true, max: 1 },
                    powerState: { name: "powerState", ID: 0x0003, type: zigbee_herdsman_1.Zcl.DataType.ENUM8, max: 3 },
                },
                commands: {},
                commandsResponse: {},
            }),
            // Starts dark at 2200 K after every boot and has no trigger effect handler.
            // configureReporting: without it the board doesn't report presses on its own button.
            m.light({ colorTemp: { range: [370, 555], startup: false }, powerOnBehavior: false, effect: false, configureReporting: true }),
            m.identify(),
            // Firmware 0.3.0 and later. Before m.battery: Zigbee2MQTT runs the configure steps in order
            // and stops at the first failure, and boards up to 0.3.1 refuse the battery voltage report.
            m.enumLookup({
                name: "power_state",
                cluster: "teelichtFlicker",
                attribute: "powerState",
                description: "On battery, charging over USB, charged, or on USB without a cell.",
                lookup: { battery: 0, charging: 1, charged: 2, no_cell: 3 },
                access: "STATE_GET",
                reporting: { min: 0, max: 3600, change: 0 },
                entityCategory: "diagnostic",
            }),
            // The defaults (at most hourly, voltage only on a 1 V change) hide almost everything. The firmware
            // updates both every 10 min, on a 2 % change and when USB comes or goes; let those through.
            m.battery({
                voltage: true,
                voltageReporting: true,
                percentageReportingConfig: { min: 10, max: 3600, change: 2 },
                voltageReportingConfig: { min: 10, max: 3600, change: 1 },
            }),
            m.numeric({
                name: "flicker_depth",
                cluster: "teelichtFlicker",
                attribute: "flickerDepth",
                description: "How strongly the flame flickers. 0 is a steady light.",
                unit: "%",
                valueMin: 0,
                valueMax: 100,
                valueStep: 1,
                access: "ALL",
                reporting: false,
                entityCategory: "config",
            }),
            m.numeric({
                name: "flicker_speed",
                cluster: "teelichtFlicker",
                attribute: "flickerSpeed",
                description: "Speed of the flicker, 1 is a calm candle.",
                unit: "x",
                valueMin: 0.3,
                valueMax: 4,
                valueStep: 0.1,
                scale: 10,
                precision: 1,
                access: "ALL",
                reporting: false,
                entityCategory: "config",
            }),
            m.enumLookup({
                name: "flicker_effect",
                cluster: "teelichtFlicker",
                attribute: "flickerEffect",
                description: "Candle flicker or a steady light.",
                lookup: { candle: 0, steady: 1 },
                access: "ALL",
                reporting: false,
                entityCategory: "config",
            }),
        ],
    },
];
//# sourceMappingURL=teelicht.js.map