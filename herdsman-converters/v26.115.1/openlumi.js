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
const m = __importStar(require("../lib/modernExtend"));
exports.definitions = [
    {
        zigbeeModel: ["openlumi.gw_router.dgnwg05lm"],
        model: "LR-DGNWG05LM",
        vendor: "OpenLumi",
        description: "Lumi Router (for Xiaomi DGNWG05LM)",
        version: "0.0.1",
        extend: [m.deviceTemperature({ reporting: { min: "5_MINUTES", max: "1_HOUR", change: 1 } })],
    },
    {
        zigbeeModel: ["openlumi.gw_router.zhwg11lm"],
        model: "LR-ZHWG11LM",
        vendor: "OpenLumi",
        description: "Lumi Router (for Aqara ZHWG11LM)",
        version: "0.0.1",
        extend: [m.deviceTemperature({ reporting: { min: "5_MINUTES", max: "1_HOUR", change: 1 } })],
    },
    // Legacy unified firmware for DGNWG05LM and ZHWG11LM; kept for backward-compatible detection.
    {
        zigbeeModel: ["openlumi.gw_router.jn5169"],
        model: "GWRJN5169",
        vendor: "OpenLumi",
        description: "Lumi Router (outdated firmware, update required)",
        extend: [m.deviceTemperature()],
    },
];
//# sourceMappingURL=openlumi.js.map