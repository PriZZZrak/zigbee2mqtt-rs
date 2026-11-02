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
const reporting = __importStar(require("../lib/reporting"));
const tuya = __importStar(require("../lib/tuya"));
const e = exposes.presets;
const ea = exposes.access;
const ch8zChannels = ["l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8"];
exports.definitions = [
    {
        fingerprint: tuya.fingerprint("TS011F", ["_TZ3000_j0ktmul1"]),
        model: "AUT000069",
        vendor: "AutomatOn",
        description: "Underfloor heating / Irrigation valves controller - 5 zones",
        extend: [
            tuya.modernExtend.tuyaBase(),
            tuya.modernExtend.tuyaOnOff({ powerOnBehavior2: true, childLock: true, onOffCountdown: true, endpoints: ["l1", "l2", "l3", "l4", "l5"] }),
        ],
        endpoint: (device) => {
            return { l1: 1, l2: 2, l3: 3, l4: 4, l5: 5 };
        },
        meta: { multiEndpoint: true },
        configure: async (device, coordinatorEndpoint) => {
            await tuya.configureMagicPacket(device, coordinatorEndpoint);
            await reporting.bind(device.getEndpoint(1), coordinatorEndpoint, ["genOnOff"]);
            await reporting.bind(device.getEndpoint(2), coordinatorEndpoint, ["genOnOff"]);
            await reporting.bind(device.getEndpoint(3), coordinatorEndpoint, ["genOnOff"]);
            await reporting.bind(device.getEndpoint(4), coordinatorEndpoint, ["genOnOff"]);
            await reporting.bind(device.getEndpoint(5), coordinatorEndpoint, ["genOnOff"]);
            device.powerSource = "Mains (single phase)";
            device.save();
        },
    },
    {
        fingerprint: tuya.fingerprint("TS0601", ["_TZE284_1oft6qso"]),
        model: "CH8Z",
        vendor: "AutomatOn",
        description: "Underfloor heating controller - 8 zones",
        // Tuya datapoints have no ZCL read path; the queries are what populate state.
        extend: [tuya.modernExtend.tuyaBase({ dp: true, queryOnConfigure: true, queryOnDeviceAnnounce: true })],
        // All channels are on endpoint 1 and addressed by datapoint.
        endpoint: () => ({ l1: 1, l2: 1, l3: 1, l4: 1, l5: 1, l6: 1, l7: 1, l8: 1 }),
        exposes: [
            ...ch8zChannels.map((channel) => tuya.exposes.switch().withEndpoint(channel)),
            ...ch8zChannels.map((channel) => tuya.exposes.countdown().withEndpoint(channel)),
            e.power_on_behavior().withAccess(ea.STATE_SET),
            e.child_lock(),
        ],
        meta: {
            multiEndpoint: true,
            tuyaDatapoints: [
                [1, "state_l1", tuya.valueConverter.onOff],
                [2, "state_l2", tuya.valueConverter.onOff],
                [3, "state_l3", tuya.valueConverter.onOff],
                [4, "state_l4", tuya.valueConverter.onOff],
                [5, "state_l5", tuya.valueConverter.onOff],
                [6, "state_l6", tuya.valueConverter.onOff],
                // DP 7 and 8 are named usb_switch_1/2 in the Tuya specification,
                // a leftover from the power strip template this firmware derives
                // from. They drive physical channels 7 and 8.
                [7, "state_l7", tuya.valueConverter.onOff],
                [8, "state_l8", tuya.valueConverter.onOff],
                [9, "countdown_l1", tuya.valueConverter.countdown],
                [10, "countdown_l2", tuya.valueConverter.countdown],
                [11, "countdown_l3", tuya.valueConverter.countdown],
                [12, "countdown_l4", tuya.valueConverter.countdown],
                [13, "countdown_l5", tuya.valueConverter.countdown],
                [14, "countdown_l6", tuya.valueConverter.countdown],
                [15, "countdown_l7", tuya.valueConverter.countdown],
                [16, "countdown_l8", tuya.valueConverter.countdown],
                [27, "power_on_behavior", tuya.valueConverter.powerOnBehaviorEnum],
                [29, "child_lock", tuya.valueConverter.lockUnlock],
                // DP 17-26 (metering) and DP 28 (indicator mode) are in the
                // Tuya specification but are not implemented by this firmware:
                // they are never reported, and writes to DP 28 have no effect.
                // DP 101/102 (schedules) have an undocumented payload format
                // and were not investigated.
            ],
        },
    },
];
//# sourceMappingURL=automaton.js.map