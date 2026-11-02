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
const logger_1 = require("../lib/logger");
const m = __importStar(require("../lib/modernExtend"));
const reporting = __importStar(require("../lib/reporting"));
const e = exposes.presets;
const NS = "zhc:waxman";
/**
 * ZCL Appliance Events and Alerts (0x0B02) carries `aalert` as LIST_UINT24 -
 * an array of 24-bit alert structures, one per active alert, with `alertscount`
 * alongside it. Each structure is:
 *
 *   bits  0-7   alert identifier (manufacturer-defined)
 *   bits  8-11  category          (1 = warning, 2 = danger, 3 = failure)
 *   bits 12-13  presence/recovery (1 = present, 0 = recovered)
 *   bits 14-23  reserved
 *
 * The cluster declares no attributes, only commands, so the alert state cannot
 * be obtained by attribute reporting - only from an unsolicited
 * `alertsNotification`, or by asking with `getAlerts`. Both carry the same
 * payload and both are absolute: the device reports every alert it currently
 * holds, so an empty list means nothing is wrong.
 *
 * Presence/recovery is documented as 0 = recovery and 1 = presence, leaving 2
 * and 3 undefined. This treats any non-zero value as "present" rather than
 * matching 1 exactly: on a leak detector, an unexpected encoding should surface
 * as an alert rather than be silently discarded.
 */
const ALERT_RECOVERED = 0;
/**
 * Captured from an 8840100H sitting at 2400 mV and reporting 0% battery. It
 * answered `getAlerts` with a single alert, raw `0x1182`:
 *
 *   id 0x82, category 1 (warning), presence 1 (present)
 *
 * A dry sibling on the same network at 3800 mV answers `alertscount: 0` with an
 * empty list, and replacing the cells in the first one cleared it to the same.
 *
 * Category 1 is "warning", which is consistent with a housekeeping notice
 * rather than water; a leak detector reporting actual water would be expected
 * to use 2 (danger) or 3 (failure).
 */
const ALERT_ID_LOW_BATTERY = 0x82;
function decodeAlerts(aalert) {
    // Defensive: the payload is declared LIST_UINT24 and so arrives as a
    // number[], but a single-element list is easy to mistake for a scalar - and
    // doing so is what broke the previous converter (see the PR for this file).
    const list = Array.isArray(aalert) ? aalert : typeof aalert === "number" ? [aalert] : [];
    return list.map((raw) => ({
        raw,
        id: raw & 0xff,
        category: (raw >>> 8) & 0x0f,
        present: ((raw >>> 12) & 0x03) !== ALERT_RECOVERED,
    }));
}
function convertAlerts(msg) {
    const alerts = decodeAlerts(msg.data.aalert);
    /**
     * Any present alert that is not the known low-battery one is reported as
     * water. This is deliberately the fail-safe direction: the identifier this
     * device uses for water has not been captured, and for a leak detector it
     * is far better to report a leak that turns out to be something else than
     * to swallow a real one.
     *
     * Do not invert this into an allowlist of water identifiers without a
     * captured wet event to populate it - a leak whose id is missing from an
     * allowlist becomes silence.
     */
    const water = alerts.filter((a) => a.present && a.id !== ALERT_ID_LOW_BATTERY);
    for (const a of water) {
        logger_1.logger.warning(`${msg.device?.ieeeAddr}: unrecognised alert id 0x${a.id.toString(16)} (category ${a.category}, raw 0x${a.raw.toString(16)}), ` +
            "reported as water_leak. If this is not water, it should be identified and excluded.", NS);
    }
    return {
        water_leak: water.length > 0,
        battery_low: alerts.some((a) => a.present && a.id === ALERT_ID_LOW_BATTERY),
    };
}
const fzLocal = {
    // Unsolicited: the device raised or cleared an alert on its own.
    alerts_notification: {
        cluster: "haApplianceEventsAlerts",
        type: ["commandAlertsNotification"],
        convert: (_model, msg) => convertAlerts(msg),
    },
    // Solicited: the reply to `getAlerts`. Same payload, same absolute meaning.
    alerts_response: {
        cluster: "haApplianceEventsAlerts",
        type: ["commandGetAlertsRsp"],
        convert: (_model, msg) => convertAlerts(msg),
    },
};
const tzLocal = {
    /**
     * Lets the alert state be re-read on demand. Because the cluster has no
     * attributes, this is the only way to resynchronise after a restart from
     * cached state, and the only way to observe an alert clearing if the device
     * does not volunteer a notification.
     */
    alerts: {
        key: ["water_leak", "battery_low"],
        convertGet: async (entity) => {
            await entity.command("haApplianceEventsAlerts", "getAlerts", {}, {});
        },
    },
};
exports.definitions = [
    {
        zigbeeModel: ["leakSMART Water Sensor V2"],
        model: "8840100H",
        vendor: "Waxman",
        description: "leakSMART water sensor v2",
        fromZigbee: [fzLocal.alerts_notification, fzLocal.alerts_response],
        toZigbee: [tzLocal.alerts],
        exposes: [e.water_leak()],
        extend: [m.battery({ voltage: true, voltageReporting: true, lowStatus: true }), m.temperature()],
        configure: async (device, coordinatorEndpoint) => {
            // battery and temperature bind and configure their own reporting via
            // the modern extends above; only the alerts cluster is left.
            await reporting.bind(device.getEndpoint(1), coordinatorEndpoint, ["haApplianceEventsAlerts"]);
        },
    },
    {
        zigbeeModel: ["House Water Valve - MDL-TBD", "leakSMART Water Valve v2.10"],
        // Should work with all manufacturer model numbers for the 2.0 series:
        // 8850000 3/4"
        // 8850100 1"
        // 8850200 1-1/4"
        // 8850300 1-1/2"
        // 8850310 2"
        model: "8850100",
        vendor: "Waxman",
        description: "leakSMART automatic water shut-off valve 2.0",
        fromZigbee: [fz.battery, fz.on_off],
        toZigbee: [tz.on_off],
        exposes: [e.battery(), e.switch()],
        configure: async (device, coordinatorEndpoint) => {
            const endpoint = device.getEndpoint(1);
            await reporting.bind(endpoint, coordinatorEndpoint, ["genPowerCfg", "haApplianceEventsAlerts", "genOnOff"]);
            await reporting.onOff(endpoint);
            await reporting.batteryPercentageRemaining(endpoint);
            await reporting.batteryVoltage(endpoint);
        },
    },
];
//# sourceMappingURL=waxman.js.map