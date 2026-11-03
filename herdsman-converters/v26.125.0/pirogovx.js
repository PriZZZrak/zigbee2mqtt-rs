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
const fz = __importStar(require("../converters/fromZigbee"));
const tz = __importStar(require("../converters/toZigbee"));
const exposes = __importStar(require("../lib/exposes"));
const e = exposes.presets;
const ea = exposes.access;
// Custom attributes reported by the firmware on the standard Analog Input cluster.
const attr = {
    power: 0xf000,
    mode: 0xf001,
    fanMode: 0xf002,
    swingMode: 0xf003,
    preset: 0xf004,
    display: 0xf005,
    indoorTemp: 0xf006,
    outdoorTemp: 0xf007,
    targetTemp: 0xf008,
    firmwareVersion: 0xf009,
};
const dataType = {
    boolean: 0x10,
    uint8: 0x20,
};
const idToMode = ["off", "auto", "cool", "heat", "dry", "fan_only"];
const idToFan = ["auto", "low", "medium", "high", "quiet"];
const fanToId = { auto: 0, low: 1, medium: 2, high: 3, quiet: 4 };
const idToSwing = ["off", "horizontal", "vertical", "both"];
const swingToId = { off: 0, horizontal: 1, vertical: 2, both: 3 };
const idToPreset = ["none", "sleep", "turbo"];
const presetToId = { none: 0, sleep: 1, turbo: 2 };
const writeAttr = async (entity, attribute, value, type) => await entity.write("genAnalogInput", { [attribute]: { value, type } });
const readAttr = async (entity, attribute) => await entity.read("genAnalogInput", [attribute]);
const fzLocal = {
    acAnalog: {
        cluster: "genAnalogInput",
        type: ["attributeReport", "readResponse"],
        convert: (model, msg) => {
            const data = msg.data;
            const result = {};
            if (data[attr.power] !== undefined)
                result.state = data[attr.power] ? "ON" : "OFF";
            if (data[attr.mode] !== undefined)
                result.system_mode = idToMode[data[attr.mode]] ?? "auto";
            if (data[attr.fanMode] !== undefined)
                result.fan_mode = idToFan[data[attr.fanMode]] ?? "auto";
            if (data[attr.swingMode] !== undefined)
                result.swing_mode = idToSwing[data[attr.swingMode]] ?? "off";
            if (data[attr.preset] !== undefined)
                result.preset = idToPreset[data[attr.preset]] ?? "none";
            if (data[attr.display] !== undefined)
                result.display = data[attr.display] ? "ON" : "OFF";
            if (data[attr.indoorTemp] !== undefined)
                result.local_temperature = data[attr.indoorTemp];
            if (data[attr.outdoorTemp] !== undefined)
                result.outdoor_temperature = data[attr.outdoorTemp];
            if (data[attr.targetTemp] !== undefined)
                result.occupied_heating_setpoint = data[attr.targetTemp];
            if (data[attr.firmwareVersion] !== undefined)
                result.firmware_version = data[attr.firmwareVersion];
            return result;
        },
    },
};
const tzLocal = {
    state: {
        key: ["state"],
        convertSet: async (entity, key, value) => {
            const on = value === "ON" || value === true;
            await writeAttr(entity, attr.power, on, dataType.boolean);
            return { state: { state: on ? "ON" : "OFF", system_mode: on ? "cool" : "off" } };
        },
    },
    fan_mode: {
        key: ["fan_mode"],
        convertSet: async (entity, key, value) => {
            const mode = String(value);
            if (!(mode in fanToId))
                throw new Error(`Unsupported fan_mode ${mode}`);
            await writeAttr(entity, attr.fanMode, fanToId[mode], dataType.uint8);
            return { state: { fan_mode: mode } };
        },
        convertGet: async (entity) => {
            await readAttr(entity, attr.fanMode);
        },
    },
    swing_mode: {
        key: ["swing_mode"],
        convertSet: async (entity, key, value) => {
            const mode = String(value);
            if (!(mode in swingToId))
                throw new Error(`Unsupported swing_mode ${mode}`);
            await writeAttr(entity, attr.swingMode, swingToId[mode], dataType.uint8);
            return { state: { swing_mode: mode } };
        },
        convertGet: async (entity) => {
            await readAttr(entity, attr.swingMode);
        },
    },
    preset: {
        key: ["preset"],
        convertSet: async (entity, key, value) => {
            const preset = String(value);
            if (!(preset in presetToId))
                throw new Error(`Unsupported preset ${preset}`);
            await writeAttr(entity, attr.preset, presetToId[preset], dataType.uint8);
            return { state: { preset } };
        },
        convertGet: async (entity) => {
            await readAttr(entity, attr.preset);
        },
    },
    display: {
        key: ["display"],
        convertSet: async (entity, key, value) => {
            const enabled = value === "ON" || value === true;
            await writeAttr(entity, attr.display, enabled, dataType.boolean);
            return { state: { display: enabled ? "ON" : "OFF" } };
        },
        convertGet: async (entity) => {
            await readAttr(entity, attr.display);
        },
    },
};
exports.definitions = [
    {
        fingerprint: [{ modelID: "ZB-MIDEA-AC", manufacturerName: "PirogovX" }],
        model: "ZB-MIDEA-AC",
        vendor: "PirogovX",
        description: "Zigbee air conditioner controller for Midea / Royal Clima / Hommyn / Neoline (ESP32-H2/C6)",
        fromZigbee: [fzLocal.acAnalog, fz.thermostat],
        toZigbee: [
            tzLocal.state,
            tz.thermostat_system_mode,
            tz.thermostat_occupied_heating_setpoint,
            tzLocal.fan_mode,
            tzLocal.swing_mode,
            tzLocal.preset,
            tzLocal.display,
        ],
        exposes: [
            e
                .climate()
                .withSetpoint("occupied_heating_setpoint", 16, 30, 1, ea.STATE_SET)
                .withLocalTemperature()
                .withSystemMode(["off", "auto", "cool", "heat", "dry", "fan_only"], ea.STATE_SET),
            e.enum("fan_mode", ea.STATE_SET, ["auto", "low", "medium", "high", "quiet"]).withDescription("Fan speed"),
            e.enum("swing_mode", ea.STATE_SET, ["off", "horizontal", "vertical", "both"]).withDescription("Swing mode"),
            e.enum("preset", ea.STATE_SET, ["none", "sleep", "turbo"]).withDescription("Preset mode"),
            e.binary("display", ea.STATE_SET, "ON", "OFF").withDescription("AC display and beep control"),
            e.numeric("outdoor_temperature", ea.STATE).withUnit("°C").withDescription("Outdoor unit temperature"),
            e.text("firmware_version", ea.STATE).withDescription("AC controller firmware version"),
        ],
        configure: async (device, coordinatorEndpoint) => {
            // Bind the clusters that carry telemetry so the device reports current/outdoor
            // temperature, mode and firmware version to the coordinator automatically.
            const endpoint = device.getEndpoint(1);
            await endpoint.bind("genAnalogInput", coordinatorEndpoint);
            try {
                await endpoint.bind("hvacThermostat", coordinatorEndpoint);
            }
            catch {
                // hvacThermostat binding is optional and may fail on some coordinators.
            }
        },
    },
];
//# sourceMappingURL=pirogovx.js.map