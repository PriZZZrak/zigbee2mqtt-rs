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
const aristonExtend = {
    // ThermostatRunningMode does not follow the ZCL enum on this device (1 outside winter mode,
    // 4 in winter mode, whether heating or not), which makes fz.thermostat throw and drop the
    // other attributes of the message. Drop it since it does not reflect the running state.
    ignoreRunningMode: () => {
        const fromZigbee = [
            {
                cluster: "hvacThermostat",
                type: ["attributeReport", "readResponse"],
                convert: (model, msg, publish, options, meta) => {
                    delete msg.data.runningMode;
                },
            },
        ];
        return { fromZigbee, isModernExtend: true };
    },
};
exports.definitions = [
    {
        zigbeeModel: ["Rem4 Heater"],
        model: "Rem4 Heater",
        vendor: "Ariston",
        description: "Boiler gateway (room thermostat interface)",
        // Only tested with a single zone thermostat (endpoint 2). The boiler supports up to 6 zones,
        // which probably appear as additional endpoints once their thermostats are connected.
        extend: [
            m.onOff({ powerOnBehavior: false }),
            // Must come before m.thermostat() so the attribute is removed before fz.thermostat runs
            aristonExtend.ignoreRunningMode(),
            m.thermostat({
                localTemperature: { values: { description: "Room temperature measured by the boiler room unit" } },
                setpoints: { values: { occupiedHeatingSetpoint: { min: 8.5, max: 30, step: 0.5 } } },
                systemMode: { values: ["off", "heat"] },
                ctrlSeqeOfOper: { values: ["heating_only"], toZigbee: { skip: true } },
            }),
            m.numeric({
                name: "outdoor_temperature",
                cluster: "hvacThermostat",
                attribute: "outdoorTemp",
                description: "Outdoor temperature known by the boiler",
                unit: "°C",
                scale: 100,
                precision: 1,
                access: "STATE_GET",
                reporting: false,
            }),
        ],
    },
];
//# sourceMappingURL=ariston.js.map