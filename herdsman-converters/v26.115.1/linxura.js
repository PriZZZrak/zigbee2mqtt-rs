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
const m = __importStar(require("../lib/modernExtend"));
const e = exposes.presets;
const actionTypes = ["click", "double_click", "hold"];
function decodeZoneStatus(zoneStatus, buttonCount) {
    if (!Number.isInteger(zoneStatus) || zoneStatus < 1 || zoneStatus > buttonCount * 6 - 1) {
        return;
    }
    const offset = (zoneStatus - 1) % 6;
    if (offset !== 0 && offset !== 2 && offset !== 4) {
        return;
    }
    const button = Math.floor((zoneStatus - 1) / 6) + 1;
    const actionType = actionTypes[offset / 2];
    return `button_${button}_${actionType}`;
}
function buttonActions(buttonCount) {
    return Array.from({ length: buttonCount }, (_, buttonIndex) => actionTypes.map((actionType) => `button_${buttonIndex + 1}_${actionType}`)).flat();
}
function linxuraButton(buttonCount) {
    const fromZigbee = {
        cluster: "ssIasZone",
        type: ["attributeReport", "readResponse", "commandStatusChangeNotification"],
        convert: (model, msg, publish, options, meta) => {
            const zoneStatus = "zonestatus" in msg.data ? msg.data.zonestatus : msg.data.zoneStatus;
            if (zoneStatus === undefined) {
                return;
            }
            const action = decodeZoneStatus(zoneStatus, buttonCount);
            if (action !== undefined) {
                return { action };
            }
        },
    };
    return {
        fromZigbee: [fromZigbee],
        exposes: [e.action(buttonActions(buttonCount))],
        configure: [m.setupConfigureForBinding("ssIasZone", "input")],
        isModernExtend: true,
    };
}
exports.definitions = [
    {
        fingerprint: [{ modelID: "Aura Smart Button", manufacturerName: "Linxura" }],
        model: "SHCB-1-MO",
        vendor: "Linxura",
        description: "Aura 12-button smart controller",
        extend: [
            linxuraButton(12),
            m.battery({
                percentageReportingConfig: { min: "1_HOUR", max: 0, change: 1 },
            }),
        ],
    },
    {
        fingerprint: [{ modelID: "Smart Controller", manufacturerName: "Linxura" }],
        model: "SCHA-1-MO",
        vendor: "Linxura",
        description: "4-button smart controller",
        extend: [linxuraButton(4)],
    },
];
//# sourceMappingURL=linxura.js.map